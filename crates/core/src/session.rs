//! Bounded, on-demand reader for archived work content (SPEC §7.2).
//!
//! `archive_read` stays as the raw protocol-1 preview; `session_read` adds a
//! structured, paged view. The package bytes are the source of truth: unknown
//! records are surfaced, opaque thinking stays opaque, and every page is bound
//! to a specific file digest so a changed source cannot reuse an old position.
use crate::{err, string, Engine, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

/// Decoded bytes returned per page. Bounded so a reader never loads whole
/// history at once.
pub(crate) const PAGE_BYTES: usize = 256 * 1024;

/// Record kinds a transcript can carry. `unknown` keeps bytes we do not parse.
fn classify_record(record: &Value) -> &'static str {
    match record.get("type").and_then(Value::as_str) {
        Some("user") => "user",
        Some("assistant") => "assistant",
        _ => "unknown",
    }
}

fn is_opaque_block(block: &Value) -> bool {
    matches!(
        block.get("type").and_then(Value::as_str),
        Some("thinking" | "redacted_thinking")
    ) || block.get("signature").is_some()
}

/// Extract the safe text of a message record. Opaque thinking is never placed
/// into derived text; it is reported as an opaque marker instead.
fn message_records(record: &Value, index: usize) -> Vec<Value> {
    let kind = classify_record(record);
    let timestamp = record
        .get("timestamp")
        .cloned()
        .unwrap_or(Value::Null);
    // `content` may be a rich array (text/tool blocks), a plain string (common
    // in user transcripts) or a single object. Normalize all three.
    let content = record["message"]
        .get("content")
        .or_else(|| record.get("content"));
    let raw_content = record["message"]["content"]
        .as_array()
        .or_else(|| record["content"].as_array());
    let mut out = vec![];
    let push = |out: &mut Vec<Value>, value: Value| {
        let mut value = value;
        value["timestamp"] = timestamp.clone();
        out.push(value);
    };
    match raw_content {
        Some(blocks) => {
            for block in blocks {
                match block.get("type").and_then(Value::as_str) {
                    Some("text") => push(&mut out, json!({
                        "index": index,
                        "kind": kind,
                        "text": block["text"],
                    })),
                    Some("tool_use") => push(&mut out, json!({
                        "index": index,
                        "kind": "tool_call",
                        "name": block["name"],
                        "tool": block["input"],
                    })),
                    Some("tool_result") => push(&mut out, json!({
                        "index": index,
                        "kind": "tool_result",
                        "text": block["content"],
                    })),
                    _ if is_opaque_block(block) => push(&mut out, json!({
                        "index": index,
                        "kind": kind,
                        "opaque": true,
                        "text": Value::Null,
                    })),
                    _ => push(&mut out, json!({
                        "index": index,
                        "kind": "unknown",
                        "unknown": true,
                        "block": block,
                    })),
                }
            }
        }
        None => {
            match content {
                Some(Value::String(text)) => push(&mut out, json!({
                    "index": index,
                    "kind": kind,
                    "text": text,
                })),
                Some(Value::Object(block)) => {
                    out.extend(message_records(&json!({"type": record["type"], "content": [block]}), index))
                }
                Some(other) => push(&mut out, json!({
                    "index": index,
                    "kind": kind,
                    "text": other.to_string(),
                })),
                // No content at all: keep the record visible rather than dropping.
                None => push(&mut out, json!({
                    "index": index,
                    "kind": "unknown",
                    "unknown": true,
                    "record": record,
                })),
            }
        }
    }
    out
}

impl Engine {
    /// Resolve a member file out of the frozen archive source, honoring an
    /// optional caller-supplied `expected_digest` binding. Returns the decoded
    /// bytes, file digest and resolved archive path.
    pub(crate) fn session_source(&self, r: &Value) -> Result<(PathBuf, Vec<u8>, String, String)> {
        let (package, package_digest, path) = self.archive_package(r)?;
        let name = string(r, "path")?;
        let entry = package["files"]
            .as_array()
            .ok_or_else(|| err("invalid_archive", "归档缺少文件清单"))?
            .iter()
            .find(|f| f["path"] == name)
            .ok_or_else(|| err("archive_file_missing", "归档没有该文件"))?
            .clone();
        let digest = string(&entry, "digest")?.to_string();
        if let Some(expected) = r.get("expected_digest").and_then(Value::as_str) {
            if expected != digest {
                return Err(err(
                    "stale_archive",
                    "来源文件摘要与选择时不同；旧阅读位置不能映射到改变后的内容",
                ));
            }
        }
        // `data` is a JSON byte array (serde serializes `Vec<u8>` that way), so
        // decoding it back gives the exact archived bytes.
        let bytes: Vec<u8> = serde_json::from_value(entry["data"].clone())?;
        Ok((path, bytes, digest, package_digest))
    }

    pub(crate) fn session_read(&self, r: &Value) -> Result<Value> {
        let (path, bytes, digest, package_digest) = self.session_source(r)?;
        let offset = r.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
        if offset > bytes.len() {
            return Err(err("invalid_offset", "偏移超出文件长度"));
        }
        let window_end = (offset + PAGE_BYTES).min(bytes.len());
        let name = string(r, "path")?.to_string();
        let content_kind = if name.ends_with(".jsonl") { "messages" } else { "text" };
        // For a JSONL transcript we advance only past complete lines so the tail
        // of an incomplete record is re-read next page instead of being lost; for
        // plain text we bound purely by bytes. Both ensure forward progress and a
        // stable byte position, even for one oversized line or many multibyte
        // characters, and never loop forever.
        let (end, complete) = if content_kind == "messages" {
            match bytes[offset..window_end].iter().rposition(|b| *b == b'\n') {
                Some(last_newline) => (offset + last_newline + 1, true),
                None if window_end == bytes.len() => (bytes.len(), true),
                None => {
                    // A single logical line longer than a page: bound by bytes so
                    // the caller can still make progress; the record is only
                    // parsed when its line is complete.
                    (window_end, false)
                }
            }
        } else {
            (window_end, true)
        };
        let done = end >= bytes.len();
        let page = &bytes[offset..end];
        let raw_text = String::from_utf8_lossy(page).into_owned();
        let mut records = vec![];
        if content_kind == "messages" && complete {
            for (i, line) in raw_text.lines().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                // Stable record identity: the byte offset where the line begins.
                let line_offset = offset
                    + raw_text
                        .lines()
                        .take(i)
                        .map(|previous| previous.len() + 1)
                        .sum::<usize>();
                match serde_json::from_str::<Value>(line) {
                    Ok(record) => {
                        for record in message_records(&record, i) {
                            let mut record = record;
                            record["offset"] = json!(line_offset);
                            records.push(record);
                        }
                    }
                    Err(_) => records.push(json!({
                        "index": i,
                        "offset": line_offset,
                        "kind": "unknown",
                        "unknown": true,
                        "raw": line,
                    })),
                }
            }
        }
        let mut data = json!({
            "path": name,
            "content_kind": content_kind,
            "offset": offset,
            "next_offset": if done { Value::Null } else { json!(end) },
            "done": done,
            "total_bytes": bytes.len(),
            "page_bytes": page.len(),
            "digest": digest,
            "source": {
                "archive_path": path,
                "job_id": r.get("job_id").cloned().unwrap_or(Value::Null),
                "package_digest": package_digest,
            },
        });
        data["raw_text"] = json!(raw_text);
        if content_kind == "messages" {
            data["records"] = Value::Array(records);
        }
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_thinking_never_enters_derived_text() {
        let record = json!({"type":"assistant","message":{"content":[
            {"type":"thinking","thinking":"secret chain","signature":"sig"},
            {"type":"text","text":"visible answer"}
        ]}});
        let records = message_records(&record, 0);
        let text: Vec<&str> = records
            .iter()
            .filter_map(|r| r.get("text").and_then(Value::as_str))
            .collect();
        assert_eq!(text, vec!["visible answer"]);
        assert!(records.iter().any(|r| r["opaque"] == true));
        assert!(!records.iter().any(|r| r["text"] == "secret chain"));
    }

    #[test]
    fn unknown_records_stay_visible() {
        let records = message_records(&json!({"type":"new-kind","payload":{"x":1}}), 3);
        assert_eq!(records[0]["kind"], "unknown");
        assert_eq!(records[0]["unknown"], true);
    }
}

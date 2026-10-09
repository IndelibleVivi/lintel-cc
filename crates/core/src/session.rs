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
    let timestamp = record.get("timestamp").cloned().unwrap_or(Value::Null);
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
        // Every source block emits one entry, including opaque/unknown blocks,
        // so hidden content never renumbers the original array positions.
        value["block_index"] = json!(out.len());
        out.push(value);
    };
    match raw_content {
        Some(blocks) => {
            for block in blocks {
                match block.get("type").and_then(Value::as_str) {
                    Some("text") => push(
                        &mut out,
                        json!({
                            "index": index,
                            "kind": kind,
                            "text": block["text"],
                        }),
                    ),
                    Some("tool_use") => push(
                        &mut out,
                        json!({
                            "index": index,
                            "kind": "tool_call",
                            "name": block["name"],
                            "tool": block["input"],
                        }),
                    ),
                    Some("tool_result") => push(
                        &mut out,
                        json!({
                            "index": index,
                            "kind": "tool_result",
                            "text": block["content"],
                        }),
                    ),
                    _ if is_opaque_block(block) => push(
                        &mut out,
                        json!({
                            "index": index,
                            "kind": kind,
                            "opaque": true,
                            "text": Value::Null,
                        }),
                    ),
                    _ => push(
                        &mut out,
                        json!({
                            "index": index,
                            "kind": "unknown",
                            "unknown": true,
                            "block": block,
                        }),
                    ),
                }
            }
        }
        None => {
            match content {
                Some(Value::String(text)) => push(
                    &mut out,
                    json!({
                        "index": index,
                        "kind": kind,
                        "text": text,
                    }),
                ),
                Some(Value::Object(block)) => out.extend(message_records(
                    &json!({"type": record["type"], "content": [block]}),
                    index,
                )),
                Some(other) => push(
                    &mut out,
                    json!({
                        "index": index,
                        "kind": kind,
                        "text": other.to_string(),
                    }),
                ),
                // No content at all: keep the record visible rather than dropping.
                None => push(
                    &mut out,
                    json!({
                        "index": index,
                        "kind": "unknown",
                        "unknown": true,
                        "record": record,
                    }),
                ),
            }
        }
    }
    out
}

impl Engine {
    /// Read a bounded byte window of one archived member file, honoring an
    /// optional caller-supplied `expected_digest` binding. Only the requested
    /// window (at most `PAGE_BYTES`) is ever held in memory, so a
    /// large transcript can be paged without loading the whole file.
    pub(crate) fn session_source(
        &self,
        r: &Value,
        offset: usize,
    ) -> Result<(PathBuf, Vec<u8>, u64, String, String)> {
        let opened = self.open_archive(r)?;
        let name = string(r, "path")?;
        let entry = opened.entry(name)?;
        if let Some(expected) = r.get("expected_digest").and_then(Value::as_str) {
            if expected != entry.digest {
                return Err(err(
                    "stale_archive",
                    "来源文件摘要与选择时不同；旧阅读位置不能映射到改变后的内容",
                ));
            }
        }
        let total = entry.bytes;
        if offset as u64 > total {
            return Err(err("invalid_offset", "偏移超出文件长度"));
        }
        // Read one bounded page directly from the verified staging file.
        let want = (PAGE_BYTES as u64).min(total.saturating_sub(offset as u64)) as usize;
        let mut window = vec![0u8; want];
        read_window(&entry.plain, offset as u64, &mut window)?;
        Ok((
            opened.path.clone(),
            window,
            total,
            entry.digest.clone(),
            opened.cipher_digest.clone(),
        ))
    }

    pub(crate) fn session_read(&self, r: &Value) -> Result<Value> {
        let offset = r.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
        let (path, window, total_bytes, digest, package_digest) = self.session_source(r, offset)?;
        let name = string(r, "path")?.to_string();
        let content_kind = if name.ends_with(".jsonl") {
            "messages"
        } else {
            "text"
        };
        // The window is a bounded slice; the number of bytes we actually keep
        // is `page_end` within it. For a JSONL transcript we advance only past
        // complete lines so the tail of an incomplete record is re-read next
        // page instead of being lost; for plain text we bound purely by bytes.
        let kept = window.len().min(PAGE_BYTES);
        let (end_in_window, complete) = if content_kind == "messages" {
            match window[..kept].iter().rposition(|b| *b == b'\n') {
                Some(last_newline) => (last_newline + 1, true),
                None if offset as u64 + kept as u64 >= total_bytes => (kept, true),
                None => {
                    // A single logical line longer than the read window; bound
                    // by bytes so the caller still makes progress.
                    if window.len() >= PAGE_BYTES {
                        (kept, false)
                    } else {
                        // Window is short only at EOF; the remainder is complete.
                        (kept, true)
                    }
                }
            }
        } else {
            (kept, true)
        };
        let end = offset + end_in_window;
        let done = end as u64 >= total_bytes;
        let page = &window[..end_in_window];
        let raw_text = String::from_utf8_lossy(page).into_owned();
        let mut records = vec![];
        if content_kind == "messages" && complete {
            let mut line_offset = offset;
            for (i, bytes) in page.split_inclusive(|b| *b == b'\n').enumerate() {
                let current_offset = line_offset;
                line_offset += bytes.len();
                let line = String::from_utf8_lossy(bytes);
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<Value>(&line) {
                    Ok(record) => {
                        for record in message_records(&record, i) {
                            let mut record = record;
                            record["offset"] = json!(current_offset);
                            records.push(record);
                        }
                    }
                    Err(_) => records.push(json!({
                        "index": i,
                        "offset": current_offset,
                        "block_index": 0,
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
            "total_bytes": total_bytes,
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

/// Read up to `buf.len()` bytes starting at `offset` from a staged plaintext
/// file, seeking directly so a large file is never fully read.
fn read_window(path: &std::path::Path, offset: u64, buf: &mut [u8]) -> Result<()> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut filled = 0;
    while filled < buf.len() {
        match file.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(err("archive_read_failed", "无法读取归档文件")),
        }
    }
    Ok(())
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
        assert_eq!(records[0]["block_index"], 0);
        assert_eq!(records[1]["block_index"], 1);
        assert!(records.iter().any(|r| r["opaque"] == true));
        assert!(!records.iter().any(|r| r["text"] == "secret chain"));
    }

    #[test]
    fn real_package_pages_keep_absolute_positions_and_text_shape() {
        use std::fs;
        let fixture = tempfile::tempdir().unwrap();
        let home = fixture.path().canonicalize().unwrap();
        let root = home.join("config");
        fs::create_dir_all(root.join("projects/p/memory")).unwrap();
        let first = format!(
            "{}\n",
            json!({"type":"user","message":{"content":"x".repeat(230000)}})
        );
        let second = format!(
            "{}\n",
            json!({"type":"assistant","message":{"content":[
                {"type":"thinking","thinking":"opaque private","signature":"sig"},
                {"type":"text","text":"y".repeat(50000)},
                {"type":"text","text":"visible block two"}
            ]}})
        );
        let original = format!("{first}{second}");
        fs::write(root.join("projects/p/s.jsonl"), &original).unwrap();
        fs::write(root.join("projects/p/memory/MEMORY.md"), "reference text\n").unwrap();
        let engine = Engine::new(home.clone(), home.join("state")).unwrap();
        let request = |r: Value| {
            let result = engine.request(r);
            assert_eq!(result["ok"], true, "{result}");
            result["data"].clone()
        };
        let environment = request(json!({"command":"register","root":root,"name":"synthetic"}));
        let plan = request(
            json!({"command":"plan_archive","environment_id":environment["id"],"categories":["memory","sessions"]}),
        );
        let receipt = request(
            json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"],"archive_passphrase":"synthetic-reader-passphrase"}),
        );
        let read = |path: &str, offset: usize| {
            request(
                json!({"command":"session_read","job_id":receipt["id"],"archive_passphrase":"synthetic-reader-passphrase","path":path,"offset":offset}),
            )
        };
        let text = read("projects/p/memory/MEMORY.md", 0);
        assert_eq!(text["content_kind"], "text");
        assert!(text.get("records").is_none());
        assert_eq!(text["raw_text"], "reference text\n");
        let one = read("projects/p/s.jsonl", 0);
        assert_eq!(one["next_offset"], first.len());
        assert_eq!(one["records"][0]["index"], 0);
        assert_eq!(one["records"][0]["offset"], 0);
        assert_eq!(one["records"][0]["block_index"], 0);
        let two = read("projects/p/s.jsonl", first.len());
        assert_eq!(two["records"][1]["index"], 0);
        assert_eq!(two["records"][1]["offset"], first.len());
        assert_eq!(two["records"][1]["block_index"], 1);
        assert_eq!(two["records"][2]["block_index"], 2);
        assert_eq!(two["records"][0]["opaque"], true);
        assert!(two["records"][0]["text"].is_null());
        assert_eq!(two["done"], true);
        assert_eq!(
            fs::read_to_string(root.join("projects/p/s.jsonl")).unwrap(),
            original
        );
    }

    #[test]
    fn unknown_records_stay_visible() {
        let records = message_records(&json!({"type":"new-kind","payload":{"x":1}}), 3);
        assert_eq!(records[0]["kind"], "unknown");
        assert_eq!(records[0]["unknown"], true);
    }
}

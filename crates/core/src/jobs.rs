//! Read journal records without allowing one malformed record to hide the rest.
use crate::{err, storage::load, Engine, Result};
use serde_json::{json, Value};
use std::os::unix::fs::MetadataExt;
use std::{fs, path::Path};

impl Engine {
    pub(crate) fn load_job(&self, path: &Path, expected_id: &str) -> Result<Value> {
        let record = load(path)?;
        // Validate fields used by reconciliation before mutating anything. Missing
        // evidence is an error, never authority to repair or replay a receipt.
        let valid = record.is_object()
            && record["id"].as_str() == Some(expected_id)
            && record["title"].is_string()
            && record["created_at"].is_string()
            && (record["environment_id"].is_null() || record["environment_id"].is_string())
            && matches!(
                record["status"].as_str(),
                Some(
                    "accepted"
                        | "executing"
                        | "verifying"
                        | "completed"
                        | "partially_completed"
                        | "failed"
                        | "needs_reconciliation"
                        | "interrupted"
                )
            )
            && record["warnings"]
                .as_array()
                .is_some_and(|items| items.iter().all(Value::is_string))
            && record["steps"].as_array().is_some_and(|items| {
                items.iter().all(|item| {
                    item.is_object()
                        && item["id"].is_string()
                        && item["status"].is_string()
                        && ["label", "message"]
                            .iter()
                            .all(|key| item.get(*key).is_none_or(Value::is_string))
                })
            });
        if !valid {
            return Err(err(
                "invalid_job_record",
                "原任务记录字段缺失或不受支持；保留原文件，不重放操作",
            ));
        }
        Ok(record)
    }

    pub(crate) fn list_jobs(&self) -> Result<Value> {
        let mut jobs = vec![];
        let mut errors = vec![];
        for entry in fs::read_dir(self.state.join("jobs"))? {
            let path = entry?.path();
            if !path.extension().is_some_and(|v| v == "json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|v| v.to_str()) else {
                errors.push(json!({"job_id":null,"code":"invalid_job_id"}));
                continue;
            };
            // Only finite IDs enter diagnostics; never expose arbitrary filenames.
            if uuid::Uuid::parse_str(stem).is_err() {
                errors.push(json!({"job_id":null,"code":"invalid_job_id"}));
                continue;
            }
            match self.load_job(&path, stem) {
                Ok(job) => jobs.push(job),
                Err(error) => errors.push(json!({"job_id":stem,"code":error.code})),
            }
        }
        jobs.sort_by(|a, b| {
            b["created_at"]
                .as_str()
                .cmp(&a["created_at"].as_str())
                .then_with(|| a["id"].as_str().cmp(&b["id"].as_str()))
        });
        errors.sort_by(|a, b| a["job_id"].as_str().cmp(&b["job_id"].as_str()));
        Ok(json!({"jobs":jobs,"records_complete":errors.is_empty(),"record_errors":errors}))
    }
}

// Read-only views are deliberately separate from the legacy full-receipt RPCs.
// Journal files remain the execution/deduplication authority; no index is written.
const PAGE_BYTES: usize = 256 * 1024;
const PAGE_ROWS: usize = 100;
const SCAN_FILES: usize = 100_000;
fn bounded_page(value: Value) -> Result<Value> {
    if serde_json::to_vec(&json!({"ok":true,"data":value}))?.len() > PAGE_BYTES {
        return Err(err(
            "history_page_too_large",
            "历史页超过 256 KiB；原记录保留",
        ));
    }
    Ok(value)
}
fn page_args(r: &Value) -> Result<(usize, usize)> {
    let offset = r
        .get("offset")
        .map_or(Some(0), Value::as_u64)
        .filter(|n| *n <= 9007199254740991)
        .ok_or_else(|| err("invalid_offset", "需要有限非负 offset"))? as usize;
    let limit = r
        .get("limit")
        .map_or(Some(50), Value::as_u64)
        .filter(|n| *n > 0 && *n <= PAGE_ROWS as u64)
        .ok_or_else(|| err("invalid_limit", "limit 需要 1–100"))? as usize;
    if offset > 0
        && !r["expected_digest"]
            .as_str()
            .is_some_and(lintel_operations::valid_plan_hash)
    {
        return Err(err("invalid_history_cursor", "后续页需要原摘要"));
    }
    Ok((offset, limit))
}
fn bind_page(r: &Value, digest: &str) -> Result<()> {
    if let Some(expected) = r.get("expected_digest") {
        if expected.as_str() != Some(digest) {
            return Err(err("stale_history", "历史记录已变化；请从第一页重新读取"));
        }
    }
    Ok(())
}
fn confirmed_archive(job: &Value) -> bool {
    job["archive_path"].as_str().is_some_and(|s| !s.is_empty())
        && (job["archive_digest"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
            || job["steps"].as_array().is_some_and(|steps| {
                steps
                    .iter()
                    .any(|s| s["id"] == "archive" && s["status"] == "completed")
            }))
}
fn short(value: &Value, max: usize) -> Value {
    Value::String(value.as_str().unwrap_or("").chars().take(max).collect())
}
impl Engine {
    // Enumerate metadata, never read every receipt to obtain the first page.
    fn history_snapshot(
        &self,
    ) -> Result<(Vec<(String, std::path::PathBuf, u128, u64, String)>, String)> {
        let mut rows = Vec::new();
        for entry in fs::read_dir(self.state.join("jobs"))? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_none_or(|v| v != "json") {
                continue;
            }
            if rows.len() >= SCAN_FILES {
                return Err(err(
                    "history_scan_limit",
                    "历史元数据超过 100000 项；未删除记录",
                ));
            }
            let id = path
                .file_stem()
                .and_then(|v| v.to_str())
                .unwrap_or("")
                .to_owned();
            let meta = fs::symlink_metadata(&path)?;
            let modified = meta
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            rows.push((
                id,
                path,
                modified,
                meta.len(),
                format!(
                    "{}:{}:{}:{}:{}",
                    meta.dev(),
                    meta.ino(),
                    meta.ctime(),
                    meta.ctime_nsec(),
                    meta.mode()
                ),
            ));
        }
        rows.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
        let metadata: Vec<_> = rows
            .iter()
            .map(|(id, _, time, size, identity)| json!([id, time.to_string(), size, identity]))
            .collect();
        let digest = crate::storage::digest(&serde_json::to_vec(&metadata)?);
        Ok((rows, digest))
    }
    pub(crate) fn jobs_page(&self, r: &Value) -> Result<Value> {
        let (offset, limit) = page_args(r)?;
        let filter = r.get("filter").and_then(Value::as_str).unwrap_or("all");
        if !["all", "attention", "archives", "network"].contains(&filter) {
            return Err(err("invalid_history_filter", "不支持的历史筛选"));
        }
        let query = r.get("query").and_then(Value::as_str).unwrap_or("");
        if query.chars().count() > 256 || r.get("query").is_some_and(|v| !v.is_string()) {
            return Err(err("invalid_history_query", "搜索需要至多 256 个字符"));
        }
        let query = query.to_lowercase();
        let environments = if query.is_empty() {
            Vec::new()
        } else {
            self.inventory()?
        };
        let (rows, metadata_digest) = self.history_snapshot()?;
        let digest = crate::storage::digest(&serde_json::to_vec(&json!([
            metadata_digest,
            filter,
            r.get("environment_id"),
            query,
            environments
        ]))?);
        bind_page(r, &digest)?;
        if offset > rows.len() {
            return Err(err("invalid_offset", "offset 超过历史快照"));
        }
        let mut jobs = Vec::new();
        let mut errors = Vec::new();
        let mut index = offset;
        // The cursor counts scanned candidates, including filtered/corrupt ones.
        // An empty page with a cursor is not an assertion of no matching jobs.
        while index < rows.len() && index - offset < limit {
            let (id, path, modified, _, _) = &rows[index];
            index += 1;
            if uuid::Uuid::parse_str(id).is_err() {
                errors.push(json!({"job_id":null,"code":"invalid_job_id"}));
                continue;
            }
            match self.load_job(path, id) {
                Err(e) => errors.push(json!({"job_id":id,"code":e.code})),
                Ok(job) => {
                    if filter == "attention"
                        && ![
                            "accepted",
                            "executing",
                            "verifying",
                            "interrupted",
                            "needs_reconciliation",
                            "failed",
                            "partially_completed",
                        ]
                        .iter()
                        .any(|s| job["status"] == *s)
                    {
                        continue;
                    }
                    if filter == "network" && !job["network_change"].is_object() {
                        continue;
                    }
                    if filter == "archives" && !confirmed_archive(&job) {
                        continue;
                    }
                    if let Some(eid) = r.get("environment_id") {
                        if job["environment_id"] != *eid && !job["network_change"].is_object() {
                            continue;
                        }
                    }
                    if !query.is_empty() {
                        let name = environments
                            .iter()
                            .find(|env| env["id"] == job["environment_id"])
                            .and_then(|env| env["name"].as_str())
                            .unwrap_or("");
                        if ![id.as_str(), job["title"].as_str().unwrap_or(""), name]
                            .iter()
                            .any(|s| s.to_lowercase().contains(&query))
                        {
                            continue;
                        }
                    }
                    jobs.push(json!({"id":id,"title":short(&job["title"],512),"title_truncated":job["title"].as_str().unwrap_or("").chars().count()>512,
                        "created_at":short(&job["created_at"],128),"updated_at_ns":modified.to_string(),"environment_id":if job["environment_id"].is_string(){short(&job["environment_id"],128)}else{Value::Null},"status":job["status"],
                        "host_shared_network":job["network_change"].is_object(),"has_work_archive":confirmed_archive(&job),"restorable":job["restorable"]==true,"service_restorable":job["service_restorable"]==true}));
                }
            }
            if serde_json::to_vec(&jobs)?.len() + serde_json::to_vec(&errors)?.len()
                > PAGE_BYTES - 4096
            {
                jobs.pop();
                index -= 1;
                break;
            }
        }
        if self.history_snapshot()?.1 != metadata_digest {
            return Err(err("stale_history", "读取期间历史已变化；请重读"));
        }
        bounded_page(
            json!({"jobs":jobs,"record_errors":errors,"records_complete":errors.is_empty(),"digest":digest,"offset":offset,"next_offset":if index<rows.len(){Some(index)}else{None},"scan_complete":index==rows.len(),"scanned":index-offset,"total_candidates":rows.len(),"order":"updated_desc_id"}),
        )
    }
    pub(crate) fn job_detail_page(&self, r: &Value) -> Result<Value> {
        let (offset, limit) = page_args(r)?;
        let id = crate::safe_id(r, "job_id")?;
        let path = self.path("jobs", &id);
        if !path.exists() {
            return Err(err("job_not_found", "原任务不存在；未重放操作"));
        }
        let job = self.load_job(&path, &id)?;
        let digest = crate::storage::digest(&serde_json::to_vec(&job)?);
        bind_page(r, &digest)?;
        let section = r.get("section").and_then(Value::as_str).unwrap_or("header");
        let items: Vec<Value> = match section {
            "header" => vec![
                json!({"id":id,"title":short(&job["title"],512),"primary":short(&job["task_result"]["primary"],1024),"created_at":short(&job["created_at"],128),"environment_id":if job["environment_id"].is_string(){short(&job["environment_id"],128)}else{Value::Null},"status":job["status"],"outcome":short(&job["outcome"],256),"restorable":job["restorable"]==true && job["network_change"].is_null(),"service_restorable":job["service_restorable"]==true,"network_restorable":job["restorable"]==true && job["network_change"].is_object(),"has_work_archive":confirmed_archive(&job),"new_environment_id":if job["new_environment_id"].is_string(){short(&job["new_environment_id"],128)}else{Value::Null}}),
            ],
            "receipt" => vec![job.clone()],
            "next_steps" => job[section].as_array().cloned().unwrap_or_default(),
            "result_coverage" => job["task_result"]["coverage"]
                .as_array()
                .cloned()
                .unwrap_or_default(),
            "next_actions" => job["task_result"]["next_actions"]
                .as_array()
                .cloned()
                .unwrap_or_default(),
            "steps" | "warnings" => job[section].as_array().cloned().unwrap_or_default(),
            "coverage" | "task_result" | "error" => match job.get(section) {
                Some(Value::Array(v)) => v.clone(),
                Some(Value::Object(v)) => v
                    .iter()
                    .map(|(key, value)| json!({"key":key,"value":value}))
                    .collect(),
                Some(v) => vec![v.clone()],
                None => vec![],
            },
            _ => return Err(err("invalid_history_section", "不支持的详情 section")),
        };
        if offset > items.len() {
            return Err(err("invalid_offset", "offset 超过详情 section"));
        }
        let mut page = Vec::new();
        let mut index = offset;
        while index < items.len() && page.len() < limit {
            let item = &items[index];
            if serde_json::to_vec(item)?.len() > PAGE_BYTES - 4096 {
                if page.is_empty() {
                    return Err(err("detail_item_too_large",&format!("section={section} index={index} 超过详情页预算；原记录保留，恢复仍使用原 ID")));
                }
                break;
            }
            page.push(item.clone());
            if serde_json::to_vec(&page)?.len() > PAGE_BYTES - 4096 {
                page.pop();
                break;
            }
            index += 1;
        }
        bounded_page(
            json!({"job_id":id,"section":section,"items":page,"digest":digest,"next_offset":if index<items.len(){Some(index)}else{None},"total_items":items.len(),"complete":index==items.len()}),
        )
    }
}

#[cfg(test)]
mod page_tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Engine) {
        let base = std::env::var_os("LINTEL_HISTORY_TEST_ROOT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let temp = tempfile::tempdir_in(base).unwrap();
        let home = temp.path().join("home");
        fs::create_dir(&home).unwrap();
        let engine = Engine::new(home, temp.path().join("state")).unwrap();
        (temp, engine)
    }
    fn record(id: &str, padding: usize) -> Value {
        json!({"id":id,"title":"synthetic","created_at":"2026-01-01","environment_id":null,"status":"completed","warnings":[],"steps":[],"coverage":{"large":"x".repeat(padding)}})
    }
    #[test]
    fn history_large_records_are_summaries_and_detail_is_bounded() {
        let (_temp, engine) = fixture();
        let mut legacy = vec![];
        for n in 1..=128 {
            let id = format!("00000000-0000-4000-8000-{n:012}");
            let job = record(&id, 16 * 1024);
            crate::storage::save(&engine.path("jobs", &id), &job).unwrap();
            legacy.push(job);
        }
        assert!(serde_json::to_vec(&legacy).unwrap().len() > 2 * 1024 * 1024);
        let page = engine.jobs_page(&json!({"limit":100})).unwrap();
        assert_eq!(page["jobs"].as_array().unwrap().len(), 100);
        assert!(serde_json::to_vec(&page).unwrap().len() < PAGE_BYTES);
        assert!(!page["scan_complete"].as_bool().unwrap());
        let tail = engine
            .jobs_page(&json!({"offset":100,"expected_digest":page["digest"]}))
            .unwrap();
        assert_eq!(tail["jobs"].as_array().unwrap().len(), 28);
        let id = "00000000-0000-4000-8000-000000000001";
        crate::storage::save(&engine.path("jobs", id), &record(id, 3 * 1024 * 1024)).unwrap();
        assert_eq!(
            engine
                .jobs_page(&json!({"offset":100,"expected_digest":page["digest"]}))
                .unwrap_err()
                .code,
            "stale_history"
        );
        let head = engine.job_detail_page(&json!({"job_id":id})).unwrap();
        assert!(serde_json::to_vec(&head).unwrap().len() < 4096);
        assert_eq!(
            engine
                .job_detail_page(&json!({"job_id":id,"section":"coverage"}))
                .unwrap_err()
                .code,
            "detail_item_too_large"
        );
        assert!(
            engine.load_job(&engine.path("jobs", id), id).unwrap()["coverage"]["large"]
                .as_str()
                .unwrap()
                .len()
                > 2 * 1024 * 1024
        );
    }
    #[test]
    fn filters_advance_across_bad_and_nonmatching_candidates() {
        let (_temp, engine) = fixture();
        let id = "00000000-0000-4000-8000-000000000001";
        crate::storage::save(&engine.path("jobs", id), &record(id, 0)).unwrap();
        let page = engine.jobs_page(&json!({"filter":"attention"})).unwrap();
        assert!(page["jobs"].as_array().unwrap().is_empty());
        assert_eq!(page["scan_complete"], true);
        fs::write(engine.path("jobs", id), b"broken").unwrap();
        let page = engine.jobs_page(&json!({})).unwrap();
        assert_eq!(page["record_errors"][0]["job_id"], id);
        assert_eq!(page["records_complete"], false);
        assert_eq!(
            engine.jobs_page(&json!({"offset":1})).unwrap_err().code,
            "invalid_history_cursor"
        );
    }
    #[test]
    fn unicode_budget_search_and_detail_cursor_preserve_original_journal() {
        let (_temp, engine) = fixture();
        for n in 1..=100 {
            let id = format!("00000000-0000-4000-8000-{n:012}");
            let mut job = record(&id, 0);
            job["title"] = json!("𠀀".repeat(800));
            crate::storage::save(&engine.path("jobs", &id), &job).unwrap();
        }
        let page = engine.jobs_page(&json!({"limit":100})).unwrap();
        assert!(
            serde_json::to_vec(&json!({"ok":true,"data":page}))
                .unwrap()
                .len()
                <= PAGE_BYTES
        );
        let no_match = engine
            .jobs_page(&json!({"limit":1,"query":"not-present"}))
            .unwrap();
        assert_eq!(no_match["jobs"], json!([]));
        assert!(!no_match["next_offset"].is_null());
        assert_eq!(engine.jobs_page(&json!({"limit":1,"offset":1,"expected_digest":no_match["digest"],"query":"changed"})).unwrap_err().code,"stale_history");
        let id = "00000000-0000-4000-8000-000000000001";
        let mut job = record(id, 0);
        job["status"] = json!("accepted");
        job["warnings"] = json!(["one", "two"]);
        crate::storage::save(&engine.path("jobs", id), &job).unwrap();
        let before = fs::read(engine.path("jobs", id)).unwrap();
        let first = engine
            .job_detail_page(&json!({"job_id":id,"section":"warnings","limit":1}))
            .unwrap();
        assert_eq!(first["items"], json!(["one"]));
        let second=engine.job_detail_page(&json!({"job_id":id,"section":"warnings","offset":1,"expected_digest":first["digest"]})).unwrap();
        assert_eq!(second["items"], json!(["two"]));
        let full = engine
            .job_detail_page(&json!({"job_id":id,"section":"receipt"}))
            .unwrap();
        assert_eq!(full["items"][0], job);
        assert_eq!(fs::read(engine.path("jobs", id)).unwrap(), before);
    }
    fn scale(count: usize) {
        let (_temp, engine) = fixture();
        for n in 1..=count {
            let id = format!("00000000-0000-4000-8000-{n:012}");
            fs::write(
                engine.path("jobs", &id),
                serde_json::to_vec(&record(&id, 0)).unwrap(),
            )
            .unwrap();
        }
        let started = std::time::Instant::now();
        let page = engine.jobs_page(&json!({"limit":50})).unwrap();
        assert_eq!(page["total_candidates"], count);
        assert_eq!(page["jobs"].as_array().unwrap().len(), 50);
        eprintln!(
            "history {count} candidates first page {:?}, {} bytes",
            started.elapsed(),
            serde_json::to_vec(&page).unwrap().len()
        );
    }
    #[test]
    #[ignore = "explicit metadata scale evidence"]
    fn history_metadata_10000() {
        scale(10000)
    }
    #[test]
    #[ignore = "independent manual 100k metadata benchmark"]
    fn history_metadata_100000() {
        scale(100000)
    }
    #[test]
    fn target_filter_includes_host_shared_network_but_not_other_environments() {
        let (_temp, engine) = fixture();
        let target = "10000000-0000-4000-8000-000000000001";
        for n in 1..=4 {
            let id = format!("00000000-0000-4000-8000-{n:012}");
            let mut job = record(&id, 0);
            job["environment_id"] = if n == 1 {
                json!(target)
            } else if n == 2 {
                json!("20000000-0000-4000-8000-000000000001")
            } else {
                Value::Null
            };
            if n == 3 {
                job["network_change"] = json!({"synthetic":true});
            }
            crate::storage::save(&engine.path("jobs", &id), &job).unwrap();
        }
        let page = engine.jobs_page(&json!({"environment_id":target})).unwrap();
        let ids: Vec<_> = page["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|j| j["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"00000000-0000-4000-8000-000000000001"));
        assert!(ids.contains(&"00000000-0000-4000-8000-000000000003"));
    }
}

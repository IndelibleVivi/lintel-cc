//! Read journal records without allowing one malformed record to hide the rest.
use crate::{err, storage::load, Engine, Result};
use serde_json::{json, Value};
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

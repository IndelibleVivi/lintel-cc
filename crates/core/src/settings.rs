//! Recover only the settings publication that this task can identify.
use crate::{err, json, storage::*, string, Engine, Result, Value};
use std::{fs, os::unix::fs::MetadataExt, path::Path};

impl Engine {
    pub(crate) fn settings_plan_environment(&self, plan: &Value) -> Result<Value> {
        let environment = self.env(&json!({"environment_id":plan["environment_id"]}))?;
        if environment["root"] != plan["root"] {
            return Err(err("stale_plan", "环境目标已经变化"));
        }
        let root = Path::new(string(&environment, "root")?);
        guard(root)?;
        let metadata = fs::metadata(root)?;
        if json!([metadata.dev(), metadata.ino()]) != plan["root_identity"] {
            return Err(err("stale_plan", "配置目录的实际对象已变化"));
        }
        Ok(environment)
    }

    /// A prepared intent alone confers no ownership. Only an exact match with
    /// the inode staged before publication proves this task's write. This is
    /// deliberately stricter than restoring an already witnessed write: an
    /// external rewrite before reconciliation cannot be attributed to us.
    pub(crate) fn reconcile_settings_write(&self, job: &mut Value) -> bool {
        if job["settings_write"]["state"] != "prepared" {
            return false;
        }
        let result = (|| -> Result<&str> {
            let plan = load(&self.path("plans", string(job, "plan_id")?))?;
            let mut unhashed = plan.clone();
            unhashed
                .as_object_mut()
                .ok_or_else(|| err("invalid_plan", "保存的计划损坏"))?
                .remove("hash");
            if plan["hash"] != digest(&serde_json::to_vec(&unhashed)?)
                || plan["hash"] != job["settings_write"]["plan_hash"]
                || plan["snapshot"] != job["settings_write"]["before"]
            {
                return Err(err("plan_changed", "保存的计划与写入意图不一致"));
            }
            let environment = self.settings_plan_environment(&plan)?;
            let current =
                snapshot(&Path::new(string(&environment, "root")?).join("settings.json"))?;
            if current == job["settings_write"]["after"] {
                Ok("written")
            } else if current == job["settings_write"]["before"] {
                Ok("not_written")
            } else {
                Ok("ownership_unproven")
            }
        })();
        let (state, reason, message) = match result {
            Ok("written") => ("written", "published_object_matches", "当前配置与本任务写前冻结的发布对象、字节及权限一致；可另行预览精确字段恢复。"),
            Ok("not_written") => ("not_written", "original_object_unchanged", "当前配置仍为写前对象；没有本任务已发布的证据，不提供此任务的恢复。"),
            Ok(_) => ("ownership_unproven", "settings_changed", "配置已由其他对象或字节取代，无法证明中断写入的归属；保留当前配置，不提供此任务的恢复。"),
            Err(ref failure) => ("ownership_unproven", failure.code.as_str(), "原计划或目标身份无法核对；保留当前配置，不提供此任务的恢复。"),
        };
        job["settings_write"]["state"] = json!(state);
        job["settings_recovery"] = json!({"state":state,"reason":reason,"message":message});
        job["restorable"] = json!(state == "written");
        true
    }
}

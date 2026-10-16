//! A finite composition of the existing config/identity/residue steps. One
//! approval and one durable original job; interruption never re-enters a step.
use crate::{err, json, policy, storage::*, string, Engine, Result, Value};
use std::{collections::BTreeSet, path::Path};

#[cfg(test)]
fn phase_hook(phase: &str) {
    if std::env::var("LINTEL_WORKFLOW_TEST_STOP").as_deref() == Ok(phase) {
        unsafe { libc::_exit(74) }
    }
}

pub(crate) fn config_step(plan: &Value) -> Option<Value> {
    let mut step = plan["extra"]["workflow"]
        .as_array()?
        .iter()
        .find(|p| p["kind"] == "config_edit")?
        .clone();
    // Publication/recovery belongs to the original composite job.
    step["id"] = plan["id"].clone();
    step["hash"] = plan["hash"].clone();
    Some(step)
}

impl Engine {
    pub(crate) fn plan_client_workflow(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        if r["writers_confirmed_stopped"] != true {
            return Err(err(
                "writers_confirmation_required",
                "先确认目标终端、IDE 与自动重启来源已停止写入",
            ));
        }
        let mut steps = vec![];
        if let Some(policy) = r.get("policy") {
            let mut request = policy.clone();
            if !request.is_object() {
                return Err(err("invalid_request", "policy 必须是有限方案对象"));
            }
            request["environment_id"] = e["id"].clone();
            request["config_path"] = json!(Path::new(string(&e, "root")?).join("settings.json"));
            let public = self.plan_config_policy(&request)?;
            steps.push(load(&self.path("plans", string(&public, "id")?))?);
        }
        if r["reset_identity"] == true {
            let mut request = r.clone();
            request.as_object_mut().unwrap().remove("policy");
            let public = self.plan_identity_reset(&request)?;
            steps.push(load(&self.path("plans", string(&public, "id")?))?);
        }
        if r["residue_paths"].as_array().is_some_and(|p| !p.is_empty()) {
            let public = self.plan_clear_residue(r)?;
            steps.push(load(&self.path("plans", string(&public, "id")?))?);
        }
        if steps.is_empty() {
            return Err(err("empty_workflow", "请选择至少一项配置、残留或标识操作"));
        }
        // A file can belong to only one selected mutator in this preview.
        let mut paths = BTreeSet::new();
        let mut changes = vec![];
        let mut actions = vec![];
        let mut file_count = 0;
        for step in &steps {
            let extra = &step["extra"];
            let mut affected = vec![];
            if step["kind"] == "config_edit" {
                affected.push(extra["config_path"].clone());
                file_count += 1;
            }
            if extra["recipe"] == "identity_reset" {
                affected.push(extra["identity"]["path"].clone());
                affected.extend(
                    extra["backups"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|f| f["path"].clone()),
                );
                file_count += affected.len();
            }
            if extra["recipe"] == "clear_residue" {
                affected.extend(
                    extra["files"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|f| f["path"].clone()),
                );
                file_count += affected.len();
            }
            for path in affected {
                if !paths.insert(
                    path.as_str()
                        .ok_or_else(|| err("invalid_plan", "缺少准确目标路径"))?
                        .to_owned(),
                ) {
                    return Err(err(
                        "overlapping_actions",
                        "同一文件不能同时属于配置、身份和残留修改；请移除重叠选择",
                    ));
                }
            }
            changes.extend(step["changes"].as_array().into_iter().flatten().cloned());
            actions.extend(step["actions"].as_array().into_iter().flatten().cloned());
        }
        let public = self.plan(
            &e,
            "client_workflow",
            "应用所选客户端控制",
            json!(changes),
            vec![
                "未选中的设置与文件",
                "登录凭据与官方登录状态（本配方不注销）",
                "其他环境、服务和浏览器 profile",
            ],
            json!(actions),
            json!({"workflow":steps,"file_count":file_count,"archive_passphrase_required":true}),
        )?;
        // Retain the preview evidence but prohibit separately executing its
        // internal constituents, even if someone later opens their plan record.
        for step in &steps {
            let mut step = step.clone();
            step["internal_parent_id"] = public["id"].clone();
            step.as_object_mut().unwrap().remove("hash");
            step["hash"] = json!(digest(&serde_json::to_vec(&step)?));
            save(&self.path("plans", string(&step, "id")?), &step)?;
        }
        Ok(public)
    }

    pub(crate) fn check_client_workflow(&self, e: &Value, p: &Value, r: &Value) -> Result<()> {
        crate::work::check_passphrase(r)?;
        for step in p["extra"]["workflow"]
            .as_array()
            .ok_or_else(|| err("invalid_plan", "组合步骤缺失"))?
        {
            if step["kind"] == "config_edit" {
                let (_, _, snap) = self.config_plan_settings(e, step)?;
                if snap != step["snapshot"] {
                    return Err(err("stale_plan", "组合计划的配置来源已经变化"));
                }
                if step["rule_version"] != policy::RULE {
                    return Err(err("rule_changed", "规则已更新，请重新预览"));
                }
                if step["extra"]["policy"]["product"]
                    != policy::product(self.executable().as_deref())
                {
                    return Err(err("stale_product", "客户端版本已经变化"));
                }
            } else if step["extra"]["recipe"] == "identity_reset" {
                self.check_identity_reset(step, r)?;
            } else if step["extra"]["recipe"] == "clear_residue" {
                self.check_residue_clear(step, r)?;
            } else {
                return Err(err("invalid_plan_kind", "组合包含不支持的步骤"));
            }
        }
        Ok(())
    }

    pub(crate) fn execute_client_workflow(
        &self,
        e: &Value,
        p: &Value,
        r: &Value,
        j: &mut Value,
        journal: &Path,
    ) -> Result<()> {
        self.check_client_workflow(e, p, r)?;
        let mut partial = false;
        for step in p["extra"]["workflow"].as_array().unwrap() {
            j["workflow_phase"] =
                json!({"kind":step["kind"],"recipe":step["extra"]["recipe"],"state":"executing"});
            save(journal, j)?;
            if step["kind"] == "config_edit" {
                self.execute_config_edit(e, &config_step(p).unwrap(), r, j, journal)?;
            } else if step["extra"]["recipe"] == "identity_reset" {
                self.identity_reset(e, step, r, j, journal)?;
            } else {
                self.clear_residue(e, step, r, j, journal)?;
            }
            partial |= j["status"] == "partially_completed";
            j["workflow_phase"]["state"] = json!("completed");
            j["status"] = json!("executing");
            save(journal, j)?;
            #[cfg(test)]
            phase_hook(if step["kind"] == "config_edit" {
                "configuration"
            } else {
                "data"
            });
        }
        j["status"] = json!(if partial {
            "partially_completed"
        } else {
            "completed"
        });
        j["outcome"] = json!("client_workflow");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    const PASS: &str = "synthetic-workflow-passphrase";
    fn fixture() -> (tempfile::TempDir, Engine, Value, std::path::PathBuf) {
        let t = tempfile::tempdir().unwrap();
        let b = t.path().canonicalize().unwrap();
        let home = b.join("home");
        let root = home.join("root");
        fs::create_dir_all(root.join("debug")).unwrap();
        save(
            &root.join("settings.json"),
            &json!({"env":{"OTEL_LOG_USER_PROMPTS":"1","UNRELATED":"keep"}}),
        )
        .unwrap();
        save(&root.join(".claude.json"),&json!({"userID":"synthetic-private-id","oauthAccount":{"email":"synthetic-private-account"},"keep":"untouched"})).unwrap();
        fs::write(root.join("debug/selected"), b"synthetic-private-log").unwrap();
        fs::write(root.join("debug/kept"), b"kept").unwrap();
        fs::write(
            root.join(".credentials.json"),
            b"synthetic-private-credential",
        )
        .unwrap();
        let mut engine = Engine::new(home, b.join("state")).unwrap();
        engine.executable_search_path = Some(std::ffi::OsString::new());
        let e = engine.register("synthetic", &root, false).unwrap();
        (t, engine, e, root)
    }
    fn preview(engine: &Engine, e: &Value, root: &Path) -> Value {
        let r=engine.request(json!({"command":"plan_client_workflow","environment_id":e["id"],"writers_confirmed_stopped":true,"policy":{"preset":"custom","keep_remote_control":true,"custom_settings":{"OTEL_LOG_USER_PROMPTS":"disable"}},"reset_identity":true,"residue_paths":[root.join("debug/selected")]}));
        assert_eq!(r["ok"], true, "{r}");
        r["data"].clone()
    }
    #[test]
    fn one_approval_binds_all_steps_preserves_unselected_and_never_replays() {
        let (_t, engine, e, root) = fixture();
        let p = preview(&engine, &e, &root);
        assert_eq!(p["workflow"].as_array().unwrap().len(), 3);
        for needle in [
            "synthetic-private-id",
            "synthetic-private-account",
            "synthetic-private-log",
            "synthetic-private-credential",
        ] {
            assert!(!p.to_string().contains(needle));
        }
        assert!(root.join("debug/selected").exists());
        let job=engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"],"archive_passphrase":PASS}));
        assert_eq!(job["data"]["status"], "completed", "{job}");
        assert!(!root.join("debug/selected").exists());
        assert_eq!(fs::read(root.join("debug/kept")).unwrap(), b"kept");
        assert_eq!(
            fs::read(root.join(".credentials.json")).unwrap(),
            b"synthetic-private-credential"
        );
        assert!(load(&root.join(".claude.json"))
            .unwrap()
            .get("userID")
            .is_none());
        assert_eq!(
            load(&root.join(".claude.json")).unwrap()["keep"],
            "untouched"
        );
        assert_eq!(
            load(&root.join("settings.json")).unwrap()["env"]["OTEL_LOG_USER_PROMPTS"],
            "0"
        );
        for key in [
            "config_archive_path",
            "identity_archive_path",
            "residue_archive_path",
        ] {
            assert!(
                Path::new(job["data"][key].as_str().unwrap()).exists(),
                "{key}"
            );
        }
        fs::write(root.join("debug/selected"), b"new-external-file").unwrap();
        let repeat =
            engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}));
        assert_eq!(repeat["data"]["id"], p["id"]);
        assert_eq!(
            fs::read(root.join("debug/selected")).unwrap(),
            b"new-external-file"
        );
        let restore=engine.request(json!({"command":"plan_config_restore","environment_id":e["id"],"job_id":p["id"],"selected_keys":["/env/OTEL_LOG_USER_PROMPTS"]}));
        assert_eq!(restore["ok"], true, "{restore}");
    }
    #[test]
    fn overlapping_targets_are_rejected_before_execution() {
        let (_t, engine, e, root) = fixture();
        fs::write(root.join(".claude.json.backup"), b"synthetic-backup").unwrap();
        let r=engine.request(json!({"command":"plan_client_workflow","environment_id":e["id"],"writers_confirmed_stopped":true,"reset_identity":true,"identity_backup_paths":[root.join(".claude.json.backup")],"residue_paths":[root.join(".claude.json.backup")]}));
        assert_eq!(r["error"]["code"], "overlapping_actions");
        // A mixed identity file is refused even earlier by the residue owner.
        let r=engine.request(json!({"command":"plan_client_workflow","environment_id":e["id"],"writers_confirmed_stopped":true,"reset_identity":true,"residue_paths":[root.join(".claude.json")]}));
        assert_eq!(r["error"]["code"], "protected_local_item");
    }
    #[test]
    fn crash_child() {
        let Ok(base) = std::env::var("LINTEL_WORKFLOW_TEST_BASE") else {
            return;
        };
        let b = std::path::PathBuf::from(base);
        let engine = Engine::new(b.join("home"), b.join("state")).unwrap();
        let id = std::env::var("LINTEL_WORKFLOW_TEST_PLAN").unwrap();
        let p = load(&engine.path("plans", &id)).unwrap();
        let _=engine.request(json!({"command":"execute","plan_id":id,"approval":p["hash"],"archive_passphrase":PASS}));
        panic!("did not stop at fault barrier");
    }
    #[test]
    fn interrupted_batch_retains_original_id_without_deleting_later_steps() {
        let (t, engine, e, root) = fixture();
        let p = preview(&engine, &e, &root);
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "client_workflow::tests::crash_child",
                "--nocapture",
            ])
            .env(
                "LINTEL_WORKFLOW_TEST_BASE",
                t.path().canonicalize().unwrap(),
            )
            .env("LINTEL_WORKFLOW_TEST_PLAN", p["id"].as_str().unwrap())
            .env("LINTEL_WORKFLOW_TEST_STOP", "configuration")
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(74));
        let q = engine.request(json!({"command":"job","job_id":p["id"]}));
        assert_eq!(q["data"]["status"], "needs_reconciliation", "{q}");
        assert_eq!(q["data"]["restorable"], true);
        let q = engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}));
        assert_eq!(q["data"]["status"], "needs_reconciliation");
        assert!(root.join("debug/selected").exists());
        assert!(load(&root.join(".claude.json"))
            .unwrap()
            .get("userID")
            .is_some());
    }
}

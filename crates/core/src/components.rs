//! Readonly, finite component coverage for one exact registered environment.
//! Presence and historical receipt evidence never attest runtime isolation.
#[path = "startup.rs"]
pub(crate) mod startup;

use crate::{err, now, policy, storage::*, string, Engine, Result};
use serde_json::{json, Value};
use std::{collections::BTreeSet, fs, path::Path};

fn presence(path: &Path) -> Value {
    let status = match guard(path).and_then(|_| match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() => Ok("observed"),
        Ok(meta) if meta.is_dir() => Ok("observed_directory"),
        Ok(_) => Ok("unsupported_type"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok("not_found"),
        Err(_) => Err(err("path_unreadable", "无法读取元数据")),
    }) {
        Ok(status) => status,
        Err(_) => "access_limited",
    };
    json!({"path":path,"state":status})
}

fn item(
    id: &str,
    title: &str,
    state: &str,
    source: &str,
    scope: &str,
    detail: &str,
    facts: Value,
    action: &str,
) -> Value {
    json!({"id":id,"title":title,"state":state,"source":source,"scope":scope,"detail":detail,"facts":facts,"next_action":action})
}

impl Engine {
    pub(crate) fn inspect_components(&self, r: &Value) -> Result<Value> {
        let environment = self.env(r)?;
        let root = Path::new(string(&environment, "root")?);
        guard(root)?;
        if !fs::metadata(root).is_ok_and(|m| m.is_dir()) {
            return Err(err("root_missing", "已登记配置根不存在或不可访问"));
        }
        let project = match r.get("project_cwd") {
            None => None,
            Some(Value::String(raw)) => {
                let path = Path::new(raw);
                guard(path)?;
                if !fs::metadata(path).is_ok_and(|m| m.is_dir()) {
                    return Err(err(
                        "project_missing",
                        "项目目录不存在或不可访问；未创建目录",
                    ));
                }
                Some(path)
            }
            Some(_) => return Err(err("invalid_request", "project_cwd 必须是明确绝对目录")),
        };
        let checked_at = now();
        let executable = self.executable();
        let cli = policy::product(executable.as_deref());
        let observed = startup::snapshot(&self.home, root, project, "readonly-components")?;
        let sources = observed.view["sources"].clone();
        let restricted = sources
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["state"] == "access_limited");
        let contents_unknown = observed.view["content_limited"] == true
            || observed.view["candidate_scan_complete"] != true;
        let credentials = presence(&root.join(".credentials.json"));
        let profile = presence(&self.home.join(".config/anthropic"));
        let mut items = vec![
            item("cli", "Claude Code CLI", if executable.is_some(){"observed"}else{"not_found"}, "static_discovery", "当前 runner 的 PATH 优先发现及当前用户 native fallback", "仅静态识别，不执行 Claude；其他安装来源、登录与实际加载未核验。", json!({"selected_executable":executable,"registered_executable":environment["executable"],"product":cli}), "launch"),
            item("configuration", "配置与项目来源", if restricted{"access_limited"}else if contents_unknown{"unknown"}else{"observed"}, "finite_config_files", "当前 root；可选项目 cwd 与有限上级候选；managed 文件及 drop-ins", "文件存在和声明只表示可能参与；组织下发、worktree 主 checkout、目录内容与实际加载仍未核验。", json!({"sources":sources,"project_selected":project.is_some(),"startup":observed.view}), "policy"),
            item("authentication", "认证位置", "unknown", "metadata_only", "已识别文件及目录外共享 profile", "未读取凭据、未调用认证命令。文件存在不代表已登录，Keychain 与服务端状态未知；认证来源需另行显式检查。", json!({"credential_file":credentials,"shared_profile":profile,"keychain":"not_checked","account":"not_checked"}), "cleanup"),
            item("desktop_ide", "Desktop／IDE", "unsupported", "adapter_support", "独立客户端与编辑器入口", "尚无存储与生命周期 adapter；暂不支持不能解释为未发现。", json!({}), "none"),
            item("browser", "浏览器 profiles", "separate_module", "native_browser_host", "本机浏览器独立配对的具体 profile", "配置 root 不证明浏览器 profile 归属；从浏览器工作空间按原 clear ID 完成重启后的步骤。远端环境不改绑本机 profile。", json!({"environment_binding":"unverified"}), "browser"),
        ];
        // Read original records without invoking job reconciliation or creating
        // intents. Bound and filtered before projecting finite public fields.
        let mut records = vec![];
        let mut records_complete = true;
        let entries = match fs::read_dir(self.state.join("jobs")) {
            Ok(entries) => Some(entries),
            Err(_) => {
                records_complete = false;
                None
            }
        };
        for (count, entry) in entries.into_iter().flatten().enumerate() {
            if count >= 10000 {
                records_complete = false;
                break;
            }
            let Ok(entry) = entry else {
                records_complete = false;
                continue;
            };
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "json") {
                continue;
            }
            let Ok(job) = load(&path) else {
                records_complete = false;
                continue;
            };
            if job["environment_id"] != environment["id"] {
                continue;
            }
            let Some(id) = job["id"]
                .as_str()
                .filter(|id| lintel_operations::valid_uuid(id))
            else {
                records_complete = false;
                continue;
            };
            if path.file_stem().and_then(|s| s.to_str()) != Some(id) {
                records_complete = false;
                continue;
            }
            records.push(job);
        }
        records.sort_by(|a, b| b["created_at"].as_str().cmp(&a["created_at"].as_str()));
        let records_truncated = records.len() > 50;
        records.truncate(50);
        let mut known = BTreeSet::new();
        let mut services = vec![];
        let mut services_truncated = false;
        for record in &records {
            let service = &record["service"];
            let (Some(manager), Some(unit)) =
                (service["manager"].as_str(), service["unit"].as_str())
            else {
                continue;
            };
            if !["user", "system"].contains(&manager)
                || !lintel_operations::valid_service_unit(unit)
            {
                continue;
            }
            if !known.insert((manager.to_owned(), unit.to_owned())) {
                continue;
            }
            if services.len() >= 8 {
                services_truncated = true;
                continue;
            }
            let original = record["id"].clone();
            let current = self.service_inspect(
                &json!({"environment_id":environment["id"],"manager":manager,"unit":unit}),
            );
            services.push(match current {
                Ok(observed) => json!({"manager":manager,"unit":unit,"original_job_id":original,"recorded_status":record["status"],"recorded_at":record["created_at"],"state":"observed","current":observed}),
                Err(error) => json!({"manager":manager,"unit":unit,"original_job_id":original,"recorded_status":record["status"],"recorded_at":record["created_at"],"state":"unknown","error_code":error.code,"detail":"原记录仍保留；当前 unit 无法核验，不据旧回执宣称仍已暂停。"}),
            });
        }
        items.push(item("services", "服务与后台来源", if services.is_empty(){"unknown"}else if services.iter().all(|s|s["state"]=="observed"){"observed"}else{"unknown"}, "original_receipts_and_service_inspect", "最多 8 个原记录中的确切 systemd unit", "只刷新已有原记录所指的 unit，不全局扫描或停止服务；PM2、Docker、cron 和未知写入者未覆盖。", json!({"services":services,"record_inventory_complete":records_complete && !records_truncated,"services_truncated":services_truncated}), "service"));
        let projected: Vec<_> = records.iter().map(|j| json!({"id":j["id"],"title":j["title"],"status":j["status"],"created_at":j["created_at"],"coverage":j["task_result"]["coverage"],"next_actions":j["task_result"]["next_actions"]})).collect();
        Ok(
            json!({"schema":"lintel.components/1","environment_id":environment["id"],"root":root,"project_cwd":project,"checked_at":checked_at,"items":items,"records":projected,"records_complete":records_complete,"records_truncated":records_truncated,"note":"当前观察与历史任务覆盖分开；操作入口只生成对应预览或查询原 ID，扫描不授予执行权限。"}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};
    #[test]
    fn components_are_scoped_sanitized_and_do_not_execute_auth_or_reconcile_jobs() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let home = base.join("home");
        let root = home.join("config");
        let cwd = home.join("project");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(cwd.join(".claude")).unwrap();
        let mut engine = Engine::new(home.clone(), base.join("state")).unwrap();
        engine.executable_search_path = Some(home.join("bin").into_os_string());
        let exe = home.join(".local/bin/claude");
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        fs::write(
            &exe,
            format!("#!/bin/sh\ntouch '{}'\n", home.join("executed").display()),
        )
        .unwrap();
        fs::set_permissions(&exe, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(root.join("settings.json"), r#"{"env":{"PRIVATE":"SYNTHETIC_PRIVATE_VALUE"},"hooks":{"anything":"SYNTHETIC_COMMAND"}}"#).unwrap();
        fs::write(root.join(".credentials.json"), "SYNTHETIC_CREDENTIAL").unwrap();
        fs::write(cwd.join(".claude/settings.json"), "malformed").unwrap();
        let e = engine.register("synthetic", &root, false).unwrap();
        let other = engine.register("neighbor", &home.join("neighbor"), false);
        assert!(other.is_err());
        let id = crate::id();
        let path = engine.path("jobs", &id);
        let original = json!({"id":id,"environment_id":e["id"],"status":"executing","title":"original","created_at":"2026-10-06","service":{"manager":"user","unit":"synthetic.service"},"task_result":{"coverage":[{"scope":"configuration","state":"done","detail":"Synthetic historical work"}]}});
        save(&path, &original).unwrap();
        let report = engine
            .inspect_components(&json!({"environment_id":e["id"],"project_cwd":cwd}))
            .unwrap();
        let text = report.to_string();
        for private in [
            "SYNTHETIC_PRIVATE_VALUE",
            "SYNTHETIC_COMMAND",
            "SYNTHETIC_CREDENTIAL",
        ] {
            assert!(!text.contains(private));
        }
        assert!(!home.join("executed").exists());
        assert_eq!(load(&path).unwrap(), original);
        assert_eq!(report["items"][1]["state"], "unknown");
        assert_eq!(report["records"][0]["id"], id);
        assert_eq!(
            report["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|i| i["id"] == "services")
                .unwrap()["facts"]["services"][0]["state"],
            "unknown"
        );
        assert_eq!(report["items"][2]["state"], "unknown");
        let linked = home.join("linked");
        symlink(&cwd, &linked).unwrap();
        assert_eq!(
            engine
                .inspect_components(&json!({"environment_id":e["id"],"project_cwd":linked}))
                .unwrap_err()
                .code,
            "symlink_target"
        );
        assert!(engine
            .inspect_components(&json!({"environment_id":e["id"],"project_cwd":"relative"}))
            .is_err());
    }
    #[test]
    fn record_and_unit_limits_keep_explicit_partial_coverage() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let home = base.join("home");
        let root = home.join("config");
        fs::create_dir_all(&root).unwrap();
        let mut engine = Engine::new(home.clone(), base.join("state")).unwrap();
        engine.executable_search_path = Some(home.join("bin").into_os_string());
        let environment = engine.register("synthetic bounds", &root, false).unwrap();
        for index in 0..51 {
            let id = crate::id();
            save(&engine.path("jobs", &id), &json!({"id":id,"environment_id":environment["id"],"created_at":format!("2026-10-06T00:{index:02}:00Z"),"status":"completed","service":{"manager":"user","unit":format!("synthetic-{index}.service")}})).unwrap();
        }
        let report = engine
            .inspect_components(&json!({"environment_id":environment["id"]}))
            .unwrap();
        assert_eq!(report["records"].as_array().unwrap().len(), 50);
        assert_eq!(report["records_truncated"], true);
        let facts = &report["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "services")
            .unwrap()["facts"];
        assert_eq!(facts["services"].as_array().unwrap().len(), 8);
        assert_eq!(facts["services_truncated"], true);
        assert_eq!(facts["record_inventory_complete"], false);
    }
}

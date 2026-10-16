//! Selective semantic baseline acceptance, separate from mutation restoration.
use crate::{err, id, json, now, policy, snapshot, storage::*, string, Engine, Result, Value};
use std::{collections::BTreeSet, path::Path};

fn replace_row(array: &mut Value, new: Option<&Value>, same: impl Fn(&Value) -> bool) {
    let rows = array.as_array_mut().unwrap();
    rows.retain(|row| !same(row));
    if let Some(row) = new {
        rows.push(row.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};
    fn fixture() -> (tempfile::TempDir, Engine, Value, PathBuf, PathBuf) {
        let t = tempfile::tempdir().unwrap();
        let b = t.path().canonicalize().unwrap();
        let h = b.join("home");
        let root = h.join("root");
        let project = h.join("project");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(project.join(".claude")).unwrap();
        save(
            &root.join("settings.json"),
            &json!({"env":{"OTEL_LOG_USER_PROMPTS":"1"}}),
        )
        .unwrap();
        save(
            &project.join(".claude/settings.local.json"),
            &json!({"env":{"OTEL_LOG_TOOL_CONTENT":"0"}}),
        )
        .unwrap();
        let mut engine = Engine::new(h, b.join("state")).unwrap();
        engine.executable_search_path = Some(std::ffi::OsString::new());
        let e = engine.register("synthetic", &root, false).unwrap();
        (t, engine, e, root, project)
    }
    fn saved(engine: &Engine, e: &Value, project: &Path) -> Value {
        engine
            .snapshot_save(&json!({"environment_id":e["id"],"project_cwd":project}))
            .unwrap()["snapshot"]
            .clone()
    }
    fn pair(engine: &Engine, e: &Value, a: &Value, b: &Value) -> Value {
        engine.snapshot_compare(&json!({"environment_id":e["id"],"old_snapshot_id":a["snapshot_id"],"new_snapshot_id":b["snapshot_id"]})).unwrap()
    }
    #[test]
    fn acceptance_merges_only_selected_fields_and_light_checks_sources_without_identity() {
        let (_t, engine, e, root, project) = fixture();
        let a = saved(&engine, &e, &project);
        save(
            &root.join("settings.json"),
            &json!({"env":{"OTEL_LOG_USER_PROMPTS":"0"}}),
        )
        .unwrap();
        save(
            &project.join(".claude/settings.local.json"),
            &json!({"env":{"OTEL_LOG_TOOL_CONTENT":"1"}}),
        )
        .unwrap();
        let b = saved(&engine, &e, &project);
        let compared = pair(&engine, &e, &a, &b);
        let rows = compared["changes"].as_array().unwrap();
        let chosen = rows
            .iter()
            .find(|r| r["key"] == "OTEL_LOG_USER_PROMPTS" && r["state"] == "changed")
            .unwrap();
        let accepted=engine.snapshot_accept(&json!({"environment_id":e["id"],"old_snapshot_id":a["snapshot_id"],"new_snapshot_id":b["snapshot_id"],"selected_changes":[chosen["id"]]})).unwrap();
        let after = pair(&engine, &e, &accepted["baseline"], &b);
        assert!(after["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["key"] == "OTEL_LOG_TOOL_CONTENT" && r["state"] == "changed"));
        assert!(!after["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["key"] == "OTEL_LOG_USER_PROMPTS" && r["state"] == "changed"));
        fs::remove_file(engine.state.join("identity-salt")).unwrap();
        let drift = engine.semantic_drift(&e).unwrap().unwrap();
        assert_eq!(drift["scope"], "configuration_sources_only");
        assert_eq!(drift["status"], "changed");
        assert!(!engine.state.join("identity-salt").exists());
        assert!(drift["changes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["area"] != "identity" && r["area"] != "residue"));
        let project_change = rows
            .iter()
            .find(|r| r["key"] == "OTEL_LOG_TOOL_CONTENT" && r["state"] == "changed")
            .unwrap();
        // Salt was deliberately removed: a fresh full acceptance can still
        // accept an independently verified configuration source.
        engine.snapshot_accept(&json!({"environment_id":e["id"],"old_snapshot_id":accepted["baseline"]["snapshot_id"],"new_snapshot_id":b["snapshot_id"],"selected_changes":[project_change["id"]]})).unwrap();
        assert_eq!(
            engine.semantic_drift(&e).unwrap().unwrap()["status"],
            "unchanged"
        );
        let baseline_id = engine.snapshot_baseline_id(&e).unwrap().unwrap();
        let path = engine
            .state
            .join("snapshots")
            .join(e["id"].as_str().unwrap())
            .join(format!("{baseline_id}.json"));
        let mut baseline = load(&path).unwrap();
        baseline["rule_version"] = json!("synthetic-older-rule");
        save(&path, &baseline).unwrap();
        assert_eq!(
            engine.semantic_drift(&e).unwrap().unwrap()["status"],
            "rules_updated"
        );
    }
    #[test]
    fn stale_selected_source_is_not_accepted_or_written() {
        let (_t, engine, e, root, project) = fixture();
        let a = saved(&engine, &e, &project);
        save(
            &root.join("settings.json"),
            &json!({"env":{"OTEL_LOG_USER_PROMPTS":"0"}}),
        )
        .unwrap();
        let b = saved(&engine, &e, &project);
        let c = pair(&engine, &e, &a, &b);
        let chosen = c["changes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["key"] == "OTEL_LOG_USER_PROMPTS" && r["state"] == "changed")
            .unwrap();
        fs::write(root.join("settings.json"), b"{broken").unwrap();
        let r=engine.snapshot_accept(&json!({"environment_id":e["id"],"old_snapshot_id":a["snapshot_id"],"new_snapshot_id":b["snapshot_id"],"selected_changes":[chosen["id"]]}));
        assert_eq!(r.unwrap_err().code, "stale_snapshot");
        assert_eq!(engine.snapshot_baseline_id(&e).unwrap(), None);
        assert_eq!(fs::read(root.join("settings.json")).unwrap(), b"{broken");
    }
}
pub(crate) fn change_id(row: &Value) -> String {
    json!([
        row["area"],
        row["path"],
        row["scope"],
        row["key"],
        row["category"],
        row["subject"]
    ])
    .to_string()
}
impl Engine {
    pub(crate) fn load_environment_snapshot(&self, e: &Value, snapshot_id: &str) -> Result<Value> {
        if !lintel_operations::valid_uuid(snapshot_id) {
            return Err(err("invalid_id", "快照 ID 无效"));
        }
        let path = self
            .state
            .join("snapshots")
            .join(string(e, "id")?)
            .join(format!("{snapshot_id}.json"));
        if !path.exists() {
            return Err(err("snapshot_not_found", "找不到此环境的快照"));
        }
        let value = load(&path)?;
        if value["schema"] != snapshot::SCHEMA
            || value["environment_id"] != e["id"]
            || value["snapshot_id"] != snapshot_id
        {
            return Err(err("invalid_snapshot", "快照身份或格式不一致"));
        }
        if !value["settings"].is_object()
            || !value["declarations"].is_array()
            || !value["identity"].is_object()
            || !value["residue"].is_object()
            || !value["coverage"].is_object()
            || value
                .get("configuration_sources")
                .is_some_and(|v| !v["settings"].is_array())
        {
            return Err(err("invalid_snapshot", "快照区域结构不完整"));
        }
        Ok(value)
    }
    pub(crate) fn snapshot_baseline_id(&self, e: &Value) -> Result<Option<String>> {
        let path = self
            .state
            .join("snapshot-baselines")
            .join(format!("{}.json", string(e, "id")?));
        guard(&path)?;
        if !path.exists() {
            return Ok(None);
        }
        let record = load(&path)?;
        if record["environment_id"] != e["id"] {
            return Err(err("invalid_baseline", "比较基线的环境不一致"));
        }
        let key = crate::safe_id(&record, "snapshot_id")?;
        Ok(Some(key))
    }
    pub(crate) fn snapshot_accept(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let old = self.load_environment_snapshot(&e, &crate::safe_id(r, "old_snapshot_id")?)?;
        let new = self.load_environment_snapshot(&e, &crate::safe_id(r, "new_snapshot_id")?)?;
        let keys = r["selected_changes"]
            .as_array()
            .filter(|a| !a.is_empty() && a.len() <= 4096)
            .ok_or_else(|| err("invalid_selection", "请选择明确的差异项"))?;
        let mut selected = BTreeSet::new();
        for key in keys {
            let key = key
                .as_str()
                .filter(|s| s.len() <= 8192)
                .ok_or_else(|| err("invalid_selection", "差异选择无效"))?;
            if !selected.insert(key.to_owned()) {
                return Err(err("invalid_selection", "差异选择重复"));
            }
        }
        let (current, _) =
            self.capture_environment(&e, &json!({"project_cwd":new["project_cwd"]}), None)?;
        let proposed = snapshot::compare(&old, &new);
        let fresh = snapshot::compare(&new, &current);
        let rows = proposed["changes"].as_array().unwrap();
        let fresh_rows = fresh["changes"].as_array().unwrap();
        let mut merged = old.clone();
        for key in &selected {
            let row = rows
                .iter()
                .find(|row| change_id(row) == *key)
                .ok_or_else(|| err("invalid_selection", "所选项不属于当前对照"))?;
            if row["masked"] == true
                || !matches!(
                    row["state"].as_str(),
                    Some(
                        "added"
                            | "removed"
                            | "changed"
                            | "appeared"
                            | "disappeared"
                            | "flags_changed"
                    )
                )
            {
                return Err(err(
                    "unverifiable_change",
                    "未完成读取、遮蔽值或未知状态不能被当作已核实值接受",
                ));
            }
            let same = fresh_rows
                .iter()
                .find(|candidate| change_id(candidate) == *key)
                .is_some_and(|candidate| {
                    matches!(candidate["state"].as_str(), Some("unchanged" | "same"))
                });
            if !same {
                return Err(err(
                    "stale_snapshot",
                    "所选原件在快照后变化或已不可读；请保存新快照再接受",
                ));
            }
            match row["area"].as_str() {
                Some("settings") if row["path"].is_null() => {
                    let key = string(row, "key")?;
                    merged["settings"][key] = new["settings"][key].clone();
                    for source in merged["configuration_sources"]["settings"]
                        .as_array_mut()
                        .into_iter()
                        .flatten()
                    {
                        if source["scope"] == "user" && source["key"] == key {
                            if let Some(value) = new["configuration_sources"]["settings"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .find(|r| r["scope"] == "user" && r["key"] == key)
                            {
                                *source = value.clone();
                            }
                        }
                    }
                }
                Some("settings") => {
                    let same = |source: &Value| {
                        source["source"] == row["path"] && source["key"] == row["key"]
                    };
                    let replacement = new["configuration_sources"]["settings"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .find(|source| same(source));
                    replace_row(
                        &mut merged["configuration_sources"]["settings"],
                        replacement,
                        same,
                    );
                }
                Some("declarations") => {
                    let same = |source: &Value| {
                        source["path"] == row["path"] && source["scope"] == row["scope"]
                    };
                    let replacement = new["declarations"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .find(|source| same(source));
                    replace_row(&mut merged["declarations"], replacement, same);
                }
                Some("identity") => merged["identity"] = new["identity"].clone(),
                Some("residue") => {
                    let category = string(row, "category")?;
                    if let Some(value) = new["residue"].get(category) {
                        merged["residue"][category] = value.clone();
                    } else {
                        merged["residue"].as_object_mut().unwrap().remove(category);
                    }
                }
                _ => return Err(err("invalid_selection", "差异类别不支持接受")),
            }
        }
        let totals = merged["residue"]
            .as_object()
            .unwrap()
            .iter()
            .filter(|(key, _)| !["totals", "scan_complete"].contains(&key.as_str()))
            .fold((0u64, 0u64), |(count, bytes), (_, value)| {
                (
                    count.saturating_add(value["count"].as_u64().unwrap_or(0)),
                    bytes.saturating_add(value["bytes"].as_u64().unwrap_or(0)),
                )
            });
        merged["residue"]["totals"] = json!({"file_count":totals.0,"total_bytes":totals.1});
        merged["snapshot_id"] = json!(id());
        merged["created_at"] = json!(now());
        merged["rule_version"] = json!(policy::RULE);
        merged["project_cwd"] = new["project_cwd"].clone();
        merged["baseline"] = json!({"kind":"selected_semantic_acceptance","old_snapshot_id":old["snapshot_id"],"new_snapshot_id":new["snapshot_id"],"selected_changes":selected,"not_a_fresh_capture":true});
        let dir = self.snapshot_dir(string(&e, "id")?)?;
        let bytes = serde_json::to_vec_pretty(&merged)?;
        if bytes.len() as u64 > STATE_JSON_LIMIT {
            return Err(err("state_json_limit", "比较基线超过可持久读取上限"));
        }
        atomic_new(
            &dir.join(format!("{}.json", string(&merged, "snapshot_id")?)),
            &bytes,
            0o600,
        )?;
        let pointers = self.state.join("snapshot-baselines");
        private_dir(&pointers)?;
        save(
            &pointers.join(format!("{}.json", string(&e, "id")?)),
            &json!({"environment_id":e["id"],"snapshot_id":merged["snapshot_id"]}),
        )?;
        Ok(
            json!({"status":"accepted","baseline":crate::observe::snapshot_summary(&merged),"note":"这是逐项接受后合成的比较基线，不是一份同时刻完整观测，也不提供恢复原值。"}),
        )
    }
    /// Configuration-only source checks. No residue tree or identity read.
    pub(crate) fn semantic_drift(&self, e: &Value) -> Result<Option<Value>> {
        let Some(key) = self.snapshot_baseline_id(e)? else {
            return Ok(None);
        };
        let base = self.load_environment_snapshot(e, &key)?;
        let root = Path::new(string(e, "root")?);
        let (_, doc, _) = self.settings(e)?;
        let project = base["project_cwd"].as_str().map(Path::new);
        let sources = crate::components::startup::snapshot(
            &self.home,
            root,
            project,
            "lightweight-readonly",
        )?;
        let mut current = snapshot::capture(
            string(e, "id")?,
            None,
            &doc,
            Some(&sources.view),
            None,
            None,
        );
        let config =
            self.config_inspect(&json!({"environment_id":e["id"],"project_cwd":project}))?;
        current["configuration_sources"] =
            json!({"files":config["files"],"settings":config["settings"]});
        current["coverage"]["configuration_sources"] = config["coverage"].clone();
        let compared = snapshot::compare(&base, &current);
        let changes: Vec<_> = compared["changes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                matches!(row["area"].as_str(), Some("settings" | "declarations"))
                    && !matches!(row["state"].as_str(), Some("unchanged" | "uncertain"))
            })
            .cloned()
            .collect();
        let changed = changes.iter().any(|row| {
            matches!(
                row["state"].as_str(),
                Some(
                    "changed" | "added" | "removed" | "appeared" | "disappeared" | "flags_changed"
                )
            )
        });
        let changes: Vec<_> = changes
            .into_iter()
            .map(|mut row| {
                row["value"] = row["new"].clone();
                row["source"] = row["path"].clone();
                row["effect_timing"] = json!("next_launch");
                row["status"] = row["state"].clone();
                if row["key"].is_null() {
                    row["key"] = row["path"].clone();
                    row["label"] = json!("配置来源声明");
                }
                row
            })
            .collect();
        Ok(Some(
            json!({"status":if changed{"changed"}else if !changes.is_empty(){"incomplete"}else if compared["rule_change"].is_object(){"rules_updated"}else{"unchanged"},"changes":changes,"scope":"configuration_sources_only","baseline_id":key,"rule_change":compared["rule_change"],"note":"轻量检查只读有限配置来源；残留与身份由明确的完整对照读取。"}),
        ))
    }
}

//! Read-only observation commands: bounded residue inventory, masked identity
//! presence, and sanitized environment snapshots (save/list/compare/pair).
//!
//! These commands mutate nothing except the private snapshot store
//! (`snapshot_save`). Identity values and masked settings never leave the
//! crate: the salted digest stays inside the stored snapshot, and every
//! response carries presence flags or three-word verdicts only.
use crate::{
    components::startup, err, id, policy, residue, safe_id, snapshot as env_snapshot, storage::*,
    string, Engine, Error, Result,
};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

/// Listed snapshots per environment; the store itself is unbounded (each
/// snapshot is a small metadata record).
const MAX_LISTED_SNAPSHOTS: usize = 200;
const MAX_SCANNED_SNAPSHOTS: usize = 2000;

fn observation_error(area: &str, error: &Error) -> Value {
    // Keep diagnostics finite. An external path/value in a parser diagnostic
    // must not accidentally become snapshot content.
    json!({"area": area, "code": error.code})
}

/// The public summary of a stored snapshot: settings/declaration counts and
/// coverage flags only, never a settings value or the identity digest.
pub(crate) fn snapshot_summary(snapshot: &Value) -> Value {
    json!({
        "snapshot_id": snapshot["snapshot_id"],
        "environment_id": snapshot["environment_id"],
        "created_at": snapshot["created_at"],
        "rule_version": snapshot["rule_version"],
        "product_version": snapshot["product_version"],
        "coverage": snapshot["coverage"],
        "settings_count": snapshot["settings"].as_object().map_or(0, |m| m.len()),
        "declarations_count": snapshot["declarations"].as_array().map_or(0, Vec::len),
        "residue_totals": snapshot["residue"]["totals"],
        "identity_state": snapshot["identity"]["userID"]["state"],
        "baseline":snapshot["baseline"],
    })
}

impl Engine {
    /// Load (creating on first use) the persistent identity-comparison salt.
    /// One salt per state directory: two observations are only comparable under
    /// the same salt. The salt is not a credential; it exists so a stored
    /// digest is never reusable as one.
    fn identity_salt(&self) -> Result<String> {
        let path = self.state.join("identity-salt");
        let existing = fs::symlink_metadata(&path);
        match existing {
            Ok(_) => return read_identity_salt(&path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
        let salt = format!("{}{}", id(), id());
        match atomic_new(&path, salt.as_bytes(), 0o600) {
            Ok(()) => Ok(salt),
            // A concurrent first use won the no-replace publish: read its salt.
            Err(failure) if failure.code == "target_exists" => read_identity_salt(&path),
            Err(failure) => Err(failure),
        }
    }

    /// Assemble a fresh sanitized snapshot of one registered environment. Every
    /// observation is taken now; the identity digest uses the persistent salt
    /// so snapshots taken at different times stay comparable.
    pub(crate) fn capture_environment(
        &self,
        e: &Value,
        r: &Value,
        comparison_salt: Option<&str>,
    ) -> Result<(Value, Value)> {
        let e = policy::environment(e.clone(), self.executable());
        let root = PathBuf::from(string(&e, "root")?);
        let project = r
            .get("project_cwd")
            .and_then(Value::as_str)
            .map(PathBuf::from);
        let mut errors = vec![];
        let settings = match self.settings(&e) {
            Ok((_, doc, _)) => Some(doc),
            Err(error) => {
                errors.push(observation_error("settings", &error));
                None
            }
        };
        // The startup view publishes no digest; its per-call salt never leaves
        // the discarded private binding.
        let startup_view = match startup::snapshot(&self.home, &root, project.as_deref(), &id()) {
            Ok(value) => Some(value.view),
            Err(error) => {
                errors.push(observation_error("startup", &error));
                None
            }
        };
        let residue_view = match residue::inventory(&self.home, &root) {
            Ok(value) => Some(value),
            Err(error) => {
                errors.push(observation_error("residue", &error));
                None
            }
        };
        let identity = (|| {
            let salt = match comparison_salt {
                Some(salt) => salt.to_owned(),
                None => self.identity_salt()?,
            };
            let mut identity = residue::identity_observation(&self.home, &root, &salt)?;
            identity["binding"]["comparison_scope"] = json!(digest(salt.as_bytes()));
            Ok::<Value, Error>(identity)
        })();
        let identity = match identity {
            Ok(value) => Some(value),
            Err(error) => {
                errors.push(observation_error("identity", &error));
                None
            }
        };
        let mut captured = env_snapshot::capture(
            string(&e, "id")?,
            e["product_version"].as_str(),
            settings.as_ref().unwrap_or(&Value::Null),
            startup_view.as_ref(),
            residue_view.as_ref(),
            identity.as_ref().map(|value| &value["binding"]),
        );
        match self.config_inspect(&json!({"environment_id":e["id"],"project_cwd":project})) {
            Ok(sources) => {
                captured["configuration_sources"] =
                    json!({"files":sources["files"],"settings":sources["settings"]});
                captured["coverage"]["configuration_sources"] = sources["coverage"].clone();
            }
            Err(error) => {
                captured["coverage"]["configuration_sources"] = json!(false);
                errors.push(observation_error("configuration_sources", &error));
            }
        }
        captured["project_cwd"] = json!(project);
        captured["coverage"]["settings"] = json!(settings.is_some());
        captured["coverage"]["errors"] = json!(errors);
        Ok((
            captured,
            identity
                .map(|value| value["public"].clone())
                .unwrap_or_else(|| json!({"state":"unknown"})),
        ))
    }

    pub(crate) fn snapshot_dir(&self, environment_id: &str) -> Result<PathBuf> {
        let dir = self.state.join("snapshots").join(environment_id);
        private_dir(&dir)?;
        Ok(dir)
    }

    pub(crate) fn residue_inventory(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let root = PathBuf::from(string(&e, "root")?);
        residue::inventory_request(&self.home, &root, r)
    }

    pub(crate) fn snapshot_save(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let (captured, identity_public) = self.capture_environment(&e, r, None)?;
        let dir = self.snapshot_dir(string(&e, "id")?)?;
        let bytes = serde_json::to_vec_pretty(&captured)?;
        if bytes.len() as u64 > STATE_JSON_LIMIT {
            return Err(err(
                "state_json_limit",
                "快照超过可持久读取的 JSON 上限，未保存",
            ));
        }
        // The snapshot is immutable, including its original ID on uncertain
        // publication. Never overwrite an existing observation.
        atomic_new(
            &dir.join(format!("{}.json", string(&captured, "snapshot_id")?)),
            &bytes,
            0o600,
        )?;
        Ok(json!({
            "snapshot": snapshot_summary(&captured),
            "identity": identity_public,
            "note": "快照只记录设置可见值、有限声明、加盐身份摘要与残留统计；遮蔽值与身份原值从未进入。",
        }))
    }

    pub(crate) fn snapshot_list(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let dir = self.state.join("snapshots").join(string(&e, "id")?);
        guard(&dir)?;
        let mut all = vec![];
        let mut unreadable = vec![];
        let mut scanned = 0usize;
        let mut scan_complete = true;
        if dir.is_dir() {
            for entry in fs::read_dir(&dir)? {
                if scanned == MAX_SCANNED_SNAPSHOTS {
                    scan_complete = false;
                    break;
                }
                scanned += 1;
                let path = match entry {
                    Ok(entry) => entry.path(),
                    Err(_) => {
                        unreadable.push(json!({"code":"entry_unreadable"}));
                        continue;
                    }
                };
                if path.extension().is_some_and(|ext| ext == "json") {
                    match load(&path) {
                        Ok(value)
                            if value["schema"] == env_snapshot::SCHEMA
                                && value["environment_id"] == e["id"] =>
                        {
                            all.push(value)
                        }
                        Ok(_) => unreadable.push(json!({"code":"invalid_snapshot"})),
                        Err(error) => unreadable.push(observation_error("snapshot", &error)),
                    }
                }
            }
        }
        all.sort_by(|a, b| b["created_at"].as_str().cmp(&a["created_at"].as_str()));
        let total = all.len();
        all.truncate(MAX_LISTED_SNAPSHOTS);
        let listed = all.len();
        let snapshots: Vec<Value> = all.iter().map(snapshot_summary).collect();
        Ok(
            json!({"environment_id": e["id"], "snapshots": snapshots, "total": total, "listed": listed, "scanned":scanned, "scan_complete":scan_complete, "errors":unreadable,"accepted_snapshot_id":self.snapshot_baseline_id(&e)?}),
        )
    }

    pub(crate) fn snapshot_compare(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let old_id = safe_id(r, "old_snapshot_id")?;
        let new_id = safe_id(r, "new_snapshot_id")?;
        if old_id == new_id {
            return Err(err("invalid_request", "请选择两份不同的快照进行对照"));
        }
        Ok(env_snapshot::compare(
            &self.load_environment_snapshot(&e, &old_id)?,
            &self.load_environment_snapshot(&e, &new_id)?,
        ))
    }

    pub(crate) fn env_compare(&self, r: &Value) -> Result<Value> {
        let a = self.env(r)?;
        let b = self.env(&json!({"environment_id": string(r, "other_environment_id")?}))?;
        if a["id"] == b["id"] {
            return Err(err("invalid_request", "请选择两个不同的环境进行并排对照"));
        }
        let salt = id();
        let (snap_a, _) = self.capture_environment(&a, r, Some(&salt))?;
        let (snap_b, _) = self.capture_environment(
            &b,
            &json!({"project_cwd":r["other_project_cwd"]}),
            Some(&salt),
        )?;
        Ok(json!({
            "a": {"environment_id": a["id"], "name": a["name"], "captured_at": snap_a["created_at"]},
            "b": {"environment_id": b["id"], "name": b["name"], "captured_at": snap_b["created_at"]},
            "comparison": env_snapshot::compare_pair(&snap_a, &snap_b),
        }))
    }
}

fn read_identity_salt(path: &std::path::Path) -> Result<String> {
    use std::os::unix::fs::MetadataExt;
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err(err(
            "salt_unreadable",
            "身份比较 salt 必须是当前用户独占的私有普通文件",
        ));
    }
    let bytes = read(path, 1024)?;
    let salt = String::from_utf8(bytes)
        .map_err(|_| err("salt_unreadable", "身份比较 salt 文件损坏；未覆盖新值"))?;
    if salt.len() != 72
        || !salt.is_ascii()
        || salt.split_at(36).0.parse::<uuid::Uuid>().is_err()
        || salt.split_at(36).1.parse::<uuid::Uuid>().is_err()
    {
        return Err(err("salt_unreadable", "身份比较 salt 文件损坏；未覆盖新值"));
    }
    Ok(salt)
}

#[cfg(test)]
#[path = "observe_tests.rs"]
mod tests;

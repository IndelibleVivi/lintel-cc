//! Local data and identity management: exact residue clearing and the
//! independent persistent-identifier reset.
//!
//! Two write operations live here, both reusing the existing plan / approval /
//! job / receipt chain rather than adding a second executor:
//!
//!   * `clear_residue` — an exact, per-file removal of selected residue
//!     originals, with an optional independent encrypted residue backup that
//!     must be written and read back before any deletion;
//!   * `identity_reset` — remove exactly the verified persistent identifier
//!     (`userID`) from the client-state document plus explicitly selected
//!     backup files, after an encrypted backup of both, keeping every other key
//!     and leaving logout and credentials to their own independent recipes.
//!
//! Residue bytes never enter `lintel.work/1`: a residue backup is its own
//! `lintel.residue/1` package sealed with the shared age/JSON codec
//! ([`crate::archive::seal`]), so it can never be imported as work content.
use crate::{archive, cleanup, err, now, residue, storage::*, string, work, Engine, Result};
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

/// The `plan_cleanup` recipe literal for the exact residue clear.
pub(crate) const CLEAR_RESIDUE_RECIPE: &str = "clear_residue";
/// The `plan_cleanup` recipe literal for the independent identity reset.
pub(crate) const IDENTITY_RESET_RECIPE: &str = "identity_reset";
/// The residue backup package schema. Deliberately distinct from `lintel.work/1`
/// so a residue archive can never be read as importable work content.
pub(crate) const RESIDUE_SCHEMA: &str = "lintel.residue/1";
/// Per-file admission for one residue original. Residue is logs/caches, not a
/// work package: the bound stays well under the shared 256 MiB work limit while
/// still admitting a large debug log. The optional residue backup is sealed
/// through the in-memory age/JSON codec ([`crate::archive::seal`]), whose JSON
/// byte-array encoding is roughly four bytes per plaintext byte, so this bound
/// keeps that single warm-path package bounded.
const RESIDUE_FILE_LIMIT: u64 = 8 * 1024 * 1024;
/// Aggregate admission for one residue plan's preserved content.
const RESIDUE_TOTAL_LIMIT: u64 = 32 * 1024 * 1024;
/// File-count admission for one residue plan.
const RESIDUE_MAX_FILES: usize = 2000;
/// Identity material bound: the same discipline `residue.rs` uses, so an
/// oversized client-state file is refused rather than partially rewritten.
const IDENTITY_BYTES: u64 = 2 * 1024 * 1024;
/// Bound for one explicitly selected identity backup file.
const IDENTITY_BACKUP_BYTES: u64 = 8 * 1024 * 1024;
/// Aggregate bound for the identity backups in one plan.
const IDENTITY_BACKUP_TOTAL: u64 = 32 * 1024 * 1024;

/// One frozen residue original: the absolute path plus the identity/content
/// facts that bind a later removal to this exact object.
fn freeze_residue_original(path: &Path) -> Result<Value> {
    guard(path)?;
    if !path.is_absolute() {
        return Err(err("invalid_path", "residue_paths 需要绝对路径"));
    }
    let meta = fs::symlink_metadata(path)
        .map_err(|_| err("residue_missing", "所选残留原件不存在或不可读"))?;
    if meta.file_type().is_symlink() {
        return Err(err("symlink_target", "残留原件不能是符号链接"));
    }
    if !meta.is_file() {
        return Err(err(
            "file_limit",
            "残留原件必须是常规文件（目录请选择其中的具体文件）",
        ));
    }
    if meta.uid() != unsafe { libc::geteuid() } || meta.nlink() != 1 {
        return Err(err("unsafe_file", "残留原件的所有者或硬链接状态不受支持"));
    }
    if meta.len() > RESIDUE_FILE_LIMIT {
        return Err(err(
            "file_limit",
            "残留原件超过单文件容量上限（8 MiB）；请缩小精确选择范围",
        ));
    }
    // `snapshot` is the shape the shared quarantine-removal verifier compares
    // against; `identity` adds the timestamp facts that distinguish an
    // immediate same-content inode reuse.
    Ok(json!({
        "path": path,
        "bytes": meta.len(),
        "digest": digest_file_bounded(path, RESIDUE_FILE_LIMIT)?,
        "snapshot": cleanup::bounded_snapshot(path, RESIDUE_FILE_LIMIT)?,
        "identity": {"device":meta.dev(),"inode":meta.ino(),"owner":meta.uid(),
            "mode":meta.mode(),"mtime":meta.mtime(),"mtime_nsec":meta.mtime_nsec(),
            "ctime":meta.ctime(),"ctime_nsec":meta.ctime_nsec()},
    }))
}

/// Streaming, bounded SHA-256 of one regular file opened without following a
/// symlink and without blocking on a special file.
fn digest_file_bounded(path: &Path, limit: u64) -> Result<String> {
    use sha2::Digest as _;
    guard(path)?;
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(err("file_limit", "残留原件不是常规文件"));
    }
    let mut hasher = sha2::Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    let mut count = 0u64;
    loop {
        let read = match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(err("io_error", "读取残留原件失败")),
        };
        count += read as u64;
        if count > limit {
            return Err(err("file_limit", "残留原件在读取期间超过容量上限"));
        }
        hasher.update(&buf[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Re-verify one frozen residue original immediately before its removal: the
/// path still resolves to exactly the frozen object (identity) with the frozen
/// bytes (digest). Any disagreement is a conflict, never a silent skip.
fn recheck_residue_original(file: &Value) -> Result<bool> {
    let path = PathBuf::from(string(file, "path")?);
    let frozen = &file["identity"];
    let meta = match fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(err(
            "cleanup_conflict",
            "残留原件在清理前被替换为非普通文件或符号链接",
        ));
    }
    let observed = json!({"device":meta.dev(),"inode":meta.ino(),"owner":meta.uid(),
        "mode":meta.mode(),"mtime":meta.mtime(),"mtime_nsec":meta.mtime_nsec(),
        "ctime":meta.ctime(),"ctime_nsec":meta.ctime_nsec()});
    if observed != *frozen {
        return Err(err(
            "cleanup_conflict",
            "残留原件在清理前被替换或修改；保留新状态，未删除",
        ));
    }
    if digest_file_bounded(&path, RESIDUE_FILE_LIMIT)? != string(file, "digest")? {
        return Err(err(
            "cleanup_conflict",
            "残留原件内容在清理前改变；保留新状态，未删除",
        ));
    }
    Ok(true)
}

/// Read one bounded residue original for the encrypted backup. Uses the shared
/// no-follow discipline and refuses anything but the frozen regular file.
fn read_residue_original(file: &Value) -> Result<Vec<u8>> {
    let path = PathBuf::from(string(file, "path")?);
    let bytes = read(&path, RESIDUE_FILE_LIMIT)?;
    if bytes.len() as u64 != file["bytes"].as_u64().unwrap_or(u64::MAX)
        || digest(&bytes) != string(file, "digest")?
        || !recheck_residue_original(file)?
    {
        return Err(err("cleanup_conflict", "残留原件在备份读取期间长度改变"));
    }
    Ok(bytes)
}

/// The sealed residue package value. Bytes are carried as the historical
/// `Vec<u8>` JSON array, exactly like the work codec, so the existing age/JSON
/// reader in this crate can open it. It is a different *schema*, never a
/// `lintel.work/1` package.
fn residue_package(environment_id: &str, files: &[Value]) -> Value {
    json!({
        "schema": RESIDUE_SCHEMA,
        "generator": "Lintel",
        "created_at": now(),
        "environment_id": environment_id,
        "files": files,
        "notes": "本地残留备份：只包含预览中明确选择的原件字节，可独立解密查看；与 lintel.work/1 工作包不通用，不会被自动迁入任何环境。",
    })
}

/// Publish complete sealed bytes at `dest` through the shared no-replace
/// primitive, with both durability barriers (staged flush before publication,
/// destination-parent flush after). A rename that lands but whose parent sync
/// fails is reported as an error so no caller marks preservation completed
/// without that barrier. The staged temp owns exactly its own name.
fn publish_sealed(dest: &Path, bytes: &[u8]) -> Result<()> {
    guard(dest)?;
    let parent = dest
        .parent()
        .ok_or_else(|| err("invalid_path", "备份目标缺少父目录"))?;
    let tmp = parent.join(format!(".lintel-{}.tmp", uuid::Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&tmp)?;
    let mut staged = StagedTemp::new(tmp.clone());
    let result = (|| -> Result<()> {
        file.write_all(bytes)?;
        sync_staged_file(&file)?;
        publish_staged_new(&tmp, dest)?;
        staged.disarm();
        Ok(())
    })();
    drop(file);
    drop(staged);
    result
}

impl Engine {
    /// Masked identity observation for the environment, plus an explicit,
    /// private reveal of the one verified persistent identifier.
    ///
    /// Default (`reveal` absent/false) publishes presence and persistence flags
    /// only. `reveal:true` adds the raw `userID` value under `revealed`, which
    /// exists only in this response: it is never written to a plan, job,
    /// receipt, snapshot or Agent metadata. `oauthAccount` is presence-only and
    /// is never revealed (it can carry account material).
    pub(crate) fn identity_inspect(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let root = PathBuf::from(string(&e, "root")?);
        guard(&root)?;
        // A per-call salt: this read-only projection publishes no digest and
        // persists nothing, so the binding is discarded rather than stored.
        let observed = residue::identity_observation(&self.home, &root, &crate::id())?;
        let mut public = observed["public"].clone();
        public["environment_id"] = e["id"].clone();
        public["config_directory"] = e["root"].clone();
        let reveal = r.get("reveal").and_then(Value::as_bool).unwrap_or(false);
        if !reveal {
            public["revealed"] = Value::Null;
            return Ok(public);
        }
        let path = residue::client_state_path(&self.home, &root);
        let mut revealed = vec![];
        let mut reveal_note =
            "仅返回已核实的持久标识 userID 原值；oauthAccount 仍只报存在性，不返回任何账号字段。此值只存在于本次响应，不写入计划、任务、收据、快照或 Agent 元数据。"
                .to_string();
        match residue::read_client_state(&path) {
            residue::ClientState::Observed(doc) => {
                if let Some(value) = doc
                    .get("userID")
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                {
                    revealed.push(json!({"key":"userID","value":value}));
                }
            }
            state => {
                reveal_note = format!(
                    "client_state 状态为 {}，未产出任何原值。{reveal_note}",
                    state.code()
                );
            }
        }
        public["revealed"] = if revealed.is_empty() {
            Value::Null
        } else {
            json!(revealed)
        };
        public["reveal_scope"] = json!("private_response_only");
        public["note"] = json!(format!(
            "{} {reveal_note}",
            public["note"].as_str().unwrap_or("")
        ));
        Ok(public)
    }

    /// Preview clearing exactly the selected residue originals. Metadata and
    /// content digests are frozen for each named file; nothing outside the
    /// explicit `residue_paths` list is included, so a clear never widens into a
    /// directory sweep.
    pub(crate) fn plan_clear_residue(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        if r["writers_confirmed_stopped"] != true {
            return Err(err(
                "writers_confirmation_required",
                "先关闭目标终端、IDE 与自动重启来源，再预览残留清理",
            ));
        }
        if !self.known_writers(&e)?.is_empty() {
            return Err(err(
                "writers_active",
                "仍检测到 Claude 进程且无法确认所属 root；请核对并关闭目标写入者，Lintel 不会全局杀进程",
            ));
        }
        let requested = r["residue_paths"].as_array().ok_or_else(|| {
            err(
                "invalid_residue_paths",
                "clear_residue 需要 residue_paths 精确原件列表",
            )
        })?;
        if requested.is_empty() {
            return Err(err(
                "invalid_residue_paths",
                "residue_paths 为空；空选择不会被当作整目录清理",
            ));
        }
        if requested.len() > RESIDUE_MAX_FILES {
            return Err(err(
                "invalid_residue_paths",
                "residue_paths 超过 2000 项上限",
            ));
        }
        let mut seen = std::collections::HashSet::new();
        let mut files = vec![];
        let mut total = 0u64;
        let root = Path::new(string(&e, "root")?);
        let sources = residue::inventory_request(
            &self.home,
            root,
            &json!({"extra_roots":r.get("extra_roots").cloned().unwrap_or(json!([]))}),
        )?;
        for raw in requested {
            let raw = raw.as_str().filter(|s| !s.is_empty()).ok_or_else(|| {
                err(
                    "invalid_residue_paths",
                    "residue_paths 只能包含非空绝对路径字符串",
                )
            })?;
            let path = Path::new(raw);
            if !seen.insert(path.to_path_buf()) {
                return Err(err("invalid_residue_paths", "residue_paths 含重复路径"));
            }
            if !path.is_absolute()
                || path
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
                || raw.chars().any(char::is_control)
            {
                return Err(err("invalid_path", "需要准确的绝对原件路径"));
            }
            guard(path)?;
            let protected = path.starts_with(&self.state)
                || path == root.join("settings.json")
                || path == root.join(".credentials.json")
                || path == residue::client_state_path(&self.home, root);
            if protected {
                return Err(err(
                    "protected_local_item",
                    "此原件属于配置、登录、身份或 Lintel 状态；请用对应的独立操作",
                ));
            }
            let known = residue::LOCATION_TABLE
                .iter()
                .any(|(relative, _, class, _)| {
                    let location = residue::resolve_location(&self.home, root, relative);
                    matches!(*class, "residue" | "work") && path.starts_with(&location)
                });
            let explicit = sources["locations"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|location| {
                    matches!(location["class"].as_str(), Some("residue" | "unclassified"))
                        && location["path"]
                            .as_str()
                            .is_some_and(|p| path.starts_with(p))
                });
            if !known && !explicit {
                return Err(err(
                    "unsupported_local_item",
                    "原件不在已知位置或本次明确选择的目录；请先加入清单",
                ));
            }
            let work_path = path
                .strip_prefix(root)
                .ok()
                .and_then(lintel_operations::work_path_category);
            if work_path.is_some() && r["delete_work"] != true && r["archive_only"] != true {
                return Err(err(
                    "work_confirmation_required",
                    "所选原件包含工作内容；需要明确选择删除工作，或改用工作保全",
                ));
            }
            let frozen = freeze_residue_original(path)?;
            total += frozen["bytes"].as_u64().unwrap_or(0);
            if total > RESIDUE_TOTAL_LIMIT {
                return Err(err(
                    "archive_limit",
                    "所选残留超过合计容量上限（32 MiB）；未截断或删除",
                ));
            }
            files.push(frozen);
        }
        let archive_only = r["archive_only"] == true;
        let archive_before_clear = r
            .get("archive_before_clear")
            .and_then(Value::as_bool)
            .unwrap_or(true)
            || archive_only;
        let services = self.check_cleanup_services(&e)?;
        let mut actions = vec![
            json!({"id":"quiescence","label":"复查目标已停止写入；不关闭其他环境","reversible":false}),
        ];
        if archive_before_clear {
            actions.push(json!({"id":"residue_backup","label":"先写入独立加密残留备份并读回","reversible":false}));
        }
        for f in files.iter().filter(|_| !archive_only) {
            actions.push(json!({"id":"clear","label":format!("移除精确残留原件：{}", f["path"].as_str().unwrap_or("")),"reversible":false}));
        }
        self.plan(
            &e,
            "cleanup",
            if archive_only {
                "加密归档所选本地数据"
            } else {
                "清理所选运行残留"
            },
            json!([]),
            vec![
                "未选中的残留、工作内容、身份与登录材料",
                "settings、hooks、MCP 与插件文件",
                "其他环境与目录外认证",
            ],
            json!(actions),
            json!({
                "recipe": CLEAR_RESIDUE_RECIPE,
                "official_logout": false,
                "outcome": "clear_residue",
                "files": files,
                "archive_before_clear": archive_before_clear,
                "archive_only": archive_only,
                "archive_passphrase_required": archive_before_clear,
                "services": services,
                "root": e["root"],
                "executable": e["executable"],
            }),
        )
    }

    /// Pre-execution recheck for a `clear_residue` plan.
    ///
    /// Only the plan-wide, all-or-nothing facts are checked here: the service
    /// hold evidence and the preservation passphrase. Each frozen original is
    /// rechecked individually at its own removal step, so one changed file
    /// becomes a per-item skip with a reason and never blocks the other,
    /// already-preserved removals.
    pub(crate) fn check_residue_clear(&self, p: &Value, r: &Value) -> Result<()> {
        if !self
            .known_writers(&json!({"id":p["environment_id"],"root":p["root"]}))?
            .is_empty()
        {
            return Err(err("writers_active", "发现目标写入者；未开始清理"));
        }
        if json!(self.check_cleanup_services(&json!({"id":p["environment_id"],"root":p["root"]}))?)
            != p["extra"]["services"]
        {
            return Err(err(
                "stale_service_plan",
                "预览后 service hold 证据变化；未开始清理",
            ));
        }
        if p["extra"]["archive_before_clear"] == true {
            work::check_passphrase(r)?;
        }
        Ok(())
    }

    /// Execute a `clear_residue` plan: optionally write and read back the sealed
    /// residue package first, then remove exactly the frozen originals one at a
    /// time, recording a per-file receipt. A single file's failure never
    /// discards the completed preservation or the other independent removals.
    pub(crate) fn clear_residue(
        &self,
        e: &Value,
        p: &Value,
        r: &Value,
        j: &mut Value,
        journal: &Path,
    ) -> Result<()> {
        let files = p["extra"]["files"].as_array().cloned().unwrap_or_default();
        // The plan-wide recheck (service hold + passphrase) runs before any
        // preservation. A per-file change is handled at its own step, so one
        // changed file never discards the others' independent preservation.
        self.check_residue_clear(p, r)?;
        let mut removed = 0usize;
        let mut skipped_absent = 0usize;
        let mut conflicts: Vec<Value> = vec![];
        let mut failures: Vec<Value> = vec![];
        // Optional independent preservation. Only files that still match their
        // frozen identity and bytes are admitted to the sealed package; a file
        // that changed or vanished since preview is never backed up against a
        // digest it no longer has, and is never removed by this plan.
        let mut preserved: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
        if p["extra"]["archive_before_clear"] == true {
            let pass = work::check_passphrase(r)?;
            let mut staged = vec![];
            for file in &files {
                match recheck_residue_original(file) {
                    Ok(true) => {}
                    Ok(false) => {
                        skipped_absent += 1;
                        continue;
                    }
                    Err(error) => {
                        conflicts.push(
                            json!({"path":file["path"],"code":error.code,"message":error.message}),
                        );
                        continue;
                    }
                }
                let data = match read_residue_original(file) {
                    Ok(data) => data,
                    Err(error) => {
                        conflicts.push(
                            json!({"path":file["path"],"code":error.code,"message":error.message}),
                        );
                        continue;
                    }
                };
                staged.push(json!({
                    "path": file["path"],
                    "bytes": file["bytes"],
                    "digest": file["digest"],
                    "data": data,
                }));
                preserved.insert(PathBuf::from(string(file, "path")?));
            }
            let bytes = archive::seal(&residue_package(string(e, "id")?, &staged), pass)?;
            let dest = self
                .state
                .join("archives")
                .join(format!("{}-residue.age", string(j, "id")?));
            j["steps"].as_array_mut().unwrap().push(json!({"id":"residue_backup","label":"独立加密残留备份","status":"executing","message":"正在写入并读回残留备份；备份成功前不会删除任何文件。"}));
            j["residue_archive_path"] = json!(dest);
            j["residue_backup"] = json!({"preserved": staged.len(), "skipped_absent": skipped_absent, "conflicts": conflicts.len()});
            save(journal, j)?;
            publish_sealed(&dest, &bytes)?;
            if read(&dest, 64 * 1024 * 1024)? != bytes {
                return Err(err("archive_readback_failed", "残留备份未能读回，未清理"));
            }
            *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":"residue_backup","label":"独立加密残留备份","status":"completed","message":"残留备份已写入并读回；schema 为 lintel.residue/1，独立于工作包与身份。"});
            save(journal, j)?;
        }
        if p["extra"]["archive_only"] == true {
            j["status"] = json!(if conflicts.is_empty() {
                "completed"
            } else {
                "partially_completed"
            });
            j["outcome"] = json!("residue_archive_only");
            j["residue_archive"] = json!({"selected":files.len(),"preserved":preserved.len(),"conflicts":conflicts,"skipped_absent":skipped_absent});
            return Ok(());
        }
        for file in &files {
            let path = Path::new(string(file, "path")?);
            // A file not admitted to the preservation is not removed here: its
            // reason was already recorded above.
            if p["extra"]["archive_before_clear"] == true
                && !preserved.contains(&path.to_path_buf())
            {
                continue;
            }
            match recheck_residue_original(file) {
                Ok(false) => {
                    skipped_absent += 1;
                    j["steps"].as_array_mut().unwrap().push(json!({"id":"clear","label":file["path"],"status":"skipped","message":"预览后该原件已不存在；未重复删除。"}));
                    save(journal, j)?;
                    continue;
                }
                Ok(true) => {}
                Err(error) => {
                    conflicts.push(
                        json!({"path":file["path"],"code":error.code,"message":error.message}),
                    );
                    j["steps"].as_array_mut().unwrap().push(json!({"id":"clear","label":file["path"],"status":"skipped","message":"该原件在清理窗口内改变；保留新状态，未删除。"}));
                    save(journal, j)?;
                    continue;
                }
            }
            j["steps"].as_array_mut().unwrap().push(json!({"id":"clear","label":file["path"],"status":"executing","message":"仅移除预览冻结的原件；使用隔离重命名，不覆盖新状态。"}));
            save(journal, j)?;
            let quarantine = path
                .parent()
                .ok_or_else(|| err("invalid_path", "残留原件缺少父目录"))?
                .join(format!(".lintel-quarantine-{}-residue", string(j, "id")?));
            j["quarantine_path"] = json!(quarantine);
            save(journal, j)?;
            match cleanup::quarantine_remove_bounded(
                path,
                &file["snapshot"],
                &quarantine,
                RESIDUE_FILE_LIMIT,
            ) {
                Ok(()) if !path.exists() => {
                    removed += 1;
                    *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":"clear","label":file["path"],"status":"completed","message":"原件已移除；备份（如启用）保留在任务记录中。"});
                }
                Ok(()) => {
                    failures.push(json!({"path":file["path"],"reason":"state_reappeared"}));
                    *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":"clear","label":file["path"],"status":"failed","message":"移除后检测到同路径内容重新出现；请核对写入来源。"});
                }
                Err(error) => {
                    failures.push(
                        json!({"path":file["path"],"code":error.code,"message":error.message}),
                    );
                    *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":"clear","label":file["path"],"status":"failed","message":error.message});
                }
            }
            save(journal, j)?;
        }
        j["residue_clear"] = json!({
            "schema": RESIDUE_SCHEMA,
            "selected": files.len(),
            "removed": removed,
            "skipped_absent": skipped_absent,
            "skipped": skipped_absent + conflicts.len(),
            "conflicts": conflicts,
            "failed": failures,
        });
        if failures.is_empty() && conflicts.is_empty() {
            j["status"] = json!("completed");
            j["local_cleanup"] = json!("completed");
        } else {
            j["status"] = json!("partially_completed");
            j["warnings"].as_array_mut().unwrap().push(json!(
                "部分残留原件未移除（见 residue_clear.failed）；其余已独立完成，备份保留在任务记录中。"
            ));
        }
        j["warnings"].as_array_mut().unwrap().push(json!(
            "此结果只覆盖预览中明确选择的残留原件；未选中的残留、工作内容、身份与登录材料保持不变。"
        ));
        Ok(())
    }

    /// Preview resetting the local client identifier.
    ///
    /// Freezes exactly the verified persistent identifier field (`userID`) and
    /// the explicitly selected backup files. Execution writes one independent
    /// encrypted backup of both, then removes the frozen field and files, keeping
    /// every other client-state key value-by-value. Logout, credentials and the
    /// browser profile are separate recipes and are never touched here.
    pub(crate) fn plan_identity_reset(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        if r["writers_confirmed_stopped"] != true {
            return Err(err(
                "writers_confirmation_required",
                "先关闭目标终端、IDE 与自动重启来源，再预览标识重置",
            ));
        }
        if !self.known_writers(&e)?.is_empty() {
            return Err(err(
                "writers_active",
                "仍检测到 Claude 进程且无法确认所属 root；请核对并关闭目标写入者，Lintel 不会全局杀进程",
            ));
        }
        let root = PathBuf::from(string(&e, "root")?);
        guard(&root)?;
        let path = residue::client_state_path(&self.home, &root);
        let doc = match residue::read_client_state(&path) {
            residue::ClientState::Observed(doc) => doc,
            state => {
                return Err(err(
                    "identity_unknown",
                    &format!(
                        "client_state 状态为 {}，没有可核实的持久标识字段；未生成重置计划。",
                        state.code()
                    ),
                ))
            }
        };
        let user_id = doc
            .get("userID")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                err(
                    "identity_absent",
                    "client_state 中没有可核实的非空 userID；未生成重置计划。",
                )
            })?;
        let _ = user_id;
        // Freeze the exact object and bytes; the raw value itself never enters
        // the plan, only the fact that this field is present and removable.
        let snapshot = snapshot(&path)?;
        let meta = fs::metadata(&path)?;
        let retained: Vec<String> = doc
            .as_object()
            .map(|map| {
                let mut keys: Vec<String> = map
                    .keys()
                    .filter(|key| key.as_str() != "userID")
                    .cloned()
                    .collect();
                keys.sort();
                keys
            })
            .unwrap_or_default();
        let mut backups = vec![];
        if let Some(list) = r["identity_backup_paths"].as_array() {
            let mut seen = std::collections::HashSet::new();
            let mut total = 0u64;
            for raw in list {
                let raw = raw.as_str().filter(|s| !s.is_empty()).ok_or_else(|| {
                    err(
                        "invalid_backup_paths",
                        "identity_backup_paths 只能包含非空绝对路径字符串",
                    )
                })?;
                let backup = Path::new(raw);
                guard(backup)?;
                if backup == path
                    || backup == root.join(".credentials.json")
                    || backup == root.join("settings.json")
                    || backup.starts_with(&self.state)
                {
                    return Err(err(
                        "protected_local_item",
                        "身份备份选择不能包含原状态、登录凭据、设置或 Lintel 状态",
                    ));
                }
                if !backup.is_absolute() || !seen.insert(backup.to_path_buf()) {
                    return Err(err(
                        "invalid_backup_paths",
                        "identity_backup_paths 需要唯一的绝对路径",
                    ));
                }
                let backup_meta = fs::symlink_metadata(backup)
                    .map_err(|_| err("residue_missing", "所选身份备份文件不存在或不可读"))?;
                if backup_meta.file_type().is_symlink() || !backup_meta.is_file() {
                    return Err(err("file_limit", "所选身份备份必须是常规文件"));
                }
                if backup_meta.uid() != unsafe { libc::geteuid() } || backup_meta.nlink() != 1 {
                    return Err(err(
                        "unsafe_file",
                        "所选身份备份的所有者或硬链接状态不受支持",
                    ));
                }
                if backup_meta.len() > IDENTITY_BACKUP_BYTES {
                    return Err(err("file_limit", "所选身份备份超过单文件容量上限（8 MiB）"));
                }
                total += backup_meta.len();
                if total > IDENTITY_BACKUP_TOTAL {
                    return Err(err(
                        "archive_limit",
                        "所选身份备份合计超过容量上限（32 MiB）；未写入任何备份",
                    ));
                }
                backups.push(json!({
                    "path": backup,
                    "bytes": backup_meta.len(),
                    "digest": digest_file_bounded(backup, IDENTITY_BACKUP_BYTES)?,
                    "snapshot": cleanup::bounded_snapshot(backup, IDENTITY_BACKUP_BYTES)?,
                    "identity": {"device":backup_meta.dev(),"inode":backup_meta.ino(),
                        "owner":backup_meta.uid(),"mode":backup_meta.mode(),
                        "mtime":backup_meta.mtime(),"mtime_nsec":backup_meta.mtime_nsec(),
                        "ctime":backup_meta.ctime(),"ctime_nsec":backup_meta.ctime_nsec()},
                }));
            }
        }
        let services = self.check_cleanup_services(&e)?;
        let mut actions = vec![
            json!({"id":"quiescence","label":"复查目标已停止写入；不关闭其他环境","reversible":false}),
        ];
        actions.push(json!({"id":"identity_backup","label":"先写入独立加密标识备份并读回","reversible":false}));
        actions.push(json!({"id":"remove_field","label":"从 client_state 移除已核实的 userID 字段，保留其他键逐值","reversible":false}));
        for backup in &backups {
            actions.push(json!({"id":"remove_backup","label":format!("在保全后移除所选身份备份：{}", backup["path"].as_str().unwrap_or("")),"reversible":false}));
        }
        self.plan(
            &e,
            "cleanup",
            "重置本地客户端标识",
            json!([]),
            vec![
                "登录与凭据（单独选择官方注销或凭据清理）",
                "client_state 中除 userID 以外的其他键",
                "工作内容、未选中的身份备份与其他环境",
            ],
            json!(actions),
            json!({
                "recipe": IDENTITY_RESET_RECIPE,
                "official_logout": false,
                "outcome": "identity_reset",
                "identity": {
                    "path": path,
                    "state": "observed",
                    "remove_keys": ["userID"],
                    "retained_keys": retained,
                    "snapshot": snapshot,
                    "bytes": meta.len(),
                    "mode": meta.permissions().mode() & 0o777,
                },
                "backups": backups,
                "services": services,
                "archive_passphrase_required": true,
                "executable": e["executable"],
            }),
        )
    }

    /// Pre-execution recheck for an `identity_reset` plan.
    pub(crate) fn check_identity_reset(&self, p: &Value, r: &Value) -> Result<()> {
        if !self
            .known_writers(&json!({"id":p["environment_id"],"root":p["root"]}))?
            .is_empty()
        {
            return Err(err("writers_active", "发现目标写入者；未开始重置"));
        }
        if json!(self.check_cleanup_services(&json!({"id":p["environment_id"],"root":p["root"]}))?)
            != p["extra"]["services"]
        {
            return Err(err(
                "stale_service_plan",
                "预览后 service hold 证据变化；未开始重置",
            ));
        }
        work::check_passphrase(r)?;
        let identity = &p["extra"]["identity"];
        let path = PathBuf::from(string(identity, "path")?);
        if snapshot(&path)? != identity["snapshot"] {
            return Err(err(
                "stale_plan",
                "预览后 client_state 已被修改；不会覆盖新的状态，请重新预览标识重置",
            ));
        }
        for backup in p["extra"]["backups"].as_array().unwrap_or(&vec![]) {
            recheck_residue_original(backup)?;
        }
        Ok(())
    }

    /// Execute an `identity_reset` plan. Preservation strictly precedes every
    /// removal; the receipt reports the removal and readback result, and the
    /// `regenerated` question is answered from the actual recheck rather than a
    /// claim.
    pub(crate) fn identity_reset(
        &self,
        e: &Value,
        p: &Value,
        r: &Value,
        j: &mut Value,
        journal: &Path,
    ) -> Result<()> {
        let pass = work::check_passphrase(r)?;
        let identity = &p["extra"]["identity"];
        let path = PathBuf::from(string(identity, "path")?);
        let remove_keys = identity["remove_keys"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        // Step 0: recheck the frozen object and the passphrase before writing
        // anything. A plan whose client-state changed (or whose passphrase is
        // absent) publishes no backup and removes nothing.
        self.check_identity_reset(p, r)?;
        // Step 1: one independent encrypted package holding the exact identity
        // document bytes plus every selected backup. This is written and read
        // back before anything is removed.
        let mut staged = vec![json!({
            "path": path,
            "role": "client_state",
            "bytes": identity["bytes"],
            "digest": digest_file_bounded(&path, IDENTITY_BYTES)?,
            "data": read(&path, IDENTITY_BYTES)?,
        })];
        for backup in p["extra"]["backups"].as_array().unwrap_or(&vec![]) {
            staged.push(json!({
                "path": backup["path"],
                "role": "identity_backup",
                "bytes": backup["bytes"],
                "digest": backup["digest"],
                "data": read_residue_original(backup)?,
            }));
        }
        let bytes = archive::seal(&residue_package(string(e, "id")?, &staged), pass)?;
        let dest = self
            .state
            .join("archives")
            .join(format!("{}-identity.age", string(j, "id")?));
        j["steps"].as_array_mut().unwrap().push(json!({"id":"identity_backup","label":"独立加密标识备份","status":"executing","message":"正在写入并读回标识备份；备份成功前不会移除任何字段或文件。"}));
        j["identity_archive_path"] = json!(dest);
        save(journal, j)?;
        publish_sealed(&dest, &bytes)?;
        if read(&dest, 64 * 1024 * 1024)? != bytes {
            return Err(err(
                "archive_readback_failed",
                "标识备份未能读回；未做任何移除",
            ));
        }
        *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":"identity_backup","label":"独立加密标识备份","status":"completed","message":"标识备份已写入并读回，可独立解密；不写入任何工作包。"});
        save(journal, j)?;
        // Step 2: recheck again after preservation, immediately before the
        // destructive boundary, so a change during the backup window stops here.
        self.check_identity_reset(p, r)?;
        // Step 3: rewrite client_state with exactly the frozen keys removed and
        // every other key kept value-by-value. Only the field removal happens
        // here; the login token store is untouched.
        let mut doc = match residue::read_client_state(&path) {
            residue::ClientState::Observed(doc) => doc,
            state => {
                return Err(err(
                    "identity_unknown",
                    &format!(
                        "备份完成后 client_state 变为 {}；未移除任何字段",
                        state.code()
                    ),
                ))
            }
        };
        let mut removed_fields = vec![];
        if let Some(map) = doc.as_object_mut() {
            for key in &remove_keys {
                if let Some(key) = key.as_str() {
                    if map.remove(key).is_some() {
                        removed_fields.push(json!(key));
                    }
                }
            }
        }
        if snapshot(&path)? != identity["snapshot"] {
            return Err(err(
                "stale_plan",
                "发布前发现 client_state 变化；未移除任何字段",
            ));
        }
        let mode = identity["mode"].as_u64().unwrap_or(0o600) as u32 & 0o777 & !0o022;
        j["steps"].as_array_mut().unwrap().push(json!({"id":"remove_field","label":"移除已核实的持久标识字段","status":"executing","message":"只移除预览冻结的字段；其余键逐值保留。"}));
        save(journal, j)?;
        atomic_recorded(
            &path,
            &serde_json::to_vec_pretty(&doc)?,
            if mode == 0 { 0o600 } else { mode },
            |staged_file| {
                if snapshot(&path)? != identity["snapshot"] {
                    return Err(err("stale_plan", "发布前发现 client_state 变化"));
                }
                j["identity_write"] = json!({"state":"prepared","before":identity["snapshot"],"after":snapshot(staged_file)?});
                save(journal, j)?;
                Ok(())
            },
        )?;
        j["identity_write"]["state"] = json!("written");
        save(journal, j)?;
        if load(&path)? != doc {
            return Err(err("readback_failed", "写后读回不同，请核对其他写入者"));
        }
        *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":"remove_field","label":"移除已核实的持久标识字段","status":"completed","message":"字段已移除并读回；其他键逐值保留。"});
        save(journal, j)?;
        // Step 4: remove exactly the selected, preserved backup files.
        let mut removed_backups = 0usize;
        let mut backup_failures: Vec<Value> = vec![];
        for backup in p["extra"]["backups"]
            .as_array()
            .cloned()
            .unwrap_or_default()
        {
            let backup_path = PathBuf::from(string(&backup, "path")?);
            match recheck_residue_original(&backup) {
                Ok(false) => continue,
                Ok(true) => {}
                Err(error) => {
                    backup_failures.push(
                        json!({"path":backup["path"],"code":error.code,"message":error.message}),
                    );
                    continue;
                }
            }
            let quarantine = backup_path
                .parent()
                .ok_or_else(|| err("invalid_path", "身份备份缺少父目录"))?
                .join(format!(".lintel-quarantine-{}-identity", string(j, "id")?));
            j["quarantine_path"] = json!(quarantine);
            save(journal, j)?;
            match cleanup::quarantine_remove_bounded(
                &backup_path,
                &backup["snapshot"],
                &quarantine,
                IDENTITY_BACKUP_BYTES,
            ) {
                Ok(()) if !backup_path.exists() => {
                    removed_backups += 1;
                    j["steps"].as_array_mut().unwrap().push(json!({"id":"remove_backup","label":backup["path"],"status":"completed","message":"已保全的身份备份原件已移除。"}));
                }
                Ok(()) => {
                    backup_failures
                        .push(json!({"path":backup["path"],"reason":"state_reappeared"}));
                }
                Err(error) => backup_failures
                    .push(json!({"path":backup["path"],"code":error.code,"message":error.message})),
            }
            save(journal, j)?;
        }
        // Step 5: report the reset from an actual re-read. Whether the client
        // regenerates a new identifier on its next run is unknowable here, so it
        // stays `unknown`; only the observed removal is claimed.
        let (removal_verified, state_code) = match residue::read_client_state(&path) {
            residue::ClientState::Observed(doc) => (
                remove_keys
                    .iter()
                    .filter_map(Value::as_str)
                    .all(|key| doc.get(key).is_none()),
                "observed",
            ),
            state => (false, state.code()),
        };
        j["identity_reset"] = json!({
            "path": path,
            "removed_keys": removed_fields,
            "retained_keys": identity["retained_keys"],
            "removed_backups": removed_backups,
            "backup_failures": backup_failures,
            "state_after": state_code,
            "removal_verified": removal_verified,
            "regenerated": "unknown",
            "note": "是否会在下次运行时重新生成新的标识取决于客户端实现，本次未能核实，故为 unknown。",
        });
        j["status"] = if backup_failures.is_empty() && removal_verified {
            json!("completed")
        } else {
            json!("partially_completed")
        };
        j["warnings"].as_array_mut().unwrap().push(json!(
            "本次只移除已核实的持久标识字段与明确选中的备份；登录、凭据、浏览器 profile 与其他配置保持不变，如需清理请分别选择对应配方。"
        ));
        Ok(())
    }
}

#[cfg(test)]
#[path = "client_data_tests.rs"]
mod tests;

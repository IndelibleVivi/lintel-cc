use crate::{
    err,
    package::{self, Package, Staging},
    storage::*,
    string, work, Engine, Result,
};
use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

#[cfg(test)]
pub(crate) const MAX_PLAIN: u64 = 200 * 1024 * 1024;

/// Legacy warm-path seal: encrypt a small in-memory `Value` (used for the
/// state-backup package and test fixtures, never for large work content).
pub(crate) fn seal(value: &Value, pass: &str) -> Result<Vec<u8>> {
    package::write_value(value, pass)
}

/// Legacy warm-path unseal: decrypt a small in-memory `Value`. Bounded by
/// `MAX_PLAIN`; large work packages must go through [`Engine::open_archive`].
#[cfg(test)]
pub(crate) fn unseal(bytes: &[u8], pass: &str) -> Result<Value> {
    let decryptor =
        age::Decryptor::new(bytes).map_err(|_| err("invalid_archive", "不是有效的 age 归档"))?;
    let identity = age::scrypt::Identity::new(age::secrecy::SecretString::from(pass.to_owned()));
    let reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|_| err("archive_locked", "口令不正确，或归档无法解密"))?;
    let mut plain = vec![];
    use std::io::Read;
    reader
        .take(MAX_PLAIN + 1)
        .read_to_end(&mut plain)
        .map_err(|_| err("invalid_archive", "归档解密或完整性校验失败"))?;
    if plain.len() as u64 > MAX_PLAIN {
        return Err(err("archive_limit", "归档展开后超过容量上限"));
    }
    parse(&plain)
}

/// A decoded work package plus the private staging that owns its plaintext.
pub(crate) struct OpenedArchive {
    pub package: Package,
    pub cipher_digest: String,
    pub path: PathBuf,
    pub _staging: Staging,
}

impl OpenedArchive {
    /// Find one entry by its frozen relative path.
    pub(crate) fn entry(&self, name: &str) -> Result<&package::Entry> {
        self.package
            .files
            .iter()
            .find(|file| file.path == name)
            .ok_or_else(|| err("archive_file_missing", "归档没有该文件"))
    }

    /// Explicitly close this read's private decoded plaintext tree, reporting
    /// whether it is actually gone. Launch callers use this before a possible
    /// `exec`, which would skip [`OpenedArchive`]'s normal drop and leave the
    /// full decoded package (including unselected sensitive members) behind.
    pub(crate) fn close(self) -> Result<()> {
        self._staging.close()
    }
}

impl Engine {
    /// Resolve the frozen archive source for a read/inspect/import request.
    ///
    /// Accepts exactly one of a `job_id` (this install's recorded archive) or an
    /// explicit absolute `archive_path` (a package carried to an independent
    /// install). The package is self-describing; readers never consult the
    /// original inventory.
    fn archive_source(&self, r: &Value) -> Result<PathBuf> {
        let job = r.get("job_id").and_then(Value::as_str).is_some();
        let path = r.get("archive_path").and_then(Value::as_str).is_some();
        if job == path {
            return Err(err(
                "invalid_request",
                "请只提供 job_id 或 archive_path 之一",
            ));
        }
        if job {
            let jid = crate::safe_id(r, "job_id")?;
            let record = load(&self.path("jobs", &jid))?;
            let path = record["archive_path"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| err("archive_missing", "此任务没有可读取的工作归档"))?;
            Ok(PathBuf::from(path))
        } else {
            let raw = string(r, "archive_path")?;
            let path = Path::new(raw);
            guard(path)?;
            if !path.is_absolute() {
                return Err(err("invalid_path", "归档路径必须是绝对路径"));
            }
            Ok(path.to_path_buf())
        }
    }

    /// The recorded ciphertext digest for a job-scoped read, when the original
    /// receipt froze one. A job-scoped read also confirms the recorded path.
    fn recorded_digest(&self, r: &Value, path: &Path) -> Result<Option<String>> {
        match r.get("job_id").and_then(Value::as_str) {
            None => Ok(None),
            Some(jid) => {
                let record = load(&self.path("jobs", jid))?;
                if record["archive_path"].as_str() != path.to_str() {
                    return Err(err("archive_missing", "此任务的归档来源已经变化"));
                }
                Ok(record["archive_digest"]
                    .as_str()
                    .or_else(|| record["archive_intent_digest"].as_str())
                    .map(str::to_owned))
            }
        }
    }

    /// Stream-decode a work package into metadata plus a private staging
    /// directory of per-file plaintext. Validates paths, categories,
    /// duplicates, per-file digests and the full ciphertext digest.
    pub(crate) fn open_archive(&self, r: &Value) -> Result<OpenedArchive> {
        let path = self.archive_source(r)?;
        let recorded = self.recorded_digest(r, &path)?;
        let pass = work::check_passphrase(r)?;
        // Staging lives in Lintel's own private state so a carried package does
        // not scatter decoded plaintext next to the archive.
        let staging = Staging::new(&self.state)?;
        let (package, cipher_digest) =
            package::read_to_temp(&path, pass, &staging, work::limits())?;
        // Path/category/duplicate validation runs on the decoded package first,
        // so a hostile external package is reported by its own defect; the
        // recorded-digest binding then refuses any replaced job archive.
        work::validate_package(&package)?;
        if recorded.is_some_and(|expected| expected != cipher_digest) {
            return Err(err(
                "stale_archive",
                "归档字节与原任务记录不一致；请核对原任务和归档来源",
            ));
        }
        Ok(OpenedArchive {
            package,
            cipher_digest,
            path,
            _staging: staging,
        })
    }

    pub(crate) fn archive_inspect(&self, r: &Value) -> Result<Value> {
        let opened = self.open_archive(r)?;
        let files: Vec<Value> = opened
            .package
            .files
            .iter()
            .map(|file| {
                json!({"path":file.path,"category":file.category,"bytes":file.bytes,"digest":file.digest})
            })
            .collect();
        Ok(
            json!({"job_id":r["job_id"],"archive_path":opened.path,"created_at":opened.package.created_at,"generator":opened.package.generator.as_str(),"schema":opened.package.schema,"categories":category_summary(&files),"files":files,"notes":"指令可迁入原位置；会话与记忆保留在 lintel-imports，不自动激活 hooks/MCP，也不保证原会话可以续聊。generator 是包内自声明元数据，缺失时为 null，不是来源认证。此清单只描述包本身，不代表来源安装或来源 job 仍存在。"}),
        )
    }

    pub(crate) fn archive_read(&self, r: &Value) -> Result<Value> {
        let opened = self.open_archive(r)?;
        let name = string(r, "path")?;
        let entry = opened.entry(name)?;
        let file = std::fs::File::open(&entry.plain)?;
        // Only the first bounded window is read for the preview.
        let mut window = vec![0u8; entry.bytes.min(1024 * 1024) as usize];
        read_exact_window(file, &mut window)?;
        Ok(json!({
            "path": name,
            "text": String::from_utf8_lossy(&window),
            "bytes": entry.bytes,
            "truncated": entry.bytes > window.len() as u64,
        }))
    }

    pub(crate) fn plan_import(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let opened = self.open_archive(r)?;
        let cats = work::categories(r)?;
        let root = Path::new(string(&e, "root")?);
        let selected: Vec<&package::Entry> = opened
            .package
            .files
            .iter()
            .filter(|file| cats.iter().any(|category| *category == file.category))
            .collect();
        if selected.is_empty() {
            return Err(err("empty_import", "归档中没有选中类别的文件"));
        }
        let activation = work::activation(r, &cats);
        let instructions_active = activation["instructions"] == true;
        let selection_paths: Vec<String> =
            selected.iter().map(|entry| entry.path.clone()).collect();
        let targets = work::migration_paths_for(&selection_paths, instructions_active)?;
        let mut files = vec![];
        for (entry, relative) in selected.iter().zip(&targets) {
            let p = root.join(relative);
            work::preflight_import_parent(root, &p)?;
            if p.exists() {
                return Err(err(
                    "import_conflict",
                    "目标已有同名内容；请选择新的环境，已有文件未覆盖",
                ));
            }
            files.push(json!({"path":entry.path,"category":entry.category,"digest":entry.digest,"destination":p,"bytes":entry.bytes}));
        }
        let frozen_target = work::freeze_existing_target(root, &files, instructions_active)?;
        self.plan(
            &e,
            "import",
            "迁入选定工作内容",
            json!([]),
            vec!["目标环境现有文件", "登录、hooks、MCP 与插件配置"],
            json!([{"id":"import","label":"解密并迁入所选类别，逐文件读回","reversible":false}]),
            json!({
                "original_job": r.get("job_id").cloned().unwrap_or(Value::Null),
                "archive_path": opened.path,
                "archive_digest": opened.cipher_digest,
                "package_format": opened.package.schema,
                "package_generator": opened.package.generator.as_str(),
                "manifest": files,
                "categories": cats,
                "archive_passphrase_required": true,
                "frozen_target": frozen_target,
                "work_purpose": work::purposes(&cats),
                "activate": activation,
            }),
        )
    }

    pub(crate) fn import_work(
        &self,
        e: &Value,
        p: &Value,
        r: &Value,
        j: &mut Value,
        journal: &Path,
    ) -> Result<()> {
        let source = if p["extra"]["original_job"].is_string() {
            json!({"job_id":p["extra"]["original_job"],"archive_passphrase":r["archive_passphrase"]})
        } else {
            json!({"archive_path":p["extra"]["archive_path"],"archive_passphrase":r["archive_passphrase"]})
        };
        let opened = self.open_archive(&source)?;
        if opened.cipher_digest != p["extra"]["archive_digest"] {
            return Err(err("stale_archive", "预览后归档发生变化"));
        }
        let root = Path::new(string(e, "root")?);
        let frozen = &p["extra"]["frozen_target"];
        if frozen["plan_revision"] == "lintel.plan/2" {
            let metadata = std::fs::metadata(root)?;
            use std::os::unix::fs::MetadataExt;
            if json!([metadata.dev(), metadata.ino()]) != frozen["new_root_parent_identity"] {
                return Err(err(
                    "stale_plan",
                    "目标配置目录的对象在预览后变化，请重新预览",
                ));
            }
        }
        let planned = p["extra"]["manifest"]
            .as_array()
            .ok_or_else(|| err("invalid_plan", "缺少迁入清单"))?;
        let selected: Vec<&package::Entry> = planned
            .iter()
            .map(|entry| {
                let path = entry["path"].as_str().unwrap_or_default();
                let digest = entry["digest"].as_str().unwrap_or_default();
                opened
                    .package
                    .files
                    .iter()
                    .find(|file| file.path == path && file.digest == digest)
                    .ok_or_else(|| err("stale_archive", "归档与计划不匹配"))
            })
            .collect::<Result<_>>()?;
        let instructions_active = p["extra"]["activate"]["instructions"]
            .as_bool()
            .unwrap_or(true);
        let selected_paths: Vec<String> = selected.iter().map(|e| e.path.clone()).collect();
        let relative_targets = work::migration_paths_for(&selected_paths, instructions_active)?;
        let targets: Vec<PathBuf> = relative_targets
            .iter()
            .map(|relative| root.join(relative))
            .collect();
        let total: u64 = planned
            .iter()
            .map(|v| v["bytes"].as_u64().unwrap_or(0))
            .sum();
        if fs2::available_space(root)? < total + 1024 * 1024 {
            return Err(err("insufficient_space", "目标空间不足；未覆盖原文件"));
        }
        for (entry, target) in planned.iter().zip(&targets) {
            work::preflight_import_parent(root, target)?;
            if entry["destination"].as_str() != target.to_str() {
                return Err(err("stale_plan", "迁入目标与冻结清单不一致；请重新预览"));
            }
            if target.exists() {
                return Err(err("import_conflict", "预览后出现同名内容；没有覆盖"));
            }
        }
        work::preflight_migration_paths(root, &relative_targets, j, journal)?;
        for ((entry, source), target) in planned.iter().zip(&selected).zip(&targets) {
            work::migration_parent(target.parent().unwrap())?;
            j["steps"].as_array_mut().unwrap().push(json!({"id":entry["path"],"label":entry["path"],"status":"executing","message":"正在发布并核验迁入文件；不替换已有内容。"}));
            save(journal, j)?;
            // Stream-copy the staged plaintext into the target and read it back,
            // hashing both, so a large file never lands fully in memory.
            copy_staged_verified(&source.plain, target, source.digest.as_str())?;
            *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":entry["path"],"label":entry["path"],"status":"completed","message":"文件已迁入并校验；可执行配置未激活。"});
            save(journal, j)?;
        }
        j["status"] = json!("completed");
        j["completed_at"] = json!(crate::now());
        Ok(())
    }
}

/// Read up to `buf.len()` bytes from the front of `file` (fewer only at EOF).
fn read_exact_window(mut file: std::fs::File, buf: &mut Vec<u8>) -> Result<()> {
    use std::io::Read;
    let capacity = buf.len();
    let mut filled = 0;
    while filled < capacity {
        match file.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(err("archive_read_failed", "无法读取归档文件")),
        }
    }
    buf.truncate(filled);
    Ok(())
}

/// Publish `source` at `target` with `atomic_new` semantics and verify the
/// readback digest matches `expected`. Streams in bounded chunks.
pub(crate) fn copy_staged_verified(source: &Path, target: &Path, expected: &str) -> Result<()> {
    atomic_new_from_file(target, source, expected)?;
    let digest = digest_file(target)?;
    if digest != expected {
        return Err(err("readback_failed", "迁入后读回不匹配"));
    }
    Ok(())
}

/// Copy `source` into a brand-new file at `target` without replacement,
/// streaming in bounded chunks. Refuses a concurrent target.
fn atomic_new_from_file(target: &Path, source: &Path, expected: &str) -> Result<()> {
    use sha2::Digest as _;
    use std::io::{Read, Write};
    guard(target)?;
    guard(source)?;
    let mut input = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(source)?;
    if !input.metadata()?.is_file() || input.metadata()?.len() > work::FILE_LIMIT_BYTES {
        return Err(err("archive_limit", "暂存来源不是有限普通文件"));
    }
    let parent = target
        .parent()
        .ok_or_else(|| err("invalid_path", "迁入目标缺少父目录"))?;
    let tmp = parent.join(format!(".lintel-import-{}.tmp", uuid::Uuid::new_v4()));
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&tmp)?;
    // Arm the guard only after `create_new` proved this process created the
    // name. It then owns exactly this temp name across the bounded
    // read/verify/write and the no-replace publication, so a normal error on any
    // step (including a post-rename parent-sync failure) removes only that
    // unpublished name, never a pre-existing or unrelated file.
    let mut staged = crate::storage::StagedTemp::new(tmp.clone());
    let result = (|| -> Result<()> {
        let mut hash = sha2::Sha256::new();
        let mut count = 0u64;
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = match input.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return Err(err("io_error", "读取来源失败")),
            };
            count += n as u64;
            if count > work::FILE_LIMIT_BYTES {
                return Err(err("archive_limit", "暂存来源在读取期间超过容量上限"));
            }
            output.write_all(&buf[..n])?;
            hash.update(&buf[..n]);
        }
        if format!("{:x}", hash.finalize()) != expected {
            return Err(err(
                "readback_failed",
                "暂存来源与冻结摘要不一致；未发布迁入文件",
            ));
        }
        crate::storage::sync_staged_file(&output)?;
        // Publish the completed file with the same no-replace primitive as all
        // other new-file mutations, which also flushes the destination parent
        // directory so the new name survives an interruption. An interrupted
        // copy leaves no partial file at its approved final name.
        publish_staged_new(&tmp, target)?;
        staged.disarm();
        Ok(())
    })();
    drop(output);
    drop(staged);
    result
}

/// Streaming SHA-256 of a file.
pub(crate) fn digest_file(path: &Path) -> Result<String> {
    use sha2::Digest as _;
    use std::io::Read;
    guard(path)?;
    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(err("readback_failed", "读回目标不是普通文件"));
    }
    let mut count = 0u64;
    let mut hasher = sha2::Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        count += n as u64;
        if count > work::FILE_LIMIT_BYTES {
            return Err(err("archive_limit", "读回目标超过容量上限"));
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Per-category file count and byte total for an archive manifest.
fn category_summary(files: &[Value]) -> Value {
    let mut out: std::collections::BTreeMap<&str, (u64, u64)> = Default::default();
    for f in files {
        if let Some(category) = f["category"].as_str() {
            let entry = out.entry(category).or_default();
            entry.0 += 1;
            entry.1 += f["bytes"].as_u64().unwrap_or(0);
        }
    }
    json!(["instructions", "memory", "sessions"]
        .iter()
        .map(|c| {
            let (count, bytes) = out.get(*c).copied().unwrap_or((0, 0));
            json!({"category": c, "count": count, "bytes": bytes})
        })
        .collect::<Vec<_>>())
}

#[cfg(test)]
mod streaming_publication_tests {
    use super::*;
    #[test]
    fn verified_copy_publishes_complete_bytes_and_preserves_concurrent_targets() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let source = base.join("source.bin");
        let target = base.join("target.bin");
        fs::write(&source, b"synthetic exact original").unwrap();
        let expected = digest(b"synthetic exact original");
        assert_eq!(
            copy_staged_verified(&source, &target, &digest(b"wrong"))
                .unwrap_err()
                .code,
            "readback_failed"
        );
        assert!(!target.exists());
        assert_eq!(
            fs::read_dir(&base).unwrap().count(),
            1,
            "failed copy left a plaintext temp file"
        );
        fs::write(&target, b"neighbor owns this name").unwrap();
        assert!(copy_staged_verified(&source, &target, &expected).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"neighbor owns this name");
        assert_eq!(fs::read_dir(&base).unwrap().count(), 2);
        let fresh = base.join("fresh.bin");
        copy_staged_verified(&source, &fresh, &expected).unwrap();
        assert_eq!(fs::read(&fresh).unwrap(), b"synthetic exact original");
        use std::os::unix::fs::MetadataExt;
        assert_eq!(fs::metadata(&fresh).unwrap().nlink(), 1);
        assert_eq!(fs::metadata(&fresh).unwrap().mode() & 0o777, 0o600);
    }

    #[test]
    fn verified_copy_parent_sync_failure_is_reported_and_leaves_no_temp() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let source = base.join("source.bin");
        let target = base.join("target.bin");
        fs::write(&source, b"synthetic exact original").unwrap();
        let expected = digest(b"synthetic exact original");
        // The imported copy's rename lands, but the destination directory flush
        // fails: the streaming publication must report an error (the migration
        // step cannot be "completed") and leave no temp or republish.
        crate::storage::sync_fault::arm_owner();
        let result = copy_staged_verified(&source, &target, &expected);
        crate::storage::sync_fault::disarm_owner();
        assert!(result.is_err(), "parent sync failure reported as success");
        assert_eq!(fs::read(&target).unwrap(), b"synthetic exact original");
        assert_eq!(
            fs::read_dir(&base).unwrap().count(),
            2,
            "parent sync failure left a temp or dropped the source"
        );
    }

    #[test]
    fn verified_copy_staged_sync_failure_leaves_no_temp_or_target() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().canonicalize().unwrap();
        let source = base.join("source.bin");
        let target = base.join("target.bin");
        fs::write(&source, b"synthetic exact original").unwrap();
        let expected = digest(b"synthetic exact original");
        crate::storage::sync_fault::arm_staged();
        let result = copy_staged_verified(&source, &target, &expected);
        crate::storage::sync_fault::disarm_staged();
        assert!(result.is_err(), "staged sync failure reported as success");
        assert!(!target.exists());
        assert_eq!(
            fs::read_dir(&base).unwrap().count(),
            1,
            "staged sync failure left a temp"
        );
    }
}

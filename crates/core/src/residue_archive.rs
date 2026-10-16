//! Independent reader and restore path for `lintel.residue/1` backups.
//!
//! Residue backups are produced by the local-data recipes (`clear_residue`,
//! `identity_reset`) and are deliberately a *different* schema from
//! `lintel.work/1`: they are never work content, never imported into an
//! environment, and never read by the work codec. This module owns the finite
//! read side: a private on-demand manifest view and an explicit, exact-path
//! restore that only writes back the original recorded locations.
//!
//! The package is a bounded age passphrase envelope over one JSON object, the
//! same wire shape the shared work codec writes. It is decoded here in memory
//! under strict limits and full per-entry validation; decoded bytes are never
//! persisted, logged or returned in a manifest.
use crate::{err, now, storage::*, string, Engine, Result};
use serde::{
    de::{self, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::{json, Value};
use std::{
    fs,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

/// The plan kind this module executes. It is admitted by `Engine::execute`
/// through the coordinator's hook and never falls through to settings writes.
pub(crate) const RESIDUE_RESTORE_KIND: &str = "residue_restore";
/// The only package schema this module accepts.
pub(crate) const RESIDUE_SCHEMA: &str = "lintel.residue/1";
/// Ciphertext bound: the JSON byte-array envelope plus age framing, generously
/// bounded so a hostile file cannot make the reader allocate without limit.
const MAX_CIPHER_BYTES: u64 = 256 * 1024 * 1024;
/// Plaintext bound for one decoded package.
const MAX_PLAIN_BYTES: u64 = 192 * 1024 * 1024;
/// Per-entry plaintext bound, mirroring the writer's admission.
const FILE_LIMIT_BYTES: u64 = 8 * 1024 * 1024;
/// Aggregate plaintext bound across all entries.
const TOTAL_LIMIT_BYTES: u64 = 32 * 1024 * 1024;
/// File-count bound for one package.
const MAX_FILES: usize = 2000;
/// Bound on untrusted passphrase work, so a hostile header cannot burn CPU.
const MAX_SCRYPT_LOG_N: u8 = 20;

/// The private, decoded residue package. `files` hold the exact recorded bytes
/// in memory; nothing is written to disk by a read.
struct ResiduePackage {
    cipher_digest: String,
    environment_id: Option<String>,
    files: Vec<ResidueEntry>,
}

struct ResidueEntry {
    /// The absolute original location recorded when the backup was written.
    path: PathBuf,
    /// The role marker written by the producer (`client_state`,
    /// `identity_backup`, ...); free-form but bounded by the read.
    role: String,
    bytes: u64,
    digest: String,
    data: Vec<u8>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DecodedEntry {
    path: String,
    #[serde(default)]
    role: String,
    bytes: u64,
    digest: String,
    #[serde(deserialize_with = "bounded_data")]
    data: Vec<u8>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DecodedPackage {
    schema: String,
    #[serde(default, rename = "generator")]
    _generator: Option<String>,
    #[serde(default, rename = "created_at")]
    _created_at: Option<String>,
    #[serde(default, rename = "environment_id")]
    _environment_id: Option<String>,
    #[serde(default, rename = "notes")]
    _notes: Option<String>,
    #[serde(deserialize_with = "bounded_entries")]
    files: Vec<DecodedEntry>,
}
fn bounded_data<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Vec<u8>, D::Error> {
    struct Bytes;
    impl<'de> Visitor<'de> for Bytes {
        type Value = Vec<u8>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("bounded residue bytes")
        }
        fn visit_seq<A: SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut bytes = Vec::new();
            while let Some(byte) = seq.next_element::<u8>()? {
                if bytes.len() as u64 >= FILE_LIMIT_BYTES {
                    return Err(de::Error::custom("residue file limit"));
                }
                bytes.push(byte);
            }
            Ok(bytes)
        }
    }
    d.deserialize_seq(Bytes)
}
fn bounded_entries<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<Vec<DecodedEntry>, D::Error> {
    struct Entries;
    impl<'de> Visitor<'de> for Entries {
        type Value = Vec<DecodedEntry>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("bounded residue entries")
        }
        fn visit_seq<A: SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut entries = Vec::new();
            let mut total = 0u64;
            while let Some(entry) = seq.next_element::<DecodedEntry>()? {
                total = total.saturating_add(entry.data.len() as u64);
                if entries.len() >= MAX_FILES
                    || total > TOTAL_LIMIT_BYTES
                    || entry.path.len() > 4096
                    || entry.role.len() > 128
                    || entry.digest.len() != 64
                {
                    return Err(de::Error::custom("residue metadata or content limit"));
                }
                entries.push(entry);
            }
            Ok(entries)
        }
    }
    d.deserialize_seq(Entries)
}

/// Open and fully validate one sealed residue package from an explicit path.
///
/// The ciphertext is read under a no-follow, non-blocking, single-link handle
/// with a fixed size bound; it is decrypted with a bounded scrypt work factor;
/// the plaintext is parsed with duplicate-key rejection; and every entry is then
/// validated (absolute path with no parent hops, non-duplicate, byte count,
/// digest). Any disagreement is a specific error and no partial package is
/// returned.
fn open_residue_package(path: &Path, pass: &str) -> Result<ResiduePackage> {
    let (cipher, identity) = read_cipher(path)?;
    let cipher_digest = identity["digest"].as_str().unwrap().to_owned();
    let decryptor = age::Decryptor::new(cipher.as_slice())
        .map_err(|_| err("invalid_residue_archive", "不是有效的残留备份归档"))?;
    let mut identity =
        age::scrypt::Identity::new(age::secrecy::SecretString::from(pass.to_owned()));
    identity.set_max_work_factor(MAX_SCRYPT_LOG_N);
    let reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|error| {
            if matches!(error, age::DecryptError::ExcessiveWork { .. }) {
                err(
                    "residue_archive_limit",
                    "残留备份 scrypt 成本超过有限 reader 上限（log N 20）",
                )
            } else {
                err("residue_archive_locked", "口令不正确，或残留备份无法解密")
            }
        })?;
    let mut plain = Vec::new();
    reader
        .take(MAX_PLAIN_BYTES + 1)
        .read_to_end(&mut plain)
        .map_err(|_| err("invalid_residue_archive", "残留备份解密或完整性校验失败"))?;
    if plain.len() as u64 > MAX_PLAIN_BYTES {
        return Err(err("residue_archive_limit", "残留备份展开后超过容量上限"));
    }
    drop(cipher);
    // Decode byte arrays directly with bounded visitors; never build a Value
    // node for every plaintext byte or reserve from untrusted length metadata.
    let doc: DecodedPackage = serde_json::from_slice(&plain).map_err(|_| {
        err(
            "invalid_residue_archive",
            "归档结构、字节预算、重复字段或 JSON 格式无效",
        )
    })?;
    if doc.schema != RESIDUE_SCHEMA {
        return Err(err(
            "residue_archive_schema",
            "只支持 lintel.residue/1；工作包不属于残留恢复材料",
        ));
    }
    drop(plain);
    let raw_files = doc.files;
    let mut seen = std::collections::HashSet::new();
    let mut total = 0u64;
    let mut files = vec![];
    for entry in raw_files {
        let raw_path = entry.path.as_str();
        if raw_path.is_empty() || raw_path.chars().any(char::is_control) {
            return Err(err("invalid_residue_archive", "残留备份条目路径无效"));
        }
        let path = PathBuf::from(raw_path);
        // Exact original locations: absolute, no parent-directory hops, and no
        // symlink anywhere in the recorded path.
        if !path.is_absolute() {
            return Err(err("invalid_residue_archive", "残留备份只接受绝对原始路径"));
        }
        if path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(err("invalid_residue_archive", "残留备份路径含父目录跳转"));
        }
        if !seen.insert(path.clone()) {
            return Err(err("invalid_residue_archive", "残留备份含重复路径"));
        }
        let bytes = entry.bytes;
        if bytes > FILE_LIMIT_BYTES {
            return Err(err("residue_archive_limit", "残留备份单文件超过容量上限"));
        }
        total = total.saturating_add(bytes);
        if total > TOTAL_LIMIT_BYTES {
            return Err(err("residue_archive_limit", "残留备份合计超过容量上限"));
        }
        let data = entry.data;
        if data.len() as u64 != bytes {
            return Err(err(
                "invalid_residue_archive",
                "残留备份条目字节数与 data 长度不一致",
            ));
        }
        let recorded = entry.digest.as_str();
        if digest(&data) != recorded {
            return Err(err(
                "invalid_residue_archive",
                "残留备份条目摘要与 data 不一致",
            ));
        }
        let role = entry.role;
        files.push(ResidueEntry {
            path,
            role,
            bytes,
            digest: recorded.to_string(),
            data,
        });
    }
    Ok(ResiduePackage {
        cipher_digest,
        environment_id: doc._environment_id,
        files,
    })
}

/// The private frozen source for a read/restore request. Exactly one of a
/// `job_id` (this install's recorded residue archive) or an explicit absolute
/// `archive_path`. A job-scoped lookup accepts only a recorded residue archive,
/// never an arbitrary path, and never creates a task record.
fn residue_source_path(engine: &Engine, r: &Value) -> Result<PathBuf> {
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
        let record = load(&engine.state.join("jobs").join(format!("{jid}.json")))?;
        let recorded = record["residue_archive_path"]
            .as_str()
            .or_else(|| record["identity_archive_path"].as_str())
            .filter(|path| !path.is_empty())
            .ok_or_else(|| err("residue_archive_missing", "此任务没有可读取的残留备份"))?;
        Ok(PathBuf::from(recorded))
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

/// The frozen cipher identity of one residue archive, so a restore is bound to
/// the exact bytes that were previewed.
fn read_cipher(path: &Path) -> Result<(Vec<u8>, Value)> {
    guard(path)?;
    let before =
        fs::symlink_metadata(path).map_err(|_| err("residue_archive_missing", "找不到残留备份"))?;
    let identity = |m: &fs::Metadata| {
        json!([
            m.dev(),
            m.ino(),
            m.uid(),
            m.nlink(),
            m.mode(),
            m.len(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec()
        ])
    };
    if !before.is_file() || before.uid() != unsafe { libc::geteuid() } || before.nlink() != 1 {
        return Err(err(
            "residue_archive_unsafe",
            "归档必须是当前用户独占的普通文件",
        ));
    }
    if before.len() > MAX_CIPHER_BYTES {
        return Err(err("residue_archive_limit", "归档密文超过容量上限"));
    }
    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    if identity(&file.metadata()?) != identity(&before) {
        return Err(err("stale_residue_archive", "打开归档时对象变化"));
    }
    let mut bytes = vec![];
    (&mut file)
        .take(MAX_CIPHER_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_CIPHER_BYTES
        || identity(&file.metadata()?) != identity(&before)
        || identity(&fs::symlink_metadata(path)?) != identity(&before)
    {
        return Err(err("stale_residue_archive", "读取归档期间对象或内容变化"));
    }
    let facts = json!({"identity":identity(&before),"digest":digest(&bytes)});
    Ok((bytes, facts))
}
fn cipher_identity(path: &Path) -> Result<Value> {
    read_cipher(path).map(|(_, facts)| facts)
}

impl Engine {
    /// Bounded private reader for a residue backup: returns a manifest of the
    /// recorded entries (path, role, bytes, digest) and never any file body.
    /// `archive_passphrase` is required and is never stored.
    pub(crate) fn residue_archive_inspect(&self, r: &Value) -> Result<Value> {
        let path = residue_source_path(self, r)?;
        let pass = string(r, "archive_passphrase")?;
        let package = open_residue_package(&path, pass)?;
        let files: Vec<Value> = package
            .files
            .iter()
            .map(|entry| {
                json!({
                    "path": entry.path,
                    "role": if entry.role.is_empty() { Value::Null } else { json!(entry.role) },
                    "bytes": entry.bytes,
                    "digest": entry.digest,
                    "restore_available":package.environment_id.as_deref().is_some_and(lintel_operations::valid_uuid)&&fs::symlink_metadata(&entry.path).is_err_and(|e|e.kind()==std::io::ErrorKind::NotFound)&&entry.path.parent().is_some_and(|p|p.is_dir()),
                })
            })
            .collect();
        let total: u64 = package.files.iter().map(|entry| entry.bytes).sum();
        Ok(json!({
            "schema": RESIDUE_SCHEMA,
            "environment_id":package.environment_id,
            "archive_path": path,
            "cipher_digest": package.cipher_digest,
            "file_count": package.files.len(),
            "total_bytes": total,
            "files": files,
            "notes": "这是独立于工作包的 lintel.residue/1 残留备份；可恢复到记录的原路径，不会迁入任何环境，也不能当作工作内容导入。manifest 不含任何文件正文。",
        }))
    }

    /// Preview restoring selected entries to their recorded original paths.
    ///
    /// Every frozen target must be currently absent (no overwrite), and the
    /// archive ciphertext is frozen by identity and digest so execution rechecks
    /// the exact previewed bytes. Only entries named in `selected_paths` are
    /// included; an empty or unknown selection is rejected rather than widened.
    pub(crate) fn plan_residue_restore(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let path = residue_source_path(self, r)?;
        let pass = string(r, "archive_passphrase")?;
        let package = open_residue_package(&path, pass)?;
        if !package
            .environment_id
            .as_deref()
            .is_some_and(lintel_operations::valid_uuid)
        {
            return Err(err(
                "environment_binding_missing",
                "此备份没有有效的原环境绑定；可读取清单，但不能据此批准写回",
            ));
        }
        if package
            .environment_id
            .as_deref()
            .is_some_and(|id| id != string(&e, "id").unwrap_or(""))
        {
            return Err(err(
                "environment_mismatch",
                "残留备份仍绑定原环境；请在原主机与环境恢复",
            ));
        }
        let selected = r["selected_paths"].as_array().ok_or_else(|| {
            err(
                "invalid_selection",
                "残留恢复需要 selected_paths 精确条目列表",
            )
        })?;
        if selected.is_empty() {
            return Err(err(
                "invalid_selection",
                "selected_paths 为空；空选择不会被当作全部恢复",
            ));
        }
        let mut seen = std::collections::HashSet::new();
        let mut files = vec![];
        for value in selected {
            let raw = value
                .as_str()
                .filter(|raw| !raw.is_empty())
                .ok_or_else(|| err("invalid_selection", "selected_paths 只能包含非空字符串"))?;
            if !seen.insert(raw.to_string()) {
                return Err(err("invalid_selection", "selected_paths 含重复条目"));
            }
            let entry = package
                .files
                .iter()
                .find(|entry| entry.path.to_string_lossy() == raw)
                .ok_or_else(|| {
                    err(
                        "invalid_selection",
                        "selected_paths 含归档中没有的条目；请重新读取 manifest",
                    )
                })?;
            // No overwrite: the exact original target must be absent now. The
            // parent must already exist as a real directory (we never create a
            // directory tree outside the frozen path).
            let parent = entry
                .path
                .parent()
                .ok_or_else(|| err("invalid_selection", "残留条目缺少父目录"))?;
            guard(parent)?;
            if !parent.is_dir() {
                return Err(err(
                    "restore_parent_missing",
                    "恢复目标的原目录不存在；请先建立该目录，Lintel 不会替你新建目录树",
                ));
            }
            if fs::symlink_metadata(&entry.path).is_ok() {
                return Err(err(
                    "restore_conflict",
                    "原路径已有内容；不会覆盖，请先核对或选择其他备份",
                ));
            }
            let parent_meta = fs::metadata(parent)?;
            if parent_meta.uid() != unsafe { libc::geteuid() } {
                return Err(err("wrong_owner", "恢复目标父目录不属于当前用户"));
            }
            files.push(json!({
                "path": entry.path,
                "role": if entry.role.is_empty() { Value::Null } else { json!(entry.role) },
                "bytes": entry.bytes,
                "digest": entry.digest,
                "destination": entry.path,
                "parent_identity":[parent_meta.dev(),parent_meta.ino(),parent_meta.uid()],
            }));
        }
        let source_identity = cipher_identity(&path)?;
        self.plan(
            &e,
            RESIDUE_RESTORE_KIND,
            "恢复所选残留备份条目",
            json!([]),
            vec![
                "其他残留备份条目（不恢复未选条目）",
                "工作内容、设置、登录与凭据",
                "已存在的原路径内容（不覆盖）",
            ],
            json!([{"id":"restore","label":"解密并把所选条目写回原路径并读回","reversible":false}]),
            json!({
                "archive_path": path,
                "source_identity": source_identity,
                "selected_paths": seen.iter().cloned().collect::<Vec<String>>(),
                "manifest": files,
                "archive_passphrase_required": true,
            }),
        )
    }

    /// Pre-execution recheck for a residue restore: the archive still has the
    /// frozen identity/digest, the passphrase is present, and every frozen
    /// destination is still absent.
    pub(crate) fn check_residue_restore(&self, p: &Value, r: &Value) -> Result<()> {
        let _ = string(r, "archive_passphrase")?;
        let path = PathBuf::from(string(&p["extra"], "archive_path")?);
        if cipher_identity(&path)? != p["extra"]["source_identity"] {
            return Err(err(
                "stale_residue_archive",
                "残留备份在预览后变化；不会恢复被替换的归档，请重新预览",
            ));
        }
        for file in p["extra"]["manifest"].as_array().unwrap_or(&vec![]) {
            let destination = PathBuf::from(string(file, "destination")?);
            guard(&destination)?;
            let parent = destination
                .parent()
                .ok_or_else(|| err("invalid_path", "恢复路径缺少父目录"))?;
            let m = fs::metadata(parent)?;
            if file["parent_identity"] != json!([m.dev(), m.ino(), m.uid()]) {
                return Err(err("restore_conflict", "恢复目标父目录已变化"));
            }
            if fs::symlink_metadata(&destination).is_ok() {
                return Err(err(
                    "restore_conflict",
                    "原路径在预览后出现内容；不会覆盖，未恢复",
                ));
            }
        }
        Ok(())
    }

    /// Execute a residue restore: revalidate the frozen archive, then write each
    /// selected entry back to its exact original path using the no-replace
    /// publication primitive, with a post-write digest readback per file. A
    /// single file's failure is recorded per item and never discards the others.
    pub(crate) fn execute_residue_restore(
        &self,
        _e: &Value,
        p: &Value,
        r: &Value,
        j: &mut Value,
        journal: &Path,
    ) -> Result<()> {
        self.check_residue_restore(p, r)?;
        let pass = string(r, "archive_passphrase")?;
        let path = PathBuf::from(string(&p["extra"], "archive_path")?);
        let package = open_residue_package(&path, pass)?;
        if package.cipher_digest != p["extra"]["source_identity"]["digest"] {
            return Err(err(
                "stale_residue_archive",
                "残留备份解密摘要与冻结值不一致；未恢复",
            ));
        }
        let mut restored = 0usize;
        let mut failures: Vec<Value> = vec![];
        for file in p["extra"]["manifest"]
            .as_array()
            .cloned()
            .unwrap_or_default()
        {
            let raw = string(&file, "path")?;
            let entry = package
                .files
                .iter()
                .find(|entry| entry.path.to_string_lossy() == raw)
                .ok_or_else(|| err("invalid_residue_archive", "归档与计划清单不一致"))?;
            if entry.digest != string(&file, "digest")?
                || Some(entry.bytes) != file["bytes"].as_u64()
            {
                return Err(err("stale_residue_archive", "归档条目与冻结清单不一致"));
            }
            let destination = &entry.path;
            j["steps"].as_array_mut().unwrap().push(json!({"id":"restore","label":raw,"status":"executing","message":"正在写回原路径；已有内容不会被覆盖。"}));
            save(journal, j)?;
            j["residue_restore_intent"] =
                json!({"path":destination,"digest":entry.digest,"bytes":entry.bytes});
            save(journal, j)?;
            guard(destination)?;
            let parent = fs::metadata(destination.parent().unwrap())?;
            if file["parent_identity"] != json!([parent.dev(), parent.ino(), parent.uid()]) {
                return Err(err("restore_conflict", "发布前恢复目标父目录已变化"));
            }
            let mode = fs::symlink_metadata(destination)
                .map(|meta| (meta.permissions().mode() & 0o777 & !0o022).max(0o600))
                .unwrap_or(0o600);
            match atomic_new(destination, &entry.data, mode) {
                Ok(()) => {
                    let readback = read(destination, FILE_LIMIT_BYTES).map(|bytes| digest(&bytes));
                    if matches!(readback, Ok(ref value) if value == &entry.digest) {
                        restored += 1;
                        *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":"restore","label":raw,"status":"completed","message":"已写回原路径并读回校验一致。"});
                    } else {
                        failures.push(json!({"path":raw,"reason":"readback_mismatch"}));
                        *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":"restore","label":raw,"status":"failed","message":"写回后读回不一致；请核对原路径。"});
                    }
                }
                Err(error) => {
                    failures.push(json!({"path":raw,"code":error.code,"message":error.message}));
                    *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":"restore","label":raw,"status":"failed","message":error.message});
                }
            }
            save(journal, j)?;
        }
        j["residue_restore"] = json!({
            "archive_path": path,
            "selected": p["extra"]["manifest"].as_array().map_or(0, Vec::len),
            "restored": restored,
            "failed": failures,
            "restored_at": now(),
        });
        if failures.is_empty() {
            j["status"] = json!("completed");
        } else {
            j["status"] = json!("partially_completed");
            j["warnings"].as_array_mut().unwrap().push(json!(
                "部分残留条目未恢复（见 residue_restore.failed）；其余已独立写回，未覆盖任何已有内容。"
            ));
        }
        j["warnings"].as_array_mut().unwrap().push(json!(
            "恢复只写回所选条目到记录的原路径；未选择的条目、工作内容、设置与登录状态均未改动。"
        ));
        Ok(())
    }
}

#[cfg(test)]
#[path = "residue_archive_tests.rs"]
mod tests;

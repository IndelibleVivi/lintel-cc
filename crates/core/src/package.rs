//! Bounded-memory codec for the on-disk work package.
//!
//! The package format is unchanged: an age passphrase envelope wrapping one
//! JSON object
//!
//! ```text
//! {"schema":"lintel.work/1","generator":"Lintel","created_at":"...",
//!  "files":[{"path":..,"category":..,"digest":..,"data":[<u8>,..],"bytes":N}..],
//!  "notes":..}
//! ```
//!
//! `data` is a JSON array of byte values (the historical `serde` encoding of a
//! `Vec<u8>`). Both directions stream one file's plaintext at a time:
//!
//! * [`write`] streams the same JSON bytes to an age [`age::StreamWriter`]
//!   through serde's serializer, encoding each entry's `data` array from the
//!   plaintext on disk (bounded buffer) while hashing it.
//! * [`read_to_temp`] streams the age plaintext through serde's streaming
//!   [`serde_json::Deserializer`] via a [`DeserializeSeed`], decoding each
//!   `data` array element-by-element (bounded buffer) into a private staging
//!   file and hashing it. Duplicate keys, wrong types, unsafe/duplicate paths,
//!   category forgeries, declared-digest mismatches and a truncated/extra tail
//!   are all refused. `Deserializer::end()` proves the JSON document end and
//!   the age reader is drained to its final tag.
//!
//! [`write_value`] also serves bounded state backups. In-memory decoding is
//! test-only for historical fixtures; production work readers use this codec.
use crate::{err, storage::*, Result};
use serde::{
    de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor},
    ser::{SerializeMap, SerializeSeq},
    Deserialize, Serialize, Serializer,
};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::{
    cell::{Cell, RefCell},
    fs::{self, File, OpenOptions},
    io::{BufReader, Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    rc::Rc,
};

/// The supported top-level schema literal.
pub(crate) const SCHEMA: &str = "lintel.work/1";
/// Default generator string written into new packages.
pub(crate) const GENERATOR: &str = "Lintel";
/// Bound for the legacy warm-path `read_value`: the historical in-memory
/// limit for a package that fits entirely in memory.
#[cfg(test)]
pub(crate) const LEGACY_PLAIN_LIMIT: u64 = 200 * 1024 * 1024;

/// Finite capacity contract for one package. Mirrored by `work` so both the
/// metadata scan and the archive/import admission use the same numbers.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Limits {
    /// Largest single file plaintext.
    pub file_bytes: u64,
    /// Largest total plaintext across all files.
    pub total_bytes: u64,
    /// Largest number of files.
    pub files: usize,
}

/// One decoded package member: metadata plus a private plaintext staging path.
/// The staging file is owned by [`Staging`]; callers must finish consuming it
/// before the staging directory is dropped.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub path: String,
    pub category: String,
    pub digest: String,
    pub bytes: u64,
    /// Private 0600 staging file holding the exact decoded bytes.
    pub plain: PathBuf,
}

/// Decoded package metadata. `files` carry private staging paths.
#[derive(Debug)]
pub(crate) struct Package {
    pub schema: String,
    pub generator: Value,
    pub created_at: Value,
    pub files: Vec<Entry>,
}

/// An RAII private staging directory: a single 0700 directory the current
/// process owns, scoped to one read. Dropping it removes the whole tree, so a
/// failed read never leaves decoded plaintext behind and never scans unrelated
/// temp paths. A process that is killed between creation and drop cannot run
/// this cleanup; the residual directory is private (0700, current-user-owned)
/// and randomly named, not silently swept.
pub(crate) struct Staging {
    pub(crate) dir: PathBuf,
}

impl Staging {
    /// Create a fresh private staging root inside `parent` (a state directory
    /// the caller has already validated). Never reuses an existing path.
    pub(crate) fn new(parent: &Path) -> Result<Self> {
        guard(parent)?;
        let dir = parent.join(format!(".lintel-work-stage-{}", uuid::Uuid::new_v4()));
        guard(&dir)?;
        fs::DirBuilder::new().mode(0o700).create(&dir)?;
        let meta = fs::symlink_metadata(&dir)?;
        if meta.file_type().is_symlink()
            || !meta.is_dir()
            || meta.uid() != unsafe { libc::geteuid() }
        {
            let _ = fs::remove_dir(&dir);
            return Err(err("wrong_owner", "暂存目录不属于当前用户"));
        }
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
        Ok(Self { dir })
    }

    /// A fresh 0600 signal-free path for one entry's decoded bytes. Created with
    /// `create_new` so a pre-existing name (a hostile symlink/FIFO planted under
    /// this private dir) can never be followed or overwritten.
    fn entry_file(&self, index: usize) -> Result<File> {
        let path = self.dir.join(format!("entry-{index:08}.bin"));
        guard(&path)?;
        Ok(OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)?)
    }

    /// Explicitly remove this read's private decoded plaintext tree and report
    /// whether it is actually gone. Used on the launch path, where a successful
    /// `exec` would skip [`Drop`]: the caller must confirm the full-package
    /// staging is closed before any process replacement. Only names this
    /// process created live under `dir`, so this never sweeps unrelated paths.
    pub(crate) fn close(self) -> Result<()> {
        match fs::remove_dir_all(&self.dir) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(err(
                "staging_cleanup_failed",
                "无法清除本次读取的私有暂存目录；未启动客户端。请检查原请求记录的阶段与错误后再决定下一步；若该请求已持久化意图，只有原请求 ID 可查询，不会自动重试。",
            )),
        }
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        // Best-effort: only names this process created live here.
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Describe one entry's plaintext file for the streaming writer. The plaintext
/// is read from `plain`; `expected_digest` must match the bytes streamed.
pub(crate) struct Source<'a> {
    pub path: &'a str,
    pub category: &'a str,
    pub expected_digest: &'a str,
    pub plain: &'a Path,
}

/// Serialize a package value to an age ciphertext written to `out`, streaming
/// each entry's `data` array from the plaintext described by `sources` (matched
/// by `path`). The value's own `data`/`bytes` members are ignored. Returns the
/// SHA-256 hex of the ciphertext and the total plaintext byte count.
pub(crate) fn write(
    out: &mut File,
    value: &Value,
    sources: &[Source<'_>],
    pass: &str,
) -> Result<(String, u64)> {
    let encryptor =
        age::Encryptor::with_user_passphrase(age::secrecy::SecretString::from(pass.to_owned()));
    let hasher = std::rc::Rc::new(std::cell::RefCell::new(Sha256::new()));
    let stream = encryptor
        .wrap_output(HashingWriter {
            inner: out,
            hasher: hasher.clone(),
        })
        .map_err(|_| err("archive_failed", "无法初始化归档加密"))?;
    let mut ser = serde_json::Serializer::new(stream);
    PackageSeed { value, sources }.serialize(&mut ser)?;
    let mut inner = ser
        .into_inner()
        .finish()
        .map_err(|_| err("archive_failed", "归档加密未完整完成"))?;
    inner
        .flush()
        .map_err(|_| err("archive_failed", "归档加密未完整完成"))?;
    let sha = format!("{:x}", hasher.borrow_mut().finalize_reset());
    let total = sources
        .iter()
        .map(|source| plain_len(source.plain))
        .sum::<Result<u64>>()?;
    Ok((sha, total))
}

fn plain_len(path: &Path) -> Result<u64> {
    Ok(fs::metadata(path)
        .map_err(|_| err("archive_failed", "无法读取来源文件"))?
        .len())
}

struct PackageSeed<'a> {
    value: &'a Value,
    sources: &'a [Source<'a>],
}

impl Serialize for PackageSeed<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let obj = self
            .value
            .as_object()
            .ok_or_else(|| serde::ser::Error::custom("package must be an object"))?;
        let mut map = serializer.serialize_map(Some(obj.len()))?;
        for (key, item) in obj {
            if key == "files" {
                map.serialize_entry(
                    key,
                    &FilesSeed {
                        value: item,
                        sources: self.sources,
                    },
                )?;
            } else {
                map.serialize_entry(key, item)?;
            }
        }
        map.end()
    }
}

struct FilesSeed<'a> {
    value: &'a Value,
    sources: &'a [Source<'a>],
}

impl Serialize for FilesSeed<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let list = self
            .value
            .as_array()
            .ok_or_else(|| serde::ser::Error::custom("files must be an array"))?;
        let mut seq = serializer.serialize_seq(Some(list.len()))?;
        for entry in list {
            seq.serialize_element(&EntrySeed {
                value: entry,
                sources: self.sources,
            })?;
        }
        seq.end()
    }
}

struct EntrySeed<'a> {
    value: &'a Value,
    sources: &'a [Source<'a>],
}

impl Serialize for EntrySeed<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let obj = self
            .value
            .as_object()
            .ok_or_else(|| serde::ser::Error::custom("entry must be an object"))?;
        let path = obj
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| serde::ser::Error::custom("entry missing path"))?;
        let source = self
            .sources
            .iter()
            .find(|candidate| candidate.path == path)
            .ok_or_else(|| serde::ser::Error::custom("entry missing source"))?;
        let size =
            plain_len(source.plain).map_err(|error| serde::ser::Error::custom(error.message))?;
        // Emit exactly the canonical member set with `data` streamed from the
        // plaintext. This is a strict allowlist: private plan metadata such as
        // `source_identity` is never written into the package, and no
        // caller-supplied body is emitted.
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("bytes", &size)?;
        map.serialize_entry("category", source.category)?;
        map.serialize_entry(
            "data",
            &BytesSeed {
                plain: source.plain,
                expected_bytes: size,
                expected_digest: source.expected_digest,
            },
        )?;
        map.serialize_entry("digest", source.expected_digest)?;
        map.serialize_entry("path", source.path)?;
        map.end()
    }
}

struct BytesSeed<'a> {
    plain: &'a Path,
    expected_bytes: u64,
    expected_digest: &'a str,
}

impl Serialize for BytesSeed<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::Error as _;
        guard(self.plain).map_err(|e| S::Error::custom(e.message))?;
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(self.plain)
            .map_err(S::Error::custom)?;
        if !file.metadata().map_err(S::Error::custom)?.is_file() {
            return Err(S::Error::custom("staged source is not a regular file"));
        }
        let mut seq = serializer.serialize_seq(None)?;
        let mut buf = [0u8; 64 * 1024];
        let mut count = 0u64;
        let mut hash = Sha256::new();
        loop {
            let n = match file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(S::Error::custom(e)),
            };
            count += n as u64;
            if count > self.expected_bytes {
                return Err(S::Error::custom("staged source grew during serialization"));
            }
            hash.update(&buf[..n]);
            for byte in &buf[..n] {
                seq.serialize_element(byte)?;
            }
        }
        if count != self.expected_bytes || format!("{:x}", hash.finalize()) != self.expected_digest
        {
            return Err(S::Error::custom(
                "staged source differs from the frozen manifest",
            ));
        }
        seq.end()
    }
}

/// Encode a small in-memory package value to age ciphertext (the legacy warm
/// path -- used by the state-backup writer with a tiny synthetic value).
pub(crate) fn write_value(value: &Value, pass: &str) -> Result<Vec<u8>> {
    let encryptor =
        age::Encryptor::with_user_passphrase(age::secrecy::SecretString::from(pass.to_owned()));
    let mut bytes = vec![];
    let mut stream = encryptor
        .wrap_output(&mut bytes)
        .map_err(|_| err("archive_failed", "无法初始化归档加密"))?;
    stream
        .write_all(&serde_json::to_vec(value)?)
        .map_err(|_| err("archive_failed", "归档加密未完整完成"))?;
    stream
        .finish()
        .map_err(|_| err("archive_failed", "归档加密未完整完成"))?;
    Ok(bytes)
}

/// Test-only decoder for historical small work fixtures.
#[cfg(test)]
pub(crate) fn read_value(path: &Path, pass: &str, limit: u64) -> Result<Value> {
    let reader = decrypt(path, pass)?;
    let mut buffered = BufReader::new(reader);
    let mut plain = Vec::new();
    (&mut buffered)
        .take(limit + 1)
        .read_to_end(&mut plain)
        .map_err(|_| err("invalid_archive", "归档解密或完整性校验失败"))?;
    if plain.len() as u64 > limit {
        return Err(err("archive_limit", "归档展开后超过容量上限"));
    }
    let mut de = serde_json::Deserializer::from_slice(&plain);
    let value = Unique::deserialize(&mut de)
        .map_err(|_| err("invalid_archive", "JSON 格式损坏或包含重复键"))?
        .0;
    de.end()
        .map_err(|_| err("invalid_archive", "JSON 文档包含多余内容"))?;
    Ok(value)
}

/// A writer wrapper that hashes every published ciphertext byte.
struct HashingWriter<'a> {
    inner: &'a mut File,
    hasher: std::rc::Rc<std::cell::RefCell<Sha256>>,
}
impl Write for HashingWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.hasher.borrow_mut().update(&buf[..written]);
        Ok(written)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
fn decrypt(path: &Path, pass: &str) -> Result<impl Read> {
    let file = open_cipher(path, crate::work::MAX_BYTES)?.0;
    let decryptor = age::Decryptor::new(BufReader::new(file))
        .map_err(|_| err("invalid_archive", "不是有效的 age 归档"))?;
    let mut identity =
        age::scrypt::Identity::new(age::secrecy::SecretString::from(pass.to_owned()));
    identity.set_max_work_factor(MAX_SCRYPT_LOG_N);
    decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|_| err("archive_locked", "口令不正确，或归档无法解密"))
}

/// Maximum bytes a single bounded metadata field (a key or a scalar value such
/// as `generator`/`created_at`/`notes`) may consume. A hostile package cannot
/// make serde allocate a larger string/number than this before the budget
/// reader refuses.
const META_FIELD_BYTES: u64 = 1024 * 1024;
/// Maximum aggregate metadata bytes (all keys and non-`data` values) in one
/// package. `data` arrays are bounded separately by the declared file size.
const META_TOTAL_BYTES: u64 = 16 * 1024 * 1024;
/// Worst-case JSON plaintext for a package: every plaintext byte may serialize
/// to four characters (`255` plus a separator), plus metadata and a margin for
/// the envelope. Used to bound the reader and to size the ciphertext envelope.
pub(crate) fn plaintext_bound(total_bytes: u64) -> u64 {
    total_bytes
        .saturating_mul(5)
        .saturating_add(META_TOTAL_BYTES)
        .saturating_add(8 * 1024 * 1024)
}

/// A `Read` wrapper enforcing a shared byte budget *before* bytes are handed to
/// serde, so a hostile package cannot make serde allocate beyond the caller's
/// declared bound. Visitors reset the budget before admitting one metadata
/// field or one byte token at a time.
struct BudgetReader<R> {
    inner: R,
    /// Bytes remaining in the currently admitted region.
    budget: Rc<Cell<u64>>,
    /// Aggregate metadata bytes still allowed; only metadata draws on it.
    meta_total: Rc<Cell<u64>>,
    /// Whether draws should count against the aggregate metadata budget.
    charging_meta: Rc<Cell<bool>>,
    plain_left: u64,
}

impl<R: Read> Read for BudgetReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // Bound the underlying read by the remaining region budget so serde
        // never receives more than the admitted region.
        let remaining = self.budget.get();
        if remaining == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "package region over budget",
            ));
        }
        let want = buf.len().min(remaining.min(self.plain_left) as usize);
        if want == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "package plaintext over budget",
            ));
        }
        let n = self.inner.read(&mut buf[..want])?;
        if n > 0 {
            self.plain_left -= n as u64;
            self.budget.set(remaining - n as u64);
            if self.charging_meta.get() {
                let left = self.meta_total.get();
                if (n as u64) > left {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "package metadata over budget",
                    ));
                }
                self.meta_total.set(left - n as u64);
            }
        }
        Ok(n)
    }
}

/// Admit one bounded region. Metadata draws on per-field and aggregate limits;
/// each data byte token has an independent small parsing budget.
#[derive(Clone, Copy)]
struct Budget<'a> {
    region: &'a Rc<Cell<u64>>,
    meta_total: &'a Rc<Cell<u64>>,
    charging_meta: &'a Rc<Cell<bool>>,
}

impl Budget<'_> {
    fn meta(&self) {
        self.charging_meta.set(true);
        let allowed = self.meta_total.get().min(META_FIELD_BYTES);
        self.region.set(allowed);
    }
    fn data_token(&self) {
        self.charging_meta.set(false);
        // Reset for each u8, before serde can allocate a malformed string or
        // numeric token. Canonical u8 elements need at most four bytes.
        self.region.set(128);
    }
    /// Restore charging to metadata after a `data` region finished, and make
    /// sure the region still has room for the following metadata separator.
    fn after_data(&self) {
        self.charging_meta.set(true);
        self.region.set(self.meta_total.get().min(META_FIELD_BYTES));
    }
}

/// Limit untrusted passphrase work independently of package-body size. Existing
/// packages above this cost are refused, never decrypted with a weaker key.
const MAX_SCRYPT_LOG_N: u8 = 20;
const AGE_HEADER_BYTES: u64 = 64 * 1024;

fn cipher_identity(m: &fs::Metadata) -> Value {
    serde_json::json!({"device":m.dev(),"inode":m.ino(),"owner":m.uid(),"links":m.nlink(),
        "mode":m.mode(),"bytes":m.len(),"mtime":m.mtime(),"mtime_nsec":m.mtime_nsec(),
        "ctime":m.ctime(),"ctime_nsec":m.ctime_nsec()})
}
fn cipher_bound(total_bytes: u64) -> u64 {
    let plain = plaintext_bound(total_bytes);
    plain + plain / (64 * 1024) * 64 + AGE_HEADER_BYTES
}
fn open_cipher(path: &Path, total_bytes: u64) -> Result<(File, fs::Metadata)> {
    guard(path)?;
    let meta =
        fs::symlink_metadata(path).map_err(|_| err("archive_missing", "找不到该归档文件"))?;
    if !meta.is_file() || meta.uid() != unsafe { libc::geteuid() } || meta.nlink() != 1 {
        return Err(err("archive_missing", "归档不是安全的常规文件"));
    }
    if meta.len() > cipher_bound(total_bytes) {
        return Err(err("archive_limit", "归档密文超过容量上限"));
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    if cipher_identity(&file.metadata()?) != cipher_identity(&meta) {
        return Err(err("stale_archive", "归档在读取前被替换"));
    }
    Ok((file, meta))
}
fn check_cipher(path: &Path, file: &File, before: &fs::Metadata) -> Result<()> {
    guard(path)?;
    if cipher_identity(&file.metadata()?) != cipher_identity(before)
        || cipher_identity(&fs::symlink_metadata(path)?) != cipher_identity(before)
    {
        return Err(err("stale_archive", "归档在读取期间被替换或修改"));
    }
    Ok(())
}
struct CipherReader {
    file: File,
    hash: Rc<RefCell<Sha256>>,
    count: Rc<Cell<u64>>,
    horizon: Rc<Cell<u64>>,
}
impl Read for CipherReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let left = self.horizon.get().saturating_sub(self.count.get());
        if left == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "ciphertext region over budget",
            ));
        }
        let want = buf.len().min(left as usize);
        let n = self.file.read(&mut buf[..want])?;
        self.hash.borrow_mut().update(&buf[..n]);
        self.count.set(self.count.get() + n as u64);
        Ok(n)
    }
}

/// Test-only digest for historical warm fixtures. Production decoding hashes
/// the very same open stream that supplies the authenticated plaintext.
#[cfg(test)]
pub(crate) fn ciphertext_digest(path: &Path) -> Result<String> {
    let (mut file, before) = open_cipher(path, crate::work::MAX_BYTES)?;
    let mut hash = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    check_cipher(path, &file, &before)?;
    Ok(format!("{:x}", hash.finalize()))
}

/// Fully authenticate and decode one bounded work package. Ciphertext digest,
/// plaintext and EOF evidence all come from the same non-following handle.
pub(crate) fn read_to_temp(
    path: &Path,
    pass: &str,
    staging: &Staging,
    limits: Limits,
) -> Result<(Package, String)> {
    let (file, before) = open_cipher(path, limits.total_bytes)?;
    let witness = file.try_clone()?;
    let hash = Rc::new(RefCell::new(Sha256::new()));
    let count = Rc::new(Cell::new(0));
    let horizon = Rc::new(Cell::new(AGE_HEADER_BYTES));
    let cipher = CipherReader {
        file,
        hash: hash.clone(),
        count: count.clone(),
        horizon: horizon.clone(),
    };
    let decryptor = age::Decryptor::new(BufReader::new(cipher))
        .map_err(|_| err("invalid_archive", "不是有效的有限 age 归档"))?;
    horizon.set(cipher_bound(limits.total_bytes));
    let mut identity =
        age::scrypt::Identity::new(age::secrecy::SecretString::from(pass.to_owned()));
    identity.set_max_work_factor(MAX_SCRYPT_LOG_N);
    let reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|e| {
            if matches!(e, age::DecryptError::ExcessiveWork { .. }) {
                err(
                    "archive_limit",
                    "归档 scrypt 成本超过有限 reader 上限（log N 20）",
                )
            } else {
                err("archive_locked", "口令不正确，或归档无法解密")
            }
        })?;
    let package = decode_plain(BufReader::new(reader), staging, limits)?;
    // end() has observed plaintext EOF and the age final tag. Require all
    // ciphertext bytes as well, and refuse any concurrent source replacement.
    if count.get() != before.len() {
        return Err(err("invalid_archive", "归档密文尾部未完整消费"));
    }
    check_cipher(path, &witness, &before)?;
    Ok((package, format!("{:x}", hash.borrow_mut().finalize_reset())))
}

fn decode_plain<R: Read>(reader: R, staging: &Staging, limits: Limits) -> Result<Package> {
    let region = Rc::new(Cell::new(0u64));
    let meta_total = Rc::new(Cell::new(META_TOTAL_BYTES));
    let charging_meta = Rc::new(Cell::new(true));
    let budget = Budget {
        region: &region,
        meta_total: &meta_total,
        charging_meta: &charging_meta,
    };
    budget.meta();
    // Buffer below the gate, so every byte serde receives is charged to the
    // current field/token, including malformed strings and numeric tokens.
    let mut gated = BudgetReader {
        inner: reader,
        budget: region.clone(),
        meta_total: meta_total.clone(),
        charging_meta: charging_meta.clone(),
        plain_left: plaintext_bound(limits.total_bytes),
    };
    let package = {
        let mut de = serde_json::Deserializer::from_reader(&mut gated);
        let package = PackageRead {
            staging,
            limits,
            budget,
        }
        .deserialize(&mut de)
        .map_err(normalize_read_error)?;
        budget.meta();
        de.end()
            .map_err(|_| err("invalid_archive", "工作包尾部损坏、被截断或有多余内容"))?;
        package
    };
    Ok(package)
}

/// Map a serde error to a specific package error where possible.
fn normalize_read_error(error: serde_json::Error) -> crate::Error {
    if error.is_syntax() || error.is_eof() || error.is_data() {
        err("invalid_archive", "工作包结构损坏或类型错误")
    } else {
        err("invalid_archive", "工作包读取失败")
    }
}

/// A seed for one bounded metadata string (a key or a scalar string value). It
/// raises the region budget to a single metadata field before serde reads any
/// byte, so a hostile value cannot make serde allocate beyond the field cap.
#[derive(Clone, Copy)]
struct MetaString<'a> {
    budget: Budget<'a>,
}

impl<'de> DeserializeSeed<'de> for MetaString<'_> {
    type Value = String;
    fn deserialize<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<String, D::Error> {
        self.budget.meta();
        String::deserialize(deserializer)
    }
}

/// A seed for one bounded metadata scalar value (`generator`/`created_at`/
/// `notes`/`bytes`/`source_identity`). The value is a small JSON scalar or a
/// shallow object/array; the metadata budget caps its serialized size.
#[derive(Clone, Copy)]
struct MetaValue<'a> {
    budget: Budget<'a>,
}

impl<'de> DeserializeSeed<'de> for MetaValue<'_> {
    type Value = Value;
    fn deserialize<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<Value, D::Error> {
        self.budget.meta();
        Ok(Unique::deserialize(deserializer)?.0)
    }
}

/// Streaming deserialize seed for one package. Verifies the schema, bound
/// metadata members, and per-entry path/category/digest while writing each
/// `data` array to a private staging file.
struct PackageRead<'a> {
    staging: &'a Staging,
    limits: Limits,
    budget: Budget<'a>,
}

impl<'de> DeserializeSeed<'de> for PackageRead<'_> {
    type Value = Package;
    fn deserialize<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<Package, D::Error> {
        deserializer.deserialize_map(PackageVisitor {
            staging: self.staging,
            limits: self.limits,
            budget: self.budget,
        })
    }
}

struct PackageVisitor<'a> {
    staging: &'a Staging,
    limits: Limits,
    budget: Budget<'a>,
}

impl<'de> Visitor<'de> for PackageVisitor<'_> {
    type Value = Package;
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a lintel.work/1 package object")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Package, A::Error> {
        use serde::de::Error as DeError;
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut schema: Option<String> = None;
        let mut generator: Option<Value> = None;
        let mut created_at: Option<Value> = None;
        let mut files: Option<Vec<Entry>> = None;
        while let Some(key) = map.next_key_seed(MetaString {
            budget: self.budget,
        })? {
            if !seen.insert(key.clone()) {
                return Err(DeError::custom("duplicate top-level field"));
            }
            match key.as_str() {
                "schema" => {
                    schema = Some(map.next_value_seed(MetaString {
                        budget: self.budget,
                    })?)
                }
                "generator" => {
                    generator = Some(map.next_value_seed(MetaValue {
                        budget: self.budget,
                    })?)
                }
                "created_at" => {
                    created_at = Some(map.next_value_seed(MetaValue {
                        budget: self.budget,
                    })?)
                }
                "notes" => {
                    let _ = map.next_value_seed(MetaValue {
                        budget: self.budget,
                    })?;
                }
                "files" => {
                    files = Some(map.next_value_seed(FilesRead {
                        staging: self.staging,
                        limits: self.limits,
                        budget: self.budget,
                    })?)
                }
                _ => return Err(DeError::custom("unknown top-level field")),
            }
        }
        let schema = schema.ok_or_else(|| DeError::custom("missing schema"))?;
        if schema != SCHEMA {
            return Err(DeError::custom("unsupported package schema"));
        }
        let files = files.ok_or_else(|| DeError::custom("missing files"))?;
        Ok(Package {
            schema,
            generator: generator.unwrap_or(Value::Null),
            created_at: created_at.unwrap_or(Value::Null),
            files,
        })
    }
}

struct FilesRead<'a> {
    staging: &'a Staging,
    limits: Limits,
    budget: Budget<'a>,
}

impl<'de> DeserializeSeed<'de> for FilesRead<'_> {
    type Value = Vec<Entry>;
    fn deserialize<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<Vec<Entry>, D::Error> {
        deserializer.deserialize_seq(FilesVisitor {
            staging: self.staging,
            limits: self.limits,
            budget: self.budget,
        })
    }
}

struct FilesVisitor<'a> {
    staging: &'a Staging,
    limits: Limits,
    budget: Budget<'a>,
}

impl<'de> Visitor<'de> for FilesVisitor<'_> {
    type Value = Vec<Entry>;
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a package files array")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<Vec<Entry>, A::Error> {
        use serde::de::Error as DeError;
        let mut entries: Vec<Entry> = vec![];
        let mut total: u64 = 0;
        while let Some(entry) = seq.next_element_seed(EntryRead {
            staging: self.staging,
            index: entries.len(),
            limits: Limits {
                file_bytes: self
                    .limits
                    .file_bytes
                    .min(self.limits.total_bytes.saturating_sub(total)),
                ..self.limits
            },
            budget: self.budget,
        })? {
            if entries.len() >= self.limits.files {
                return Err(DeError::custom("too many files"));
            }
            total = total.saturating_add(entry.bytes);
            if total > self.limits.total_bytes {
                return Err(DeError::custom("total bytes over limit"));
            }
            entries.push(entry);
        }
        Ok(entries)
    }
}

struct EntryRead<'a> {
    staging: &'a Staging,
    index: usize,
    limits: Limits,
    budget: Budget<'a>,
}

impl<'de> DeserializeSeed<'de> for EntryRead<'_> {
    type Value = Entry;
    fn deserialize<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<Entry, D::Error> {
        deserializer.deserialize_map(EntryVisitor {
            staging: self.staging,
            index: self.index,
            limits: self.limits,
            budget: self.budget,
        })
    }
}

struct EntryVisitor<'a> {
    staging: &'a Staging,
    index: usize,
    limits: Limits,
    budget: Budget<'a>,
}

impl<'de> Visitor<'de> for EntryVisitor<'_> {
    type Value = Entry;
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a package file entry")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Entry, A::Error> {
        use serde::de::Error as DeError;
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut path: Option<String> = None;
        let mut category: Option<String> = None;
        let mut digest: Option<String> = None;
        let mut bytes: Option<u64> = None;
        let mut source_identity = false;
        let mut data: Option<(PathBuf, u64, String)> = None;
        while let Some(key) = map.next_key_seed(MetaString {
            budget: self.budget,
        })? {
            if !seen.insert(key.clone()) {
                return Err(DeError::custom("duplicate entry field"));
            }
            match key.as_str() {
                "path" => {
                    path = Some(map.next_value_seed(MetaString {
                        budget: self.budget,
                    })?)
                }
                "category" => {
                    category = Some(map.next_value_seed(MetaString {
                        budget: self.budget,
                    })?)
                }
                "digest" => {
                    digest = Some(map.next_value_seed(MetaString {
                        budget: self.budget,
                    })?)
                }
                "bytes" => {
                    let value = map.next_value_seed(MetaValue {
                        budget: self.budget,
                    })?;
                    bytes =
                        Some(value.as_u64().ok_or_else(|| {
                            DeError::custom("bytes must be a non-negative integer")
                        })?);
                }
                "source_identity" => {
                    // A private frozen-plan field; it must never appear in a
                    // package. Refuse rather than silently accept.
                    source_identity = true;
                    let _ = map.next_value_seed(MetaValue {
                        budget: self.budget,
                    })?;
                }
                "data" => {
                    // The data array is admitted with its own declared-size
                    // budget, then charging returns to metadata.
                    data = Some(map.next_value_seed(DataRead {
                        staging: self.staging,
                        index: self.index,
                        limit: self.limits.file_bytes,
                        declared: bytes,
                        budget: self.budget,
                    })?);
                    self.budget.after_data();
                }
                _ => return Err(DeError::custom("unknown entry field")),
            }
        }
        if source_identity {
            return Err(DeError::custom("package must not carry source_identity"));
        }
        let path = path.ok_or_else(|| DeError::custom("entry missing path"))?;
        let category = category.ok_or_else(|| DeError::custom("entry missing category"))?;
        let digest = digest.ok_or_else(|| DeError::custom("entry missing digest"))?;
        let (plain, actual_bytes, actual_digest) =
            data.ok_or_else(|| DeError::custom("entry missing data"))?;
        if let Some(declared) = bytes {
            if actual_bytes != declared {
                return Err(DeError::custom("data byte count mismatch"));
            }
        }
        if actual_digest != digest {
            return Err(DeError::custom("data digest mismatch"));
        }
        Ok(Entry {
            path,
            category,
            digest,
            bytes: actual_bytes,
            plain,
        })
    }
}

struct DataRead<'a> {
    staging: &'a Staging,
    index: usize,
    limit: u64,
    declared: Option<u64>,
    budget: Budget<'a>,
}

impl<'de> DeserializeSeed<'de> for DataRead<'_> {
    type Value = (PathBuf, u64, String);
    fn deserialize<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<(PathBuf, u64, String), D::Error> {
        let mut file = self
            .staging
            .entry_file(self.index)
            .map_err(|error| serde::de::Error::custom(error.message))?;
        let path = self
            .staging
            .dir
            .join(format!("entry-{:08}.bin", self.index));
        let mut hasher = Sha256::new();
        let allowed = match self.declared {
            Some(declared) => declared.min(self.limit),
            None => self.limit,
        };
        // Admit the array opening, then reset before each individual byte.
        self.budget.data_token();
        let result = deserializer.deserialize_seq(DataVisitor {
            file: &mut file,
            hasher: &mut hasher,
            limit: allowed,
            budget: self.budget,
        });
        match result {
            Ok(count) => {
                file.sync_all()
                    .map_err(|error| serde::de::Error::custom(error.to_string()))?;
                Ok((path, count, format!("{:x}", hasher.finalize_reset())))
            }
            Err(error) => {
                let _ = fs::remove_file(&path);
                Err(error)
            }
        }
    }
}

struct DataVisitor<'a> {
    file: &'a mut File,
    hasher: &'a mut Sha256,
    limit: u64,
    budget: Budget<'a>,
}

impl<'de> Visitor<'de> for DataVisitor<'_> {
    type Value = u64;
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("an array of byte values")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<u64, A::Error> {
        use serde::de::Error as DeError;
        let mut buf = [0u8; 64 * 1024];
        let mut filled = 0usize;
        let mut count: u64 = 0;
        loop {
            self.budget.data_token();
            let Some(byte) = seq.next_element::<u8>()? else {
                break;
            };
            if count >= self.limit {
                return Err(DeError::custom("data over declared size"));
            }
            buf[filled] = byte;
            filled += 1;
            count += 1;
            if filled == buf.len() {
                self.file
                    .write_all(&buf[..filled])
                    .map_err(DeError::custom)?;
                self.hasher.update(&buf[..filled]);
                filled = 0;
            }
        }
        if filled > 0 {
            self.file
                .write_all(&buf[..filled])
                .map_err(DeError::custom)?;
            self.hasher.update(&buf[..filled]);
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::digest;
    use serde_json::json;

    const PASS: &str = "synthetic package passphrase";

    fn staging() -> (tempfile::TempDir, Staging) {
        let temp = tempfile::tempdir().unwrap();
        let st = Staging::new(&temp.path().canonicalize().unwrap()).unwrap();
        (temp, st)
    }

    fn limits() -> Limits {
        Limits {
            file_bytes: 256 * 1024 * 1024,
            total_bytes: 1024 * 1024 * 1024,
            files: 10000,
        }
    }

    fn write_pkg(base: &Path, bytes: &[u8]) -> (PathBuf, String) {
        let plain = base.join("source.bin");
        fs::write(&plain, bytes).unwrap();
        let digest_hex = digest(bytes);
        let value = json!({
            "schema": SCHEMA,
            "generator": GENERATOR,
            "created_at": "2026-01-01T00:00:00Z",
            "notes": "synthetic",
            "files": [{
                "path": "projects/demo/session.jsonl",
                "category": "sessions",
                "digest": digest_hex,
            }],
        });
        let out = base.join("work.age");
        let mut file = File::create(&out).unwrap();
        let (cipher, total) = write(
            &mut file,
            &value,
            &[Source {
                path: "projects/demo/session.jsonl",
                category: "sessions",
                expected_digest: &digest_hex,
                plain: &plain,
            }],
            PASS,
        )
        .unwrap();
        file.sync_all().unwrap();
        assert_eq!(total, bytes.len() as u64);
        (out, cipher)
    }

    #[test]
    fn hostile_tokens_and_metadata_are_refused_before_materialization() {
        struct Counted<R> {
            inner: R,
            count: Rc<Cell<u64>>,
        }
        impl<R: Read> Read for Counted<R> {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                let n = self.inner.read(buf)?;
                self.count.set(self.count.get() + n as u64);
                Ok(n)
            }
        }
        for (prefix, bound) in [
            (
                br#"{"schema":"lintel.work/1","files":[{"data":[""#.as_slice(),
                512,
            ),
            (
                br#"{"schema":"lintel.work/1","notes":""#.as_slice(),
                META_FIELD_BYTES + 512,
            ),
        ] {
            let count = Rc::new(Cell::new(0));
            let input =
                std::io::Cursor::new(prefix).chain(std::io::repeat(b'x').take(1024 * 1024 * 1024));
            let (_temp, stage) = staging();
            assert!(decode_plain(
                Counted {
                    inner: input,
                    count: count.clone()
                },
                &stage,
                limits()
            )
            .is_err());
            assert!(
                count.get() <= bound,
                "hostile field was read past its budget: {}",
                count.get()
            );
        }
        for input in [
            br#"{"schema":"lintel.work/1","files":[],"notes":null,"notes":null}"#.as_slice(),
            br#"{"schema":"lintel.work/1","files":[],"notes":{"x":null,"x":null}}"#.as_slice(),
        ] {
            let (_temp, stage) = staging();
            assert!(decode_plain(std::io::Cursor::new(input), &stage, limits()).is_err());
        }
    }

    #[test]
    fn streaming_round_trip_preserves_exact_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let bytes: Vec<u8> = (0..10_000u32).map(|i| (i % 251) as u8).collect();
        let (out, cipher) = write_pkg(&base, &bytes);
        let (_t, st) = staging();
        let (package, digest_hex) = read_to_temp(&out, PASS, &st, limits()).unwrap();
        assert_eq!(package.schema, SCHEMA);
        assert_eq!(package.generator, json!(GENERATOR));
        assert_eq!(digest_hex, cipher);
        assert_eq!(package.files.len(), 1);
        assert_eq!(package.files[0].bytes, bytes.len() as u64);
        assert_eq!(package.files[0].digest, digest(&bytes));
        assert_eq!(fs::read(&package.files[0].plain).unwrap(), bytes);
        // Wrong passphrase refused.
        assert_eq!(
            read_to_temp(&out, "wrong passphrase here", &st, limits())
                .unwrap_err()
                .code,
            "archive_locked"
        );
    }

    #[test]
    fn excessive_kdf_and_concurrent_cipher_replacement_are_refused() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let path = base.join("work.age");
        let mut bytes = write_value(&json!({"schema":SCHEMA,"files":[]}), PASS).unwrap();
        let header_end = bytes.iter().position(|b| *b == b'\n').unwrap() + 1;
        let stanza_end = header_end
            + bytes[header_end..]
                .iter()
                .position(|b| *b == b'\n')
                .unwrap();
        let stanza = std::str::from_utf8(&bytes[header_end..stanza_end]).unwrap();
        let salt = stanza.split_whitespace().nth(2).unwrap();
        let expensive = format!("-> scrypt {salt} {}", MAX_SCRYPT_LOG_N + 1);
        bytes.splice(header_end..stanza_end, expensive.bytes());
        fs::write(&path, &bytes).unwrap();
        let (_temp, stage) = staging();
        assert_eq!(
            read_to_temp(&path, PASS, &stage, limits())
                .unwrap_err()
                .code,
            "archive_limit"
        );
        let (handle, identity) = open_cipher(&path, limits().total_bytes).unwrap();
        let replacement = base.join("replacement.age");
        fs::write(&replacement, &bytes).unwrap();
        fs::rename(&replacement, &path).unwrap();
        assert_eq!(
            check_cipher(&path, &handle, &identity).unwrap_err().code,
            "stale_archive"
        );
    }

    #[test]
    fn declared_digest_mismatch_is_refused() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let plain = base.join("s.bin");
        fs::write(&plain, b"actual bytes").unwrap();
        let zero = "0".repeat(64);
        let value = json!({
            "schema": SCHEMA, "generator": GENERATOR, "created_at": "x",
            "files": [{"path":"projects/a.jsonl","category":"sessions","digest":zero,"data":b"actual bytes"}],
        });
        let out = base.join("bad.age");
        fs::write(&out, write_value(&value, PASS).unwrap()).unwrap();
        let (_t, st) = staging();
        assert!(read_to_temp(&out, PASS, &st, limits()).is_err());
    }

    #[test]
    fn truncation_and_duplicate_keys_are_refused() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let good = json!({"schema":SCHEMA,"generator":GENERATOR,"created_at":"x","files":[]});
        let full = write_value(&good, PASS).unwrap();
        let path = base.join("full.age");
        fs::write(&path, &full).unwrap();
        let (_t, st) = staging();
        assert_eq!(
            read_to_temp(&path, PASS, &st, limits())
                .unwrap()
                .0
                .files
                .len(),
            0
        );
        // Missing final tag.
        let truncated = base.join("truncated.age");
        fs::write(&truncated, &full[..full.len() - 20]).unwrap();
        assert!(read_to_temp(&truncated, PASS, &st, limits()).is_err());
        // Appended garbage after the age stream.
        let mut extra = full.clone();
        extra.push(0);
        let trailing = base.join("trailing.age");
        fs::write(&trailing, &extra).unwrap();
        assert!(read_to_temp(&trailing, PASS, &st, limits()).is_err());
        // Duplicate top-level key.
        let dup = base.join("dup.age");
        let encryptor = age::Encryptor::with_user_passphrase(age::secrecy::SecretString::from(
            PASS.to_string(),
        ));
        let mut bytes = vec![];
        let mut stream = encryptor.wrap_output(&mut bytes).unwrap();
        stream
            .write_all(br#"{"schema":"lintel.work/1","schema":"lintel.work/1","files":[]}"#)
            .unwrap();
        stream.finish().unwrap();
        fs::write(&dup, &bytes).unwrap();
        assert!(read_to_temp(&dup, PASS, &st, limits()).is_err());
        // Schema is a strict string, independently of digest/body validation.
        let wrong = base.join("wrong.age");
        let encryptor = age::Encryptor::with_user_passphrase(age::secrecy::SecretString::from(
            PASS.to_string(),
        ));
        let mut bytes = vec![];
        let mut stream = encryptor.wrap_output(&mut bytes).unwrap();
        stream
            .write_all(
                br#"{"schema":5,"files":[]}"#,
            )
            .unwrap();
        stream.finish().unwrap();
        fs::write(&wrong, &bytes).unwrap();
        assert!(read_to_temp(&wrong, PASS, &st, limits()).is_err());
    }

    #[test]
    fn declared_byte_count_mismatch_is_refused() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let path = base.join("mismatch.age");
        let encryptor = age::Encryptor::with_user_passphrase(age::secrecy::SecretString::from(
            PASS.to_string(),
        ));
        let payload = format!(
            r#"{{"schema":"lintel.work/1","files":[{{"path":"p","category":"sessions","digest":"{}","bytes":99,"data":[65,66]}}]}}"#,
            digest(b"AB")
        );
        let mut bytes = vec![];
        let mut stream = encryptor.wrap_output(&mut bytes).unwrap();
        stream.write_all(payload.as_bytes()).unwrap();
        stream.finish().unwrap();
        fs::write(&path, &bytes).unwrap();
        let (_t, st) = staging();
        assert!(read_to_temp(&path, PASS, &st, limits()).is_err());
    }
}

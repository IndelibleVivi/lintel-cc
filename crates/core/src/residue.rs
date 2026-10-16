//! Bounded, read-only observation of the local data and identity surface.
//!
//! Two questions, both answered without any content read:
//!   * which known local locations exist, and how much data / how recent they
//!     are (work content, runtime residue, identity material, plugin config);
//!   * whether the two documented persistent client-state identity keys are
//!     present — as presence flags plus a salted private digest used only to
//!     compare two snapshots.
//!
//! Known coverage is a finite table. Anything outside it is *not covered* rather
//! than assumed absent. Identity values never leave this module: the public
//! projection carries presence/persistence flags, never a field value.
// The surface is `pub(crate)` and complete but not yet reached from
// `dispatch`/`Engine`; the callers that consume it are the next change. Keeping
// the allow here (rather than deleting the surface) is what makes this module
// reviewable as a whole.
#![allow(dead_code)]
use crate::{err, now, storage::*, Result};
use serde_json::{json, Map, Value};
use std::{
    fs::{self, OpenOptions},
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

/// Bounded recursion inside one location. The location directory itself is depth
/// 0, so entries at path depth 1..=MAX_DEPTH are collected; a deeper directory is
/// never entered, and declining that entry marks the location truncated.
const MAX_DEPTH: usize = 4;
/// Directory entries visited per location, across all depths.
const MAX_ENTRIES: usize = 5000;
/// Listed rows per location. The aggregates (`file_count`/`total_bytes`) stay
/// real even when the list is capped.
const MAX_LISTED: usize = 100;
/// Identity observation bound: a larger client-state file is reported as
/// `too_large` and no byte of it is read.
const IDENTITY_BYTES: u64 = 2 * 1024 * 1024;

/// Two locations in the table are not plain `root`-relative paths; each carries a
/// self-describing prefix resolved by [`resolve_location`]:
///   * `${HOME}/…`         — relative to the user home;
///   * `${CLIENT_STATE}/…` — the client-state directory, which is `home` when the
///     observed root IS `home/.claude` and the root itself otherwise (the same
///     rule `cleanup.rs` uses to locate the client-state file).
const HOME_SENTINEL: &str = "${HOME}/";
const CLIENT_STATE_SENTINEL: &str = "${CLIENT_STATE}/";
const CLIENT_STATE_ENTRY: &str = "${CLIENT_STATE}/.claude.json";

/// The finite known-location table: (relative path, category, class) with
/// `class ∈ work | residue | identity | config`. A relative path that is a file
/// is presence-only; an `identity` location is presence-only even when it is a
/// directory. Nothing outside this table is scanned, in any mode.
pub(crate) const LOCATION_TABLE: &[(&str, &str, &str)] = &[
    ("debug", "debug_logs", "residue"),
    ("shell-snapshots", "shell_snapshots", "residue"),
    ("todos", "todo_state", "residue"),
    ("backups", "config_backups", "residue"),
    ("session-env", "session_env", "residue"),
    ("projects", "sessions", "work"),
    ("plugins", "plugins", "config"),
    (".credentials.json", "credentials", "identity"),
    (CLIENT_STATE_ENTRY, "client_state", "identity"),
    (
        "${CLIENT_STATE}/.claude.json.backup",
        "client_state_backup",
        "residue",
    ),
    ("${HOME}/.config/anthropic", "shared_profile", "identity"),
];

/// The acknowledged persistent identity keys. `userID` is the random anonymous
/// identifier generated on first run and persisted in the client-state file (not
/// personal data, not derived from an account; deleting the file yields an
/// unrelated new value on the next run). `oauthAccount` is only evidence that an
/// account system was signed in to at some point.
const IDENTITY_KEYS: &[(&str, &str)] = &[
    ("userID", "persistent_across_sessions"),
    ("oauthAccount", "session_account"),
];

const IDENTITY_NOTE: &str = "只识别 userID 与 oauthAccount 两个已知键，且只报告存在性与持久性，不返回任何字段值。userID 是首次运行生成的随机匿名标识，持久化在 client_state 中，不含个人信息、不由账号派生；oauthAccount 的存在只表示曾登录过账号体系。跨快照比较需要调用方持久保存同一个 salt：不同 salt 会对同一 userID 产生不同 digest，此时两次观察只能比出 unknown。";

fn resolve_location(home: &Path, root: &Path, relative: &str) -> PathBuf {
    if let Some(tail) = relative.strip_prefix(CLIENT_STATE_SENTINEL) {
        // cleanup.rs picks home/.claude.json when the root IS home/.claude, and
        // root/.claude.json otherwise; the ".backup" sibling follows the same base.
        let base = if root == home.join(".claude") {
            home
        } else {
            root
        };
        return base.join(tail);
    }
    if let Some(tail) = relative.strip_prefix(HOME_SENTINEL) {
        return home.join(tail);
    }
    root.join(relative)
}

#[derive(Default)]
struct Scan {
    files: Vec<(PathBuf, u64, i64)>,
    file_count: u64,
    total_bytes: u64,
    newest_mtime: Option<i64>,
    truncated: bool,
    skipped_special: u64,
    skipped_unsafe: u64,
}

/// Recursive metadata walk of one location directory. Entries are never opened:
/// type/size/mtime only. Symlinks are never followed, non-regular entries are
/// skipped as special, and multi-link / foreign-owner entries are skipped as
/// unsafe — all three are counted instead of silently dropped.
fn scan_directory(dir: &Path) -> Result<Scan> {
    let euid = unsafe { libc::geteuid() };
    let mut scan = Scan::default();
    let mut stack = vec![(dir.to_path_buf(), 0usize)];
    let mut visited = 0usize;
    'walk: while let Some((current, depth)) = stack.pop() {
        let mut items = fs::read_dir(&current)?;
        loop {
            let item = match items.next() {
                None => break,
                Some(Ok(item)) => item,
                // One unreadable entry never discards the rest of the location.
                Some(Err(_)) => {
                    scan.truncated = true;
                    continue;
                }
            };
            visited += 1;
            if visited > MAX_ENTRIES {
                scan.truncated = true;
                break 'walk;
            }
            let path = item.path();
            let meta = match fs::symlink_metadata(&path) {
                Ok(meta) => meta,
                Err(_) => {
                    scan.truncated = true;
                    continue;
                }
            };
            let kind = meta.file_type();
            if kind.is_symlink() {
                // Never followed. Under `symlink_metadata` a symlink is not a
                // regular file, so it is counted with the special entries.
                scan.skipped_special += 1;
                continue;
            }
            if kind.is_dir() {
                let child = depth + 1;
                if child < MAX_DEPTH {
                    stack.push((path, child));
                } else if fs::read_dir(&path).map_or(true, |mut inner| inner.next().is_some()) {
                    // Beyond the depth bound: never entered. Report the cut only
                    // when that directory actually holds entries, so an empty one
                    // is not a false truncation.
                    scan.truncated = true;
                }
                continue;
            }
            if !kind.is_file() {
                // FIFO/socket/device: skipped, and never opened (a blocking open
                // on a FIFO would stall the scan).
                scan.skipped_special += 1;
                continue;
            }
            if meta.nlink() != 1 || meta.uid() != euid {
                scan.skipped_unsafe += 1;
                continue;
            }
            let bytes = meta.len();
            let mtime = meta.mtime();
            scan.file_count += 1;
            scan.total_bytes += bytes;
            scan.newest_mtime = Some(scan.newest_mtime.map_or(mtime, |seen| seen.max(mtime)));
            scan.files.push((path, bytes, mtime));
        }
    }
    Ok(scan)
}

/// Observe one table location. A failure here is local: it becomes
/// `state: "access_limited"` and the remaining locations still run.
fn observe_location(home: &Path, root: &Path, relative: &str, category: &str, class: &str) -> Value {
    let path = resolve_location(home, root, relative);
    let mut out = json!({
        "path": path,
        "category": category,
        "class": class,
        "state": "not_found",
        "file_count": 0,
        "total_bytes": 0,
        "newest_mtime": Value::Null,
        "entries": [],
        "truncated": false,
        "skipped_special": 0,
        "skipped_unsafe": 0,
    });
    if guard(&path).is_err() {
        out["state"] = json!("access_limited");
        return out;
    }
    let meta = match fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return out,
        Err(_) => {
            out["state"] = json!("access_limited");
            return out;
        }
    };
    // Presence-only: identity material (credential/global state/shared profile)
    // is never entered, and a location that is a file records only its own
    // metadata. No byte of any of these is read here.
    if !meta.is_dir() || class == "identity" {
        out["state"] = json!("present");
        if meta.is_file() {
            out["file_count"] = json!(1);
            out["total_bytes"] = json!(meta.len());
            out["newest_mtime"] = json!(meta.mtime());
        }
        return out;
    }
    let scan = match scan_directory(&path) {
        Ok(scan) => scan,
        Err(_) => {
            out["state"] = json!("access_limited");
            return out;
        }
    };
    out["state"] = json!("present");
    out["file_count"] = json!(scan.file_count);
    out["total_bytes"] = json!(scan.total_bytes);
    out["newest_mtime"] = json!(scan.newest_mtime);
    out["skipped_special"] = json!(scan.skipped_special);
    out["skipped_unsafe"] = json!(scan.skipped_unsafe);
    let mut truncated = scan.truncated;
    let mut files = scan.files;
    // Largest first, with the absolute path as a stable tie-break.
    files.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    if files.len() > MAX_LISTED {
        files.truncate(MAX_LISTED);
        truncated = true;
    }
    let entries: Vec<Value> = files
        .iter()
        .map(|(entry_path, bytes, mtime)| {
            let mut entry = json!({"path": entry_path, "bytes": bytes, "mtime": mtime});
            if class == "work" {
                // Shared static rule; null when the path is not work content.
                let relative = entry_path.strip_prefix(root).unwrap_or(entry_path.as_path());
                entry["work_category"] = json!(lintel_operations::work_path_category(relative));
            }
            entry
        })
        .collect();
    out["entries"] = json!(entries);
    out["truncated"] = json!(truncated);
    out
}

/// Bounded read-only inventory of the known local-data and identity locations.
/// Metadata only: no file body is read, no file is opened for a directory
/// location, and nothing outside [`LOCATION_TABLE`] is scanned.
pub(crate) fn inventory(home: &Path, root: &Path) -> Result<Value> {
    guard(root)?;
    match fs::symlink_metadata(root) {
        Ok(_) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(err("root_missing", "找不到要观察的配置目录；未扫描任何位置"))
        }
        Err(error) => return Err(error.into()),
    }
    let locations: Vec<Value> = LOCATION_TABLE
        .iter()
        .map(|(relative, category, class)| observe_location(home, root, relative, category, class))
        .collect();
    let file_count: u64 = locations
        .iter()
        .filter_map(|location| location["file_count"].as_u64())
        .sum();
    let total_bytes: u64 = locations
        .iter()
        .filter_map(|location| location["total_bytes"].as_u64())
        .sum();
    // A complete scan means every table location was readable and none reached a
    // depth/entry/list bound. `not_found` is a complete observation, not a gap.
    let scan_complete = locations
        .iter()
        .all(|location| location["state"] != "access_limited" && location["truncated"] != true);
    Ok(json!({
        "schema": "lintel.residue/1",
        "root": root,
        "checked_at": now(),
        "locations": locations,
        "totals": {"file_count": file_count, "total_bytes": total_bytes},
        "scan_complete": scan_complete,
        "note": "只按已知位置表观察文件元数据（类型/大小/时间），不读取任何文件正文；表外位置（例如 root 下其他目录）不在本次覆盖范围，因此不代表本地不存在其他数据。identity 位置只报存在性，不进入。truncated 表示该位置到达深度/条目/列表上限；scan_complete 为 false 表示至少一个位置达到上限或无法读取。",
    }))
}

/// What one client-state observation produced. Only [`ClientState::Observed`]
/// carries a parsed document, and even then only the two allowlisted keys are
/// ever consulted.
enum ClientState {
    Missing,
    TooLarge,
    Unreadable,
    Unknown,
    Observed(Value),
}

fn metadata_identity(meta: &fs::Metadata) -> Value {
    json!({"device":meta.dev(),"inode":meta.ino(),"owner":meta.uid(),"mode":meta.mode(),
        "bytes":meta.len(),"mtime":meta.mtime(),"mtime_nsec":meta.mtime_nsec(),
        "ctime":meta.ctime(),"ctime_nsec":meta.ctime_nsec()})
}

/// Read the client-state document under the same discipline `startup.rs` uses for
/// shared sources: no symlink, no non-regular file, no multi-link/foreign file,
/// `O_NOFOLLOW | O_NONBLOCK`, and the before/during/after identity must agree. A
/// change at any point is `unreadable` rather than a guess.
fn read_client_state(path: &Path) -> ClientState {
    if guard(path).is_err() {
        return ClientState::Unreadable;
    }
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return ClientState::Missing,
        Err(_) => return ClientState::Unreadable,
    };
    if !meta.is_file() || meta.nlink() != 1 || meta.uid() != unsafe { libc::geteuid() } {
        return ClientState::Unreadable;
    }
    if meta.len() > IDENTITY_BYTES {
        return ClientState::TooLarge;
    }
    let mut file = match OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
    {
        Ok(file) => file,
        Err(_) => return ClientState::Unreadable,
    };
    let opened = match file.metadata() {
        Ok(meta) => metadata_identity(&meta),
        Err(_) => return ClientState::Unreadable,
    };
    if opened != metadata_identity(&meta) {
        return ClientState::Unreadable;
    }
    let mut bytes = Vec::new();
    if (&mut file)
        .take(IDENTITY_BYTES + 1)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return ClientState::Unreadable;
    }
    if bytes.len() as u64 > IDENTITY_BYTES {
        return ClientState::TooLarge;
    }
    let after = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(_) => return ClientState::Unreadable,
    };
    if file
        .metadata()
        .map(|m| metadata_identity(&m))
        .unwrap_or(Value::Null)
        != opened
        || metadata_identity(&after) != metadata_identity(&meta)
    {
        return ClientState::Unreadable;
    }
    match parse(&bytes) {
        Ok(doc) if doc.is_object() => ClientState::Observed(doc),
        // Unparsable or duplicated keys: reported as unknown. Nothing is
        // extracted from a document we could not parse.
        _ => ClientState::Unknown,
    }
}

/// Masked observation of the persistent identity fields. Returns one value with
/// two parts: `public` (safe to publish: presence/persistence flags only, never a
/// field value) and `binding` (private: a salted digest over the raw value, used
/// only by [`identity_compare`]).
pub(crate) fn identity_observation(home: &Path, root: &Path, salt: &str) -> Result<Value> {
    let path = resolve_location(home, root, CLIENT_STATE_ENTRY);
    let mut public = json!({
        "schema": "lintel.identity/1",
        "path": path,
        "state": "not_found",
        "fields": [],
        "note": IDENTITY_NOTE,
    });
    let mut digests = Map::new();
    match read_client_state(&path) {
        ClientState::Missing => {}
        ClientState::TooLarge => {
            public["state"] = json!("too_large");
            public["note"] =
                json!(format!("{IDENTITY_NOTE} client_state 超过 2 MiB 上限，未读取任何内容。"));
        }
        ClientState::Unreadable => {
            public["state"] = json!("unreadable");
            public["note"] = json!(format!(
                "{IDENTITY_NOTE} client_state 不可按当前纪律读取（符号链接、非普通文件、多硬链接、非本人所有，或读取期间发生变化），未产出任何字段。"
            ));
        }
        ClientState::Unknown => {
            public["state"] = json!("unknown");
            public["note"] = json!(format!(
                "{IDENTITY_NOTE} client_state 不是可解析的 JSON 对象；不猜测提取任何字段。"
            ));
        }
        ClientState::Observed(doc) => {
            public["state"] = json!("observed");
            let user_id = doc
                .get("userID")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty());
            let account = doc.get("oauthAccount").is_some_and(Value::is_object);
            let fields: Vec<Value> = IDENTITY_KEYS
                .iter()
                .map(|(key, persistence)| {
                    json!({
                        "key": key,
                        "present": match *key {
                            "userID" => user_id.is_some(),
                            _ => account,
                        },
                        "persistence": persistence,
                        "storage": "client_state",
                        "masked": true,
                    })
                })
                .collect();
            public["fields"] = json!(fields);
            let mut notes = vec![];
            if doc.get("userID").is_some() && user_id.is_none() {
                notes.push("userID 存在但不是非空字符串，未识别为持久标识");
            }
            if doc.get("oauthAccount").is_some() && !account {
                notes.push("oauthAccount 存在但不是对象，未识别为账号体系痕迹");
            }
            if let Some(value) = user_id {
                // Salted so the private binding can compare two observations
                // without publishing a reusable digest of the raw identifier.
                digests.insert(
                    "userID".into(),
                    json!(digest(format!("{salt}|userID|{value}").as_bytes())),
                );
            }
            if !notes.is_empty() {
                public["note"] = json!(format!("{IDENTITY_NOTE} 附加：{}。", notes.join("；")));
            }
        }
    }
    let binding = json!({
        "schema": "lintel.identity-binding/1",
        "salt_schema": "sha256(salt|key|value)",
        "digests": Value::Object(digests),
    });
    Ok(json!({"public": public, "binding": binding}))
}

/// Compare two private bindings. Output is exactly one of `same` / `changed` /
/// `unknown` and never a digest. A missing digest on either side (no client-state
/// file, no recognizable `userID`, or unreadable state) is `unknown`: absence is
/// not evidence of a change.
pub(crate) fn identity_compare(a_binding: &Value, b_binding: &Value) -> Value {
    let digest_of = |binding: &Value| -> Option<String> {
        binding["digests"]["userID"].as_str().map(str::to_string)
    };
    let state = match (digest_of(a_binding), digest_of(b_binding)) {
        (Some(a), Some(b)) if a == b => "same",
        (Some(_), Some(_)) => "changed",
        _ => "unknown",
    };
    json!({"userID": state})
}

#[cfg(test)]
#[path = "residue_tests.rs"]
mod tests;

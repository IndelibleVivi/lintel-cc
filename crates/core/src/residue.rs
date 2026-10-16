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
pub(crate) const CLIENT_STATE_ENTRY: &str = "${CLIENT_STATE}/.claude.json";

/// The finite known-location table: (relative path, category, class, adapter)
/// with `class ∈ work | residue | identity | config`.
///
/// The `adapter` column states how well this build actually knows the location,
/// so the surface never overclaims coverage:
///   * `known`         — layout verified against a supported client version;
///   * `unclassified`  — a plausible/legacy location whose exact contents are
///     version-dependent: presence and metadata are reported, but nothing about
///     the layout is asserted.
/// A relative path that is a file is presence-only; an `identity` location is
/// presence-only even when it is a directory. Nothing outside this table (plus
/// an explicitly requested `extra_roots` directory) is ever scanned.
pub(crate) const LOCATION_TABLE: &[(&str, &str, &str, &str)] = &[
    ("debug", "debug_logs", "residue", "known"),
    ("shell-snapshots", "shell_snapshots", "residue", "known"),
    ("todos", "todo_state", "residue", "known"),
    ("backups", "config_backups", "residue", "known"),
    ("session-env", "session_env", "residue", "known"),
    ("projects", "sessions", "work", "known"),
    ("plugins", "plugins", "config", "known"),
    (".credentials.json", "credentials", "identity", "known"),
    (CLIENT_STATE_ENTRY, "client_state", "identity", "known"),
    (
        "${CLIENT_STATE}/.claude.json.backup",
        "client_state_backup",
        "residue",
        "known",
    ),
    (
        "${HOME}/.config/anthropic",
        "shared_profile",
        "identity",
        "known",
    ),
    // Version-dependent locations: reported when present, never asserted as a
    // complete classifier. A future version may rename or restructure these.
    (
        "file-history",
        "file_history",
        "unclassified",
        "unclassified",
    ),
    (
        "tool-results",
        "tool_results",
        "unclassified",
        "unclassified",
    ),
    ("image-cache", "image_cache", "unclassified", "unclassified"),
    (
        "statsig",
        "feature_flags_cache",
        "unclassified",
        "unclassified",
    ),
    ("plans", "plan_state", "unclassified", "unclassified"),
    ("ide", "ide_state", "unclassified", "unclassified"),
];

/// The single settings key this module reads. It is a location selector
/// (`0` / `1` / `file:<dir>`), never a credential: the value's directory is
/// reported, but the value is never echoed and no other env key is consulted.
pub(crate) const RAW_BODY_KEY: &str = "OTEL_LOG_RAW_API_BODIES";
/// Bounded read of the settings document for that one key. Only this key is
/// extracted; a larger document is refused rather than partially trusted.
const SETTINGS_READ_BYTES: u64 = 4 * 1024 * 1024;
/// Maximum caller-supplied explicit scan roots per request.
pub(crate) const MAX_EXTRA_ROOTS: usize = 8;
/// Inventory page bounds. The default keeps a first page small; `<= 200` keeps
/// any single page a bounded payload.
const INVENTORY_PAGE_ROWS: usize = 200;
const INVENTORY_MAX_ROWS: usize = 200;
/// Largest flat row set one request will list. Coverage/truncation is reported
/// independently of the page, so a capped set still pages deterministically.
const INVENTORY_ROW_BUDGET: usize = 5000;

/// The acknowledged persistent identity keys. `userID` is the random anonymous
/// identifier generated on first run and persisted in the client-state file (not
/// personal data, not derived from an account; deleting the file yields an
/// unrelated new value on the next run). `oauthAccount` is only evidence that an
/// account system was signed in to at some point.
pub(crate) const IDENTITY_KEYS: &[(&str, &str)] = &[
    ("userID", "persistent_across_sessions"),
    ("oauthAccount", "session_account"),
];

pub(crate) const IDENTITY_NOTE: &str = "只识别 userID 与 oauthAccount 两个已知键，且只报告存在性与持久性，不返回任何字段值。userID 是首次运行生成的随机匿名标识，持久化在 client_state 中，不含个人信息、不由账号派生；oauthAccount 的存在只表示曾登录过账号体系。跨快照比较需要调用方持久保存同一个 salt：不同 salt 会对同一 userID 产生不同 digest，此时两次观察只能比出 unknown。";

pub(crate) fn resolve_location(home: &Path, root: &Path, relative: &str) -> PathBuf {
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

/// The one client-state document this crate may read for identity work, under
/// the shared `${CLIENT_STATE}/.claude.json` location rule. Callers never derive
/// this path themselves, so no second rule can disagree with `cleanup.rs`.
pub(crate) fn client_state_path(home: &Path, root: &Path) -> PathBuf {
    resolve_location(home, root, CLIENT_STATE_ENTRY)
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

/// One observed location: its finite public summary plus the flat metadata rows
/// it contributes. Every field is metadata-only; no entry is ever opened here.
struct Observation {
    path: PathBuf,
    state: &'static str,
    category: String,
    class: String,
    adapter: String,
    origin: &'static str,
    file_count: u64,
    total_bytes: u64,
    newest_mtime: Option<i64>,
    truncated: bool,
    skipped_special: u64,
    skipped_unsafe: u64,
    rows: Vec<(PathBuf, u64, i64)>,
}

impl Observation {
    /// The legacy per-location summary (aggregates and state, no entries).
    fn summary(&self) -> Value {
        json!({
            "path": self.path,
            "category": self.category,
            "class": self.class,
            "adapter": self.adapter,
            "origin": self.origin,
            "state": self.state,
            "file_count": self.file_count,
            "total_bytes": self.total_bytes,
            "newest_mtime": self.newest_mtime,
            "truncated": self.truncated,
            "skipped_special": self.skipped_special,
            "skipped_unsafe": self.skipped_unsafe,
        })
    }
}

/// Observe one already-resolved location. A failure here is local: it becomes
/// `state: "access_limited"` and the remaining locations still run.
fn observe_path(
    path: PathBuf,
    root: &Path,
    category: &str,
    class: &str,
    adapter: &str,
    origin: &'static str,
) -> Observation {
    let mut observation = Observation {
        path,
        state: "not_found",
        category: category.to_string(),
        class: class.to_string(),
        adapter: adapter.to_string(),
        origin,
        file_count: 0,
        total_bytes: 0,
        newest_mtime: None,
        truncated: false,
        skipped_special: 0,
        skipped_unsafe: 0,
        rows: vec![],
    };
    let _ = root;
    if guard(&observation.path).is_err() {
        observation.state = "access_limited";
        return observation;
    }
    let meta = match fs::symlink_metadata(&observation.path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return observation,
        Err(_) => {
            observation.state = "access_limited";
            return observation;
        }
    };
    // Presence-only: identity material (credential/global state/shared profile)
    // is never entered, and a location that is a file records only its own
    // metadata. No byte of any of these is read here.
    if !meta.is_dir() || class == "identity" {
        observation.state = "present";
        if meta.is_file() {
            observation.file_count = 1;
            observation.total_bytes = meta.len();
            observation.newest_mtime = Some(meta.mtime());
            // Identity material stays presence-only: it is never listed as a
            // row, so no path/value of it can be selected for an operation.
            if class != "identity" {
                observation
                    .rows
                    .push((observation.path.clone(), meta.len(), meta.mtime()));
            }
        }
        return observation;
    }
    let scan = match scan_directory(&observation.path) {
        Ok(scan) => scan,
        Err(_) => {
            observation.state = "access_limited";
            return observation;
        }
    };
    observation.state = "present";
    observation.file_count = scan.file_count;
    observation.total_bytes = scan.total_bytes;
    observation.newest_mtime = scan.newest_mtime;
    observation.skipped_special = scan.skipped_special;
    observation.skipped_unsafe = scan.skipped_unsafe;
    observation.truncated = scan.truncated;
    observation.rows = scan.files;
    observation
}

/// Observe one table location by its self-describing relative path.
fn observe_location_path(
    home: &Path,
    root: &Path,
    relative: &str,
    category: &str,
    class: &str,
    adapter: &str,
) -> Observation {
    observe_path(
        resolve_location(home, root, relative),
        root,
        category,
        class,
        adapter,
        "known_table",
    )
}

/// The shared work-category annotation for a row under `root` (or `null`).
fn work_category_of(root: &Path, entry_path: &Path) -> Value {
    let relative = entry_path.strip_prefix(root).unwrap_or(entry_path);
    json!(lintel_operations::work_path_category(relative))
}

/// The default query uses the same canonical paged implementation.
pub(crate) fn inventory(home: &Path, root: &Path) -> Result<Value> {
    inventory_request(home, root, &json!({}))
}

/// Read *only* the raw-body-record directory selector from `root/settings.json`.
///
/// The document is read under the shared bounded non-following discipline, but
/// only `env.OTEL_LOG_RAW_API_BODIES` is consulted, and only its `file:<dir>`
/// directory is returned. The raw value is never echoed (a bare `1`, `0`, absent
/// or any other form yields no directory), so no secret can flow out of a
/// settings document through this projection. A file that is missing, too
/// large, unreadable or malformed is reported by state and the caller continues.
fn raw_body_directory(root: &Path) -> (Value, Value) {
    let path = root.join("settings.json");
    if guard(&path).is_err() {
        return (json!("unreadable"), Value::Null);
    }
    let meta = match fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return (json!("not_found"), Value::Null)
        }
        Err(_) => return (json!("unreadable"), Value::Null),
    };
    if !meta.is_file() || meta.nlink() != 1 || meta.uid() != unsafe { libc::geteuid() } {
        return (json!("unreadable"), Value::Null);
    }
    if meta.len() > SETTINGS_READ_BYTES {
        return (json!("too_large"), Value::Null);
    }
    let bytes = match read(&path, SETTINGS_READ_BYTES) {
        Ok(bytes) => bytes,
        Err(_) => return (json!("unreadable"), Value::Null),
    };
    let doc = match parse(&bytes) {
        Ok(doc) if doc.is_object() => doc,
        _ => return (json!("malformed"), Value::Null),
    };
    let Some(raw) = doc["env"][RAW_BODY_KEY].as_str() else {
        // Absent or non-string: the client treats an empty/default value as off.
        return (json!("observed"), Value::Null);
    };
    let directory = raw
        .strip_prefix("file:")
        .map(str::trim)
        .filter(|directory| !directory.is_empty())
        .map(PathBuf::from);
    if directory.as_ref().is_some_and(|path| !path.is_absolute()) {
        return (json!("relative_directory_unsupported"), Value::Null);
    }
    (
        json!("observed"),
        directory.map_or(Value::Null, |p| json!(p)),
    )
}

/// The finite public projection of the raw-body-record directory. The
/// directory itself is a *configured* fact from this environment's settings; it
/// is not proof that the client is currently writing there. `configured`,
/// `present` and the value's `state` are reported separately.
fn raw_body_destination(root: &Path) -> Value {
    let (state, directory) = raw_body_directory(root);
    let mut out = json!({
        "key": RAW_BODY_KEY,
        "value_echoed": false,
        "state": state,
        "directory": directory,
        "origin": "user_settings",
        "note": "只读取这一个键的 file:<dir> 目录选择；不返回设置原值、不读取正文，也不断言客户端此刻正在写入。",
    });
    if let Some(directory) = out["directory"].as_str() {
        let path = PathBuf::from(directory);
        out["configured"] = json!(true);
        out["present"] = json!(guard(&path).is_ok() && fs::symlink_metadata(&path).is_ok());
    } else {
        out["configured"] = json!(false);
        out["present"] = json!(false);
    }
    out
}

/// Observe the configured raw-body-record directory as one residue location.
/// Only the directory's own metadata is scanned; the settings value is never
/// echoed and no body file is ever opened.
fn observe_raw_body(root: &Path) -> Option<Observation> {
    let (_, directory) = raw_body_directory(root);
    let directory = directory.as_str().map(PathBuf::from)?;
    Some(observe_path(
        directory,
        root,
        "raw_body_dump",
        "residue",
        "configured",
        "settings_raw_body",
    ))
}

/// Observe one caller-supplied explicit root. It is scanned only as an opaque
/// metadata tree: its class/adapter are `explicit`/`unclassified`, because an
/// arbitrary directory may hold anything. It is never treated as work content.
fn observe_extra_root(path: &Path, root: &Path) -> Observation {
    observe_path(
        path.to_path_buf(),
        root,
        "explicit_root",
        "unclassified",
        "unclassified",
        "request_extra_root",
    )
}

/// One flat metadata row, the unit of inventory paging and filtering.
struct Row {
    location: String,
    category: String,
    class: String,
    adapter: String,
    origin: String,
    path: PathBuf,
    bytes: u64,
    mtime: i64,
    work_category: Value,
}

/// Validate `request.extra_roots` as up to [`MAX_EXTRA_ROOTS`] absolute
/// directories (no symlinks in any component, already existing). An empty,
/// relative, duplicated or over-limit list is refused rather than ignored.
fn extra_roots(r: &Value) -> Result<Vec<PathBuf>> {
    let Some(values) = r.get("extra_roots") else {
        return Ok(vec![]);
    };
    let values = values
        .as_array()
        .ok_or_else(|| err("invalid_extra_roots", "extra_roots 必须是绝对目录数组"))?;
    if values.len() > MAX_EXTRA_ROOTS {
        return Err(err(
            "invalid_extra_roots",
            "extra_roots 最多 8 个目录；请缩小显式选择范围",
        ));
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = vec![];
    for value in values {
        let raw = value
            .as_str()
            .filter(|raw| !raw.is_empty())
            .ok_or_else(|| err("invalid_extra_roots", "extra_roots 只能包含非空绝对路径"))?;
        let path = PathBuf::from(raw);
        if !path.is_absolute() {
            return Err(err("invalid_extra_roots", "extra_roots 需要绝对路径"));
        }
        guard(&path)?;
        if !seen.insert(path.clone()) {
            return Err(err("invalid_extra_roots", "extra_roots 含重复目录"));
        }
        let meta = fs::symlink_metadata(&path)
            .map_err(|_| err("invalid_extra_roots", "extra_roots 中的目录不存在或不可读"))?;
        if !meta.is_dir() {
            return Err(err("invalid_extra_roots", "extra_roots 只接受目录"));
        }
        out.push(path);
    }
    Ok(out)
}

/// The finite filter set. Every field is optional; an absent field matches
/// everything. All matching is over the metadata only.
struct Filters {
    class: Option<String>,
    category: Option<String>,
    project: Option<String>,
    /// Rows with `mtime >= since` (unix seconds) are kept.
    since: Option<i64>,
    /// Rows with `mtime <= until` are kept.
    until: Option<i64>,
    min_bytes: Option<u64>,
    max_bytes: Option<u64>,
}

fn filters(r: &Value) -> Result<Filters> {
    let string_of = |key: &str| -> Result<Option<String>> {
        match r.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => value
                .as_str()
                .map(|s| Some(s.to_string()))
                .ok_or_else(|| err("invalid_filter", &format!("{key} 必须是字符串"))),
        }
    };
    let number_of = |key: &str| -> Result<Option<i64>> {
        match r.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => value
                .as_i64()
                .map(Some)
                .ok_or_else(|| err("invalid_filter", &format!("{key} 必须是整数"))),
        }
    };
    Ok(Filters {
        class: string_of("class")?,
        category: string_of("category")?,
        project: string_of("project")?,
        since: number_of("since")?.or(number_of("modified_after")?),
        until: number_of("until")?,
        min_bytes: number_of("min_bytes")?.map(|n| n.max(0) as u64),
        max_bytes: number_of("max_bytes")?.map(|n| n.max(0) as u64),
    })
}

impl Filters {
    fn matches(&self, row: &Row) -> bool {
        if self
            .class
            .as_deref()
            .is_some_and(|class| class != row.class)
        {
            return false;
        }
        if self
            .category
            .as_deref()
            .is_some_and(|category| category != row.category)
        {
            return false;
        }
        if let Some(project) = &self.project {
            if !row.path.to_string_lossy().contains(project.as_str()) {
                return false;
            }
        }
        if self.since.is_some_and(|since| row.mtime < since) {
            return false;
        }
        if self.until.is_some_and(|until| row.mtime > until) {
            return false;
        }
        if self.min_bytes.is_some_and(|min| row.bytes < min) {
            return false;
        }
        if self.max_bytes.is_some_and(|max| row.bytes > max) {
            return false;
        }
        true
    }
}

/// Whether one location reached a depth/entry bound (its coverage is partial).
fn truncated(location: &Value) -> bool {
    location["truncated"] == true
}

/// The request-driven, paged, filterable metadata inventory.
///
/// Reuses the known-table observation, then adds two caller-driven sources: the
/// configured raw-body-record directory (`file:<dir>` in this root's settings)
/// and up to [`MAX_EXTRA_ROOTS`] explicit absolute directories. All three are
/// metadata only. The result is a bounded page (`limit <= 200`, deterministic
/// path-ordered paging) plus an independent coverage report, so a capped or
/// truncated scan is reported rather than read as a complete empty result.
pub(crate) fn inventory_request(home: &Path, root: &Path, request: &Value) -> Result<Value> {
    guard(root)?;
    match fs::symlink_metadata(root) {
        Ok(_) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(err(
                "root_missing",
                "找不到要观察的配置目录；未扫描任何位置",
            ))
        }
        Err(error) => return Err(error.into()),
    }
    let offset = match request.get("offset") {
        None | Some(Value::Null) => 0usize,
        Some(value) => value
            .as_u64()
            .map(|n| n as usize)
            .ok_or_else(|| err("invalid_offset", "offset 必须是非负整数"))?,
    };
    let limit = match request.get("limit") {
        None | Some(Value::Null) => INVENTORY_PAGE_ROWS,
        Some(value) => {
            let limit = value
                .as_u64()
                .ok_or_else(|| err("invalid_limit", "limit 必须是正整数"))?;
            if limit == 0 || limit as usize > INVENTORY_MAX_ROWS {
                return Err(err(
                    "invalid_limit",
                    "limit 需要在 1..=200 之间；一次返回量有界",
                ));
            }
            limit as usize
        }
    };
    let filter = filters(request)?;
    let roots = extra_roots(request)?;

    let mut observations: Vec<Observation> = LOCATION_TABLE
        .iter()
        .map(|(relative, category, class, adapter)| {
            observe_location_path(home, root, relative, category, class, adapter)
        })
        .collect();
    if let Some(raw_body) = observe_raw_body(root) {
        observations.push(raw_body);
    }
    for extra in &roots {
        observations.push(observe_extra_root(extra, root));
    }

    let locations: Vec<Value> = observations.iter().map(Observation::summary).collect();
    let mut rows: Vec<Row> = vec![];
    for observation in &observations {
        for (path, bytes, mtime) in &observation.rows {
            rows.push(Row {
                location: observation.path.to_string_lossy().into_owned(),
                category: observation.category.clone(),
                class: observation.class.clone(),
                adapter: observation.adapter.clone(),
                origin: observation.origin.to_string(),
                path: path.clone(),
                bytes: *bytes,
                mtime: *mtime,
                work_category: if observation.class == "work" {
                    work_category_of(root, path)
                } else {
                    Value::Null
                },
            });
        }
    }
    // Explicit roots may overlap known locations. They cannot turn a known
    // identity location into a selectable unclassified row.
    rows.retain(|row| {
        !observations.iter().any(|location| {
            location.class == "identity" && Path::new(&row.path).starts_with(&location.path)
        })
    });
    // Stable order independent of directory iteration: absolute path then
    // location, so two pages of the same snapshot never overlap or drop rows.
    rows.sort_by(|a, b| {
        let priority = |adapter: &str| match adapter {
            "known" => 0,
            "configured" => 1,
            _ => 2,
        };
        a.path
            .cmp(&b.path)
            .then_with(|| priority(&a.adapter).cmp(&priority(&b.adapter)))
            .then_with(|| a.location.cmp(&b.location))
    });
    rows.dedup_by(|a, b| a.path == b.path);
    let mut unique_files: std::collections::BTreeMap<PathBuf, u64> = rows
        .iter()
        .map(|row| (row.path.clone(), row.bytes))
        .collect();
    for location in &observations {
        if location.class == "identity" && location.file_count == 1 {
            unique_files.insert(location.path.clone(), location.total_bytes);
        }
    }
    let unique_file_count = unique_files.len();
    let unique_total_bytes = unique_files
        .values()
        .fold(0u64, |total, bytes| total.saturating_add(*bytes));
    // The flat row set is bounded independently of paging so a hostile tree
    // cannot make one request unbounded; a cut is reported as `rows_truncated`.
    let rows_truncated = rows.len() > INVENTORY_ROW_BUDGET;
    if rows_truncated {
        rows.truncate(INVENTORY_ROW_BUDGET);
    }
    let matched: Vec<&Row> = rows.iter().filter(|row| filter.matches(row)).collect();
    let total = matched.len();
    if offset > total {
        return Err(err(
            "invalid_offset",
            "offset 超过此快照的匹配行数；不会返回空页冒充完整",
        ));
    }
    let metadata:Vec<_>=matched.iter().map(|row|json!({"path":row.path,"bytes":row.bytes,"mtime":row.mtime,"category":row.category,"class":row.class,"location":row.location})).collect();
    let digest = digest(&serde_json::to_vec(&metadata)?);
    if request
        .get("expected_digest")
        .is_some_and(|expected| expected != &json!(digest))
        || (offset > 0 && request.get("expected_digest").is_none())
    {
        return Err(err(
            "stale_inventory",
            "清单在分页期间变化，或缺少原清单摘要；请刷新后重新选择",
        ));
    }
    let page: Vec<Value> = matched
        .iter()
        .skip(offset)
        .take(limit)
        .map(|row| {
            json!({
                "path": row.path,
                "location": row.location,
                "category": row.category,
                "class": row.class,
                "adapter": row.adapter,
                "origin": row.origin,
                "bytes": row.bytes,
                "mtime": row.mtime,
                "work_category": row.work_category,
            })
        })
        .collect();
    let next_offset = offset + page.len();
    let page_complete = next_offset >= total;
    // Coverage is about the *observation*, not the page: a scan that reached a
    // depth/entry/row bound stays incomplete even while more pages remain.
    let truncated_locations: Vec<Value> = locations
        .iter()
        .filter(|location| truncated(location))
        .map(|location| location["path"].clone())
        .collect();
    let scan_complete = !rows_truncated
        && truncated_locations.is_empty()
        && locations
            .iter()
            .all(|location| matches!(location["state"].as_str(), Some("present" | "not_found")));
    Ok(json!({
        "schema": "lintel.residue/1",
        "root": root,
        "checked_at": now(),
        "totals":{"file_count":unique_file_count,"total_bytes":unique_total_bytes},
        "scan_complete":scan_complete,
        "locations": locations.iter().map(|location|{let mut location=location.clone();location["entries"]=json!(page.iter().filter(|row|row["location"]==location["path"]).map(|row|json!({"path":row["path"],"bytes":row["bytes"],"mtime":row["mtime"],"work_category":row["work_category"]})).collect::<Vec<_>>());location}).collect::<Vec<_>>(),
        "rows": page,
        "offset": offset,
        "limit": limit,
        "matched_rows": total,
        "digest":digest,
        "next_offset": if page_complete { Value::Null } else { json!(next_offset) },
        "page_complete": page_complete,
        "rows_truncated": rows_truncated,
        "coverage": {
            "scan_complete": scan_complete,
            "additional_sources": {
                "raw_body": raw_body_destination(root),
                "extra_roots": roots.iter().map(|p| json!(p)).collect::<Vec<_>>(),
            },
            "truncated_locations": truncated_locations,
            "note": "覆盖范围是已知位置表 + 本次请求显式给出的目录。表外位置、其他 home 与其他用户不在覆盖内，因此不代表本地不存在其他数据；identity 位置只报存在性。",
        },
        "note": "只观察文件元数据（类型/大小/时间）并分页返回，不读取任何文件正文；raw_body 只读取 settings.json 中 OTEL_LOG_RAW_API_BODIES 的 file:<dir> 目录选择，不返回设置原值。",
    }))
}

/// What one client-state observation produced. Only [`ClientState::Observed`]
/// carries a parsed document, and even then only the two allowlisted keys are
/// ever consulted.
pub(crate) enum ClientState {
    Missing,
    TooLarge,
    Unreadable,
    Unknown,
    Observed(Value),
}

impl ClientState {
    /// The finite public state name, shared by every projection so a caller
    /// never has to match on the enum to report what was observed.
    pub(crate) fn code(&self) -> &'static str {
        match self {
            ClientState::Missing => "not_found",
            ClientState::TooLarge => "too_large",
            ClientState::Unreadable => "unreadable",
            ClientState::Unknown => "unknown",
            ClientState::Observed(_) => "observed",
        }
    }
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
pub(crate) fn read_client_state(path: &Path) -> ClientState {
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
            public["note"] = json!(format!(
                "{IDENTITY_NOTE} client_state 超过 2 MiB 上限，未读取任何内容。"
            ));
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

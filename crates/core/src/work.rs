use crate::{archive, err, now, storage::*, string, Engine, Result};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::Read,
    os::unix::ffi::OsStrExt,
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};
const MAX_BYTES: u64 = 32 * 1024 * 1024;
const MAX_FILES: usize = 10000;
/// Upper bound for one serialized inventory page. A page that cannot fit within
/// this budget is reported as explicitly incomplete rather than silently
/// truncated (dropping rows). The page size is also capped at 100 rows.
const INVENTORY_PAGE_BYTES: usize = 128 * 1024;
const INVENTORY_PAGE_ROWS: usize = 100;
/// Explicit path selection bounds. These mirror the finite transport contract
/// so the named CLI, finite SSH and the raw core all enforce the same shape.
const SELECTION_MAX_PATHS: usize = 10000;
const SELECTION_TOTAL_BYTES: usize = 512 * 1024;

/// Absolute-path guard for an explicit archive destination: parent must already
/// exist (we never create directories outside the frozen path), the path must be
/// absolute without parent-directory hops and contain no symlinks. Existing
/// content is never a valid target. Freeze the parent's device/inode so the
/// same pathname cannot select a replacement directory after approval.
pub(crate) fn freeze_output_path(path: &Path) -> Result<(PathBuf, [u64; 2])> {
    guard(path)?;
    if path.file_name().is_none() {
        return Err(err("invalid_output_path", "请给出一个完整的归档文件名"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| err("invalid_output_path", "归档路径缺少父目录"))?;
    if !parent.is_dir() {
        return Err(err(
            "invalid_output_path",
            "归档目标目录不存在；Lintel 不会替你新建目录",
        ));
    }
    guard(parent)?;
    if path.exists() {
        return Err(err(
            "output_exists",
            "归档目标已存在；请选择新文件名，不覆盖已有内容",
        ));
    }
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(err("symlink_target", "归档目标不能是符号链接"));
    }
    let metadata = fs::metadata(parent)?;
    Ok((path.to_path_buf(), [metadata.dev(), metadata.ino()]))
}

pub(crate) fn check_output_path(plan: &Value) -> Result<()> {
    if let Some(destination) = plan["extra"]["output_path"].as_str() {
        let (_, identity) = freeze_output_path(Path::new(destination))?;
        if json!(identity) != plan["extra"]["output_parent_identity"] {
            return Err(err(
                "stale_plan",
                "归档输出目录的实际对象已变化或旧计划未冻结身份，请重新预览",
            ));
        }
    }
    Ok(())
}

/// Read back an encrypted archive package from an explicit frozen path and
/// return it with the digest of the on-disk encrypted bytes. Lintel-sealed
/// archives are always single-link regular files owned by the current user, so
/// they travel between independent installs but never through a symlink.
pub(crate) fn read_package(path: &Path, pass: &str) -> Result<(Value, String)> {
    guard(path)?;
    let meta =
        fs::symlink_metadata(path).map_err(|_| err("archive_missing", "找不到该归档文件"))?;
    use std::os::unix::fs::MetadataExt;
    if !meta.is_file()
        || meta.file_type().is_symlink()
        || meta.uid() != unsafe { libc::geteuid() }
        || meta.nlink() != 1
    {
        return Err(err("archive_missing", "归档不是安全的常规文件"));
    }
    // The ciphertext envelope is slightly larger than the plaintext bound used
    // by `unseal`; allow headroom while still bounded.
    let bytes = read(path, crate::archive::MAX_PLAIN + 16 * 1024 * 1024)?;
    let package = archive::unseal(&bytes, pass)?;
    Ok((package, digest(&bytes)))
}

/// Validate a work package's file list. It is completely self-contained: every
/// entry is judged on its own path/category/digest, so an external package that
/// arrives without the original inventory or job still passes or fails on its
/// own merits. Unsupported/unapproved categories and unsafe paths are refused.
pub(crate) fn validate_package_files(package: &Value) -> Result<Vec<Value>> {
    if package["schema"] != "lintel.work/1" {
        return Err(err(
            "archive_schema",
            "只支持 Lintel 工作内容包；状态备份不能自动迁入",
        ));
    }
    let files = package["files"]
        .as_array()
        .ok_or_else(|| err("invalid_archive", "归档缺少文件清单"))?;
    if files.len() > MAX_FILES {
        return Err(err("archive_limit", "归档文件数量超过上限"));
    }
    let mut seen = HashSet::new();
    let mut total = 0usize;
    for f in files {
        let name = string(f, "path")?;
        let path = Path::new(name);
        if path.is_absolute()
            || name.contains('\\')
            || name.contains('\0')
            || path
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            || !seen.insert(path.components().collect::<PathBuf>())
        {
            return Err(err("archive_path", "归档包含重复路径或不安全路径"));
        }
        let category = string(f, "category")?;
        if !["instructions", "memory", "sessions"].contains(&category) {
            return Err(err(
                "archive_category",
                "归档含不受支持的类别；不能声明已覆盖全部所选内容",
            ));
        }
        if classify(path) != Some(category) {
            return Err(err("archive_category", "归档文件与批准的工作类别不符"));
        }
        let data: Vec<u8> = serde_json::from_value(f["data"].clone())?;
        total += data.len();
        if data.len() > 8 * 1024 * 1024 || total > MAX_BYTES as usize {
            return Err(err("archive_limit", "工作内容超过容量上限"));
        }
        if digest(&data) != f["digest"] {
            return Err(err("archive_integrity", "归档文件完整性校验失败"));
        }
    }
    Ok(files.clone())
}

pub fn categories(r: &Value) -> Result<Vec<String>> {
    let values = r["categories"]
        .as_array()
        .ok_or_else(|| err("invalid_categories", "请选择要保留的工作类别"))?;
    let mut out = vec![];
    for v in values {
        let s = v
            .as_str()
            .ok_or_else(|| err("invalid_categories", "类别格式无效"))?;
        if !["instructions", "memory", "sessions"].contains(&s) {
            return Err(err("unsupported_category", "不支持迁入该类别"));
        }
        if !out.iter().any(|x| x == s) {
            out.push(s.to_string());
        }
    }
    Ok(out)
}

/// Whether an explicit exact-path selection was requested.
///
/// `selected_paths` is additive and only meaningful for archive/preserve-style
/// plans: absent keeps the historical whole-category behavior (older raw/named
/// callers), while a present array selects the exact original files instead of
/// every file in the named categories. An empty array is never a silent
/// fallback to the whole category and is rejected by `selection`.
pub(crate) fn has_selection(r: &Value) -> bool {
    r.get("selected_paths").is_some()
}

/// The finite public description of a plan's work selection, published in place
/// of the private `extra`. Category mode carries no path list; exact mode names
/// the frozen paths so the App and CLI show exactly what was chosen.
pub(crate) fn work_selection(selected: &[String]) -> Value {
    if selected.is_empty() {
        json!({"mode": "categories"})
    } else {
        json!({"mode": "paths", "paths": selected})
    }
}

/// Rebuild the exact manifest a frozen plan was approved with. Whole-category
/// plans use `manifest`; a plan that froze `extra.selected_paths` (an additive
/// field, absent for legacy callers) re-derives the same exact selection. The
/// caller compares the result to the frozen `extra.manifest` and refuses on any
/// difference, so a changed/vanished selected path is a pre-publication stale
/// error rather than a partial archive.
pub(crate) fn frozen_manifest(root: &Path, extra: &Value) -> Result<Vec<Value>> {
    let categories = categories(extra)?;
    if has_selection(extra) {
        selected_manifest(root, &categories, &selection(extra)?)
    } else {
        manifest(root, &categories)
    }
}

/// Validate the finite structural shape of an explicit `selected_paths`
/// request and return the unique canonical relative paths in request order.
///
/// Reuses operations' canonical path syntax (including raw core requests).
/// Category and on-disk identity checks live in `selected_manifest`.
pub fn selection(r: &Value) -> Result<Vec<String>> {
    if r.get("selected_paths").is_none() {
        return Ok(vec![]);
    }
    let values = r["selected_paths"]
        .as_array()
        .ok_or_else(|| err("invalid_selection", "selected_paths 必须是路径数组"))?;
    if values.is_empty() {
        return Err(err(
            "invalid_selection",
            "empty selected_paths 是无效请求；不会退回整类选择，请给出至少一个精确原件路径",
        ));
    }
    if values.len() > SELECTION_MAX_PATHS {
        return Err(err(
            "invalid_selection",
            "selected_paths 超过 10000 项上限；请缩小精确选择范围",
        ));
    }
    let mut out: Vec<String> = Vec::with_capacity(values.len());
    if serde_json::to_vec(&r["selected_paths"])?.len() > SELECTION_TOTAL_BYTES {
        return Err(err(
            "invalid_selection",
            "selected_paths 序列化后超过 512 KiB 上限",
        ));
    }
    let mut seen = HashSet::new();
    for value in values {
        let raw = value
            .as_str()
            .ok_or_else(|| err("invalid_selection", "selected_paths 只能包含字符串路径"))?;
        let relative = valid_relative_path(raw)?;
        if !seen.insert(relative.clone()) {
            return Err(err("invalid_selection", "selected_paths 含重复路径"));
        }
        out.push(relative);
    }
    Ok(out)
}

/// Adapt the shared static path rule to a core error; never normalize a path.
fn valid_relative_path(raw: &str) -> Result<String> {
    if !lintel_operations::valid_selected_path(raw) {
        return Err(err(
            "invalid_selection",
            "selected_paths 需要有限 canonical 相对路径；不接受空白、控制字符、. / .. 或空路径段，最多 4096 UTF-8 字节",
        ));
    }
    Ok(raw.to_string())
}

/// The purpose of each selected category. Only CLAUDE.md (instructions) has a
/// position the client auto-discovers; memory/sessions stay in the reference
/// area and are never registered as active native sessions this round.
pub(crate) fn purposes(categories: &[String]) -> Value {
    let mut out = serde_json::Map::new();
    for category in ["instructions", "memory", "sessions"] {
        let selected = categories.iter().any(|c| c == category);
        let purpose = if category == "instructions" && selected {
            "instructions"
        } else if selected {
            "reference"
        } else {
            "not_selected"
        };
        out.insert(category.into(), json!(purpose));
    }
    Value::Object(out)
}

/// Activation choices per category with their legacy defaults. A protocol-1
/// raw caller that omits `activate` keeps the historical behavior (instructions
/// land at the discovered root CLAUDE.md); strict named callers state it.
pub(crate) fn activation(r: &Value, categories: &[String]) -> Value {
    let requested = r.get("activate").and_then(Value::as_object);
    let mut out = serde_json::Map::new();
    for category in ["instructions", "memory", "sessions"] {
        let selected = categories.iter().any(|c| c == category);
        let default = category == "instructions";
        let value = requested
            .and_then(|map| map.get(category))
            .and_then(Value::as_bool)
            .unwrap_or(default);
        out.insert(category.into(), json!(selected && value));
    }
    Value::Object(out)
}

/// Freeze the full planned migration for a plan that will create a new root:
/// the planned environment ID, the planned root under the state environments
/// directory, the parent directory identity and the per-file final
/// destinations. This does not create anything; execution rechecks it.
pub(crate) fn freeze_new_target(
    base: &Path,
    manifest: &[Value],
    instructions_active: bool,
) -> Result<Value> {
    let environments = base.join("environments");
    guard(&environments)?;
    let metadata = fs::metadata(&environments)
        .map_err(|_| err("invalid_state", "无法读取 environments 目录"))?;
    let new_environment_id = crate::id();
    let new_root = environments.join(&new_environment_id);
    let targets = migration_paths(manifest, instructions_active)?;
    let files = manifest
        .iter()
        .zip(targets)
        .map(|(f, relative)| {
            json!({
                "path": f["path"],
                "category": f["category"],
                "bytes": f["bytes"],
                "digest": f["digest"],
                "destination": new_root.join(&relative),
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "plan_revision": "lintel.plan/2",
        "new_environment_id": new_environment_id,
        "new_root": new_root,
        "new_root_parent_identity": [metadata.dev(), metadata.ino()],
        "import_manifest": files,
    }))
}

/// Freeze the planned migration into an already-registered existing root.
pub(crate) fn freeze_existing_target(
    root: &Path,
    manifest: &[Value],
    instructions_active: bool,
) -> Result<Value> {
    guard(root)?;
    let metadata = fs::metadata(root)?;
    let targets = migration_paths(manifest, instructions_active)?;
    let files = manifest
        .iter()
        .zip(targets)
        .map(|(f, relative)| {
            json!({
                "path": f["path"],
                "category": f["category"],
                "bytes": f["bytes"],
                "digest": f["digest"],
                "destination": root.join(&relative),
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "plan_revision": "lintel.plan/2",
        "new_environment_id": Value::Null,
        "new_root": root,
        "new_root_parent_identity": [metadata.dev(), metadata.ino()],
        "import_manifest": files,
    }))
}

/// Recheck a frozen new-root target at execution: the parent directory identity
/// must be unchanged and the planned root must still be absent. Returns an
/// explicit stale_plan for a legacy plan that did not freeze these fields.
pub(crate) fn check_frozen_target(plan: &Value) -> Result<PathBuf> {
    let extra = &plan["extra"];
    let frozen = &extra["frozen_target"];
    if frozen["plan_revision"] != "lintel.plan/2" {
        return Err(err(
            "stale_plan",
            "此创建新环境的旧计划未冻结目标身份，请在接受前重新预览",
        ));
    }
    let new_root = frozen["new_root"]
        .as_str()
        .ok_or_else(|| err("invalid_plan", "计划缺少规划的新 root"))?;
    let parent = Path::new(new_root)
        .parent()
        .ok_or_else(|| err("invalid_plan", "规划的新 root 缺少父目录"))?;
    guard(parent)?;
    let metadata = fs::metadata(parent)?;
    if json!([metadata.dev(), metadata.ino()]) != frozen["new_root_parent_identity"] {
        return Err(err("stale_plan", "规划目标的父目录对象已变化，请重新预览"));
    }
    if Path::new(new_root).exists() {
        return Err(err(
            "stale_plan",
            "规划的新 root 已被占用；不会在既有目录上继续，请重新预览",
        ));
    }
    Ok(PathBuf::from(new_root))
}
pub(crate) fn classify(relative: &Path) -> Option<&'static str> {
    lintel_operations::work_path_category(relative)
}

/// Map one selected batch into the inactive work area. Reserve every original
/// name first, then disambiguate colliding file/ancestor names without changing suffixes
/// or wrapping already imported paths. Preserve and portable import share this.
///
/// `instructions_active` selects whether the `instructions` category lands at
/// the discovered root instruction position (the legacy default) or is kept,
/// like memory/sessions, in the `lintel-imports` reference area. The same flag
/// drives the frozen plan, the published manifest and the actual execution, so a
/// plan and its receipt can never disagree about where CLAUDE.md goes.
pub(crate) fn migration_paths(files: &[Value], instructions_active: bool) -> Result<Vec<PathBuf>> {
    let preferred: Vec<PathBuf> = files
        .iter()
        .map(|file| {
            let relative = Path::new(string(file, "path")?);
            Ok(
                if (file["category"] == "instructions" && instructions_active)
                    || relative.starts_with("lintel-imports")
                {
                    relative.to_path_buf()
                } else {
                    Path::new("lintel-imports").join(relative)
                },
            )
        })
        .collect::<Result<_>>()?;
    let reserved: HashSet<_> = preferred.iter().cloned().collect();
    let directories: HashSet<_> = preferred
        .iter()
        .flat_map(|path| path.ancestors().skip(1))
        .map(Path::to_path_buf)
        .collect();
    let mut used = HashSet::new();
    let mut targets = Vec::with_capacity(files.len());
    for path in preferred {
        let mut target = path.clone();
        if used.contains(&target) || directories.contains(&target) {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| err("invalid_path", "迁入文件名无效"))?;
            let mut index = 1;
            loop {
                target.set_file_name(format!("lintel-{index}-{name}"));
                if !reserved.contains(&target)
                    && !directories.contains(&target)
                    && !used.contains(&target)
                {
                    break;
                }
                index += 1;
            }
        }
        used.insert(target.clone());
        targets.push(target);
    }
    Ok(targets)
}

/// Missing work directories are private at creation. Existing destination
/// directories belong to the approved environment and keep their permissions.
pub(crate) fn migration_parent(path: &Path) -> Result<()> {
    guard(path)?;
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)?;
    if fs::metadata(path)?.uid() != unsafe { libc::geteuid() } {
        return Err(err("wrong_owner", "迁入目录不属于当前用户"));
    }
    Ok(())
}

fn migration_access(path: &Path, mode: libc::c_int) -> Result<()> {
    let path = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| err("invalid_path", "迁入目录路径含无效字符"))?;
    if unsafe { libc::faccessat(libc::AT_FDCWD, path.as_ptr(), mode, libc::AT_EACCESS) } != 0 {
        return Err(err(
            "migration_destination_unwritable",
            "迁入目录不可写入或访问；请核对权限后重新预览，已有权限不会自动改变",
        ));
    }
    Ok(())
}

/// Inspect actual ancestors inside the approved root before any content write.
/// Missing parents require write access only to their nearest existing parent;
/// existing ancestors keep their modes and need search access, not blanket chmod.
pub(crate) fn preflight_import_parent(root: &Path, target: &Path) -> Result<()> {
    guard(target)?;
    let relative = target
        .parent()
        .unwrap()
        .strip_prefix(root)
        .map_err(|_| err("invalid_path", "迁入目标不在批准的配置目录内"))?;
    let mut directory = root.to_path_buf();
    let mut last_existing = root.to_path_buf();
    for component in std::iter::once(None).chain(relative.components().map(Some)) {
        if let Some(component) = component {
            directory.push(component);
        }
        match fs::metadata(&directory) {
            Ok(metadata) => {
                if metadata.uid() != unsafe { libc::geteuid() } {
                    return Err(err("wrong_owner", "迁入目录不属于当前用户"));
                }
                migration_access(&directory, libc::X_OK)?;
                last_existing = directory.clone();
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(_) => return Err(err("path_unreadable", "无法检查迁入目录")),
        }
    }
    migration_access(&last_existing, libc::W_OK | libc::X_OK)
}

/// Check this batch against the actual destination filesystem's name rules.
/// Runs only inside approved execution. The private scratch tree contains zero
/// byte placeholders, never archived content; cleanup removes only names we
/// created, and leaves any unexpected extra entry intact.
pub(crate) fn preflight_migration_paths(
    root: &Path,
    paths: &[PathBuf],
    j: &mut Value,
    journal: &Path,
) -> Result<()> {
    struct Probe {
        files: Vec<PathBuf>,
        directories: Vec<PathBuf>,
    }
    impl Drop for Probe {
        fn drop(&mut self) {
            for file in self.files.iter().rev() {
                let _ = fs::remove_file(file);
            }
            for directory in self.directories.iter().rev() {
                let _ = fs::remove_dir(directory);
            }
        }
    }
    let scratch = record_migration_probe(root, j, journal)?;
    let result = (|| {
        fs::create_dir(&scratch)?;
        let mut probe = Probe {
            files: vec![],
            directories: vec![scratch.clone()],
        };
        private_dir(&scratch)?;
        let conflict = || {
            err("migration_path_conflict", "目标文件系统将所选路径视为同名或文件／目录冲突；未新建环境或迁入正文，请整理源内容后重新归档，或使用能区分这些路径的文件系统目标")
        };
        for relative in paths {
            let mut parent = scratch.clone();
            for component in relative.parent().unwrap_or(Path::new("")).components() {
                parent.push(component);
                match fs::create_dir(&parent) {
                    Ok(()) => {
                        probe.directories.push(parent.clone());
                        private_dir(&parent)?;
                    }
                    Err(error)
                        if error.kind() == std::io::ErrorKind::AlreadyExists && parent.is_dir() => {
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::NotADirectory
                        ) =>
                    {
                        return Err(conflict())
                    }
                    Err(error) => return Err(error.into()),
                }
            }
            let target = scratch.join(relative);
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&target)
            {
                Ok(_) => probe.files.push(target),
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::NotADirectory
                    ) =>
                {
                    return Err(conflict())
                }
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    })();
    let removed = match fs::symlink_metadata(&scratch) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        _ => false,
    };
    j["migration_probe"]["status"] = json!(if removed { "removed" } else { "retained" });
    if removed {
        let status = if result.is_ok() {
            "completed"
        } else {
            "failed"
        };
        *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":"migration_preflight","label":"检查目标文件系统路径","status":status,"message":"临时空文件检查目录已清理；此步骤没有迁入归档正文。"});
    }
    save(journal, j)?;
    if !removed {
        return Err(err("migration_probe_retained", "路径检查目录未确认清理；按原任务的 migration_probe.path 核对，未迁入正文，不自动重发或删除额外内容"));
    }
    result
}

fn record_migration_probe(root: &Path, j: &mut Value, journal: &Path) -> Result<PathBuf> {
    let scratch = root.join(format!(".lintel-path-check-{}", uuid::Uuid::new_v4()));
    guard(&scratch)?;
    j["migration_probe"] = json!({"path":scratch,"status":"executing"});
    j["steps"].as_array_mut().unwrap().push(json!({"id":"migration_preflight","label":"检查目标文件系统路径","status":"executing","message":"正在以临时空文件检查路径；中断后按本任务记录核对检查目录。"}));
    save(journal, j)?;
    Ok(scratch)
}

const SCAN_ENTRIES: usize = 50000;
const SCAN_TIME: Duration = Duration::from_secs(30);
const FILE_LIMIT_BYTES: u64 = 8 * 1024 * 1024;
const BLOCKER_LIMIT: usize = 200;

/// One metadata-only walk of the selected work scope. It shares the exact
/// traversal scope, entry/time budget, non-regular-file refusal and symlink
/// boundary that `manifest` (content admission) and `summaries` (inspector
/// overview) already use, so the three follow the same rules about which files
/// are content and which directory is walked:
///   * only descendants of `projects` and `lintel-imports` are descended into,
///   * a non-regular entry is never opened or admitted as content,
///   * a symlink anywhere in the selected scope is never followed,
///   * the entry/time budget bounds every walk.
///
/// The three are still independent snapshots: each is taken at its own time, so
/// they can observe a different set of files if content changes in between. A
/// preflight result is provisional metadata admission, never a substitute for
/// the full `manifest` content + digest check that a real plan runs.
///
/// The walker never reads file contents, hashes, credentials or invokes Claude;
/// it records only per-file metadata for paths `classify` accepts. It never
/// opens an entry, so a FIFO/socket/device is observed by metadata only and is
/// recorded as non-regular rather than counted, admitted or read.
struct Scan {
    /// Regular files whose relative path `classify` accepts: (category, rel, len).
    files: Vec<(&'static str, PathBuf, u64)>,
    /// `classify`-accepted entries that are not regular files (FIFO, socket,
    /// device, ...). The real `manifest` refuses these through `read()`, so a
    /// preflight must never admit them as countable content. Never opened.
    nonregular: Vec<(&'static str, PathBuf)>,
    complete: bool,
    reason: Option<&'static str>,
    /// Bounded relative path of the first entry/directory the walk could not
    /// observe (unreadable dir/entry/metadata, budget cut, symlink in scope).
    /// Root-level failures use `.`. Bounded by the walk's own path depth.
    unobserved: Option<PathBuf>,
}

fn scan(root: &Path) -> Result<Scan> {
    guard(root)?;
    let start = Instant::now();
    let mut stack = vec![root.to_path_buf()];
    let mut files = vec![];
    let mut nonregular = vec![];
    let mut complete = true;
    let mut reason: Option<&'static str> = None;
    let mut unobserved: Option<PathBuf> = None;
    let mut entries = 0usize;
    // Record the first unobserved source once; later ones keep the first reason.
    let note = |reason_code: &'static str,
                path: Option<PathBuf>,
                reason: &mut Option<&'static str>,
                unobserved: &mut Option<PathBuf>| {
        if reason.is_none() {
            *reason = Some(reason_code);
        }
        if unobserved.is_none() {
            *unobserved = Some(
                path.filter(|p| !p.as_os_str().is_empty())
                    .unwrap_or_else(|| PathBuf::from(".")),
            );
        }
    };
    'walk: while let Some(dir) = stack.pop() {
        let items = match fs::read_dir(&dir) {
            Ok(items) => items,
            Err(_) => {
                complete = false;
                let rel = dir.strip_prefix(root).unwrap_or(Path::new(""));
                let rel = if rel.as_os_str().is_empty() {
                    PathBuf::from(".")
                } else {
                    rel.to_path_buf()
                };
                note(
                    "unreadable_directory",
                    Some(rel),
                    &mut reason,
                    &mut unobserved,
                );
                continue;
            }
        };
        for item in items {
            entries += 1;
            if entries > SCAN_ENTRIES || start.elapsed() > SCAN_TIME {
                complete = false;
                note(
                    "scan_budget_exceeded",
                    dir.strip_prefix(root).ok().map(Path::to_path_buf),
                    &mut reason,
                    &mut unobserved,
                );
                break 'walk;
            }
            let item = match item {
                Ok(item) => item,
                Err(_) => {
                    complete = false;
                    note(
                        "unreadable_entry",
                        dir.strip_prefix(root).ok().map(Path::to_path_buf),
                        &mut reason,
                        &mut unobserved,
                    );
                    continue;
                }
            };
            let p = item.path();
            let rel = match p.strip_prefix(root) {
                Ok(rel) => rel,
                Err(_) => {
                    complete = false;
                    note("path_escape", None, &mut reason, &mut unobserved);
                    continue;
                }
            };
            let m = match fs::symlink_metadata(&p) {
                Ok(m) => m,
                Err(_) => {
                    complete = false;
                    note(
                        "unreadable_metadata",
                        Some(rel.to_path_buf()),
                        &mut reason,
                        &mut unobserved,
                    );
                    continue;
                }
            };
            if m.file_type().is_symlink() {
                // Never followed. A symlink inside the selected scope is
                // explicitly uncovered, not silently skipped as empty.
                if rel.starts_with("projects")
                    || rel.starts_with("lintel-imports")
                    || rel == Path::new("CLAUDE.md")
                {
                    complete = false;
                    note(
                        "symlink_in_scope",
                        Some(rel.to_path_buf()),
                        &mut reason,
                        &mut unobserved,
                    );
                }
                continue;
            }
            if m.is_dir() {
                if rel.starts_with("projects") || rel.starts_with("lintel-imports") {
                    stack.push(p)
                }
                continue;
            }
            if let Some(category) = classify(rel) {
                if m.is_file() {
                    files.push((category, rel.to_path_buf(), m.len()));
                } else {
                    // A FIFO/socket/device/etc.: never opened, never counted.
                    // `manifest` refuses it, so preflight must not admit it.
                    nonregular.push((category, rel.to_path_buf()));
                    complete = false;
                    note(
                        "nonregular_entry",
                        Some(rel.to_path_buf()),
                        &mut reason,
                        &mut unobserved,
                    );
                }
            }
        }
    }
    Ok(Scan {
        files,
        nonregular,
        complete,
        reason,
        unobserved,
    })
}

/// Read-only capacity preflight for one explicit selection. Metadata only: it
/// reads no file contents, computes no digests, invokes no Claude and touches no
/// credential. It never fails on an incomplete scan; instead it reports the
/// first uncovered source and sets `complete:false` so a truncated result can never be
/// read as an admitted one. A scan that cannot complete is *not* an empty
/// directory: unreadable/over-budget coverage remains explicit.
///
/// `eligible` is a *provisional metadata-level* admission signal only. The scan
/// and the real `manifest` are separate snapshots taken at different times, so
/// this report cannot guarantee the archive will agree: a file can grow, appear
/// or vanish in between. A real archive/preserve plan still runs the full
/// `manifest` content + digest check before approval, and only that check
/// admits or refuses the selection.
pub fn preflight(
    environment_id: &str,
    root: &Path,
    categories: &[String],
    r: &Value,
) -> Result<Value> {
    if has_selection(r) {
        return selected_preflight(environment_id, root, categories, r);
    }
    let scan = scan(root)?;
    let selected = |category: &str| categories.iter().any(|c| c == category);
    let roots = ["instructions", "memory", "sessions"];
    let mut per: std::collections::BTreeMap<&'static str, (u64, u64)> = Default::default();
    let mut totals = (0u64, 0u64);
    let mut blockers: Vec<Value> = vec![];
    let mut blocker_count = 0usize;
    let mut push_blocker = |code: &str, path: Option<&Path>, message: String| {
        blocker_count += 1;
        if blockers.len() < BLOCKER_LIMIT {
            blockers.push(json!({
                "code": code,
                "path": path.map(|p| p.to_string_lossy().into_owned()),
                "message": message,
            }));
        }
    };
    // Per-category counts are always reported for every category (selected or
    // not) so callers can see the full picture; the running total is only the
    // selected subset.
    for entry in &scan.files {
        let stat = per.entry(entry.0).or_default();
        stat.0 += 1;
        stat.1 += entry.2;
        if selected(entry.0) {
            totals.0 += 1;
            totals.1 += entry.2;
            if entry.2 > FILE_LIMIT_BYTES {
                push_blocker(
                    "file_too_large",
                    Some(&entry.1),
                    format!(
                        "单个文件 {} 字节，超过 {} 字节上限。可取消整个类别、或改用 --path 精确选择其余原件后重新检查。原件不受影响。",
                        entry.2, FILE_LIMIT_BYTES
                    ),
                );
            }
        }
    }
    // A non-regular selected entry (FIFO/socket/device) can never be read as
    // content: `manifest` refuses it. Never opened here either.
    for (category, path) in &scan.nonregular {
        if selected(category) {
            push_blocker(
                "nonregular_entry",
                Some(path),
                "所选范围包含非普通文件（如 FIFO、socket 或设备）；归档会拒绝读取它，预检没有打开或计入。请核对该入口后重新检查。".to_string(),
            );
        }
    }
    // Aggregate limits are only meaningful when the scan actually finished; an
    // incomplete scan cannot prove the selection fits.
    if scan.complete {
        if totals.1 > MAX_BYTES {
            push_blocker(
                "total_bytes_exceeded",
                None,
                format!(
                    "所选合计 {} 字节，超过 {} 字节上限；未截断，可取消整个类别或改用 --path 精确选择后重新检查",
                    totals.1, MAX_BYTES
                ),
            );
        }
        if totals.0 > MAX_FILES as u64 {
            push_blocker(
                "file_count_exceeded",
                None,
                format!(
                    "所选合计 {} 个文件，超过 {} 个上限；未截断，可取消整个类别后重新检查",
                    totals.0, MAX_FILES
                ),
            );
        }
    } else {
        push_blocker(
            "scan_incomplete",
            scan.unobserved.as_deref(),
            format!(
                "扫描未完成（{}）；已统计部分不代表完整目录，未覆盖范围必须重新检查后才能作为容量结论",
                scan.reason.unwrap_or("unknown")
            ),
        );
    }
    // `entries` budget is shared across the whole scan; expose the same bound
    // the walker enforces so callers can explain an incomplete result.
    let eligible = scan.complete && blocker_count == 0;
    Ok(json!({
        "environment_id": environment_id,
        "root": root.to_string_lossy(),
        "checked_at": now(),
        "complete": scan.complete,
        "eligible": eligible,
        "work_selection": {"mode": "categories"},
        "totals": {"files": totals.0, "bytes": totals.1},
        "limits": {
            "file_bytes": FILE_LIMIT_BYTES,
            "total_bytes": MAX_BYTES,
            "files": MAX_FILES,
            "entries": SCAN_ENTRIES,
        },
        "categories": roots.iter().map(|c| {
            let (count, bytes) = per.get(c).copied().unwrap_or((0, 0));
            json!({"category": c, "count": count, "bytes": bytes})
        }).collect::<Vec<_>>(),
        "blockers": blockers,
        "blockers_truncated": blocker_count > BLOCKER_LIMIT,
    }))
}

/// Metadata-only preflight for an exact `selected_paths` selection. It inspects
/// only the chosen original files' metadata; it never walks or reads the
/// unselected neighbours, so an unrelated oversized session or FIFO elsewhere
/// in the same category cannot block an exact selection. Root/parent symlink
/// guards from `guard` still apply. A missing selected path is an explicit
/// failure (it is not an empty directory and not an eligible selection).
fn selected_preflight(
    environment_id: &str,
    root: &Path,
    categories: &[String],
    r: &Value,
) -> Result<Value> {
    guard(root)?;
    let selected = selection(r)?;
    let mut files = vec![];
    let mut totals = (0u64, 0u64);
    let mut blockers: Vec<Value> = vec![];
    let mut blocker_count = 0usize;
    let mut push_blocker = |code: &str, path: &str, message: String| {
        blocker_count += 1;
        if blockers.len() < BLOCKER_LIMIT {
            blockers.push(json!({"code": code, "path": path, "message": message}));
        }
    };
    for raw in &selected {
        let rel = Path::new(raw);
        let category = classify(rel).ok_or_else(|| {
            err(
                "invalid_selection",
                &format!("精确路径不属于受支持的工作类别: {raw}"),
            )
        })?;
        if !categories.iter().any(|c| c == category) {
            return Err(err(
                "invalid_selection",
                &format!("精确路径 {raw} 的类别 {category} 不在显式选择的 categories 内"),
            ));
        }
        let path = root.join(rel);
        guard(&path)?;
        let meta = fs::symlink_metadata(&path).map_err(|_| {
            err(
                "selected_missing",
                &format!("所选原件不存在或不可读: {raw}"),
            )
        })?;
        if meta.file_type().is_symlink() || !meta.is_file() {
            return Err(err(
                "invalid_selection",
                &format!("所选路径不是常规文件（符号链接/特殊文件）: {raw}"),
            ));
        }
        let bytes = meta.len();
        totals.0 += 1;
        totals.1 += bytes;
        if bytes > FILE_LIMIT_BYTES {
            push_blocker(
                "file_too_large",
                raw,
                format!(
                    "所选单个文件 {} 字节，超过 {} 字节上限。取消对该路径的选择即可完整保全其余原件；原件不受影响。",
                    bytes, FILE_LIMIT_BYTES
                ),
            );
        }
        files.push(json!({"path": raw, "category": category, "bytes": bytes}));
    }
    if totals.1 > MAX_BYTES {
        push_blocker(
            "total_bytes_exceeded",
            "",
            format!(
                "精确选择合计 {} 字节，超过 {} 字节上限；未截断，请缩小精确选择",
                totals.1, MAX_BYTES
            ),
        );
    }
    if totals.0 > MAX_FILES as u64 {
        push_blocker(
            "file_count_exceeded",
            "",
            format!(
                "精确选择合计 {} 个文件，超过 {} 个上限；请缩小精确选择",
                totals.0, MAX_FILES
            ),
        );
    }
    files.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    let eligible = blocker_count == 0;
    let category_rows: Vec<Value> = ["instructions", "memory", "sessions"]
        .iter()
        .map(|c| {
            let count = files.iter().filter(|f| f["category"] == *c).count() as u64;
            let bytes: u64 = files
                .iter()
                .filter(|f| f["category"] == *c)
                .map(|f| f["bytes"].as_u64().unwrap_or(0))
                .sum();
            json!({"category": c, "count": count, "bytes": bytes})
        })
        .collect();
    Ok(json!({
        "environment_id": environment_id,
        "root": root.to_string_lossy(),
        "checked_at": now(),
        "complete": true,
        "eligible": eligible,
        "work_selection": {"mode": "paths", "paths": selected},
        "totals": {"files": totals.0, "bytes": totals.1},
        "limits": {
            "file_bytes": FILE_LIMIT_BYTES,
            "total_bytes": MAX_BYTES,
            "files": MAX_FILES,
            "entries": SCAN_ENTRIES,
        },
        "categories": category_rows,
        "blockers": blockers,
        "blockers_truncated": blocker_count > BLOCKER_LIMIT,
    }))
}

pub fn manifest(root: &Path, categories: &[String]) -> Result<Vec<Value>> {
    guard(root)?;
    let start = Instant::now();
    let mut stack = vec![root.to_path_buf()];
    let mut out = vec![];
    let mut bytes = 0;
    let mut entries = 0;
    while let Some(dir) = stack.pop() {
        for item in fs::read_dir(&dir)? {
            let item = item?;
            entries += 1;
            if entries > 50000 || start.elapsed() > Duration::from_secs(30) {
                return Err(err(
                    "scan_incomplete",
                    "工作目录扫描超过上限；没有生成部分清理计划",
                ));
            }
            let p = item.path();
            let rel = p
                .strip_prefix(root)
                .map_err(|_| err("path_escape", "路径离开了选定目录"))?;
            let m = fs::symlink_metadata(&p)?;
            if m.file_type().is_symlink() {
                if rel.starts_with("projects")
                    || rel.starts_with("lintel-imports")
                    || rel == Path::new("CLAUDE.md")
                {
                    return Err(err(
                        "symlink_target",
                        "工作内容包含符号链接，需要先明确其实际目标",
                    ));
                }
                continue;
            }
            if m.is_dir() {
                if rel.starts_with("projects") || rel.starts_with("lintel-imports") {
                    stack.push(p)
                }
                continue;
            }
            if let Some(category) = classify(rel) {
                if categories.iter().any(|c| c == category) {
                    let data = read(&p, 8 * 1024 * 1024)?;
                    bytes += data.len() as u64;
                    if bytes > MAX_BYTES || out.len() >= MAX_FILES {
                        return Err(err(
                            "archive_limit",
                            "选定工作内容超过本候选的 32 MiB / 10000 文件上限；未截断或删除",
                        ));
                    }
                    out.push(json!({"path":rel,"category":category,"bytes":data.len(),"digest":digest(&data)}));
                }
            }
        }
    }
    out.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    Ok(out)
}

/// Build the exact-selection content manifest for a frozen plan.
///
/// Unlike the whole-category `manifest` it never reads unrelated files: for
/// every selected relative path it independently re-derives the category from
/// the path (never trusting a caller-supplied category), verifies the entry is
/// a regular file inside the frozen root, reads and digests only the selected
/// bytes, and refuses absolute escapes, symlinks, non-regular files and
/// unselected oversized neighbours. It applies the same 8 MiB/32 MiB/10000-file
/// limits but excludes every unselected oversized session, exactly as intended.
///
/// A missing/changed selected path is an explicit failure (`selected_missing`/
/// `selected_changed`), never read as an empty or eligible selection.
fn source_identity(metadata: &fs::Metadata) -> Value {
    json!({"device":metadata.dev(),"inode":metadata.ino(),"owner":metadata.uid(),"ctime":metadata.ctime(),"ctime_nsec":metadata.ctime_nsec()})
}

/// Read the chosen original through one non-following, nonblocking handle and
/// bind its path and handle identity before/after reading. ctime distinguishes
/// immediate inode reuse. This is observation, not OS CAS against external writers.
fn read_selected_original(path: &Path) -> Result<(Vec<u8>, Value)> {
    guard(path)?;
    let before = fs::symlink_metadata(path)?;
    if !before.is_file() || before.len() > FILE_LIMIT_BYTES {
        return Err(err("selected_changed", "所选原件不是普通文件或已超限"));
    }
    let identity = source_identity(&before);
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    if !file.metadata()?.is_file() || source_identity(&file.metadata()?) != identity {
        return Err(err("selected_changed", "所选原件在读取前被替换"));
    }
    let mut data = Vec::new();
    (&mut file)
        .take(FILE_LIMIT_BYTES + 1)
        .read_to_end(&mut data)?;
    guard(path)?;
    if data.len() as u64 > FILE_LIMIT_BYTES
        || source_identity(&file.metadata()?) != identity
        || source_identity(&fs::symlink_metadata(path)?) != identity
    {
        return Err(err("selected_changed", "所选原件在读取期间被替换或修改"));
    }
    Ok((data, identity))
}

pub fn selected_manifest(
    root: &Path,
    categories: &[String],
    selected: &[String],
) -> Result<Vec<Value>> {
    guard(root)?;
    if selected.is_empty() {
        return Err(err(
            "invalid_selection",
            "empty selected_paths 是无效请求；不会退回整类选择",
        ));
    }
    let mut out = vec![];
    let mut bytes = 0u64;
    for raw in selected {
        let relative = valid_relative_path(raw)?;
        let rel = Path::new(&relative);
        let category = classify(rel).ok_or_else(|| {
            err(
                "invalid_selection",
                &format!("精确路径不属于受支持的工作类别: {relative}"),
            )
        })?;
        if !categories.iter().any(|c| c == category) {
            return Err(err(
                "invalid_selection",
                &format!("精确路径 {relative} 的类别 {category} 不在显式选择的 categories 内"),
            ));
        }
        // Refuse special files before the bounded nonblocking read. Unselected
        // neighbours are never opened or included in this exact manifest.
        let path = root.join(rel);
        guard(
            path.parent()
                .ok_or_else(|| err("invalid_selection", "所选原件缺少父路径"))?,
        )?;
        let meta = fs::symlink_metadata(&path).map_err(|_| {
            err(
                "selected_missing",
                &format!("所选原件不存在或不可读: {relative}"),
            )
        })?;
        if meta.file_type().is_symlink() || !meta.is_file() {
            return Err(err(
                "invalid_selection",
                &format!("所选路径不是常规文件（符号链接/特殊文件）: {relative}"),
            ));
        }
        let (data, identity) = read_selected_original(&path)?;
        bytes += data.len() as u64;
        if bytes > MAX_BYTES || out.len() >= MAX_FILES {
            return Err(err(
                "archive_limit",
                "精确选择的内容超过本候选的 32 MiB / 10000 文件上限；未截断或删除",
            ));
        }
        out.push(json!({
            "path": relative,
            "category": category,
            "bytes": data.len(),
            "digest": digest(&data),
            "source_identity":identity,
        }));
    }
    out.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    Ok(out)
}

/// Metadata inventory for the finite read-only `work_inventory` operation.
///
/// Shares the exact traversal scope, budgets, symlink/non-regular refusal and
/// classification of `scan`, so it reports the same file set a real plan would
/// select. It reads metadata only: no content/content digest, Claude or credential,
/// and it writes no state. The scan is performed once per request; paging is a
/// deterministic slice of that single complete snapshot keyed by `offset`.
pub fn inventory(
    environment_id: &str,
    root: &Path,
    categories: &[String],
    r: &Value,
) -> Result<Value> {
    let expected = r
        .get("expected_digest")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let offset = match r.get("offset") {
        None | Some(Value::Null) => 0usize,
        Some(v) => v
            .as_u64()
            .map(|n| n as usize)
            .ok_or_else(|| err("invalid_offset", "offset 必须是非负整数"))?,
    };
    if r.get("expected_digest").is_some()
        && !r["expected_digest"]
            .as_str()
            .is_some_and(lintel_operations::valid_plan_hash)
    {
        return Err(err(
            "invalid_inventory",
            "expected_digest 需要原清单的 64 位小写摘要",
        ));
    }
    if offset > 9007199254740991usize || (offset > 0 && expected.is_none()) {
        return Err(err(
            "invalid_inventory",
            "后续 metadata 页需要原清单摘要和有限 offset",
        ));
    }
    let scan = scan(root)?;
    // Grouped by full relative path (UTF-8 byte order), categories filtered to
    // the explicit selection. A path that cannot be expressed as UTF-8 already
    // makes the whole scan incomplete below.
    let selected = |category: &str| categories.iter().any(|c| c == category);
    let mut rows: Vec<(&'static str, String, u64)> = vec![];
    let mut total_files = 0u64;
    let mut utf8_error = false;
    for (category, rel, bytes) in &scan.files {
        if !selected(category) {
            continue;
        }
        total_files += 1;
        match rel.to_str() {
            Some(text) if lintel_operations::valid_selected_path(text) => {
                rows.push((category, text.to_string(), *bytes))
            }
            Some(_) => utf8_error = true,
            None => utf8_error = true,
        }
    }
    rows.sort_by(|a, b| a.1.as_bytes().cmp(b.1.as_bytes()));
    // The digest binds the complete metadata inventory of this scan. It is
    // stable across paging of the same snapshot and changes when the observed
    // file set/order/sizes change.
    let digests: Vec<Value> = rows
        .iter()
        .map(|(category, path, bytes)| json!({"path":path,"category":category,"bytes":bytes}))
        .collect();
    let digest_value = digest(&serde_json::to_vec(&digests)?);
    // A stale expected_digest means a previous page came from another snapshot;
    // refuse to stitch two observations together.
    if let Some(expected) = &expected {
        if expected != &digest_value {
            return Err(err(
                "stale_inventory",
                "目录在分页之间发生变化；不会拼接两次快照，请从 offset 0 重新读取",
            ));
        }
    }
    if offset > rows.len() {
        return Err(err(
            "invalid_offset",
            "offset 超过此快照的文件数量；不会返回空页冒充完整",
        ));
    }

    // Build one bounded page. The page keeps whole rows up to the byte budget;
    // if a single row cannot be shown the caller is told the result is
    // incomplete instead of dropping rows silently.
    let mut files: Vec<Value> = vec![];
    // Reserve the bounded root/envelope/reason fields too, not only row bytes.
    let mut used = serde_json::to_vec(&json!({"environment_id":environment_id,"root":root,"unobserved":scan.unobserved.as_deref().map(|path|path.to_string_lossy()),"reason":scan.reason}))?.len()+1024;
    let mut index = offset;
    let mut row_too_large = false;
    while index < rows.len() && files.len() < INVENTORY_PAGE_ROWS {
        let (category, path, bytes) = &rows[index];
        let row = json!({"path":path,"category":category,"bytes":bytes});
        let size = serde_json::to_vec(&row)?.len() + 1;
        if used + size > INVENTORY_PAGE_BYTES {
            if files.is_empty() {
                row_too_large = true;
            }
            break;
        }
        used += size;
        files.push(row);
        index += 1;
    }
    let next_offset = index;
    let page_complete = next_offset >= rows.len();
    let scan_complete = scan.complete && !utf8_error && !row_too_large;
    let complete = scan_complete;
    let reason = if row_too_large {
        Some("page_row_too_large")
    } else if utf8_error {
        Some("path_not_selectable")
    } else if !scan.complete {
        scan.reason
    } else {
        None
    };
    let unobserved = if utf8_error {
        Some("<unselectable-path>".to_string())
    } else {
        scan.unobserved
            .as_deref()
            .map(|p| p.to_string_lossy().into_owned())
    };
    Ok(json!({
        "environment_id": environment_id,
        "root": root.to_string_lossy(),
        "complete": complete,
        "digest": digest_value,
        "files": files,
        "total_files": total_files,
        "next_offset": if page_complete || !scan_complete { Value::Null } else { json!(next_offset) },
        "reason": reason,
        "unobserved": unobserved,
    }))
}
/// Overview statistics for the inspector. This walk reads metadata only: no
/// file contents, no digests, and none of the archive admission limits (those
/// stay on `manifest`, which destructive plans must still pass in full). When
/// the scan budget runs out or entries cannot be read, the result is marked
/// `complete: false` instead of failing the unrelated settings inspection.
pub fn summaries(root: &Path) -> Result<Vec<Value>> {
    let scan = scan(root)?;
    let mut stats: std::collections::BTreeMap<&'static str, (u64, u64)> = Default::default();
    for (category, _, bytes) in &scan.files {
        let stat = stats.entry(category).or_default();
        stat.0 += 1;
        stat.1 += bytes;
    }
    Ok(["instructions", "memory", "sessions"]
        .iter()
        .map(|c| {
            let (count, bytes) = stats.get(c).copied().unwrap_or((0, 0));
            json!({"category":c,"count":count,"bytes":bytes,"complete":scan.complete})
        })
        .collect())
}
pub fn check_passphrase(r: &Value) -> Result<&str> {
    let s = string(r, "archive_passphrase")?;
    if s.chars().count() < 12 {
        return Err(err(
            "passphrase_required",
            "归档口令至少 12 个字符；仅用于本次加密，不保存到任务记录",
        ));
    }
    Ok(s)
}
impl Engine {
    /// Build the encrypted work package for a selection and write it to `dest`.
    /// The caller has already frozen `dest` (an explicit output path or the
    /// plan's private state path). Returns the on-disk archive path and digest.
    pub(crate) fn write_archive(
        &self,
        e: &Value,
        p: &Value,
        r: &Value,
        dest: &Path,
        j: &mut Value,
        journal: &Path,
    ) -> Result<(PathBuf, Vec<Value>)> {
        let pass = check_passphrase(r)?;
        let root = PathBuf::from(string(e, "root")?);
        let entries = frozen_manifest(&root, &p["extra"])?;
        if json!(entries) != p["extra"]["manifest"] {
            return Err(err(
                "stale_plan",
                "所选工作内容在执行前改变（选中原件增加、消失或内容变化），请重新生成计划",
            ));
        }
        let size: u64 = entries
            .iter()
            .map(|v| v["bytes"].as_u64().unwrap_or(0))
            .sum();
        let destination_dir = dest
            .parent()
            .ok_or_else(|| err("invalid_output_path", "归档路径缺少父目录"))?;
        if fs2::available_space(destination_dir)? < size * 6 + 1024 * 1024 {
            return Err(err(
                "insufficient_space",
                "归档目标所在存储空间不足；原始内容尚未删除",
            ));
        }
        let mut files = vec![];
        for entry in &entries {
            let path = root.join(string(entry, "path")?);
            let data = if entry.get("source_identity").is_some() {
                let (data, identity) = read_selected_original(&path)?;
                if identity != entry["source_identity"] {
                    return Err(err("stale_plan", "归档期间所选原件身份改变"));
                }
                data
            } else {
                read(&path, 8 * 1024 * 1024)?
            };
            if digest(&data) != entry["digest"] {
                return Err(err("stale_plan", "归档期间工作内容变化；原环境保持不变"));
            }
            files.push(json!({"path":entry["path"],"category":entry["category"],"digest":entry["digest"],"data":data}));
        }
        let package = json!({"schema":"lintel.work/1","generator":"Lintel","created_at":now(),"files":files,"notes":"Selected working content only; runtime credentials and executable config excluded."});
        let encrypted = archive::seal(&package, pass)?;
        // Recheck the frozen destination, then atomically publish without replacement.
        if dest.exists() {
            return Err(err(
                "output_exists",
                "归档目标在执行前已出现；没有覆盖，原内容保持不变",
            ));
        }
        j["steps"].as_array_mut().unwrap().push(json!({"id":"archive","label":"加密工作归档","status":"executing","message":"正在发布并核验完整加密工作包；目标已有文件不会覆盖。"}));
        j["archive_path"] = json!(dest);
        let archive_digest = digest(&encrypted);
        j["archive_intent_digest"] = json!(archive_digest);
        save(journal, j)?;
        check_output_path(p)?;
        atomic_new(dest, &encrypted, 0o600)?;
        if read(dest, (MAX_BYTES * 6) + 1024 * 1024)? != encrypted {
            return Err(err("archive_readback_failed", "归档写后校验未通过"));
        }
        j["archive_digest"] = json!(archive_digest);
        *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":"archive","label":"加密工作归档","status":"completed","message":"age 口令加密；口令未保存。原始工作内容保持不变。"});
        save(journal, j)?;
        Ok((dest.to_path_buf(), files))
    }

    /// Archive into the plan's private state path. Used by rebuild/cleanup,
    /// where the archive stays inside Lintel's own state directory so a receipt
    /// can point back to it without a caller-supplied destination.
    pub(crate) fn archive_work(
        &self,
        e: &Value,
        p: &Value,
        r: &Value,
        j: &mut Value,
        journal: &Path,
    ) -> Result<Vec<Value>> {
        let dest = self
            .state
            .join("archives")
            .join(format!("{}.age", string(p, "id")?));
        let (_, files) = self.write_archive(e, p, r, &dest, j, journal)?;
        Ok(files)
    }

    /// Archive-only plan: encrypt the selected work into an explicit output path
    /// (frozen at preview) or into Lintel's private state. Never creates a new
    /// environment, never touches settings, credentials or the original files.
    pub(crate) fn plan_archive(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let categories = categories(r)?;
        let selected = selection(r)?;
        let root = Path::new(string(&e, "root")?);
        let manifest = if selected.is_empty() {
            manifest(root, &categories)?
        } else {
            selected_manifest(root, &categories, &selected)?
        };
        let (output, output_parent_identity) = match r.get("output_path") {
            Some(Value::String(s)) if !s.is_empty() => {
                let (path, identity) = freeze_output_path(Path::new(s))?;
                (Some(stringify_path(path)), Some(identity))
            }
            _ => (None, None),
        };
        let actions = json!([{"id":"archive","label":"加密归档选中的工作内容","reversible":false}]);
        let mut extra = json!({
            "categories": categories,
            "manifest": manifest,
            "archive_passphrase_required": true,
            "output_path": output,
            "output_parent_identity": output_parent_identity,
            "outcome": "archive_only",
            "work_selection": work_selection(&selected),
        });
        if !selected.is_empty() {
            extra["selected_paths"] = json!(selected);
        }
        self.plan(
            &e,
            "archive",
            "仅加密归档所选工作内容",
            json!([]),
            vec![
                "原环境全部内容（不注销、不删除、不新建）",
                "settings、hooks、MCP 与插件文件",
            ],
            actions,
            extra,
        )
    }

    /// Preserve plan: encrypt the selected work and prepare a fresh root to
    /// migrate into. Distinct from plan_reset/rebuild because its receipt
    /// reports its own outcome and next steps instead of a partially-completed
    /// cleanup.
    pub(crate) fn plan_preserve(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let categories = categories(r)?;
        let selected = selection(r)?;
        let root = Path::new(string(&e, "root")?);
        let manifest = if selected.is_empty() {
            manifest(root, &categories)?
        } else {
            selected_manifest(root, &categories, &selected)?
        };
        let name = r
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("保全");
        let activation = activation(r, &categories);
        let frozen_target =
            freeze_new_target(&self.state, &manifest, activation["instructions"] == true)?;
        let actions = json!([
            {"id":"archive","label":"加密归档选中的工作内容","reversible":false},
            {"id":"create","label":"创建新的配置目录","reversible":false},
            {"id":"migrate","label":"选择性迁入；不启用 hooks/MCP","reversible":false}
        ]);
        let mut extra = json!({
            "categories": categories,
            "manifest": manifest,
            "archive_passphrase_required": true,
            "preserve_name": name,
            "outcome": "preserve",
            "frozen_target": frozen_target,
            "work_purpose": purposes(&categories),
            "activate": activation,
            "work_selection": work_selection(&selected),
        });
        if !selected.is_empty() {
            extra["selected_paths"] = json!(selected);
        }
        self.plan(
            &e,
            "preserve",
            "保全工作内容并准备新环境",
            json!([]),
            vec![
                "原环境全部内容（不注销、不删除、不停止进程）",
                "settings、hooks、MCP 与插件文件",
            ],
            actions,
            extra,
        )
    }

    /// Reset/rebuild plan. Freezes the planned new environment identity and the
    /// whole import mapping so a receipt never has to invent a target.
    pub(crate) fn plan_rebuild(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        if string(r, "recipe")? != "rebuild" {
            return Err(err("unsupported_recipe", "当前支持新环境重建配方"));
        }
        let categories = categories(r)?;
        let manifest = manifest(Path::new(string(&e, "root")?), &categories)?;
        let activation = activation(r, &categories);
        let frozen_target =
            freeze_new_target(&self.state, &manifest, activation["instructions"] == true)?;
        self.plan(&e,"rebuild","保留内容，准备新环境",json!([]),vec!["原环境全部内容（尚未注销或删除）","未选中的实例与项目文件"],json!([{"id":"archive","label":"加密归档选中的工作内容","reversible":false},{"id":"create","label":"创建新的配置目录","reversible":false},{"id":"migrate","label":"选择性迁入；不启用 hooks/MCP","reversible":false},{"id":"credentials","label":"旧登录及客户端状态尚需独立处理","reversible":false}]),json!({"categories":categories,"manifest":manifest,"archive_passphrase_required":true,"frozen_target":frozen_target,"work_purpose":purposes(&categories),"activate":activation}))
    }

    /// Preserve execution: archive the selection, then create a fresh owned root
    /// and migrate the selected categories read-back-verified. The original
    /// root, its login and its process state are left untouched.
    pub(crate) fn preserve(
        &self,
        e: &Value,
        p: &Value,
        r: &Value,
        j: &mut Value,
        journal: &Path,
    ) -> Result<()> {
        let files = self.archive_work(e, p, r, j, journal)?;
        self.migrate_files(
            e,
            p["extra"]["preserve_name"].as_str().unwrap_or("保全"),
            &files,
            j,
            journal,
            Some(&p["extra"]["frozen_target"]),
            &p["extra"]["activate"],
        )?;
        // Preservation keeps the source install untouched; that is the desired
        // outcome, not an outstanding task. Report it as retained with explicit
        // coverage instead of a misleading "not completed" step.
        j["steps"].as_array_mut().unwrap().push(json!({"id":"status","label":"旧环境、登录与运行进程","status":"preserved","message":"原 root、旧登录与运行进程全部保留；本次不注销、不删除、不停止。"}));
        j["coverage"] = json!({
            "categories": p["extra"]["categories"],
            "file_count": p["extra"]["manifest"].as_array().map_or(0, Vec::len),
            "work_purpose": p["extra"]["work_purpose"],
            "activation": p["extra"]["activate"],
            "old_login": "retained",
            "old_root": "retained",
            "service_binding": "unchanged"
        });
        j["next_steps"] = json!([
            if p["extra"]["activate"]["instructions"]
                .as_bool()
                .unwrap_or(true)
            {
                "在新环境采用自己的保护方案并完成一次真实启动；已启用的 CLAUDE.md 位于根指令位置，其余资料在 lintel-imports 参考区、未注册为活动会话。"
            } else {
                "在新环境采用自己的保护方案并完成一次真实启动；CLAUDE.md 与其余资料保留在 lintel-imports 参考区，未启用为指令或活动会话。"
            },
            "在新环境按官方流程正常登录；旧登录仍在原 root。",
            "原 root 的后台服务/进程绑定保持不变，Lintel 不会把新 root 改绑到已有 service。"
        ]);
        j["status"] = json!("completed");
        j["outcome"] = json!("preserved");
        Ok(())
    }

    /// Create a fresh owned root and copy the given archived entries into it,
    /// read-back verified. Does not archive or delete anything.
    fn migrate_files(
        &self,
        e: &Value,
        label: &str,
        files: &[Value],
        j: &mut Value,
        journal: &Path,
        frozen_target: Option<&Value>,
        activate: &Value,
    ) -> Result<()> {
        let instructions_active = activate["instructions"].as_bool().unwrap_or(true);
        let targets = migration_paths(files, instructions_active)?;
        preflight_migration_paths(&self.state.join("environments"), &targets, j, journal)?;
        j["steps"].as_array_mut().unwrap().push(json!({"id":"create","label":"新配置目录","status":"executing","message":"正在创建新环境；失败后需核对原任务与已生成目录。"}));
        save(journal, j)?;
        let new = match frozen_target {
            Some(frozen) => self.create_frozen(
                &format!("{} · {}", string(e, "name")?, label),
                frozen,
                Some((j, journal)),
            )?,
            None => self.create(
                &format!("{} · {}", string(e, "name")?, label),
                Some((j, journal)),
            )?,
        };
        j["new_environment_id"] = new["id"].clone();
        j["new_root"] = new["root"].clone();
        *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":"create","label":"新配置目录","status":"completed","message":"新建目录，没有复制登录或执行配置。目录外凭据仍可能共享。"});
        save(journal, j)?;
        let destination = PathBuf::from(string(&new, "root")?);
        let instruction_note = if instructions_active {
            "CLAUDE.md 放入新 root 指令位置（客户端会发现）；"
        } else {
            "CLAUDE.md 放入 lintel-imports 参考区，未放到指令位置；"
        };
        j["steps"].as_array_mut().unwrap().push(json!({"id":"migrate","label":"选择性迁入","status":"executing","message":"正在迁入并核验工作内容；失败时新 root 可能已有部分文件，请核对原任务。"}));
        save(journal, j)?;
        for (f, relative) in files.iter().zip(targets) {
            let target = destination.join(relative);
            migration_parent(
                target
                    .parent()
                    .ok_or_else(|| err("invalid_path", "缺少迁入目标"))?,
            )?;
            let bytes: Vec<u8> = serde_json::from_value(f["data"].clone())?;
            atomic_new(&target, &bytes, 0o600)?;
            if digest(&read(&target, 8 * 1024 * 1024)?) != f["digest"] {
                return Err(err("migration_failed", "迁入文件校验失败"));
            }
        }
        *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":"migrate","label":"选择性迁入","status":"completed","message":format!("{instruction_note}会话与记忆保存在 lintel-imports，未宣称可直接续聊。hooks、MCP、插件配置没有启用。")});
        save(journal, j)?;
        Ok(())
    }

    /// reset_client path: the work archive was already written earlier in the
    /// receipt (so nothing is deleted before it is preserved). Re-open that
    /// frozen archive and migrate it into a new root.
    pub(crate) fn migrate_to_new_root(
        &self,
        e: &Value,
        p: &Value,
        r: &Value,
        j: &mut Value,
        journal: &Path,
    ) -> Result<()> {
        let path = j["archive_path"]
            .as_str()
            .ok_or_else(|| err("archive_missing", "此任务没有工作归档"))?
            .to_owned();
        let (package, archive_digest) = read_package(Path::new(&path), check_passphrase(r)?)?;
        if j["archive_digest"].as_str() != Some(archive_digest.as_str()) {
            return Err(err(
                "stale_archive",
                "工作归档与本任务写入时的记录不一致；未新建环境、迁入或继续清理",
            ));
        }
        let files = validate_package_files(&package)?;
        self.migrate_files(
            e,
            "重建",
            &files,
            j,
            journal,
            Some(&p["extra"]["frozen_target"]),
            &p["extra"]["activate"],
        )
    }

    pub(crate) fn rebuild(
        &self,
        e: &Value,
        p: &Value,
        r: &Value,
        j: &mut Value,
        journal: &Path,
    ) -> Result<()> {
        let files = self.archive_work(e, p, r, j, journal)?;
        self.migrate_files(
            e,
            "重建",
            &files,
            j,
            journal,
            Some(&p["extra"]["frozen_target"]),
            &p["extra"]["activate"],
        )?;
        if p["kind"] == "rebuild" {
            j["steps"].as_array_mut().unwrap().push(json!({"id":"credentials","label":"旧登录与客户端状态","status":"not_completed","message":"旧环境没有注销、删除或停进程；完整处理请使用清理配方。"}));
            j["status"] = json!("partially_completed");
            j["warnings"].as_array_mut().unwrap().push(json!(
                "本次完成加密归档与新环境准备，旧状态清理尚未完成。请妥善保管归档口令。"
            ));
        }
        Ok(())
    }
}

fn stringify_path(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_root_intent_survives_registration_failure() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let home = base.join("home");
        let root = home.join("source");
        fs::create_dir_all(&root).unwrap();
        let engine = Engine::new(home.clone(), base.join("state")).unwrap();
        let environment = engine.register("synthetic", &root, false).unwrap();
        let job_id = crate::id();
        let journal = engine.path("jobs", &job_id);
        let mut receipt = json!({"id":job_id,"status":"executing","warnings":[],"steps":[]});
        // Registration will fail after the fresh directory has been created.
        // Only this synthetic inventory is changed.
        save(&engine.state.join("inventory.json"), &json!({})).unwrap();
        let failure = engine
            .migrate_files(
                &environment,
                "synthetic",
                &[],
                &mut receipt,
                &journal,
                None,
                &json!({"instructions": true}),
            )
            .unwrap_err();
        assert_eq!(failure.code, "invalid_inventory");
        let stored = load(&journal).unwrap();
        assert!(
            stored["new_root"].is_string(),
            "Orphaned root is absent from original job: {stored}"
        );
        let destination = Path::new(stored["new_root"].as_str().unwrap());
        assert!(destination.is_dir());
        assert_eq!(
            destination.parent().unwrap(),
            engine.state.join("environments")
        );
        uuid::Uuid::parse_str(stored["new_environment_id"].as_str().unwrap()).unwrap();
        assert_eq!(
            stored["steps"].as_array().unwrap().last().unwrap()["status"],
            "executing"
        );
        let reopened = Engine::new(home, engine.state.clone()).unwrap();
        let query = reopened.request(json!({"command":"job","job_id":job_id}));
        assert_eq!(query["ok"], true, "{query}");
        assert_eq!(query["data"]["status"], "needs_reconciliation");
        assert_eq!(query["data"]["new_root"], stored["new_root"]);
        assert_eq!(
            query["data"]["new_environment_id"],
            stored["new_environment_id"]
        );
        assert_eq!(
            fs::read_dir(engine.state.join("environments"))
                .unwrap()
                .count(),
            1
        );
        assert_eq!(
            query,
            reopened.request(json!({"command":"job","job_id":job_id}))
        );
    }

    #[test]
    fn new_root_journal_failure_prevents_creation_and_success_registers_intended_id() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let home = base.join("home");
        fs::create_dir(&home).unwrap();
        let engine = Engine::new(home, base.join("state")).unwrap();
        let blocked_journal = engine.path("jobs", &crate::id());
        fs::create_dir(&blocked_journal).unwrap();
        let mut failed = json!({"steps":[]});
        assert!(engine
            .create("synthetic", Some((&mut failed, &blocked_journal)))
            .is_err());
        assert!(!Path::new(failed["new_root"].as_str().unwrap()).exists());
        assert_eq!(
            fs::read_dir(engine.state.join("environments"))
                .unwrap()
                .count(),
            0
        );
        assert!(engine.inventory().unwrap().is_empty());
        let journal = engine.path("jobs", &crate::id());
        let mut receipt = json!({"steps":[]});
        let created = engine
            .create("synthetic", Some((&mut receipt, &journal)))
            .unwrap();
        let stored = load(&journal).unwrap();
        assert_eq!(stored["new_root"], created["root"]);
        assert_eq!(stored["new_environment_id"], created["id"]);
        assert_eq!(engine.inventory().unwrap(), vec![created]);
    }

    #[test]
    fn archive_space_check_uses_destination_before_publication() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let home = base.join("home");
        fs::create_dir(&home).unwrap();
        let root = home.join("source");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("CLAUDE.md"), "synthetic instruction").unwrap();
        let engine = Engine::new(home, base.join("state")).unwrap();
        let environment = engine.register("synthetic", &root, false).unwrap();
        let parent = base.join("export-volume");
        fs::create_dir(&parent).unwrap();
        let destination = parent.join("work.age");
        let preview = engine
            .plan_archive(&json!({"environment_id":environment["id"],"categories":["instructions"],"output_path":destination}))
            .unwrap();
        let plan = load(&engine.path("plans", string(&preview, "id").unwrap())).unwrap();
        // Isolate the shared writer's volume check. The state volume stays
        // usable; an unavailable destination must fail before encryption or
        // publishing an archive step/journal, rather than querying state.
        fs::remove_dir(&parent).unwrap();
        let journal = engine.state.join("jobs/synthetic.json");
        let mut receipt = json!({"steps":[]});
        let result = engine.write_archive(
            &environment,
            &plan,
            &json!({"archive_passphrase":"synthetic passphrase only"}),
            &destination,
            &mut receipt,
            &journal,
        );
        assert!(result.is_err());
        assert_eq!(receipt["steps"], json!([]), "{receipt}");
        assert!(!journal.exists());
        assert_eq!(
            fs::read_to_string(root.join("CLAUDE.md")).unwrap(),
            "synthetic instruction"
        );
    }

    #[test]
    fn partial_migration_keeps_active_step_and_created_root() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let home = base.join("home");
        let root = home.join("source");
        fs::create_dir_all(&root).unwrap();
        let engine = Engine::new(home, base.join("state")).unwrap();
        let environment = engine.register("synthetic", &root, false).unwrap();
        let first = b"synthetic instruction";
        let second = b"synthetic session";
        let files = vec![
            json!({"path":"CLAUDE.md","category":"instructions","data":first,"digest":digest(first)}),
            // Model a failed readback after the second file was published.
            json!({"path":"projects/example/session.jsonl","category":"sessions","data":second,"digest":digest(b"different readback")}),
        ];
        let journal = engine.state.join("jobs/synthetic.json");
        let mut receipt = json!({"steps":[]});
        // Legacy raw callers omit `activate`; the mapping keeps its historical
        // default where instructions land at the root position.
        let failure = engine
            .migrate_files(
                &environment,
                "synthetic",
                &files,
                &mut receipt,
                &journal,
                None,
                &json!({"instructions": true}),
            )
            .unwrap_err();
        assert_eq!(failure.code, "migration_failed");
        let stored = load(&journal).unwrap();
        let active = stored["steps"]
            .as_array()
            .unwrap()
            .iter()
            .find(|step| step["status"] == "executing")
            .unwrap();
        assert_eq!(active["id"], "migrate");
        let destination = Path::new(stored["new_root"].as_str().unwrap());
        assert_eq!(fs::read(destination.join("CLAUDE.md")).unwrap(), first);
        assert_eq!(
            fs::read(destination.join("lintel-imports/projects/example/session.jsonl")).unwrap(),
            second
        );
        assert!(stored["new_environment_id"].is_string());
    }

    #[test]
    fn migration_preserves_active_and_previously_imported_names() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let home = base.join("home");
        let root = home.join("source");
        fs::create_dir_all(&root).unwrap();
        let engine = Engine::new(home, base.join("state")).unwrap();
        let environment = engine.register("synthetic", &root, false).unwrap();
        let entries = [
            (
                "lintel-imports/projects/example/session.jsonl",
                "sessions",
                b"older session".as_slice(),
            ),
            (
                "projects/example/session.jsonl",
                "sessions",
                b"active session".as_slice(),
            ),
            (
                "lintel-imports/projects/example/lintel-1-session.jsonl",
                "sessions",
                b"reserved name".as_slice(),
            ),
            (
                "lintel-imports/projects/example/memory/notes.md",
                "memory",
                b"older memory".as_slice(),
            ),
            (
                "projects/example/memory/notes.md",
                "memory",
                b"active memory".as_slice(),
            ),
            (
                "lintel-imports/projects/foo.jsonl",
                "sessions",
                b"ancestor file".as_slice(),
            ),
            (
                "projects/foo.jsonl/session.jsonl",
                "sessions",
                b"child session".as_slice(),
            ),
            (
                "lintel-imports/projects/lintel-1-foo.jsonl/session.jsonl",
                "sessions",
                b"reserved directory".as_slice(),
            ),
        ];
        let files: Vec<Value> = entries.iter().map(|(path, category, bytes)|
            json!({"path":path,"category":category,"data":bytes,"digest":digest(bytes)})).collect();
        let journal = engine.state.join("jobs/synthetic.json");
        let mut receipt = json!({"steps":[]});
        engine
            .migrate_files(
                &environment,
                "synthetic",
                &files,
                &mut receipt,
                &journal,
                None,
                &json!({"instructions": true}),
            )
            .unwrap();
        let destination = Path::new(receipt["new_root"].as_str().unwrap());
        for (relative, bytes) in [
            ("session.jsonl", b"older session".as_slice()),
            ("lintel-2-session.jsonl", b"active session".as_slice()),
            ("lintel-1-session.jsonl", b"reserved name".as_slice()),
            ("memory/notes.md", b"older memory".as_slice()),
            ("memory/lintel-1-notes.md", b"active memory".as_slice()),
        ] {
            assert_eq!(
                fs::read(
                    destination
                        .join("lintel-imports/projects/example")
                        .join(relative)
                )
                .unwrap(),
                bytes
            );
        }
        for (relative, bytes) in [
            ("lintel-2-foo.jsonl", b"ancestor file".as_slice()),
            ("foo.jsonl/session.jsonl", b"child session".as_slice()),
            (
                "lintel-1-foo.jsonl/session.jsonl",
                b"reserved directory".as_slice(),
            ),
        ] {
            assert_eq!(
                fs::read(destination.join("lintel-imports/projects").join(relative)).unwrap(),
                bytes
            );
        }
        let again = manifest(destination, &["sessions".to_string(), "memory".to_string()]).unwrap();
        assert_eq!(again.len(), entries.len());
        assert!(again.iter().all(|file| !file["path"]
            .as_str()
            .unwrap()
            .contains("lintel-imports/lintel-imports")));
        assert_eq!(
            migration_paths(&again, false).unwrap(),
            again
                .iter()
                .map(|file| PathBuf::from(file["path"].as_str().unwrap()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn classify_covers_reference_instruction_names_only() {
        // The active root slot.
        assert_eq!(classify(Path::new("CLAUDE.md")), Some("instructions"));
        // The inactive reference slot and its generated collision names.
        assert_eq!(
            classify(Path::new("lintel-imports/CLAUDE.md")),
            Some("instructions")
        );
        assert_eq!(
            classify(Path::new("lintel-imports/lintel-1-CLAUDE.md")),
            Some("instructions")
        );
        assert_eq!(
            classify(Path::new("lintel-imports/lintel-42-CLAUDE.md")),
            Some("instructions")
        );
        // Finite rule: unrelated lookalikes are not promoted to user assets.
        assert_eq!(classify(Path::new("lintel-imports/README.md")), None);
        assert_eq!(classify(Path::new("lintel-imports/lintel-CLAUDE.md")), None);
        assert_eq!(
            classify(Path::new("lintel-imports/lintel-x-CLAUDE.md")),
            None
        );
        assert_eq!(classify(Path::new("lintel-imports/nested/CLAUDE.md")), None);
        assert_eq!(classify(Path::new("lintel-imports/CLAUDE.md.bak")), None);
    }

    #[test]
    fn reference_instructions_survive_repeated_imports_without_double_wrap() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let home = base.join("home");
        let root = home.join("source");
        fs::create_dir_all(&root).unwrap();
        // A reference position produced by an earlier `instructions_active:false`
        // migration, plus its collision disambiguation, plus unrelated memory.
        let instruction = b"reference instruction".as_slice();
        let collision = b"reference collision".as_slice();
        let memory = b"reference memory".as_slice();
        let groups = json!([
            {"path":"lintel-imports/CLAUDE.md","category":"instructions"},
            {"path":"lintel-imports/lintel-1-CLAUDE.md","category":"instructions"},
            {"path":"lintel-imports/projects/demo/memory/MEMORY.md","category":"memory"},
        ]);
        // Model what the manifest would classify from that tree, then freeze the
        // reference mapping (instructions inactive) and assert it is unchanged.
        let mut files = vec![];
        for (entry, bytes) in
            groups
                .as_array()
                .unwrap()
                .iter()
                .zip([instruction, collision, memory])
        {
            let path = entry["path"].as_str().unwrap();
            assert_eq!(
                classify(Path::new(path)),
                Some(entry["category"].as_str().unwrap()),
                "classification of retained path {path}"
            );
            files.push(json!({
                "path": path,
                "category": entry["category"],
                "data": bytes,
                "digest": digest(bytes),
            }));
        }
        let engine = Engine::new(home, base.join("state")).unwrap();
        let environment = engine.register("synthetic", &root, false).unwrap();
        let journal = engine.state.join("jobs/synthetic.json");
        let mut receipt = json!({"steps":[]});
        engine
            .migrate_files(
                &environment,
                "synthetic",
                &files,
                &mut receipt,
                &journal,
                None,
                &json!({"instructions": false}),
            )
            .unwrap();
        let destination = Path::new(receipt["new_root"].as_str().unwrap());
        // Reference instructions stay reference (no root CLAUDE.md) and keep
        // their exact names; nothing is wrapped a second time.
        assert!(!destination.join("CLAUDE.md").exists());
        assert_eq!(
            fs::read(destination.join("lintel-imports/CLAUDE.md")).unwrap(),
            instruction
        );
        assert_eq!(
            fs::read(destination.join("lintel-imports/lintel-1-CLAUDE.md")).unwrap(),
            collision
        );
        assert_eq!(
            fs::read(destination.join("lintel-imports/projects/demo/memory/MEMORY.md")).unwrap(),
            memory
        );
        assert!(!destination.join("lintel-imports/lintel-imports").exists());
        // A later archive over the migrated tree still sees all three sources.
        let again = manifest(destination, &["instructions".into(), "memory".into()]).unwrap();
        let paths: Vec<&str> = again
            .iter()
            .map(|file| file["path"].as_str().unwrap())
            .collect();
        assert_eq!(
            paths,
            vec![
                "lintel-imports/CLAUDE.md",
                "lintel-imports/lintel-1-CLAUDE.md",
                "lintel-imports/projects/demo/memory/MEMORY.md",
            ]
        );
        // Repeated imports keep the same targets: reference stays reference.
        assert_eq!(
            migration_paths(&again, false).unwrap(),
            again
                .iter()
                .map(|file| PathBuf::from(file["path"].as_str().unwrap()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn active_and_reference_instructions_coexist_with_generated_names() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let home = base.join("home");
        let root = home.join("source");
        fs::create_dir_all(&root).unwrap();
        // Active root instructions, a reference instruction and an already
        // collided reference must all be retained distinctly.
        let active = b"active instruction".as_slice();
        let reference = b"reference instruction".as_slice();
        let collided = b"already collided reference".as_slice();
        let files = vec![
            json!({"path":"CLAUDE.md","category":"instructions","data":active,"digest":digest(active)}),
            json!({"path":"lintel-imports/CLAUDE.md","category":"instructions","data":reference,"digest":digest(reference)}),
            json!({"path":"lintel-imports/lintel-1-CLAUDE.md","category":"instructions","data":collided,"digest":digest(collided)}),
        ];
        let engine = Engine::new(home, base.join("state")).unwrap();
        let environment = engine.register("synthetic", &root, false).unwrap();
        let journal = engine.state.join("jobs/synthetic.json");
        let mut receipt = json!({"steps":[]});
        // Instructions inactive: the active root CLAUDE.md joins the reference
        // area, where it must not overwrite either existing reference.
        engine
            .migrate_files(
                &environment,
                "synthetic",
                &files,
                &mut receipt,
                &journal,
                None,
                &json!({"instructions": false}),
            )
            .unwrap();
        let destination = Path::new(receipt["new_root"].as_str().unwrap());
        let mut retained = fs::read_dir(destination.join("lintel-imports"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect::<Vec<_>>();
        retained.sort();
        assert_eq!(
            retained,
            vec!["CLAUDE.md", "lintel-1-CLAUDE.md", "lintel-2-CLAUDE.md"]
        );
        let bytes: HashSet<Vec<u8>> = retained
            .iter()
            .map(|name| fs::read(destination.join("lintel-imports").join(name)).unwrap())
            .collect();
        assert_eq!(
            bytes,
            [active, reference, collided]
                .iter()
                .map(|b| b.to_vec())
                .collect::<HashSet<_>>()
        );
    }

    #[test]
    fn archive_publication_intent_keeps_its_destination_on_failure() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let home = base.join("home");
        let root = home.join("source");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("CLAUDE.md"), "synthetic instruction").unwrap();
        let engine = Engine::new(home, base.join("state")).unwrap();
        let environment = engine.register("synthetic", &root, false).unwrap();
        let preview = engine
            .plan_archive(
                &json!({"environment_id":environment["id"],"categories":["instructions"]}),
            )
            .unwrap();
        let plan = load(&engine.path("plans", string(&preview, "id").unwrap())).unwrap();
        // A destination whose leaf cannot be published isolates the durable
        // boundary immediately before atomic_new, without a process race.
        let destination = engine.state.join("archives").join("x".repeat(300));
        let journal = engine.path("jobs", string(&preview, "id").unwrap());
        let mut receipt = json!({"id":preview["id"],"plan_id":preview["id"],"steps":[]});
        assert!(engine
            .write_archive(
                &environment,
                &plan,
                &json!({"archive_passphrase":"synthetic passphrase only"}),
                &destination,
                &mut receipt,
                &journal
            )
            .is_err());
        let stored = load(&journal).unwrap();
        assert_eq!(stored["archive_path"].as_str(), destination.to_str());
        assert_eq!(stored["steps"][0]["status"], "executing");
        assert!(stored["archive_digest"].is_null());
        assert!(stored["archive_intent_digest"].is_string());
        assert!(!destination.exists());
        assert_eq!(
            fs::read_to_string(root.join("CLAUDE.md")).unwrap(),
            "synthetic instruction"
        );
    }

    #[test]
    fn migration_case_equivalence_is_checked_before_creating_a_root() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        fs::write(base.join("filesystem-case-check"), b"synthetic").unwrap();
        let folds_case = base.join("FILESYSTEM-CASE-CHECK").exists();
        let home = base.join("home");
        let root = home.join("source");
        fs::create_dir_all(&root).unwrap();
        let engine = Engine::new(home, base.join("state")).unwrap();
        let environment = engine.register("synthetic", &root, false).unwrap();
        let files: Vec<_> = [("projects/foo.jsonl", b"lower".as_slice()), ("projects/Foo.jsonl", b"upper".as_slice())]
            .into_iter().map(|(path, bytes)| json!({"path":path,"category":"sessions","digest":digest(bytes),"data":bytes})).collect();
        let journal = engine.state.join("jobs/synthetic.json");
        let mut receipt = json!({"steps":[]});
        let result = engine.migrate_files(
            &environment,
            "synthetic",
            &files,
            &mut receipt,
            &journal,
            None,
            &json!({"instructions": true}),
        );
        if folds_case {
            assert_eq!(result.unwrap_err().code, "migration_path_conflict");
            assert!(receipt["new_root"].is_null());
            assert_eq!(engine.inventory().unwrap().len(), 1);
            assert_eq!(
                fs::read_dir(engine.state.join("environments"))
                    .unwrap()
                    .count(),
                0
            );
        } else {
            result.unwrap();
            let new_root = Path::new(receipt["new_root"].as_str().unwrap());
            assert_eq!(
                fs::read(new_root.join("lintel-imports/projects/foo.jsonl")).unwrap(),
                b"lower"
            );
            assert_eq!(
                fs::read(new_root.join("lintel-imports/projects/Foo.jsonl")).unwrap(),
                b"upper"
            );
        }
    }

    #[test]
    fn interrupted_probe_is_discoverable_from_the_original_job() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let home = base.join("home");
        fs::create_dir(&home).unwrap();
        let state = base.join("state");
        let engine = Engine::new(home.clone(), state.clone()).unwrap();
        let job_id = uuid::Uuid::new_v4().to_string();
        let journal = engine.path("jobs", &job_id);
        let mut receipt = json!({"id":job_id,"status":"executing","steps":[],"warnings":[]});
        let scratch =
            record_migration_probe(&state.join("environments"), &mut receipt, &journal).unwrap();
        assert!(!scratch.exists(), "intent must precede directory creation");
        let before = load(&journal).unwrap();
        assert_eq!(before["migration_probe"]["path"].as_str(), scratch.to_str());
        assert_eq!(before["steps"][0]["status"], "executing");
        // Model a killed worker after the durable boundary: cleanup cannot run.
        private_dir(&scratch).unwrap();
        fs::write(scratch.join("placeholder"), b"").unwrap();
        drop(engine);
        let next = Engine::new(home, state).unwrap();
        for _ in 0..2 {
            let response = next.request(json!({"command":"job","job_id":job_id}));
            assert_eq!(response["ok"], true);
            assert_eq!(response["data"]["id"], job_id);
            assert_eq!(response["data"]["status"], "needs_reconciliation");
            assert_eq!(
                response["data"]["migration_probe"],
                before["migration_probe"]
            );
            assert_eq!(fs::read(scratch.join("placeholder")).unwrap(), b"");
        }
    }

    fn selection_root() -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap().join("root");
        fs::create_dir_all(root.join("projects/demo/memory")).unwrap();
        (temp, root)
    }

    #[test]
    fn selection_syntax_is_finite_and_rejects_escapes() {
        assert_eq!(selection(&json!({})).unwrap(), Vec::<String>::new());
        assert!(selection(&json!({"selected_paths": []})).is_err());
        for bad in [
            json!([""]),
            json!(["/abs"]),
            json!(["../escape"]),
            json!(["./x"]),
            json!(["a//b"]),
            json!(["a/./b"]),
            json!(["back\\slash"]),
            json!(["ctrl\nchar"]),
            json!(["  padded  "]),
            json!(["dup", "dup"]),
            json!([1]),
            json!(["猫".repeat(2000)]),
        ] {
            assert!(selection(&json!({"selected_paths": bad})).is_err(), "{bad}");
        }
        assert_eq!(
            selection(&json!({"selected_paths": ["projects/demo/session.jsonl"]})).unwrap(),
            vec!["projects/demo/session.jsonl".to_string()]
        );
        assert!(has_selection(&json!({"selected_paths": []})));
        assert!(!has_selection(&json!({})));
    }

    #[test]
    fn selected_manifest_excludes_unselected_oversized_files() {
        let (_t, root) = selection_root();
        fs::create_dir_all(root.join("projects/demo")).unwrap();
        let good = b"kept session bytes\n";
        fs::write(root.join("projects/demo/keep.jsonl"), good).unwrap();
        fs::write(root.join("projects/demo/other.jsonl"), b"unrelated\n").unwrap();
        // An unselected 9 MiB session exceeds the 8 MiB per-file limit.
        let big = root.join("projects/demo/huge.jsonl");
        fs::write(&big, vec![b'x'; 9 * 1024 * 1024]).unwrap();
        let memory = b"# memory\n";
        fs::write(root.join("projects/demo/memory/MEMORY.md"), memory).unwrap();

        // Whole-category manifest refuses the oversized neighbour.
        let categories = vec!["memory".to_string(), "sessions".to_string()];
        assert!(manifest(&root, &categories).is_err());

        // Exact selection that excludes it keeps the other originals intact.
        let entries = selected_manifest(
            &root,
            &categories,
            &[
                "projects/demo/keep.jsonl".to_string(),
                "projects/demo/memory/MEMORY.md".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["path"], "projects/demo/keep.jsonl");
        assert_eq!(entries[0]["digest"], digest(good));
        assert_eq!(entries[0]["category"], "sessions");
        assert_eq!(entries[1]["category"], "memory");
        assert_eq!(fs::read(&big).unwrap().len(), 9 * 1024 * 1024);
        assert_eq!(
            fs::read(root.join("projects/demo/other.jsonl")).unwrap(),
            b"unrelated\n"
        );
    }

    #[test]
    fn selected_manifest_rejects_missing_out_of_category_and_nonregular() {
        let (_t, root) = selection_root();
        fs::write(root.join("projects/demo/keep.jsonl"), b"x\n").unwrap();
        let categories = vec!["sessions".to_string()];
        assert_eq!(
            selected_manifest(&root, &categories, &["projects/demo/keep.jsonl".into()])
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            selected_manifest(&root, &categories, &["projects/demo/missing.jsonl".into()])
                .unwrap_err()
                .code,
            "selected_missing"
        );
        // A path whose category is not in the explicit selection is refused.
        fs::write(root.join("projects/demo/memory/notes.md"), b"# n\n").unwrap();
        assert_eq!(
            selected_manifest(
                &root,
                &categories,
                &["projects/demo/memory/notes.md".into()]
            )
            .unwrap_err()
            .code,
            "invalid_selection"
        );
        // A symlinked selected path is refused, never followed.
        std::os::unix::fs::symlink(
            root.join("projects/demo/keep.jsonl"),
            root.join("projects/demo/link.jsonl"),
        )
        .unwrap();
        assert_eq!(
            selected_manifest(&root, &categories, &["projects/demo/link.jsonl".into()])
                .unwrap_err()
                .code,
            "invalid_selection"
        );
    }

    #[test]
    fn inventory_is_bounded_paged_and_digest_bound() {
        let (_t, root) = selection_root();
        fs::write(root.join("projects/demo/session.jsonl"), b"{\"a\":1}\n").unwrap();
        fs::write(root.join("projects/demo/memory/MEMORY.md"), b"# m\n").unwrap();
        let categories = vec!["memory".to_string(), "sessions".to_string()];
        let first = inventory("env", &root, &categories, &json!({})).unwrap();
        assert_eq!(first["complete"], true);
        assert_eq!(first["next_offset"], Value::Null);
        assert_eq!(first["total_files"], 2);
        let digest_value = first["digest"].as_str().unwrap().to_string();
        assert_eq!(digest_value.len(), 64);
        // Rows are ordered by full relative UTF-8 path.
        let paths: Vec<_> = first["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["path"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            paths,
            vec![
                "projects/demo/memory/MEMORY.md",
                "projects/demo/session.jsonl"
            ]
        );
        assert!(first["reason"].is_null());

        // Paging with the matching digest is accepted; a mismatched one is stale.
        assert!(inventory(
            "env",
            &root,
            &categories,
            &json!({"offset": 1, "expected_digest": digest_value}),
        )
        .is_ok());
        assert_eq!(
            inventory(
                "env",
                &root,
                &categories,
                &json!({"expected_digest": "a".repeat(64)}),
            )
            .unwrap_err()
            .code,
            "stale_inventory"
        );
    }

    #[test]
    fn inventory_first_page_is_complete_and_exact_encoding_is_bounded() {
        let (_temp, root) = selection_root();
        for index in 0..101 {
            fs::write(
                root.join(format!("projects/demo/memory/{index:03}.md")),
                b"synthetic",
            )
            .unwrap();
        }
        let categories = vec!["memory".into()];
        let first = inventory("env", &root, &categories, &json!({})).unwrap();
        assert_eq!(first["complete"], true);
        assert_eq!(first["next_offset"], 100);
        let last = inventory(
            "env",
            &root,
            &categories,
            &json!({"offset":100,"expected_digest":first["digest"]}),
        )
        .unwrap();
        assert_eq!(last["complete"], true);
        assert!(last["next_offset"].is_null());
        assert_eq!(last["files"].as_array().unwrap().len(), 1);
        let paths: Vec<String> = (0..200)
            .map(|index| format!("projects/demo/{index}-{}.jsonl", "\"".repeat(2000)))
            .collect();
        assert!(paths.iter().map(String::len).sum::<usize>() < SELECTION_TOTAL_BYTES);
        assert!(selection(&json!({"selected_paths":paths})).is_err());
        assert!(
            frozen_manifest(
                &root,
                &json!({"categories":["memory"],"selected_paths":null})
            )
            .is_err(),
            "Malformed frozen selection must never become whole category"
        );
    }

    #[test]
    fn inventory_marks_scan_incomplete_without_reading_content() {
        let (_t, root) = selection_root();
        fs::write(root.join("projects/demo/real.jsonl"), b"x\n").unwrap();
        os_symlink("/etc/hosts", &root.join("projects/demo/link.jsonl"));
        let report = inventory("env", &root, &["sessions".to_string()], &json!({})).unwrap();
        assert_eq!(report["complete"], false);
        assert!(
            report["next_offset"].is_null(),
            "Incomplete scan cannot offer a page that makes no progress"
        );
        assert_eq!(report["reason"], "symlink_in_scope");
        assert_eq!(report["unobserved"], "projects/demo/link.jsonl");
        assert_eq!(report["total_files"], 1);
    }

    fn os_symlink(target: &str, link: &Path) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }
}

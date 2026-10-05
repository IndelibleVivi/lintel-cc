use crate::{archive, err, now, storage::*, string, Engine, Result};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    os::unix::ffi::OsStrExt,
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};
const MAX_BYTES: u64 = 32 * 1024 * 1024;
const MAX_FILES: usize = 10000;

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
    let name = relative.file_name()?.to_str()?;
    if relative == Path::new("CLAUDE.md") {
        return Some("instructions");
    }
    // Work previously preserved into lintel-imports keeps its category so a
    // later archive covers it again; only the active root CLAUDE.md counts as
    // instructions.
    let work_area =
        relative.starts_with("projects") || relative.starts_with("lintel-imports/projects");
    if work_area
        && relative.components().any(|c| c.as_os_str() == "memory")
        && relative.extension().is_some_and(|x| x == "md")
    {
        return Some("memory");
    }
    if work_area && name.ends_with(".jsonl") {
        return Some("sessions");
    }
    None
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
/// Overview statistics for the inspector. This walk reads metadata only: no
/// file contents, no digests, and none of the archive admission limits (those
/// stay on `manifest`, which destructive plans must still pass in full). When
/// the scan budget runs out or entries cannot be read, the result is marked
/// `complete: false` instead of failing the unrelated settings inspection.
pub fn summaries(root: &Path) -> Result<Vec<Value>> {
    guard(root)?;
    let start = Instant::now();
    let mut stack = vec![root.to_path_buf()];
    let mut stats: std::collections::BTreeMap<&'static str, (u64, u64)> = Default::default();
    let mut complete = true;
    let mut entries = 0;
    'walk: while let Some(dir) = stack.pop() {
        let items = match fs::read_dir(&dir) {
            Ok(items) => items,
            Err(_) => {
                complete = false;
                continue;
            }
        };
        for item in items {
            entries += 1;
            if entries > 50000 || start.elapsed() > Duration::from_secs(30) {
                complete = false;
                break 'walk;
            }
            let item = match item {
                Ok(item) => item,
                Err(_) => {
                    complete = false;
                    continue;
                }
            };
            let p = item.path();
            let rel = match p.strip_prefix(root) {
                Ok(rel) => rel,
                Err(_) => {
                    complete = false;
                    continue;
                }
            };
            let m = match fs::symlink_metadata(&p) {
                Ok(m) => m,
                Err(_) => {
                    complete = false;
                    continue;
                }
            };
            if m.file_type().is_symlink() {
                if rel.starts_with("projects")
                    || rel.starts_with("lintel-imports")
                    || rel == Path::new("CLAUDE.md")
                {
                    complete = false;
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
                let stat = stats.entry(category).or_default();
                stat.0 += 1;
                stat.1 += m.len();
            }
        }
    }
    Ok(["instructions", "memory", "sessions"]
        .iter()
        .map(|c| {
            let (count, bytes) = stats.get(c).copied().unwrap_or((0, 0));
            json!({"category":c,"count":count,"bytes":bytes,"complete":complete})
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
        let categories = categories(&p["extra"])?;
        let entries = manifest(&root, &categories)?;
        if json!(entries) != p["extra"]["manifest"] {
            return Err(err("stale_plan", "工作内容在执行前改变，请重新生成计划"));
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
            let data = read(&root.join(string(entry, "path")?), 8 * 1024 * 1024)?;
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
        let manifest = manifest(Path::new(string(&e, "root")?), &categories)?;
        let (output, output_parent_identity) = match r.get("output_path") {
            Some(Value::String(s)) if !s.is_empty() => {
                let (path, identity) = freeze_output_path(Path::new(s))?;
                (Some(stringify_path(path)), Some(identity))
            }
            _ => (None, None),
        };
        let actions = json!([{"id":"archive","label":"加密归档选中的工作内容","reversible":false}]);
        self.plan(
            &e,
            "archive",
            "仅加密归档所选工作内容",
            json!([]),
            vec!["原环境全部内容（不注销、不删除、不新建）", "settings、hooks、MCP 与插件文件"],
            actions,
            json!({"categories":categories,"manifest":manifest,"archive_passphrase_required":true,"output_path":output,"output_parent_identity":output_parent_identity,"outcome":"archive_only"}),
        )
    }

    /// Preserve plan: encrypt the selected work and prepare a fresh root to
    /// migrate into. Distinct from plan_reset/rebuild because its receipt
    /// reports its own outcome and next steps instead of a partially-completed
    /// cleanup.
    pub(crate) fn plan_preserve(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let categories = categories(r)?;
        let manifest = manifest(Path::new(string(&e, "root")?), &categories)?;
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
        self.plan(
            &e,
            "preserve",
            "保全工作内容并准备新环境",
            json!([]),
            vec!["原环境全部内容（不注销、不删除、不停止进程）", "settings、hooks、MCP 与插件文件"],
            actions,
            json!({"categories":categories,"manifest":manifest,"archive_passphrase_required":true,"preserve_name":name,"outcome":"preserve","frozen_target":frozen_target,"work_purpose":purposes(&categories),"activate":activation}),
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
}

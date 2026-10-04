use crate::{archive, err, now, storage::*, string, Engine, Result};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};
const MAX_BYTES: u64 = 32 * 1024 * 1024;
const MAX_FILES: usize = 10000;

/// Absolute-path guard for an explicit archive destination: parent must already
/// exist (we never create directories outside the frozen path), the path must be
/// absolute without parent-directory hops, must not itself be a symlink, and if
/// it already exists it must be a regular single-link file owned by this user.
/// Existing content is never a valid target: callers reject before writing.
pub(crate) fn freeze_output_path(path: &Path) -> Result<PathBuf> {
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
    Ok(path.to_path_buf())
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
        save(journal, j)?;
        atomic_new(dest, &encrypted, 0o600)?;
        j["archive_path"] = json!(dest);
        if read(dest, (MAX_BYTES * 6) + 1024 * 1024)? != encrypted {
            return Err(err("archive_readback_failed", "归档写后校验未通过"));
        }
        j["archive_path"] = json!(dest);
        j["archive_digest"] = json!(digest(&encrypted));
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
        let output = match r.get("output_path") {
            Some(Value::String(s)) if !s.is_empty() => {
                Some(stringify_path(freeze_output_path(Path::new(s))?))
            }
            _ => None,
        };
        let actions = json!([{"id":"archive","label":"加密归档选中的工作内容","reversible":false}]);
        self.plan(
            &e,
            "archive",
            "仅加密归档所选工作内容",
            json!([]),
            vec!["原环境全部内容（不注销、不删除、不新建）", "settings、hooks、MCP 与插件文件"],
            actions,
            json!({"categories":categories,"manifest":manifest,"archive_passphrase_required":true,"output_path":output,"outcome":"archive_only"}),
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
            json!({"categories":categories,"manifest":manifest,"archive_passphrase_required":true,"preserve_name":name,"outcome":"preserve"}),
        )
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
        )?;
        // Preservation keeps the source install untouched; that is the desired
        // outcome, not an outstanding task. Report it as retained with explicit
        // coverage instead of a misleading "not completed" step.
        j["steps"].as_array_mut().unwrap().push(json!({"id":"status","label":"旧环境、登录与运行进程","status":"preserved","message":"原 root、旧登录与运行进程全部保留；本次不注销、不删除、不停止。"}));
        j["coverage"] = json!({
            "categories": p["extra"]["categories"],
            "file_count": p["extra"]["manifest"].as_array().map_or(0, Vec::len),
            "old_login": "retained",
            "old_root": "retained",
            "service_binding": "unchanged"
        });
        j["next_steps"] = json!([
            "在新环境采用自己的保护方案并完成一次真实启动，再核对运行效果；迁移内容不会自动启用。",
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
    ) -> Result<()> {
        j["steps"].as_array_mut().unwrap().push(json!({"id":"create","label":"新配置目录","status":"executing","message":"正在创建新环境；失败后需核对原任务与已生成目录。"}));
        save(journal, j)?;
        let new = self.create(&format!("{} · {}", string(e, "name")?, label))?;
        j["new_environment_id"] = new["id"].clone();
        j["new_root"] = new["root"].clone();
        *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":"create","label":"新配置目录","status":"completed","message":"新建目录，没有复制登录或执行配置。目录外凭据仍可能共享。"});
        save(journal, j)?;
        let destination = PathBuf::from(string(&new, "root")?);
        j["steps"].as_array_mut().unwrap().push(json!({"id":"migrate","label":"选择性迁入","status":"executing","message":"正在迁入并核验工作内容；失败时新 root 可能已有部分文件，请核对原任务。"}));
        save(journal, j)?;
        for f in files {
            let relative = Path::new(string(f, "path")?);
            // Only the one supported text instruction location is active. Session/memory formats are preserved for inspection, not falsely claimed resumable.
            // Content already held in lintel-imports keeps its logical path; it
            // must not be wrapped into lintel-imports/lintel-imports.
            let target =
                if f["category"] == "instructions" || relative.starts_with("lintel-imports") {
                    destination.join(relative)
                } else {
                    destination.join("lintel-imports").join(relative)
                };
            private_dir(
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
        *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":"migrate","label":"选择性迁入","status":"completed","message":"CLAUDE.md 放入新 root；会话与记忆保存在 lintel-imports，未宣称可直接续聊。hooks、MCP、插件配置没有启用。"});
        save(journal, j)?;
        Ok(())
    }

    /// reset_client path: the work archive was already written earlier in the
    /// receipt (so nothing is deleted before it is preserved). Re-open that
    /// frozen archive and migrate it into a new root.
    pub(crate) fn migrate_to_new_root(
        &self,
        e: &Value,
        _p: &Value,
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
        self.migrate_files(e, "重建", &files, j, journal)
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
        self.migrate_files(e, "重建", &files, j, journal)?;
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
        let failure = engine
            .migrate_files(&environment, "synthetic", &files, &mut receipt, &journal)
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
}

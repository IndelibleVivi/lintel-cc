use crate::{err, now, storage::*, string, Engine, Result};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
const MAX_BYTES: u64 = 32 * 1024 * 1024;
const MAX_FILES: usize = 10000;

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
    if relative.starts_with("projects")
        && relative.components().any(|c| c.as_os_str() == "memory")
        && relative.extension().is_some_and(|x| x == "md")
    {
        return Some("memory");
    }
    if relative.starts_with("projects") && name.ends_with(".jsonl") {
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
                if rel.starts_with("projects") || rel == Path::new("CLAUDE.md") {
                    return Err(err(
                        "symlink_target",
                        "工作内容包含符号链接，需要先明确其实际目标",
                    ));
                }
                continue;
            }
            if m.is_dir() {
                if rel.starts_with("projects") {
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
pub fn summaries(root: &Path) -> Result<Vec<Value>> {
    let all = manifest(
        root,
        &["instructions".into(), "memory".into(), "sessions".into()],
    )?;
    Ok(["instructions","memory","sessions"].iter().map(|c|{let files:Vec<_>=all.iter().filter(|v|v["category"]==*c).collect();json!({"category":c,"count":files.len(),"bytes":files.iter().map(|v|v["bytes"].as_u64().unwrap_or(0)).sum::<u64>()})}).collect())
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
    pub(crate) fn archive_work(
        &self,
        e: &Value,
        p: &Value,
        r: &Value,
        j: &mut Value,
        journal: &Path,
    ) -> Result<Vec<Value>> {
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
        if fs2::available_space(&self.state)? < size * 6 + 1024 * 1024 {
            return Err(err(
                "insufficient_space",
                "恢复存储空间不足；原始内容尚未删除",
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
        let package = json!({"schema":"lintel.work/1","created_at":now(),"files":files,"notes":"Selected working content only; runtime credentials and executable config excluded."});
        let encrypted = crate::archive::seal(&package, pass)?;
        let archive = self
            .state
            .join("archives")
            .join(format!("{}.age", string(p, "id")?));
        atomic(&archive, &encrypted, 0o600)?;
        if read(&archive, (MAX_BYTES * 6) + 1024 * 1024)? != encrypted {
            return Err(err("archive_readback_failed", "归档写后校验未通过"));
        }
        j["archive_path"] = json!(archive);
        j["steps"] = json!([{"id":"archive","label":"加密工作归档","status":"completed","message":"age 口令加密；口令未保存。原始工作内容保持不变。"}]);
        save(journal, j)?;
        Ok(files)
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
        let new = self.create(&format!("{} · 重建", string(e, "name")?))?;
        j["new_environment_id"] = new["id"].clone();
        j["new_root"] = new["root"].clone();
        j["steps"].as_array_mut().unwrap().push(json!({"id":"create","label":"新配置目录","status":"completed","message":"新建目录，没有复制登录或执行配置。目录外凭据仍可能共享。"}));
        save(journal, j)?;
        let destination = PathBuf::from(string(&new, "root")?);
        for f in &files {
            let relative = Path::new(string(f, "path")?);
            // Only the one supported text instruction location is active. Session/memory formats are preserved for inspection, not falsely claimed resumable.
            let target = if f["category"] == "instructions" {
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
            atomic(&target, &bytes, 0o600)?;
            if digest(&read(&target, 8 * 1024 * 1024)?) != f["digest"] {
                return Err(err("migration_failed", "迁入文件校验失败"));
            }
        }
        j["steps"].as_array_mut().unwrap().push(json!({"id":"migrate","label":"选择性迁入","status":"completed","message":"CLAUDE.md 放入新 root；会话与记忆保存在 lintel-imports，未宣称可直接续聊。hooks、MCP、插件配置没有启用。"}));
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

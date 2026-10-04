use crate::{err, now, safe_id, storage::*, string, work, Engine, Result};
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(crate) const MAX_PLAIN: u64 = 200 * 1024 * 1024;

pub(crate) fn seal(value: &Value, pass: &str) -> Result<Vec<u8>> {
    let encryptor =
        age::Encryptor::with_user_passphrase(age::secrecy::SecretString::from(pass.to_owned()));
    let mut bytes = vec![];
    let mut output = encryptor
        .wrap_output(&mut bytes)
        .map_err(|_| err("archive_failed", "无法初始化归档加密"))?;
    output.write_all(&serde_json::to_vec(value)?)?;
    output
        .finish()
        .map_err(|_| err("archive_failed", "归档加密未完整完成"))?;
    Ok(bytes)
}

pub(crate) fn unseal(bytes: &[u8], pass: &str) -> Result<Value> {
    let decryptor =
        age::Decryptor::new(bytes).map_err(|_| err("invalid_archive", "不是有效的 age 归档"))?;
    let identity = age::scrypt::Identity::new(age::secrecy::SecretString::from(pass.to_owned()));
    let reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|_| err("archive_locked", "口令不正确，或归档无法解密"))?;
    let mut plain = vec![];
    reader
        .take(MAX_PLAIN + 1)
        .read_to_end(&mut plain)
        .map_err(|_| err("invalid_archive", "归档解密或完整性校验失败"))?;
    if plain.len() as u64 > MAX_PLAIN {
        return Err(err("archive_limit", "归档展开后超过容量上限"));
    }
    parse(&plain)
}

fn validated_files(package: &Value) -> Result<Vec<Value>> {
    work::validate_package_files(package)
}

impl Engine {
    /// Resolve the frozen archive source for a read/inspect/import request.
    ///
    /// Accepts exactly one of:
    ///   - `job_id`: the archive inside this install's own state, matched against
    ///     the original receipt's frozen `archive_path`;
    ///   - `archive_path`: an explicit absolute path, so a package can be carried
    ///     to a different Lintel install that has no access to the original
    ///     job/state.
    /// The package is self-describing; read/inspect/import never consult the
    /// original inventory. A job-scoped archive additionally verifies the job
    /// path is the recorded one.
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
            let jid = safe_id(r, "job_id")?;
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

    /// Read a package, resolving either a job_id or an explicit absolute path.
    /// Job-scoped reads also confirm the recorded path matches the frozen one.
    fn archive_package(&self, r: &Value) -> Result<(Value, String, PathBuf)> {
        let path = self.archive_source(r)?;
        let recorded_digest = if let Some(jid) = r.get("job_id").and_then(Value::as_str) {
            let record = load(&self.path("jobs", jid))?;
            if record["archive_path"].as_str() != path.to_str() {
                return Err(err("archive_missing", "此任务的归档来源已经变化"));
            }
            record["archive_digest"]
                .as_str()
                .or_else(|| record["archive_intent_digest"].as_str())
                .map(str::to_owned)
        } else {
            None
        };
        let pass = work::check_passphrase(r)?;
        let (package, archive_digest) = work::read_package(&path, pass)?;
        validated_files(&package)?;
        if recorded_digest.is_some_and(|expected| expected != archive_digest) {
            return Err(err(
                "stale_archive",
                "归档字节与原任务记录不一致；请核对原任务和归档来源",
            ));
        }
        Ok((package, archive_digest, path))
    }

    pub(crate) fn archive_inspect(&self, r: &Value) -> Result<Value> {
        let (package, _, path) = self.archive_package(r)?;
        let files: Vec<Value> = validated_files(&package)?.iter().map(|f| json!({"path":f["path"],"category":f["category"],"bytes":f["data"].as_array().map_or(0,Vec::len),"digest":f["digest"]})).collect();
        Ok(
            json!({"job_id":r["job_id"],"archive_path":path,"created_at":package["created_at"],"generator":package["generator"].as_str(),"schema":package["schema"],"categories":category_summary(&files),"files":files,"notes":"指令可迁入原位置；会话与记忆保留在 lintel-imports，不自动激活 hooks/MCP，也不保证原会话可以续聊。generator 是包内自声明元数据，缺失时为 null，不是来源认证。此清单只描述包本身，不代表来源安装或来源 job 仍存在。"}),
        )
    }

    pub(crate) fn archive_read(&self, r: &Value) -> Result<Value> {
        let (package, _, _) = self.archive_package(r)?;
        let name = string(r, "path")?;
        let f = package["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["path"] == name)
            .ok_or_else(|| err("archive_file_missing", "归档没有该文件"))?;
        let bytes: Vec<u8> = serde_json::from_value(f["data"].clone())?;
        let end = bytes.len().min(1024 * 1024);
        Ok(
            json!({"path":name,"text":String::from_utf8_lossy(&bytes[..end]),"bytes":bytes.len(),"truncated":end<bytes.len()}),
        )
    }

    pub(crate) fn plan_import(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let (package, archive_digest, path) = self.archive_package(r)?;
        let cats = work::categories(r)?;
        let root = Path::new(string(&e, "root")?);
        let mut files = vec![];
        let selected: Vec<_> = validated_files(&package)?
            .into_iter()
            .filter(|file| cats.iter().any(|category| file["category"] == *category))
            .collect();
        for (f, relative) in selected.iter().zip(work::migration_paths(&selected)?) {
            let p = root.join(relative);
            work::preflight_import_parent(root, &p)?;
            if p.exists() {
                return Err(err(
                    "import_conflict",
                    "目标已有同名内容；请选择新的环境，已有文件未覆盖",
                ));
            }
            files.push(json!({"path":f["path"],"category":f["category"],"digest":f["digest"],"destination":p,"bytes":f["data"].as_array().unwrap().len()}));
        }
        if files.is_empty() {
            return Err(err("empty_import", "归档中没有选中类别的文件"));
        }
        self.plan(&e,"import","迁入选定工作内容",json!([]),vec!["目标环境现有文件","登录、hooks、MCP 与插件配置"],json!([{"id":"import","label":"解密并迁入所选类别，逐文件读回","reversible":false}]),json!({"original_job":r.get("job_id").cloned().unwrap_or(Value::Null),"archive_path":path,"archive_digest":archive_digest,"manifest":files,"categories":cats,"archive_passphrase_required":true}))
    }

    pub(crate) fn import_work(
        &self,
        e: &Value,
        p: &Value,
        r: &Value,
        j: &mut Value,
        journal: &Path,
    ) -> Result<()> {
        // The plan froze the exact archive source (job_id or explicit path) and
        // the encrypted digest. Re-read it at execution and re-verify.
        let source = if p["extra"]["original_job"].is_string() {
            json!({"job_id":p["extra"]["original_job"],"archive_passphrase":r["archive_passphrase"]})
        } else {
            json!({"archive_path":p["extra"]["archive_path"],"archive_passphrase":r["archive_passphrase"]})
        };
        let (package, archive_digest, _) = self.archive_package(&source)?;
        if archive_digest != p["extra"]["archive_digest"] {
            return Err(err("stale_archive", "预览后归档发生变化"));
        }
        let files = validated_files(&package)?;
        let root = Path::new(string(e, "root")?);
        let planned = p["extra"]["manifest"]
            .as_array()
            .ok_or_else(|| err("invalid_plan", "缺少迁入清单"))?;
        let selected: Vec<Value> = planned
            .iter()
            .map(|entry| {
                files
                    .iter()
                    .find(|file| file["path"] == entry["path"] && file["digest"] == entry["digest"])
                    .cloned()
                    .ok_or_else(|| err("stale_archive", "归档与计划不匹配"))
            })
            .collect::<Result<_>>()?;
        let targets: Vec<_> = work::migration_paths(&selected)?
            .into_iter()
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
        work::preflight_migration_paths(root, &work::migration_paths(&selected)?, j, journal)?;
        for ((entry, f), pth) in planned.iter().zip(&selected).zip(&targets) {
            work::migration_parent(pth.parent().unwrap())?;
            let bytes: Vec<u8> = serde_json::from_value(f["data"].clone())?;
            j["steps"].as_array_mut().unwrap().push(json!({"id":entry["path"],"label":entry["path"],"status":"executing","message":"正在发布并核验迁入文件；不替换已有内容。"}));
            save(journal, j)?;
            atomic_new(&pth, &bytes, 0o600).map_err(|error| {
                if error.code == "target_exists" {
                    err("import_conflict", "目标文件已出现；已有内容保持不变")
                } else {
                    error
                }
            })?;
            if digest(&read(&pth, 8 * 1024 * 1024)?) != f["digest"] {
                return Err(err("readback_failed", "迁入后读回不匹配"));
            }
            *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":entry["path"],"label":entry["path"],"status":"completed","message":"文件已迁入并校验；可执行配置未激活。"});
            save(journal, j)?;
        }
        j["status"] = json!("completed");
        j["completed_at"] = json!(now());
        Ok(())
    }
}

/// Per-category file count and byte total for an archive manifest. Derived only
/// from the package contents, so an external package reports its own coverage.
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

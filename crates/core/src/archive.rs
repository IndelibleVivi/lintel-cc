use crate::{err, now, safe_id, storage::*, string, work, Engine, Result};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

const MAX_PLAIN: u64 = 200 * 1024 * 1024;

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
    if package["schema"] != "lintel.work/1" {
        return Err(err(
            "archive_schema",
            "只支持 Lintel 工作内容包；状态备份不能自动迁入",
        ));
    }
    let files = package["files"]
        .as_array()
        .ok_or_else(|| err("invalid_archive", "归档缺少文件清单"))?;
    if files.len() > 10000 {
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
        if work::classify(path) != f["category"].as_str() || work::classify(path).is_none() {
            return Err(err("archive_category", "归档文件与批准的工作类别不符"));
        }
        let data: Vec<u8> = serde_json::from_value(f["data"].clone())?;
        total += data.len();
        if data.len() > 8 * 1024 * 1024 || total > 32 * 1024 * 1024 {
            return Err(err("archive_limit", "工作内容超过容量上限"));
        }
        if digest(&data) != f["digest"] {
            return Err(err("archive_integrity", "归档文件完整性校验失败"));
        }
    }
    Ok(files.clone())
}

fn target(root: &Path, f: &Value) -> Result<PathBuf> {
    let relative = Path::new(string(f, "path")?);
    let p = if f["category"] == "instructions" {
        root.join(relative)
    } else {
        root.join("lintel-imports").join(relative)
    };
    guard(&p)?;
    Ok(p)
}

impl Engine {
    fn archive_package(&self, r: &Value) -> Result<(Value, String)> {
        let jid = safe_id(r, "job_id")?;
        let job = load(&self.path("jobs", &jid))?;
        let path = self.state.join("archives").join(format!("{jid}.age"));
        if job["archive_path"].as_str() != path.to_str() {
            return Err(err("archive_missing", "此任务没有可读取的工作归档"));
        }
        let bytes = read(&path, MAX_PLAIN)?;
        let package = unseal(&bytes, work::check_passphrase(r)?)?;
        validated_files(&package)?;
        Ok((package, digest(&bytes)))
    }

    pub(crate) fn archive_inspect(&self, r: &Value) -> Result<Value> {
        let (package, _) = self.archive_package(r)?;
        let files: Vec<Value> = validated_files(&package)?.iter().map(|f| json!({"path":f["path"],"category":f["category"],"bytes":f["data"].as_array().map_or(0,Vec::len),"digest":f["digest"]})).collect();
        Ok(
            json!({"job_id":r["job_id"],"created_at":package["created_at"],"files":files,"notes":"指令可迁入原位置；会话与记忆保留在 lintel-imports，不自动激活 hooks/MCP，也不保证原会话可以续聊。"}),
        )
    }

    pub(crate) fn archive_read(&self, r: &Value) -> Result<Value> {
        let (package, _) = self.archive_package(r)?;
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
        let (package, archive_digest) = self.archive_package(r)?;
        let cats = work::categories(r)?;
        let root = Path::new(string(&e, "root")?);
        let mut files = vec![];
        for f in validated_files(&package)? {
            if !cats.iter().any(|c| f["category"] == *c) {
                continue;
            }
            let p = target(root, &f)?;
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
        self.plan(&e,"import","迁入选定工作内容",json!([]),vec!["目标环境现有文件","登录、hooks、MCP 与插件配置"],json!([{"id":"import","label":"解密并迁入所选类别，逐文件读回","reversible":false}]),json!({"original_job":r["job_id"],"archive_digest":archive_digest,"manifest":files,"categories":cats,"archive_passphrase_required":true}))
    }

    pub(crate) fn import_work(
        &self,
        e: &Value,
        p: &Value,
        r: &Value,
        j: &mut Value,
        journal: &Path,
    ) -> Result<()> {
        let (package, archive_digest) = self.archive_package(&json!({"job_id":p["extra"]["original_job"],"archive_passphrase":r["archive_passphrase"]}))?;
        if archive_digest != p["extra"]["archive_digest"] {
            return Err(err("stale_archive", "预览后归档发生变化"));
        }
        let files = validated_files(&package)?;
        let root = Path::new(string(e, "root")?);
        let planned = p["extra"]["manifest"]
            .as_array()
            .ok_or_else(|| err("invalid_plan", "缺少迁入清单"))?;
        let total: u64 = planned
            .iter()
            .map(|v| v["bytes"].as_u64().unwrap_or(0))
            .sum();
        if fs2::available_space(root)? < total + 1024 * 1024 {
            return Err(err("insufficient_space", "目标空间不足；未覆盖原文件"));
        }
        for entry in planned {
            if target(root, entry)?.exists() {
                return Err(err("import_conflict", "预览后出现同名内容；没有覆盖"));
            }
        }
        for entry in planned {
            let f = files
                .iter()
                .find(|f| f["path"] == entry["path"] && f["digest"] == entry["digest"])
                .ok_or_else(|| err("stale_archive", "归档与计划不匹配"))?;
            let pth = target(root, f)?;
            private_dir(pth.parent().unwrap())?;
            // create_new ensures a concurrent creator's file is never replaced.
            use std::os::unix::fs::OpenOptionsExt;
            let mut out = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW)
                .open(&pth)
                .map_err(|_| err("import_conflict", "目标文件无法独占建立，已有内容保持不变"))?;
            let bytes: Vec<u8> = serde_json::from_value(f["data"].clone())?;
            out.write_all(&bytes)?;
            out.sync_all()?;
            fs::File::open(pth.parent().unwrap())?.sync_all()?;
            if digest(&read(&pth, 8 * 1024 * 1024)?) != f["digest"] {
                return Err(err("readback_failed", "迁入后读回不匹配"));
            }
            j["steps"].as_array_mut().unwrap().push(json!({"id":entry["path"],"label":entry["path"],"status":"completed","message":"文件已迁入并校验；可执行配置未激活。"}));
            save(journal, j)?;
        }
        j["status"] = json!("completed");
        j["completed_at"] = json!(now());
        Ok(())
    }
}

//! Lintel's single filesystem plan/execution authority.
mod archive;
mod cleanup;
#[cfg(test)]
mod lifecycle_tests;
mod policy;
mod storage;
mod work;
use fs2::FileExt;
use policy::RULE;
use serde_json::{json, Value};
#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};
use std::{
    fs::{self, OpenOptions},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
use storage::*;

#[derive(Debug)]
pub struct Error {
    pub code: String,
    pub message: String,
}
pub type Result<T> = std::result::Result<T, Error>;
pub(crate) fn err(code: &str, message: &str) -> Error {
    Error {
        code: code.into(),
        message: message.into(),
    }
}
fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| err("invalid_request", &format!("缺少有效的 {key}")))
}
fn safe_id(v: &Value, key: &str) -> Result<String> {
    let s = string(v, key)?;
    uuid::Uuid::parse_str(s).map_err(|_| err("invalid_id", "目标 ID 格式无效"))?;
    Ok(s.into())
}
fn valstr(v: &Value) -> Value {
    if v.is_string() {
        v.clone()
    } else {
        Value::Null
    }
}
fn find_executable(home: &Path, path: Option<&std::ffi::OsStr>) -> Option<String> {
    // Non-interactive SSH need not inherit the user's ~/.local/bin PATH.
    // PATH keeps its precedence; the native install fallback is this user's only.
    let candidates = path
        .into_iter()
        .flat_map(std::env::split_paths)
        .map(|d| d.join("claude"))
        .chain(std::iter::once(home.join(".local/bin/claude")));
    candidates
        .filter(|p| {
            fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
        .find_map(|p| p.canonicalize().ok())
        .map(|p| p.to_string_lossy().into_owned())
}
fn physical(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(err("invalid_path", "需要不含父目录跳转的绝对路径"));
    }
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(err("symlink_target", "请选择实际目录，不使用符号链接"));
    }
    Ok(path.canonicalize()?)
}

pub struct Engine {
    home: PathBuf,
    state: PathBuf,
    accept_hook: Option<fn(&Value)>,
}
impl Engine {
    pub fn new(home: PathBuf, state: PathBuf) -> Result<Self> {
        let home = physical(&home)?;
        if !state.exists() {
            fs::create_dir_all(&state)?;
        }
        let state = physical(&state)?;
        private_dir(&state)?;
        for name in ["plans", "jobs", "environments", "archives", "baselines"] {
            private_dir(&state.join(name))?;
        }
        Ok(Self {
            home,
            state,
            accept_hook: None,
        })
    }
    fn executable(&self) -> Option<String> {
        find_executable(&self.home, std::env::var_os("PATH").as_deref())
    }
    fn path(&self, dir: &str, id: &str) -> PathBuf {
        self.state.join(dir).join(format!("{id}.json"))
    }
    pub fn request(&self, r: Value) -> Value {
        let result = (|| {
            let lock_path = self.state.join("operation.lock");
            guard(&lock_path)?;
            let lock = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW)
                .open(lock_path)?;
            if lock.try_lock_exclusive().is_err() {
                match r["command"].as_str() {
                    Some("jobs") => return self.dispatch(&r),
                    Some("job") => {
                        let key = if r.get("job_id").is_some() {
                            "job_id"
                        } else {
                            "plan_id"
                        };
                        let jid = safe_id(&r, key)?;
                        let path = self.path("jobs", &jid);
                        if !path.exists() {
                            return Err(err("job_not_found", "任务尚未持久接收"));
                        }
                        return load(&path); // Atomic journal snapshots while the actual writer owns the lock.
                    }
                    _ => {
                        return Err(err(
                            "target_busy",
                            "另一个 Lintel 操作正在执行，请查询原任务",
                        ))
                    }
                }
            }
            let response = self.dispatch(&r);
            // Release explicitly: a concurrent subprocess fork may briefly retain
            // the open file description until exec closes inherited descriptors.
            fs2::FileExt::unlock(&lock)?;
            response
        })();
        match result {
            Ok(data) => json!({"ok":true,"data":data}),
            Err(e) => json!({"ok":false,"error":{"code":e.code,"message":e.message}}),
        }
    }
    fn inventory(&self) -> Result<Vec<Value>> {
        let p = self.state.join("inventory.json");
        if !p.exists() {
            return Ok(vec![]);
        }
        load(&p)?
            .as_array()
            .cloned()
            .ok_or_else(|| err("invalid_inventory", "环境登记文件损坏"))
    }
    fn env(&self, r: &Value) -> Result<Value> {
        let eid = safe_id(r, "environment_id")?;
        self.inventory()?
            .into_iter()
            .find(|e| e["id"] == eid)
            .ok_or_else(|| err("environment_missing", "没有找到此环境"))
    }
    fn register(&self, name: &str, root: &Path, owned: bool) -> Result<Value> {
        let root = physical(root)?;
        guard(&root)?;
        if !root.is_dir() || root == Path::new("/") || root == self.home {
            return Err(err("invalid_root", "请选择专用配置目录"));
        }
        let mut all = self.inventory()?;
        if let Some(e) = all
            .iter()
            .find(|e| e["root"] == root.to_string_lossy().as_ref())
        {
            return Ok(e.clone());
        }
        let env = policy::environment(
            json!({"id":id(),"name":name,"host":"local","surface":"claude-code","root":root,"ownership":if owned{"lintel"}else{"registered"},"status":"discovered","credential_scope":"unverified"}),
            self.executable(),
        );
        all.push(env.clone());
        save(&self.state.join("inventory.json"), &json!(all))?;
        Ok(env)
    }
    fn create(&self, name: &str) -> Result<Value> {
        let root = self.state.join("environments").join(id());
        private_dir(&root)?;
        self.register(name, &root, true)
    }
    fn settings(&self, e: &Value) -> Result<(PathBuf, Value, Value)> {
        let root = PathBuf::from(string(e, "root")?);
        guard(&root)?;
        let path = root.join("settings.json");
        let snap = snapshot(&path)?;
        let doc = if snap.is_null() {
            json!({})
        } else {
            load(&path)?
        };
        if !doc.is_object() || doc.get("env").is_some_and(|x| !x.is_object()) {
            return Err(err(
                "unsupported_settings",
                "settings.json 或 env 必须是 JSON 对象；原文件未改动",
            ));
        }
        for (key, _) in policy::fields() {
            if doc["env"].get(key).is_some_and(|v| !v.is_string()) {
                return Err(err(
                    "unsupported_settings",
                    "外发设置包含非字符串值，需要先修复该来源",
                ));
            }
        }
        Ok((path, doc, snap))
    }
    fn warnings(&self, e: &Value) -> Vec<String> {
        let mut out=vec!["这里只检查已登记 root 的 user settings；项目、组织策略、IDE 与已有进程的生效状态尚未确认。".into(),"独立配置目录不是 OS sandbox；某类 Console 登录位于目录外。".into()];
        if e["executable"].is_null() {
            out.push("没有在 PATH 中找到 Claude Code；可先准备环境，安装后重新发现。".into())
        }
        out
    }
    fn block_managed(&self, e: &Value) -> Result<()> {
        let root = PathBuf::from(string(e, "root")?);
        let candidates = [
            root.join("managed-settings.json"),
            self.home.join(".claude/managed-settings.json"),
            PathBuf::from("/Library/Application Support/ClaudeCode/managed-settings.json"),
            PathBuf::from("/etc/claude-code/managed-settings.json"),
        ];
        if candidates.iter().any(|p| p.exists()) {
            return Err(err(
                "managed_settings",
                "发现组织管理配置；当前适配器不能可靠解释控制权，未生成覆盖操作",
            ));
        }
        Ok(())
    }
    fn plan(
        &self,
        e: &Value,
        kind: &str,
        title: &str,
        changes: Value,
        preserves: Vec<&str>,
        actions: Value,
        extra: Value,
    ) -> Result<Value> {
        // Freeze the raw settings bytes and root identity without parsing:
        // plans that never write settings (rebuild/cleanup/import) must not be
        // gated on a parseable settings file. Settings-writing plans parse
        // separately in their dispatch arm before calling this.
        let root = PathBuf::from(string(e, "root")?);
        guard(&root)?;
        let snap = snapshot(&root.join("settings.json"))?;
        let rm = fs::metadata(&root)?;
        use std::os::unix::fs::MetadataExt;
        let mut warnings = self.warnings(e);
        if extra["policy"]["keep_remote_control"] == true {
            warnings.push(
                extra["policy"]["remote_control"]["summary"]
                    .as_str()
                    .unwrap()
                    .into(),
            );
        }
        if extra["policy"].is_object()
            && changes
                .as_array()
                .is_some_and(|v| v.iter().any(|change| change["after"].is_null()))
        {
            warnings.push("仅删除预览中显式选择的 user settings 字段；删除后此来源不再提供该开关，不保证功能开启。外部 shell/项目/组织配置不受修改，已有进程中的变量需重新启动才会重新读取。".into());
        }
        let mut p = json!({"id":id(),"environment_id":e["id"],"kind":kind,"title":title,"changes":changes,"preserves":preserves,"warnings":warnings,"actions":actions,"created_at":now(),"status":"planned","rule_version":RULE,"root":e["root"],"root_identity":[rm.dev(),rm.ino()],"snapshot":snap,"extra":extra});
        p["hash"] = json!(digest(&serde_json::to_vec(&p)?));
        save(&self.path("plans", string(&p, "id")?), &p)?;
        Ok(public_plan(p))
    }
    fn dispatch(&self, r: &Value) -> Result<Value> {
        match string(r, "command")? {
            "discover" => {
                let root = self.home.join(".claude");
                if root.is_dir() {
                    let _ = self.register("Claude Code · 默认环境", &root, false);
                }
                let mut all = self.inventory()?;
                let exe = self.executable();
                for e in &mut all {
                    *e = policy::environment(e.clone(), exe.clone());
                }
                save(&self.state.join("inventory.json"), &json!(all))?;
                Ok(
                    json!({"environments":all,"capabilities":[{"name":"Claude Code 配置","status":"available","reason":"精确字段预览、读回与恢复；运行时效果另行验证"},{"name":"工作内容迁入","status":"available","reason":"加密归档与选择性迁入；不会注销共享凭据"},{"name":"浏览器伴随扩展","status":"separate_module","reason":"需要安装并配对对应 profile"},{"name":"网络强约束","status":"not_delivered","reason":"未安装或验证平台高权限组件"},{"name":"Claude Desktop / IDE / service","status":"not_delivered","reason":"当前不修改这些入口"}]}),
                )
            }
            "register" => self.register(string(r, "name")?, Path::new(string(r, "root")?), false),
            "create_environment" => self.create(string(r, "name")?),
            "inspect" => {
                let e = policy::environment(self.env(r)?, self.executable());
                let (path, doc, _) = self.settings(&e)?;
                let settings:Vec<Value>=policy::fields().map(|(key,label)|json!({"key":key,"label":label,"value":valstr(&doc["env"][key]),"source":path,"effect_timing":"next_launch","runtime_verified":false,"status":policy::setting_status(key,&doc["env"][key])})).collect();
                let assets = work::summaries(Path::new(string(&e, "root")?))?;
                Ok(
                    json!({"environment":e,"settings":settings,"assets":assets,"policy":policy::assessment(&doc,&e["product_evidence"],policy::trusted_devices(r)?),"warnings":self.warnings(&e)}),
                )
            }
            "plan_policy" => {
                let e = policy::environment(self.env(r)?, self.executable());
                self.block_managed(&e)?;
                let (path, doc, _) = self.settings(&e)?;
                let preset = string(r, "preset")?;
                let (changes, assessment) = policy::plan(&doc, &e["product_evidence"], r, &path)?;
                let p = self.plan(
                    &e,
                    "policy",
                    match preset {
                        "reduce" => "减少外发",
                        "custom" => "自定义保护",
                        _ => "保持功能",
                    },
                    changes,
                    vec![
                        "登录与凭据",
                        "会话、指令与记忆",
                        "更新与 WebFetch 安全检查",
                        "通用代理与自设 OTel",
                    ],
                    json!([{"id":"settings","label":"应用明确的外发设置并读回","reversible":true}]),
                    json!({"preset":preset,"product":e["product_evidence"],"policy":assessment}),
                )?;
                Ok(p)
            }
            "plan_restore" => {
                let jid = safe_id(r, "job_id")?;
                let job = load(&self.path("jobs", &jid))?;
                if job["restorable"] != true {
                    return Err(err("not_restorable", "此任务没有可恢复的配置改动"));
                }
                let old = load(&self.path("plans", string(&job, "plan_id")?))?;
                if !old.is_object() || !old["hash"].is_string() {
                    return Err(err("invalid_plan", "保存的计划损坏，无法据此恢复"));
                }
                let mut unhashed = old.clone();
                unhashed.as_object_mut().unwrap().remove("hash");
                if old["hash"] != digest(&serde_json::to_vec(&unhashed)?) {
                    return Err(err("plan_changed", "保存的计划已变化，无法据此恢复"));
                }
                let e = self.env(&json!({"environment_id":old["environment_id"]}))?;
                let (path, doc, _) = self.settings(&e)?;
                let mut changes = vec![];
                for c in old["changes"]
                    .as_array()
                    .ok_or_else(|| err("invalid_plan", "计划损坏"))?
                {
                    let k = string(c, "key")?;
                    if valstr(&doc["env"][k]) != c["after"] {
                        return Err(err(
                            "restore_conflict",
                            "相关设置在任务后被修改，保留后续编辑；请重新查看来源并选择方案",
                        ));
                    }
                    changes.push(json!({"key":k,"label":c["label"],"before":c["after"],"after":c["before"],"path":path}));
                }
                self.plan(
                    &e,
                    "restore",
                    "恢复配置",
                    json!(changes),
                    vec!["任务后修改的其他字段", "当前登录与工作内容"],
                    json!([{"id":"settings","label":"恢复仍属于本工具的字段","reversible":true}]),
                    json!({"original_job":jid}),
                )
            }
            "plan_reset" => {
                let e = self.env(r)?;
                if string(r, "recipe")? != "rebuild" {
                    return Err(err("unsupported_recipe", "当前支持新环境重建配方"));
                }
                let categories = work::categories(r)?;
                let manifest = work::manifest(Path::new(string(&e, "root")?), &categories)?;
                self.plan(&e,"rebuild","保留内容，准备新环境",json!([]),vec!["原环境全部内容（尚未注销或删除）","未选中的实例与项目文件"],json!([{"id":"archive","label":"加密归档选中的工作内容","reversible":false},{"id":"create","label":"创建新的配置目录","reversible":false},{"id":"migrate","label":"选择性迁入；不启用 hooks/MCP","reversible":false},{"id":"credentials","label":"旧登录及客户端状态尚需独立处理","reversible":false}]),json!({"categories":categories,"manifest":manifest,"archive_passphrase_required":true}))
            }
            "cleanup_inspect" => self.cleanup_inspect(r),
            "auth_probe" => self.auth_probe(r),
            "plan_cleanup" => self.plan_cleanup(r),
            "reactivate_environment" => {
                let e = self.env(r)?;
                let mut all = self.inventory()?;
                for item in &mut all {
                    if item["id"] == e["id"] {
                        item["status"] = json!("discovered");
                    }
                }
                save(&self.state.join("inventory.json"), &json!(all))?;
                Ok(json!({"status":"reactivated","environment_id":e["id"]}))
            }
            "archive_inspect" => self.archive_inspect(r),
            "archive_read" => self.archive_read(r),
            "plan_import" => self.plan_import(r),
            "execute" => self.execute(r),
            "jobs" => {
                let mut jobs = vec![];
                for f in fs::read_dir(self.state.join("jobs"))? {
                    let p = f?.path();
                    if p.extension().is_some_and(|v| v == "json") {
                        jobs.push(load(&p)?);
                    }
                }
                jobs.sort_by(|a, b| b["created_at"].as_str().cmp(&a["created_at"].as_str()));
                Ok(json!({"jobs":jobs}))
            }
            "job" => {
                let key = if r.get("job_id").is_some() {
                    "job_id"
                } else {
                    "plan_id"
                };
                let jid = safe_id(r, key)?;
                let p = self.path("jobs", &jid);
                if !p.exists() {
                    return Err(err("job_not_found", "任务尚未持久接收；没有执行证据"));
                }
                let mut j = load(&p)?;
                if ["accepted", "executing", "verifying"]
                    .iter()
                    .any(|s| j["status"] == *s)
                {
                    j["status"] = json!("needs_reconciliation");
                    j["warnings"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!("先前执行被中断；不要重试破坏性步骤。"));
                    save(&p, &j)?;
                }
                Ok(j)
            }
            "drift" => {
                let e = self.env(r)?;
                let (_, doc, _) = self.settings(&e)?;
                let p = self.path("baselines", string(&e, "id")?);
                if !p.exists() {
                    return Ok(json!({"changes":[],"status":"no_baseline"}));
                }
                let base = load(&p)?;
                let mut changes = vec![];
                for (key, label) in policy::fields() {
                    if base.get(key).is_some() && base[key] != valstr(&doc["env"][key]) {
                        changes.push(json!({"key":key,"label":label,"value":valstr(&doc["env"][key]),"source":"user_settings","effect_timing":"next_launch","status":"changed"}))
                    }
                }
                Ok(
                    json!({"status":if changes.is_empty(){"unchanged"}else{"changed"},"changes":changes}),
                )
            }
            "accept_drift" => {
                let e = self.env(r)?;
                self.baseline(&e)?;
                Ok(json!({"status":"accepted"}))
            }
            "launch_context" => {
                let e = self.env(r)?;
                if e["status"] == "retired" {
                    return Err(err("environment_retired", "请先恢复此环境登记再启动"));
                }
                let root = PathBuf::from(string(&e, "root")?);
                guard(&root)?;
                let exe = self
                    .executable()
                    .ok_or_else(|| err("executable_missing", "没有找到 Claude Code"))?;
                Ok(json!({"root":root,"executable":exe}))
            }
            "launch" => self.launch(&self.env(r)?, r.get("proxy_url").and_then(Value::as_str)),
            "export_support" => {
                let all = self.inventory()?;
                Ok(
                    json!({"product":"Lintel","version":env!("CARGO_PKG_VERSION"),"platform":std::env::consts::OS,"architecture":std::env::consts::ARCH,"rule_version":RULE,"environment_count":all.len(),"capabilities":["user_settings","field_restore","encrypted_archive","new_root"],"omitted":["paths","names","accounts","settings values","conversation content","network destinations"],"uploaded":false}),
                )
            }
            _ => Err(err("unknown_command", "此版本不支持该操作")),
        }
    }
    fn baseline(&self, e: &Value) -> Result<()> {
        let (_, doc, _) = self.settings(e)?;
        let mut b = json!({});
        for (key, _) in policy::fields() {
            b[key] = valstr(&doc["env"][key])
        }
        save(&self.path("baselines", string(e, "id")?), &b)
    }
    fn execute(&self, r: &Value) -> Result<Value> {
        let pid = safe_id(r, "plan_id")?;
        let p = load(&self.path("plans", &pid))?;
        if !p.is_object() || !p["hash"].is_string() {
            return Err(err("invalid_plan", "保存的计划损坏，请重新预览"));
        }
        if r["approval"] != p["hash"] {
            return Err(err("approval_mismatch", "授权与当前预览不一致"));
        }
        let mut unhashed = p.clone();
        unhashed.as_object_mut().unwrap().remove("hash");
        if p["hash"] != digest(&serde_json::to_vec(&unhashed)?) {
            return Err(err("plan_changed", "保存的计划已变化，需要重新预览"));
        }
        let jp = self.path("jobs", &pid);
        if jp.exists() {
            return self.dispatch(&json!({"command":"job","job_id":pid}));
        }
        let e = self.env(&json!({"environment_id":p["environment_id"]}))?;
        if e["root"] != p["root"] {
            return Err(err("stale_plan", "环境目标已经变化"));
        }
        let root = PathBuf::from(string(&e, "root")?);
        guard(&root)?;
        use std::os::unix::fs::MetadataExt;
        let m = fs::metadata(&root)?;
        if json!([m.dev(), m.ino()]) != p["root_identity"] {
            return Err(err("stale_plan", "配置目录的实际对象已变化"));
        }
        // Preconditions follow the plan's actual write scope. Plans that never
        // touch settings (rebuild/cleanup/import) freeze and compare the raw
        // settings bytes; an unparseable settings file must not block them.
        // Settings-writing plans still require a fully parsed document.
        let (path, mut doc, snap) = match p["kind"].as_str() {
            Some("rebuild" | "cleanup" | "import") => {
                let path = root.join("settings.json");
                let snap = snapshot(&path)?;
                (path, Value::Null, snap)
            }
            _ => self.settings(&e)?,
        };
        if snap != p["snapshot"] {
            return Err(err("stale_plan", "预览后配置被修改，请重新预览"));
        }
        if p["kind"] == "policy" {
            if p["rule_version"] != RULE {
                return Err(err("rule_changed", "策略规则已更新，请重新预览"));
            }
            if p["extra"]["product"] != policy::product(self.executable().as_deref()) {
                return Err(err(
                    "stale_product",
                    "目标程序或产品版本在预览后改变，请重新预览",
                ));
            }
        }
        if p["kind"] == "rebuild" {
            work::check_passphrase(r)?;
            let manifest = work::manifest(&root, &work::categories(&p["extra"])?)?;
            if json!(manifest) != p["extra"]["manifest"] {
                return Err(err("stale_plan", "预览后工作内容发生变化，请重新预览"));
            }
        } else if p["kind"] == "cleanup" {
            self.block_managed(&e)?;
            self.check_cleanup(&e, &p, r)?;
        } else if p["kind"] == "import" {
            work::check_passphrase(r)?;
        } else {
            self.block_managed(&e)?;
        }
        let mut j = json!({"id":pid,"plan_id":pid,"environment_id":e["id"],"title":p["title"],"status":"accepted","created_at":now(),"restorable":false,"warnings":p["warnings"],"steps":[]});
        if p["extra"]["policy"].is_object() {
            j["policy"] = p["extra"]["policy"].clone();
        }
        save(&jp, &j)?;
        if let Some(hook) = self.accept_hook {
            hook(&j);
        }
        j["status"] = json!("executing");
        save(&jp, &j)?;
        let result = (|| -> Result<()> {
            if p["kind"] == "rebuild" {
                self.rebuild(&e, &p, r, &mut j, &jp)?;
                return Ok(());
            }
            if p["kind"] == "cleanup" {
                return self.cleanup(&e, &p, r, &mut j, &jp);
            }
            if p["kind"] == "import" {
                return self.import_work(&e, &p, r, &mut j, &jp);
            }
            if p["kind"] == "policy"
                && p["extra"]["policy"]["preset"] == "custom"
                && p["changes"].as_array().is_some_and(Vec::is_empty)
            {
                self.baseline(&e)?;
                j["steps"] = json!([{"id":"settings","label":"精确字段检查","status":"completed","message":"所选字段无需修改；原文件和权限保持不变，运行效果尚未验证。"}]);
                j["status"] = json!("completed");
                return Ok(());
            }
            if doc.get("env").is_none() {
                doc["env"] = json!({});
            }
            for c in p["changes"]
                .as_array()
                .ok_or_else(|| err("invalid_plan", "计划变更列表缺失"))?
            {
                let key = string(c, "key")?;
                if !policy::fields().any(|(k, _)| k == key) {
                    return Err(err("invalid_action", "计划包含不受支持的字段"));
                }
                if c["after"].is_null() {
                    doc["env"].as_object_mut().unwrap().remove(key);
                } else {
                    doc["env"][key] = c["after"].clone();
                }
            }
            if snapshot(&path)? != snap {
                return Err(err("stale_plan", "写入前发现配置变化"));
            }
            let mode = if path.exists() {
                // Never preserve a group/other-writable mode on rewrite; that
                // would keep the file open to unsupervised local writers.
                fs::metadata(&path)?.permissions().mode() & 0o777 & !0o022
            } else {
                0o600
            };
            atomic(&path, &serde_json::to_vec_pretty(&doc)?, mode)?;
            j["status"] = json!("verifying");
            j["restorable"] = json!(true);
            save(&jp, &j)?;
            if load(&path)? != doc {
                return Err(err("readback_failed", "写后读回不同，请核对其他写入者"));
            }
            self.baseline(&e)?;
            j["steps"] = json!([{"id":"settings","label":"精确字段写入与读回","status":"completed","message":"配置已写入；新启动及网络效果尚未验证。"}]);
            j["status"] = json!("completed");
            Ok(())
        })();
        if let Err(failure) = result {
            j["status"] = json!("needs_reconciliation");
            j["warnings"]
                .as_array_mut()
                .unwrap()
                .push(json!(failure.message));
        }
        save(&jp, &j)?;
        Ok(j)
    }
    fn launch(&self, e: &Value, proxy_url: Option<&str>) -> Result<Value> {
        if e["status"] == "retired" {
            return Err(err("environment_retired", "请先恢复此环境登记再启动"));
        }
        let route = if let Some(url) = proxy_url {
            let raw = url
                .strip_prefix("http://")
                .ok_or_else(|| err("invalid_proxy", "只接受 loopback HTTP 通道"))?;
            let addr: std::net::SocketAddr = raw
                .parse()
                .map_err(|_| err("invalid_proxy", "通道地址无效"))?;
            if !addr.ip().is_loopback() || addr.port() == 0 {
                return Err(err("invalid_proxy", "只接受已监听的 loopback 通道"));
            }
            Some(format!("http://{addr}"))
        } else {
            None
        };
        let exe = self.executable().ok_or_else(|| {
            err(
                "executable_missing",
                "没有找到 Claude Code，请安装后重新检查",
            )
        })?;
        let root = PathBuf::from(string(e, "root")?);
        guard(&root)?;
        // A terminal launcher must not silently run in a hidden pipe or feed an agent prompt.
        #[cfg(target_os = "macos")]
        {
            let path = self
                .state
                .join(format!("launch-{}.command", string(e, "id")?));
            let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
            let proxy_args = route
                .as_ref()
                .map(|url| {
                    format!(
                        " HTTP_PROXY={} HTTPS_PROXY={} http_proxy={} https_proxy={}",
                        quote(url),
                        quote(url),
                        quote(url),
                        quote(url)
                    )
                })
                .unwrap_or_default();
            let content = format!(
                "#!/bin/sh\ncd {} || exit 1\nexec env CLAUDE_CONFIG_DIR={}{} {}\n",
                quote(&root.to_string_lossy()),
                quote(&root.to_string_lossy()),
                proxy_args,
                quote(&exe)
            );
            atomic(&path, content.as_bytes(), 0o700)?;
            let status = Command::new("/usr/bin/open")
                .arg("-a")
                .arg("Terminal")
                .arg(&path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;
            if !status.success() {
                return Err(err("launch_failed", "Terminal 未接受启动请求"));
            }
            Ok(
                json!({"status":"launch_requested","message":"已请求在 Terminal 打开指定配置目录；未声称当前进程或凭据完全隔离。"}),
            )
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (exe, route);
            Ok(
                json!({"status":"terminal_required","message":"在交互终端运行 lintel launch <环境ID>。"}),
            )
        }
    }
}
fn public_plan(mut p: Value) -> Value {
    if p["extra"]["policy"].is_object() {
        p["policy"] = p["extra"]["policy"].clone();
    }
    for key in ["snapshot", "root_identity", "root"] {
        p.as_object_mut().unwrap().remove(key);
    }
    if p["extra"]["archive_passphrase_required"] == true {
        p["archive_passphrase_required"] = json!(true);
        p["file_count"] = json!(p["extra"]["manifest"].as_array().map_or(0, Vec::len));
    }
    p.as_object_mut().unwrap().remove("extra");
    p
}

pub fn handle_request(request: Value) -> Value {
    handle_request_inner(request, None)
}
/// Runner-only callback after durable acceptance; never a second execution path.
pub fn handle_request_with_accept(request: Value, hook: fn(&Value)) -> Value {
    handle_request_inner(request, Some(hook))
}
fn handle_request_inner(request: Value, hook: Option<fn(&Value)>) -> Value {
    let result = (|| {
        let home = std::env::var_os("LINTEL_TEST_HOME")
            .or_else(|| std::env::var_os("HOME"))
            .ok_or_else(|| err("home_missing", "找不到用户目录"))?;
        let home = PathBuf::from(home);
        let state = std::env::var_os("LINTEL_STATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                if cfg!(target_os = "macos") {
                    home.join("Library/Application Support/Lintel")
                } else {
                    home.join(".local/share/lintel")
                }
            });
        Engine::new(home, state)
    })();
    match result {
        Ok(mut engine) => {
            engine.accept_hook = hook;
            engine.request(request)
        }
        Err(e) => json!({"ok":false,"error":{"code":e.code,"message":e.message}}),
    }
}
pub fn decode_request(bytes: &[u8]) -> Result<Value> {
    parse(bytes)
}

pub fn parse_request(bytes: &[u8]) -> Value {
    match parse(bytes) {
        Ok(v) => handle_request(v),
        Err(e) => json!({"ok":false,"error":{"code":e.code,"message":e.message}}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    fn setup() -> (TempDir, Engine, Value, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let home = base.join("home");
        fs::create_dir(&home).unwrap();
        let root = home.join("cc");
        fs::create_dir(&root).unwrap();
        let engine = Engine::new(home, base.join("state")).unwrap();
        let e = engine.register("fixture", &root, false).unwrap();
        (temp, engine, e, root)
    }
    fn plan(engine: &Engine, e: &Value) -> Value {
        engine.request(json!({"command":"plan_policy","environment_id":e["id"],"preset":"reduce","keep_remote_control":false}))["data"].clone()
    }
    #[test]
    fn duplicate_json_kept() {
        let (_t, engine, e, root) = setup();
        let raw = r#"{"env":{"DISABLE_TELEMETRY":"0","DISABLE_TELEMETRY":"1"}}"#;
        fs::write(root.join("settings.json"), raw).unwrap();
        let r = engine
            .request(json!({"command":"plan_policy","environment_id":e["id"],"preset":"reduce"}));
        assert_eq!(r["error"]["code"], "invalid_json");
        assert_eq!(fs::read_to_string(root.join("settings.json")).unwrap(), raw);
    }
    #[test]
    fn approval_stale_and_field_restore() {
        let (_t, engine, e, root) = setup();
        fs::write(
            root.join("settings.json"),
            r#"{"env":{"UNRELATED":"keep"},"permissions":{"allow":["Read"]}}"#,
        )
        .unwrap();
        let p = plan(&engine, &e);
        let bad = engine.request(json!({"command":"execute","plan_id":p["id"],"approval":"no"}));
        assert_eq!(bad["error"]["code"], "approval_mismatch");
        let j = engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}));
        assert_eq!(j["data"]["status"], "completed", "{j}");
        let mut d = load(&root.join("settings.json")).unwrap();
        d["new_field"] = json!("later");
        save(&root.join("settings.json"), &d).unwrap();
        let restore = engine.request(json!({"command":"plan_restore","job_id":p["id"]}));
        assert_eq!(restore["ok"], true, "{restore}");
        let rp = &restore["data"];
        let result =
            engine.request(json!({"command":"execute","plan_id":rp["id"],"approval":rp["hash"]}));
        assert_eq!(result["data"]["status"], "completed");
        let d = load(&root.join("settings.json")).unwrap();
        assert_eq!(d["new_field"], "later");
        assert!(d["env"].get("DISABLE_TELEMETRY").is_none());
        assert_eq!(d["env"]["UNRELATED"], "keep");
    }
    #[test]
    fn restore_does_not_overwrite_new_owner() {
        let (_t, engine, e, root) = setup();
        let p = plan(&engine, &e);
        engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}));
        let mut d = load(&root.join("settings.json")).unwrap();
        d["env"]["DISABLE_TELEMETRY"] = json!("external");
        save(&root.join("settings.json"), &d).unwrap();
        let r = engine.request(json!({"command":"plan_restore","job_id":p["id"]}));
        assert_eq!(r["error"]["code"], "restore_conflict");
        assert_eq!(load(&root.join("settings.json")).unwrap(), d);
    }
    #[test]
    fn staleness_prevents_execution() {
        let (_t, engine, e, root) = setup();
        let p = plan(&engine, &e);
        fs::write(root.join("settings.json"), "{\"changed\":true}").unwrap();
        let r = engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}));
        assert_eq!(r["error"]["code"], "stale_plan");
    }
    #[test]
    fn custom_apply_receipt_and_restore_preserve_unrelated_values() {
        let (_t, engine, e, root) = setup();
        let path = root.join("settings.json");
        let original = json!({"env":{
            "DISABLE_TELEMETRY":"false", "DISABLE_ERROR_REPORTING":"0",
            "DISABLE_FEEDBACK_COMMAND":"false", "CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY":"原字节值",
            "DO_NOT_TRACK":"off", "DISABLE_GROWTHBOOK":"yes", "UNRELATED":"keep"
        },"permissions":{"allow":["Read"]},"hooks":{"synthetic":"keep"}});
        save(&path, &original).unwrap();
        let choices = json!({"DISABLE_TELEMETRY":"disable","DISABLE_ERROR_REPORTING":"disable",
            "DISABLE_FEEDBACK_COMMAND":"disable","DO_NOT_TRACK":"remove","DISABLE_GROWTHBOOK":"remove"});
        let response = engine.request(json!({"command":"plan_policy","environment_id":e["id"],"preset":"custom","custom_settings":choices}));
        assert_eq!(response["ok"], true, "{response}");
        let p = &response["data"];
        assert_eq!(p["title"], "自定义保护");
        assert_eq!(
            load(&path).unwrap(),
            original,
            "preview must not mutate settings"
        );
        let j = engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}));
        assert_eq!(j["data"]["status"], "completed", "{j}");
        assert_eq!(j["data"]["policy"], p["policy"]);
        assert_eq!(j["data"]["policy"]["custom_settings"], choices);
        let mut expected = original.clone();
        expected["env"]["DISABLE_FEEDBACK_COMMAND"] = json!("1");
        expected["env"]
            .as_object_mut()
            .unwrap()
            .remove("DO_NOT_TRACK");
        expected["env"]
            .as_object_mut()
            .unwrap()
            .remove("DISABLE_GROWTHBOOK");
        assert_eq!(load(&path).unwrap(), expected);
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        expected["env"]["UNRELATED_LATER"] = json!("preserve");
        save(&path, &expected).unwrap();
        let restored = engine.request(json!({"command":"plan_restore","job_id":p["id"]}));
        assert_eq!(restored["ok"], true, "{restored}");
        let rp = &restored["data"];
        let result =
            engine.request(json!({"command":"execute","plan_id":rp["id"],"approval":rp["hash"]}));
        assert_eq!(result["data"]["status"], "completed", "{result}");
        let mut final_doc = original;
        final_doc["env"]["UNRELATED_LATER"] = json!("preserve");
        assert_eq!(load(&path).unwrap(), final_doc);
    }
    #[test]
    fn custom_noop_keeps_original_bytes_mode_and_drift_baseline() {
        let (_t, engine, e, root) = setup();
        let path = root.join("settings.json");
        let raw = b"{ \"env\": { \"DISABLE_TELEMETRY\": \"false\", \"UNRELATED\": \"keep\" } }\n";
        fs::write(&path, raw).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        let response = engine.request(
            json!({"command":"plan_policy","environment_id":e["id"],"preset":"custom",
            "custom_settings":{"DISABLE_TELEMETRY":"disable","DO_NOT_TRACK":"remove"}}),
        );
        assert_eq!(response["ok"], true, "{response}");
        let p = &response["data"];
        assert!(p["changes"].as_array().unwrap().is_empty());
        let result =
            engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}));
        assert_eq!(result["data"]["status"], "completed", "{result}");
        assert_eq!(result["data"]["restorable"], false);
        assert_eq!(fs::read(&path).unwrap(), raw);
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
        assert_eq!(
            engine.request(json!({"command":"drift","environment_id":e["id"]}))["data"]["status"],
            "unchanged"
        );
        let mut doc = load(&path).unwrap();
        doc["env"]["DISABLE_TELEMETRY"] = json!("");
        save(&path, &doc).unwrap();
        assert_eq!(
            engine.request(json!({"command":"drift","environment_id":e["id"]}))["data"]["status"],
            "changed"
        );
        let absent = engine.create("absent settings").unwrap();
        let response = engine.request(
            json!({"command":"plan_policy","environment_id":absent["id"],"preset":"custom"}),
        );
        let p = &response["data"];
        let result =
            engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}));
        assert_eq!(result["data"]["status"], "completed", "{result}");
        assert!(!Path::new(absent["root"].as_str().unwrap())
            .join("settings.json")
            .exists());
    }
    #[test]
    fn policy_plan_from_old_rule_requires_new_preview() {
        let (_t, engine, e, root) = setup();
        let p = plan(&engine, &e);
        let path = engine.path("plans", string(&p, "id").unwrap());
        let mut saved = load(&path).unwrap();
        saved["rule_version"] = json!("claude-privacy-v2-2026-10-03");
        saved.as_object_mut().unwrap().remove("hash");
        saved["hash"] = json!(digest(&serde_json::to_vec(&saved).unwrap()));
        save(&path, &saved).unwrap();
        let response = engine
            .request(json!({"command":"execute","plan_id":saved["id"],"approval":saved["hash"]}));
        assert_eq!(response["error"]["code"], "rule_changed", "{response}");
        assert!(!root.join("settings.json").exists());
    }
    #[test]
    fn symlink_target_not_changed() {
        let (_t, engine, e, root) = setup();
        let outside = root.parent().unwrap().join("outside.json");
        fs::write(&outside, "{}").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("settings.json")).unwrap();
        let r = engine.request(json!({"command":"inspect","environment_id":e["id"]}));
        assert_eq!(r["error"]["code"], "symlink_target");
        assert_eq!(fs::read_to_string(outside).unwrap(), "{}");
    }
    #[test]
    fn replay_never_reapplies_and_crash_is_unknown() {
        let (_t, engine, e, root) = setup();
        let p = plan(&engine, &e);
        let r = json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]});
        let first = engine.request(r.clone());
        fs::write(root.join("settings.json"), "{\"new_login\":\"synthetic\"}").unwrap();
        let second = engine.request(r.clone());
        assert_eq!(first, second);
        assert!(fs::read_to_string(root.join("settings.json"))
            .unwrap()
            .contains("new_login"));
        let jp = engine.path("jobs", string(&p, "id").unwrap());
        let mut job = load(&jp).unwrap();
        job["status"] = json!("executing");
        save(&jp, &job).unwrap();
        let third = engine.request(r);
        assert_eq!(third["data"]["status"], "needs_reconciliation");
        assert!(fs::read_to_string(root.join("settings.json"))
            .unwrap()
            .contains("new_login"));
    }
    #[test]
    fn live_journal_query_is_not_misclassified_as_interrupted() {
        let (_t, engine, e, _root) = setup();
        let p = plan(&engine, &e);
        let jid = string(&p, "id").unwrap();
        let jp = engine.path("jobs", jid);
        save(&jp, &json!({"id":jid,"status":"executing","warnings":[]})).unwrap();
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(engine.state.join("operation.lock"))
            .unwrap();
        lock.lock_exclusive().unwrap();
        let live = engine.request(json!({"command":"job","job_id":jid}));
        assert_eq!(live["data"]["status"], "executing");
        assert_eq!(
            engine.request(json!({"command":"inspect","environment_id":e["id"]}))["error"]["code"],
            "target_busy"
        );
        drop(lock);
        let orphan = engine.request(json!({"command":"job","job_id":jid}));
        assert_eq!(orphan["data"]["status"], "needs_reconciliation");
    }
    #[test]
    fn executable_discovery_uses_current_user_native_install_without_running_it() {
        let t = TempDir::new().unwrap();
        let home = t.path().join("home");
        let native = home.join(".local/bin/claude");
        fs::create_dir_all(native.parent().unwrap()).unwrap();
        fs::write(&native, "not executed").unwrap();
        fs::set_permissions(&native, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(find_executable(&home, None).is_none());
        fs::set_permissions(&native, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            find_executable(&home, None),
            Some(
                native
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            )
        );
        let bin = t.path().join("bin");
        fs::create_dir(&bin).unwrap();
        fs::write(bin.join("claude"), "PATH first").unwrap();
        fs::set_permissions(bin.join("claude"), fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            find_executable(&home, Some(bin.as_os_str())),
            Some(
                bin.join("claude")
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            )
        );
        assert!(find_executable(&t.path().join("other-user"), None).is_none());
    }
    #[test]
    fn launch_rejects_non_loopback_proxy_before_starting_any_process() {
        let (_t, engine, e, _root) = setup();
        for url in [
            "http://example.com:8080",
            "http://192.0.2.1:443",
            "http://127.0.0.1:0",
            "socks5://127.0.0.1:9000",
        ] {
            let result = engine
                .request(json!({"command":"launch","environment_id":e["id"],"proxy_url":url}));
            assert_eq!(result["error"]["code"], "invalid_proxy");
        }
    }
    #[test]
    fn managed_policy_blocks_override() {
        let (_t, engine, e, root) = setup();
        fs::write(root.join("managed-settings.json"), "{}").unwrap();
        let r = engine
            .request(json!({"command":"plan_policy","environment_id":e["id"],"preset":"reduce"}));
        assert_eq!(r["error"]["code"], "managed_settings");
    }
    #[test]
    fn drift_acceptance_and_export_redaction() {
        let (_t, engine, e, root) = setup();
        let p = plan(&engine, &e);
        engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}));
        fs::write(root.join("settings.json"), "{}").unwrap();
        let r = engine.request(json!({"command":"drift","environment_id":e["id"]}));
        assert_eq!(r["data"]["status"], "changed");
        engine.request(json!({"command":"accept_drift","environment_id":e["id"]}));
        let r = engine.request(json!({"command":"drift","environment_id":e["id"]}));
        assert_eq!(r["data"]["status"], "unchanged");
        let s = engine
            .request(json!({"command":"export_support"}))
            .to_string();
        assert!(!s.contains(root.to_str().unwrap()));
    }
    #[test]
    fn encrypted_archive_and_inert_migration() {
        let (_t, engine, e, root) = setup();
        fs::write(root.join("CLAUDE.md"), "synthetic instruction").unwrap();
        fs::create_dir_all(root.join("projects/example/memory")).unwrap();
        fs::write(
            root.join("projects/example/memory/MEMORY.md"),
            "synthetic memory",
        )
        .unwrap();
        fs::write(
            root.join("projects/example/session.jsonl"),
            "{\"text\":\"synthetic session\"}\n",
        )
        .unwrap();
        fs::write(
            root.join(".credentials.json"),
            "SYNTHETIC_CREDENTIAL_DO_NOT_COPY",
        )
        .unwrap();
        fs::write(
            root.join("settings.json"),
            "{\"hooks\":{\"sentinel\":\"do-not-run\"}}",
        )
        .unwrap();
        let p=engine.request(json!({"command":"plan_reset","environment_id":e["id"],"recipe":"rebuild","categories":["instructions","memory","sessions"]}))["data"].clone();
        let j=engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"],"archive_passphrase":"synthetic passphrase for testing only"}));
        assert_eq!(j["data"]["status"], "partially_completed", "{j}");
        let new = PathBuf::from(j["data"]["new_root"].as_str().unwrap());
        assert!(new.join("CLAUDE.md").exists());
        assert!(!new.join("settings.json").exists());
        assert!(!new.join(".credentials.json").exists());
        assert!(new
            .join("lintel-imports/projects/example/session.jsonl")
            .exists());
        assert!(root.join(".credentials.json").exists());
        let encrypted = fs::read(j["data"]["archive_path"].as_str().unwrap()).unwrap();
        assert!(encrypted.starts_with(b"age-encryption.org/"));
        assert!(!String::from_utf8_lossy(&encrypted).contains("synthetic instruction"));
        let decryptor = age::Decryptor::new(encrypted.as_slice()).unwrap();
        let identity = age::scrypt::Identity::new(age::secrecy::SecretString::from(
            "synthetic passphrase for testing only".to_string(),
        ));
        let mut reader = decryptor
            .decrypt(std::iter::once(&identity as &dyn age::Identity))
            .unwrap();
        let mut plain = String::new();
        use std::io::Read;
        reader.read_to_string(&mut plain).unwrap();
        assert!(!plain.contains("SYNTHETIC_CREDENTIAL_DO_NOT_COPY"));
        assert!(!plain.contains("do-not-run"));
        let parsed: Value = serde_json::from_str(&plain).unwrap();
        assert_eq!(parsed["files"].as_array().unwrap().len(), 3);
    }
    fn rebuild_once(engine: &Engine, env_id: &Value) -> Value {
        let p = engine.request(json!({"command":"plan_reset","environment_id":env_id,"recipe":"rebuild","categories":["instructions","memory","sessions"]}))["data"].clone();
        let j = engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"],"archive_passphrase":"synthetic passphrase for testing only"}));
        assert_eq!(j["data"]["status"], "partially_completed", "{j}");
        j["data"].clone()
    }
    fn unseal_work(path: &str) -> Value {
        let encrypted = fs::read(path).unwrap();
        let decryptor = age::Decryptor::new(encrypted.as_slice()).unwrap();
        let identity = age::scrypt::Identity::new(age::secrecy::SecretString::from(
            "synthetic passphrase for testing only".to_string(),
        ));
        let mut reader = decryptor
            .decrypt(std::iter::once(&identity as &dyn age::Identity))
            .unwrap();
        let mut plain = String::new();
        use std::io::Read;
        reader.read_to_string(&mut plain).unwrap();
        serde_json::from_str(&plain).unwrap()
    }
    #[test]
    fn second_rebuild_retains_previously_imported_work() {
        let (_t, engine, e, root) = setup();
        fs::write(root.join("CLAUDE.md"), "synthetic instruction").unwrap();
        fs::create_dir_all(root.join("projects/example/memory")).unwrap();
        fs::write(
            root.join("projects/example/memory/MEMORY.md"),
            "synthetic memory",
        )
        .unwrap();
        fs::write(
            root.join("projects/example/session.jsonl"),
            "{\"text\":\"synthetic session\"}\n",
        )
        .unwrap();
        let first = rebuild_once(&engine, &e["id"]);
        let second = rebuild_once(&engine, &first["new_environment_id"]);
        let new2 = PathBuf::from(second["new_root"].as_str().unwrap());
        assert!(new2.join("CLAUDE.md").exists());
        assert!(new2
            .join("lintel-imports/projects/example/session.jsonl")
            .exists());
        assert!(new2
            .join("lintel-imports/projects/example/memory/MEMORY.md")
            .exists());
        assert!(!new2.join("lintel-imports/lintel-imports").exists());
        let package = unseal_work(second["archive_path"].as_str().unwrap());
        let digests: std::collections::HashSet<String> = package["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["digest"].as_str().unwrap().to_string())
            .collect();
        for content in [
            "synthetic instruction",
            "synthetic memory",
            "{\"text\":\"synthetic session\"}\n",
        ] {
            assert!(
                digests.contains(&digest(content.as_bytes())),
                "second archive lost earlier work: {content}"
            );
        }
    }
    #[test]
    fn oversized_work_does_not_break_inspection() {
        let (_t, engine, e, root) = setup();
        fs::write(
            root.join("settings.json"),
            r#"{"env":{"DISABLE_ERROR_REPORTING":"1"}}"#,
        )
        .unwrap();
        fs::create_dir_all(root.join("projects/demo")).unwrap();
        fs::write(
            root.join("projects/demo/session.jsonl"),
            vec![b'x'; 9 * 1024 * 1024],
        )
        .unwrap();
        let r = engine.request(json!({"command":"inspect","environment_id":e["id"]}));
        assert_eq!(r["ok"], true, "{r}");
        let assets = r["data"]["assets"].as_array().unwrap();
        let sessions = assets.iter().find(|a| a["category"] == "sessions").unwrap();
        assert_eq!(sessions["count"], 1);
        assert_eq!(sessions["complete"], true);
        // The destructive plan still enforces its own admission limit in full.
        let p = engine.request(json!({"command":"plan_reset","environment_id":e["id"],"recipe":"rebuild","categories":["sessions"]}));
        assert_eq!(p["ok"], false);
        assert!(
            ["file_limit", "archive_limit"].contains(&p["error"]["code"].as_str().unwrap()),
            "{p}"
        );
    }
    #[test]
    fn malformed_settings_allows_work_preservation_plan_and_execution() {
        let (_t, engine, e, root) = setup();
        let damaged = r#"{"env":{"DISABLE_TELEMETRY":"1",}}"#;
        fs::write(root.join("settings.json"), damaged).unwrap();
        fs::write(root.join("CLAUDE.md"), "preservable instructions").unwrap();
        let policy = engine
            .request(json!({"command":"plan_policy","environment_id":e["id"],"preset":"reduce"}));
        assert_eq!(policy["error"]["code"], "invalid_json");
        let p = engine.request(json!({"command":"plan_reset","environment_id":e["id"],"recipe":"rebuild","categories":["instructions"]}));
        assert_eq!(p["ok"], true, "{p}");
        let j = engine.request(json!({"command":"execute","plan_id":p["data"]["id"],"approval":p["data"]["hash"],"archive_passphrase":"synthetic passphrase for testing only"}));
        assert_eq!(j["data"]["status"], "partially_completed", "{j}");
        assert_eq!(
            fs::read_to_string(root.join("settings.json")).unwrap(),
            damaged
        );
        let new = PathBuf::from(j["data"]["new_root"].as_str().unwrap());
        assert_eq!(
            fs::read_to_string(new.join("CLAUDE.md")).unwrap(),
            "preservable instructions"
        );
    }
}

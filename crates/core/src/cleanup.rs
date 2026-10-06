//! Explicit Claude Code cleanup recipes. No Keychain service-name guessing.
use crate::{archive, err, now, storage::*, string, work, Engine, Result};
use serde_json::{json, Value};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn auth_status_digest(public: &Value, raw: &Value, nonce: &str) -> String {
    // Canonical JSON detects account/organization/status changes without
    // retaining their values. A per-plan nonce prevents a public stable ID.
    digest(&serde_json::to_vec(&json!([nonce, public["logged_in"], raw])).unwrap())
}

pub(crate) fn bounded(mut command: Command) -> Result<(i32, Vec<u8>)> {
    use std::os::unix::process::CommandExt;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0);
    let mut child = command
        .spawn()
        .map_err(|_| err("adapter_start_failed", "无法启动已登记的 Claude 命令"))?;
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut data = vec![];
        let result = stdout.take(64 * 1024 + 1).read_to_end(&mut data);
        let _ = tx.send((result, data));
    });
    let started = Instant::now();
    let mut status = None;
    let mut output = None;
    loop {
        if status.is_none() {
            status = child.try_wait()?;
        }
        if output.is_none() {
            output = rx.try_recv().ok();
        }
        if status.is_some() && output.is_some() {
            break;
        }
        if started.elapsed() > Duration::from_secs(20) {
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.wait();
            let _ = reader.join();
            return Err(err(
                "adapter_timeout",
                "Claude 命令超时；执行结果需核对，不自动重跑",
            ));
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let _ = reader.join();
    let (result, bytes) = output.unwrap();
    result?;
    if bytes.len() > 64 * 1024 {
        return Err(err("adapter_output_limit", "命令返回超出适配器上限"));
    }
    Ok((status.unwrap().code().unwrap_or(-1), bytes))
}

impl Engine {
    fn auth_command(&self, e: &Value, action: &str) -> Result<Command> {
        let exe = string(e, "executable").map_err(|_| {
            err(
                "executable_missing",
                "未找到 Claude Code；可先准备环境或只处理明确的本地文件",
            )
        })?;
        guard(Path::new(exe))?;
        let mut command = Command::new(exe);
        command
            .args(["auth", action])
            .current_dir(string(e, "root")?)
            .env("CLAUDE_CONFIG_DIR", string(e, "root")?);
        Ok(command)
    }

    pub(crate) fn auth_probe(&self, r: &Value) -> Result<Value> {
        self.auth_observation(r).map(|(public, _)| public)
    }

    // Keep the raw status only in memory. Public callers receive no account,
    // organization, token, or opaque comparison value.
    fn auth_observation(&self, r: &Value) -> Result<(Value, Value)> {
        let e = self.env(r)?;
        let (code, bytes) = bounded(self.auth_command(&e, "status")?)?;
        let raw = parse(&bytes).map_err(|_| {
            err(
                "auth_schema_unknown",
                "此 Claude 版本的认证状态格式无法识别；没有执行注销",
            )
        })?;
        if ![0, 1].contains(&code)
            || raw["configDirectory"] != e["root"]
            || ![
                "none",
                "claude.ai",
                "oauth_token",
                "api_key",
                "api_key_helper",
                "third_party",
            ]
            .iter()
            .any(|x| raw["authMethod"] == *x)
        {
            return Err(err("auth_scope_unverified","认证来源或 configDirectory 无法匹配；需要支持 configDirectory 的 Claude 版本（官方说明为 2.1.268+）"));
        }
        let identity_observed = code == 0
            && raw["authMethod"] == "claude.ai"
            && raw["email"]
                .as_str()
                .is_some_and(|email| !email.trim().is_empty());
        let public = json!({"environment_id":e["id"],"config_directory":e["root"],"auth_method":raw["authMethod"],"logged_in":code==0,"identity_observed":identity_observed,"checked_at":now(),"remote_revocation":"unverified"});
        Ok((public, raw))
    }

    fn cleanup_files(&self, e: &Value, recipe: &str) -> Result<Vec<Value>> {
        let root = PathBuf::from(string(e, "root")?);
        let mut paths = vec![(root.join(".credentials.json"), "credentials")];
        if recipe != "repair_login" {
            let global = if root == self.home.join(".claude") {
                self.home.join(".claude.json")
            } else {
                root.join(".claude.json")
            };
            paths.push((global, "client_state"));
        }
        paths
            .into_iter()
            .map(|(path, category)| {
                Ok(json!({"path":path,"category":category,"snapshot":snapshot(&path)?}))
            })
            .collect()
    }

    fn known_writers(&self, e: &Value) -> Result<Vec<Value>> {
        #[cfg(test)]
        {
            let _ = e;
            return Ok(vec![]);
        }
        #[cfg(not(test))]
        {
            #[cfg(not(target_os = "linux"))]
            let _ = e;
            let mut cmd = Command::new("/bin/ps");
            cmd.args([
                "-U",
                &unsafe { libc::geteuid() }.to_string(),
                "-o",
                "pid=,comm=",
            ]);
            let (code, bytes) = bounded(cmd)?;
            if code != 0 {
                return Err(err(
                    "writers_unknown",
                    "无法核验 Claude 写入进程，尚未开始清理",
                ));
            }
            let mut out = vec![];
            for line in String::from_utf8_lossy(&bytes).lines() {
                let line = line.trim();
                let Some((pid, comm)) = line.split_once(char::is_whitespace) else {
                    continue;
                };
                let name = Path::new(comm.trim())
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                if ["claude", "Claude", "claude-code"].contains(&name) {
                    #[cfg(target_os = "linux")]
                    if let Ok(bytes) = fs::read(format!("/proc/{pid}/environ")) {
                        let bindings: Vec<_> = bytes
                            .split(|b| *b == 0)
                            .filter(|v| v.starts_with(b"CLAUDE_CONFIG_DIR="))
                            .collect();
                        if bindings.len() == 1
                            && bindings[0]
                                != format!("CLAUDE_CONFIG_DIR={}", string(e, "root")?).as_bytes()
                        {
                            continue; // A verified different root is a neighbor, not this writer.
                        }
                    }
                    out.push(json!({"pid":pid,"name":name,"scope":"not_attributed"}));
                }
            }
            Ok(out)
        }
    }

    pub(crate) fn cleanup_inspect(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let files = self.cleanup_files(&e, "reset_client")?;
        let visible:Vec<Value>=files.iter().map(|f|json!({"path":f["path"],"category":f["category"],"present":!f["snapshot"].is_null()})).collect();
        let services = self.check_cleanup_services(&e);
        let profiles = self.home.join(".config/anthropic");
        Ok(
            json!({"environment_id":e["id"],"files":visible,"writers":self.known_writers(&e)?,"services":services.as_ref().map(|v|json!(v)).unwrap_or_else(|error|json!({"code":error.code,"message":error.message})),"shared_profile_present":profiles.exists(),"official_logout_available":e["executable"].is_string(),"coverage":"Claude Code 已识别的凭据文件与混合客户端状态；浏览器、Desktop、IDE 与目录外 profile 需独立处理"}),
        )
    }

    pub(crate) fn plan_cleanup(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        self.block_managed(&e)?;
        let recipe = string(r, "recipe")?;
        if !["repair_login", "reset_client", "retire"].contains(&recipe) {
            return Err(err("unsupported_recipe", "未知清理配方"));
        }
        if r["writers_confirmed_stopped"] != true {
            return Err(err(
                "writers_confirmation_required",
                "先关闭目标终端、IDE 与自动重启来源，再预览清理",
            ));
        }
        let logout = r["official_logout"].as_bool().unwrap_or(false);
        // Known shared authentication already forbids this operation. Reject it
        // before any slower process/service reads or external auth command.
        if logout {
            self.check_auth_scope()?;
        }
        if !self.known_writers(&e)?.is_empty() {
            return Err(err("writers_active","仍检测到 Claude 进程且无法确认所属 root；请核对并关闭目标写入者，Lintel 不会全局杀进程"));
        }
        let services = self.check_cleanup_services(&e)?;
        let (auth, auth_binding) = if logout {
            let (a, raw) = self.auth_observation(r)?;
            if a["auth_method"] != "claude.ai" && a["auth_method"] != "none" {
                return Err(err(
                    "auth_source_external",
                    "当前认证来自 key、helper、token 或第三方来源；不会把这些来源当成本地登录删除",
                ));
            }
            if a["logged_in"] == true && a["identity_observed"] != true {
                return Err(err("auth_identity_unverified", "CLI 没有提供可核对的登录主体；不能冻结官方注销对象。可仅处理预览内的本地文件，或核对客户端后重新预览"));
            }
            let nonce = crate::id();
            let binding = json!({"schema":"lintel.auth-status/1","nonce":nonce,"status_digest":auth_status_digest(&a, &raw, &nonce)});
            (a, binding)
        } else {
            (Value::Null, Value::Null)
        };
        let categories = if recipe == "repair_login" {
            vec![]
        } else {
            work::categories(r)?
        };
        let manifest = work::manifest(Path::new(string(&e, "root")?), &categories)?;
        let files = self.cleanup_files(&e, recipe)?;
        let mut actions = vec![
            json!({"id":"quiescence","label":"复查目标已停止写入；不关闭其他环境","reversible":false}),
        ];
        if recipe != "repair_login" {
            actions.push(
                json!({"id":"state_backup","label":"先加密备份混合客户端状态","reversible":false}),
            );
            actions.push(json!({"id":"archive","label":"加密归档所选工作内容","reversible":false}));
        }
        // Canonical reset order, preserved end to end: state backup + work
        // archive first, then the fresh root + migration, then official logout
        // (which may contact the server), then the approved local file removals,
        // then retirement. No destructive action moves ahead of preservation.
        if recipe == "reset_client" {
            actions.push(
                json!({"id":"rebuild","label":"建立新环境并迁入选中的工作内容（在任何删除之前）","reversible":false}),
            );
        }
        if logout {
            actions.push(json!({"id":"logout","label":"调用官方 auth logout（可能联系服务端），再核验登录状态","reversible":false}));
        }
        for f in &files {
            if !f["snapshot"].is_null() {
                actions.push(json!({"id":f["category"],"label":format!("移除已预览文件：{}",f["path"].as_str().unwrap()),"reversible":false}));
            }
        }
        if recipe == "retire" {
            actions.push(json!({"id":"retire","label":"从可启动环境中退役；保留原工作目录","reversible":true}));
        }
        // reset_client creates a fresh root: freeze its planned identity and the
        // import mapping now, in the same approved plan.
        let activation = work::activation(r, &categories);
        let frozen_target = if recipe == "reset_client" {
            work::freeze_new_target(&self.state, &manifest, activation["instructions"] == true)?
        } else {
            Value::Null
        };
        self.plan(&e,"cleanup",match recipe{"repair_login"=>"修复目标登录","reset_client"=>"清理并重建客户端",_=>"退役此环境"},json!([]),vec!["所有原始工作内容与项目文件","settings、hooks、MCP 与插件文件（不自动激活到新环境）","其他环境、浏览器与目录外认证"],json!(actions),json!({"recipe":recipe,"services":services,"official_logout":logout,"auth":auth,"auth_binding":auth_binding,"files":files,"categories":categories,"manifest":manifest,"archive_passphrase_required":recipe!="repair_login","executable":e["executable"],"frozen_target":frozen_target,"work_purpose":work::purposes(&categories),"activate":activation}))
    }

    fn check_auth_scope(&self) -> Result<()> {
        if self.home.join(".config/anthropic").exists()
            || std::env::var_os("ANTHROPIC_CONFIG_DIR").is_some()
            || std::env::var_os("ANTHROPIC_PROFILE").is_some()
        {
            return Err(err(
                "shared_auth_scope",
                "发现目录外 Anthropic profile；无法独立核验共享注销范围，不自动注销该来源",
            ));
        }
        Ok(())
    }

    pub(crate) fn check_cleanup(&self, e: &Value, p: &Value, r: &Value) -> Result<()> {
        if p["extra"]["official_logout"] == true {
            self.check_auth_scope()?;
        }
        if json!(self.check_cleanup_services(e)?) != p["extra"]["services"] {
            return Err(err(
                "stale_service_plan",
                "预览后 service hold 证据变化；未开始清理",
            ));
        }
        if !self.known_writers(&e)?.is_empty() {
            return Err(err(
                "writers_active",
                "预览后出现 Claude 写入进程；未开始清理",
            ));
        }
        if p["extra"]["files"] != json!(self.cleanup_files(e, string(&p["extra"], "recipe")?)?) {
            return Err(err("stale_plan", "预览后目标状态文件改变，请重新预览"));
        }
        if p["extra"]["archive_passphrase_required"] == true {
            work::check_passphrase(r)?;
        }
        if p["extra"]["official_logout"] == true {
            if p["extra"]["executable"] != e["executable"]
                || self.executable().as_deref() != e["executable"].as_str()
            {
                return Err(err("executable_changed", "Claude 启动来源改变，请重新检查"));
            }
            let binding = &p["extra"]["auth_binding"];
            let nonce = binding["nonce"].as_str().filter(|_| binding["schema"] == "lintel.auth-status/1")
                .ok_or_else(|| err("stale_auth", "这份未执行计划没有认证主体绑定；请重新预览官方注销，原任务 ID 仍只用于查询"))?;
            let (auth, raw) = self.auth_observation(&json!({"environment_id":e["id"]}))?;
            if auth["auth_method"] != p["extra"]["auth"]["auth_method"]
                || auth["logged_in"] != p["extra"]["auth"]["logged_in"]
                || binding["status_digest"] != auth_status_digest(&auth, &raw, nonce)
            {
                return Err(err(
                    "stale_auth",
                    "预览后认证主体或状态改变；未注销新的登录，请重新核对并预览",
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn cleanup(
        &self,
        e: &Value,
        p: &Value,
        r: &Value,
        j: &mut Value,
        journal: &Path,
    ) -> Result<()> {
        let recipe = string(&p["extra"], "recipe")?;
        let files = p["extra"]["files"].as_array().unwrap();
        // Step 1: write the encrypted state backup for the mixed client-state
        // files. This is a prerequisite for reset_client/retire and is always
        // produced before logout or any removal. The work-content archive (with
        // its own receipt field) is written later, still before any deletion.
        if recipe != "repair_login" {
            let mut state = vec![];
            for f in files
                .iter()
                .filter(|f| f["category"] == "client_state" && !f["snapshot"].is_null())
            {
                state.push(json!({"path":f["path"],"data":read(Path::new(string(f,"path")?),8*1024*1024)?}));
            }
            let bytes = archive::seal(
                &json!({"schema":"lintel.state/1","files":state}),
                work::check_passphrase(r)?,
            )?;
            let backup = self
                .state
                .join("archives")
                .join(format!("{}-state.age", string(j, "id")?));
            j["steps"].as_array_mut().unwrap().push(json!({"id":"state_backup","label":"混合客户端状态备份","status":"executing","message":"正在写入加密状态备份；尚未注销或删除旧文件。"}));
            j["state_archive_path"] = json!(backup);
            save(journal, j)?;
            atomic_new(&backup, &bytes, 0o600)?;
            if read(&backup, 64 * 1024 * 1024)? != bytes {
                return Err(err("archive_readback_failed", "状态备份未能读回，未清理"));
            }
            *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":"state_backup","label":"混合客户端状态备份","status":"completed","message":"加密状态备份已写入并读回；独立于可迁入的工作包。"});
            save(journal, j)?;
        }
        // Step 2: independently archive the selected work content. Never destructive.
        if recipe != "repair_login" {
            self.archive_work(e, p, r, j, journal)?;
        }
        // Step 3: for reset_client, create the new root and migrate the selected
        // work that step 2 already archived. The canonical reset order keeps all
        // preservation (state backup, work archive, fresh root + migration) ahead
        // of every destructive action (logout and file removal).
        if recipe == "reset_client" {
            self.migrate_to_new_root(e, p, r, j, journal)?;
        }
        // Preservation can take time. Recheck both recipes at the common first
        // destructive boundary, including local-only cleanup without logout.
        j["steps"].as_array_mut().unwrap().push(json!({"id":"quiescence","label":"破坏性操作前复查","status":"executing","message":"复查写入进程、service hold 与冻结文件；失败时保留已完成工作产物。"}));
        save(journal, j)?;
        self.check_cleanup(e, p, r)?;
        j["steps"].as_array_mut().unwrap().last_mut().unwrap()["status"] = json!("completed");
        save(journal, j)?;
        // Step 4: official logout (may contact the server), rechecked against the
        // approved scope. A failure here stops the run with the archives and the
        // prepared new root retained, and no approved local file deleted.
        if p["extra"]["official_logout"] == true {
            j["steps"].as_array_mut().unwrap().push(json!({"id":"logout","label":"官方注销","status":"executing","message":"命令结果不确定时不会自动重跑。"}));
            save(journal, j)?;
            let (code, _) = bounded(self.auth_command(e, "logout")?)?;
            if code != 0 {
                return Err(err(
                    "logout_failed",
                    "官方注销未成功；后续本地删除未执行，已完成归档保留",
                ));
            }
            let auth = self.auth_probe(&json!({"environment_id":e["id"]}))?;
            if auth["logged_in"] != false {
                return Err(err(
                    "logout_unverified",
                    "官方命令返回后仍存在认证来源；未继续删除",
                ));
            }
            *j["steps"].as_array_mut().unwrap().last_mut().unwrap() = json!({"id":"logout","label":"官方注销与读回","status":"completed","message":"CLI 报告已退出登录；服务端 token 撤销未独立验证。"});
            save(journal, j)?;
        }
        // Step 5: remove exactly the frozen local files, each rechecked.
        for f in files {
            let path = Path::new(string(f, "path")?);
            let current = snapshot(path)?;
            if current.is_null() {
                continue;
            }
            if current != f["snapshot"] {
                return Err(err(
                    "cleanup_conflict",
                    "处理期间文件发生变化，保留新状态；未自动重复清理",
                ));
            }
            j["steps"].as_array_mut().unwrap().push(json!({"id":f["category"],"label":f["path"],"status":"executing","message":"仅处理预览冻结的文件。"}));
            let quarantine = path.parent().unwrap().join(format!(
                ".lintel-quarantine-{}-{}",
                string(j, "id")?,
                string(f, "category")?
            ));
            j["steps"].as_array_mut().unwrap().last_mut().unwrap()["quarantine_path"] =
                json!(quarantine);
            save(journal, j)?;
            quarantine_remove(path, &f["snapshot"], &quarantine)?;
            if path.exists() {
                return Err(err(
                    "state_reappeared",
                    "清理后检测到状态重新出现，需要核对写入来源",
                ));
            }
            j["steps"].as_array_mut().unwrap().last_mut().unwrap()["status"] = json!("completed");
            save(journal, j)?;
        }
        // Step 6: retire the environment registration (retain recipe only).
        if recipe == "retire" {
            let mut all = self.inventory()?;
            for entry in &mut all {
                if entry["id"] == e["id"] {
                    entry["status"] = json!("retired");
                }
            }
            save(&self.state.join("inventory.json"), &json!(all))?;
            j["steps"].as_array_mut().unwrap().push(json!({"id":"retire","label":"环境已退役","status":"completed","message":"环境停止作为启动目标；原始工作文件仍保留。"}));
        }
        let verified = p["extra"]["official_logout"] == true;
        j["status"] = json!(if verified {
            "completed"
        } else {
            "partially_completed"
        });
        j["local_cleanup"] = json!("completed");
        j["remote_revocation"] = json!("unverified");
        if !verified {
            j["warnings"].as_array_mut().unwrap().push(json!(
                "已处理预览内的本地文件；未执行官方注销，Keychain 与目录外认证状态未知。"
            ));
        }
        j["warnings"].as_array_mut().unwrap().push(json!("此结果仅覆盖预览列出的 Claude Code 状态；浏览器、Desktop/IDE、外部 supervisor 与目录外 profile 不在本次范围内。"));
        Ok(())
    }
}

// Bind removal to the object moved out of the client path, not whatever later
// occupies that path. A replaced file is restored without overwriting new state.
pub(crate) fn quarantine_remove(path: &Path, expected: &Value, directory: &Path) -> Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new().mode(0o700).create(directory)?;
    let held = directory.join("state");
    fs::rename(path, &held)?;
    fs::File::open(directory)?.sync_all()?;
    fs::File::open(path.parent().unwrap())?.sync_all()?;
    let observed = snapshot(&held);
    if !matches!(&observed, Ok(actual) if actual == expected) {
        // hard_link is no-replace. If a writer created another file meanwhile,
        // retain both; the receipt contains the private recovery directory.
        if fs::hard_link(&held, path).is_ok() {
            fs::remove_file(&held)?;
            fs::remove_dir(directory)?;
            fs::File::open(path.parent().unwrap())?.sync_all()?;
        }
        return Err(err(
            "cleanup_conflict",
            "目标在清理窗口内改变；新状态未删除。若原位置已被占用，文件保留在回执的隔离目录中。",
        ));
    }
    fs::remove_file(&held)?;
    fs::remove_dir(directory)?;
    fs::File::open(path.parent().unwrap())?.sync_all()?;
    Ok(())
}

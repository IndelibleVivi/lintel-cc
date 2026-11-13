//! Explicit Claude Code cleanup recipes. No Keychain service-name guessing.
use crate::{err, now, storage::*, string, Engine, Result};
use serde_json::{json, Value};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub(crate) fn bounded(command: Command) -> Result<(i32, Vec<u8>)> {
    bounded_with_timeout(command, Duration::from_secs(20))
}

fn bounded_with_timeout(mut command: Command, deadline: Duration) -> Result<(i32, Vec<u8>)> {
    use std::os::unix::process::CommandExt;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0);
    let mut child = command
        .spawn()
        .map_err(|_| err("adapter_start_failed", "无法启动已登记的 Claude 命令"))?;
    use std::os::fd::AsRawFd;
    let mut stdout = child.stdout.take().unwrap();
    let flags = unsafe { libc::fcntl(stdout.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(stdout.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
        let _ = child.wait();
        return Err(std::io::Error::last_os_error().into());
    }
    let started = Instant::now();
    let mut status = None;
    let mut bytes = Vec::new();
    let mut eof = false;
    loop {
        let mut chunk = [0u8; 4096];
        match stdout.read(&mut chunk) {
            Ok(0) => eof = true,
            Ok(n) => bytes.extend_from_slice(&chunk[..n]),
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => {
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.wait();
                return Err(e.into());
            }
        }
        if status.is_none() {
            match child.try_wait() {
                Ok(value) => status = value,
                Err(e) => {
                    unsafe {
                        libc::kill(-(child.id() as i32), libc::SIGKILL);
                    }
                    let _ = child.wait();
                    return Err(e.into());
                }
            }
        }
        if bytes.len() > 64 * 1024 || started.elapsed() > deadline {
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.wait();
            return Err(if bytes.len() > 64 * 1024 {
                err("adapter_output_limit", "命令返回超出适配器上限")
            } else {
                err(
                    "adapter_timeout",
                    "Claude 命令超时；执行结果需核对，不自动重跑",
                )
            });
        }
        if status.is_some() && eof {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
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
        guard(Path::new(string(e, "root")?))?;
        if self.executable().as_deref() != Some(exe) {
            return Err(err(
                "executable_changed",
                "当前启动程序与已登记来源不同；未运行认证命令，请重新检查环境",
            ));
        }
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
        if (code == 0 && raw["authMethod"] == "none")
            || raw
                .get("loggedIn")
                .is_some_and(|value| value.as_bool() != Some(code == 0))
        {
            return Err(err(
                "auth_schema_unknown",
                "认证状态与命令退出结果不一致；未将矛盾状态视为已登录或已退出",
            ));
        }
        let identity_observed = code == 0
            && raw["authMethod"] == "claude.ai"
            && raw["email"]
                .as_str()
                .is_some_and(|email| !email.trim().is_empty());
        let public = json!({"environment_id":e["id"],"config_directory":e["root"],"auth_method":raw["authMethod"],"logged_in":code==0,"identity_observed":identity_observed,"checked_at":now(),"remote_revocation":"unverified","context_scope":"lintel_runner_selected_root","cwd":e["root"],"other_contexts":"unverified"});
        Ok((public, raw))
    }

    fn cleanup_paths(&self, e: &Value, recipe: &str) -> Result<Vec<(PathBuf, &'static str)>> {
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
        Ok(paths)
    }

    pub(crate) fn known_writers(&self, e: &Value) -> Result<Vec<Value>> {
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
        use std::os::unix::fs::MetadataExt;
        let e = self.env(r)?;
        // Inspection reports presence only. Full content snapshots belong to
        // plan_cleanup/check_cleanup, never this automatic page-load read.
        let visible = self
            .cleanup_paths(&e, "reset_client")?
            .into_iter()
            .map(|(path, category)| {
                guard(&path)?;
                let present = match fs::symlink_metadata(&path) {
                    Ok(meta) => {
                        if meta.uid() != unsafe { libc::geteuid() } || meta.nlink() != 1 {
                            return Err(err("unsafe_file", "文件所有者或硬链接状态不受支持"));
                        }
                        if !meta.is_file() {
                            return Err(err("file_limit", "文件类型不受支持"));
                        }
                        true
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
                    Err(error) => return Err(error.into()),
                };
                Ok(json!({"path":path,"category":category,"present":present}))
            })
            .collect::<Result<Vec<Value>>>()?;
        let services = self.check_cleanup_services(&e);
        let profiles = self.home.join(".config/anthropic");
        Ok(
            json!({"environment_id":e["id"],"files":visible,"writers":self.known_writers(&e)?,"services":services.as_ref().map(|v|json!(v)).unwrap_or_else(|error|json!({"code":error.code,"message":error.message})),"shared_profile_present":profiles.exists(),"official_logout_available":false,"auth_probe_available":e["executable"].is_string(),"official_logout_limitation":"official_logout_scope_unsafe","auth_inventory_available":true,"coverage":"Claude Code 已识别的凭据文件与混合客户端状态；浏览器、Desktop、IDE 与目录外 profile 需独立处理"}),
        )
    }

    pub(crate) fn plan_cleanup(&self, r: &Value) -> Result<Value> {
        match string(r, "recipe")? {
            crate::client_data::CLEAR_RESIDUE_RECIPE => self.plan_clear_residue(r),
            crate::client_data::IDENTITY_RESET_RECIPE => self.plan_identity_reset(r),
            _ => self.plan_auth_cleanup(r),
        }
    }

    pub(crate) fn check_cleanup(&self, e: &Value, p: &Value, r: &Value) -> Result<()> {
        match string(&p["extra"], "recipe")? {
            crate::client_data::CLEAR_RESIDUE_RECIPE => self.check_residue_clear(p, r),
            crate::client_data::IDENTITY_RESET_RECIPE => self.check_identity_reset(p, r),
            _ => self.check_auth_cleanup(e, p, r),
        }
    }

    pub(crate) fn cleanup(
        &self,
        e: &Value,
        p: &Value,
        r: &Value,
        j: &mut Value,
        journal: &Path,
    ) -> Result<()> {
        match string(&p["extra"], "recipe")? {
            crate::client_data::CLEAR_RESIDUE_RECIPE => self.clear_residue(e, p, r, j, journal),
            crate::client_data::IDENTITY_RESET_RECIPE => self.identity_reset(e, p, r, j, journal),
            _ => self.execute_auth_cleanup(e, p, r, j, journal),
        }
    }
}

/// Bounded snapshot of one path: identity (device/inode/owner/mode) plus a
/// digest of its bytes read under an explicit limit. Shared by the residue clear
/// (larger originals) and the standard cleanup paths, so both bind a later
/// removal to the same fields.
pub(crate) fn bounded_snapshot(path: &Path, limit: u64) -> Result<Value> {
    use std::os::unix::fs::MetadataExt;
    guard(path)?;
    if !path.exists() {
        return Ok(Value::Null);
    }
    let m = fs::metadata(path)?;
    if m.uid() != unsafe { libc::geteuid() } || m.nlink() != 1 {
        return Err(err("unsafe_file", "文件所有者或硬链接状态不受支持"));
    }
    let bytes = read(path, limit)?;
    Ok(
        json!({"digest":digest(&bytes),"inode":m.ino(),"device":m.dev(),"mode":m.mode(),"owner":m.uid()}),
    )
}

// Bind removal to the object moved out of the client path, not whatever later
// occupies that path. A replaced file is restored without overwriting new state.
pub(crate) fn quarantine_remove(path: &Path, expected: &Value, directory: &Path) -> Result<()> {
    let outcome = quarantine_remove_bounded(path, expected, directory, 8 * 1024 * 1024)?;
    if !outcome.observed.is_null() {
        return Err(err(
            outcome.observed["code"].as_str().unwrap_or("io_error"),
            outcome.observed["message"]
                .as_str()
                .unwrap_or("隔离删除未完成；按原任务核对"),
        ));
    }
    Ok(())
}

/// The outcome of one quarantine attempt. A verified removal is complete. A
/// conflict leaves the exact moved object at `held`; callers persist that
/// locator before treating the failure as reconciled.
pub(crate) struct Quarantined {
    pub(crate) held: PathBuf,
    pub(crate) retained: bool,
    pub(crate) observed: Value,
}

#[cfg(test)]
thread_local! {
    static FINAL_PARENT_FAILURE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static POST_RENAME_FAILURE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn arm_final_parent_failure() {
    FINAL_PARENT_FAILURE.with(|armed| armed.set(true));
}

#[cfg(test)]
pub(crate) fn arm_post_rename_failure() {
    POST_RENAME_FAILURE.with(|armed| armed.set(armed.get().saturating_add(1)));
}

#[cfg(test)]
fn take_post_rename_failure() -> bool {
    POST_RENAME_FAILURE.with(|armed| {
        let remaining = armed.get();
        if remaining == 0 {
            false
        } else {
            armed.set(remaining - 1);
            true
        }
    })
}

/// The same quarantine removal with a caller-chosen read limit for the
/// post-rename verification. Used by the residue clear, whose exact originals
/// can be larger than the standard state-file bound.
pub(crate) fn quarantine_remove_bounded(
    path: &Path,
    expected: &Value,
    directory: &Path,
    limit: u64,
) -> Result<Quarantined> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new().mode(0o700).create(directory)?;
    let held = directory.join("state");
    #[cfg(test)]
    if let Some(errno) = crate::storage::io_fault::take(path) {
        return Err(std::io::Error::from_raw_os_error(errno).into());
    }
    fs::rename(path, &held)?;
    #[cfg(test)]
    if take_post_rename_failure() {
        return Ok(Quarantined {
            held,
            retained: true,
            observed: json!({"stage":"post_rename_sync","code":"io_error","message":"synthetic post-rename sync failure"}),
        });
    }
    if let Err(error) = fs::File::open(directory).and_then(|file| file.sync_all()) {
        return Ok(Quarantined {
            held,
            retained: true,
            observed: json!({"stage":"quarantine_sync","code":"io_error","message":crate::Error::from(error).message}),
        });
    }
    if let Err(error) = fs::File::open(path.parent().unwrap()).and_then(|file| file.sync_all()) {
        return Ok(Quarantined {
            held,
            retained: true,
            observed: json!({"stage":"source_parent_sync","code":"io_error","message":crate::Error::from(error).message}),
        });
    }
    let observed = bounded_snapshot(&held, limit);
    if !matches!(&observed, Ok(actual) if actual == expected) {
        // hard_link is no-replace. If a writer created another file meanwhile,
        // retain both; the receipt contains the private recovery directory.
        if fs::hard_link(&held, path).is_ok() {
            fs::remove_file(&held)?;
            fs::remove_dir(directory)?;
            fs::File::open(path.parent().unwrap())?.sync_all()?;
        }
        let retained = held.exists();
        return Ok(Quarantined {
            held,
            retained,
            observed: json!({"stage":"verification","code":"cleanup_conflict","message":"目标在清理窗口内改变；新状态未删除。"}),
        });
    }
    remove_quarantined(path, &held, directory)
}

fn remove_quarantined(path: &Path, held: &Path, directory: &Path) -> Result<Quarantined> {
    match fs::remove_file(held) {
        Ok(()) => {}
        Err(error) => {
            return Ok(Quarantined {
                held: held.to_path_buf(),
                retained: true,
                observed: json!({"stage":"remove_held","code":"io_error","message":crate::Error::from(error).message}),
            })
        }
    }
    if let Err(error) = fs::remove_dir(directory) {
        return Ok(Quarantined {
            held: held.to_path_buf(),
            retained: false,
            observed: json!({"stage":"remove_quarantine","code":"io_error","message":crate::Error::from(error).message}),
        });
    }
    #[cfg(test)]
    if FINAL_PARENT_FAILURE.with(|armed| armed.replace(false)) {
        return Ok(Quarantined {
            held: held.to_path_buf(),
            retained: false,
            observed: json!({"stage":"final_parent_sync","code":"io_error","message":"synthetic final parent sync failure"}),
        });
    }
    if let Err(error) = fs::File::open(path.parent().unwrap()).and_then(|file| file.sync_all()) {
        return Ok(Quarantined {
            held: held.to_path_buf(),
            retained: false,
            observed: json!({"stage":"final_parent_sync","code":"io_error","message":crate::Error::from(error).message}),
        });
    }
    Ok(Quarantined {
        held: held.to_path_buf(),
        retained: false,
        observed: Value::Null,
    })
}

#[cfg(test)]
mod bounded_tests {
    use super::*;
    #[test]
    fn bounded_output_and_exit() {
        let mut c = Command::new("/bin/sh");
        c.args(["-c", "printf hello; exit 7"]);
        let (code, bytes) = bounded(c).unwrap();
        assert_eq!(code, 7);
        assert_eq!(bytes, b"hello");
        let mut c = Command::new("/bin/sh");
        c.args([
            "-c",
            "while :; do printf 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'; done",
        ]);
        assert!(bounded(c).unwrap_err().code == "adapter_output_limit");
    }
    #[test]
    fn escaped_writer_cannot_hold_reader_or_operation() {
        use std::os::unix::fs::PermissionsExt;
        for parent_waits in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let base = temp.path().canonicalize().unwrap();
            let home = base.join("home");
            let root = home.join("cc");
            fs::create_dir_all(&root).unwrap();
            let bin = home.join(".local/bin");
            fs::create_dir_all(&bin).unwrap();
            let ready = base.join("ready");
            let pid = base.join("escaped-pid");
            // Resolve the inert fixture interpreter from the current test PATH.
            let interpreter = Command::new("python3")
                .args(["-c", "import sys;print(sys.executable)"])
                .output()
                .unwrap();
            assert!(interpreter.status.success());
            let interpreter = String::from_utf8(interpreter.stdout).unwrap();
            let executable = bin.join("claude");
            fs::write(&executable, format!("#!{}\nimport os,time\np=os.fork()\nif p==0:\n os.setsid()\n open({:?},'w').write(str(os.getpid()))\n open({:?},'w').write('ready')\n time.sleep(30)\n os._exit(0)\nwhile not os.path.exists({:?}): time.sleep(.005)\n{}\n", interpreter.trim(), pid.to_str().unwrap(), ready.to_str().unwrap(), ready.to_str().unwrap(), if parent_waits {"time.sleep(30)"} else {"os._exit(0)"})).unwrap();
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
            let mut engine = Engine::new(home, base.join("state")).unwrap();
            engine.executable_search_path = Some(bin.into_os_string());
            let e = engine
                .register("synthetic escaped writer", &root, false)
                .unwrap();
            let started = Instant::now();
            let response = engine.request(json!({"command":"auth_probe","environment_id":e["id"]}));
            // Terminate only the exact inert escaped fixture; the adapter cannot
            // own a descendant which deliberately creates another session.
            let escaped: i32 = fs::read_to_string(&pid).unwrap().parse().unwrap();
            unsafe {
                libc::kill(escaped, libc::SIGKILL);
            }
            assert_eq!(
                fs::read(&ready).unwrap(),
                b"ready",
                "fault state must precede successful timeout assertion"
            );
            assert_eq!(response["error"]["code"], "adapter_timeout", "{response}");
            assert!(started.elapsed() < Duration::from_secs(24));
            // `discover` acquires the actual Engine operation lock; it must not
            // return target_busy after the failed auth adapter has returned.
            let next = engine.request(json!({"command":"discover"}));
            assert_eq!(next["ok"], true, "{next}");
        }
    }
    #[test]
    fn inherited_pipe_and_running_parent_obey_deadline() {
        for script in ["sleep 2 & exit 0", "sleep 2 & wait"] {
            for _ in 0..3 {
                let mut c = Command::new("/bin/sh");
                c.args(["-c", script]);
                let start = Instant::now();
                assert!(
                    bounded_with_timeout(c, Duration::from_millis(40))
                        .unwrap_err()
                        .code
                        == "adapter_timeout"
                );
                assert!(start.elapsed() < Duration::from_secs(1));
            }
        }
        let mut c = Command::new("/bin/sh");
        c.args(["-c", "printf usable"]);
        assert_eq!(bounded(c).unwrap().1, b"usable");
    }
}

#[cfg(test)]
mod quarantine_outcome_tests {
    use super::*;
    #[test]
    fn legacy_cleanup_does_not_swallow_a_retained_quarantine() {
        let t = tempfile::tempdir().unwrap();
        let base = t.path().canonicalize().unwrap();
        let path = base.join("synthetic.json");
        fs::write(&path, b"original").unwrap();
        let expected = bounded_snapshot(&path, 1024).unwrap();
        arm_post_rename_failure();
        assert!(quarantine_remove(&path, &expected, &base.join("hold")).is_err());
        assert_eq!(fs::read(base.join("hold/state")).unwrap(), b"original");
    }
    #[test]
    fn removed_bytes_with_failed_parent_sync_are_not_completion() {
        let t = tempfile::tempdir().unwrap();
        let base = t.path().canonicalize().unwrap();
        let path = base.join("synthetic.json");
        fs::write(&path, b"original").unwrap();
        let expected = bounded_snapshot(&path, 1024).unwrap();
        FINAL_PARENT_FAILURE.with(|armed| armed.set(true));
        let outcome =
            quarantine_remove_bounded(&path, &expected, &base.join("hold"), 1024).unwrap();
        assert!(!outcome.retained);
        assert!(!path.exists());
        assert_eq!(outcome.observed["stage"], "final_parent_sync");
    }
}

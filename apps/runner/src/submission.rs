use crate::supervisor::{self, Mode, Selection};
use serde_json::{json, Value};
use std::{
    io::{self, BufRead, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
static ACK_SENT: AtomicBool = AtomicBool::new(false);
fn emit(v: &Value) {
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{v}");
    let _ = out.flush();
}
// The synthetic barriers never change what a normal submission does. Both are
// opt-in through LINTEL_TEST_HOME plus one marker path that must resolve inside
// that synthetic home; a real submission returns here immediately.
enum Barrier {
    None,
    // Freeze the worker in place. Makes a genuinely stopped worker observable for
    // the logout/reboot acceptance cases that need a frozen process.
    Stop,
    // Keep running but block on an explicit release file between the durable ACK
    // and the original plan body. Lets acceptance compare a live worker against a
    // stopped one under the same effective logout policy.
    Hold { release: PathBuf },
}
fn accepted(job: &Value) {
    ACK_SENT.store(true, Ordering::SeqCst);
    let barrier = acceptance_barrier(job);
    emit(&json!({"ok":true,"data":job}));
    match barrier {
        Barrier::None => {}
        Barrier::Stop => unsafe {
            libc::raise(libc::SIGSTOP);
        },
        Barrier::Hold { release } => hold_until_released(&release),
    }
}
fn synthetic_marker(marker: &std::ffi::OsStr, home: &Path) -> Option<PathBuf> {
    let marker = PathBuf::from(marker);
    if !marker.is_absolute() {
        return None;
    }
    let parent = marker.parent()?.canonicalize().ok()?;
    parent.starts_with(home).then(|| marker.clone())
}
// Identity of the worker at the after-ACK boundary. Finite /proc facts only; no
// environment, credentials or request payload is recorded.
fn process_identity(job: &Value) -> Value {
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
    // comm may contain spaces and parentheses; the fields that matter follow the
    // final ')': state, ppid, pgrp, session ... starttime. starttime lets a
    // caller disambiguate a reused pid from the same worker.
    let rest = stat.rsplit(')').next().unwrap_or("");
    let fields: Vec<&str> = rest.split_whitespace().collect();
    let field = |i: usize| fields.get(i).and_then(|v| v.parse::<i64>().ok());
    json!({
        "pid": std::process::id(),
        "ppid": field(1),
        "pgrp": field(2),
        "session": field(3),
        "starttime": field(19),
        "plan_id": job["plan_id"],
        "cgroup": std::fs::read_to_string("/proc/self/cgroup").ok(),
        "boot_id": std::fs::read_to_string("/proc/sys/kernel/random/boot_id").ok().map(|s| s.trim().to_owned()),
    })
}
fn write_marker(path: &Path, evidence: &Value) -> bool {
    use std::os::unix::fs::OpenOptionsExt;
    let Ok(mut file) = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
    else {
        return false;
    };
    writeln!(file, "{evidence}").is_ok() && file.sync_all().is_ok()
}
// A stopped worker makes logout/cgroup/reboot tests reproducible; a held worker
// stays live until the harness writes its release file. Only an explicitly
// synthetic HOME can opt in; normal submissions never stop or wait here.
fn acceptance_barrier(job: &Value) -> Barrier {
    let Some(home) = std::env::var_os("LINTEL_TEST_HOME") else {
        return Barrier::None;
    };
    let Ok(home) = PathBuf::from(home).canonicalize() else {
        return Barrier::None;
    };
    if let Some(marker) = std::env::var_os("LINTEL_TEST_ACCEPT_BARRIER") {
        let Some(marker) = synthetic_marker(&marker, &home) else {
            return Barrier::None;
        };
        return if write_marker(&marker, &process_identity(job)) {
            Barrier::Stop
        } else {
            Barrier::None
        };
    }
    let (Some(marker), Some(release)) = (
        std::env::var_os("LINTEL_TEST_WAIT_BARRIER"),
        std::env::var_os("LINTEL_TEST_WAIT_RELEASE"),
    ) else {
        return Barrier::None;
    };
    let (Some(marker), Some(release)) = (
        synthetic_marker(&marker, &home),
        synthetic_marker(&release, &home),
    ) else {
        return Barrier::None;
    };
    if write_marker(&marker, &process_identity(job)) {
        Barrier::Hold { release }
    } else {
        Barrier::None
    }
}
// Bounded wait for the harness's explicit release. The release file is created
// under the synthetic home; nothing else can unblock the worker. The timeout
// keeps a lost harness from pinning the worker forever.
fn hold_until_released(release: &Path) {
    let deadline = Instant::now() + Duration::from_secs(300);
    while Instant::now() < deadline {
        if release.is_file() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
// Truthful capability: selection depends on the current host, and the setsid
// route never claims logout continuation.
pub fn capability(response: &mut Value) {
    if let Some(c) = response["data"]["capabilities"].as_array_mut() {
        c.push(json!({"name":"detached_submission","status":"available","reason":"lintel submit 在 journal 持久接收后返回；仅当当前主机已满足 system manager 或已启用 Linger 的 user manager 时由 transient service 持有，否则为 setsid 且 logout 后继续执行未经验证"}));
    }
}

// Parse the runner-internal worker argument. The selected execution context is
// not secret (mode/manager/unit/continuation) and is never part of the request
// JSON a caller could forge.
fn exec_context_arg(args: &[String]) -> Option<Value> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--exec-context" {
            return iter.next().and_then(|raw| serde_json::from_str(raw).ok());
        }
    }
    None
}

pub fn run(mode: &str, args: &[String]) -> bool {
    let mut input = io::BufReader::new(io::stdin());
    if mode == "__worker" {
        // Match the original caller's environment without putting credentials in
        // D-Bus unit properties, argv, a spool file or the journal. This first
        // private pipe frame is runner-internal; public request JSON cannot set it.
        use std::os::unix::ffi::OsStringExt;
        let mut header = vec![];
        if input
            .by_ref()
            .take(1024 * 1024 + 1)
            .read_until(b'\n', &mut header)
            .is_err()
            || header.len() > 1024 * 1024
            || header.last() != Some(&b'\n')
        {
            emit(
                &json!({"ok":false,"error":{"code":"worker_environment_invalid","message":"worker 调用环境未能完整接收；未持久接受，请查询原任务"}}),
            );
            return false;
        }
        let environment: Vec<(Vec<u8>, Vec<u8>)> = match serde_json::from_slice(&header) {
            Ok(value) => value,
            Err(_) => {
                emit(
                    &json!({"ok":false,"error":{"code":"worker_environment_invalid","message":"worker 调用环境无效；未持久接受，请查询原任务"}}),
                );
                return false;
            }
        };
        // Before any worker threads/core work, replace manager-inherited values
        // with the exact submitting process context (including non-UTF8 bytes).
        for (key, _) in std::env::vars_os() {
            std::env::remove_var(key);
        }
        for (key, value) in environment {
            std::env::set_var(
                std::ffi::OsString::from_vec(key),
                std::ffi::OsString::from_vec(value),
            );
        }
    }
    let mut bytes = vec![];
    if input.take(1024 * 1024 + 1).read_to_end(&mut bytes).is_err() || bytes.len() > 1024 * 1024 {
        emit(
            &json!({"ok":false,"error":{"code":"request_limit","message":"提交请求超过 1 MiB 或读取失败"}}),
        );
        return false;
    }
    let request: Value = match lintel_core::decode_request(&bytes) {
        Ok(v) => v,
        Err(_) => {
            emit(&json!({"ok":false,"error":{"code":"invalid_json","message":"提交请求无效"}}));
            return false;
        }
    };
    if request["command"] != "execute" {
        emit(
            &json!({"ok":false,"error":{"code":"invalid_submission","message":"submit 只接受批准的 execute；查询用 request"}}),
        );
        return false;
    }
    if mode == "__worker" {
        // A manager-launched worker must already live in its exact selected unit's
        // cgroup. If the service failed to move outside the login scope, this is a
        // specific failure BEFORE durable acceptance, never a false success.
        let context = exec_context_arg(args);
        if let Some(unit) = context.as_ref().and_then(|c| c["unit"].as_str()) {
            if !supervisor::cgroup_matches_unit(unit) {
                emit(
                    &json!({"ok":false,"error":{"code":"execution_context_mismatch","message":"worker 实际 cgroup 不属于所选 unit；未写入持久接收，请查询原 plan_id"}}),
                );
                return false;
            }
        }
        let response = lintel_core::handle_request_with_execution(request, context, Some(accepted));
        if !ACK_SENT.load(Ordering::SeqCst) {
            emit(&response);
        }
        return response["ok"] == true;
    }
    let response = submit(&request);
    emit(&response);
    response["ok"] == true
}

pub fn submit(request: &Value) -> Value {
    if request["command"] != "execute" {
        return json!({"ok":false,"error":{"code":"invalid_submission","message":"submit 只接受批准的 execute"}});
    }
    let bytes = serde_json::to_vec(request).unwrap();
    if bytes.len() > 1024 * 1024 {
        return json!({"ok":false,"error":{"code":"request_limit","message":"提交请求超过 1 MiB"}});
    }
    // Select the exact execution context for this original plan. Selection is
    // read-only; a manager is used only when this session can legitimately own the
    // transient unit, otherwise setsid records an explicit limitation. The context
    // is persisted into the accepted receipt before the durable ACK hook fires.
    let plan_id = request["plan_id"].as_str().unwrap_or_default().to_owned();
    let selection = supervisor::select(&plan_id);
    let context = supervisor::execution_context(&selection);
    match launch(&selection, &context, &bytes, &request) {
        Ok(v) => v,
        Err(message) => {
            json!({"ok":false,"error":{"code":"submission_uncertain","message":message},"plan_id":request["plan_id"]})
        }
    }
}

// Launch the worker for the selected context and read back exactly one durable
// ACK line. Manager launch failures return uncertainty; the caller must query the
// original plan and never fall back to a second launch.
fn launch(
    selection: &Selection,
    context: &Value,
    bytes: &[u8],
    request: &Value,
) -> Result<Value, String> {
    use std::os::unix::ffi::OsStrExt;
    let environment: Vec<(Vec<u8>, Vec<u8>)> = std::env::vars_os()
        .map(|(key, value)| (key.as_bytes().to_vec(), value.as_bytes().to_vec()))
        .collect();
    let mut header = serde_json::to_vec(&environment).map_err(|_| "无法传递原调用环境")?;
    header.push(b'\n');
    if header.len() > 1024 * 1024 {
        return Err("原调用环境超过 worker pipe 上限；未启动 worker".into());
    }
    let mut child = match selection.mode {
        Mode::SystemManager | Mode::UserManager => manager_child(selection, context, request)?,
        Mode::Setsid => setsid_child(context)?,
    };
    let mut input = child.stdin.take().ok_or("worker stdin 不可用")?;
    if input
        .write_all(&header)
        .and_then(|_| input.write_all(bytes))
        .is_err()
    {
        return Err("后台提交未能写入 worker stdin；查询原 plan_id，不能据此认定尚未执行".into());
    }
    drop(input);
    let output = child.stdout.take().ok_or("worker stdout 不可用")?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = io::BufReader::new(output.take(128 * 1024)).read_line(&mut line);
        let _ = tx.send((result, line));
    });
    match rx.recv_timeout(Duration::from_secs(40)) {
        Ok((Ok(_), line)) => serde_json::from_str(&line)
            .map_err(|_| "worker 回执无法解析；查询原 plan_id，不能据此认定尚未执行".to_string()),
        _ => Err("尚未收到持久接收回执；请查询原 plan_id，不要重提".into()),
    }
}

fn context_arg(context: &Value) -> String {
    serde_json::to_string(context).unwrap_or_else(|_| "{}".into())
}

fn manager_child(
    selection: &Selection,
    context: &Value,
    request: &Value,
) -> Result<std::process::Child, String> {
    let exe = std::env::current_exe().map_err(|_| "无法定位当前 runner".to_string())?;
    let cwd = std::env::current_dir().map_err(|_| "无法定位当前工作目录")?;
    let mut command = supervisor::manager_command(selection, &exe, &cwd, &context_arg(context))
        .ok_or("所选执行上下文缺少 unit 名称")?;
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command.spawn().map_err(|_| {
        format!(
            "无法启动所选 manager unit；查询原 plan_id {}",
            request["plan_id"]
        )
    })
}

fn setsid_child(context: &Value) -> Result<std::process::Child, String> {
    use std::os::unix::process::CommandExt;
    let exe = std::env::current_exe().map_err(|_| "无法定位当前 runner".to_string())?;
    let mut command = Command::new(exe);
    command
        .arg("__worker")
        .arg("--exec-context")
        .arg(context_arg(context))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    // Only the new worker leaves this SSH/terminal session. Its stdin closes
    // after one request; passphrases never enter arguments or a spool file.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command
        .spawn()
        .map_err(|_| "无法启动后台 worker".to_string())
}

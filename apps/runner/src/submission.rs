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
pub fn capability(response: &mut Value) {
    if let Some(c) = response["data"]["capabilities"].as_array_mut() {
        c.push(json!({"name":"detached_submission","status":"available","reason":"lintel submit 在 journal 持久接收后返回；worker 独立 session，主机 logout/cgroup 政策仍须实际验证"}));
    }
}
pub fn run(mode: &str) {
    let mut bytes = vec![];
    if io::stdin()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() > 1024 * 1024
    {
        emit(
            &json!({"ok":false,"error":{"code":"request_limit","message":"提交请求超过 1 MiB 或读取失败"}}),
        );
        return;
    }
    let request: Value = match lintel_core::decode_request(&bytes) {
        Ok(v) => v,
        Err(_) => {
            emit(&json!({"ok":false,"error":{"code":"invalid_json","message":"提交请求无效"}}));
            return;
        }
    };
    if request["command"] != "execute" {
        emit(
            &json!({"ok":false,"error":{"code":"invalid_submission","message":"submit 只接受批准的 execute；查询用 request"}}),
        );
        return;
    }
    if mode == "__worker" {
        let response = lintel_core::handle_request_with_accept(request, accepted);
        if !ACK_SENT.load(Ordering::SeqCst) {
            emit(&response);
        }
        return;
    }
    let result = (|| -> io::Result<Value> {
        use std::os::unix::process::CommandExt;
        let mut command = Command::new(std::env::current_exe()?);
        command
            .arg("__worker")
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
        let mut child = command.spawn()?;
        let mut input = child.stdin.take().unwrap();
        input.write_all(&bytes)?;
        drop(input);
        let output = child.stdout.take().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let result = io::BufReader::new(output.take(128 * 1024)).read_line(&mut line);
            let _ = tx.send((result, line));
        });
        match rx.recv_timeout(Duration::from_secs(40)) {
            Ok((Ok(_), line)) => serde_json::from_str(&line)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "worker response invalid")),
            _ => Ok(
                json!({"ok":false,"error":{"code":"submission_uncertain","message":"尚未收到持久接收回执；请查询原 plan_id，不要重提"},"plan_id":request["plan_id"]}),
            ),
        }
    })();
    match result {
        Ok(v) => emit(&v),
        Err(_) => emit(
            &json!({"ok":false,"error":{"code":"submission_uncertain","message":"后台提交未能确认；查询原 plan_id，不能据此认定尚未执行"},"plan_id":request["plan_id"]}),
        ),
    }
}

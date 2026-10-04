use serde_json::{json, Value};
use std::{
    io::{self, BufRead, Read, Write},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
static ACK_SENT: AtomicBool = AtomicBool::new(false);
fn emit(v: &Value) {
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{v}");
    let _ = out.flush();
}
fn accepted(job: &Value) {
    ACK_SENT.store(true, Ordering::SeqCst);
    let pause = acceptance_barrier(job);
    emit(&json!({"ok":true,"data":job}));
    if pause {
        unsafe {
            libc::raise(libc::SIGSTOP);
        }
    }
}
// A stopped worker makes logout/cgroup/reboot tests reproducible. Only an
// explicitly synthetic HOME can opt in; normal submissions never stop here.
fn acceptance_barrier(job: &Value) -> bool {
    let (Some(home), Some(marker)) = (
        std::env::var_os("LINTEL_TEST_HOME"),
        std::env::var_os("LINTEL_TEST_ACCEPT_BARRIER"),
    ) else {
        return false;
    };
    let (home, marker) = (
        std::path::PathBuf::from(home),
        std::path::PathBuf::from(marker),
    );
    let Ok(home) = home.canonicalize() else {
        return false;
    };
    let Some(parent) = marker.parent() else {
        return false;
    };
    let Ok(parent) = parent.canonicalize() else {
        return false;
    };
    if !marker.is_absolute() || !parent.starts_with(&home) {
        return false;
    }
    use std::os::unix::fs::OpenOptionsExt;
    let Ok(mut file) = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&marker)
    else {
        return false;
    };
    let evidence = json!({"pid":std::process::id(),"plan_id":job["plan_id"],
        "cgroup":std::fs::read_to_string("/proc/self/cgroup").ok(),
        "boot_id":std::fs::read_to_string("/proc/sys/kernel/random/boot_id").ok().map(|s|s.trim().to_owned())});
    writeln!(file, "{evidence}").is_ok() && file.sync_all().is_ok()
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

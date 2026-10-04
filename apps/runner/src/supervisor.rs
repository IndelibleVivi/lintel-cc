//! Finite runner supervisor selection for detached submission.
//!
//! One original plan is launched as exactly one transient systemd service when
//! the current session can legitimately own it:
//!   * PID 1 systemd and current euid 0 -> the system manager;
//!   * PID 1 systemd, non-root, and the current UID's `Linger` is already `yes`
//!     with a usable user bus -> the user manager.
//! Anything else keeps the existing `setsid` route and records explicitly that
//! logout continuation is unverified. No sudo, no `enable-linger`, no login-policy
//! edit and no persistent daemon. No automatic fallback: once a manager launch may
//! have started, a failure returns uncertainty and the caller queries the original
//! job instead of launching again.
use serde_json::{json, Value};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Fixed native system tools. Never resolved through a user-controlled PATH so a
/// submission cannot redirect the supervisor to a different binary.
const SYSTEMD_RUN: &str = "/usr/bin/systemd-run";
const SYSTEMCTL: &str = "/usr/bin/systemctl";
const LOGINCTL: &str = "/usr/bin/loginctl";

#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    SystemManager,
    UserManager,
    Setsid,
}

#[derive(Clone, Debug)]
pub struct Selection {
    pub mode: Mode,
    pub manager: Option<&'static str>,
    pub unit: Option<String>,
    pub limitation: Option<&'static str>,
}

fn bounded(mut command: Command) -> Option<(i32, String)> {
    use std::os::unix::process::CommandExt;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0);
    let mut child = command.spawn().ok()?;
    let stdout = child.stdout.take()?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        use std::io::Read;
        let mut data = String::new();
        let result = stdout.take(64 * 1024).read_to_string(&mut data);
        let _ = tx.send((result, data));
    });
    let started = std::time::Instant::now();
    loop {
        if let Some(status) = child.try_wait().ok().flatten() {
            let (_, data) = rx.recv_timeout(Duration::from_secs(2)).ok()?;
            return Some((status.code().unwrap_or(255), data));
        }
        if started.elapsed() > Duration::from_secs(3) {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn tool(path: &str, args: &[&str]) -> Option<(i32, String)> {
    let mut command = Command::new(path);
    command
        .args(args)
        .env_clear()
        .env("LC_ALL", "C")
        .env("SYSTEMD_COLORS", "0");
    if args.first() == Some(&"--user") {
        user_bus_environment(&mut command, unsafe { libc::geteuid() });
    }
    bounded(command)
}

/// A validated original plan id is the canonical 8-4-4-4-12 lowercase hex UUID
/// that core emits. The per-job unit name is derived from it directly rather than
/// an invented opaque hash, and no other shape is accepted.
fn valid_uuid(id: &str) -> bool {
    let bytes = id.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    for (i, b) in bytes.iter().enumerate() {
        match i {
            8 | 13 | 18 | 23 => {
                if *b != b'-' {
                    return false;
                }
            }
            _ => {
                if !b.is_ascii_hexdigit() {
                    return false;
                }
            }
        }
    }
    true
}

pub fn unit_name(plan_id: &str) -> Option<String> {
    valid_uuid(plan_id).then(|| format!("lintel-{plan_id}.service"))
}

fn systemd_pid1() -> bool {
    std::fs::read_to_string("/proc/1/comm")
        .map(|c| c.trim() == "systemd")
        .unwrap_or(false)
}

/// Read-only eligibility. Manager reachability alone is not proof of logout
/// persistence, so the `Linger` value is read explicitly for the current UID.
fn linger_enabled(uid: u32) -> bool {
    let uid = uid.to_string();
    tool(LOGINCTL, &["show-user", &uid, "-p", "Linger", "--value"])
        .map(|(code, out)| code == 0 && out.trim() == "yes")
        .unwrap_or(false)
}

fn user_bus_usable(uid: u32) -> bool {
    let runtime = format!("/run/user/{uid}");
    Path::new(&runtime).is_dir()
        && tool(SYSTEMCTL, &["--user", "is-system-running"])
            .map(|(code, out)| {
                (code == 0 || code == 1) && matches!(out.trim(), "running" | "degraded")
            })
            .unwrap_or(false)
}

fn user_bus_environment(command: &mut Command, uid: u32) {
    let runtime = format!("/run/user/{uid}");
    command.env("XDG_RUNTIME_DIR", &runtime).env(
        "DBUS_SESSION_BUS_ADDRESS",
        format!("unix:path={runtime}/bus"),
    );
}

pub fn select(plan_id: &str) -> Selection {
    let linux_systemd = cfg!(target_os = "linux") && systemd_pid1();
    let euid = unsafe { libc::geteuid() };
    select_with(
        plan_id,
        linux_systemd,
        euid,
        linux_systemd && euid != 0 && linger_enabled(euid),
        linux_systemd && euid != 0 && user_bus_usable(euid),
    )
}

/// Pure selection over factual host eligibility, also used by synthetic tests.
pub fn select_with(
    plan_id: &str,
    linux_systemd: bool,
    euid: u32,
    linger: bool,
    user_bus: bool,
) -> Selection {
    let setsid = |rationale: &'static str| Selection {
        mode: Mode::Setsid,
        manager: None,
        unit: None,
        limitation: Some(rationale),
    };
    let unit = unit_name(plan_id);
    if !linux_systemd {
        return setsid("当前主机没有 PID 1 systemd；logout 后继续执行未经验证");
    }
    let Some(unit) = unit else {
        return setsid("原任务 ID 不是有效 UUID，无法派生精确 transient unit");
    };
    if euid == 0 {
        return Selection {
            mode: Mode::SystemManager,
            manager: Some("system"),
            unit: Some(unit),
            limitation: None,
        };
    }
    if linger && user_bus {
        return Selection {
            mode: Mode::UserManager,
            manager: Some("user"),
            unit: Some(unit),
            limitation: None,
        };
    }
    setsid("当前用户没有已启用的 Linger 或可用 user bus；logout 后继续执行未经验证")
}

/// The worker-side factual context persisted into the accepted receipt. It states
/// what the runner selected and what it does NOT guarantee; it never claims
/// reboot survival.
pub fn execution_context(selection: &Selection) -> Value {
    let (mode, continuation) = match selection.mode {
        Mode::SystemManager => (
            "system_manager",
            "logout 后由 PID 1 系统 manager 持有；重启仍会中断并需要核对",
        ),
        Mode::UserManager => (
            "user_manager",
            "logout 后由当前用户已启用的持久 user manager 持有；重启仍会中断并需要核对",
        ),
        Mode::Setsid => (
            "setsid",
            "仅脱离当前进程 session；此路径不提供退出登录后继续执行的保证，重启也会中断",
        ),
    };
    json!({
        "mode": mode,
        "manager": selection.manager,
        "unit": selection.unit,
        "continuation": continuation,
        "limitation": selection.limitation,
        "reboot_survival": false,
    })
}

/// Does the current process's own cgroup belong to the exact selected unit? The
/// worker calls this before accepting so a service that failed to leave the login
/// scope never produces a durable acceptance.
pub fn cgroup_matches_unit(unit: &str) -> bool {
    let Ok(cgroup) = std::fs::read_to_string("/proc/self/cgroup") else {
        return false;
    };
    cgroup_text_matches_unit(&cgroup, unit)
}

/// Pure predicate over cgroup text so the synthetic suite can assert both the
/// match and the mismatch without a real `/proc`.
pub fn cgroup_text_matches_unit(cgroup: &str, unit: &str) -> bool {
    // systemd names the transient unit's cgroup after the unit (system manager) or
    // under a user slice (user manager); both contain the unit name literally.
    cgroup.lines().any(|line| {
        line.rsplit(':')
            .next()
            .is_some_and(|path| path.split('/').any(|part| part == unit))
    })
}

/// Build the `systemd-run` command for a validated selection plus the exact
/// worker invocation. Only non-secret identity/PATH configuration is exported;
/// the request JSON travels on stdin and the archive passphrase never enters
/// argv, the environment, a spool file or the journal.
pub fn manager_command(
    selection: &Selection,
    executable: &Path,
    home: &Path,
    state: &Path,
    path_env: &str,
    context: &str,
) -> Option<Command> {
    let unit = selection.unit.as_deref()?;
    let mut command = Command::new(SYSTEMD_RUN);
    command
        .env_clear()
        .arg("--quiet")
        .arg("--collect")
        .arg("--pipe")
        .arg("--no-ask-password")
        .arg("--service-type=exec")
        .arg("--expand-environment=no");
    if selection.manager == Some("user") {
        command.arg("--user");
        user_bus_environment(&mut command, unsafe { libc::geteuid() });
    }
    command
        .arg(format!("--unit={unit}"))
        .arg("--property=Restart=no")
        .arg("--property=UMask=0077")
        .arg("--property=StandardError=null")
        .arg(format!("--setenv=HOME={}", home.display()))
        .arg(format!("--setenv=LINTEL_STATE_DIR={}", state.display()))
        .arg(format!("--setenv=PATH={path_env}"))
        .env("LC_ALL", "C")
        .env("SYSTEMD_COLORS", "0");
    if std::env::var_os("LINTEL_TEST_HOME").is_some() {
        for name in [
            "LINTEL_TEST_HOME",
            "LINTEL_TEST_ACCEPT_BARRIER",
            "LINTEL_TEST_WAIT_BARRIER",
            "LINTEL_TEST_WAIT_RELEASE",
        ] {
            if let Some(value) = std::env::var_os(name) {
                command.arg(format!("--setenv={name}={}", value.to_string_lossy()));
            }
        }
    }
    command
        .arg("--")
        .arg(executable)
        .arg("__worker")
        .arg("--exec-context")
        .arg(context);
    Some(command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const UUID: &str = "344e11f2-f95b-40d0-bc69-4034aa1fcd13";

    #[test]
    fn unit_name_requires_canonical_uuid() {
        assert_eq!(
            unit_name(UUID).as_deref(),
            Some("lintel-344e11f2-f95b-40d0-bc69-4034aa1fcd13.service")
        );
        assert!(unit_name("not-a-uuid").is_none());
        assert!(unit_name(&UUID.to_uppercase()).is_some()); // hex case is irrelevant to systemd
        assert!(unit_name("344e11f2-f95b-40d0-bc69-4034aa1fcd1").is_none());
    }

    #[test]
    fn non_systemd_host_stays_setsid() {
        let s = select_with(UUID, false, 0, false, false);
        assert_eq!(s.mode, Mode::Setsid);
        assert!(s.limitation.is_some());
        assert!(s.unit.is_none());
    }

    #[test]
    fn root_systemd_selects_system_manager() {
        let s = select_with(UUID, true, 0, false, false);
        assert_eq!(s.mode, Mode::SystemManager);
        assert_eq!(s.manager, Some("system"));
        assert_eq!(
            s.unit.as_deref(),
            Some("lintel-344e11f2-f95b-40d0-bc69-4034aa1fcd13.service")
        );
        assert!(s.limitation.is_none());
    }

    #[test]
    fn non_root_needs_linger_and_bus() {
        let no_linger = select_with(UUID, true, 1001, false, true);
        assert_eq!(no_linger.mode, Mode::Setsid);
        assert!(no_linger.limitation.is_some());
        let no_bus = select_with(UUID, true, 1001, true, false);
        assert_eq!(no_bus.mode, Mode::Setsid);
        let eligible = select_with(UUID, true, 1001, true, true);
        assert_eq!(eligible.mode, Mode::UserManager);
        assert_eq!(eligible.manager, Some("user"));
    }

    #[test]
    fn invalid_uuid_never_gets_a_unit() {
        let s = select_with("bad", true, 0, false, false);
        assert_eq!(s.mode, Mode::Setsid);
        assert!(s.limitation.is_some());
    }

    #[test]
    fn context_states_limits_and_never_claims_reboot_survival() {
        let setsid = execution_context(&select_with(UUID, false, 0, false, false));
        assert_eq!(setsid["mode"], "setsid");
        assert_eq!(setsid["reboot_survival"], false);
        assert!(setsid["limitation"].is_string());
        let system = execution_context(&select_with(UUID, true, 0, false, false));
        assert_eq!(system["mode"], "system_manager");
        assert_eq!(system["reboot_survival"], false);
        assert_eq!(
            system["unit"],
            "lintel-344e11f2-f95b-40d0-bc69-4034aa1fcd13.service"
        );
    }

    #[test]
    fn cgroup_match_is_exact_unit() {
        let unit = "lintel-344e11f2-f95b-40d0-bc69-4034aa1fcd13.service";
        assert!(cgroup_text_matches_unit(
            "0::/system.slice/lintel-344e11f2-f95b-40d0-bc69-4034aa1fcd13.service\n",
            unit
        ));
        assert!(cgroup_text_matches_unit(
            "0::/user.slice/user-1001.slice/user@1001.service/app.slice/lintel-344e11f2-f95b-40d0-bc69-4034aa1fcd13.service\n",
            unit
        ));
        assert!(!cgroup_text_matches_unit(
            &format!("0::/system.slice/{unit}-neighbor\n"),
            unit
        ));
        // Still inside the login session scope must NOT match.
        assert!(!cgroup_text_matches_unit(
            "0::/user.slice/user-1001.slice/session-27.scope\n",
            unit
        ));
    }

    #[test]
    fn manager_command_carries_no_secret_and_exact_worker() {
        let selection = select_with(UUID, true, 0, false, false);
        let command = manager_command(
            &selection,
            &PathBuf::from("/opt/lintel/lintel"),
            &PathBuf::from("/home/synthetic"),
            &PathBuf::from("/home/synthetic/state"),
            "/usr/bin",
            r#"{"mode":"system_manager","unit":"lintel-344e11f2-f95b-40d0-bc69-4034aa1fcd13.service"}"#,
        )
        .unwrap();
        let args: Vec<String> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.contains(&"--pipe".to_string()));
        assert!(args.contains(&"--property=Restart=no".to_string()));
        assert!(
            args.contains(&"lintel-344e11f2-f95b-40d0-bc69-4034aa1fcd13.service".to_string())
                || args
                    .iter()
                    .any(|a| a == "--unit=lintel-344e11f2-f95b-40d0-bc69-4034aa1fcd13.service")
        );
        // exactly the worker subcommand after "--"
        let dashdash = args.iter().position(|a| a == "--").unwrap();
        assert_eq!(args[dashdash + 1], "/opt/lintel/lintel");
        assert_eq!(args[dashdash + 2], "__worker");
        // no passphrase or request body ever appears in argv
        assert!(!args.iter().any(|a| a.contains("passphrase")));
    }
}

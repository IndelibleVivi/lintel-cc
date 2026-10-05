//! Readonly execution-context resolver.
//!
//! This deliberately does **not** construct an `Engine`: it must be usable
//! before any state exists, and it must never create a state directory, run
//! discovery or start Claude. It reports where state *would* live and whether
//! that path already exists, plus static Claude discovery facts.
use crate::err;
use serde_json::{json, Value};
use std::{fs, os::unix::fs::PermissionsExt, path::{Path, PathBuf}};

/// Resolve (home, state, state_was_explicit) without touching the filesystem.
/// Overrides are injected so tests can exercise this without mutating the
/// process environment that other parallel tests share.
fn resolve_paths(
    home_override: Option<std::ffi::OsString>,
    state_override: Option<std::ffi::OsString>,
) -> crate::Result<(PathBuf, PathBuf, bool)> {
    let home = home_override
        .or_else(|| std::env::var_os("LINTEL_TEST_HOME"))
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .ok_or_else(|| err("home_missing", "找不到用户目录"))?;
    let state_from_env = state_override.or_else(|| std::env::var_os("LINTEL_STATE_DIR"));
    let explicit = state_from_env.is_some();
    let state = state_from_env.map(PathBuf::from).unwrap_or_else(|| {
        if cfg!(target_os = "macos") {
            home.join("Library/Application Support/Lintel")
        } else {
            home.join(".local/share/lintel")
        }
    });
    Ok((home, state, explicit))
}

fn runtime_paths_readonly() -> crate::Result<(PathBuf, PathBuf)> {
    let (home, state, _) = resolve_paths(None, None)?;
    Ok((home, state))
}

/// Static Claude discovery: PATH precedence, then this user's native install.
/// Never executed.
fn discover_executable(home: &Path, path: Option<&std::ffi::OsStr>) -> Option<String> {
    let candidates = path
        .into_iter()
        .flat_map(std::env::split_paths)
        .map(|dir| dir.join("claude"))
        .chain(std::iter::once(home.join(".local/bin/claude")));
    candidates
        .filter(|p| fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0))
        .find_map(|p| p.canonicalize().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

fn build(home: &Path, state: &Path, explicit: bool, path: Option<&std::ffi::OsStr>) -> Value {
    let config_home = home.join(".claude");
    json!({
        "product": "Lintel",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": 1,
        "catalog_version": 1,
        "platform": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
        "user": {
            "uid": unsafe { libc::getuid() },
            "euid": unsafe { libc::geteuid() },
            "home": home.to_string_lossy(),
        },
        "state": {
            "source": if explicit { "LINTEL_STATE_DIR" } else { "platform_default" },
            "path": state.to_string_lossy(),
            "exists": state.exists(),
        },
        "config_home": { "path": config_home.to_string_lossy(), "exists": config_home.exists() },
        "executable": discover_executable(home, path),
        "initialized": state.exists(),
        "notes": [
            "本命令不创建 state、不执行 discover，也不启动 Claude。",
            "configured 与 observed 是不同事实：这里只报告路径与静态发现。"
        ]
    })
}

pub fn context_value() -> Value {
    match runtime_paths_readonly() {
        Ok((home, state)) => build(&home, &state, std::env::var_os("LINTEL_STATE_DIR").is_some(), std::env::var_os("PATH").as_deref()),
        Err(error) => json!({"error":{"code":error.code,"message":error.message}}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_paths_are_resolved_without_touching_the_process_env() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        fs::create_dir(&home).unwrap();
        let state = temp.path().join("state");
        let (resolved_home, resolved_state, explicit) = resolve_paths(
            Some(home.clone().into_os_string()),
            Some(state.clone().into_os_string()),
        )
        .unwrap();
        assert_eq!(resolved_home, home);
        assert_eq!(resolved_state, state);
        assert!(explicit);
        // The resolver must never create anything.
        assert!(!state.exists());
        let value = build(&resolved_home, &resolved_state, explicit, None);
        assert_eq!(value["state"]["exists"], false);
        assert_eq!(value["state"]["source"], "LINTEL_STATE_DIR");
        assert_eq!(value["initialized"], false);
        assert!(!state.exists(), "resolver created state");
    }

    #[test]
    fn default_state_path_depends_on_home_only() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        fs::create_dir(&home).unwrap();
        let (_, state, explicit) =
            resolve_paths(Some(home.clone().into_os_string()), None).unwrap();
        assert!(!explicit);
        if cfg!(target_os = "macos") {
            assert_eq!(state, home.join("Library/Application Support/Lintel"));
        } else {
            assert_eq!(state, home.join(".local/share/lintel"));
        }
    }
}

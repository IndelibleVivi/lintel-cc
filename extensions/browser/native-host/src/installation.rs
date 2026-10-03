//! Explicit local installer. This module is never exposed to extension messages.
use crate::manifest;
use serde_json::{json, Value};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::{fs, io::Write, path::Path};

pub fn plan(
    browser: &str,
    extension: &str,
    host: &Path,
    home: &Path,
    platform: &str,
) -> Result<Value, String> {
    if !home.is_absolute() {
        return Err("home_must_be_absolute".into());
    }
    let relative = match (platform, browser) {
        ("macos", "chrome") => "Library/Application Support/Google/Chrome/NativeMessagingHosts",
        ("macos", "edge") => "Library/Application Support/Microsoft Edge/NativeMessagingHosts",
        ("macos", "firefox") => "Library/Application Support/Mozilla/NativeMessagingHosts",
        ("linux", "chrome") => ".config/google-chrome/NativeMessagingHosts",
        ("linux", "edge") => ".config/microsoft-edge/NativeMessagingHosts",
        ("linux", "firefox") => ".mozilla/native-messaging-hosts",
        _ => return Err("unsupported_browser_platform".into()),
    };
    let state = if platform == "macos" {
        "Library/Application Support/Lintel/browser-bridge"
    } else {
        ".local/state/lintel/browser-bridge"
    };
    Ok(
        json!({"browser":browser,"extension_id":extension,"manifest_path":home.join(relative).join("app.lintel.browser.json"),"manifest":manifest(browser,extension,host)?,"state_path":home.join(state),"effect":"register exact extension and authorize native host; pairing still requires short-code approval"}),
    )
}

pub fn install(
    browser: &str,
    extension: &str,
    host: &Path,
    home: &Path,
    platform: &str,
) -> Result<Value, String> {
    let plan = plan(browser, extension, host, home, platform)?;
    apply(plan, None)
}

/// App installer entry after it has checked its ownership record and the full
/// user-approved preview. The expected registration is null only when absent.
/// This remains installer-only: native extension messages cannot reach it.
pub fn install_reviewed(
    browser: &str,
    extension: &str,
    host: &Path,
    home: &Path,
    platform: &str,
    approved_existing: &Value,
) -> Result<Value, String> {
    let plan = plan(browser, extension, host, home, platform)?;
    apply(plan, Some(approved_existing))
}

fn apply(plan: Value, approved_existing: Option<&Value>) -> Result<Value, String> {
    let host = Path::new(plan["manifest"]["path"].as_str().unwrap());
    let host_meta = fs::metadata(host).map_err(|e| format!("host_not_available:{e}"))?;
    if !host_meta.is_file() {
        return Err("host_not_file".into());
    }
    #[cfg(unix)]
    if host_meta.permissions().mode() & 0o111 == 0 {
        return Err("host_not_executable".into());
    }
    let target = Path::new(plan["manifest_path"].as_str().unwrap());
    // A pre-existing different registration belongs to an explicit upgrade decision.
    let existing = match fs::symlink_metadata(target) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                return Err("manifest_symlink".into());
            }
            let existing: Value =
                serde_json::from_slice(&fs::read(target).map_err(|e| e.to_string())?)
                    .map_err(|e| format!("manifest_conflict:{e}"))?;
            if approved_existing.is_some_and(|approved| approved != &existing) {
                return Err("stale_plan:registration changed after preview".into());
            }
            if approved_existing.is_none() && existing != plan["manifest"] {
                return Err(
                    "manifest_conflict:existing registration differs; review it before replacing"
                        .into(),
                );
            }
            Some(existing)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if approved_existing.is_some_and(|approved| !approved.is_null()) {
                return Err("stale_plan:registration disappeared after preview".into());
            }
            None
        }
        Err(e) => return Err(e.to_string()),
    };
    // The exact registration and any reviewed previous manifest have now been
    // validated. Complete the installer-only authorization before publishing a
    // manifest: an invalid bridge DB must not leave an unowned registration
    // that blocks the user's repaired-state retry.
    crate::authorize_extension(
        Path::new(plan["state_path"].as_str().unwrap()),
        plan["extension_id"].as_str().unwrap(),
    )
    .map_err(|e| format!("authorization_failed:{e}"))?;
    let existed = existing.is_some();
    let changed = existing
        .as_ref()
        .is_none_or(|value| value != &plan["manifest"]);
    if changed {
        fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
        let write_path = if existed {
            target.with_extension(format!("{}.tmp", crate::random()))
        } else {
            target.to_path_buf()
        };
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&write_path).map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec_pretty(&plan["manifest"]).unwrap())
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        if existed {
            fs::rename(write_path, target).map_err(|e| e.to_string())?;
        }
    }
    Ok(
        json!({"status":if existed && changed {"updated"} else if existed {"already-registered"} else {"registered"},"plan":plan,"pairing":"required"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plans_and_idempotent_install_cover_supported_browsers() {
        let home = std::env::temp_dir().join(format!("lintel-install-{}", crate::random()));
        fs::create_dir_all(&home).unwrap();
        let host = home.join("synthetic-host");
        fs::write(&host, b"synthetic fixture").unwrap();
        #[cfg(unix)]
        fs::set_permissions(&host, fs::Permissions::from_mode(0o700)).unwrap();
        for platform in ["macos", "linux"] {
            for browser in ["chrome", "edge", "firefox"] {
                let extension = if browser == "firefox" {
                    "lintel@lintel.local"
                } else {
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                };
                let p = plan(browser, extension, &host, &home, platform).unwrap();
                assert!(!Path::new(p["manifest_path"].as_str().unwrap()).exists());
                assert_eq!(
                    install(browser, extension, &host, &home, platform).unwrap()["status"],
                    "registered"
                );
                assert_eq!(
                    install(browser, extension, &host, &home, platform).unwrap()["status"],
                    "already-registered"
                );
                fs::write(p["manifest_path"].as_str().unwrap(), b"{}").unwrap();
                assert!(install(browser, extension, &host, &home, platform)
                    .unwrap_err()
                    .starts_with("manifest_conflict"));
            }
        }
        assert!(manifest("chrome", "*", &host).is_err());
        assert!(manifest("firefox", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", &host).is_err());
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn reviewed_registration_rechecks_exact_previous_manifest() {
        let home =
            std::env::temp_dir().join(format!("lintel-reviewed-install-{}", crate::random()));
        fs::create_dir_all(&home).unwrap();
        let host = home.join("synthetic-host");
        fs::write(&host, b"synthetic host").unwrap();
        #[cfg(unix)]
        fs::set_permissions(&host, fs::Permissions::from_mode(0o700)).unwrap();
        let id = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let p = plan("chrome", id, &host, &home, "linux").unwrap();
        install_reviewed("chrome", id, &host, &home, "linux", &Value::Null).unwrap();
        assert!(
            install_reviewed("chrome", id, &host, &home, "linux", &Value::Null)
                .unwrap_err()
                .starts_with("stale_plan:")
        );
        let new_id = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        assert!(install("chrome", new_id, &host, &home, "linux")
            .unwrap_err()
            .starts_with("manifest_conflict:"));
        let reviewed =
            install_reviewed("chrome", new_id, &host, &home, "linux", &p["manifest"]).unwrap();
        assert_eq!(reviewed["status"], "updated");
        let current = fs::read(p["manifest_path"].as_str().unwrap()).unwrap();
        assert!(
            install_reviewed("chrome", id, &host, &home, "linux", &p["manifest"])
                .unwrap_err()
                .starts_with("stale_plan:")
        );
        assert_eq!(
            fs::read(p["manifest_path"].as_str().unwrap()).unwrap(),
            current
        );
        fs::remove_dir_all(home).unwrap();
    }
}

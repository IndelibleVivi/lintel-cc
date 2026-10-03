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
    let existed = match fs::symlink_metadata(target) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                return Err("manifest_symlink".into());
            }
            let existing: Value =
                serde_json::from_slice(&fs::read(target).map_err(|e| e.to_string())?)
                    .map_err(|e| format!("manifest_conflict:{e}"))?;
            if existing != plan["manifest"] {
                return Err(
                    "manifest_conflict:existing registration differs; review it before replacing"
                        .into(),
                );
            }
            true
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(e.to_string()),
    };
    if !existed {
        fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(target).map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec_pretty(&plan["manifest"]).unwrap())
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    }
    crate::authorize_extension(Path::new(plan["state_path"].as_str().unwrap()), extension)
        .map_err(|e| format!("authorization_failed:{e}"))?;
    Ok(
        json!({"status":if existed {"already-registered"} else {"registered"},"plan":plan,"pairing":"required"}),
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
}

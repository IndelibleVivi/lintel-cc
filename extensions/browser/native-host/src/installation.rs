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
    apply(plan, None, &mut |_, _| Ok(()))
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
    apply(plan, Some(approved_existing), &mut |_, _| Ok(()))
}

pub fn install_reviewed_tracked(
    browser: &str,
    extension: &str,
    host: &Path,
    home: &Path,
    platform: &str,
    approved_existing: &Value,
    before_publish: &mut dyn FnMut(&Path, &Path) -> Result<(), String>,
) -> Result<Value, String> {
    apply(
        plan(browser, extension, host, home, platform)?,
        Some(approved_existing),
        before_publish,
    )
}

/// Cleans only the exact temporary inode created by this call.
pub struct OwnedStaging {
    path: std::path::PathBuf,
    identity: Value,
    armed: bool,
}
impl OwnedStaging {
    pub fn new(path: &Path) -> Result<Self, String> {
        let mut value = identity(path)?;
        value.as_object_mut().unwrap().remove("size");
        Ok(Self {
            path: path.into(),
            identity: value,
            armed: true,
        })
    }
    pub fn disarm(&mut self) {
        self.armed = false;
    }
}
impl Drop for OwnedStaging {
    fn drop(&mut self) {
        if self.armed {
            if let Ok(mut value) = identity(&self.path) {
                value.as_object_mut().unwrap().remove("size");
                if value == self.identity {
                    let _ = fs::remove_file(&self.path);
                }
            }
        }
    }
}

fn identity(path: &Path) -> Result<Value, String> {
    use std::os::unix::fs::MetadataExt;
    let m = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    Ok(
        json!({"dev":m.dev(),"ino":m.ino(),"size":m.len(),"birth":m.created().map_err(|e| format!("identity_birth_unavailable:{e}"))?.duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos().to_string()}),
    )
}
fn sync_parent(path: &Path) -> Result<(), String> {
    fs::File::open(path.parent().unwrap())
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())
}
fn bounded_json(path: &Path) -> Result<Value, String> {
    use std::io::Read;
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| e.to_string())?;
    let m = file.metadata().map_err(|e| e.to_string())?;
    if !m.is_file() || m.len() > 1024 * 1024 {
        return Err("invalid_publication_record".into());
    }
    let mut bytes = Vec::new();
    file.take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("invalid_publication_record".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
fn owned(path: &Path, expected: &Value, content: &Value) -> Result<(), String> {
    if identity(path)? != *expected || bounded_json(path)? != *content {
        return Err("registration_reconciliation:owned object changed".into());
    }
    Ok(())
}
fn recover_publication(
    target: &Path,
    next: &Value,
    previous: Option<&Value>,
) -> Result<bool, String> {
    let journal = target.with_extension("lintel-publication-intent.json");
    if !journal.exists() {
        return Ok(false);
    }
    let j = bounded_json(&journal)?;
    if j["target"] != target.to_string_lossy().as_ref()
        || j["next"] != *next
        || previous != Some(&j["previous"])
    {
        return Err("registration_reconciliation:invalid approval or target".into());
    }
    let backup = Path::new(j["backup"].as_str().ok_or("invalid publication intent")?);
    let staged = Path::new(j["staged"].as_str().ok_or("invalid publication intent")?);
    let prefix = format!("{}.", target.file_stem().unwrap().to_string_lossy());
    if [backup, staged].iter().any(|p| {
        p.parent() != target.parent()
            || !p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .starts_with(&prefix)
    }) || staged.extension().is_none_or(|e| e != "tmp")
        || backup != staged.with_extension("previous-registration")
    {
        return Err("registration_reconciliation:invalid locators".into());
    }
    // Validate every retained object before deleting any of them.
    if staged.exists() {
        owned(staged, &j["next_identity"], next)?;
    }
    if backup.exists() {
        owned(backup, &j["previous_identity"], &j["previous"])?;
    }
    if !backup.exists() && target.exists() && identity(target)? == j["previous_identity"] {
        owned(target, &j["previous_identity"], &j["previous"])?;
        if staged.exists() {
            fs::remove_file(staged).map_err(|e| e.to_string())?;
        }
        fs::remove_file(&journal).map_err(|e| e.to_string())?;
        sync_parent(target)?;
        return Ok(false);
    }
    if !target.exists() && backup.exists() {
        owned(staged, &j["next_identity"], next)?;
        fs::hard_link(staged, target).map_err(|e| e.to_string())?;
    }
    owned(target, &j["next_identity"], next)?;
    sync_parent(target)?;
    for path in [staged, backup] {
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
    }
    fs::remove_file(&journal).map_err(|e| e.to_string())?;
    sync_parent(target)?;
    Ok(true)
}

fn move_absent(from: &Path, to: &Path) -> Result<(), String> {
    use std::os::unix::ffi::OsStrExt;
    let a = std::ffi::CString::new(from.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
    let b = std::ffi::CString::new(to.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    let result = unsafe { libc::renamex_np(a.as_ptr(), b.as_ptr(), libc::RENAME_EXCL) };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            libc::AT_FDCWD,
            a.as_ptr(),
            libc::AT_FDCWD,
            b.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

fn apply(
    plan: Value,
    approved_existing: Option<&Value>,
    before_publish: &mut dyn FnMut(&Path, &Path) -> Result<(), String>,
) -> Result<Value, String> {
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
    let recovered = recover_publication(target, &plan["manifest"], approved_existing)?;
    let recovered_expected = if recovered {
        Some(&plan["manifest"])
    } else {
        approved_existing
    };
    // A pre-existing different registration belongs to an explicit upgrade decision.
    let existing = match fs::symlink_metadata(target) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                return Err("manifest_symlink".into());
            }
            let existing: Value =
                serde_json::from_slice(&fs::read(target).map_err(|e| e.to_string())?)
                    .map_err(|e| format!("manifest_conflict:{e}"))?;
            if recovered_expected.is_some_and(|approved| approved != &existing) {
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
        let write_path = target.with_extension(format!("{}.tmp", crate::random()));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&write_path).map_err(|e| e.to_string())?;
        let mut staging_guard = OwnedStaging::new(&write_path)?;
        file.write_all(&serde_json::to_vec_pretty(&plan["manifest"]).unwrap())
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        before_publish(&write_path, target)?;
        if existed {
            let latest: Value =
                serde_json::from_slice(&fs::read(target).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            if Some(&latest) != existing.as_ref() {
                return Err("stale_plan:registration changed before publication".into());
            }
            // Freeze recovery locators before moving any existing registration.
            let backup = write_path.with_extension("previous-registration");
            let journal = target.with_extension("lintel-publication-intent.json");
            let journal_staging = journal.with_extension(format!("{}.tmp", crate::random()));
            let mut jf = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&journal_staging)
                .map_err(|e| e.to_string())?;
            let _journal_guard = OwnedStaging::new(&journal_staging)?;
            jf.write_all(&serde_json::to_vec(&json!({"target":target,"staged":write_path,"backup":backup,"previous":existing,"next":plan["manifest"],"previous_identity":identity(target)?,"next_identity":identity(&write_path)?})).unwrap()).map_err(|e| e.to_string())?;
            jf.sync_all().map_err(|e| e.to_string())?;
            let expected_journal = bounded_json(&journal_staging)?;
            fs::hard_link(&journal_staging, &journal).map_err(|e| e.to_string())?;
            staging_guard.disarm(); // durable recovery record now owns this stage
            sync_parent(&journal)?;
            if bounded_json(&journal)? != expected_journal {
                return Err("publication_intent_readback".into());
            }
            fs::File::open(target.parent().unwrap())
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())?;
            move_absent(target, &backup)
                .map_err(|e| format!("registration_reconciliation:{}:{e}", journal.display()))?;
            let moved: Value = serde_json::from_slice(
                &fs::read(&backup).map_err(|e| e.to_string())?,
            )
            .map_err(|e| format!("registration_reconciliation:{}:{e}", journal.display()))?;
            if Some(&moved) != existing.as_ref() {
                return Err(format!("registration_reconciliation:{}:moved registration changed; original retained at {}",journal.display(),backup.display()));
            }
            fs::hard_link(&write_path, target).map_err(|e| {
                format!(
                    "registration_reconciliation:{}:{e}; previous retained at {}",
                    journal.display(),
                    backup.display()
                )
            })?;
            recover_publication(target, &plan["manifest"], approved_existing)?;
        } else {
            fs::hard_link(&write_path, target).map_err(|e| e.to_string())?;
            fs::remove_file(&write_path).map_err(|e| e.to_string())?;
        }
        fs::File::open(target.parent().unwrap())
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        let actual: Value = serde_json::from_slice(&fs::read(target).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if actual != plan["manifest"] {
            return Err("manifest_conflict:publication readback".into());
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
    fn publication_recovery_covers_before_move_after_move_and_after_publish() {
        for phase in 0..3 {
            let root = std::env::temp_dir().join(format!("lintel-publication-{}", crate::random()));
            fs::create_dir_all(&root).unwrap();
            let target = root.join("manifest.json");
            let staged = root.join("manifest.synthetic.tmp");
            let backup = staged.with_extension("previous-registration");
            let previous = json!({"value":"previous"});
            let next = json!({"value":"next"});
            fs::write(&target, serde_json::to_vec(&previous).unwrap()).unwrap();
            fs::write(&staged, serde_json::to_vec(&next).unwrap()).unwrap();
            let journal = target.with_extension("lintel-publication-intent.json");
            fs::write(&journal, serde_json::to_vec(&json!({"target":target,"staged":staged,"backup":backup,"previous":previous,"next":next,"previous_identity":identity(&target).unwrap(),"next_identity":identity(&staged).unwrap()})).unwrap()).unwrap();
            if phase > 0 {
                move_absent(&target, &backup).unwrap();
            }
            if phase > 1 {
                fs::hard_link(&staged, &target).unwrap();
            }
            if phase == 2 {
                let original = fs::read(&target).unwrap();
                let mut changed = original.clone();
                let at = changed.iter().position(|b| *b == b'n').unwrap();
                changed[at] = b'x';
                fs::write(&target, &changed).unwrap();
                assert!(recover_publication(&target, &next, Some(&previous)).is_err());
                assert!(backup.exists());
                assert!(staged.exists());
                assert!(journal.exists());
                fs::write(&target, original).unwrap();
            }
            recover_publication(&target, &next, Some(&previous)).unwrap();
            let actual: Value = serde_json::from_slice(&fs::read(&target).unwrap()).unwrap();
            assert_eq!(actual, if phase == 0 { previous } else { next });
            assert!(!staged.exists());
            assert!(!backup.exists());
            assert!(!journal.exists());
            fs::remove_dir_all(root).unwrap();
        }
    }

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
        let error = install_reviewed_tracked(
            "chrome",
            new_id,
            &host,
            &home,
            "linux",
            &p["manifest"],
            &mut |_, _| Err("synthetic callback failure".into()),
        )
        .unwrap_err();
        assert_eq!(error, "synthetic callback failure");
        let directory = Path::new(p["manifest_path"].as_str().unwrap())
            .parent()
            .unwrap();
        assert!(!fs::read_dir(directory).unwrap().any(|e| e
            .unwrap()
            .path()
            .extension()
            .is_some_and(|x| x == "tmp")));
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

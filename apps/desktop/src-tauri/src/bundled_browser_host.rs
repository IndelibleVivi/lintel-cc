//! Explicit approval of the App-supplied Native Messaging executable.
//! Preview is read-only; no browser profile or browser storage is opened here.
use fs2::FileExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, String>;
const INSTALLER: &str = "lintel-desktop-bundled-host";
const MAX_BINARY: u64 = 32 * 1024 * 1024;

pub(super) fn envelope(result: Result<Value>) -> Value {
    match result {
        Ok(data) => json!({"ok":true,"data":data}),
        Err(error) => {
            let (code, message) = error.split_once(':').unwrap_or((&error, &error));
            json!({"ok":false,"error":{"code":code,"message":message}})
        }
    }
}
fn fields(value: &Value, allowed: &[&str]) -> Result<()> {
    let object = value
        .as_object()
        .ok_or("invalid_object:请求必须是 object")?;
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err("unknown_field:请求包含不支持的字段".into());
    }
    Ok(())
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("missing_field:{key}"))
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &Path) -> Result<Option<Vec<u8>>> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW);
    let mut file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("storage_error:{error}")),
    };
    if !file
        .metadata()
        .map_err(|e| format!("storage_error:{e}"))?
        .is_file()
    {
        return Err("storage_error:目标不是普通文件".into());
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_BINARY + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("storage_error:{e}"))?;
    if bytes.len() as u64 > MAX_BINARY {
        return Err("storage_error:文件超出本地 host 资源上限".into());
    }
    Ok(Some(bytes))
}
fn load(path: &Path) -> Result<Value> {
    match read(path)? {
        Some(bytes) => {
            serde_json::from_slice(&bytes).map_err(|e| format!("storage_error:无效 JSON：{e}"))
        }
        None => Ok(Value::Null),
    }
}
fn private_dir(path: &Path) -> Result<()> {
    if path
        .symlink_metadata()
        .is_ok_and(|meta| meta.file_type().is_symlink())
    {
        return Err("storage_error:安装目录不能是 symbolic link".into());
    }
    fs::create_dir_all(path).map_err(|e| format!("storage_error:{e}"))?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|e| format!("storage_error:{e}"))?;
    Ok(())
}
fn save(path: &Path, value: &Value) -> Result<()> {
    let temp = path.with_extension(format!("{}.tmp", lintel_browser_host::random()));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&temp)
        .map_err(|e| format!("storage_error:{e}"))?;
    file.write_all(&serde_json::to_vec_pretty(value).unwrap())
        .map_err(|e| format!("storage_error:{e}"))?;
    file.sync_all().map_err(|e| format!("storage_error:{e}"))?;
    fs::rename(temp, path).map_err(|e| format!("storage_error:{e}"))?;
    Ok(())
}

struct Bundle {
    meta: Value,
    bytes: Vec<u8>,
}
pub(super) struct Installer {
    home: PathBuf,
    platform: String,
    architecture: String,
    resources: PathBuf,
}
impl Installer {
    pub(super) fn system(resources: PathBuf) -> Result<Self> {
        if std::env::consts::OS != "macos" {
            return Err("platform_unsupported:App 内置 browser host 安装目前支持 macOS；Linux runner 可使用独立 CLI installer".into());
        }
        let home = PathBuf::from(
            std::env::var_os("HOME").ok_or("home_unavailable:无法确定当前用户 home")?,
        );
        Ok(Self {
            home,
            platform: std::env::consts::OS.into(),
            architecture: std::env::consts::ARCH.into(),
            resources,
        })
    }
    fn install_root(&self) -> PathBuf {
        self.home.join(if self.platform == "macos" {
            "Library/Application Support/Lintel/browser-host"
        } else {
            ".local/share/lintel/browser-host"
        })
    }
    fn receipt_path(&self, browser: &str) -> PathBuf {
        self.install_root()
            .join("registrations")
            .join(format!("{browser}.json"))
    }
    fn bundle(&self) -> Result<Bundle> {
        let meta = load(&self.resources.join("manifest.json")).map_err(|_| "bundle_unavailable:无法读取 App 内置 browser host metadata；请安装完整桌面构建，开发环境可运行 npm run prepare:browser-host".to_owned())?;
        if meta.is_null() {
            return Err("bundle_unavailable:此 App 缺少内置 browser host；请安装完整桌面构建，开发环境可运行 npm run prepare:browser-host".into());
        }
        let version = meta["version"].as_str().filter(|version| {
            !version.is_empty()
                && version.len() < 64
                && version
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b".-+".contains(&c))
        });
        let sha = meta["sha256"].as_str().filter(|sha| {
            sha.len() == 64
                && sha
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        });
        if meta["schema"] != 1
            || version.is_none()
            || sha.is_none()
            || meta["platform"] != self.platform
            || meta["architecture"] != self.architecture
        {
            return Err(
                "bundle_incompatible:内置 browser host 的版本、系统或架构 metadata 无效".into(),
            );
        }
        let bytes = read(&self.resources.join("lintel-browser-host"))?
            .ok_or("bundle_unavailable:App 内缺少 browser host executable；请使用完整桌面构建")?;
        if bytes.is_empty()
            || meta["bytes"].as_u64() != Some(bytes.len() as u64)
            || digest(&bytes) != sha.unwrap()
        {
            return Err("bundle_changed:内置 browser host 与 metadata 不一致；没有安装".into());
        }
        Ok(Bundle { meta, bytes })
    }
    fn file_identity(&self, path: &Path) -> Result<Value> {
        let Some(bytes) = read(path)? else {
            return Ok(Value::Null);
        };
        let metadata = fs::symlink_metadata(path).map_err(|e| format!("storage_error:{e}"))?;
        #[cfg(unix)]
        let mode = metadata.permissions().mode() & 0o777;
        #[cfg(not(unix))]
        let mode = 0;
        Ok(json!({"path":path,"sha256":digest(&bytes),"bytes":bytes.len(),"mode":mode}))
    }
    fn registration(plan: &Value) -> Value {
        let mut registration = json!({"installer":INSTALLER});
        for key in [
            "browser",
            "extension_id",
            "version",
            "sha256",
            "bytes",
            "host_path",
            "manifest_path",
            "manifest",
            "state_path",
        ] {
            registration[key] = plan[key].clone();
        }
        registration
    }
    fn plan_with(&self, browser: &str, extension: &str, bundle: &Bundle) -> Result<Value> {
        let host = self
            .install_root()
            .join(format!(
                "{}-{}",
                bundle.meta["version"].as_str().unwrap(),
                bundle.meta["sha256"].as_str().unwrap()
            ))
            .join("lintel-browser-host");
        // Reuse manifest and extension-ID rules from the CLI installer.
        let mut plan = lintel_browser_host::installation::plan(
            browser,
            extension,
            &host,
            &self.home,
            &self.platform,
        )?;
        plan["host_path"] = json!(host);
        for key in ["version", "sha256", "bytes"] {
            plan[key] = bundle.meta[key].clone();
        }
        plan["schema"] = json!(1);
        plan["platform"] = json!(self.platform);
        plan["architecture"] = json!(self.architecture);
        plan["effect"] = json!("将 App 内置 executable 安装到当前用户 Lintel 稳定目录；仅注册并授权所选 exact extension；pairing 仍需单独批准");
        let manifest_path = Path::new(plan["manifest_path"].as_str().unwrap());
        let existing_manifest = load(manifest_path)?;
        let receipt = load(&self.receipt_path(browser))?;
        let existing_registration = receipt.get("registration").cloned().unwrap_or(Value::Null);
        let installed_host = self.file_identity(&host)?;
        let existing_host = if let Some(path) = existing_manifest["path"].as_str() {
            // Only inspect binaries previously recorded by this installer.
            if existing_registration["installer"] == INSTALLER
                && existing_manifest == existing_registration["manifest"]
                && existing_registration["manifest_path"] == plan["manifest_path"]
            {
                self.file_identity(Path::new(path))?
            } else {
                Value::Null
            }
        } else {
            Value::Null
        };
        let owned = !existing_manifest.is_null()
            && existing_registration["installer"] == INSTALLER
            && existing_registration["browser"] == browser
            && existing_registration["manifest_path"] == plan["manifest_path"]
            && existing_registration["manifest"] == existing_manifest
            && existing_host["sha256"] == existing_registration["sha256"]
            && existing_host["bytes"] == existing_registration["bytes"]
            && existing_host["mode"] == 0o700;
        let desired = Self::registration(&plan);
        let host_matches = installed_host["sha256"] == plan["sha256"]
            && installed_host["bytes"] == plan["bytes"]
            && installed_host["mode"] == 0o700;
        let host_conflict = !installed_host.is_null() && !host_matches;
        let (status, action) = if host_conflict || (!existing_manifest.is_null() && !owned) {
            ("conflict", "blocked")
        } else if existing_manifest.is_null() {
            ("ready", "install")
        } else if existing_registration == desired && host_matches {
            ("already-registered", "none")
        } else {
            ("ready", "upgrade")
        };
        plan["existing_manifest"] = existing_manifest;
        plan["existing_registration"] = existing_registration;
        plan["existing_host"] = existing_host;
        plan["installed_host"] = installed_host;
        plan["status"] = json!(status);
        plan["install_action"] = json!(action);
        Ok(plan)
    }
    fn plan(&self, browser: &str, extension: &str) -> Result<Value> {
        // Validate finite browser/extension/home before reading App metadata.
        lintel_browser_host::installation::plan(
            browser,
            extension,
            &self.install_root().join("lintel-browser-host"),
            &self.home,
            &self.platform,
        )?;
        self.plan_with(browser, extension, &self.bundle()?)
    }
    fn install(&self, approved: &Value) -> Result<Value> {
        let browser = string(approved, "browser")?;
        let extension = string(approved, "extension_id")?;
        let bundle = self.bundle()?;
        let current = self.plan_with(browser, extension, &bundle)?;
        let receipt = load(&self.receipt_path(browser))?;
        // Exact successful replay is query-only, without reauthorizing or writing.
        if current["status"] == "already-registered"
            && (approved == &current || receipt["approved_plan"] == *approved)
        {
            return Ok(json!({"status":"already-registered","plan":current,"pairing":"required"}));
        }
        if approved != &current {
            return Err("stale_plan:资源或当前登记已变化；请重新生成并审核 preview".into());
        }
        if current["install_action"] == "blocked" {
            return Err("manifest_conflict:现有登记或 host 不属于此 bundled installer；保留原登记，请先人工核对".into());
        }
        // Serialize this installer's approvals, then recheck under the lock.
        let root = self.install_root();
        private_dir(&root)?;
        let mut options = fs::OpenOptions::new();
        options.create(true).read(true).write(true);
        #[cfg(unix)]
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        let lock = options
            .open(root.join("install.lock"))
            .map_err(|e| format!("storage_error:{e}"))?;
        lock.lock_exclusive()
            .map_err(|e| format!("storage_error:{e}"))?;
        let fresh_bundle = self.bundle()?;
        let fresh = self.plan_with(browser, extension, &fresh_bundle)?;
        if fresh != current {
            return Err("stale_plan:安装条件在批准后发生变化；请重新生成 preview".into());
        }
        let host = Path::new(current["host_path"].as_str().unwrap());
        private_dir(host.parent().unwrap())?;
        if current["installed_host"].is_null() {
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            options.mode(0o700);
            let mut file = options
                .open(host)
                .map_err(|e| format!("host_conflict:{e}"))?;
            file.write_all(&fresh_bundle.bytes)
                .map_err(|e| format!("storage_error:{e}"))?;
            #[cfg(unix)]
            file.set_permissions(fs::Permissions::from_mode(0o700))
                .map_err(|e| format!("storage_error:{e}"))?;
            file.sync_all().map_err(|e| format!("storage_error:{e}"))?;
        }
        let outcome = lintel_browser_host::installation::install_reviewed(
            browser,
            extension,
            host,
            &self.home,
            &self.platform,
            &current["existing_manifest"],
        )?;
        private_dir(&root.join("registrations"))?;
        save(
            &self.receipt_path(browser),
            &json!({"registration":Self::registration(&current),"approved_plan":approved}),
        )?;
        FileExt::unlock(&lock).map_err(|e| format!("storage_error:{e}"))?;
        Ok(
            json!({"status":outcome["status"],"plan":self.plan_with(browser, extension, &fresh_bundle)?,"pairing":"required"}),
        )
    }
    pub(super) fn dispatch(&self, payload: Value) -> Result<Value> {
        match string(&payload, "op")? {
            "bundled_host_plan" => {
                fields(&payload, &["op", "browser", "extension_id"])?;
                self.plan(
                    string(&payload, "browser")?,
                    string(&payload, "extension_id")?,
                )
            }
            "install_bundled_host" => {
                fields(&payload, &["op", "approved_plan"])?;
                self.install(
                    payload
                        .get("approved_plan")
                        .ok_or("missing_field:approved_plan")?,
                )
            }
            _ => Err("unsupported_browser_operation:不支持此 bundled host 操作".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const CHROME_ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    struct Fixture {
        root: tempfile::TempDir,
        installer: Installer,
    }
    impl Fixture {
        fn new(platform: &str) -> Self {
            let root = tempfile::tempdir().unwrap();
            let installer = Installer {
                home: root.path().join("synthetic-home"),
                resources: root.path().join("resources"),
                platform: platform.into(),
                architecture: "aarch64".into(),
            };
            let fixture = Self { root, installer };
            fixture.bundle(b"synthetic dedicated host v1", "0.1.0");
            fixture
        }
        fn bundle(&self, bytes: &[u8], version: &str) {
            fs::create_dir_all(&self.installer.resources).unwrap();
            fs::write(self.installer.resources.join("lintel-browser-host"), bytes).unwrap();
            fs::write(self.installer.resources.join("manifest.json"), serde_json::to_vec(&json!({"schema":1,"version":version,"platform":self.installer.platform,"architecture":self.installer.architecture,"sha256":digest(bytes),"bytes":bytes.len()})).unwrap()).unwrap();
        }
        fn plan(&self) -> Value {
            self.installer.plan("chrome", CHROME_ID).unwrap()
        }
        fn db(&self, plan: &Value) -> Value {
            load(&Path::new(plan["state_path"].as_str().unwrap()).join("bridge.json")).unwrap()
        }
    }
    #[test]
    fn preview_reads_only_and_validates_finite_requests() {
        let f = Fixture::new("macos");
        let p = f.plan();
        assert!(!f.installer.home.exists());
        assert_eq!(p["status"], "ready");
        assert_eq!(p["install_action"], "install");
        assert!(p["existing_manifest"].is_null());
        assert!(p["host_path"]
            .as_str()
            .unwrap()
            .contains("Library/Application Support/Lintel/browser-host/0.1.0-"));
        assert_eq!(
            p["manifest"]["allowed_origins"],
            json!([format!("chrome-extension://{CHROME_ID}/")])
        );
        assert_eq!(envelope(f.installer.dispatch(json!({"op":"bundled_host_plan","browser":"chrome","extension_id":CHROME_ID,"host_path":"/arbitrary"})))["error"]["code"], "unknown_field");
        assert_eq!(
            envelope(f.installer.dispatch(json!({"op":"install_native_host"})))["error"]["code"],
            "unsupported_browser_operation"
        );
        assert!(f.installer.plan("chrome", "*").is_err());
        assert!(f.installer.plan("firefox", CHROME_ID).is_err());
        assert!(f.installer.plan("chromium", CHROME_ID).is_err());
        assert!(!f.installer.home.exists());
        // Keep the fixture root alive through every assertion.
        assert!(f.root.path().exists());
    }
    #[test]
    fn missing_and_changed_artifacts_leave_home_untouched() {
        let f = Fixture::new("macos");
        fs::remove_file(f.installer.resources.join("manifest.json")).unwrap();
        let error = envelope(f.installer.dispatch(
            json!({"op":"bundled_host_plan","browser":"chrome","extension_id":CHROME_ID}),
        ));
        assert_eq!(error["error"]["code"], "bundle_unavailable");
        assert!(error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("prepare:browser-host"));
        f.bundle(b"host", "0.1.0");
        fs::write(
            f.installer.resources.join("lintel-browser-host"),
            b"changed",
        )
        .unwrap();
        assert_eq!(
            envelope(f.installer.dispatch(
                json!({"op":"bundled_host_plan","browser":"chrome","extension_id":CHROME_ID})
            ))["error"]["code"],
            "bundle_changed"
        );
        assert!(!f.installer.home.exists());
    }
    #[test]
    fn installs_exact_binary_private_mode_and_only_selected_authorization() {
        for platform in ["macos", "linux"] {
            for browser in ["chrome", "edge", "firefox"] {
                let f = Fixture::new(platform);
                let extension = if browser == "firefox" {
                    "lintel@lintel.local"
                } else {
                    CHROME_ID
                };
                let p = f.installer.plan(browser, extension).unwrap();
                let result = f
                    .installer
                    .dispatch(json!({"op":"install_bundled_host","approved_plan":p}))
                    .unwrap();
                assert_eq!(result["status"], "registered");
                let host = Path::new(p["host_path"].as_str().unwrap());
                assert_eq!(fs::read(host).unwrap(), b"synthetic dedicated host v1");
                #[cfg(unix)]
                assert_eq!(
                    fs::metadata(host).unwrap().permissions().mode() & 0o777,
                    0o700
                );
                assert_eq!(
                    load(Path::new(p["manifest_path"].as_str().unwrap())).unwrap(),
                    p["manifest"]
                );
                let db = f.db(&p);
                assert_eq!(db["allowed_extensions"], json!([extension]));
                assert_eq!(db["pairings"], json!({}));
                assert_eq!(result["pairing"], "required");
                assert_eq!(result["plan"]["install_action"], "none");
            }
        }
    }
    #[test]
    fn successful_approval_replay_is_query_only() {
        let f = Fixture::new("macos");
        let p = f.plan();
        let result = f.installer.install(&p).unwrap();
        let state = Path::new(p["state_path"].as_str().unwrap()).join("bridge.json");
        let state_before = fs::read(&state).unwrap();
        let receipt_before = fs::read(f.installer.receipt_path("chrome")).unwrap();
        assert_eq!(
            f.installer.install(&p).unwrap()["status"],
            "already-registered"
        );
        assert_eq!(
            f.installer.install(&result["plan"]).unwrap()["status"],
            "already-registered"
        );
        assert_eq!(fs::read(&state).unwrap(), state_before);
        assert_eq!(
            fs::read(f.installer.receipt_path("chrome")).unwrap(),
            receipt_before
        );
    }
    #[test]
    fn stale_resource_and_registration_approvals_reject_without_installing() {
        let f = Fixture::new("macos");
        let p = f.plan();
        f.bundle(b"new bundled host", "0.2.0");
        assert!(f
            .installer
            .install(&p)
            .unwrap_err()
            .starts_with("stale_plan:"));
        assert!(!f.installer.home.exists());
        let p = f.plan();
        let manifest_path = Path::new(p["manifest_path"].as_str().unwrap());
        fs::create_dir_all(manifest_path.parent().unwrap()).unwrap();
        fs::write(manifest_path, b"{\"name\":\"unrelated\"}").unwrap();
        assert!(f
            .installer
            .install(&p)
            .unwrap_err()
            .starts_with("stale_plan:"));
        assert_eq!(
            fs::read(manifest_path).unwrap(),
            b"{\"name\":\"unrelated\"}"
        );
        assert!(!Path::new(p["host_path"].as_str().unwrap()).exists());
        assert!(!Path::new(p["state_path"].as_str().unwrap()).exists());
    }
    #[test]
    fn reviewed_owned_upgrade_keeps_old_host_and_requires_new_preview() {
        let f = Fixture::new("macos");
        let old = f.plan();
        f.installer.install(&old).unwrap();
        f.bundle(b"synthetic dedicated host v2", "0.2.0");
        assert!(f
            .installer
            .install(&old)
            .unwrap_err()
            .starts_with("stale_plan:"));
        let upgrade = f.plan();
        assert_eq!(upgrade["install_action"], "upgrade");
        assert_eq!(upgrade["existing_manifest"], old["manifest"]);
        assert_eq!(f.installer.install(&upgrade).unwrap()["status"], "updated");
        assert_eq!(
            fs::read(old["host_path"].as_str().unwrap()).unwrap(),
            b"synthetic dedicated host v1"
        );
        assert_eq!(
            load(Path::new(upgrade["manifest_path"].as_str().unwrap())).unwrap(),
            upgrade["manifest"]
        );
        assert_eq!(
            f.installer.install(&upgrade).unwrap()["status"],
            "already-registered"
        );
    }
    #[test]
    fn manual_registration_is_preserved_even_if_it_matches_new_manifest() {
        let f = Fixture::new("macos");
        let p = f.plan();
        let target = Path::new(p["manifest_path"].as_str().unwrap());
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, serde_json::to_vec(&p["manifest"]).unwrap()).unwrap();
        let before = fs::read(target).unwrap();
        let blocked = f.plan();
        assert_eq!(blocked["status"], "conflict");
        assert_eq!(blocked["install_action"], "blocked");
        assert!(f
            .installer
            .install(&blocked)
            .unwrap_err()
            .starts_with("manifest_conflict:"));
        assert_eq!(fs::read(target).unwrap(), before);
        assert!(!f.installer.install_root().exists());
    }
    #[test]
    fn modified_owned_registration_and_executable_are_not_overwritten() {
        let f = Fixture::new("macos");
        let p = f.plan();
        let result = f.installer.install(&p).unwrap();
        fs::write(p["host_path"].as_str().unwrap(), b"modified outside App").unwrap();
        assert!(f
            .installer
            .install(&result["plan"])
            .unwrap_err()
            .starts_with("stale_plan:"));
        let blocked = f.plan();
        assert_eq!(blocked["install_action"], "blocked");
        assert!(f
            .installer
            .install(&blocked)
            .unwrap_err()
            .starts_with("manifest_conflict:"));
        assert_eq!(
            fs::read(p["host_path"].as_str().unwrap()).unwrap(),
            b"modified outside App"
        );
        fs::write(p["manifest_path"].as_str().unwrap(), b"{}").unwrap();
        assert_eq!(f.plan()["status"], "conflict");
    }

    #[test]
    fn failed_authorization_does_not_leave_an_unowned_registration() {
        let f = Fixture::new("macos");
        let p = f.plan();
        let state = Path::new(p["state_path"].as_str().unwrap());
        fs::create_dir_all(state).unwrap();
        fs::write(state.join("bridge.json"), b"invalid synthetic state").unwrap();
        assert!(f
            .installer
            .install(&p)
            .unwrap_err()
            .starts_with("authorization_failed:"));
        assert!(!Path::new(p["manifest_path"].as_str().unwrap()).exists());
        assert!(!f.installer.receipt_path("chrome").exists());
        // Repair only the observed failing state and review a fresh preview.
        fs::write(
            state.join("bridge.json"),
            serde_json::to_vec(
                &json!({"pairings":{},"instances":{},"operations":{},"allowed_extensions":[]}),
            )
            .unwrap(),
        )
        .unwrap();
        let retry = f.plan();
        assert_eq!(retry["install_action"], "install");
        assert_eq!(f.installer.install(&retry).unwrap()["status"], "registered");
        assert_eq!(f.db(&retry)["allowed_extensions"], json!([CHROME_ID]));
    }

    #[test]
    #[ignore = "requires npm run prepare:browser-host; uses only a synthetic home"]
    fn built_host_runs_from_installed_manifest_and_returns_native_frames() {
        use std::process::{Command, Stdio};
        let root = tempfile::tempdir().unwrap();
        let installer = Installer {
            home: root.path().join("synthetic-home"),
            resources: std::env::var_os("LINTEL_TEST_BROWSER_HOST_RESOURCES")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("browser-host-bundle")
                }),
            platform: std::env::consts::OS.into(),
            architecture: std::env::consts::ARCH.into(),
        };
        let p = installer.plan("chrome", CHROME_ID).unwrap();
        // Build metadata is the host package version; the native protocol does
        // not expose a --version command or an unauthenticated version message.
        let cargo = include_str!("../../../../extensions/browser/native-host/Cargo.toml");
        assert!(cargo
            .lines()
            .any(|line| line == format!("version = \"{}\"", p["version"].as_str().unwrap())));
        installer.install(&p).unwrap();
        let manifest = load(Path::new(p["manifest_path"].as_str().unwrap())).unwrap();
        for (extension, expected) in [
            (CHROME_ID, "unknown_native_operation"),
            ("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "extension_not_allowed"),
        ] {
            let mut child = Command::new(manifest["path"].as_str().unwrap())
                .arg(format!("chrome-extension://{extension}/"))
                .env_clear()
                .env("HOME", &installer.home)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let mut input = child.stdin.take().unwrap();
            lintel_browser_host::write_frame(&mut input, &json!({"op":"synthetic_unsupported_operation","request_id":"bundled-host-framing"})).unwrap();
            drop(input);
            let output = child.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let mut cursor = std::io::Cursor::new(output.stdout);
            let response = lintel_browser_host::read_frame(&mut cursor)
                .unwrap()
                .unwrap();
            assert_eq!(response["ok"], false);
            assert_eq!(response["error"]["code"], expected);
            assert_eq!(response["request_id"], "bundled-host-framing");
            assert!(lintel_browser_host::read_frame(&mut cursor)
                .unwrap()
                .is_none());
        }
    }
}

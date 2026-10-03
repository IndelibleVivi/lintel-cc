//! App-owned extension files, independent of browser profiles and host registration.
//! Preview is read-only; installation requires equality with the complete frozen plan.
use fs2::FileExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

type Result<T> = std::result::Result<T, String>;
const INSTALLER: &str = "lintel-desktop-bundled-extension";
const OWNERSHIP: &str = ".lintel-installation.json";
const MAX_FILE: u64 = 4 * 1024 * 1024;
const MAX_PACKAGE: u64 = 16 * 1024 * 1024;

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
fn package(browser: &str) -> Result<&'static str> {
    match browser {
        "chrome" | "edge" => Ok("chromium"),
        "firefox" => Ok("firefox"),
        _ => Err("invalid_browser:只支持 chrome、edge、firefox".into()),
    }
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
        return Err("storage_error:扩展资源必须是普通文件".into());
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_FILE + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("storage_error:{e}"))?;
    if bytes.len() as u64 > MAX_FILE {
        return Err("storage_error:扩展文件超过大小上限".into());
    }
    Ok(Some(bytes))
}
fn load(path: &Path) -> Result<Value> {
    match read(path)? {
        None => Ok(Value::Null),
        Some(bytes) => {
            serde_json::from_slice(&bytes).map_err(|e| format!("storage_error:无效 JSON：{e}"))
        }
    }
}
fn regular_directory(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("storage_error:{e}")),
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => Ok(true),
        Ok(_) => Err("path_conflict:扩展目录不能是 symbolic link 或文件".into()),
    }
}
fn private_directory(path: &Path) -> Result<()> {
    regular_directory(path)?;
    fs::create_dir_all(path).map_err(|e| format!("storage_error:{e}"))?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|e| format!("storage_error:{e}"))?;
    Ok(())
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    let mut file = options
        .open(path)
        .map_err(|e| format!("storage_error:{e}"))?;
    file.write_all(bytes)
        .map_err(|e| format!("storage_error:{e}"))?;
    file.sync_all().map_err(|e| format!("storage_error:{e}"))
}
fn sync_directory(path: &Path) -> Result<()> {
    fs::File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| format!("storage_error:无法同步安装目录：{e}"))
}

struct File {
    path: String,
    bytes: Vec<u8>,
}
struct Tree {
    files: Vec<File>,
    directories: Vec<String>,
}
impl Tree {
    fn inventory(&self) -> Value {
        json!(self.files.iter().map(|file| json!({"path":file.path,"sha256":digest(&file.bytes),"bytes":file.bytes.len()})).collect::<Vec<_>>())
    }
    fn identity(&self) -> String {
        let identity = self
            .files
            .iter()
            .map(|file| {
                format!(
                    "{}\0{}\0{}\n",
                    file.path,
                    digest(&file.bytes),
                    file.bytes.len()
                )
            })
            .collect::<String>();
        digest(identity.as_bytes())
    }
    fn bytes(&self) -> u64 {
        self.files.iter().map(|file| file.bytes.len() as u64).sum()
    }
    fn file(&self, name: &str) -> Result<&[u8]> {
        self.files
            .iter()
            .find(|file| file.path == name)
            .map(|file| file.bytes.as_slice())
            .ok_or_else(|| format!("bundle_invalid:缺少扩展文件 {name}"))
    }
}
fn tree(path: &Path, managed: bool) -> Result<Option<Tree>> {
    if !regular_directory(path)? {
        return Ok(None);
    }
    fn walk(root: &Path, relative: &Path, managed: bool, output: &mut Tree) -> Result<()> {
        for entry in fs::read_dir(root.join(relative)).map_err(|e| format!("storage_error:{e}"))? {
            let entry = entry.map_err(|e| format!("storage_error:{e}"))?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "path_conflict:扩展文件名必须为 UTF-8")?;
            if managed && relative.as_os_str().is_empty() && name == OWNERSHIP {
                continue;
            }
            if name.is_empty()
                || name == "."
                || name == ".."
                || !name
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
            {
                return Err("path_conflict:扩展文件名不属于打包路径范围".into());
            }
            let relative = relative.join(name);
            let name = relative.to_str().unwrap().replace('\\', "/");
            let kind = entry
                .file_type()
                .map_err(|e| format!("storage_error:{e}"))?;
            if kind.is_dir() {
                output.directories.push(name);
                walk(root, &relative, managed, output)?;
            } else if kind.is_file() {
                let bytes =
                    read(&root.join(&relative))?.ok_or("storage_changed:扩展文件在读取期间消失")?;
                output.files.push(File { path: name, bytes });
                if output.bytes() > MAX_PACKAGE {
                    return Err("storage_error:扩展包超过大小上限".into());
                }
            } else {
                return Err("path_conflict:扩展目录不能包含 symbolic link 或特殊文件".into());
            }
        }
        Ok(())
    }
    let mut output = Tree {
        files: Vec::new(),
        directories: Vec::new(),
    };
    walk(path, Path::new(""), managed, &mut output)?;
    output.files.sort_by(|a, b| a.path.cmp(&b.path));
    output.directories.sort();
    Ok(Some(output))
}

struct Bundle {
    version: String,
    tree: Tree,
}
fn canonical_config(package: &str, version: &str, tree: &Tree) -> Result<()> {
    let manifest: Value = serde_json::from_slice(tree.file("manifest.json")?)
        .map_err(|_| "bundle_invalid:扩展 manifest 无效")?;
    let config =
        std::str::from_utf8(tree.file("config.js")?).map_err(|_| "bundle_invalid:扩展配置无效")?;
    let config: Value = serde_json::from_str(
        config
            .strip_prefix("export const CONFIG = ")
            .and_then(|text| text.trim_end().strip_suffix(';'))
            .ok_or("bundle_invalid:扩展配置必须来自 canonical builder")?,
    )
    .map_err(|_| "bundle_invalid:扩展配置无效")?;
    let sites = json!([
        {"origin":"https://claude.ai","domain":"claude.ai","label":"Claude","default":true},
        {"origin":"https://console.anthropic.com","domain":"anthropic.com","label":"Anthropic Console","default":false}
    ]);
    if config != json!({"browser":package,"fixture":false,"sites":sites})
        || manifest["name"] != "Lintel"
        || manifest["manifest_version"] != 3
        || manifest["version"] != version
        || manifest.get("host_permissions").is_some()
        || manifest["permissions"]
            != json!([
                "storage",
                "browsingData",
                "privacy",
                "nativeMessaging",
                "alarms"
            ])
        || (package == "chromium"
            && manifest["background"] != json!({"service_worker":"background.js","type":"module"}))
        || (package == "firefox"
            && (manifest["background"] != json!({"scripts":["background.js"],"type":"module"})
                || manifest["browser_specific_settings"]["gecko"]["id"] != "lintel@lintel.local"))
    {
        return Err("bundle_fixture_rejected:App 只接受 canonical 非 fixture 扩展；未安装".into());
    }
    Ok(())
}

pub(super) struct Installer {
    home: PathBuf,
    resources: PathBuf,
    platform: String,
    #[cfg(test)]
    interrupt_at: Option<&'static str>,
}
impl Installer {
    pub(super) fn system(resources: PathBuf) -> Result<Self> {
        if std::env::consts::OS != "macos" {
            return Err(
                "platform_unsupported:App 内置扩展文件安装与 Finder 打开目前只支持 macOS".into(),
            );
        }
        Ok(Self {
            home: PathBuf::from(
                std::env::var_os("HOME").ok_or("home_unavailable:无法确定当前用户 home")?,
            ),
            resources,
            platform: "macos".into(),
            #[cfg(test)]
            interrupt_at: None,
        })
    }
    fn root(&self) -> PathBuf {
        self.home
            .join("Library/Application Support/Lintel/browser-extensions")
    }
    fn destination(&self, package: &str) -> PathBuf {
        self.root().join(package)
    }
    fn scope(&self) -> Result<()> {
        if self.platform != "macos" {
            return Err("platform_unsupported:扩展安装目前只支持 macOS".into());
        }
        if !self.home.is_absolute()
            || self
                .home
                .components()
                .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
        {
            return Err("invalid_home:用户 home 必须是绝对路径".into());
        }
        let mut path = self.home.clone();
        regular_directory(&path)?;
        for part in [
            "Library",
            "Application Support",
            "Lintel",
            "browser-extensions",
        ] {
            path.push(part);
            regular_directory(&path)?;
        }
        Ok(())
    }
    fn bundle(&self, package: &str) -> Result<Bundle> {
        regular_directory(&self.resources)?;
        let meta = load(&self.resources.join("manifest.json"))?;
        if meta.is_null() {
            return Err("bundle_unavailable:此 App 缺少内置扩展；请使用完整 App，开发环境可运行 npm run prepare:browser-extension".into());
        }
        if meta["schema"] != 1 {
            return Err("bundle_invalid:内置扩展 metadata schema 无效".into());
        }
        let meta = &meta["packages"][package];
        let version = string(meta, "version")?;
        if version.is_empty()
            || version.len() > 64
            || !version
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b".-+".contains(&c))
        {
            return Err("bundle_invalid:内置扩展版本无效".into());
        }
        let tree = tree(&self.resources.join(package), false)?
            .ok_or("bundle_unavailable:App 内缺少扩展目录")?;
        let mut expected_directories = std::collections::BTreeSet::new();
        for file in &tree.files {
            let mut parent = Path::new(&file.path).parent();
            while let Some(path) = parent.filter(|path| !path.as_os_str().is_empty()) {
                expected_directories.insert(path.to_str().unwrap().replace('\\', "/"));
                parent = path.parent();
            }
        }
        if tree.files.is_empty()
            || tree.directories != expected_directories.into_iter().collect::<Vec<_>>()
            || tree.inventory() != meta["files"]
            || tree.bytes() != meta["bytes"].as_u64().unwrap_or(0)
            || tree.identity() != meta["sha256"]
        {
            return Err("bundle_changed:内置扩展文件与 metadata 不一致；没有安装".into());
        }
        if tree
            .files
            .iter()
            .any(|file| file.path == OWNERSHIP || file.path.contains("fixture"))
        {
            return Err(
                "bundle_fixture_rejected:App 资源不能包含 fixture 或安装 ownership 文件".into(),
            );
        }
        canonical_config(package, version, &tree)?;
        Ok(Bundle {
            version: version.into(),
            tree,
        })
    }
    fn installation(package: &str, destination: &Path, bundle: &Bundle) -> Value {
        json!({"schema":1,"installer":INSTALLER,"package":package,"extension_path":destination,"version":bundle.version,"sha256":bundle.tree.identity(),"bytes":bundle.tree.bytes(),"files":bundle.tree.inventory(),"directories":bundle.tree.directories})
    }
    fn snapshot(&self, package: &str) -> Result<(Value, Value, Value, bool)> {
        self.snapshot_at(package, &self.destination(package))
    }
    fn snapshot_at(&self, package: &str, path: &Path) -> Result<(Value, Value, Value, bool)> {
        let Some(tree) = tree(path, true)? else {
            return Ok((Value::Null, Value::Null, Value::Null, false));
        };
        let receipt = load(&path.join(OWNERSHIP))?;
        let installation = receipt.get("installation").cloned().unwrap_or(Value::Null);
        let owned = installation["schema"] == 1
            && installation["installer"] == INSTALLER
            && installation["package"] == package
            && installation["extension_path"] == json!(self.destination(package))
            && installation["files"] == tree.inventory()
            && installation["directories"] == json!(tree.directories)
            && installation["sha256"] == tree.identity()
            && installation["bytes"].as_u64() == Some(tree.bytes());
        Ok((installation, tree.inventory(), receipt, owned))
    }
    fn transaction_path(&self, package: &str) -> PathBuf {
        self.root().join(format!("{package}.transaction.json"))
    }
    fn transaction_paths(&self, package: &str, transaction: &Value) -> Result<(PathBuf, PathBuf)> {
        let id = string(transaction, "id")?;
        if transaction["schema"] != 1
            || transaction["installer"] != INSTALLER
            || transaction["package"] != package
            || id.len() != 64
            || !id
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            return Err(
                "extension_recovery_conflict:安装中断记录不属于此 installer；保留现有目录".into(),
            );
        }
        Ok((
            self.root().join(format!(".{package}.pending-{id}")),
            self.root().join(format!(".{package}.previous-{id}")),
        ))
    }
    fn recovery(&self, package: &str, bundle: &Bundle) -> Result<Value> {
        let transaction = load(&self.transaction_path(package))?;
        if transaction.is_null() {
            return Ok(Value::Null);
        }
        let (stage, previous) = self.transaction_paths(package, &transaction)?;
        let destination = self.snapshot(package)?;
        let old = self.snapshot_at(package, &previous)?;
        // A interrupted write may leave a partial staging ownership file;
        // freeze its identity without requiring that unfinished JSON to parse.
        let staged_files = tree(&stage, true)?
            .map(|tree| tree.inventory())
            .unwrap_or(Value::Null);
        let staged_record = read(&stage.join(OWNERSHIP))?
            .map(|bytes| json!({"sha256":digest(&bytes),"bytes":bytes.len()}))
            .unwrap_or(Value::Null);
        let desired = Self::installation(package, &self.destination(package), bundle);
        let original = &transaction["original"];
        let original_valid = original.is_null()
            || (original["installer"] == INSTALLER
                && original["package"] == package
                && original["extension_path"] == json!(self.destination(package)));
        let previous_remaining_owned = old.1.as_array().is_some_and(|files| {
            files.iter().all(|file| {
                original["files"]
                    .as_array()
                    .is_some_and(|expected| expected.contains(file))
            })
        }) && tree(&previous, true)?.is_some_and(|tree| {
            tree.directories.iter().all(|directory| {
                original["directories"]
                    .as_array()
                    .is_some_and(|expected| expected.contains(&json!(directory)))
            })
        });
        let phase = if transaction["desired"] != desired || !original_valid {
            "conflict"
        } else if destination.3
            && destination.0 == desired
            && (old.1.is_null() || (old.3 && old.0 == *original) || previous_remaining_owned)
        {
            "new-installed"
        } else if destination.1.is_null() && !original.is_null() && old.3 && old.0 == *original {
            "previous-moved"
        } else if old.1.is_null()
            && ((original.is_null() && destination.1.is_null())
                || (destination.3 && destination.0 == *original))
        {
            "prepared"
        } else {
            "conflict"
        };
        Ok(
            json!({"phase":phase,"action":"finish-approved-install","message":"上次扩展文件准备中断；批准当前完整预览后继续复制或切换，并清理已审查的 staging/旧受管目录；完成前不允许 Finder 打开","transaction":transaction,"stage_path":stage,"previous_path":previous,"stage_files":staged_files,"stage_record":staged_record,"previous_installation":old.0,"previous_files":old.1}),
        )
    }
    fn checkpoint(&self, phase: &str) -> Result<()> {
        #[cfg(test)]
        if self.interrupt_at == Some(phase) {
            return Err(format!("extension_install_interrupted:synthetic interruption after {phase}; reopen preview to recover"));
        }
        let _ = phase;
        Ok(())
    }
    fn plan_with(&self, browser: &str, bundle: &Bundle) -> Result<Value> {
        let package = package(browser)?;
        self.scope()?;
        let destination = self.destination(package);
        let (existing, files, _, owned) = self.snapshot(package)?;
        let desired = Self::installation(package, &destination, bundle);
        let recovery = self.recovery(package, bundle)?;
        let (status, action) = if !recovery.is_null() {
            if recovery["phase"] == "conflict" {
                ("conflict", "blocked")
            } else if recovery["transaction"]["original"].is_null() {
                ("ready", "install")
            } else {
                ("ready", "upgrade")
            }
        } else if files.is_null() {
            ("ready", "install")
        } else if !owned {
            ("conflict", "blocked")
        } else if existing == desired {
            ("already-installed", "none")
        } else {
            ("ready", "upgrade")
        };
        Ok(
            json!({"schema":1,"browser":browser,"package":package,"version":bundle.version,"sha256":bundle.tree.identity(),"bytes":bundle.tree.bytes(),"files":bundle.tree.inventory(),"extension_path":destination,"manifest_path":destination.join("manifest.json"),"resource_path":self.resources.join(package),"platform":self.platform,"effect":"把 App 的 canonical 扩展文件复制到当前用户固定 Lintel 目录；批准升级会替换原受管文件；不加载、注册、签名或配对浏览器，不修改 profile、偏好或 host allowlist","existing_installation":existing,"existing_files":files,"recovery":recovery,"status":status,"install_action":action}),
        )
    }
    fn plan(&self, browser: &str) -> Result<Value> {
        let package = package(browser)?;
        self.scope()?;
        self.plan_with(browser, &self.bundle(package)?)
    }
    fn install(&self, approved: &Value) -> Result<Value> {
        let browser = string(approved, "browser")?;
        let package = package(browser)?;
        self.scope()?;
        let bundle = self.bundle(package)?;
        let current = self.plan_with(browser, &bundle)?;
        let (_, _, receipt, _) = self.snapshot(package)?;
        if current["status"] == "already-installed"
            && (approved == &current || receipt["approved_plan"] == *approved)
        {
            return Ok(
                json!({"status":"already-installed","plan":current,"loading":"required","pairing":"required"}),
            );
        }
        if approved != &current {
            return Err("stale_plan:扩展资源或安装目录已变化；请重新预览并批准".into());
        }
        if current["install_action"] == "blocked" {
            return Err("extension_conflict:现有目录不属于此 installer 或受管文件已修改；保留目录，请人工核对".into());
        }
        private_directory(&self.root())?;
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        let lock = options
            .open(self.root().join("install.lock"))
            .map_err(|e| format!("storage_error:{e}"))?;
        lock.lock_exclusive()
            .map_err(|e| format!("storage_error:{e}"))?;
        let fresh = self.bundle(package)?;
        if self.plan_with(browser, &fresh)? != current {
            return Err("stale_plan:批准后的安装条件已变化；请重新预览".into());
        }
        let transaction_path = self.transaction_path(package);
        let transaction = if current["recovery"].is_null() {
            let transaction = json!({"schema":1,"installer":INSTALLER,"package":package,"id":lintel_browser_host::random(),"original":current["existing_installation"],"desired":Self::installation(package,&self.destination(package),&fresh),"approved_plan":approved});
            // Intent is durable before staging or moving the current directory.
            write_new(
                &transaction_path,
                &serde_json::to_vec_pretty(&transaction).unwrap(),
            )?;
            sync_directory(&self.root())?;
            transaction
        } else {
            current["recovery"]["transaction"].clone()
        };
        let (stage, previous) = self.transaction_paths(package, &transaction)?;
        let destination = self.destination(package);
        let destination_before = self.snapshot(package)?;
        let previous_before = self.snapshot_at(package, &previous)?;
        let already_switched =
            destination_before.3 && destination_before.0 == transaction["desired"];
        if !already_switched {
            // A fresh recovery approval includes the exact interrupted staging
            // inventory. Only this transaction's finite staging path is rebuilt.
            if regular_directory(&stage)? {
                fs::remove_dir_all(&stage).map_err(|e| {
                    format!(
                        "extension_install_interrupted:无法清理已批准的 staging：{e}；请重新预览"
                    )
                })?;
            }
            private_directory(&stage)?;
            for directory in &fresh.tree.directories {
                private_directory(&stage.join(directory))?;
            }
            for file in &fresh.tree.files {
                write_new(&stage.join(&file.path), &file.bytes)?;
            }
            write_new(&stage.join(OWNERSHIP), &serde_json::to_vec_pretty(&json!({"installation":Self::installation(package, &self.destination(package), &fresh),"approved_plan":approved})).unwrap())?;
            for directory in fresh.tree.directories.iter().rev() {
                sync_directory(&stage.join(directory))?;
            }
            sync_directory(&stage)?;
            // Recheck source and destination immediately before the directory switch.
            let checked = self.bundle(package)?;
            self.scope()?;
            if Self::installation(package, &destination, &checked) != transaction["desired"]
                || self.snapshot(package)? != destination_before
                || self.snapshot_at(package, &previous)? != previous_before
                || load(&transaction_path)? != transaction
            {
                return Err(
                    "stale_plan:写入前扩展资源或安装目录已变化；安装中断记录已保留，请重新预览"
                        .into(),
                );
            }
            self.checkpoint("staged")?;
            if !destination_before.1.is_null() {
                fs::rename(&destination, &previous).map_err(|e| {
                    format!("extension_install_interrupted:旧目录切换失败：{e}；请重新预览")
                })?;
                sync_directory(&self.root())?;
                self.checkpoint("previous-moved")?;
            }
            if let Err(error) = fs::rename(&stage, &destination) {
                if !destination_before.1.is_null() {
                    fs::rename(&previous, &destination).map_err(|e| format!("extension_install_interrupted:安装切换失败 ({error})，旧目录恢复失败 ({e})；请重新预览当前中断状态"))?;
                }
                return Err(format!(
                    "extension_install_interrupted:安装切换失败：{error}；请重新预览"
                ));
            }
            sync_directory(&self.root())?;
            self.checkpoint("new-installed")?;
        } else {
            // The files switched successfully before interruption. Persist the
            // newly approved recovery without rewriting them; use this same
            // finite staging directory for an atomic ownership-record rename.
            if regular_directory(&stage)? {
                fs::remove_dir_all(&stage).map_err(|e| {
                    format!(
                        "extension_install_interrupted:无法清理已批准的 staging：{e}；请重新预览"
                    )
                })?;
            }
            private_directory(&stage)?;
            write_new(
                &stage.join(OWNERSHIP),
                &serde_json::to_vec_pretty(
                    &json!({"installation":transaction["desired"],"approved_plan":approved}),
                )
                .unwrap(),
            )?;
            if self.snapshot(package)? != destination_before
                || self.snapshot_at(package, &previous)? != previous_before
                || load(&transaction_path)? != transaction
            {
                return Err("stale_plan:恢复批准后目录发生变化；请重新预览".into());
            }
            fs::rename(stage.join(OWNERSHIP), destination.join(OWNERSHIP)).map_err(|e| {
                format!("extension_install_interrupted:恢复批准记录未能保存：{e}；请重新预览")
            })?;
            sync_directory(&destination)?;
        }
        // Residual directories remain an explicit interrupted state until an
        // approved recovery has actually cleaned them, never a successful result.
        self.checkpoint("cleanup")?;
        for path in [&previous, &stage] {
            if regular_directory(path)? {
                fs::remove_dir_all(path).map_err(|e| {
                    format!("extension_install_interrupted:受管暂存目录清理失败：{e}；请重新预览")
                })?;
            }
        }
        sync_directory(&self.root())?;
        fs::remove_file(&transaction_path).map_err(|e| {
            format!("extension_install_interrupted:安装中断记录未能关闭：{e}；请重新预览")
        })?;
        sync_directory(&self.root())?;
        let upgrade = !transaction["original"].is_null();
        FileExt::unlock(&lock).map_err(|e| format!("storage_error:{e}"))?;
        Ok(
            json!({"status":if upgrade {"updated"} else {"installed"},"plan":self.plan_with(browser,&fresh)?,"loading":"required","pairing":"required"}),
        )
    }
    fn reveal_path(&self, browser: &str) -> Result<PathBuf> {
        let package = package(browser)?;
        self.scope()?;
        if !load(&self.transaction_path(package))?.is_null() {
            return Err("extension_install_interrupted:扩展文件准备尚未完成；请重新预览并批准继续，完成前不打开 Finder".into());
        }
        let (_, files, _, owned) = self.snapshot(package)?;
        if files.is_null() {
            return Err("extension_not_installed:请先预览并批准扩展文件安装".into());
        }
        if !owned {
            return Err("extension_conflict:固定扩展目录不是未修改的受管安装；没有打开".into());
        }
        Ok(self.destination(package))
    }
    fn reveal(&self, browser: &str) -> Result<Value> {
        let destination = self.reveal_path(browser)?;
        #[cfg(target_os = "macos")]
        {
            let status = std::process::Command::new("/usr/bin/open")
                .args(["-a", "Finder"])
                .arg(&destination)
                .status()
                .map_err(|e| format!("reveal_failed:无法打开 Finder：{e}"))?;
            if !status.success() {
                return Err("reveal_failed:Finder 未能打开扩展目录".into());
            }
            Ok(
                json!({"status":"revealed","browser":browser,"package":package(browser)?,"extension_path":destination}),
            )
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = destination;
            Err("platform_unsupported:扩展目录打开目前只支持 macOS Finder".into())
        }
    }
    pub(super) fn dispatch(&self, payload: Value) -> Result<Value> {
        match string(&payload, "op")? {
            "bundled_extension_plan" => {
                fields(&payload, &["op", "browser"])?;
                self.plan(string(&payload, "browser")?)
            }
            "install_bundled_extension" => {
                fields(&payload, &["op", "approved_plan"])?;
                self.install(
                    payload
                        .get("approved_plan")
                        .ok_or("missing_field:approved_plan")?,
                )
            }
            "reveal_bundled_extension" => {
                fields(&payload, &["op", "browser"])?;
                self.reveal(string(&payload, "browser")?)
            }
            _ => Err("unsupported_browser_operation:不支持此 bundled extension 操作".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundled_browser_host::envelope;
    struct Fixture {
        _root: tempfile::TempDir,
        installer: Installer,
    }
    impl Fixture {
        fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            let installer = Installer {
                home: root.path().join("synthetic-home"),
                resources: root.path().join("resources"),
                platform: "macos".into(),
                interrupt_at: None,
            };
            let fixture = Self {
                _root: root,
                installer,
            };
            fixture.bundle("0.1.0", false);
            fixture
        }
        fn bundle(&self, version: &str, fixture: bool) {
            fs::create_dir_all(&self.installer.resources).unwrap();
            let mut packages = json!({});
            for package in ["chromium", "firefox"] {
                let path = self.installer.resources.join(package);
                fs::create_dir_all(&path).unwrap();
                let mut manifest = json!({"manifest_version":3,"name":"Lintel","version":version,"permissions":["storage","browsingData","privacy","nativeMessaging","alarms"]});
                manifest["background"] = if package == "chromium" {
                    json!({"service_worker":"background.js","type":"module"})
                } else {
                    json!({"scripts":["background.js"],"type":"module"})
                };
                if package == "firefox" {
                    manifest["browser_specific_settings"] =
                        json!({"gecko":{"id":"lintel@lintel.local"}});
                }
                let config = json!({"browser":package,"fixture":fixture,"sites":[{"origin":"https://claude.ai","domain":"claude.ai","label":"Claude","default":true},{"origin":"https://console.anthropic.com","domain":"anthropic.com","label":"Anthropic Console","default":false}]});
                fs::write(
                    path.join("config.js"),
                    format!(
                        "export const CONFIG = {};\n",
                        serde_json::to_string(&config).unwrap()
                    ),
                )
                .unwrap();
                fs::write(
                    path.join("manifest.json"),
                    serde_json::to_vec(&manifest).unwrap(),
                )
                .unwrap();
                fs::write(
                    path.join("background.js"),
                    format!("// inert synthetic extension {version}\n"),
                )
                .unwrap();
                let tree = tree(&path, false).unwrap().unwrap();
                packages[package] = json!({"version":version,"sha256":tree.identity(),"bytes":tree.bytes(),"files":tree.inventory()});
            }
            fs::write(
                self.installer.resources.join("manifest.json"),
                serde_json::to_vec(&json!({"schema":1,"packages":packages})).unwrap(),
            )
            .unwrap();
        }
        fn plan(&self, browser: &str) -> Value {
            self.installer.plan(browser).unwrap()
        }
        fn error(&self, payload: Value) -> String {
            envelope(self.installer.dispatch(payload))["error"]["code"]
                .as_str()
                .unwrap()
                .into()
        }
    }
    #[test]
    fn preview_is_readonly_finite_and_freezes_actual_content_and_scope() {
        let f = Fixture::new();
        let p = f.plan("chrome");
        assert!(!f.installer.home.exists());
        assert_eq!(p["install_action"], "install");
        assert_eq!(p["status"], "ready");
        assert_eq!(p["version"], "0.1.0");
        assert_eq!(p["files"].as_array().unwrap().len(), 3);
        assert_eq!(
            p["extension_path"],
            json!(f.installer.destination("chromium"))
        );
        assert_eq!(p["extension_path"], f.plan("edge")["extension_path"]);
        assert_ne!(p["extension_path"], f.plan("firefox")["extension_path"]);
        assert_eq!(
            f.error(json!({"op":"bundled_extension_plan","browser":"chrome","path":"/arbitrary"})),
            "unknown_field"
        );
        assert_eq!(
            f.error(json!({"op":"bundled_extension_plan","browser":"safari"})),
            "invalid_browser"
        );
        assert_eq!(
            f.error(json!({"op":"install_bundled_extension"})),
            "missing_field"
        );
        assert_eq!(
            f.error(json!({"op":"reveal_bundled_extension","browser":"chrome"})),
            "extension_not_installed"
        );
        assert_eq!(
            f.error(
                json!({"op":"reveal_bundled_extension","browser":"chrome","path":"/arbitrary"})
            ),
            "unknown_field"
        );
        assert!(!f.installer.home.exists());
    }
    #[test]
    fn exact_approval_installs_once_and_keeps_browser_profiles_and_host_state_absent() {
        let f = Fixture::new();
        let p = f.plan("chrome");
        let mut incomplete = p.clone();
        incomplete.as_object_mut().unwrap().remove("files");
        assert_eq!(
            f.error(json!({"op":"install_bundled_extension","approved_plan":incomplete})),
            "stale_plan"
        );
        assert!(!f.installer.home.exists());
        let first = f.installer.install(&p).unwrap();
        assert_eq!(first["status"], "installed");
        assert_eq!(first["loading"], "required");
        assert_eq!(first["pairing"], "required");
        let destination = f.installer.destination("chromium");
        assert_eq!(
            tree(&destination, true).unwrap().unwrap().inventory(),
            p["files"]
        );
        let ownership = destination.join(OWNERSHIP);
        let marker_time = fs::metadata(&ownership).unwrap().modified().unwrap();
        let lock_time = fs::metadata(f.installer.root().join("install.lock"))
            .unwrap()
            .modified()
            .unwrap();
        assert_eq!(
            f.installer.install(&p).unwrap()["status"],
            "already-installed"
        );
        assert_eq!(
            f.installer.install(&f.plan("edge")).unwrap()["status"],
            "already-installed"
        );
        assert_eq!(
            fs::metadata(ownership).unwrap().modified().unwrap(),
            marker_time
        );
        assert_eq!(
            fs::metadata(f.installer.root().join("install.lock"))
                .unwrap()
                .modified()
                .unwrap(),
            lock_time
        );
        assert_eq!(f.installer.reveal_path("chrome").unwrap(), destination);
        assert_eq!(f.installer.reveal_path("edge").unwrap(), destination);
        assert!(!f
            .installer
            .home
            .join("Library/Application Support/Google")
            .exists());
        assert!(!f
            .installer
            .home
            .join("Library/Application Support/Microsoft Edge")
            .exists());
        assert!(!f
            .installer
            .home
            .join("Library/Application Support/Mozilla")
            .exists());
        assert!(!f
            .installer
            .home
            .join("Library/Application Support/Lintel/browser-host")
            .exists());
        assert!(!f.installer.home.join(".local").exists());
    }
    #[test]
    fn changed_bundle_and_destination_reject_stale_approval_without_overwriting() {
        let f = Fixture::new();
        let p = f.plan("chrome");
        f.bundle("0.2.0", false);
        assert_eq!(
            f.error(json!({"op":"install_bundled_extension","approved_plan":p})),
            "stale_plan"
        );
        assert!(!f.installer.home.exists());
        let current = f.plan("chrome");
        fs::write(
            f.installer.resources.join("chromium/background.js"),
            b"modified bundled file",
        )
        .unwrap();
        assert_eq!(
            f.error(json!({"op":"install_bundled_extension","approved_plan":current})),
            "bundle_changed"
        );
        assert!(!f.installer.home.exists());
        f.bundle("0.2.0", false);
        let current = f.plan("chrome");
        fs::create_dir_all(f.installer.destination("chromium")).unwrap();
        fs::write(
            f.installer.destination("chromium").join("unowned.txt"),
            b"preserve me",
        )
        .unwrap();
        assert_eq!(
            f.error(json!({"op":"install_bundled_extension","approved_plan":current})),
            "stale_plan"
        );
        let blocked = f.plan("chrome");
        assert_eq!(blocked["status"], "conflict");
        assert_eq!(
            f.error(json!({"op":"install_bundled_extension","approved_plan":blocked})),
            "extension_conflict"
        );
        assert_eq!(
            fs::read(f.installer.destination("chromium").join("unowned.txt")).unwrap(),
            b"preserve me"
        );
        assert!(!f.installer.root().join("install.lock").exists());
    }
    #[test]
    fn owned_upgrade_keeps_stable_directory_and_retires_previous_files() {
        let f = Fixture::new();
        // A removed file belongs to the original managed package, not an external writer.
        for package in ["chromium", "firefox"] {
            fs::write(
                f.installer.resources.join(package).join("old.js"),
                b"old managed source",
            )
            .unwrap();
        }
        f.bundle("0.1.0", false);
        let first = f.plan("chrome");
        f.installer.install(&first).unwrap();
        assert!(f.installer.destination("chromium").join("old.js").exists());
        for package in ["chromium", "firefox"] {
            fs::remove_file(f.installer.resources.join(package).join("old.js")).unwrap();
        }
        f.bundle("0.2.0", false);
        let upgrade = f.plan("edge");
        assert_eq!(upgrade["install_action"], "upgrade");
        assert_eq!(upgrade["existing_installation"]["version"], "0.1.0");
        assert_eq!(upgrade["extension_path"], first["extension_path"]);
        assert_eq!(f.installer.install(&upgrade).unwrap()["status"], "updated");
        assert!(!f.installer.destination("chromium").join("old.js").exists());
        assert_eq!(f.plan("chrome")["version"], "0.2.0");
        assert_eq!(f.plan("chrome")["status"], "already-installed");
        fs::write(
            f.installer.destination("chromium").join("background.js"),
            b"user changed",
        )
        .unwrap();
        let blocked = f.plan("chrome");
        assert_eq!(blocked["install_action"], "blocked");
        assert_eq!(
            f.error(json!({"op":"install_bundled_extension","approved_plan":blocked})),
            "extension_conflict"
        );
        assert!(f
            .installer
            .reveal_path("chrome")
            .unwrap_err()
            .starts_with("extension_conflict:"));
        assert_eq!(
            fs::read(f.installer.destination("chromium").join("background.js")).unwrap(),
            b"user changed"
        );
    }
    #[test]
    fn identical_unowned_files_are_not_silently_adopted() {
        let f = Fixture::new();
        let destination = f.installer.destination("chromium");
        fs::create_dir_all(&destination).unwrap();
        for file in f.installer.bundle("chromium").unwrap().tree.files {
            fs::write(destination.join(file.path), file.bytes).unwrap();
        }
        let plan = f.plan("chrome");
        assert_eq!(plan["status"], "conflict");
        assert_eq!(
            f.error(json!({"op":"install_bundled_extension","approved_plan":plan})),
            "extension_conflict"
        );
        assert!(!destination.join(OWNERSHIP).exists());
    }
    #[test]
    fn interrupted_install_and_each_directory_switch_phase_require_fresh_approved_recovery() {
        for (upgrade, phase) in [
            (false, "staged"),
            (false, "new-installed"),
            (true, "staged"),
            (true, "previous-moved"),
            (true, "new-installed"),
            (true, "cleanup"),
        ] {
            let mut f = Fixture::new();
            if upgrade {
                f.installer.install(&f.plan("chrome")).unwrap();
                f.bundle("0.2.0", false);
            }
            let approved = f.plan("chrome");
            f.installer.interrupt_at = Some(phase);
            assert!(f
                .installer
                .install(&approved)
                .unwrap_err()
                .starts_with("extension_install_interrupted:"));
            assert!(f
                .installer
                .reveal_path("chrome")
                .unwrap_err()
                .starts_with("extension_install_interrupted:"));
            let recovery = f.plan("chrome");
            assert_eq!(recovery["status"], "ready");
            assert_eq!(
                recovery["install_action"],
                if upgrade { "upgrade" } else { "install" }
            );
            assert_eq!(recovery["extension_path"], approved["extension_path"]);
            assert_eq!(
                recovery["recovery"]["phase"],
                if phase == "staged" {
                    "prepared"
                } else if phase == "previous-moved" {
                    "previous-moved"
                } else {
                    "new-installed"
                }
            );
            assert_eq!(
                f.error(json!({"op":"install_bundled_extension","approved_plan":approved})),
                "stale_plan"
            );
            f.installer.interrupt_at = None;
            let finished = f.installer.install(&recovery).unwrap();
            assert_eq!(
                finished["status"],
                if upgrade { "updated" } else { "installed" }
            );
            assert!(finished["plan"]["recovery"].is_null());
            assert_eq!(finished["plan"]["status"], "already-installed");
            assert_eq!(
                f.installer.install(&recovery).unwrap()["status"],
                "already-installed"
            );
            let contents = fs::read_dir(f.installer.root())
                .unwrap()
                .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(contents.len(), 2);
            assert!(contents.contains(&"chromium".into()));
            assert!(contents.contains(&"install.lock".into()));
            assert_eq!(
                tree(&f.installer.reveal_path("chrome").unwrap(), true)
                    .unwrap()
                    .unwrap()
                    .inventory(),
                approved["files"]
            );
        }
    }
    #[test]
    fn partial_staging_record_and_partial_owned_backup_cleanup_remain_recoverable() {
        let mut f = Fixture::new();
        let first = f.plan("chrome");
        f.installer.interrupt_at = Some("staged");
        assert!(f.installer.install(&first).is_err());
        let interrupted = f.plan("chrome");
        let stage = PathBuf::from(interrupted["recovery"]["stage_path"].as_str().unwrap());
        fs::write(stage.join(OWNERSHIP), b"{partial staging write").unwrap();
        fs::write(stage.join("background.js"), b"partial bytes").unwrap();
        let recovery = f.plan("chrome");
        assert_eq!(recovery["status"], "ready");
        assert_ne!(
            recovery["recovery"]["stage_files"],
            interrupted["recovery"]["stage_files"]
        );
        f.installer.interrupt_at = None;
        f.installer.install(&recovery).unwrap();
        f.bundle("0.2.0", false);
        f.installer.interrupt_at = Some("cleanup");
        assert!(f.installer.install(&f.plan("chrome")).is_err());
        let interrupted = f.plan("chrome");
        let previous = PathBuf::from(interrupted["recovery"]["previous_path"].as_str().unwrap());
        fs::remove_file(previous.join("background.js")).unwrap();
        fs::remove_file(previous.join(OWNERSHIP)).unwrap();
        let recovery = f.plan("chrome");
        assert_eq!(recovery["status"], "ready");
        assert_eq!(recovery["recovery"]["phase"], "new-installed");
        f.installer.interrupt_at = None;
        assert_eq!(f.installer.install(&recovery).unwrap()["status"], "updated");
        assert!(!previous.exists());
    }
    #[test]
    fn interrupted_recovery_cannot_overwrite_a_new_unowned_destination_or_backup_file() {
        let mut f = Fixture::new();
        f.installer.install(&f.plan("chrome")).unwrap();
        f.bundle("0.2.0", false);
        f.installer.interrupt_at = Some("previous-moved");
        assert!(f.installer.install(&f.plan("chrome")).is_err());
        fs::create_dir_all(f.installer.destination("chromium")).unwrap();
        fs::write(
            f.installer.destination("chromium").join("unowned.txt"),
            b"preserve me",
        )
        .unwrap();
        let conflict = f.plan("chrome");
        assert_eq!(conflict["status"], "conflict");
        assert_eq!(conflict["install_action"], "blocked");
        assert_eq!(
            f.error(json!({"op":"install_bundled_extension","approved_plan":conflict})),
            "extension_conflict"
        );
        assert_eq!(
            fs::read(f.installer.destination("chromium").join("unowned.txt")).unwrap(),
            b"preserve me"
        );
        let mut f = Fixture::new();
        f.installer.install(&f.plan("chrome")).unwrap();
        f.bundle("0.2.0", false);
        f.installer.interrupt_at = Some("new-installed");
        assert!(f.installer.install(&f.plan("chrome")).is_err());
        let interrupted = f.plan("chrome");
        let previous = PathBuf::from(interrupted["recovery"]["previous_path"].as_str().unwrap());
        fs::write(previous.join("unowned.txt"), b"preserve backup addition").unwrap();
        let conflict = f.plan("chrome");
        assert_eq!(conflict["status"], "conflict");
        assert_eq!(
            f.error(json!({"op":"install_bundled_extension","approved_plan":conflict})),
            "extension_conflict"
        );
        assert_eq!(
            fs::read(previous.join("unowned.txt")).unwrap(),
            b"preserve backup addition"
        );
    }
    #[test]
    fn fixture_metadata_cannot_make_fixture_bytes_installable() {
        let f = Fixture::new();
        f.bundle("0.1.0", true);
        assert_eq!(
            f.error(json!({"op":"bundled_extension_plan","browser":"chrome"})),
            "bundle_fixture_rejected"
        );
        assert_eq!(
            f.error(json!({"op":"bundled_extension_plan","browser":"firefox"})),
            "bundle_fixture_rejected"
        );
        assert!(!f.installer.home.exists());
    }
    #[test]
    fn fixture_manifest_with_fresh_identity_is_still_rejected() {
        let f = Fixture::new();
        let manifest_path = f.installer.resources.join("chromium/manifest.json");
        let mut manifest = load(&manifest_path).unwrap();
        manifest["name"] = json!("Lintel — SYNTHETIC TEST ONLY");
        manifest["permissions"]
            .as_array_mut()
            .unwrap()
            .push(json!("cookies"));
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        let mut metadata = load(&f.installer.resources.join("manifest.json")).unwrap();
        let tree = tree(&f.installer.resources.join("chromium"), false)
            .unwrap()
            .unwrap();
        metadata["packages"]["chromium"] = json!({"version":"0.1.0","sha256":tree.identity(),"bytes":tree.bytes(),"files":tree.inventory()});
        fs::write(
            f.installer.resources.join("manifest.json"),
            serde_json::to_vec(&metadata).unwrap(),
        )
        .unwrap();
        assert_eq!(
            f.error(json!({"op":"bundled_extension_plan","browser":"chrome"})),
            "bundle_fixture_rejected"
        );
        assert!(!f.installer.home.exists());
    }
    #[cfg(unix)]
    #[test]
    fn installation_scope_rejects_parent_and_package_symlinks() {
        use std::os::unix::fs::symlink;
        let f = Fixture::new();
        let outside = f._root.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::create_dir_all(&f.installer.home).unwrap();
        symlink(&outside, f.installer.home.join("Library")).unwrap();
        assert_eq!(
            f.error(json!({"op":"bundled_extension_plan","browser":"chrome"})),
            "path_conflict"
        );
        assert!(fs::read_dir(&outside).unwrap().next().is_none());
        fs::remove_file(f.installer.home.join("Library")).unwrap();
        fs::create_dir_all(f.installer.root()).unwrap();
        symlink(&outside, f.installer.destination("chromium")).unwrap();
        assert_eq!(
            f.error(json!({"op":"bundled_extension_plan","browser":"chrome"})),
            "path_conflict"
        );
        assert!(f.installer.reveal_path("chrome").is_err());
        assert!(fs::read_dir(&outside).unwrap().next().is_none());
    }
    #[test]
    #[ignore = "requires npm run prepare:browser-extension; uses only a synthetic home"]
    fn actual_packaged_extensions_install_from_bundle_with_no_profile_mutation() {
        let root = tempfile::tempdir().unwrap();
        let installer = Installer {
            home: root.path().join("synthetic-home"),
            resources: std::env::var_os("LINTEL_TEST_BROWSER_EXTENSION_RESOURCES")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("browser-extension-bundle")
                }),
            platform: "macos".into(),
            interrupt_at: None,
        };
        for browser in ["chrome", "edge", "firefox"] {
            let plan = installer
                .dispatch(json!({"op":"bundled_extension_plan","browser":browser}))
                .unwrap();
            let expected = if browser == "edge" {
                "already-installed"
            } else {
                "installed"
            };
            let installed = installer
                .dispatch(json!({"op":"install_bundled_extension","approved_plan":plan}))
                .unwrap();
            assert_eq!(installed["status"], expected);
            assert_eq!(installed["plan"]["status"], "already-installed");
            let package = package(browser).unwrap();
            let actual = tree(&installer.reveal_path(browser).unwrap(), true)
                .unwrap()
                .unwrap();
            assert_eq!(actual.inventory(), plan["files"]);
            canonical_config(package, plan["version"].as_str().unwrap(), &actual).unwrap();
            for file in actual.files {
                assert_eq!(
                    file.bytes,
                    fs::read(installer.resources.join(package).join(file.path)).unwrap()
                );
            }
        }
        let lintel = installer.home.join("Library/Application Support/Lintel");
        assert_eq!(
            fs::read_dir(lintel)
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect::<Vec<_>>(),
            vec![std::ffi::OsString::from("browser-extensions")]
        );
        let support = installer.home.join("Library/Application Support");
        assert_eq!(
            fs::read_dir(support)
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect::<Vec<_>>(),
            vec![std::ffi::OsString::from("Lintel")]
        );
    }
}

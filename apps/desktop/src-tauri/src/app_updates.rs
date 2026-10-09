//! App-only updater. Webview requests select finite actions, never keys or URLs.
use crate::{activity::ActivityGate, network::NetworkState};
use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::Manager;
use tauri_plugin_updater::{Update, UpdaterExt};

const MAX_DOWNLOAD: u64 = 512 * 1024 * 1024;
const PREVIEW_FEED: &str = "https://lintel.page/updates/preview.json";
const STABLE_FEED: &str = "https://lintel.page/updates/stable.json";
const STATE_LIMIT: usize = 16 * 1024;

fn error(code: &str, message: &str) -> Value {
    json!({"ok":false,"error":{"code":code,"message":message}})
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
#[derive(Default)]
struct Inner {
    loaded: bool,
    background_check: bool,
    last_install: Option<Value>,
    saved_install: Option<Value>,
    saved_background: bool,
    phase: String,
    checked_at: Option<u64>,
    failure: Option<String>,
    candidate_id: Option<String>,
    update: Option<Update>,
    bytes: Option<Vec<u8>>,
    downloaded: u64,
    total: Option<u64>,
}
#[derive(Default)]
pub(crate) struct AppUpdateState {
    inner: Arc<Mutex<Inner>>,
    serial: tokio::sync::Mutex<()>,
}

fn configuration(config: Option<&Value>) -> Result<Option<&'static str>, String> {
    let Some(config) = config else {
        return Ok(None);
    };
    if !config.as_object().is_some_and(|fields| {
        fields.keys().all(|key| {
            [
                "pubkey",
                "endpoints",
                "requireSignedVersion",
                "allowDowngrades",
            ]
            .contains(&key.as_str())
        })
    }) {
        return Err("更新配置包含不支持的字段".into());
    }
    if config["pubkey"].as_str().unwrap_or("").is_empty() {
        return Ok(None);
    }
    let endpoints = config["endpoints"].as_array().ok_or("更新 feed 配置缺失")?;
    let channel = match endpoints.as_slice() {
        [url] if url == PREVIEW_FEED => "preview",
        [url] if url == STABLE_FEED => "stable",
        _ => return Err("更新只接受构建时固定的 Lintel HTTPS feed".into()),
    };
    if config["requireSignedVersion"] != true
        || config["allowDowngrades"] == true
        || config["dangerousInsecureTransportProtocol"] == true
        || config["dangerousAcceptInvalidCerts"] == true
        || config["dangerousAcceptInvalidHostnames"] == true
    {
        return Err("更新配置必须绑定签名版本并保持 TLS 与升序检查".into());
    }
    if config["pubkey"].as_str().map(str::len).unwrap_or(0) > 4096 {
        return Err("更新公钥配置超限".into());
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(config["pubkey"].as_str().unwrap())
        .map_err(|_| "更新公钥编码无效")?;
    let text = std::str::from_utf8(&decoded).map_err(|_| "更新公钥编码无效")?;
    minisign_verify::PublicKey::decode(text).map_err(|_| "更新公钥格式无效")?;
    Ok(Some(channel))
}

fn allowed_artifact(url: &tauri::Url, version: &str) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("github.com")
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url.path().starts_with(&format!(
            "/IndelibleVivi/lintel-cc/releases/download/v{version}/"
        ))
        && url.path().ends_with(".app.tar.gz")
        && !url.path().contains('%')
}

// A public read does not initialize core, scan a Claude root, or create storage.
fn read_state(file: &Path) -> Result<Value, String> {
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    let mut handle = match fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(file)
    {
        Ok(handle) => handle,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(json!({"background_check":false}))
        }
        Err(_) => return Err("无法读取 App 更新偏好".into()),
    };
    let meta = handle.metadata().map_err(|_| "无法核对 App 更新记录")?;
    if !meta.is_file() || meta.len() > STATE_LIMIT as u64 {
        return Err("App 更新记录不是有限普通文件".into());
    }
    let mut bytes = Vec::new();
    (&mut handle)
        .take((STATE_LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "无法读取更新记录")?;
    if bytes.len() > STATE_LIMIT {
        return Err("App 更新记录读取超限".into());
    }
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| "更新记录损坏，请保留原文件核对")?;
    if !value.as_object().is_some_and(|fields| {
        fields
            .keys()
            .all(|key| ["schema", "background_check", "last_install"].contains(&key.as_str()))
    }) || value["schema"] != "lintel.app-update-state/1"
        || !value["background_check"].is_boolean()
    {
        return Err("更新记录格式不兼容，请保留原文件核对".into());
    }
    if let Some(record) = value.get("last_install").filter(|v| !v.is_null()) {
        let valid = record.as_object().is_some_and(|fields| {
            fields.keys().all(|key| {
                [
                    "id",
                    "from_version",
                    "to_version",
                    "status",
                    "attempted_at",
                    "finished_at",
                ]
                .contains(&key.as_str())
            })
        }) && record["id"].as_str().is_some_and(|id| {
            id.len() == 64
                && id
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        }) && ["from_version", "to_version"].iter().all(|key| {
            record[*key]
                .as_str()
                .is_some_and(|v| v.len() <= 128 && semver::Version::parse(v).is_ok())
        }) && matches!(
            record["status"].as_str(),
            Some("installing" | "installed" | "interrupted" | "uncertain")
        ) && semver::Version::parse(record["from_version"].as_str().unwrap_or(""))
            .ok()
            .zip(semver::Version::parse(record["to_version"].as_str().unwrap_or("")).ok())
            .is_some_and(|(from, to)| from.cmp_precedence(&to).is_lt())
            && (record["status"] != "installed" || record["finished_at"].is_u64())
            && record["attempted_at"].is_u64()
            && record.get("finished_at").is_none_or(Value::is_u64);
        if !valid {
            return Err("原更新记录内容损坏，请保留文件核对".into());
        }
    }
    Ok(value)
}
fn write_state(file: &Path, inner: &mut Inner) -> Result<(), String> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let parent = file.parent().ok_or("更新记录目录不可用")?;
    fs::create_dir_all(parent).map_err(|_| "无法准备 App 更新记录目录")?;
    let meta = fs::symlink_metadata(parent).map_err(|_| "无法核对更新目录")?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err("更新目录身份不可用".into());
    }
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
        .map_err(|_| "无法保护更新目录")?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(parent.join("state.lock"))
        .map_err(|_| "无法取得更新记录锁")?;
    if !lock.metadata().map_err(|_| "无法核对更新记录锁")?.is_file() {
        return Err("更新记录锁不是普通文件".into());
    }
    fs2::FileExt::try_lock_exclusive(&lock).map_err(|_| "另一个 App 正在写更新记录，请稍后核对")?;
    let saved = read_state(file)?;
    if saved.get("last_install").filter(|v| !v.is_null()).cloned() != inner.saved_install
        || saved["background_check"] != inner.saved_background
    {
        return Err("更新记录被另一个 App 或外部编辑改变；重新打开核对，不覆盖原记录".into());
    }
    let bytes = serde_json::to_vec(&json!({"schema":"lintel.app-update-state/1","background_check":inner.background_check,"last_install":inner.last_install})).map_err(|_| "无法编码更新记录")?;
    if bytes.len() > STATE_LIMIT {
        return Err("更新记录超限".into());
    }
    let temp = parent.join(format!(
        ".state-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let mut created = false;
    let result = (|| {
        let mut handle = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)
            .map_err(|_| "无法暂存更新记录")?;
        created = true;
        handle
            .write_all(&bytes)
            .and_then(|_| handle.sync_all())
            .map_err(|_| "更新记录暂存未确认")?;
        fs::rename(&temp, file).map_err(|_| "更新记录发布未确认")?;
        fs::File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|_| "更新记录目录同步未确认")?;
        let saved = read_state(file)?;
        if saved["last_install"] != inner.last_install.clone().unwrap_or(Value::Null)
            || saved["background_check"] != inner.background_check
        {
            return Err("更新记录读回不一致".into());
        }
        Ok(())
    })();
    if created && temp.exists() {
        let _ = fs::remove_file(&temp);
    }
    if result.is_ok() {
        inner.saved_install = inner.last_install.clone();
        inner.saved_background = inner.background_check;
    }
    result
}

fn unresolved(inner: &Inner, current: &str) -> bool {
    let Some(record) = &inner.last_install else {
        return false;
    };
    let pending = matches!(
        record["status"].as_str(),
        Some("installing" | "interrupted" | "uncertain")
    );
    let repaired = record["to_version"]
        .as_str()
        .and_then(|v| semver::Version::parse(v).ok())
        .zip(semver::Version::parse(current).ok())
        .map(|(target, current)| !current.cmp_precedence(&target).is_lt())
        .unwrap_or(false);
    pending && !repaired
}
impl AppUpdateState {
    fn load(&self, file: &Path) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        if inner.loaded {
            return Ok(());
        }
        let saved = read_state(file)?;
        inner.background_check = saved["background_check"].as_bool().unwrap_or(false);
        inner.last_install = saved.get("last_install").filter(|v| !v.is_null()).cloned();
        inner.saved_install = inner.last_install.clone();
        inner.saved_background = inner.background_check;
        if let Some(record) = &mut inner.last_install {
            if record["status"] == "installing" {
                record["status"] = json!("interrupted");
            }
        }
        inner.loaded = true;
        Ok(())
    }
    fn snapshot(&self, current: &str, channel: Option<&str>) -> Value {
        let inner = self.inner.lock().unwrap();
        let pending_restart = inner.last_install.as_ref().is_some_and(|record| {
            record["status"] == "installed" && record["from_version"] == current
        });
        let candidate = inner.update.as_ref().map(|update| json!({"id":inner.candidate_id,"version":update.version,"notes":update.body,"url":update.download_url,"signature_verified":inner.bytes.is_some()}));
        json!({"schema":"lintel.app-update/1","current_version":current,"channel":channel,
            "configured":channel.is_some(),"phase":if channel.is_none(){"unconfigured"}else if inner.phase.is_empty(){if pending_restart {"installed"} else {"idle"}}else{&inner.phase},
            "background_check":inner.background_check,"checked_at":inner.checked_at,"failure":inner.failure,
            "candidate":candidate,"downloaded_bytes":inner.downloaded,"total_bytes":inner.total,
            "last_install":inner.last_install,"installation_unresolved":unresolved(&inner,current),
            "scope":"this_app","external_components_updated":false})
    }
    fn fail(&self, message: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.phase = "error".into();
        inner.failure = Some(message.into());
    }
}

#[tauri::command]
pub(crate) async fn app_update_request(app: tauri::AppHandle, payload: Value) -> Value {
    let state = app.state::<AppUpdateState>();
    let gate = app.state::<ActivityGate>();
    let network = app.state::<NetworkState>();
    let Some(fields) = payload.as_object() else {
        return error("invalid_update_request", "更新请求必须是对象");
    };
    let Some(op) = fields.get("op").and_then(Value::as_str) else {
        return error("invalid_update_request", "请选择更新操作");
    };
    let allowed = match op {
        "status" | "check" => vec!["op"],
        "preference" => vec!["op", "background_check"],
        "download" | "install" | "restart" => vec!["op", "candidate_id"],
        _ => return error("unknown_update_operation", "不支持此更新操作"),
    };
    if fields.keys().any(|key| !allowed.contains(&key.as_str())) {
        return error("invalid_update_request", "更新请求包含不支持的字段");
    }
    let file: PathBuf = match app.path().app_data_dir() {
        Ok(dir) => dir.join("updater/state.json"),
        Err(_) => return error("update_storage_unavailable", "无法定位 App 更新记录"),
    };
    if let Err(message) = state.load(&file) {
        return error("update_state_unavailable", &message);
    }
    let current = app.package_info().version.to_string();
    let channel = match configuration(app.config().plugins.0.get("updater")) {
        Ok(channel) => channel,
        Err(message) => return error("invalid_update_configuration", &message),
    };
    if op == "status" {
        return json!({"ok":true,"data":state.snapshot(&current,channel)});
    }
    let Ok(_serial) = state.serial.try_lock() else {
        return error("update_busy", "更新操作仍在进行，请查看当前状态");
    };
    if op == "preference" {
        let Some(enabled) = payload["background_check"].as_bool() else {
            return error("invalid_update_request", "后台检查必须是布尔值");
        };
        let mut inner = state.inner.lock().unwrap();
        let previous = inner.background_check;
        inner.background_check = enabled;
        if let Err(message) = write_state(&file, &mut inner) {
            inner.background_check = previous;
            return error("update_state_write_failed", &message);
        }
        drop(inner);
        return json!({"ok":true,"data":state.snapshot(&current,channel)});
    }
    let Some(channel) = channel else {
        return error(
            "updater_unconfigured",
            "此构建尚未配置发行公钥与更新 feed；请从官方源码／候选说明获取安装方式",
        );
    };
    if !cfg!(target_os = "macos") {
        return error(
            "unsupported_update_platform",
            "当前 App 更新路径只支持 macOS",
        );
    }
    if op == "restart" {
        let permit = match gate.installation() {
            Ok(permit) => permit,
            Err(code) => return error(code, "App 正在执行操作；完成后再重启"),
        };
        if network.has_active_channels().await {
            return error("proxy_active", "请先停止本 App 的所有受控通道，再重启");
        }
        let inner = state.inner.lock().unwrap();
        let id = payload["candidate_id"].as_str().unwrap_or("");
        if id.is_empty()
            || !inner
                .last_install
                .as_ref()
                .is_some_and(|record| record["id"] == id && record["status"] == "installed")
        {
            return error(
                "update_not_installed",
                "原更新尚未确认安装，不能由此操作重启",
            );
        }
        drop(inner);
        drop(_serial);
        let _permit = permit;
        app.restart();
    }
    if op == "check" {
        if state
            .inner
            .lock()
            .unwrap()
            .last_install
            .as_ref()
            .is_some_and(|record| {
                record["status"] == "installed" && record["from_version"] == current
            })
        {
            return error(
                "update_restart_pending",
                "本次安装已确认，请保存好工作并重启后再检查下一版本",
            );
        }
        {
            let mut inner = state.inner.lock().unwrap();
            inner.phase = "checking".into();
            inner.failure = None;
            inner.update = None;
            inner.bytes = None;
            inner.candidate_id = None;
            inner.downloaded = 0;
            inner.total = None;
        }
        let updater = match app
            .updater_builder()
            .timeout(Duration::from_secs(20))
            .configure_client(|client| client.https_only(true).timeout(Duration::from_secs(20)))
            .build()
        {
            Ok(updater) => updater,
            Err(_) => {
                state.fail("更新配置无法初始化");
                return error("update_check_failed", "更新配置无法初始化");
            }
        };
        let result = tokio::time::timeout(Duration::from_secs(25), updater.check()).await;
        let update = match result {
            Ok(Ok(update)) => update,
            _ => {
                state.fail("更新检查未完成，可能离线或 feed 不可用；当前是否最新尚未确认");
                return error(
                    "update_check_failed",
                    "更新检查未完成，当前是否最新尚未确认",
                );
            }
        };
        let mut inner = state.inner.lock().unwrap();
        if let Some(mut update) = update {
            if !allowed_artifact(&update.download_url, &update.version)
                || update.version.len() > 128
                || update.signature.is_empty()
                || update.signature.len() > 8192
                || update
                    .body
                    .as_ref()
                    .map(|s| s.len() > 16 * 1024)
                    .unwrap_or(false)
                || (channel == "stable"
                    && semver::Version::parse(&update.version)
                        .map(|v| !v.pre.is_empty())
                        .unwrap_or(true))
            {
                drop(inner);
                state.fail("feed 候选不符合 Lintel 平台、来源或元数据约束");
                return error(
                    "invalid_update_candidate",
                    "feed 候选不符合 Lintel 平台、来源或元数据约束",
                );
            }
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            inner.candidate_id = Some(format!(
                "{:x}",
                Sha256::digest(format!(
                    "{}\n{}\n{}\n{stamp}",
                    update.version, update.download_url, update.signature
                ))
            ));
            update.timeout = Some(Duration::from_secs(300));
            inner.update = Some(update);
            inner.phase = "available".into();
        } else {
            inner.phase = "latest".into();
        }
        inner.checked_at = Some(now());
        drop(inner);
        return json!({"ok":true,"data":state.snapshot(&current,Some(channel))});
    }
    let id = payload["candidate_id"].as_str().unwrap_or("");
    let update = {
        let inner = state.inner.lock().unwrap();
        if id.is_empty() || inner.candidate_id.as_deref() != Some(id) {
            return error("stale_update_candidate", "更新候选已改变，请重新检查并审阅");
        }
        inner.update.clone()
    };
    if op == "download" {
        let Some(update) = update else {
            return error("stale_update_candidate", "请先检查更新");
        };
        if state.inner.lock().unwrap().bytes.is_some() {
            return json!({"ok":true,"data":state.snapshot(&current,Some(channel))});
        }
        {
            let mut inner = state.inner.lock().unwrap();
            inner.phase = "downloading".into();
            inner.failure = None;
            inner.downloaded = 0;
        }
        let over_budget = Arc::new(tokio::sync::Notify::new());
        let notify = over_budget.clone();
        let shared = state.inner.clone();
        let download = update.download(
            move |chunk, total| {
                let mut inner = shared.lock().unwrap();
                inner.downloaded = inner.downloaded.saturating_add(chunk as u64);
                inner.total = total;
                if inner.downloaded > MAX_DOWNLOAD || total.is_some_and(|n| n > MAX_DOWNLOAD) {
                    notify.notify_one();
                }
            },
            || {},
        );
        let result = tokio::select! {
            biased;
            _ = over_budget.notified() => Err("更新下载超过 512 MiB admission"),
            result = tokio::time::timeout(Duration::from_secs(300),download) => match result {Ok(Ok(bytes)) if bytes.len() as u64<=MAX_DOWNLOAD=>Ok(bytes),_=>Err("下载或签名／版本核验未完成，请重新检查")},
        };
        match result {
            Ok(bytes) => {
                let mut inner = state.inner.lock().unwrap();
                inner.bytes = Some(bytes);
                inner.phase = "verified".into();
                inner.failure = None;
            }
            Err(message) => {
                state.fail(message);
                return error("update_download_failed", message);
            }
        }
        return json!({"ok":true,"data":state.snapshot(&current,Some(channel))});
    }
    let permit = match gate.installation() {
        Ok(permit) => permit,
        Err(code) => return error(code, "App 正在执行操作；完成后再安装或重启"),
    };
    if network.has_active_channels().await {
        return error(
            "proxy_active",
            "请先停止本 App 的所有受控通道，再安装或重启",
        );
    }
    let Some(update) = update else {
        return error("stale_update_candidate", "请先检查更新");
    };
    let bytes = {
        let mut inner = state.inner.lock().unwrap();
        if unresolved(&inner, &current) {
            return error(
                "update_needs_reconciliation",
                "上次安装结果待核对；保留原记录，通过官方安装包修复后再检查",
            );
        }
        if inner
            .last_install
            .as_ref()
            .is_some_and(|record| record["id"] == id)
        {
            return error(
                "update_already_attempted",
                "该更新安装已经尝试；请查看原结果，不重复安装",
            );
        }
        let Some(bytes) = inner.bytes.take() else {
            return error("update_not_verified", "请先下载并完成签名核验");
        };
        inner.last_install = Some(
            json!({"id":id,"from_version":current,"to_version":update.version,"status":"installing","attempted_at":now()}),
        );
        if let Err(message) = write_state(&file, &mut inner) {
            inner.bytes = Some(bytes);
            inner.phase = "uncertain".into();
            return error("update_intent_unconfirmed", &message);
        }
        inner.phase = "installing".into();
        bytes
    };
    let result =
        tauri::async_runtime::spawn_blocking(move || (update.install(bytes), permit)).await;
    let (result, _permit) = match result {
        Ok((result, permit)) => (Some(result), Some(permit)),
        Err(_) => (None, None),
    };
    let mut inner = state.inner.lock().unwrap();
    let succeeded = matches!(result, Some(Ok(())));
    let record = inner.last_install.as_mut().unwrap();
    record["status"] = json!(if succeeded { "installed" } else { "uncertain" });
    record["finished_at"] = json!(now());
    inner.phase = if succeeded { "installed" } else { "uncertain" }.into();
    if let Err(message) = write_state(&file, &mut inner) {
        inner.phase = "uncertain".into();
        inner.last_install.as_mut().unwrap()["status"] = json!("uncertain");
        inner.failure = Some(message);
    } else if !succeeded {
        inner.failure = Some("安装结果未确认；保留原更新记录，不重放安装".into());
    }
    drop(inner);
    json!({"ok":true,"data":state.snapshot(&current,Some(channel))})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_configuration_and_release_origin() {
        assert_eq!(configuration(None).unwrap(), None);
        let mut raw = vec![0u8; 42];
        raw[..2].copy_from_slice(b"Ed");
        let key = base64::engine::general_purpose::STANDARD.encode(format!(
            "untrusted comment: synthetic test only\n{}\n",
            base64::engine::general_purpose::STANDARD.encode(raw)
        ));
        let mut config =
            json!({"pubkey":key,"endpoints":[PREVIEW_FEED],"requireSignedVersion":true});
        assert_eq!(configuration(Some(&config)).unwrap(), Some("preview"));
        config["allowDowngrades"] = json!(true);
        assert!(configuration(Some(&config)).is_err());
        config["allowDowngrades"] = json!(false);
        config["endpoints"] = json!(["https://example.invalid/feed"]);
        assert!(configuration(Some(&config)).is_err());
        assert!(allowed_artifact(&tauri::Url::parse("https://github.com/IndelibleVivi/lintel-cc/releases/download/v0.2.0/Lintel.app.tar.gz").unwrap(),"0.2.0"));
        for url in [
            "http://github.com/IndelibleVivi/lintel-cc/releases/download/v0.2.0/Lintel.app.tar.gz",
            "https://github.com/other/repo/releases/download/v0.2.0/Lintel.app.tar.gz",
            "https://github.com/IndelibleVivi/lintel-cc/releases/download/v0.1.0/Lintel.app.tar.gz",
        ] {
            assert!(!allowed_artifact(&tauri::Url::parse(url).unwrap(), "0.2.0"));
        }
    }
    #[test]
    fn durable_intent_interruptions_never_attest_installation() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("updater/state.json");
        let mut inner = Inner {
            background_check: true,
            last_install: Some(
                json!({"id":"a".repeat(64),"from_version":"0.1.0","to_version":"0.2.0","status":"installing","attempted_at":1}),
            ),
            ..Default::default()
        };
        write_state(&file, &mut inner).unwrap();
        let state = AppUpdateState::default();
        state.load(&file).unwrap();
        let view = state.snapshot("0.1.0", Some("preview"));
        assert_eq!(view["last_install"]["id"], "a".repeat(64));
        assert_eq!(view["last_install"]["status"], "interrupted");
        assert_eq!(view["installation_unresolved"], true);
        let newer = state.snapshot("0.2.0", Some("preview"));
        assert_eq!(newer["installation_unresolved"], false);
        assert_eq!(newer["last_install"]["status"], "interrupted");
        assert_eq!(state.snapshot("0.1.0", None)["phase"], "unconfigured");
        let mut completed = Inner {
            last_install: Some(
                json!({"id":"b".repeat(64),"from_version":"0.1.0","to_version":"0.2.0","status":"installed","attempted_at":1,"finished_at":2}),
            ),
            ..Default::default()
        };
        let completed_file = temp.path().join("completed/state.json");
        write_state(&completed_file, &mut completed).unwrap();
        let recovered = AppUpdateState::default();
        recovered.load(&completed_file).unwrap();
        assert_eq!(
            recovered.snapshot("0.1.0", Some("preview"))["phase"],
            "installed",
            "original confirmed installation remains restartable"
        );
        assert_eq!(
            recovered.snapshot("0.2.0", Some("preview"))["phase"],
            "idle"
        );
        let mut unsupported = read_state(&completed_file).unwrap();
        unsupported["extra"] = json!("unknown future field");
        fs::write(&completed_file, serde_json::to_vec(&unsupported).unwrap()).unwrap();
        assert!(
            read_state(&completed_file).is_err(),
            "older App cannot erase unknown state fields"
        );
        unsupported.as_object_mut().unwrap().remove("extra");
        unsupported["last_install"]["to_version"] = json!("0.1.0+same-precedence");
        fs::write(&completed_file, serde_json::to_vec(&unsupported).unwrap()).unwrap();
        assert!(
            read_state(&completed_file).is_err(),
            "malformed intent cannot claim an upgrade"
        );
        fs::write(&file, b"broken").unwrap();
        assert!(AppUpdateState::default().load(&file).is_err());
    }
    #[test]
    fn independent_app_cannot_erase_an_original_install_record() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("updater/state.json");
        let mut first = Inner {
            last_install: Some(
                json!({"id":"a".repeat(64),"from_version":"0.1.0","to_version":"0.2.0","status":"installing","attempted_at":1}),
            ),
            ..Default::default()
        };
        let mut stale = Inner {
            background_check: true,
            ..Default::default()
        };
        write_state(&file, &mut first).unwrap();
        assert!(write_state(&file, &mut stale).is_err());
        assert_eq!(
            read_state(&file).unwrap()["last_install"]["id"],
            "a".repeat(64)
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let link = temp.path().join("link.json");
            symlink(&file, &link).unwrap();
            assert!(read_state(&link).is_err());
        }
    }

    // Actual official updater HTTP parsing, signature/version verification and
    // macOS bundle replacement; every file and socket belongs to this fixture.
    #[cfg(target_os = "macos")]
    #[test]
    fn official_updater_verifies_and_installs_only_synthetic_bundle() {
        use std::io::{Read, Write};
        let temp = tempfile::tempdir().unwrap();
        let fixture = std::process::Command::new("node")
            .arg(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../tests/fixtures/app_update_fixture.mjs"),
            )
            .arg(temp.path())
            .output()
            .expect("Node is required for disposable signing fixtures");
        assert!(
            fixture.status.success(),
            "{}",
            String::from_utf8_lossy(&fixture.stderr)
        );
        let fixture: Value = serde_json::from_slice(&fixture.stdout).unwrap();
        let bytes = fs::read(fixture["archive"].as_str().unwrap()).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let manifest = json!({"version":"0.2.0","notes":"Synthetic only","platforms":{"darwin-aarch64":{"url":format!("http://{address}/app"),"signature":fixture["signature"]}}});
        let body = serde_json::to_vec(&manifest).unwrap();
        let server = std::thread::spawn(move || {
            for _ in 0..5 {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = [0u8; 4096];
                let n = stream.read(&mut request).unwrap();
                let data = if request[..n].starts_with(b"GET /feed ") {
                    &body
                } else {
                    &bytes
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    data.len()
                )
                .unwrap();
                stream.write_all(data).unwrap();
            }
        });
        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        context.config_mut().plugins.0.insert(
            "updater".into(),
            json!({"pubkey":fixture["pubkey"],"endpoints":[],"requireSignedVersion":true}),
        );
        let app = tauri::test::mock_builder()
            .plugin(tauri_plugin_updater::Builder::new().build())
            .build(context)
            .unwrap();
        let old = temp
            .path()
            .join("installed/Lintel.app/Contents/MacOS/lintel-desktop");
        fs::create_dir_all(old.parent().unwrap()).unwrap();
        fs::write(&old, b"ORIGINAL SYNTHETIC APP").unwrap();
        let updater = app
            .updater_builder()
            .endpoints(vec![format!("http://{address}/feed").parse().unwrap()])
            .unwrap()
            .target("darwin-aarch64")
            .executable_path(&old)
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        tauri::async_runtime::block_on(async {
            let update = updater.check().await.unwrap().unwrap();
            let mut forged = update.clone();
            forged.version = "0.3.0".into();
            assert!(
                forged.download(|_, _| {}, || {}).await.is_err(),
                "Feed cannot mislabel a signed old version"
            );
            let mut bad = update.clone();
            bad.signature = "invalid signature".into();
            assert!(bad.download(|_, _| {}, || {}).await.is_err());
            let data = update.download(|_, _| {}, || {}).await.unwrap();
            assert_eq!(
                fs::read(&old).unwrap(),
                b"ORIGINAL SYNTHETIC APP",
                "Download must not install"
            );
            update.install(data).unwrap();
            assert_ne!(fs::read(&old).unwrap(), b"ORIGINAL SYNTHETIC APP");
            let metadata =
                fs::read(old.parent().unwrap().parent().unwrap().join("Info.plist")).unwrap();
            assert!(String::from_utf8_lossy(&metadata).contains("0.2.0"));
            assert!(updater.check().await.is_ok());
        });
        server.join().unwrap();
    }
}

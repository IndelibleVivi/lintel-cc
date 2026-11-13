//! Finite Claude login stores. Credential bytes stay in memory; only the
//! original item's opaque identity may enter a private frozen plan.
use crate::{err, storage, Result};
use serde_json::{json, Value};
use std::{
    ffi::CStr,
    path::{Path, PathBuf},
};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(all(test, target_os = "macos"))]
mod native_tests;

pub(crate) const MAX_ITEM_BYTES: usize = 1024 * 1024;
const SERVICE: &str = "Claude Code-credentials";
const FIXTURE: &str = ".lintel-test-keychain.json";
const MAX_FIXTURE_BYTES: usize = 8 * MAX_ITEM_BYTES;

#[derive(Clone)]
pub(crate) struct Item {
    pub identity: Value,
    pub bytes: Vec<u8>,
}

pub(crate) fn services(home: &Path, root: &Path) -> Vec<String> {
    let mut result = vec![];
    if root == home.join(".claude") {
        result.push(SERVICE.into());
    }
    let mut paths = vec![root.to_string_lossy().into_owned()];
    if let Some(raw) = std::env::var_os("CLAUDE_CONFIG_DIR") {
        if Path::new(&raw).canonicalize().ok().as_deref() == Some(root) {
            paths.push(raw.to_string_lossy().into_owned());
        }
    }
    for path in paths {
        #[cfg(target_os = "macos")]
        let path = macos::normalize(&path);
        let service = format!("{SERVICE}-{}", &storage::digest(path.as_bytes())[..8]);
        if !result.contains(&service) {
            result.push(service);
        }
    }
    result
}

fn valid_service(service: &str) -> bool {
    service == SERVICE
        || service
            .strip_prefix(&format!("{SERVICE}-"))
            .is_some_and(|suffix| {
                suffix.len() == 8
                    && suffix
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            })
}

fn user_home() -> Option<PathBuf> {
    use std::os::unix::ffi::OsStrExt;
    let mut entry = std::mem::MaybeUninit::<libc::passwd>::uninit();
    let mut output = std::ptr::null_mut();
    let mut buffer = vec![0u8; 16384];
    let status = unsafe {
        libc::getpwuid_r(
            libc::geteuid(),
            entry.as_mut_ptr(),
            buffer.as_mut_ptr().cast(),
            buffer.len(),
            &mut output,
        )
    };
    if status != 0 || output.is_null() {
        return None;
    }
    let entry = unsafe { entry.assume_init() };
    if entry.pw_dir.is_null() {
        return None;
    }
    Some(PathBuf::from(std::ffi::OsStr::from_bytes(
        unsafe { CStr::from_ptr(entry.pw_dir) }.to_bytes(),
    )))
}

#[cfg(all(test, target_os = "macos"))]
thread_local! {
    static NATIVE_FIXTURE: std::cell::RefCell<Option<(PathBuf, PathBuf)>> = const { std::cell::RefCell::new(None) };
}

enum Backend {
    Synthetic(PathBuf),
    #[cfg(target_os = "macos")]
    Native(Option<PathBuf>),
}

fn backend(home: &Path) -> Result<Backend> {
    #[cfg(all(test, target_os = "macos"))]
    if let Some(path) = NATIVE_FIXTURE.with(|slot| {
        slot.borrow()
            .as_ref()
            .filter(|(scope, _)| scope == home)
            .map(|(_, path)| path.clone())
    }) {
        return Ok(Backend::Native(Some(path)));
    }
    let actual_home = user_home().and_then(|path| path.canonicalize().ok());
    let selected_home = home.canonicalize().ok();
    let synthetic = cfg!(test)
        || std::env::var_os("LINTEL_TEST_HOME").is_some()
        || selected_home.is_none()
        || selected_home != actual_home;
    if synthetic {
        let temporary = std::env::temp_dir().canonicalize().map_err(|_| {
            err(
                "keychain_synthetic_unavailable",
                "合成 Keychain fixture 不可用",
            )
        })?;
        let selected = selected_home.ok_or_else(|| {
            err(
                "keychain_synthetic_unavailable",
                "合成 Keychain fixture 不可用",
            )
        })?;
        if selected == temporary || !selected.starts_with(&temporary) {
            return Err(err(
                "keychain_synthetic_unavailable",
                "合成 Keychain 只允许独立临时 HOME",
            ));
        }
        storage::guard(&selected)?;
        return Ok(Backend::Synthetic(selected.join(FIXTURE)));
    }
    #[cfg(target_os = "macos")]
    {
        Ok(Backend::Native(None))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(err("keychain_unsupported", "当前主机不使用 macOS Keychain"))
    }
}

fn fixture_error(value: &Value) -> Option<crate::Error> {
    let code = match value.as_str()? {
        "keychain_locked" => "keychain_locked",
        "keychain_access_denied" => "keychain_access_denied",
        "keychain_unavailable" => "keychain_unavailable",
        "keychain_write_uncertain" => "keychain_write_uncertain",
        _ => "keychain_invalid_format",
    };
    Some(err(code, "合成 Keychain 操作未完成"))
}

fn fixture(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Err(err(
            "keychain_synthetic_unavailable",
            "未提供合成 Keychain fixture",
        ));
    }
    let bytes = storage::read(path, MAX_FIXTURE_BYTES as u64)
        .map_err(|_| err("keychain_unavailable", "无法读取合成 Keychain fixture"))?;
    let value = storage::parse(&bytes).map_err(|_| {
        err(
            "keychain_invalid_format",
            "合成 Keychain fixture 格式无法识别",
        )
    })?;
    if value["schema"] != "lintel.test-keychain/1" || !value["items"].is_object() {
        return Err(err(
            "keychain_invalid_format",
            "合成 Keychain fixture 格式无法识别",
        ));
    }
    if let Some(error) = fixture_error(&value["error"]) {
        return Err(error);
    }
    Ok(value)
}

fn fixture_item(value: &Value, service: &str) -> Result<Option<Item>> {
    let Some(item) = value["items"].get(service) else {
        return Ok(None);
    };
    let id = item["id"]
        .as_str()
        .filter(|id| !id.is_empty() && id.len() <= 256)
        .ok_or_else(|| err("keychain_invalid_format", "合成 Keychain 缺少原件身份"))?;
    let bytes = match item["raw"].as_str() {
        Some(raw)
            if raw.len() <= MAX_ITEM_BYTES
                && storage::parse(raw.as_bytes()).ok().as_ref() == Some(&item["data"]) =>
        {
            raw.as_bytes().to_vec()
        }
        _ => serde_json::to_vec(&item["data"])?,
    };
    if bytes.len() > MAX_ITEM_BYTES {
        return Err(err("keychain_too_large", "Keychain 凭据容器超出读取上限"));
    }
    Ok(Some(Item {
        identity: json!({"synthetic_id":id,"service":service}),
        bytes,
    }))
}

pub(crate) fn read(home: &Path, service: &str) -> Result<Option<Item>> {
    if !valid_service(service) {
        return Err(err("keychain_invalid_format", "Keychain 来源不在支持范围"));
    }
    match backend(home)? {
        Backend::Synthetic(path) => fixture_item(&fixture(&path)?, service),
        #[cfg(target_os = "macos")]
        Backend::Native(path) => macos::read(service, None, path.as_deref()),
    }
}

pub(crate) fn read_bound(home: &Path, service: &str, identity: &Value) -> Result<Option<Item>> {
    if !valid_service(service) {
        return Err(err("keychain_invalid_format", "Keychain 来源不在支持范围"));
    }
    let item = match backend(home)? {
        Backend::Synthetic(path) => fixture_item(&fixture(&path)?, service)?,
        #[cfg(target_os = "macos")]
        Backend::Native(path) => macos::read(service, Some(identity), path.as_deref())?,
    };
    if item.as_ref().is_some_and(|item| item.identity != *identity) {
        return Err(err(
            "keychain_item_changed",
            "Keychain 原件已被替换；请重新预览",
        ));
    }
    Ok(item)
}

pub(crate) fn replace(
    home: &Path,
    service: &str,
    expected: &Item,
    replacement: &[u8],
) -> Result<Item> {
    if replacement.len() > MAX_ITEM_BYTES {
        return Err(err("keychain_too_large", "Keychain 凭据容器超出写入上限"));
    }
    let current = read_bound(home, service, &expected.identity)?
        .ok_or_else(|| err("keychain_item_changed", "Keychain 原件已变化；请重新预览"))?;
    if current.bytes != expected.bytes {
        return Err(err(
            "keychain_item_changed",
            "Keychain 内容已变化；请重新预览",
        ));
    }
    match backend(home)? {
        Backend::Synthetic(path) => {
            let mut value = fixture(&path)?;
            let current = fixture_item(&value, service)?
                .ok_or_else(|| err("keychain_item_changed", "Keychain 原件已变化"))?;
            if current.identity != expected.identity || current.bytes != expected.bytes {
                return Err(err("keychain_item_changed", "Keychain 内容已变化"));
            }
            value["items"][service]["data"] = storage::parse(replacement)?;
            value["items"][service]["raw"] =
                json!(std::str::from_utf8(replacement).map_err(|_| err(
                    "keychain_invalid_format",
                    "Keychain 替代内容不是 UTF-8 JSON"
                ))?);
            let encoded = serde_json::to_vec_pretty(&value)?;
            if encoded.len() > MAX_FIXTURE_BYTES {
                return Err(err(
                    "keychain_too_large",
                    "合成 Keychain fixture 超出容量上限",
                ));
            }
            storage::atomic(&path, &encoded, 0o600)?;
            if let Some(error) = fixture_error(&value["after_write_error"]) {
                return Err(error);
            }
        }
        #[cfg(target_os = "macos")]
        Backend::Native(path) => {
            macos::update(service, &expected.identity, replacement, path.as_deref())?
        }
    }
    let actual = read_bound(home, service, &expected.identity)
        .map_err(|_| {
            err(
                "keychain_write_uncertain",
                "Keychain 已尝试更新；请核对原任务，不要重提",
            )
        })?
        .ok_or_else(|| {
            err(
                "keychain_write_uncertain",
                "Keychain 更新后原件不可读；请核对原任务",
            )
        })?;
    if actual.bytes != replacement {
        return Err(err(
            "keychain_write_uncertain",
            "Keychain 更新后内容变化；请核对原任务",
        ));
    }
    Ok(actual)
}

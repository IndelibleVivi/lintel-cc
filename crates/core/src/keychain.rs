//! Finite Claude login stores. Credential bytes stay in memory; only the
//! original item's opaque identity may enter a private frozen plan.
use crate::{err, storage, Result};
use serde_json::{json, Value};
use std::{
    ffi::{CStr, OsString},
    path::{Path, PathBuf},
    process::Command,
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

/// A directory's physical identity and its client's credential namespace are
/// different facts. In particular, explicitly setting the default config path
/// selects a hashed Keychain service instead of the ordinary bare service.
/// Preserve a matching inherited absolute spelling; do not invent an override
/// for the default environment. Current clients may also select secure storage
/// independently, so refuse a cross-root override rather than silently running
/// against another environment's credentials.
pub(crate) fn execution_context(home: &Path, root: &Path) -> Result<Value> {
    execution_context_with(
        home,
        root,
        std::env::var_os("CLAUDE_CONFIG_DIR"),
        std::env::var_os("CLAUDE_SECURESTORAGE_CONFIG_DIR"),
    )
}

fn execution_context_with(
    home: &Path,
    root: &Path,
    inherited_config: Option<OsString>,
    inherited_secure: Option<OsString>,
) -> Result<Value> {
    if inherited_config.as_ref().is_some_and(|raw| {
        !raw.is_empty()
            && !Path::new(raw).is_absolute()
            && Path::new(raw).canonicalize().ok().as_deref() == Some(root)
    }) {
        return Err(err(
            "auth_scope_unverified",
            "所选环境继承了相对 CLAUDE_CONFIG_DIR；改变工作目录或改写成绝对路径可能选择不同凭据。请从使用该原始配置的入口核对后重新预览，未启动客户端",
        ));
    }
    let matching = |value: &OsString| {
        let path = Path::new(value);
        path.is_absolute() && path.canonicalize().ok().as_deref() == Some(root)
    };
    let config = match inherited_config.filter(matching) {
        Some(raw) => Some(raw.into_string().map_err(|_| {
            err(
                "auth_scope_unverified",
                "配置目录的原始拼写不是可核对的 UTF-8 路径",
            )
        })?),
        None if root == home.join(".claude") => None,
        None => Some(root.to_string_lossy().into_owned()),
    };
    let secure = match inherited_secure {
        Some(raw) if (raw.is_empty() && root == home.join(".claude")) || matching(&raw) => {
            Some(raw.into_string().map_err(|_| {
                err("auth_scope_unverified", "安全存储目录的原始拼写不是可核对的 UTF-8 路径")
            })?)
        }
        Some(_) => {
            return Err(err(
                "auth_scope_unverified",
                "CLAUDE_SECURESTORAGE_CONFIG_DIR 指向所选环境之外或无法核对；请在启动 Lintel 的入口核对该变量后重试，未启动客户端或改动其他环境",
            ))
        }
        None => None,
    };
    Ok(
        json!({"schema":"lintel.auth-execution/1","home":home,"config_dir":config,"securestorage_config_dir":secure}),
    )
}

pub(crate) fn check_execution_context(home: &Path, root: &Path, frozen: &Value) -> Result<()> {
    if frozen["schema"] != "lintel.auth-execution/1" || execution_context(home, root)? != *frozen {
        return Err(err(
            "stale_auth_context",
            "客户端认证目录选择在预览后改变或旧预览未绑定该选择；请重新预览，原任务仍只查询",
        ));
    }
    Ok(())
}

pub(crate) fn configure_execution(command: &mut Command, context: &Value) -> Result<()> {
    if context["schema"] != "lintel.auth-execution/1" {
        return Err(err("stale_auth_context", "客户端认证目录选择缺少当前绑定"));
    }
    command.env("HOME", crate::string(context, "home")?);
    for (name, field) in [
        ("CLAUDE_CONFIG_DIR", "config_dir"),
        (
            "CLAUDE_SECURESTORAGE_CONFIG_DIR",
            "securestorage_config_dir",
        ),
    ] {
        match &context[field] {
            Value::Null => {
                command.env_remove(name);
            }
            Value::String(value) => {
                command.env(name, value);
            }
            _ => return Err(err("stale_auth_context", "客户端认证目录选择无效")),
        }
    }
    Ok(())
}

#[cfg(any(target_os = "macos", test))]
pub(crate) fn execution_shell_args(context: &Value) -> Result<String> {
    if context["schema"] != "lintel.auth-execution/1" {
        return Err(err("stale_auth_context", "客户端认证目录选择缺少当前绑定"));
    }
    let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
    // Clear inherited Terminal values first; an absent selector is part of the
    // frozen namespace, not permission for the receiving shell to choose one.
    let mut args = format!(
        "-u CLAUDE_CONFIG_DIR -u CLAUDE_SECURESTORAGE_CONFIG_DIR HOME={}",
        quote(crate::string(context, "home")?),
    );
    for (name, field) in [
        ("CLAUDE_CONFIG_DIR", "config_dir"),
        (
            "CLAUDE_SECURESTORAGE_CONFIG_DIR",
            "securestorage_config_dir",
        ),
    ] {
        match &context[field] {
            Value::Null => {}
            Value::String(value) => args.push_str(&format!(" {name}={}", quote(value))),
            _ => return Err(err("stale_auth_context", "客户端认证目录选择无效")),
        }
    }
    Ok(args)
}

pub(crate) fn services(home: &Path, root: &Path) -> Vec<String> {
    let mut result = vec![];
    if root == home.join(".claude") {
        result.push(SERVICE.into());
    }
    let mut paths = vec![root.to_string_lossy().into_owned()];
    for name in ["CLAUDE_CONFIG_DIR", "CLAUDE_SECURESTORAGE_CONFIG_DIR"] {
        if let Some(raw) = std::env::var_os(name) {
            if !raw.is_empty() && Path::new(&raw).canonicalize().ok().as_deref() == Some(root) {
                paths.push(raw.to_string_lossy().into_owned());
            }
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

#[cfg(any(target_os = "macos", test))]
fn supported_account(value: &str) -> Result<String> {
    // Current clients sanitize unsupported USER spellings to a different
    // account, while older clients used the raw value. Do not guess either
    // account or turn a lookup in the wrong namespace into proof of absence.
    if !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Ok(value.to_owned());
    }
    Err(err(
        "keychain_unavailable",
        "当前用户的 Keychain account 选择与已核对的客户端规则不一致；未查询其他 account，也未把无法核对当作无登录",
    ))
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

#[cfg(test)]
mod execution_context_tests {
    use super::*;
    use std::fs;

    #[test]
    fn account_selector_drift_is_unknown_instead_of_false_absence() {
        for value in ["alice", "_service-user.42", "claude-code-user"] {
            assert_eq!(supported_account(value).unwrap(), value);
        }
        for value in [
            "",
            "name with spaces",
            "姓名",
            "quote'",
            "a/b",
            "line\nbreak",
        ] {
            assert_eq!(
                supported_account(value).unwrap_err().code,
                "keychain_unavailable"
            );
        }
        assert!(supported_account(&"a".repeat(257)).is_err());
    }

    #[test]
    fn preserve_default_namespace_and_matching_absolute_spelling() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().canonicalize().unwrap();
        let root = home.join(".claude");
        let custom = home.join("custom 'quoted");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&custom).unwrap();
        let default = execution_context_with(&home, &root, None, None).unwrap();
        assert!(default["config_dir"].is_null());
        let explicit = format!("{}/", root.display());
        let selected =
            execution_context_with(&home, &root, Some(explicit.clone().into()), None).unwrap();
        assert_eq!(selected["config_dir"], explicit);
        let neighboring =
            execution_context_with(&home, &root, Some(custom.clone().into_os_string()), None)
                .unwrap();
        assert!(neighboring["config_dir"].is_null());
        assert_eq!(
            execution_context_with(&home, &custom, None, None).unwrap()["config_dir"],
            json!(custom)
        );
        let cwd = std::env::current_dir().unwrap();
        let mut relative = PathBuf::new();
        for _ in cwd
            .components()
            .filter(|part| matches!(part, std::path::Component::Normal(_)))
        {
            relative.push("..");
        }
        relative.push(root.strip_prefix("/").unwrap());
        assert_eq!(relative.canonicalize().unwrap(), root);
        assert_eq!(
            execution_context_with(&home, &root, Some(relative.into_os_string()), None)
                .unwrap_err()
                .code,
            "auth_scope_unverified"
        );
        for context in [
            default,
            selected,
            execution_context_with(&home, &custom, None, None).unwrap(),
        ] {
            let expected = format!(
                "{}\n{}\n{}",
                context["config_dir"].as_str().unwrap_or("unset"),
                context["securestorage_config_dir"]
                    .as_str()
                    .unwrap_or("unset"),
                home.display()
            );
            let probe = "printf '%s\\n%s\\n%s' \"${CLAUDE_CONFIG_DIR-unset}\" \"${CLAUDE_SECURESTORAGE_CONFIG_DIR-unset}\" \"$HOME\"";
            let mut direct = Command::new("/bin/sh");
            direct
                .args(["-c", probe])
                .env("CLAUDE_CONFIG_DIR", "WRONG")
                .env("CLAUDE_SECURESTORAGE_CONFIG_DIR", "WRONG");
            configure_execution(&mut direct, &context).unwrap();
            let result = direct.output().unwrap();
            assert!(result.status.success());
            assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);
            let quoted_probe = format!("'{}'", probe.replace('\'', "'\\''"));
            let script = format!(
                "exec env {} /bin/sh -c {}",
                execution_shell_args(&context).unwrap(),
                quoted_probe
            );
            let result = Command::new("/bin/sh")
                .args(["-c", &script])
                .env("CLAUDE_CONFIG_DIR", "WRONG")
                .env("CLAUDE_SECURESTORAGE_CONFIG_DIR", "WRONG")
                .output()
                .unwrap();
            assert!(result.status.success());
            assert_eq!(String::from_utf8(result.stdout).unwrap(), expected);
        }
    }

    #[test]
    fn secure_storage_override_cannot_cross_the_selected_root() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().canonicalize().unwrap();
        let root = home.join(".claude");
        let custom = home.join("custom");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&custom).unwrap();
        assert_eq!(
            execution_context_with(&home, &root, None, Some(OsString::new())).unwrap()
                ["securestorage_config_dir"],
            ""
        );
        let alias = format!("{}/", custom.display());
        assert_eq!(
            execution_context_with(&home, &custom, None, Some(alias.clone().into())).unwrap()
                ["securestorage_config_dir"],
            alias
        );
        for value in [
            OsString::new(),
            root.into_os_string(),
            OsString::from("relative"),
            home.join("missing").into_os_string(),
        ] {
            assert_eq!(
                execution_context_with(&home, &custom, None, Some(value))
                    .unwrap_err()
                    .code,
                "auth_scope_unverified"
            );
        }
        assert_eq!(
            check_execution_context(&home, &custom, &Value::Null)
                .unwrap_err()
                .code,
            "stale_auth_context"
        );
        let mut altered = execution_context(&home, &custom).unwrap();
        altered["config_dir"] = Value::Null;
        assert_eq!(
            check_execution_context(&home, &custom, &altered)
                .unwrap_err()
                .code,
            "stale_auth_context"
        );
    }
}

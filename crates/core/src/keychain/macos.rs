//! Small Security.framework adapter: exact generic-password queries and
//! data-only updates. There is deliberately no production delete operation.
use super::{Item, MAX_ITEM_BYTES};
use crate::{err, Result};
use serde_json::{json, Value};
use std::{
    ffi::{c_void, CString},
    path::Path,
    ptr,
};

type CF = *const c_void;

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(value: CF);
    fn CFGetTypeID(value: CF) -> usize;
    fn CFStringCreateWithBytes(
        allocator: CF,
        bytes: *const u8,
        count: isize,
        encoding: u32,
        external: u8,
    ) -> CF;
    fn CFStringCreateMutableCopy(allocator: CF, capacity: isize, value: CF) -> CF;
    fn CFStringNormalize(value: CF, form: isize);
    fn CFStringGetLength(value: CF) -> isize;
    fn CFStringGetMaximumSizeForEncoding(length: isize, encoding: u32) -> isize;
    fn CFStringGetCString(value: CF, buffer: *mut i8, capacity: isize, encoding: u32) -> u8;
    fn CFDataCreate(allocator: CF, bytes: *const u8, count: isize) -> CF;
    fn CFDataGetTypeID() -> usize;
    fn CFDataGetLength(value: CF) -> isize;
    fn CFDataGetBytePtr(value: CF) -> *const u8;
    fn CFDictionaryCreate(
        allocator: CF,
        keys: *const CF,
        values: *const CF,
        count: isize,
        key_callbacks: CF,
        value_callbacks: CF,
    ) -> CF;
    fn CFDictionaryGetTypeID() -> usize;
    fn CFDictionaryGetValue(dictionary: CF, key: CF) -> CF;
    fn CFArrayCreate(allocator: CF, values: *const CF, count: isize, callbacks: CF) -> CF;
    static kCFBooleanTrue: CF;
    // These constants are opaque callback structures. Only their addresses
    // cross the C ABI; Rust never assumes or reads their layout.
    static kCFTypeDictionaryKeyCallBacks: u8;
    static kCFTypeDictionaryValueCallBacks: u8;
    static kCFTypeArrayCallBacks: u8;
}

#[link(name = "Security", kind = "framework")]
extern "C" {
    fn SecItemCopyMatching(query: CF, result: *mut CF) -> i32;
    fn SecItemUpdate(query: CF, attributes: CF) -> i32;
    fn SecKeychainOpen(path: *const i8, keychain: *mut CF) -> i32;
    static kSecClass: CF;
    static kSecClassGenericPassword: CF;
    static kSecAttrService: CF;
    static kSecAttrAccount: CF;
    static kSecMatchLimit: CF;
    static kSecMatchLimitOne: CF;
    static kSecMatchSearchList: CF;
    static kSecMatchItemList: CF;
    static kSecReturnData: CF;
    static kSecReturnAttributes: CF;
    static kSecReturnPersistentRef: CF;
    static kSecValueData: CF;
    static kSecValuePersistentRef: CF;
    #[cfg(test)]
    fn SecKeychainCreate(
        path: *const i8,
        password_length: u32,
        password: *const c_void,
        prompt: u8,
        access: CF,
        keychain: *mut CF,
    ) -> i32;
    #[cfg(test)]
    fn SecKeychainDelete(keychain: CF) -> i32;
    #[cfg(test)]
    fn SecItemAdd(attributes: CF, result: *mut CF) -> i32;
    #[cfg(test)]
    static kSecUseKeychain: CF;
}

const UTF8: u32 = 0x08000100;
struct Owned(CF);
impl Drop for Owned {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0) };
        }
    }
}
impl Owned {
    fn new(value: CF) -> Result<Self> {
        if value.is_null() {
            Err(err("keychain_unavailable", "无法准备 Keychain 请求"))
        } else {
            Ok(Self(value))
        }
    }
}
fn text(value: &str) -> Result<Owned> {
    Owned::new(unsafe {
        CFStringCreateWithBytes(ptr::null(), value.as_ptr(), value.len() as isize, UTF8, 0)
    })
}
fn data(value: &[u8]) -> Result<Owned> {
    Owned::new(unsafe { CFDataCreate(ptr::null(), value.as_ptr(), value.len() as isize) })
}
fn dictionary(entries: &[(CF, CF)]) -> Result<Owned> {
    let keys = entries.iter().map(|pair| pair.0).collect::<Vec<_>>();
    let values = entries.iter().map(|pair| pair.1).collect::<Vec<_>>();
    Owned::new(unsafe {
        CFDictionaryCreate(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            keys.len() as isize,
            ptr::addr_of!(kCFTypeDictionaryKeyCallBacks).cast(),
            ptr::addr_of!(kCFTypeDictionaryValueCallBacks).cast(),
        )
    })
}
fn array(values: &[CF]) -> Result<Owned> {
    Owned::new(unsafe {
        CFArrayCreate(
            ptr::null(),
            values.as_ptr(),
            values.len() as isize,
            ptr::addr_of!(kCFTypeArrayCallBacks).cast(),
        )
    })
}
fn bytes(value: CF, limit: usize) -> Result<Vec<u8>> {
    if value.is_null() || unsafe { CFGetTypeID(value) != CFDataGetTypeID() } {
        return Err(err("keychain_invalid_format", "Keychain 返回内容无法识别"));
    }
    let count = unsafe { CFDataGetLength(value) };
    if count < 0 || count as usize > limit {
        return Err(err("keychain_too_large", "Keychain 容器超出容量上限"));
    }
    if count == 0 {
        return Ok(vec![]);
    }
    let start = unsafe { CFDataGetBytePtr(value) };
    if start.is_null() {
        return Err(err("keychain_unavailable", "Keychain 内容不可读"));
    }
    Ok(unsafe { std::slice::from_raw_parts(start, count as usize) }.to_vec())
}

pub(super) fn normalize(value: &str) -> String {
    let normalized = (|| -> Result<String> {
        let input = text(value)?;
        let output = Owned::new(unsafe { CFStringCreateMutableCopy(ptr::null(), 0, input.0) })?;
        unsafe { CFStringNormalize(output.0, 2) }; // kCFStringNormalizationFormC
        let size = unsafe { CFStringGetMaximumSizeForEncoding(CFStringGetLength(output.0), UTF8) };
        if !(0..=65536).contains(&size) {
            return Err(err("keychain_unavailable", "配置目录无法标准化"));
        }
        let mut buffer = vec![0u8; size as usize + 1];
        if unsafe {
            CFStringGetCString(
                output.0,
                buffer.as_mut_ptr().cast(),
                buffer.len() as isize,
                UTF8,
            )
        } == 0
        {
            return Err(err("keychain_unavailable", "配置目录无法标准化"));
        }
        let count = buffer.iter().position(|b| *b == 0).unwrap_or(buffer.len());
        String::from_utf8(buffer[..count].to_vec())
            .map_err(|_| err("keychain_unavailable", "配置目录无法标准化"))
    })();
    normalized.unwrap_or_else(|_| value.into())
}

fn native_error(status: i32) -> crate::Error {
    match status {
        -128 | -25293 => err(
            "keychain_access_denied",
            "Keychain 访问未获允许；可在系统提示中允许后重查",
        ),
        -25308 => err("keychain_locked", "Keychain 当前不可交互访问；解锁后重查"),
        _ => err(
            "keychain_unavailable",
            "Keychain 当前不可用；没有把读取失败当作未发现登录",
        ),
    }
}
fn account(identity: Option<&Value>) -> Result<String> {
    if let Some(identity) = identity {
        return identity["account"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 256)
            .map(str::to_owned)
            .ok_or_else(|| err("keychain_item_changed", "Keychain 原件身份无法核对"));
    }
    if let Ok(value) = std::env::var("USER") {
        if !value.is_empty() && value.len() <= 256 {
            return Ok(value);
        }
    }
    let mut entry = std::mem::MaybeUninit::<libc::passwd>::uninit();
    let mut output = ptr::null_mut();
    let mut buffer = vec![0u8; 16384];
    if unsafe {
        libc::getpwuid_r(
            libc::geteuid(),
            entry.as_mut_ptr(),
            buffer.as_mut_ptr().cast(),
            buffer.len(),
            &mut output,
        )
    } != 0
        || output.is_null()
    {
        return Err(err(
            "keychain_unavailable",
            "无法确定当前用户的 Keychain 来源",
        ));
    }
    let entry = unsafe { entry.assume_init() };
    if entry.pw_name.is_null() {
        return Err(err(
            "keychain_unavailable",
            "无法确定当前用户的 Keychain 来源",
        ));
    }
    Ok(unsafe { std::ffi::CStr::from_ptr(entry.pw_name) }
        .to_string_lossy()
        .into_owned())
}
fn persistent(identity: &Value) -> Result<Vec<u8>> {
    let values = identity["persistent_ref"]
        .as_array()
        .filter(|a| !a.is_empty() && a.len() <= 65536)
        .ok_or_else(|| err("keychain_item_changed", "Keychain 原件身份无法核对"))?;
    values
        .iter()
        .map(|value| {
            value
                .as_u64()
                .filter(|n| *n <= 255)
                .map(|n| n as u8)
                .ok_or_else(|| err("keychain_item_changed", "Keychain 原件身份无法核对"))
        })
        .collect()
}

struct Query {
    entries: Vec<(CF, CF)>,
    owned: Vec<Owned>,
}
impl Query {
    fn new(service: &str, identity: Option<&Value>, keychain: Option<&Path>) -> Result<Self> {
        let service_value = text(service)?;
        let account_value = text(&account(identity)?)?;
        let mut entries = unsafe {
            vec![
                (kSecClass, kSecClassGenericPassword),
                (kSecAttrService, service_value.0),
                (kSecAttrAccount, account_value.0),
            ]
        };
        let mut owned = vec![service_value, account_value];
        if let Some(identity) = identity {
            if identity["service"] != service {
                return Err(err("keychain_item_changed", "Keychain 原来源不匹配"));
            }
            let reference = data(&persistent(identity)?)?;
            let list = array(&[reference.0])?;
            entries.push(unsafe { (kSecMatchItemList, list.0) });
            owned.extend([reference, list]);
        }
        if let Some(path) = keychain {
            let path = CString::new(path.to_string_lossy().as_bytes())
                .map_err(|_| err("keychain_unavailable", "测试 Keychain 路径无效"))?;
            let mut keychain = ptr::null();
            let status = unsafe { SecKeychainOpen(path.as_ptr(), &mut keychain) };
            if status != 0 {
                return Err(native_error(status));
            }
            let keychain = Owned::new(keychain)?;
            let list = array(&[keychain.0])?;
            entries.push(unsafe { (kSecMatchSearchList, list.0) });
            owned.extend([keychain, list]);
        }
        Ok(Self { entries, owned })
    }
    fn dictionary(&self) -> Result<Owned> {
        let _keep_alive = &self.owned;
        dictionary(&self.entries)
    }
}

pub(super) fn read(
    service: &str,
    identity: Option<&Value>,
    keychain: Option<&Path>,
) -> Result<Option<Item>> {
    let mut query = Query::new(service, identity, keychain)?;
    unsafe {
        query.entries.extend([
            (kSecMatchLimit, kSecMatchLimitOne),
            (kSecReturnData, kCFBooleanTrue),
            (kSecReturnAttributes, kCFBooleanTrue),
            (kSecReturnPersistentRef, kCFBooleanTrue),
        ])
    };
    let query = query.dictionary()?;
    let mut output = ptr::null();
    let status = unsafe { SecItemCopyMatching(query.0, &mut output) };
    if status == -25300 {
        return Ok(None);
    }
    if status != 0 {
        return Err(native_error(status));
    }
    let result = Owned::new(output)?;
    if unsafe { CFGetTypeID(result.0) != CFDictionaryGetTypeID() } {
        return Err(err("keychain_invalid_format", "Keychain 返回内容无法识别"));
    }
    let content = bytes(
        unsafe { CFDictionaryGetValue(result.0, kSecValueData) },
        MAX_ITEM_BYTES,
    )?;
    let reference = bytes(
        unsafe { CFDictionaryGetValue(result.0, kSecValuePersistentRef) },
        65536,
    )?;
    Ok(Some(Item {
        identity: json!({"persistent_ref":reference,"account":account(identity)?,"service":service}),
        bytes: content,
    }))
}

pub(super) fn update(
    service: &str,
    identity: &Value,
    replacement: &[u8],
    keychain: Option<&Path>,
) -> Result<()> {
    let query = Query::new(service, Some(identity), keychain)?.dictionary()?;
    let replacement = data(replacement)?;
    let attributes = dictionary(&[unsafe { (kSecValueData, replacement.0) }])?;
    let status = unsafe { SecItemUpdate(query.0, attributes.0) };
    match status {
        0 => Ok(()),
        -128 | -25293 | -25308 => Err(native_error(status)),
        _ => Err(err(
            "keychain_write_uncertain",
            "Keychain 更新结果需核对；请查询原任务，不要重提",
        )),
    }
}

#[cfg(test)]
pub(super) struct TestKeychain {
    handle: Owned,
    deleted: bool,
}
#[cfg(test)]
impl TestKeychain {
    pub(super) fn create(path: &Path) -> Result<Self> {
        if std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true")
            || std::env::var("RUNNER_ENVIRONMENT").as_deref() != Ok("github-hosted")
        {
            return Err(err(
                "keychain_test_requires_disposable_host",
                "原生 Keychain fixture 仅在一次性托管 CI 账号运行",
            ));
        }
        let path = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|_| err("keychain_unavailable", "测试 Keychain 路径无效"))?;
        let password = b"lintel-disposable-synthetic-keychain";
        let mut output = ptr::null();
        let status = unsafe {
            SecKeychainCreate(
                path.as_ptr(),
                password.len() as u32,
                password.as_ptr().cast(),
                0,
                ptr::null(),
                &mut output,
            )
        };
        if status != 0 {
            return Err(native_error(status));
        }
        Ok(Self {
            handle: Owned::new(output)?,
            deleted: false,
        })
    }
    pub(super) fn insert(&self, service: &str, payload: &[u8]) -> Result<()> {
        let service = text(service)?;
        let account = text(&account(None)?)?;
        let payload = data(payload)?;
        let query = dictionary(&unsafe {
            [
                (kSecClass, kSecClassGenericPassword),
                (kSecAttrService, service.0),
                (kSecAttrAccount, account.0),
                (kSecValueData, payload.0),
                (kSecUseKeychain, self.handle.0),
            ]
        })?;
        let status = unsafe { SecItemAdd(query.0, ptr::null_mut()) };
        if status == 0 {
            Ok(())
        } else {
            Err(native_error(status))
        }
    }
    pub(super) fn close(mut self) -> Result<()> {
        let status = unsafe { SecKeychainDelete(self.handle.0) };
        if status != 0 {
            return Err(native_error(status));
        }
        self.deleted = true;
        Ok(())
    }
}
#[cfg(test)]
impl Drop for TestKeychain {
    fn drop(&mut self) {
        if !self.deleted {
            unsafe {
                SecKeychainDelete(self.handle.0);
            }
        }
    }
}

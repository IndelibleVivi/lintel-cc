use crate::{err, Error, Result};
use serde::{
    de::{Error as DeError, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::{
    fmt,
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Component, Path},
};

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn guard(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        return Err(err("invalid_path", "需要绝对路径"));
    }
    let mut cursor = std::path::PathBuf::new();
    for component in path.components() {
        match component {
            Component::RootDir | Component::Normal(_) => cursor.push(component),
            _ => return Err(err("invalid_path", "路径不能包含父目录跳转")),
        }
        match fs::symlink_metadata(&cursor) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(err(
                    "symlink_target",
                    "目标路径包含符号链接，请选择实际目录",
                ))
            }
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err(err("path_unreadable", "无法检查目标路径")),
        }
    }
    Ok(())
}
pub fn private_dir(path: &Path) -> Result<()> {
    guard(path)?;
    fs::create_dir_all(path)?;
    let m = fs::metadata(path)?;
    if m.uid() != unsafe { libc::geteuid() } {
        return Err(err("wrong_owner", "数据目录不属于当前用户"));
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}
pub fn read(path: &Path, limit: u64) -> Result<Vec<u8>> {
    guard(path)?;
    // Refuse FIFOs and other non-regular files before opening: a blocking
    // open on a FIFO would stall the operation while holding the global lock.
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(err("file_limit", "文件类型或容量不受支持"));
    }
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() > limit {
        return Err(err("file_limit", "文件类型或容量不受支持"));
    }
    let mut bytes = Vec::new();
    (&mut file).take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(err("file_limit", "读取期间文件超过容量上限"));
    }
    Ok(bytes)
}
pub fn atomic(path: &Path, bytes: &[u8], mode: u32) -> Result<()> {
    write_atomic(path, bytes, mode, true)
}
/// Publish a complete new file without replacing a concurrently created target.
/// The temporary file and destination share a directory/filesystem. A hard link
/// atomically claims the unused name; removing the temporary name restores the
/// single-link invariant before the operation reports success.
pub fn atomic_new(path: &Path, bytes: &[u8], mode: u32) -> Result<()> {
    write_atomic(path, bytes, mode, false)
}
fn write_atomic(path: &Path, bytes: &[u8], mode: u32, overwrite: bool) -> Result<()> {
    guard(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| err("invalid_path", "缺少父目录"))?;
    let tmp = parent.join(format!(".lintel-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(mode)
            .open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(mode))?;
        guard(path)?;
        if overwrite {
            fs::rename(&tmp, path)?;
        } else {
            fs::hard_link(&tmp, path).map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    err(
                        "target_exists",
                        "目标文件已经出现；原内容保持不变，请核对原任务",
                    )
                } else {
                    e.into()
                }
            })?;
            fs::remove_file(&tmp)?;
        }
        std::fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}
pub fn save(path: &Path, value: &Value) -> Result<()> {
    atomic(path, &serde_json::to_vec_pretty(value)?, 0o600)
}
pub fn load(path: &Path) -> Result<Value> {
    parse(&read(path, 16 * 1024 * 1024)?)
}
pub fn snapshot(path: &Path) -> Result<Value> {
    guard(path)?;
    if !path.exists() {
        return Ok(Value::Null);
    }
    let m = fs::metadata(path)?;
    if m.uid() != unsafe { libc::geteuid() } || m.nlink() != 1 {
        return Err(err("unsafe_file", "文件所有者或硬链接状态不受支持"));
    }
    let bytes = read(path, 8 * 1024 * 1024)?;
    Ok(
        serde_json::json!({"digest":digest(&bytes),"inode":m.ino(),"device":m.dev(),"mode":m.mode(),"owner":m.uid()}),
    )
}
pub fn parse(bytes: &[u8]) -> Result<Value> {
    let mut d = serde_json::Deserializer::from_slice(bytes);
    let value = Unique::deserialize(&mut d)
        .map_err(|_| err("invalid_json", "JSON 格式损坏或包含重复键；原文件保持不变"))?
        .0;
    d.end()
        .map_err(|_| err("invalid_json", "JSON 文档包含多余内容"))?;
    Ok(value)
}
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        d.deserialize_any(UniqueVisitor)
    }
}
struct UniqueVisitor;
impl<'de> Visitor<'de> for UniqueVisitor {
    type Value = Unique;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "JSON without duplicate keys")
    }
    fn visit_bool<E: DeError>(self, v: bool) -> std::result::Result<Unique, E> {
        Ok(Unique(v.into()))
    }
    fn visit_i64<E: DeError>(self, v: i64) -> std::result::Result<Unique, E> {
        Ok(Unique(v.into()))
    }
    fn visit_u64<E: DeError>(self, v: u64) -> std::result::Result<Unique, E> {
        Ok(Unique(v.into()))
    }
    fn visit_f64<E: DeError>(self, v: f64) -> std::result::Result<Unique, E> {
        Ok(Unique(
            serde_json::Number::from_f64(v)
                .map(Value::Number)
                .ok_or_else(|| E::custom("invalid number"))?,
        ))
    }
    fn visit_str<E: DeError>(self, v: &str) -> std::result::Result<Unique, E> {
        Ok(Unique(v.into()))
    }
    fn visit_string<E: DeError>(self, v: String) -> std::result::Result<Unique, E> {
        Ok(Unique(v.into()))
    }
    fn visit_none<E: DeError>(self) -> std::result::Result<Unique, E> {
        Ok(Unique(Value::Null))
    }
    fn visit_unit<E: DeError>(self) -> std::result::Result<Unique, E> {
        Ok(Unique(Value::Null))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<Unique, A::Error> {
        let mut v = Vec::new();
        while let Some(x) = seq.next_element::<Unique>()? {
            v.push(x.0)
        }
        Ok(Unique(v.into()))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Unique, A::Error> {
        let mut v = Map::new();
        while let Some((k, x)) = map.next_entry::<String, Unique>()? {
            if v.insert(k, x.0).is_some() {
                return Err(A::Error::custom("duplicate key"));
            }
        }
        Ok(Unique(Value::Object(v)))
    }
}
impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        err("io_error", "文件操作未完成；检查访问权限与可用空间")
    }
}
impl From<serde_json::Error> for Error {
    fn from(_: serde_json::Error) -> Self {
        err("invalid_json", "无法处理该 JSON 数据")
    }
}

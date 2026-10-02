//! Local-only allowlisted control API. Browser data is never read by this host.
use fs2::FileExt;
use serde_json::{json, Value};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
pub const MAX_FRAME: usize = 65536;
type Result<T> = std::result::Result<T, String>;
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
pub fn random() -> String {
    let mut b = [0u8; 32];
    getrandom::getrandom(&mut b).expect("OS entropy");
    b.iter().map(|b| format!("{b:02x}")).collect()
}
fn root() -> PathBuf {
    if let Some(s) = std::env::var_os("LINTEL_BROWSER_STATE") {
        return PathBuf::from(s);
    }
    let home = std::env::var_os("HOME").expect("HOME is required");
    PathBuf::from(home).join(if cfg!(target_os = "macos") {
        "Library/Application Support/Lintel/browser-bridge"
    } else {
        ".local/state/lintel/browser-bridge"
    })
}
fn err(e: String) -> Value {
    let (code, message) = e.split_once(':').unwrap_or((&e, &e));
    json!({"ok":false,"error":{"code":code,"message":message}})
}
fn envelope(r: Result<Value>) -> Value {
    match r {
        Ok(data) => json!({"ok":true,"data":data}),
        Err(e) => err(e),
    }
}
fn fields(v: &Value, allowed: &[&str]) -> Result<()> {
    let m = v.as_object().ok_or("invalid_object")?;
    if m.keys().any(|k| !allowed.contains(&k.as_str())) {
        return Err("unknown_field".into());
    }
    Ok(())
}
fn string<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str().ok_or(format!("missing_field:{k}"))
}
fn id(v: &Value, k: &str) -> Result<String> {
    let s = string(v, k)?;
    if s.len() < 8
        || s.len() > 80
        || !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!("invalid_id:{k}"));
    }
    Ok(s.to_owned())
}
fn transaction<F>(path: &Path, f: F) -> Result<Value>
where
    F: FnOnce(&mut Value) -> Result<Value>,
{
    if path
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        return Err("state_symlink".into());
    }
    fs::create_dir_all(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    let lock_path = path.join("bridge.lock");
    if lock_path
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        return Err("lock_symlink".into());
    }
    let mut options = OpenOptions::new();
    options.create(true).read(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let lock = options.open(lock_path).map_err(|e| e.to_string())?;
    lock.lock_exclusive().map_err(|e| e.to_string())?;
    let db_path = path.join("bridge.json");
    if db_path
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        return Err("state_symlink".into());
    }
    let mut db = if db_path.exists() {
        let mut bytes = Vec::new();
        fs::File::open(&db_path)
            .map_err(|e| e.to_string())?
            .take(32 * 1024 * 1024)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        serde_json::from_slice(&bytes).map_err(|e| format!("invalid_state:{e}"))?
    } else {
        json!({"pairings":{},"instances":{},"operations":{},"allowed_extensions":[]})
    };
    // Persist conflicts as well as successful commands.
    let result = f(&mut db);
    let tmp = path.join(format!("bridge-{}.tmp", random()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(&tmp).map_err(|e| e.to_string())?;
    file.write_all(&serde_json::to_vec(&db).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    fs::rename(tmp, db_path).map_err(|e| e.to_string())?;
    fs::File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    FileExt::unlock(&lock).map_err(|e| e.to_string())?;
    result
}
fn action(a: &Value) -> Result<()> {
    let kind = string(a, "kind")?;
    let extra: &[&str] = match kind {
        "clear" => &["origins", "types", "cookieStoreId"],
        "clearProfileCache" => &[],
        "webrtc" => &["setting"],
        "sitePermission" => &["origins", "setting"],
        "proxy" => &["origins", "port"],
        "blockSites" => &["origins"],
        "pauseRules" => &["minutes"],
        "restore" => &["receiptId"],
        _ => return Err("unknown_action".into()),
    };
    let mut all = vec!["kind"];
    all.extend(extra);
    fields(a, &all)?;
    if extra.contains(&"origins") {
        let origins = a["origins"].as_array().ok_or("invalid_origins")?;
        if origins.is_empty()
            || origins.len() > 2
            || origins.iter().any(|o| {
                !matches!(
                    o.as_str(),
                    Some("https://claude.ai" | "https://console.anthropic.com")
                )
            })
        {
            return Err("site_not_allowlisted".into());
        }
    }
    match kind {
        "clear" => {
            let types = a["types"].as_array().ok_or("invalid_types")?;
            if types.is_empty()
                || types.len() > 6
                || types.iter().any(|t| {
                    !matches!(
                        t.as_str(),
                        Some(
                            "cookies"
                                | "localStorage"
                                | "indexedDB"
                                | "serviceWorkers"
                                | "cacheStorage"
                                | "cache"
                        )
                    )
                })
            {
                return Err("invalid_types".into());
            }
            if let Some(store) = a.get("cookieStoreId") {
                let s = store.as_str().ok_or("invalid_store")?;
                if s != "firefox-default"
                    && !s
                        .strip_prefix("firefox-container-")
                        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
                {
                    return Err("invalid_store".into());
                }
            }
        }
        "webrtc" => {
            if !matches!(
                a["setting"].as_str(),
                Some("default" | "default_public_interface_only" | "disable_non_proxied_udp")
            ) {
                return Err("invalid_policy".into());
            }
        }
        "sitePermission" => {
            if !matches!(
                a["setting"].as_str(),
                Some("location" | "camera" | "microphone" | "notifications")
            ) {
                return Err("invalid_permission".into());
            }
        }
        "proxy" => {
            if !a["port"]
                .as_u64()
                .is_some_and(|n| (1024..=65535).contains(&n))
            {
                return Err("invalid_port".into());
            }
        }
        "pauseRules" => {
            if !a["minutes"].as_u64().is_some_and(|n| (1..=60).contains(&n)) {
                return Err("invalid_duration".into());
            }
        }
        "restore" => {
            id(a, "receiptId")?;
        }
        _ => {}
    }
    Ok(())
}
fn public_instance(key: &str, v: &Value) -> Value {
    json!({"instance_id":key,"label":v["label"],"browser":v["browser"],"extension_id":v["extension_id"],"paired":v["paired"],"conflict":v["conflict"],"last_seen":v["last_seen"],"online":v["last_seen"].as_u64().is_some_and(|t|now().saturating_sub(t)<20)})
}
fn control_inner(db: &mut Value, r: &Value) -> Result<Value> {
    let op = string(r, "op")?;
    match op {
        "pair_create" => {
            fields(r, &["op"])?;
            let challenge = random();
            let code = random()[..8].to_uppercase();
            db["pairings"][&challenge] =
                json!({"code":code,"expires_at":now()+300,"status":"waiting"});
            Ok(json!({"challenge":challenge,"code":code,"expires_at":now()+300}))
        }
        "pair_pending" => {
            fields(r, &["op"])?;
            Ok(Value::Array(db["pairings"].as_object().unwrap().iter().filter(|(_,v)|v["status"]=="requested" && v["expires_at"].as_u64().unwrap_or(0)>now()).map(|(key,v)|json!({"challenge":key,"code":v["code"],"instance_id":v["instance_id"],"label":v["label"],"browser":v["browser"],"extension_id":v["extension_id"]})).collect()))
        }
        "pair_approve" => {
            fields(r, &["op", "challenge"])?;
            let c = id(r, "challenge")?;
            let p = db["pairings"][&c].clone();
            if p["status"] != "requested" || p["expires_at"].as_u64().unwrap_or(0) <= now() {
                return Err("pairing_expired_or_not_requested".into());
            }
            let i = string(&p, "instance_id")?;
            db["instances"][i] = json!({"token":p["token"],"label":p["label"],"browser":p["browser"],"extension_id":p["extension_id"],"paired":true,"conflict":false,"last_seen":0});
            db["pairings"][&c]["status"] = json!("approved");
            db["pairings"][&c].as_object_mut().unwrap().remove("token");
            Ok(public_instance(i, &db["instances"][i]))
        }
        "instances" => {
            fields(r, &["op"])?;
            Ok(Value::Array(
                db["instances"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| public_instance(k, v))
                    .collect(),
            ))
        }
        "submit" => {
            fields(r, &["op", "instance_id", "operation_id", "action"])?;
            let i = id(r, "instance_id")?;
            let oid = id(r, "operation_id")?;
            let v = &db["instances"][&i];
            if v["paired"] != true || v["conflict"] == true {
                return Err("pairing_required_or_conflicted".into());
            }
            action(&r["action"])?;
            let key = format!("{i}:{oid}");
            if !db["operations"][&key].is_null() {
                if db["operations"][&key]["action"] != r["action"] {
                    return Err("operation_id_conflict".into());
                }
                return Ok(db["operations"][&key].clone());
            }
            if db["operations"].as_object().unwrap().len() >= 1000 {
                return Err(
                    "journal_capacity:Export and archive receipts before accepting more operations"
                        .into(),
                );
            }
            let result = json!({"instance_id":i,"id":oid,"action":r["action"],"phase":"awaiting-browser-confirmation","created_at":now()});
            db["operations"][key] = result.clone();
            Ok(result)
        }
        "query" => {
            fields(r, &["op", "instance_id", "operation_id"])?;
            let key = format!("{}:{}", id(r, "instance_id")?, id(r, "operation_id")?);
            let result = db["operations"][key].clone();
            if result.is_null() {
                return Err("unknown_operation".into());
            }
            Ok(result)
        }
        "allow_extension" => {
            fields(r, &["op", "extension_id"])?;
            let ext = string(r, "extension_id")?;
            if !valid_extension(ext) {
                return Err("invalid_extension_id".into());
            }
            let list = db["allowed_extensions"].as_array_mut().unwrap();
            if !list.contains(&json!(ext)) {
                list.push(json!(ext));
            }
            Ok(json!({"allowed":ext}))
        }
        _ => Err("unknown_control_operation".into()),
    }
}
fn valid_extension(s: &str) -> bool {
    (s.len() == 32 && s.chars().all(|c| ('a'..='p').contains(&c))) || s == "lintel@lintel.local"
}
/// GUI/CLI entry: local-only fixed operations. No path, executable, or file API.
pub fn control(request: Value) -> Value {
    control_at(&root(), request)
}
/// Explicit storage root for synthetic tests and embedding. Never accepts a remote message path.
pub fn control_at(path: &Path, request: Value) -> Value {
    envelope(transaction(path, |db| control_inner(db, &request)))
}
fn process_alive(pid: u64) -> bool {
    if pid == 0 || pid > i32::MAX as u64 {
        return false;
    }
    // Signal 0 only checks the process; it sends no signal or mutation.
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}
fn auth<'a>(
    db: &'a mut Value,
    r: &Value,
    extension: &str,
    connection: &str,
) -> Result<&'a mut Value> {
    let i = id(r, "instance_id")?;
    let token = id(r, "token")?;
    let v = &mut db["instances"][i];
    if v["paired"] != true || v["token"] != token || v["extension_id"] != extension {
        return Err("not_paired".into());
    }
    if v["conflict"] == true {
        return Err(
            "duplicate_instance_conflict:Generate a new local instance identity and pair again"
                .into(),
        );
    }
    if v["connection"].as_str().is_some_and(|c| c != connection)
        && v["connection_pid"].as_u64().is_some_and(process_alive)
        && v["last_seen"]
            .as_u64()
            .is_some_and(|t| now().saturating_sub(t) < 20)
    {
        v["conflict"] = json!(true);
        return Err("duplicate_instance_conflict:Two live connections claimed the same installation identity".into());
    }
    v["connection"] = json!(connection);
    v["connection_pid"] = json!(std::process::id());
    v["last_seen"] = json!(now());
    Ok(v)
}
/// Native pipe handler; caller extension identity is supplied only from browser process arguments.
pub fn native_at(path: &Path, request: Value, extension: &str, connection: &str) -> Value {
    let correlation = request.get("request_id").cloned().unwrap_or(Value::Null);
    let mut result = envelope(transaction(path, |db| {
        if !db["allowed_extensions"]
            .as_array()
            .unwrap()
            .contains(&json!(extension))
        {
            return Err("extension_not_allowed".into());
        }
        let op = string(&request, "op")?;
        match op {
            "pair_request" => {
                fields(
                    &request,
                    &[
                        "op",
                        "request_id",
                        "instance_id",
                        "token",
                        "code",
                        "label",
                        "browser",
                    ],
                )?;
                let i = id(&request, "instance_id")?;
                let token = id(&request, "token")?;
                if token.len() != 64 {
                    return Err("invalid_token".into());
                }
                let code = string(&request, "code")?;
                let label = string(&request, "label")?;
                if label.len() > 80 || label.chars().any(char::is_control) {
                    return Err("invalid_label".into());
                }
                if !matches!(request["browser"].as_str(), Some("chromium" | "firefox")) {
                    return Err("invalid_browser".into());
                }
                let (challenge, p) = db["pairings"]
                    .as_object_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|(_, v)| {
                        v["code"] == code
                            && v["status"] == "waiting"
                            && v["expires_at"].as_u64().unwrap_or(0) > now()
                    })
                    .ok_or("invalid_pairing_code")?;
                p["status"] = json!("requested");
                p["instance_id"] = json!(i);
                p["token"] = json!(token);
                p["label"] = json!(label);
                p["browser"] = request["browser"].clone();
                p["extension_id"] = json!(extension);
                Ok(json!({"paired":false,"pending":true,"challenge":challenge,"code":code}))
            }
            "poll" => {
                fields(&request, &["op", "request_id", "instance_id", "token"])?;
                auth(db, &request, extension, connection)?;
                let i = string(&request, "instance_id")?;
                let proposals: Vec<Value> = db["operations"]
                    .as_object()
                    .unwrap()
                    .values()
                    .filter(|v| {
                        v["instance_id"] == i && v["phase"] == "awaiting-browser-confirmation"
                    })
                    .take(2)
                    .cloned()
                    .collect();
                Ok(json!({"paired":true,"proposals":proposals}))
            }
            "receipt" => {
                fields(
                    &request,
                    &["op", "request_id", "instance_id", "token", "receipt"],
                )?;
                auth(db, &request, extension, connection)?;
                let receipt = &request["receipt"];
                fields(receipt, &["id", "phase", "result", "error", "completedAt"])?;
                let oid = id(receipt, "id")?;
                if !matches!(
                    receipt["phase"].as_str(),
                    Some("running" | "completed" | "uncertain" | "rejected")
                ) {
                    return Err("invalid_receipt_phase".into());
                }
                if receipt.to_string().len() > 16000 {
                    return Err("receipt_too_large".into());
                }
                let key = format!("{}:{oid}", string(&request, "instance_id")?);
                let operation = &mut db["operations"][key];
                if operation.is_null() {
                    return Err("unknown_operation".into());
                }
                if matches!(
                    operation["phase"].as_str(),
                    Some("completed" | "uncertain" | "rejected")
                ) {
                    return Ok(operation.clone());
                }
                operation["phase"] = receipt["phase"].clone();
                operation["receipt"] = receipt.clone();
                Ok(operation.clone())
            }
            _ => Err("unknown_native_operation".into()),
        }
    }));
    result["request_id"] = correlation;
    result
}
pub fn native(request: Value, extension: &str, connection: &str) -> Value {
    native_at(&root(), request, extension, connection)
}
pub fn release(connection: &str) {
    let _ = transaction(&root(), |db| {
        for v in db["instances"].as_object_mut().unwrap().values_mut() {
            if v["connection"] == connection {
                v.as_object_mut().unwrap().remove("connection");
                v["last_seen"] = json!(0);
            }
        }
        Ok(Value::Null)
    });
}
pub fn read_frame<R: Read>(r: &mut R) -> Result<Option<Value>> {
    let mut header = [0u8; 4];
    match r.read(&mut header[..1]) {
        Ok(0) => return Ok(None),
        Ok(_) => {}
        Err(e) => return Err(e.to_string()),
    }
    r.read_exact(&mut header[1..]).map_err(|e| e.to_string())?;
    let n = u32::from_ne_bytes(header) as usize;
    if n == 0 || n > MAX_FRAME {
        return Err("invalid_frame_length".into());
    }
    let mut body = vec![0; n];
    r.read_exact(&mut body).map_err(|e| e.to_string())?;
    Ok(Some(
        serde_json::from_slice(&body).map_err(|e| format!("invalid_json:{e}"))?,
    ))
}
pub fn write_frame<W: Write>(w: &mut W, v: &Value) -> Result<()> {
    let b = serde_json::to_vec(v).map_err(|e| e.to_string())?;
    if b.len() > MAX_FRAME {
        return Err("response_too_large".into());
    }
    w.write_all(&(b.len() as u32).to_ne_bytes())
        .and_then(|_| w.write_all(&b))
        .and_then(|_| w.flush())
        .map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn path() -> PathBuf {
        std::env::temp_dir().join(format!("lintel-host-test-{}", random()))
    }
    fn setup(p: &Path) -> (String, String) {
        let ext = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        assert_eq!(
            control_at(p, json!({"op":"allow_extension","extension_id":ext}))["ok"],
            true
        );
        let pair = control_at(p, json!({"op":"pair_create"}));
        let token = random();
        let req = json!({"op":"pair_request","request_id":"r","instance_id":"instance-one","token":token,"code":pair["data"]["code"],"label":"Synthetic","browser":"chromium"});
        assert_eq!(native_at(p, req, ext, "one")["ok"], true);
        assert_eq!(
            control_at(
                p,
                json!({"op":"pair_approve","challenge":pair["data"]["challenge"]})
            )["ok"],
            true
        );
        (ext.into(), token)
    }
    #[test]
    fn pairing_and_duplicate_live_instance_conflict() {
        let p = path();
        let (ext, token) = setup(&p);
        let poll = json!({"op":"poll","instance_id":"instance-one","token":token});
        assert_eq!(native_at(&p, poll.clone(), &ext, "one")["ok"], true);
        assert_eq!(
            native_at(&p, poll.clone(), &ext, "two")["error"]["code"],
            "duplicate_instance_conflict"
        );
        assert_eq!(
            native_at(&p, poll, &ext, "one")["error"]["code"],
            "duplicate_instance_conflict"
        );
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn durable_receipt_deduplicates_after_lost_ack() {
        let p = path();
        let (ext, token) = setup(&p);
        let submit = json!({"op":"submit","instance_id":"instance-one","operation_id":"operation-one","action":{"kind":"webrtc","setting":"disable_non_proxied_udp"}});
        assert_eq!(control_at(&p, submit.clone())["ok"], true);
        let receipt = json!({"op":"receipt","instance_id":"instance-one","token":token,"receipt":{"id":"operation-one","phase":"completed","result":{"verification":"browser-acknowledged"}}});
        assert_eq!(native_at(&p, receipt, &ext, "one")["ok"], true);
        assert_eq!(control_at(&p, submit)["data"]["phase"], "completed");
        assert_eq!(
            control_at(
                &p,
                json!({"op":"query","instance_id":"instance-one","operation_id":"operation-one"})
            )["data"]["phase"],
            "completed"
        );
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn unknown_actions_and_origins_rejected() {
        let p = path();
        setup(&p);
        for action in [
            json!({"kind":"exec","command":"anything"}),
            json!({"kind":"clear","origins":["https://mail.example"],"types":["cookies"]}),
        ] {
            assert_eq!(
                control_at(
                    &p,
                    json!({"op":"submit","instance_id":"instance-one","operation_id":"operation-one","action":action})
                )["ok"],
                false
            );
        }
        fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn frame_bound_and_partial_header() {
        assert!(read_frame(&mut std::io::Cursor::new(
            (MAX_FRAME as u32 + 1).to_ne_bytes()
        ))
        .is_err());
        assert!(read_frame(&mut std::io::Cursor::new(vec![1])).is_err());
        let mut data = Vec::new();
        write_frame(&mut data, &json!({"ok":true})).unwrap();
        assert_eq!(
            read_frame(&mut std::io::Cursor::new(data))
                .unwrap()
                .unwrap()["ok"],
            true
        );
    }
    #[test]
    fn dead_host_connection_can_reconnect_without_false_clone_conflict() {
        let p = path();
        let (ext, token) = setup(&p);
        let poll = json!({"op":"poll","instance_id":"instance-one","token":token});
        assert_eq!(native_at(&p, poll.clone(), &ext, "old")["ok"], true);
        transaction(&p, |db| {
            db["instances"]["instance-one"]["connection_pid"] = json!(0);
            Ok(Value::Null)
        })
        .unwrap();
        assert_eq!(native_at(&p, poll, &ext, "restarted")["ok"], true);
        fs::remove_dir_all(p).unwrap();
    }
}

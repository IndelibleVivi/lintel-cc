//! User-approved installation of caller-supplied static Linux runner resources.
use super::*;
use sha2::{Digest, Sha256};
const MAX_BUNDLE: usize = 32 * 1024 * 1024;
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hash(value: &Value) -> String {
    digest(&serde_json::to_vec(value).unwrap())
}
fn checked_digest(value: &Value) -> Result<&str> {
    value
        .as_str()
        .filter(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        })
        .ok_or_else(|| failure("invalid_bundle", "Runner 校验值无效"))
}
pub(super) fn runner_program(bound: Option<&Value>) -> Result<String> {
    match bound {
        Some(value) => Ok(format!(
            "\"$HOME/.local/share/lintel/runners/{}/lintel\"",
            checked_digest(value)?
        )),
        None => Ok("lintel".into()),
    }
}
fn shell_script(script: &str) -> Vec<String> {
    vec![format!("sh -c '{}'", script.replace('\'', "'\\''"))]
}
// No Claude paths are opened. All variable text is encoded before JSON output.
const PROBE_BODY: &str = r#"
os=$(uname -s); arch=$(uname -m); uid=$(id -u)
[ "$os" = Linux ] || { printf '{"ok":false,"error":{"code":"platform_unsupported","message":"内置 runner 安装仅支持 Linux"}}\n'; exit 1; }
mid=''; if [ -r /etc/machine-id ]; then mid=$(cat /etc/machine-id); fi
for tool in base64 sha256sum stat mktemp chmod ln rm mkdir cat tr cut; do
  command -v "$tool" >/dev/null 2>&1 || { printf '{"ok":false,"error":{"code":"install_tools_missing","message":"Linux 用户级安装所需基础工具不可用"}}\n'; exit 1; }
done
home=$(printf '%s' "$HOME" | base64 | tr -d '\n')
path=$(command -v lintel 2>/dev/null || true); path=$(printf '%s' "$path" | base64 | tr -d '\n')
probe="$os|$arch|$uid|$mid|$home"
"#;
const PROBE_END: &str = r#"
printf '{"ok":true,"data":{"probe":"%s","path_runner":"%s"}}\n' "$probe" "$path"
"#;
struct Bundle {
    meta: Value,
    bytes: Vec<u8>,
}
impl Controller {
    pub(super) fn runner_call(
        &self,
        alias: &str,
        payload: &Value,
        submit: bool,
        bound: Option<&Value>,
    ) -> Result<Value> {
        let Some(bound) = bound else {
            return self.transport.call(alias, payload, submit);
        };
        let program = runner_program(Some(bound))?;
        let mut bytes = serde_json::to_vec(payload).unwrap();
        bytes.push(b'\n');
        if bytes.len() > MAX_JSON {
            return Err(failure("request_too_large", "请求超过 SSH transport 上限"));
        }
        let mode = if submit { "submit" } else { "request" };
        // The only interpolated field is a validated lowercase SHA-256.
        let command = vec![format!("{program} {mode}")];
        self.transport
            .wire(alias, payload, submit, &bytes, &command)
    }
    pub(super) fn binding(&self, alias: &str) -> Result<Option<Value>> {
        let path = self.state.join("bindings").join(format!("{alias}.json"));
        if !path.exists() {
            return Ok(None);
        }
        let value = load(&path)?;
        checked_digest(&value["digest"])?;
        Ok(Some(value["digest"].clone()))
    }
    fn probe(&self, alias: &str) -> Result<Value> {
        let script = format!("{PROBE_BODY}{PROBE_END}");
        let response = self.transport.wire(
            alias,
            &json!({"command":"probe_runner"}),
            false,
            &[],
            &shell_script(&script),
        )?;
        if response["ok"] != true {
            return Err(match response["error"]["code"].as_str() {
                Some("platform_unsupported") => failure(
                    "platform_unsupported",
                    "内置 runner 安装只支持 Linux x86_64 / arm64",
                ),
                Some("install_tools_missing") => failure(
                    "install_tools_missing",
                    "Linux 用户级安装所需基础工具不可用；尚未安装",
                ),
                _ => failure("probe_failed", "无法取得远端安装条件；尚未安装 runner"),
            });
        }
        let raw = field(&response["data"], "probe")?;
        let parts: Vec<_> = raw.split('|').collect();
        if parts.len() != 5
            || parts[0] != "Linux"
            || !["x86_64", "aarch64", "arm64"].contains(&parts[1])
        {
            return Err(failure(
                "platform_unsupported",
                "内置安装支持 Linux x86_64 / arm64；此目标系统或架构尚不支持",
            ));
        }
        if parts[2].parse::<u32>().is_err()
            || parts[3].len() != 32
            || !parts[3].bytes().all(|c| c.is_ascii_hexdigit())
            || parts[4].is_empty()
        {
            return Err(failure(
                "target_identity_unavailable",
                "无法取得 Linux machine-id、目标用户及 home 身份；尚未安装",
            ));
        }
        let target = if parts[1] == "x86_64" {
            "x86_64-unknown-linux-musl"
        } else {
            "aarch64-unknown-linux-musl"
        };
        Ok(
            json!({"os":"Linux","architecture":parts[1],"uid":parts[2],"target":target,"identity":digest(raw.as_bytes()),"path_runner_present":response["data"]["path_runner"].as_str().is_some_and(|s| !s.is_empty())}),
        )
    }
    fn bundle(&self, target: &str) -> Result<Bundle> {
        if !["x86_64-unknown-linux-musl", "aarch64-unknown-linux-musl"].contains(&target) {
            return Err(failure("platform_unsupported", "Runner 目标架构无效"));
        }
        let manifest = load(&self.bundles.join("manifest.json")).map_err(|_| {
            failure(
                "bundle_unavailable",
                "本次调用未提供 Linux runner 资源；请指定完整的静态 runner 资源目录",
            )
        })?;
        let meta = manifest["runners"]
            .as_array()
            .and_then(|all| all.iter().find(|r| r["target"] == target))
            .ok_or_else(|| failure("bundle_unavailable", "提供的资源目录缺少目标架构的 runner"))?
            .clone();
        let sha = checked_digest(&meta["sha256"])?;
        if meta["protocol"] != 1 || meta["version"].as_str().is_none() {
            return Err(failure(
                "bundle_incompatible",
                "内置 runner manifest 协议无效",
            ));
        }
        let mut bytes = Vec::new();
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(self.bundles.join(target).join("lintel"))
            .map_err(|_| failure("bundle_unavailable", "内置 runner 文件不可读"))?
            .take(MAX_BUNDLE as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(storage)?;
        if bytes.is_empty()
            || bytes.len() > MAX_BUNDLE
            || meta["bytes"].as_u64() != Some(bytes.len() as u64)
            || digest(&bytes) != sha
        {
            return Err(failure(
                "bundle_changed",
                "内置 runner 与 manifest 不一致；没有上传",
            ));
        }
        Ok(Bundle { meta, bytes })
    }
    fn install_root(&self, alias: &str) -> Result<PathBuf> {
        let root = self.state.join("installations").join(alias);
        private_dir(&root)?;
        Ok(root)
    }
    pub(super) fn install_record_exists(&self, alias: &str, payload: &Value) -> bool {
        payload["install_id"].as_str().is_some_and(|id| {
            valid_id(id).is_ok()
                && self
                    .state
                    .join("installations")
                    .join(alias)
                    .join(format!("{id}.json"))
                    .exists()
        })
    }
    pub(super) fn install_inventory(&self) -> Result<Vec<Value>> {
        let mut result = vec![];
        let root = self.state.join("installations");
        if !root.exists() {
            return Ok(result);
        }
        for alias in fs::read_dir(root).map_err(storage)? {
            let alias = alias.map_err(storage)?;
            if !alias.file_type().map_err(storage)?.is_dir() {
                continue;
            }
            for item in fs::read_dir(alias.path()).map_err(storage)? {
                let path = item.map_err(storage)?.path();
                if path.extension().is_some_and(|s| s == "json") {
                    if let Ok(record) = load(&path) {
                        result.push(record);
                    }
                }
            }
        }
        Ok(result)
    }
    pub(super) fn install_dispatch(&self, alias: &str, payload: &Value) -> Result<Value> {
        if payload["op"] == "prepare_runner" {
            exact_operation_fields(payload)?;
            let root = self.install_root(alias)?;
            let _held = lock(&root.join("install.lock"))?;
            if self
                .install_inventory()?
                .iter()
                .any(|r| r["alias"] == alias && r["status"] == "needs_reconciliation")
            {
                return Err(failure(
                    "install_reconciliation_required",
                    "此主机有结果未确认的安装；请先核对原安装，不能再次上传",
                ));
            }
            let probe = self.probe(alias)?;
            let bundle = self.bundle(field(&probe, "target")?)?;
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .to_string();
            let id = format!(
                "install-{}",
                &hash(&json!([alias, probe, bundle.meta, stamp]))[..24]
            );
            let mut plan = json!({"install_id":id,"alias":alias,"probe":probe,"bundle":bundle.meta,"destination":format!("$HOME/.local/share/lintel/runners/{}/lintel",bundle.meta["sha256"].as_str().unwrap()),"effects":["上传本次调用提供的静态 runner 到当前 SSH 用户的专用版本目录","核验 SHA-256 与执行权限，再用 discover 核验 durable submit 能力","成功后 Lintel 绑定此版本；已有任务继续使用原 runner"],"status":"previewed","created_at":stamp});
            plan["approval"] = json!(hash(&plan));
            save(&root.join(format!("{id}.json")), &plan)?;
            return Ok(json!({"ok":true,"data":plan}));
        }
        exact_operation_fields(payload)?;
        let id = valid_id(field(payload, "install_id")?)?;
        let root = self.install_root(alias)?;
        let _held = lock(&root.join("install.lock"))?;
        let path = root.join(format!("{id}.json"));
        let mut record = load(&path)?;
        if record["install_id"] != id || record["alias"] != alias {
            return Err(failure("install_record_invalid", "安装记录与主机不匹配"));
        }
        checked_digest(&record["bundle"]["sha256"])?;
        if payload["op"] == "query_install" && record["status"] == "previewed" {
            return Ok(json!({"ok":true,"data":record}));
        }
        let probe = self.probe(alias)?;
        if probe["identity"] != record["probe"]["identity"]
            || probe["target"] != record["probe"]["target"]
        {
            return Err(failure(
                "stale_install_target",
                "目标主机、用户或平台在预览后变化；没有安装或切换绑定",
            ));
        }
        if payload["op"] == "install_runner" && record["status"] == "previewed" {
            if payload["approval"] != record["approval"] {
                return Err(failure("approval_required", "请批准本次具体安装预览"));
            }
            let bundle = self.bundle(field(&probe, "target")?)?;
            if bundle.meta != record["bundle"] {
                return Err(failure(
                    "stale_install_bundle",
                    "内置 runner 在预览后变化；请重新预览",
                ));
            }
            // Persist before the sole possible upload. A lost ACK cannot authorize retry.
            record["status"] = json!("needs_reconciliation");
            save(&path, &record)?;
            let upload = self.upload_script(&record)?;
            let response = self.transport.wire(
                alias,
                &json!({"command":"install_runner","approval":record["approval"]}),
                true,
                &bundle.bytes,
                &shell_script(&upload),
            );
            match response {
                Ok(value) if value["ok"] == true => {}
                Ok(value) => {
                    record["last_error"] = value["error"].clone();
                    save(&path, &record)?;
                    return Ok(json!({"ok":true,"data":record}));
                }
                Err(error) => {
                    save(&path, &record)?;
                    return Err(error);
                }
            }
        }
        // Repeated install and query_install are read-only from this point.
        self.verify_install(alias, &path, record)
    }
    fn upload_script(&self, record: &Value) -> Result<String> {
        let sha = checked_digest(&record["bundle"]["sha256"])?;
        let identity = checked_digest(&record["probe"]["identity"])?;
        Ok(format!(
            r#"set -eu
fail() {{ printf '{{"ok":false,"error":{{"code":"install_failed","message":"用户级 runner 安装未完成；核对原安装后再决定下一步"}}}}\n'; exit 1; }}
{PROBE_BODY}
[ "$(printf '%s' "$probe" | sha256sum | cut -d ' ' -f 1)" = '{identity}' ] || fail
umask 077
for dir in "$HOME/.local" "$HOME/.local/share" "$HOME/.local/share/lintel" "$HOME/.local/share/lintel/runners" "$HOME/.local/share/lintel/runners/{sha}"; do
  [ ! -L "$dir" ] || fail
  if [ ! -e "$dir" ]; then mkdir "$dir" || fail; fi
  [ -d "$dir" ] && [ "$(stat -c %u "$dir")" = "$uid" ] || fail
done
dir="$HOME/.local/share/lintel/runners/{sha}"
if [ -e "$dir/lintel" ] || [ -L "$dir/lintel" ]; then
  [ ! -L "$dir/lintel" ] && [ -f "$dir/lintel" ] && [ "$(sha256sum "$dir/lintel" | cut -d ' ' -f 1)" = '{sha}' ] && [ -x "$dir/lintel" ] || fail
  cat >/dev/null
else
  tmp=$(mktemp "$dir/.upload.XXXXXX") || fail
  trap 'rm -f "$tmp"' EXIT HUP INT TERM
  cat > "$tmp" || fail
  [ "$(sha256sum "$tmp" | cut -d ' ' -f 1)" = '{sha}' ] || fail
  chmod 700 "$tmp" || fail
  ln "$tmp" "$dir/lintel" || fail
fi
printf '{{"ok":true,"data":{{"uploaded":true}}}}\n'
"#
        ))
    }
    fn verify_install(&self, alias: &str, path: &Path, mut record: Value) -> Result<Value> {
        let sha = checked_digest(&record["bundle"]["sha256"])?;
        let script = format!(
            r#"file="$HOME/.local/share/lintel/runners/{sha}/lintel"
if [ ! -L "$file" ] && [ -f "$file" ] && [ -x "$file" ] && [ "$(sha256sum "$file" | cut -d ' ' -f 1)" = '{sha}' ]; then
 printf '{{"ok":true,"data":{{"verified":true}}}}\n'
else
 printf '{{"ok":true,"data":{{"verified":false}}}}\n'
fi"#
        );
        let response = self.transport.wire(
            alias,
            &json!({"command":"verify_runner"}),
            false,
            &[],
            &shell_script(&script),
        )?;
        if response["data"]["verified"] != true {
            record["status"] = json!("not_installed");
            save(path, &record)?;
            return Ok(json!({"ok":true,"data":record}));
        }
        let discover = self.runner_call(
            alias,
            &json!({"command":"discover"}),
            false,
            Some(&record["bundle"]["sha256"]),
        )?;
        if discover["ok"] != true
            || !discover["data"]["capabilities"]
                .as_array()
                .is_some_and(|all| {
                    all.iter()
                        .any(|c| c["name"] == "detached_submission" && c["status"] == "available")
                })
        {
            record["status"] = json!("installed_unverified");
            save(path, &record)?;
            return Ok(json!({"ok":true,"data":record}));
        }
        if record["activated"] == true {
            record["status"] = json!("ready");
            save(path, &record)?;
            return Ok(json!({"ok":true,"data":record}));
        }
        let dir = self.state.join("bindings");
        private_dir(&dir)?;
        let _held = lock(&dir.join(format!("{alias}.lock")))?;
        save(
            &dir.join(format!("{alias}.json")),
            &json!({"digest":record["bundle"]["sha256"],"install_id":record["install_id"]}),
        )?;
        record["status"] = json!("ready");
        record["activated"] = json!(true);
        record["last_error"] = Value::Null;
        save(path, &record)?;
        Ok(json!({"ok":true,"data":record}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    fn fixture() -> (tempfile::TempDir, Controller) {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let home = root.join("home");
        fs::create_dir(&home).unwrap();
        let tools = root.join("tools");
        fs::create_dir(&tools).unwrap();
        // Linux command fixtures over a synthetic home; not Linux runtime evidence.
        fs::write(
            root.join("machine-id"),
            "0123456789abcdef0123456789abcdef\n",
        )
        .unwrap();
        for (name, body) in [
            (
                "uname",
                "case \"$1\" in -s) echo Linux;; -m) echo x86_64;; esac",
            ),
            (
                "stat",
                if cfg!(target_os = "macos") {
                    "/usr/bin/stat -f %u \"$3\""
                } else {
                    "/usr/bin/stat -c %u \"$3\""
                },
            ),
        ] {
            let path = tools.join(name);
            fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let ssh = root.join("ssh");
        fs::write(
            &ssh,
            format!(
                r#"#!/bin/sh
export HOME='{home}'
export PATH='{tools}':/usr/bin:/bin:/usr/sbin:/sbin
for arg do last="$arg"; done
printf '%s\n' "$last" >> '{root}/commands'
last=$(printf '%s' "$last" | sed 's|/etc/machine-id|{root}/machine-id|g')
if [ -f '{root}/lose-ack' ]; then
 case "$last" in *uploaded*) rm '{root}/lose-ack'; eval "$last" >/dev/null; exit 0;; esac
fi
eval "$last"
"#,
                home = home.display(),
                tools = tools.display(),
                root = root.display()
            ),
        )
        .unwrap();
        let c = Controller {
            terminal: None,
            state: root.join("state/remote"),
            config: home.join(".ssh/config"),
            bundles: root.join("bundles"),
            transport: Transport {
                ssh,
                deadline: Duration::from_secs(4),
                interpreter: Some("/bin/sh".into()),
            },
        };
        c.dispatch(json!({"op":"add_host","alias":"synthetic-host"}))
            .unwrap();
        write_bundle(&c, b"#!/bin/sh\ncat >/dev/null\nprintf '%s\\n' '{\"ok\":true,\"data\":{\"environments\":[],\"capabilities\":[{\"name\":\"detached_submission\",\"status\":\"available\"}]}}'\n");
        (temp, c)
    }
    fn write_bundle(c: &Controller, bytes: &[u8]) {
        let target = "x86_64-unknown-linux-musl";
        fs::create_dir_all(c.bundles.join(target)).unwrap();
        fs::write(c.bundles.join(target).join("lintel"), bytes).unwrap();
        fs::write(c.bundles.join("manifest.json"),serde_json::to_vec(&json!({"runners":[{"target":target,"sha256":digest(bytes),"bytes":bytes.len(),"version":"0.1.0","protocol":1}]})).unwrap()).unwrap();
    }
    fn prepare(c: &Controller) -> Value {
        c.dispatch(json!({"op":"prepare_runner","alias":"synthetic-host"}))
            .unwrap()["data"]
            .clone()
    }
    fn install(c: &Controller, p: &Value) -> Result<Value> {
        c.dispatch(json!({"op":"install_runner","alias":"synthetic-host","install_id":p["install_id"],"approval":p["approval"]}))
    }
    fn query(c: &Controller, p: &Value) -> Result<Value> {
        c.dispatch(
            json!({"op":"query_install","alias":"synthetic-host","install_id":p["install_id"]}),
        )
    }
    #[test]
    fn approval_and_fresh_target_and_artifact_precede_upload() {
        let (_t, c) = fixture();
        let p = prepare(&c);
        let mut bad = p.clone();
        bad["approval"] = json!("wrong");
        assert_eq!(install(&c, &bad).unwrap_err().code, "approval_required");
        assert!(c.binding("synthetic-host").unwrap().is_none());
        assert_eq!(query(&c, &p).unwrap()["data"]["status"], "previewed");
        write_bundle(&c, b"changed");
        assert_eq!(install(&c, &p).unwrap_err().code, "stale_install_bundle");
        let p = prepare(&c);
        fs::write(
            c.transport.ssh.parent().unwrap().join("machine-id"),
            "fedcba9876543210fedcba9876543210",
        )
        .unwrap();
        assert_eq!(install(&c, &p).unwrap_err().code, "stale_install_target");
    }
    #[test]
    fn upload_verifies_hash_and_permissions_then_handshakes_without_external_changes() {
        let (t, c) = fixture();
        let root = fs::canonicalize(t.path()).unwrap();
        let home = root.join("home");
        fs::create_dir(home.join(".claude")).unwrap();
        fs::write(home.join(".claude/settings.json"), "EXTERNAL").unwrap();
        fs::write(home.join(".profile"), "EXTERNAL").unwrap();
        let p = prepare(&c);
        let response = install(&c, &p).unwrap();
        assert_eq!(response["data"]["status"], "ready");
        let sha = field(&p["bundle"], "sha256").unwrap();
        let runner = home.join(format!(".local/share/lintel/runners/{sha}/lintel"));
        assert_eq!(digest(&fs::read(&runner).unwrap()), sha);
        assert_eq!(
            fs::metadata(runner).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::read_to_string(home.join(".profile")).unwrap(),
            "EXTERNAL"
        );
        assert_eq!(
            fs::read_to_string(home.join(".claude/settings.json")).unwrap(),
            "EXTERNAL"
        );
        let before = fs::read_to_string(root.join("commands"))
            .unwrap()
            .matches("uploaded")
            .count();
        install(&c, &p).unwrap();
        assert_eq!(
            fs::read_to_string(root.join("commands"))
                .unwrap()
                .matches("uploaded")
                .count(),
            before
        );
    }
    #[test]
    fn lost_ack_recovers_query_only_even_after_alias_removal() {
        let (t, c) = fixture();
        let root = fs::canonicalize(t.path()).unwrap();
        let p = prepare(&c);
        fs::write(root.join("lose-ack"), "").unwrap();
        assert!(install(&c, &p).is_err());
        assert!(c.binding("synthetic-host").unwrap().is_none());
        assert_eq!(
            c.install_inventory().unwrap().last().unwrap()["status"],
            "needs_reconciliation"
        );
        assert_eq!(
            c.dispatch(json!({"op":"prepare_runner","alias":"synthetic-host"}))
                .unwrap_err()
                .code,
            "install_reconciliation_required"
        );
        c.dispatch(json!({"op":"remove_host","alias":"synthetic-host"}))
            .unwrap();
        assert_eq!(query(&c, &p).unwrap()["data"]["status"], "ready");
        assert_eq!(
            fs::read_to_string(root.join("commands"))
                .unwrap()
                .matches("uploaded")
                .count(),
            1
        );
    }
    #[test]
    fn install_failure_never_activates_and_old_queries_never_switch_new_binding() {
        let (t, c) = fixture();
        let root = fs::canonicalize(t.path()).unwrap();
        let p = prepare(&c);
        let runner = root.join(format!(
            "home/.local/share/lintel/runners/{}/lintel",
            p["bundle"]["sha256"].as_str().unwrap()
        ));
        fs::create_dir_all(runner.parent().unwrap()).unwrap();
        fs::write(&runner, "EXTERNAL COLLISION").unwrap();
        assert_eq!(
            install(&c, &p).unwrap()["data"]["status"],
            "needs_reconciliation"
        );
        assert!(c.binding("synthetic-host").unwrap().is_none());
        assert_eq!(query(&c, &p).unwrap()["data"]["status"], "not_installed");
        assert_eq!(fs::read_to_string(runner).unwrap(), "EXTERNAL COLLISION");
        let (_t, c) = fixture();
        let p = prepare(&c);
        install(&c, &p).unwrap();
        save(
            &c.state.join("bindings/synthetic-host.json"),
            &json!({"digest":"a".repeat(64)}),
        )
        .unwrap();
        query(&c, &p).unwrap();
        assert_eq!(
            c.binding("synthetic-host").unwrap(),
            Some(json!("a".repeat(64)))
        );
    }
    #[test]
    fn unsupported_platform_and_missing_submission_capability_do_not_activate() {
        let (_t, c) = fixture();
        fs::write(
            c.transport.ssh.parent().unwrap().join("tools/uname"),
            "#!/bin/sh\necho Darwin\n",
        )
        .unwrap();
        assert_eq!(
            c.dispatch(json!({"op":"prepare_runner","alias":"synthetic-host"}))
                .unwrap_err()
                .code,
            "platform_unsupported"
        );
        let (_t, c) = fixture();
        write_bundle(&c,b"#!/bin/sh\ncat >/dev/null\nprintf '%s\\n' '{\"ok\":true,\"data\":{\"capabilities\":[]}}'\n");
        let p = prepare(&c);
        assert_eq!(
            install(&c, &p).unwrap()["data"]["status"],
            "installed_unverified"
        );
        assert!(c.binding("synthetic-host").unwrap().is_none());
    }
    #[test]
    fn finite_policy_schema_accepts_version_conditions_but_rejects_unmodeled_fields() {
        assert!(validate_request(&json!({"command":"inspect","environment_id":"00000000-0000-4000-8000-000000000001","trusted_devices":"not_required"})).is_ok());
        assert!(validate_request(&json!({"command":"plan_policy","environment_id":"00000000-0000-4000-8000-000000000001","preset":"reduce","keep_remote_control":true,"trusted_devices":"unknown","release_settings":["DISABLE_TELEMETRY"]})).is_ok());
        assert!(validate_request(
            &json!({"command":"inspect","environment_id":"00000000-0000-4000-8000-000000000001","trusted_devices":"maybe"})
        )
        .is_err());
        assert!(validate_request(&json!({"command":"plan_policy","environment_id":"00000000-0000-4000-8000-000000000001","preset":"reduce","keep_remote_control":true,"release_settings":["UNMODELED_ENV"]})).is_err());
    }
    #[test]
    fn task_queries_use_original_digest_after_update() {
        let (_t, c) = fixture();
        let p = prepare(&c);
        install(&c, &p).unwrap();
        let (path, held) = c
            .record("synthetic-host", "00000000-0000-4000-8000-000000000007")
            .unwrap();
        save(&path,&json!({"plan_id":"00000000-0000-4000-8000-000000000007","lookup_id":"00000000-0000-4000-8000-000000000007","status":"submission_unknown","runner_digest":p["bundle"]["sha256"]})).unwrap();
        save(
            &c.state.join("bindings/synthetic-host.json"),
            &json!({"digest":"b".repeat(64)}),
        )
        .unwrap();
        drop(held);
        let result =
            c.dispatch(json!({"op":"reconnect","alias":"synthetic-host","plan_id":"00000000-0000-4000-8000-000000000007"}));
        assert!(result.is_err()); // inert fixture's discover is not a matching receipt
        let commands =
            fs::read_to_string(c.transport.ssh.parent().unwrap().join("commands")).unwrap();
        assert!(commands
            .lines()
            .last()
            .unwrap()
            .contains(p["bundle"]["sha256"].as_str().unwrap()));
        assert!(!commands.lines().last().unwrap().contains(&"b".repeat(64)));
    }
}

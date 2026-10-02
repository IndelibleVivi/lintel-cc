//! Native OpenSSH controller. Only finite runner operations cross this boundary.
use fs2::FileExt;
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::{
            fs::{DirBuilderExt, OpenOptionsExt},
            process::CommandExt,
        },
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const MAX_JSON: usize = 2 * 1024 * 1024;
#[derive(Debug)]
struct Failure {
    code: &'static str,
    message: &'static str,
}
type Result<T> = std::result::Result<T, Failure>;
fn failure(code: &'static str, message: &'static str) -> Failure {
    Failure { code, message }
}
fn storage(_: std::io::Error) -> Failure {
    failure(
        "local_state_unavailable",
        "无法读写本地远程任务记录；没有据此重发任何操作",
    )
}
fn envelope(result: Result<Value>) -> Value {
    result.unwrap_or_else(|e| json!({"ok":false,"error":{"code":e.code,"message":e.message}}))
}
fn valid_alias(alias: &str) -> Result<&str> {
    if alias.len() > 128
        || !alias
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !alias
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || b"._-".contains(&v))
    {
        return Err(failure(
            "invalid_alias",
            "请选择 literal SSH alias；不接受选项、user@host、通配符或 shell 语法",
        ));
    }
    Ok(alias)
}
fn valid_id(id: &str) -> Result<&str> {
    if id.len() > 160
        || !id.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
        || !id
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || b"_-".contains(&v))
    {
        return Err(failure("invalid_id", "远端 plan/job 标识无效"));
    }
    Ok(id)
}
fn field<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| failure("invalid_request", "请求缺少必需的文本字段"))
}
fn exact_fields(value: &Value, required: &[&str], optional: &[&str]) -> Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| failure("invalid_request", "请求必须是 JSON object"))?;
    if required.iter().any(|key| !object.contains_key(*key))
        || object
            .keys()
            .any(|key| !required.contains(&key.as_str()) && !optional.contains(&key.as_str()))
    {
        return Err(failure("invalid_request", "请求字段与此操作不匹配"));
    }
    Ok(())
}

// Import is deliberately static: no ssh -G, Include expansion or Match exec.
fn alias_words(line: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    for ch in line.chars() {
        if escaped {
            word.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' && quote != Some('\'') {
            escaped = true;
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            } else {
                word.push(ch);
            }
        } else if ch == '\'' || ch == '"' {
            quote = Some(ch);
        } else if ch == '#' {
            break;
        } else if ch.is_whitespace() || ch == '=' {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            word.push(ch);
        }
    }
    if escaped || quote.is_some() {
        return None;
    }
    if !word.is_empty() {
        words.push(word);
    }
    Some(words)
}
fn list_aliases(path: &Path) -> Result<Value> {
    let raw = match File::open(path) {
        Ok(file) => {
            let mut raw = Vec::new();
            file.take(MAX_JSON as u64 + 1)
                .read_to_end(&mut raw)
                .map_err(|_| failure("config_unreadable", "无法读取 SSH alias 列表"))?;
            raw
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(_) => return Err(failure("config_unreadable", "无法读取 SSH alias 列表")),
    };
    if raw.len() > MAX_JSON {
        return Err(failure("config_too_large", "SSH 配置超过静态读取上限"));
    }
    let text = std::str::from_utf8(&raw)
        .map_err(|_| failure("config_unreadable", "SSH alias 配置不是 UTF-8"))?;
    let mut aliases = BTreeSet::new();
    let mut ignored = BTreeSet::new();
    for line in text.lines() {
        let Some(words) = alias_words(line) else {
            ignored.insert("unparsed_line");
            continue;
        };
        let Some(key) = words.first() else {
            continue;
        };
        match key.to_ascii_lowercase().as_str() {
            "host" => {
                for alias in words.iter().skip(1) {
                    if valid_alias(alias).is_ok() {
                        aliases.insert(alias.clone());
                    } else {
                        ignored.insert("Host patterns");
                    }
                }
            }
            "include" => {
                ignored.insert("Include");
            }
            "match" => {
                ignored.insert("Match");
            }
            _ => {}
        }
    }
    Ok(
        json!({"ok":true,"data":{"aliases":aliases,"ignored":ignored,"coverage":"只列出当前文件的 literal Host；不解析 Include、Match 或通配符，也不执行配置"}}),
    )
}

struct Transport {
    ssh: PathBuf,
    deadline: Duration,
    #[cfg(test)]
    interpreter: Option<PathBuf>,
}
impl Default for Transport {
    fn default() -> Self {
        Self {
            ssh: "/usr/bin/ssh".into(),
            deadline: Duration::from_secs(60),
            #[cfg(test)]
            interpreter: None,
        }
    }
}
impl Transport {
    fn call(&self, alias: &str, payload: &Value, submit: bool) -> Result<Value> {
        valid_alias(alias)?;
        let mut encoded = serde_json::to_vec(payload)
            .map_err(|_| failure("invalid_request", "请求不能编码为 JSON"))?;
        encoded.push(b'\n');
        if encoded.len() > MAX_JSON {
            return Err(failure("request_too_large", "请求超过 SSH transport 上限"));
        }
        let mut command = Command::new(&self.ssh);
        #[cfg(test)]
        if let Some(interpreter) = &self.interpreter {
            // The fixture is data read by the installed shell. Direct execution
            // of newly created scripts can stall before their first instruction
            // on macOS; that startup latency is unrelated to SSH pipe behavior.
            command = Command::new(interpreter);
            command.arg(&self.ssh);
        }
        let mut child = command
            .args([
                "-T",
                "-oBatchMode=yes",
                "-oStrictHostKeyChecking=yes",
                "-oUpdateHostKeys=no",
                "-oPermitLocalCommand=no",
                "-oClearAllForwardings=yes",
                "-oRequestTTY=no",
                "-oConnectTimeout=10",
                "-oServerAliveInterval=15",
                "-oServerAliveCountMax=2",
                alias,
                "lintel",
                if submit { "submit" } else { "request" },
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .map_err(|_| failure("ssh_unavailable", "无法启动系统 OpenSSH"))?;
        let result = (|| {
            let mut input = child.stdin.take();
            let mut output = child.stdout.take().expect("piped stdout");
            for fd in [input.as_ref().unwrap().as_raw_fd(), output.as_raw_fd()] {
                // Nonblocking pipes make stalled input, hostile output and the deadline bounded.
                let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
                if flags < 0
                    || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
                {
                    return Err(failure("transport_unknown", "SSH 管道不可用；请查询原任务"));
                }
            }
            let started = Instant::now();
            let mut sent = 0;
            let mut bytes = Vec::new();
            let mut eof = false;
            loop {
                if started.elapsed() >= self.deadline {
                    return Err(failure(
                        "transport_unknown",
                        "SSH 请求超时；请重连查询原任务，不要重建清理任务",
                    ));
                }
                if let Some(stdin) = input.as_mut() {
                    match stdin.write(&encoded[sent..]) {
                        Ok(0) => input = None,
                        Ok(count) => {
                            sent += count;
                            if sent == encoded.len() {
                                input = None;
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => input = None,
                        Err(_) => {
                            return Err(failure(
                                "transport_unknown",
                                "SSH 请求写入中断；请查询原任务",
                            ))
                        }
                    }
                }
                if !eof {
                    let mut buffer = [0; 65536];
                    loop {
                        match output.read(&mut buffer) {
                            Ok(0) => {
                                eof = true;
                                break;
                            }
                            Ok(count) => {
                                bytes.extend_from_slice(&buffer[..count]);
                                if bytes.len() > MAX_JSON {
                                    return Err(failure(
                                        "transport_unknown",
                                        "远端响应超过上限；请查询原任务",
                                    ));
                                }
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                            Err(_) => {
                                return Err(failure(
                                    "transport_unknown",
                                    "SSH 响应读取中断；请查询原任务",
                                ))
                            }
                        }
                    }
                }
                if let Some(status) = child
                    .try_wait()
                    .map_err(|_| failure("transport_unknown", "SSH 状态未知；请查询原任务"))?
                {
                    if eof {
                        if status.code() == Some(255) {
                            return Err(failure("transport_unknown", "SSH 连接失败；请核验主机身份与连接后查询原任务。未自动接受或修改 host key"));
                        }
                        let response: Value = serde_json::from_slice(&bytes).map_err(|_| {
                            failure("transport_unknown", "未收到完整 runner 响应；请查询原任务")
                        })?;
                        if response["ok"].as_bool().is_none()
                            || (response["ok"] == true && response.get("data").is_none())
                            || (response["ok"] == false
                                && (response["error"]["code"].as_str().is_none()
                                    || response["error"]["message"].as_str().is_none()))
                        {
                            return Err(failure(
                                "transport_unknown",
                                "runner response envelope 无效；请查询原任务",
                            ));
                        }
                        if !status.success() && response["ok"] == true {
                            return Err(failure(
                                "transport_unknown",
                                "runner 异常退出；请查询原任务",
                            ));
                        }
                        return Ok(response);
                    }
                }
                let mut polls = [
                    libc::pollfd {
                        fd: input.as_ref().map_or(-1, AsRawFd::as_raw_fd),
                        events: libc::POLLOUT,
                        revents: 0,
                    },
                    libc::pollfd {
                        fd: if eof { -1 } else { output.as_raw_fd() },
                        events: libc::POLLIN,
                        revents: 0,
                    },
                ];
                let wait = self
                    .deadline
                    .saturating_sub(started.elapsed())
                    .as_millis()
                    .min(50) as i32;
                unsafe {
                    libc::poll(polls.as_mut_ptr(), polls.len() as libc::nfds_t, wait);
                }
            }
        })();
        if result.is_err() {
            // Only our isolated SSH process group, never another terminal/session.
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.wait();
        }
        result
    }
}

fn private_dir(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() && !m.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(failure(
            "local_state_invalid",
            "本地记录目录不能是文件或符号链接",
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let parent = path
                .parent()
                .ok_or_else(|| failure("local_state_invalid", "本地记录目录无效"))?;
            private_dir(parent)?;
            match fs::DirBuilder::new().mode(0o700).create(path) {
                Ok(()) => File::open(parent)
                    .and_then(|f| f.sync_all())
                    .map_err(storage),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => private_dir(path),
                Err(e) => Err(storage(e)),
            }
        }
        Err(e) => Err(storage(e)),
    }
}
fn lock(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(storage)?;
    file.lock_exclusive().map_err(storage)?;
    Ok(file)
}
fn save(path: &Path, value: &Value) -> Result<()> {
    // Callers hold the stable neighbouring lock across atomic replacement.
    let pending = path.with_extension("pending");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&pending)
        .map_err(storage)?;
    file.write_all(&serde_json::to_vec(value).unwrap())
        .map_err(storage)?;
    file.sync_all().map_err(storage)?;
    fs::rename(&pending, path).map_err(storage)?;
    File::open(path.parent().unwrap())
        .and_then(|f| f.sync_all())
        .map_err(storage)
}
fn load(path: &Path) -> Result<Value> {
    let mut bytes = Vec::new();
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(storage)?
        .take(MAX_JSON as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(storage)?;
    if bytes.len() > MAX_JSON {
        return Err(failure(
            "local_record_invalid",
            "本地记录超过上限；未重发操作",
        ));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| failure("local_record_invalid", "本地记录损坏；未重发操作"))
}
fn validate_request(request: &Value) -> Result<()> {
    let (required, optional): (&[&str], &[&str]) = match field(request, "command")? {
        "discover" | "jobs" | "export_support" => (&["command"], &[]),
        "inspect"
        | "drift"
        | "accept_drift"
        | "cleanup_inspect"
        | "auth_probe"
        | "reactivate_environment" => (&["command", "environment_id"], &[]),
        "register" => (&["command", "name", "root"], &[]),
        "create_environment" => (&["command", "name"], &[]),
        "plan_policy" => (
            &["command", "environment_id", "preset", "keep_remote_control"],
            &[],
        ),
        "plan_reset" => (&["command", "environment_id", "recipe", "categories"], &[]),
        "plan_restore" | "job" => (&["command", "job_id"], &[]),
        "plan_cleanup" => (
            &[
                "command",
                "environment_id",
                "recipe",
                "writers_confirmed_stopped",
                "official_logout",
                "categories",
            ],
            &[],
        ),
        "archive_inspect" => (&["command", "job_id", "archive_passphrase"], &[]),
        "archive_read" => (&["command", "job_id", "archive_passphrase", "path"], &[]),
        "plan_import" => (
            &[
                "command",
                "environment_id",
                "job_id",
                "categories",
                "archive_passphrase",
            ],
            &[],
        ),
        _ => {
            return Err(failure(
                "unsupported_command",
                "此远程命令未受支持；变更执行必须使用独立 execute 操作",
            ))
        }
    };
    exact_fields(request, required, optional)?;
    for key in required.iter().filter(|key| {
        ![
            "keep_remote_control",
            "categories",
            "writers_confirmed_stopped",
            "official_logout",
        ]
        .contains(key)
    }) {
        if field(request, key)?.len() > 4096 {
            return Err(failure("invalid_request", "请求字段过长"));
        }
    }
    for key in [
        "keep_remote_control",
        "writers_confirmed_stopped",
        "official_logout",
    ] {
        if request.get(key).is_some_and(|value| !value.is_boolean()) {
            return Err(failure("invalid_request", "确认字段必须是 boolean"));
        }
    }
    if request.get("categories").is_some_and(|categories| {
        !categories.as_array().is_some_and(|values| {
            values.iter().all(|value| {
                value
                    .as_str()
                    .is_some_and(|s| ["instructions", "memory", "sessions"].contains(&s))
            })
        })
    }) {
        return Err(failure("invalid_request", "工作内容类别无效"));
    }
    match request["command"].as_str().unwrap() {
        "plan_policy"
            if ![json!("preserve"), json!("reduce")].contains(&request["preset"])
                || !request["keep_remote_control"].is_boolean() =>
        {
            return Err(failure("invalid_request", "保护方案无效"))
        }
        "plan_reset" if request["recipe"] != "rebuild" => {
            return Err(failure("invalid_request", "重建配方无效"))
        }
        "plan_cleanup"
            if !request["recipe"].as_str().is_some_and(|recipe| {
                ["repair_login", "reset_client", "retire"].contains(&recipe)
            }) =>
        {
            return Err(failure("invalid_request", "清理配方无效"))
        }
        _ => {}
    }
    Ok(())
}

struct Controller {
    state: PathBuf,
    config: PathBuf,
    transport: Transport,
}
impl Controller {
    fn system() -> Result<Self> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| failure("home_missing", "找不到当前用户目录"))?;
        let state = std::env::var_os("LINTEL_STATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                if cfg!(target_os = "macos") {
                    home.join("Library/Application Support/Lintel")
                } else {
                    home.join(".local/share/lintel")
                }
            });
        Ok(Self {
            state: state.join("remote"),
            config: home.join(".ssh/config"),
            transport: Transport::default(),
        })
    }
    fn hosts(&self) -> Result<Value> {
        let path = self.state.join("hosts.json");
        if !path.exists() {
            return Ok(json!([]));
        }
        let hosts = load(&path)?;
        let valid = hosts.as_array().is_some_and(|hosts| {
            hosts.iter().all(|host| {
                host["alias"]
                    .as_str()
                    .is_some_and(|a| valid_alias(a).is_ok())
            })
        });
        if !valid {
            return Err(failure("local_record_invalid", "已登记主机记录无效"));
        }
        Ok(hosts)
    }
    fn record(&self, alias: &str, plan_id: &str) -> Result<(PathBuf, File)> {
        valid_alias(alias)?;
        valid_id(plan_id)?;
        let root = self.state.join("tasks").join(alias);
        private_dir(&root)?;
        let held = lock(&root.join(format!("{plan_id}.lock")))?;
        Ok((root.join(format!("{plan_id}.json")), held))
    }
    fn checked_record(&self, path: &Path, plan_id: &str) -> Result<Value> {
        let record = load(path)?;
        if record["plan_id"] != plan_id
            || record["status"].as_str().is_none()
            || !record["lookup_id"]
                .as_str()
                .is_some_and(|v| valid_id(v).is_ok())
        {
            return Err(failure(
                "local_record_invalid",
                "本地任务记录无效；未重发操作",
            ));
        }
        Ok(record)
    }
    fn observe(&self, path: &Path, record: &mut Value, response: Value) -> Result<Value> {
        if response["ok"] == true {
            let receipt = &response["data"];
            if receipt["plan_id"] != record["plan_id"]
                || !receipt["id"].as_str().is_some_and(|v| valid_id(v).is_ok())
                || receipt["status"].as_str().is_none()
            {
                return Err(failure(
                    "receipt_mismatch",
                    "远端 receipt 与原任务不匹配；执行保持禁用，请查询原任务",
                ));
            }
            record["lookup_id"] = receipt["id"].clone();
            record["status"] = receipt["status"].clone();
        } else {
            record["status"] = json!("response_received");
        }
        save(path, record)?;
        Ok(response)
    }
    fn query(&self, alias: &str, path: &Path, mut record: Value) -> Result<Value> {
        let response = self.transport.call(
            alias,
            &json!({"command":"job","job_id":record["lookup_id"]}),
            false,
        )?;
        if response["ok"] == false {
            return Err(failure(
                "reconciliation_required",
                "原任务尚无法核对；请检查远端 journal 与目标状态。本工具不会重新提交此计划",
            ));
        }
        self.observe(path, &mut record, response)
    }
    fn dispatch(&self, payload: Value) -> Result<Value> {
        let op = field(&payload, "op")?;
        if op == "aliases" {
            exact_fields(&payload, &["op"], &[])?;
            return list_aliases(&self.config);
        }
        private_dir(&self.state)?;
        if op == "hosts" {
            exact_fields(&payload, &["op"], &[])?;
            let hosts = self.hosts()?;
            let mut tasks = Vec::new();
            for host in hosts.as_array().unwrap() {
                let alias = host["alias"].as_str().unwrap();
                let root = self.state.join("tasks").join(alias);
                if !root.exists() {
                    continue;
                }
                for item in fs::read_dir(root).map_err(storage)? {
                    let path = item.map_err(storage)?.path();
                    if path.extension().is_some_and(|v| v == "json") {
                        let plan_id = path
                            .file_stem()
                            .and_then(|v| v.to_str())
                            .ok_or_else(|| failure("local_record_invalid", "任务记录标识无效"))?;
                        let mut record = self.checked_record(&path, valid_id(plan_id)?)?;
                        record["alias"] = json!(alias);
                        tasks.push(record);
                    }
                }
            }
            return Ok(json!({"ok":true,"data":{"hosts":hosts,"tasks":tasks}}));
        }
        let alias = valid_alias(field(&payload, "alias")?)?;
        if op == "add_host" {
            exact_fields(&payload, &["op", "alias"], &[])?;
            let _held = lock(&self.state.join("hosts.lock"))?;
            let mut hosts = self.hosts()?;
            if !hosts
                .as_array()
                .unwrap()
                .iter()
                .any(|host| host["alias"] == alias)
            {
                hosts.as_array_mut().unwrap().push(json!({"alias":alias}));
                save(&self.state.join("hosts.json"), &hosts)?;
            }
            return Ok(json!({"ok":true,"data":{"alias":alias}}));
        }
        if !self
            .hosts()?
            .as_array()
            .unwrap()
            .iter()
            .any(|host| host["alias"] == alias)
        {
            return Err(failure(
                "host_not_registered",
                "请先登记此 SSH alias 再连接",
            ));
        }
        match op {
            "connect" => {
                exact_fields(&payload, &["op", "alias"], &[])?;
                self.transport
                    .call(alias, &json!({"command":"discover"}), false)
            }
            "request" => {
                exact_fields(&payload, &["op", "alias", "request"], &[])?;
                validate_request(&payload["request"])?;
                self.transport.call(alias, &payload["request"], false)
            }
            "execute" | "reconnect" => {
                let required = if op == "execute" {
                    vec!["op", "alias", "plan_id", "approval"]
                } else {
                    vec!["op", "alias", "plan_id"]
                };
                exact_fields(
                    &payload,
                    &required,
                    if op == "execute" {
                        &["archive_passphrase"]
                    } else {
                        &[]
                    },
                )?;
                let plan_id = valid_id(field(&payload, "plan_id")?)?;
                let (path, _held) = self.record(alias, plan_id)?;
                if path.exists() {
                    return self.query(alias, &path, self.checked_record(&path, plan_id)?);
                }
                let mut record =
                    json!({"plan_id":plan_id,"lookup_id":plan_id,"status":"submission_unknown"});
                if op == "reconnect" {
                    record["status"] = json!("query_only");
                    save(&path, &record)?;
                    return self.query(alias, &path, record);
                }
                if field(&payload, "approval")?.len() > 512 {
                    return Err(failure(
                        "invalid_approval",
                        "请使用远端计划返回的准确 approval hash",
                    ));
                }
                let mut request =
                    json!({"command":"execute","plan_id":plan_id,"approval":payload["approval"]});
                if let Some(passphrase) = payload.get("archive_passphrase") {
                    if !passphrase.as_str().is_some_and(|v| v.chars().count() >= 12) {
                        return Err(failure(
                            "invalid_archive_passphrase",
                            "归档口令至少需要 12 个字符",
                        ));
                    }
                    request["archive_passphrase"] = passphrase.clone();
                }
                save(&path, &record)?; // Durable local intent precedes the one possible submission.
                let response = self.transport.call(alias, &request, true)?;
                self.observe(&path, &mut record, response)
            }
            _ => Err(failure("unsupported_operation", "不支持此远程操作")),
        }
    }
}

#[tauri::command]
pub async fn remote_request(payload: Value) -> Value {
    match tauri::async_runtime::spawn_blocking(move || {
        envelope(Controller::system().and_then(|controller| controller.dispatch(payload)))
    })
    .await
    {
        Ok(response) => response,
        Err(_) => envelope(Err(failure(
            "remote_bridge_failed",
            "远程桥接中断；请保留原任务 ID 并查询",
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    // This fake executable never starts OpenSSH. Paths, responses and homes are synthetic.
    fn fixture(body: &str) -> (TempDir, Controller) {
        let temp = tempfile::tempdir().unwrap();
        let script = temp.path().join("ssh");
        fs::write(
            &script,
            format!(
                "#!/bin/sh\ncd '{}'\nprintf '%s\\n' \"$@\" >> args\ncat > input\n{body}\n",
                temp.path().display()
            ),
        )
        .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
        let controller = Controller {
            state: temp.path().join("state/remote"),
            config: temp.path().join("home/.ssh/config"),
            transport: Transport {
                ssh: script,
                deadline: Duration::from_secs(2),
                interpreter: Some("/bin/sh".into()),
            },
        };
        controller
            .dispatch(json!({"op":"add_host","alias":"synthetic-host"}))
            .unwrap();
        (temp, controller)
    }
    const RECEIPT: &str = r#"printf '%s\n' '{"ok":true,"data":{"id":"plan-1","plan_id":"plan-1","status":"completed"}}'"#;

    #[test]
    fn static_aliases_do_not_execute_config_or_expose_contents() {
        let (temp, controller) = fixture(RECEIPT);
        fs::create_dir_all(controller.config.parent().unwrap()).unwrap();
        fs::write(&controller.config, format!("Host synthetic-one *.secret !secret\nHost=\"synthetic-two\" # comment\nHost 'synthetic-three'\nMatch exec \"touch {}/executed\"\nInclude secret-path\nIdentityFile SECRET_PRIVATE_KEY\nProxyCommand secret-command\n", temp.path().display())).unwrap();
        let value = controller.dispatch(json!({"op":"aliases"})).unwrap();
        assert_eq!(
            value["data"]["aliases"],
            json!(["synthetic-one", "synthetic-three", "synthetic-two"])
        );
        assert_eq!(
            value["data"]["ignored"],
            json!(["Host patterns", "Include", "Match"])
        );
        assert!(!value.to_string().contains("SECRET"));
        assert!(!temp.path().join("args").exists());
        assert!(!temp.path().join("executed").exists());
        assert_eq!(alias_words("Host 'unclosed"), None);
    }

    #[test]
    fn host_registration_persists_without_connection_and_rejects_shell_syntax() {
        let (temp, controller) = fixture(RECEIPT);
        controller
            .dispatch(json!({"op":"add_host","alias":"second-host"}))
            .unwrap();
        let reopened = Controller {
            state: controller.state.clone(),
            config: controller.config.clone(),
            transport: Transport::default(),
        };
        assert_eq!(
            reopened.dispatch(json!({"op":"hosts"})).unwrap()["data"]["hosts"],
            json!([{"alias":"synthetic-host"},{"alias":"second-host"}])
        );
        for alias in [
            "-oProxyCommand=bad",
            "user@host",
            "$(bad)",
            "host;bad",
            "x/y",
            "x y",
            "*.host",
            "",
        ] {
            assert!(controller
                .dispatch(json!({"op":"add_host","alias":alias}))
                .is_err());
        }
        assert!(!temp.path().join("args").exists());
        let mode = fs::metadata(controller.state.join("hosts.json"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn fixed_command_strict_host_key_and_stdin_data() {
        let (temp, controller) = fixture(
            "printf '%s\\n' '{\"ok\":true,\"data\":{\"environments\":[],\"capabilities\":[]}}'",
        );
        let response = controller.dispatch(json!({"op":"request","alias":"synthetic-host","request":{"command":"register","name":"$(touch not-executed)","root":"/synthetic/path; touch bad"}})).unwrap();
        assert_eq!(response["ok"], true);
        let args = fs::read_to_string(temp.path().join("args")).unwrap();
        assert!(args.contains("-oStrictHostKeyChecking=yes\n"));
        assert!(args.contains("-oUpdateHostKeys=no\n"));
        assert!(args.contains("-oPermitLocalCommand=no\n"));
        assert!(args.ends_with("synthetic-host\nlintel\nrequest\n"));
        assert!(!args.contains("touch"));
        let input: Value =
            serde_json::from_slice(&fs::read(temp.path().join("input")).unwrap()).unwrap();
        assert_eq!(input["root"], "/synthetic/path; touch bad");
        assert!(!temp.path().join("not-executed").exists());
        assert!(!temp.path().join("bad").exists());
    }

    #[test]
    fn execute_persists_intent_and_secret_only_crosses_stdin() {
        let (temp, controller) = fixture(&format!(
            "test -f state/remote/tasks/synthetic-host/plan-1.json || exit 7\n{RECEIPT}"
        ));
        let request = json!({"op":"execute","alias":"synthetic-host","plan_id":"plan-1","approval":"SECRET_APPROVAL","archive_passphrase":"SECRET_ARCHIVE_PASSPHRASE"});
        let response = controller.dispatch(request.clone()).unwrap();
        assert_eq!(response["data"]["status"], "completed");
        assert!(fs::read_to_string(temp.path().join("args"))
            .unwrap()
            .ends_with("lintel\nsubmit\n"));
        let input = fs::read_to_string(temp.path().join("input")).unwrap();
        assert!(input.contains("SECRET_ARCHIVE_PASSPHRASE"));
        let record =
            fs::read_to_string(controller.state.join("tasks/synthetic-host/plan-1.json")).unwrap();
        assert!(!record.contains("SECRET"));
        assert!(!record.contains("approval"));
        controller.dispatch(request).unwrap();
        let input = fs::read_to_string(temp.path().join("input")).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&input).unwrap(),
            json!({"command":"job","job_id":"plan-1"})
        );
        let tasks = controller.dispatch(json!({"op":"hosts"})).unwrap();
        assert_eq!(tasks["data"]["tasks"][0]["alias"], "synthetic-host");
        assert_eq!(tasks["data"]["tasks"][0]["status"], "completed");
    }

    #[test]
    fn ack_loss_and_reopen_query_same_job_without_resubmitting() {
        let (temp, controller) = fixture(
            r#"if test ! -f called; then touch called; printf 'private path/key/account' >&2; exit 255; fi
printf '%s\n' '{"ok":true,"data":{"id":"plan-1","plan_id":"plan-1","status":"completed"}}'"#,
        );
        let request =
            json!({"op":"execute","alias":"synthetic-host","plan_id":"plan-1","approval":"secret"});
        let error = envelope(controller.dispatch(request.clone()));
        assert_eq!(error["error"]["code"], "transport_unknown");
        assert!(!error.to_string().contains("private path"));
        assert_eq!(
            load(&controller.state.join("tasks/synthetic-host/plan-1.json")).unwrap()["status"],
            "submission_unknown"
        );
        let reopened = Controller {
            state: controller.state,
            config: controller.config,
            transport: controller.transport,
        };
        assert_eq!(
            reopened.dispatch(request).unwrap()["data"]["status"],
            "completed"
        );
        let args = fs::read_to_string(temp.path().join("args")).unwrap();
        assert_eq!(args.lines().filter(|v| *v == "submit").count(), 1);
        assert_eq!(args.lines().filter(|v| *v == "request").count(), 1);
        let input = fs::read_to_string(temp.path().join("input")).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&input).unwrap(),
            json!({"command":"job","job_id":"plan-1"})
        );
    }

    #[test]
    fn concurrent_submit_is_serialized_and_second_call_only_queries() {
        let (temp, controller) = fixture(RECEIPT);
        let other = Controller {
            state: controller.state.clone(),
            config: controller.config.clone(),
            transport: Transport {
                ssh: controller.transport.ssh.clone(),
                deadline: Duration::from_secs(2),
                interpreter: controller.transport.interpreter.clone(),
            },
        };
        let request =
            json!({"op":"execute","alias":"synthetic-host","plan_id":"plan-1","approval":"secret"});
        std::thread::scope(|scope| {
            let first = scope.spawn(|| controller.dispatch(request.clone()));
            let second = scope.spawn(|| other.dispatch(request.clone()));
            assert!(first.join().unwrap().is_ok());
            assert!(second.join().unwrap().is_ok());
        });
        let args = fs::read_to_string(temp.path().join("args")).unwrap();
        assert_eq!(args.lines().filter(|v| *v == "submit").count(), 1);
        assert_eq!(args.lines().filter(|v| *v == "request").count(), 1);
    }

    #[test]
    fn failed_runner_envelope_is_preserved_but_missing_job_never_replays() {
        let (temp, controller) = fixture(
            r#"printf '%s\n' '{"ok":false,"error":{"code":"job_not_found","message":"synthetic missing job"}}'; exit 1"#,
        );
        let response = controller.dispatch(json!({"op":"request","alias":"synthetic-host","request":{"command":"job","job_id":"plan-1"}})).unwrap();
        assert_eq!(response["error"]["code"], "job_not_found");
        let request = json!({"op":"reconnect","alias":"synthetic-host","plan_id":"plan-1"});
        assert_eq!(
            envelope(controller.dispatch(request))["error"]["code"],
            "reconciliation_required"
        );
        assert_eq!(envelope(controller.dispatch(json!({"op":"execute","alias":"synthetic-host","plan_id":"plan-1","approval":"secret"})))["error"]["code"], "reconciliation_required");
        assert!(!fs::read_to_string(temp.path().join("args"))
            .unwrap()
            .lines()
            .any(|v| v == "submit"));
    }

    #[test]
    fn receipt_identity_mismatch_never_changes_lookup_or_replays() {
        let (temp, controller) = fixture(
            r#"printf '%s\n' '{"ok":true,"data":{"id":"other-id","plan_id":"other-plan","status":"completed"}}'"#,
        );
        let request =
            json!({"op":"execute","alias":"synthetic-host","plan_id":"plan-1","approval":"secret"});
        assert_eq!(
            envelope(controller.dispatch(request.clone()))["error"]["code"],
            "receipt_mismatch"
        );
        assert_eq!(
            envelope(controller.dispatch(request))["error"]["code"],
            "receipt_mismatch"
        );
        assert_eq!(
            load(&controller.state.join("tasks/synthetic-host/plan-1.json")).unwrap()["lookup_id"],
            "plan-1"
        );
        assert_eq!(
            fs::read_to_string(temp.path().join("args"))
                .unwrap()
                .lines()
                .filter(|v| *v == "submit")
                .count(),
            1
        );
    }

    #[test]
    fn deadline_and_output_budget_are_enforced() {
        let (temp, mut controller) = fixture("sleep 5");
        controller.transport.deadline = Duration::from_millis(100);
        let started = Instant::now();
        let error = controller
            .transport
            .call("synthetic-host", &json!({"command":"discover"}), false)
            .unwrap_err();
        assert_eq!(error.code, "transport_unknown");
        assert!(error.message.contains("超时"));
        assert!(started.elapsed() < Duration::from_secs(2));
        drop(temp);
        let (_temp, controller) = fixture("head -c 2097153 /dev/zero");
        let error = controller
            .transport
            .call("synthetic-host", &json!({"command":"discover"}), false)
            .unwrap_err();
        assert_eq!(error.code, "transport_unknown");
        assert!(error.message.contains("超过上限"));
    }

    #[test]
    fn finite_schema_blocks_arbitrary_commands_and_mutation_bypass() {
        let (temp, controller) = fixture(RECEIPT);
        for request in [
            json!({"command":"execute","plan_id":"x","approval":"x"}),
            json!({"command":"shell","argv":["bad"]}),
            json!({"command":"discover","shell":"bad"}),
            json!({"command":"plan_policy","environment_id":"x","preset":"reduce","keep_remote_control":"false"}),
        ] {
            assert!(controller
                .dispatch(json!({"op":"request","alias":"synthetic-host","request":request}))
                .is_err());
        }
        assert!(!temp.path().join("args").exists());
    }
    #[test]
    fn cleanup_and_archive_payloads_keep_data_on_stdin_without_persistence() {
        let (temp, controller) = fixture("printf '%s\\n' '{\"ok\":true,\"data\":{}}'");
        for request in [
            json!({"command":"plan_cleanup","environment_id":"env-1","recipe":"reset_client","writers_confirmed_stopped":true,"official_logout":false,"categories":["memory"]}),
            json!({"command":"archive_inspect","job_id":"plan-1","archive_passphrase":"SECRET_SYNTHETIC_PHRASE"}),
            json!({"command":"archive_read","job_id":"plan-1","archive_passphrase":"SECRET_SYNTHETIC_PHRASE","path":"memory/notes.md"}),
            json!({"command":"plan_import","environment_id":"env-1","job_id":"plan-1","categories":["memory"],"archive_passphrase":"SECRET_SYNTHETIC_PHRASE"}),
        ] {
            assert!(controller
                .dispatch(json!({"op":"request","alias":"synthetic-host","request":request}))
                .unwrap()["ok"]
                .as_bool()
                .unwrap());
        }
        assert!(!fs::read_to_string(temp.path().join("args"))
            .unwrap()
            .contains("SECRET"));
        assert!(!controller.state.join("tasks").exists());
    }
}

//! Shared OpenSSH controller for the desktop App and headless CLI.
//! Only finite runner operations cross this boundary. Mutation submission and
//! runner installation persist local intent before their sole transport attempt;
//! retries reconcile the original record instead of submitting again.
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

mod catalog;
pub use catalog::operation_catalog;

#[cfg(test)]
#[path = "remote_acceptance.rs"]
mod acceptance;
#[path = "remote_install.rs"]
mod installation;
#[path = "remote_launch.rs"]
mod launching;

// Both JSON transport and interactive launch keep the same SSH trust boundary.
fn ssh_options(interactive: bool) -> Vec<&'static str> {
    vec![
        if interactive { "-tt" } else { "-T" },
        "-oBatchMode=yes",
        "-oStrictHostKeyChecking=yes",
        "-oUpdateHostKeys=no",
        "-oPermitLocalCommand=no",
        "-oProxyCommand=none",
        "-oRemoteCommand=none",
        "-oClearAllForwardings=yes",
        if interactive {
            "-oRequestTTY=force"
        } else {
            "-oRequestTTY=no"
        },
        "-oConnectTimeout=10",
        "-oServerAliveInterval=15",
        "-oServerAliveCountMax=2",
    ]
}
const MAX_JSON: usize = 2 * 1024 * 1024;
#[derive(Debug)]
struct Failure {
    code: &'static str,
    message: String,
    diagnostic: Option<Value>,
}
type Result<T> = std::result::Result<T, Failure>;
fn failure(code: &'static str, message: &'static str) -> Failure {
    Failure {
        code,
        message: message.into(),
        diagnostic: None,
    }
}
fn storage(_: std::io::Error) -> Failure {
    failure(
        "local_state_unavailable",
        "无法读写本地远程任务记录；没有据此重发任何操作",
    )
}
fn envelope(result: Result<Value>) -> Value {
    result.unwrap_or_else(|e| {
        let mut response = json!({"ok":false,"error":{"code":e.code,"message":e.message}});
        if let Some(diagnostic) = e.diagnostic {
            response["error"]["diagnostic"] = diagnostic;
        }
        response
    })
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
fn valid_core_id(id: &str) -> Result<&str> {
    if !lintel_operations::valid_uuid(id) {
        return Err(failure(
            "invalid_request",
            "远端环境或计划 ID 需要 UUID 格式",
        ));
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

fn exact_operation_fields(value: &Value) -> Result<()> {
    let (required, optional) = catalog::operation_fields(field(value, "op")?)
        .ok_or_else(|| failure("unsupported_operation", "不支持此远程操作"))?;
    exact_fields(value, required, optional)
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
        json!({"ok":true,"data":{"aliases":aliases,"ignored":ignored,"coverage":"只列出当前文件的 literal Host；不解析 Include、Match 或通配符，静态导入不执行配置。连接固定关闭 ProxyCommand / RemoteCommand / PermitLocalCommand"}}),
    )
}

const MAX_STDERR: usize = 64 * 1024;
const MAX_EXCERPT: usize = 4096;
#[derive(Default)]
struct StderrCapture {
    bytes: Vec<u8>,
    truncated: bool,
}
impl StderrCapture {
    fn append(&mut self, bytes: &[u8]) {
        let keep = bytes.len().min(MAX_STDERR.saturating_sub(self.bytes.len()));
        self.bytes.extend_from_slice(&bytes[..keep]);
        self.truncated |= keep < bytes.len();
    }
    fn redacted(&self, payload: &Value) -> String {
        let text = clean_terminal(&String::from_utf8_lossy(&self.bytes));
        let mut values = Vec::new();
        request_strings(payload, &mut values);
        let mut tokens = BTreeSet::new();
        for value in values {
            tokens.insert(clean_terminal(value));
            let escaped = serde_json::to_string(value).unwrap();
            tokens.insert(clean_terminal(&escaped));
            if escaped.len() > 2 {
                tokens.insert(clean_terminal(&escaped[1..escaped.len() - 1]));
            }
        }
        // Match only the original text, never replacements. Short payload
        // values cannot recursively expand/redact the replacement marker.
        let mut ranges = Vec::new();
        for token in tokens.iter().filter(|value| !value.is_empty()) {
            ranges.extend(
                text.match_indices(token)
                    .map(|(start, matched)| (start, start + matched.len())),
            );
        }
        ranges.sort_unstable();
        let mut result = String::new();
        let mut cursor = 0;
        let mut index = 0;
        while index < ranges.len() {
            let (start, mut end) = ranges[index];
            index += 1;
            while index < ranges.len() && ranges[index].0 <= end {
                end = end.max(ranges[index].1);
                index += 1;
            }
            result.push_str(&text[cursor..start]);
            result.push_str("[request value redacted]");
            cursor = end;
        }
        result.push_str(&text[cursor..]);
        result
    }
}
fn request_strings<'a>(value: &'a Value, strings: &mut Vec<&'a str>) {
    match value {
        Value::String(value) => strings.push(value),
        Value::Array(values) => {
            for value in values {
                request_strings(value, strings);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                request_strings(value, strings);
            }
        }
        _ => {}
    }
}
// Strip terminal escape/control sequences before text is shown or classified.
fn clean_terminal(text: &str) -> String {
    let mut result = String::new();
    let mut mode = 0;
    for ch in text.chars() {
        match mode {
            1 => {
                mode = match ch {
                    '[' => 2,
                    ']' => 3,
                    _ => 0,
                };
            }
            2 => {
                if ('@'..='~').contains(&ch) {
                    mode = 0;
                }
            }
            3 => {
                if ch == '\u{7}' {
                    mode = 0;
                } else if ch == '\u{1b}' {
                    mode = 4;
                }
            }
            4 => {
                mode = if ch == '\\' { 0 } else { 3 };
            }
            _ => {
                if ch == '\u{1b}' {
                    mode = 1;
                } else if ch == '\u{9b}' {
                    mode = 2;
                } else if ch == '\u{9d}' {
                    mode = 3;
                } else if (!ch.is_control() || ch == '\n' || ch == '\t')
                    && !matches!(ch, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}' | '\u{2066}'..='\u{2069}' | '\u{feff}')
                {
                    result.push(ch);
                }
            }
        }
    }
    result
}
fn ssh_reason(stderr: &str) -> &'static str {
    let text = stderr.to_ascii_lowercase();
    if text.contains("remote host identification has changed")
        || text.contains("host key for") && text.contains("has changed")
    {
        "host_key_changed"
    } else if text.contains("no ") && text.contains("host key is known")
        || text.contains("authenticity of host")
    {
        "host_key_unknown"
    } else if text.contains("host key verification failed") {
        "host_key_verification_failed"
    } else if text.contains("could not resolve hostname")
        || text.contains("name or service not known")
        || text.contains("nodename nor servname")
    {
        "dns"
    } else if text.contains("connection timed out")
        || text.contains("operation timed out")
        || text.contains("connection timeout")
    {
        "timeout"
    } else if text.contains("connection refused") {
        "connection_refused"
    } else if text.contains("network is unreachable")
        || text.contains("no route to host")
        || text.contains("host is unreachable")
    {
        "network_unreachable"
    } else if text.contains("permission denied")
        || text.contains("authentication failed")
        || text.contains("too many authentication failures")
    {
        "authentication_failed"
    } else if text.contains("bad configuration option")
        || text.contains("bad configuration options")
        || text.contains("missing argument")
        || text.contains("bad port")
        || text.contains("bad owner or permissions on")
        || text.contains("unsupported option")
    {
        "config_invalid"
    } else {
        "ssh_unknown"
    }
}
fn diagnose(
    reason: &'static str,
    payload: &Value,
    submit: bool,
    exit_code: Option<i32>,
    stderr: &StderrCapture,
) -> Failure {
    let (stage, summary, steps): (&str, &str, &[&str]) = match reason {
        "ssh_unavailable" => ("local", "无法启动系统 OpenSSH。", &["检查系统 /usr/bin/ssh 是否可用及本机执行权限。"]),
        "pipe_error" => ("local", "本机 SSH 管道读取或写入失败。", &["检查本机进程与资源状态，再连接目标。"]),
        "dns" => ("ssh", "SSH 无法解析目标主机名。", &["检查 alias 的 HostName、DNS 与当前网络/VPN。"]),
        "timeout" => ("ssh", "SSH 连接超时。", &["检查目标地址、SSH 端口、防火墙和当前网络；核对主机是否在线。"]),
        "connection_refused" => ("ssh", "目标拒绝 SSH 连接。", &["核对 SSH 端口及远端 sshd 是否在监听。"]),
        "network_unreachable" => ("ssh", "当前网络无法到达 SSH 目标。", &["检查路由、VPN 和目标所在网络。"]),
        "authentication_failed" => ("ssh", "SSH 身份认证失败。", &["核对 alias 的 User、IdentityFile 和系统 ssh-agent 中的身份；本工具使用 BatchMode，不弹出密码输入。"]),
        "host_key_changed" => ("ssh", "SSH 主机密钥发生变化，连接已停止。", &["通过可信渠道核验主机指纹与变更原因，再自行处理 known_hosts；不要关闭 host key 检查。"]),
        "host_key_unknown" => ("ssh", "此 SSH 主机的身份尚未获得信任。", &["先在自己的 SSH 工具中通过可信渠道核验并确认主机指纹，再返回 Lintel 连接。"]),
        "host_key_verification_failed" => ("ssh", "SSH 主机身份验证失败。", &["核对 known_hosts 与可信主机指纹；当前输出不足以判断是首次连接还是密钥变化。"]),
        "config_invalid" => ("ssh", "OpenSSH 配置无法使用。", &["依据诊断片段检查用户 SSH 配置的选项、参数与访问权限。"]),
        "ssh_unknown" => ("ssh", "SSH 连接失败，当前输出不能确定具体原因。", &["在本机终端用同一 alias 检查 SSH 连接，并对照本次诊断片段。"]),
        "runner_missing" => ("runner", "SSH 已到达远端，但找不到 lintel runner。已安装 Claude Code 不代表已经安装 Lintel runner。", &["在目标用户的非交互 SSH PATH 中确认 lintel runner；请在此面板选择“检查并准备运行器”，预览后批准用户级安装。"]),
        "runner_not_executable" => ("runner", "远端找到了 lintel，但它无法执行。", &["检查 lintel 的执行权限、文件格式、架构及加载器/依赖；不要把 Claude Code 可执行文件当作 Lintel runner。"]),
        "abnormal_exit" => ("runner", "远端 runner 异常退出，未得到可信结果。", &["检查目标上的 lintel runner 版本、运行环境与本次退出码。"]),
        "runner_rejected" => ("runner", "远端 runner 返回了明确的请求错误。", &["按照原始错误 code/message 核对请求范围、计划与 runner 版本。"]),
        "invalid_json" => ("response", "远端输出不是完整的 Lintel JSON 响应。", &["确认非交互 shell 启动文件不会向 stdout 打印欢迎信息，并检查 lintel request 是否输出单个 JSON Envelope。"]),
        "protocol_invalid" => ("response", "远端 JSON 不符合 Lintel response Envelope。", &["核对两端 Lintel 协议/版本，确认远端 lintel 命令没有被其他程序占用。"]),
        "output_limit" => ("response", "远端响应超过上限。", &["缩小请求范围，并检查远端 runner 输出是否异常。"]),
        "deadline" => ("response", "远程请求超时，已超过控制端等待时限。", &["SSH 连接或远端处理尚未完成；检查网络及 runner 状态。"]),
        _ => ("response", "远程响应中断，结果尚未确认。", &["检查网络及 runner 状态。"]),
    };
    let query = payload["command"] == "job";
    let install = payload["command"] == "install_runner";
    let mutating = submit
        || matches!(
            payload["command"].as_str(),
            Some("register" | "create_environment" | "accept_drift" | "reactivate_environment")
        );
    let uncertain = mutating && reason != "ssh_unavailable";
    let continuation = if install {
        " 请保留原安装记录，使用“核对原安装”确认结果；不会重新上传。"
    } else if submit {
        " 请保留原任务 ID，重连后只查询原任务；不会自动重新提交。"
    } else if query {
        " 原任务状态仍待核对；连接恢复后继续查询同一任务。"
    } else if mutating {
        " 远端状态需核对；不要据此假定本次操作没有生效。"
    } else {
        " 请按诊断建议排查后再次连接或读取。"
    };
    let mut next_steps = steps.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    if install {
        next_steps.push("在主机面板核对同一个 install_id；不创建第二份上传计划。".into());
    } else if submit || query {
        next_steps
            .push("使用原 plan/job ID 查询；不要为重试而移除本地任务记录或创建第二份清理。".into());
    }
    let mut diagnostic = json!({"stage":stage,"reason":reason,"summary":summary,"next_steps":next_steps,"submission_uncertain":uncertain});
    if let Some(code) = exit_code {
        diagnostic["exit_code"] = json!(code);
    }
    let mut text = stderr.redacted(payload);
    let mut cut = text.len().min(MAX_EXCERPT);
    while !text.is_char_boundary(cut) {
        cut -= 1;
    }
    let truncated = stderr.truncated || cut < text.len();
    text.truncate(cut);
    // Authorization-bearing requests suppress the excerpt entirely: even a
    // partial or transformed echo of a long passphrase must not reach the UI.
    if payload.get("approval").is_none()
        && payload.get("archive_passphrase").is_none()
        && !text.trim().is_empty()
    {
        diagnostic["stderr_excerpt"] = json!(text);
    }
    diagnostic["stderr_truncated"] = json!(truncated);
    Failure {
        code: if reason == "ssh_unavailable" {
            "ssh_unavailable"
        } else {
            "transport_unknown"
        },
        message: format!("{summary}{continuation}"),
        diagnostic: Some(diagnostic),
    }
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
        let mut result = self.exchange(alias, payload, submit);
        // A diagnostic command is for a user's terminal, never a replay of the
        // failed operation. Literal alias validation precedes interpolation.
        let check = format!("/usr/bin/ssh -T -oBatchMode=yes -oStrictHostKeyChecking=yes -oUpdateHostKeys=no -oPermitLocalCommand=no -oProxyCommand=none -oRemoteCommand=none -oClearAllForwardings=yes -oRequestTTY=no -oConnectTimeout=10 -oServerAliveInterval=15 -oServerAliveCountMax=2 {alias} 'command -v lintel'");
        let diagnostic = match &mut result {
            Err(error) => error.diagnostic.as_mut(),
            Ok(response) if response["ok"] == false => response["error"].get_mut("diagnostic"),
            _ => None,
        };
        if let Some(diagnostic) = diagnostic {
            diagnostic["command"] = json!(check);
        }
        result
    }
    fn exchange(&self, alias: &str, payload: &Value, submit: bool) -> Result<Value> {
        let mut encoded = serde_json::to_vec(payload)
            .map_err(|_| failure("invalid_request", "请求不能编码为 JSON"))?;
        encoded.push(b'\n');
        if encoded.len() > MAX_JSON {
            return Err(failure("request_too_large", "请求超过 SSH transport 上限"));
        }
        self.wire(
            alias,
            payload,
            submit,
            &encoded,
            &[
                "lintel".into(),
                if submit { "submit" } else { "request" }.into(),
            ],
        )
    }
    fn wire(
        &self,
        alias: &str,
        payload: &Value,
        submit: bool,
        encoded: &[u8],
        remote: &[String],
    ) -> Result<Value> {
        valid_alias(alias)?;
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
            .args(ssh_options(false))
            .arg(alias)
            .args(remote)
            .env("LC_ALL", "C")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .map_err(|_| {
                diagnose(
                    "ssh_unavailable",
                    payload,
                    submit,
                    None,
                    &StderrCapture::default(),
                )
            })?;
        let mut captured = StderrCapture::default();
        let result = (|| {
            let mut input = child.stdin.take();
            let mut output = child.stdout.take().expect("piped stdout");
            let mut errors = child.stderr.take().expect("piped stderr");
            for fd in [
                input.as_ref().unwrap().as_raw_fd(),
                output.as_raw_fd(),
                errors.as_raw_fd(),
            ] {
                let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
                if flags < 0
                    || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
                {
                    return Err(diagnose("pipe_error", payload, submit, None, &captured));
                }
            }
            let started = Instant::now();
            let mut sent = 0;
            let mut bytes = Vec::new();
            let mut eof = false;
            let mut stderr_eof = false;
            loop {
                if started.elapsed() >= self.deadline {
                    return Err(diagnose("deadline", payload, submit, None, &captured));
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
                        Err(e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                || e.kind() == std::io::ErrorKind::Interrupted => {}
                        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => input = None,
                        Err(_) => {
                            return Err(diagnose("pipe_error", payload, submit, None, &captured))
                        }
                    }
                }
                // Drain both pipes even after the stderr retention budget is
                // exhausted. Limit each turn so continuous stderr cannot starve
                // stdin, stdout or the deadline.
                let mut buffer = [0; 65536];
                if !stderr_eof {
                    for _ in 0..8 {
                        match errors.read(&mut buffer) {
                            Ok(0) => {
                                stderr_eof = true;
                                break;
                            }
                            Ok(count) => captured.append(&buffer[..count]),
                            Err(e)
                                if e.kind() == std::io::ErrorKind::WouldBlock
                                    || e.kind() == std::io::ErrorKind::Interrupted =>
                            {
                                break
                            }
                            Err(_) => {
                                return Err(diagnose(
                                    "pipe_error",
                                    payload,
                                    submit,
                                    None,
                                    &captured,
                                ))
                            }
                        }
                    }
                }
                if !eof {
                    for _ in 0..8 {
                        match output.read(&mut buffer) {
                            Ok(0) => {
                                eof = true;
                                break;
                            }
                            Ok(count) => {
                                bytes.extend_from_slice(&buffer[..count]);
                                if bytes.len() > MAX_JSON {
                                    return Err(diagnose(
                                        "output_limit",
                                        payload,
                                        submit,
                                        None,
                                        &captured,
                                    ));
                                }
                            }
                            Err(e)
                                if e.kind() == std::io::ErrorKind::WouldBlock
                                    || e.kind() == std::io::ErrorKind::Interrupted =>
                            {
                                break
                            }
                            Err(_) => {
                                return Err(diagnose(
                                    "pipe_error",
                                    payload,
                                    submit,
                                    None,
                                    &captured,
                                ))
                            }
                        }
                    }
                }
                if let Some(status) = child
                    .try_wait()
                    .map_err(|_| diagnose("pipe_error", payload, submit, None, &captured))?
                {
                    if eof && stderr_eof {
                        let text = captured.redacted(payload).to_ascii_lowercase();
                        let reason = if status.code() == Some(255) {
                            Some(ssh_reason(&text))
                        } else if status.code() == Some(127)
                            && text.contains("lintel")
                            && (text.contains("not found") || text.contains("no such file"))
                        {
                            Some("runner_missing")
                        } else if status.code() == Some(126)
                            && text.contains("lintel")
                            && (text.contains("permission denied")
                                || text.contains("cannot execute")
                                || text.contains("exec format"))
                        {
                            Some("runner_not_executable")
                        } else {
                            None
                        };
                        if let Some(reason) = reason {
                            return Err(diagnose(
                                reason,
                                payload,
                                submit,
                                status.code(),
                                &captured,
                            ));
                        }
                        let mut response: Value = serde_json::from_slice(&bytes).map_err(|_| {
                            diagnose(
                                if status.success() {
                                    "invalid_json"
                                } else {
                                    "abnormal_exit"
                                },
                                payload,
                                submit,
                                status.code(),
                                &captured,
                            )
                        })?;
                        if response["ok"].as_bool().is_none()
                            || (response["ok"] == true && response.get("data").is_none())
                            || (response["ok"] == false
                                && (response["error"]["code"].as_str().is_none()
                                    || response["error"]["message"].as_str().is_none()))
                        {
                            return Err(diagnose(
                                "protocol_invalid",
                                payload,
                                submit,
                                status.code(),
                                &captured,
                            ));
                        }
                        if !status.success() && response["ok"] == true {
                            return Err(diagnose(
                                "abnormal_exit",
                                payload,
                                submit,
                                status.code(),
                                &captured,
                            ));
                        }
                        if response["ok"] == false {
                            response["error"]["diagnostic"] = diagnose(
                                "runner_rejected",
                                payload,
                                submit,
                                status.code(),
                                &captured,
                            )
                            .diagnostic
                            .unwrap();
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
                    libc::pollfd {
                        fd: if stderr_eof { -1 } else { errors.as_raw_fd() },
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
    use std::os::unix::fs::MetadataExt;
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {
            if m.uid() != unsafe { libc::geteuid() } {
                return Err(failure("local_state_invalid", "本地记录目录不属于当前用户"));
            }
            Ok(())
        }
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
// These runner operations may cross the JSON request transport. Execution and
// interactive launch retain their separate controller operations and safeguards.
const REQUEST_COMMANDS: &[&str] = &[
    "discover",
    "register",
    "create_environment",
    "inspect",
    "inspect_components",
    "work_preflight",
    "work_inventory",
    "drift",
    "accept_drift",
    "cleanup_inspect",
    "auth_probe",
    "reactivate_environment",
    "plan_policy",
    "plan_reset",
    "plan_archive",
    "plan_preserve",
    "plan_restore",
    "plan_show",
    "job",
    "jobs",
    "service_inspect",
    "plan_service_quiesce",
    "plan_service_resume",
    "plan_cleanup",
    "archive_inspect",
    "archive_read",
    "session_read",
    "plan_import",
    "plan_launch",
    "plan_resume",
    "launch_query",
    "launches",
    "export_support",
];
fn validate_request(request: &Value) -> Result<()> {
    if !REQUEST_COMMANDS.contains(&field(request, "command")?) {
        return Err(failure(
            "unsupported_command",
            "此远程命令未受支持；变更执行必须使用独立 execute 操作",
        ));
    }
    lintel_operations::validate(request).map_err(|message| Failure {
        code: "invalid_request",
        message,
        diagnostic: None,
    })?;
    if matches!(
        request["command"].as_str(),
        Some("service_inspect" | "plan_service_quiesce")
    ) {
        let unit = field(request, "unit")?;
        if unit.len() > 240
            || !unit.ends_with(".service")
            || unit.ends_with("@.service")
            || !unit
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphanumeric)
            || !unit
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_.@-".contains(&c))
        {
            return Err(failure(
                "invalid_request",
                "需要单一完整 service unit 名称；不接受 shell、路径或 glob",
            ));
        }
    }
    for value in request.as_object().unwrap().values() {
        if value
            .as_str()
            .is_some_and(|value| value.is_empty() || value.len() > 4096)
        {
            return Err(failure("invalid_request", "请求文本字段为空或过长"));
        }
    }
    Ok(())
}

struct Controller {
    state: PathBuf,
    config: PathBuf,
    transport: Transport,
    bundles: PathBuf,
    terminal: Option<PathBuf>,
}
impl Controller {
    fn system(bundles: PathBuf) -> Result<Self> {
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
            terminal: cfg!(target_os = "macos").then(|| PathBuf::from("/usr/bin/open")),
            bundles,
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
                .is_some_and(lintel_operations::valid_uuid)
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
                || !receipt["id"]
                    .as_str()
                    .is_some_and(lintel_operations::valid_uuid)
                || receipt["status"].as_str().is_none()
            {
                return Err(failure(
                    "receipt_mismatch",
                    "远端 receipt 与原任务不匹配；执行保持禁用，请查询原任务",
                ));
            }
            // A receipt only redirects later queries when the remote echoes the
            // original plan id; anything else keeps lookup_id pinned locally.
            if receipt["id"] == record["plan_id"] {
                record["lookup_id"] = receipt["id"].clone();
            }
            record["status"] = receipt["status"].clone();
        } else {
            record["status"] = json!("response_received");
        }
        save(path, record)?;
        Ok(response)
    }
    fn query(&self, alias: &str, path: &Path, mut record: Value) -> Result<Value> {
        let response = self.runner_call(
            alias,
            &json!({"command":"job","job_id":record["lookup_id"]}),
            false,
            record.get("runner_digest"),
        )?;
        if response["ok"] == false {
            let mut error = failure("reconciliation_required", "原任务尚无法核对");
            error.message = format!("原任务尚无法核对。远端错误 {}：{}。请检查远端 journal 与目标状态；本工具不会重新提交此计划。", response["error"]["code"].as_str().unwrap_or("unknown"), response["error"]["message"].as_str().unwrap_or("未提供具体错误"));
            error.diagnostic = response["error"].get("diagnostic").cloned();
            return Err(error);
        }
        self.observe(path, &mut record, response)
    }
    fn dispatch(&self, payload: Value) -> Result<Value> {
        let op = field(&payload, "op")?;
        if op == "execute" || op == "reconnect" {
            valid_core_id(field(&payload, "plan_id")?)?;
        } else if op == "launch" {
            valid_core_id(field(&payload, "environment_id")?)?;
        } else if op == "launch_request" || op == "resume_request" {
            valid_core_id(field(&payload, "request_id")?)?;
        }
        if op == "aliases" {
            exact_operation_fields(&payload)?;
            return list_aliases(&self.config);
        }
        private_dir(&self.state)?;
        if op == "hosts" {
            exact_operation_fields(&payload)?;
            let hosts = self.hosts()?;
            let mut tasks = Vec::new();
            let task_root = self.state.join("tasks");
            if task_root.exists() {
                for host in fs::read_dir(&task_root).map_err(storage)? {
                    let host = host.map_err(storage)?;
                    if !host.file_type().map_err(storage)?.is_dir() {
                        continue;
                    }
                    // A stray or non-conforming entry must not break the
                    // whole inventory; skip it and list what is readable.
                    let alias = host
                        .file_name()
                        .into_string()
                        .ok()
                        .filter(|name| valid_alias(name).is_ok());
                    let Some(alias) = alias else { continue };
                    for item in fs::read_dir(host.path()).map_err(storage)? {
                        let path = item.map_err(storage)?.path();
                        if path.extension().is_some_and(|v| v == "json") {
                            let Ok(plan_id) = path
                                .file_stem()
                                .and_then(|v| v.to_str())
                                .ok_or_else(|| failure("local_record_invalid", "任务记录标识无效"))
                                .and_then(|stem| valid_id(stem))
                            else {
                                continue;
                            };
                            let Ok(mut record) = self.checked_record(&path, plan_id) else {
                                continue;
                            };
                            record["alias"] = json!(alias);
                            tasks.push(record);
                        }
                    }
                }
            }
            let installs = self.install_inventory()?;
            let launches = self.launch_inventory(None)?;
            return Ok(
                json!({"ok":true,"data":{"hosts":hosts,"tasks":tasks,"installations":installs,"launches":launches}}),
            );
        }
        let alias = valid_alias(field(&payload, "alias")?)?;
        if op == "add_host" {
            exact_operation_fields(&payload)?;
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
        if op == "remove_host" {
            exact_operation_fields(&payload)?;
            let _held = lock(&self.state.join("hosts.lock"))?;
            let mut hosts = self.hosts()?;
            let entries = hosts.as_array_mut().unwrap();
            let before = entries.len();
            entries.retain(|host| host["alias"] != alias);
            let removed = before != entries.len();
            if removed {
                save(&self.state.join("hosts.json"), &hosts)?;
            }
            return Ok(json!({"ok":true,"data":{"alias":alias,"removed":removed}}));
        }
        let registered = self
            .hosts()?
            .as_array()
            .unwrap()
            .iter()
            .any(|host| host["alias"] == alias);
        if !registered {
            // Removing a display entry cannot erase or strand a durable task.
            let task_id = match op {
                "reconnect" => payload["plan_id"].as_str(),
                "request" if payload["request"]["command"] == "job" => payload["request"]["job_id"]
                    .as_str()
                    .or_else(|| payload["request"]["plan_id"].as_str()),
                _ => None,
            };
            let existing_query = (op == "query_install"
                && self.install_record_exists(alias, &payload))
                || task_id.is_some_and(|id| {
                    valid_id(id).is_ok()
                        && self
                            .state
                            .join("tasks")
                            .join(alias)
                            .join(format!("{id}.json"))
                            .exists()
                })
                // A frozen launch/resume record stays queryable after the alias
                // is removed, using its pinned runner.
                || (op == "launch_query"
                    && valid_id(payload["request_id"].as_str().unwrap_or("")).is_ok()
                    && self
                        .state
                        .join("launches")
                        .join(alias)
                        .join(format!("{}.json", payload["request_id"].as_str().unwrap_or("")))
                        .exists())
                || (op == "launches" && self.state.join("launches").join(alias).is_dir())
                || (matches!(op, "launch_request" | "resume_request")
                    && valid_core_id(payload["request_id"].as_str().unwrap_or("")).is_ok()
                    && load(&self.state.join("launches").join(alias)
                        .join(format!("{}.json", payload["request_id"].as_str().unwrap_or(""))))
                        .is_ok_and(|record| record["attempted"] == true));
            if !existing_query {
                return Err(failure(
                    "host_not_registered",
                    "请先登记此 SSH alias 再连接；已保留的任务仍可使用原任务查询",
                ));
            }
        }
        match op {
            "launch" => self.launch_remote(alias, &payload),
            "launch_request" => self.launch_request_remote(alias, &payload),
            "resume_request" => self.resume_request_remote(alias, &payload),
            "launch_query" | "launches" => {
                exact_operation_fields(&payload)?;
                if op == "launch_query" {
                    return self.launch_query_response(alias, field(&payload, "request_id")?);
                }
                // Inventory is local metadata. Explicit queries use the original
                // pinned runner; listing never fans out SSH requests.
                Ok(json!({"ok":true,"data":{"launches":self.launch_inventory(Some(alias))?}}))
            }
            "prepare_runner" | "install_runner" | "query_install" => {
                self.install_dispatch(alias, &payload)
            }
            "connect" => {
                exact_operation_fields(&payload)?;
                self.runner_call(
                    alias,
                    &json!({"command":"discover"}),
                    false,
                    self.binding(alias)?.as_ref(),
                )
            }
            "request" => {
                exact_operation_fields(&payload)?;
                validate_request(&payload["request"])?;
                // A frozen launch/resume plan pins the current bound runner BEFORE
                // any launch attempt, so later query/repeat uses the original
                // binding even after the alias is removed.
                if matches!(
                    payload["request"]["command"].as_str(),
                    Some("plan_launch" | "plan_resume")
                ) {
                    let bound = self.binding(alias)?;
                    let response =
                        self.runner_call(alias, &payload["request"], false, bound.as_ref())?;
                    self.pin_launch_binding(alias, &response, bound.as_ref())?;
                    return Ok(response);
                }
                if payload["request"]["command"] == "job" {
                    let id = payload["request"]["job_id"]
                        .as_str()
                        .or_else(|| payload["request"]["plan_id"].as_str())
                        .unwrap(); // The shared schema requires exactly one UUID.
                    let path = self
                        .state
                        .join("tasks")
                        .join(alias)
                        .join(format!("{id}.json"));
                    if path.exists() {
                        let (path, _held) = self.record(alias, id)?;
                        return self.query(alias, &path, self.checked_record(&path, id)?);
                    }
                    // A read-only lookup may name an unsubmitted plan. Keep its
                    // submission slot unused; only execute/reconnect create intent.
                    return self.runner_call(
                        alias,
                        &payload["request"],
                        false,
                        self.binding(alias)?.as_ref(),
                    );
                }
                self.runner_call(
                    alias,
                    &payload["request"],
                    false,
                    self.binding(alias)?.as_ref(),
                )
            }
            "execute" | "reconnect" => {
                exact_operation_fields(&payload)?;
                let plan_id = field(&payload, "plan_id")?;
                let existing = self
                    .state
                    .join("tasks")
                    .join(alias)
                    .join(format!("{plan_id}.json"));
                if op == "execute"
                    && !existing.exists()
                    && !lintel_operations::valid_plan_hash(field(&payload, "approval")?)
                {
                    return Err(failure(
                        "invalid_approval",
                        "请使用原远端计划返回的 64 位小写 hex approval hash；尚未记录或提交任务",
                    ));
                }
                let (path, _held) = self.record(alias, plan_id)?;
                if path.exists() {
                    return self.query(alias, &path, self.checked_record(&path, plan_id)?);
                }
                let mut record =
                    json!({"plan_id":plan_id,"lookup_id":plan_id,"status":"submission_unknown"});
                if let Some(digest) = self.binding(alias)? {
                    record["runner_digest"] = digest;
                }
                if op == "reconnect" {
                    record["status"] = json!("query_only");
                    save(&path, &record)?;
                    return self.query(alias, &path, record);
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
                let response =
                    self.runner_call(alias, &request, true, record.get("runner_digest"))?;
                self.observe(&path, &mut record, response)
            }
            _ => Err(failure("unsupported_operation", "不支持此远程操作")),
        }
    }
}

/// Run one finite SSH controller operation using the current user's HOME and
/// LINTEL_STATE_DIR. `bundles` names caller-supplied static Linux runner resources;
/// missing resources are an explicit installation limitation. No Tauri runtime
/// or browser storage is used by this crate.
pub fn control(payload: Value, bundles: PathBuf) -> Value {
    envelope(Controller::system(bundles).and_then(|controller| controller.dispatch(payload)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    // This fake executable never starts OpenSSH. Paths, responses and homes are synthetic.
    pub(super) fn fixture(body: &str) -> (TempDir, Controller) {
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
            terminal: Some(temp.path().join("open")),
            bundles: temp.path().join("bundles"),
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
    const APPROVAL: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const RECEIPT: &str = r#"printf '%s\n' '{"ok":true,"data":{"id":"00000000-0000-4000-8000-000000000002","plan_id":"00000000-0000-4000-8000-000000000002","status":"completed"}}'"#;

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
            terminal: controller.terminal.clone(),
            bundles: controller.bundles.clone(),
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
            "test -f state/remote/tasks/synthetic-host/00000000-0000-4000-8000-000000000002.json || exit 7\n{RECEIPT}"
        ));
        let request = json!({"op":"execute","alias":"synthetic-host","plan_id":"00000000-0000-4000-8000-000000000002","approval":APPROVAL,"archive_passphrase":"SECRET_ARCHIVE_PASSPHRASE"});
        let response = controller.dispatch(request.clone()).unwrap();
        assert_eq!(response["data"]["status"], "completed");
        assert!(fs::read_to_string(temp.path().join("args"))
            .unwrap()
            .ends_with("lintel\nsubmit\n"));
        let input = fs::read_to_string(temp.path().join("input")).unwrap();
        assert!(input.contains("SECRET_ARCHIVE_PASSPHRASE"));
        let record = fs::read_to_string(
            controller
                .state
                .join("tasks/synthetic-host/00000000-0000-4000-8000-000000000002.json"),
        )
        .unwrap();
        assert!(!record.contains("SECRET"));
        assert!(!record.contains("approval"));
        assert!(!record.contains(APPROVAL));
        controller.dispatch(request).unwrap();
        let input = fs::read_to_string(temp.path().join("input")).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&input).unwrap(),
            json!({"command":"job","job_id":"00000000-0000-4000-8000-000000000002"})
        );
        let tasks = controller.dispatch(json!({"op":"hosts"})).unwrap();
        assert_eq!(tasks["data"]["tasks"][0]["alias"], "synthetic-host");
        assert_eq!(tasks["data"]["tasks"][0]["status"], "completed");
    }

    #[test]
    fn ack_loss_and_reopen_query_same_job_without_resubmitting() {
        let (temp, controller) = fixture(
            r#"if test ! -f called; then touch called; printf 'private path/key/account' >&2; exit 255; fi
printf '%s\n' '{"ok":true,"data":{"id":"00000000-0000-4000-8000-000000000002","plan_id":"00000000-0000-4000-8000-000000000002","status":"completed"}}'"#,
        );
        let request = json!({"op":"execute","alias":"synthetic-host","plan_id":"00000000-0000-4000-8000-000000000002","approval":APPROVAL});
        let error = envelope(controller.dispatch(request.clone()));
        assert_eq!(error["error"]["code"], "transport_unknown");
        assert!(!error.to_string().contains("private path"));
        assert_eq!(
            load(
                &controller
                    .state
                    .join("tasks/synthetic-host/00000000-0000-4000-8000-000000000002.json")
            )
            .unwrap()["status"],
            "submission_unknown"
        );
        let reopened = Controller {
            terminal: controller.terminal.clone(),
            bundles: controller.bundles.clone(),
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
            json!({"command":"job","job_id":"00000000-0000-4000-8000-000000000002"})
        );
    }

    #[test]
    fn concurrent_submit_is_serialized_and_second_call_only_queries() {
        let (temp, controller) = fixture(RECEIPT);
        let other = Controller {
            terminal: controller.terminal.clone(),
            bundles: controller.bundles.clone(),
            state: controller.state.clone(),
            config: controller.config.clone(),
            transport: Transport {
                ssh: controller.transport.ssh.clone(),
                deadline: Duration::from_secs(2),
                interpreter: controller.transport.interpreter.clone(),
            },
        };
        let request = json!({"op":"execute","alias":"synthetic-host","plan_id":"00000000-0000-4000-8000-000000000002","approval":APPROVAL});
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
        let response = controller.dispatch(json!({"op":"request","alias":"synthetic-host","request":{"command":"job","job_id":"00000000-0000-4000-8000-000000000002"}})).unwrap();
        assert_eq!(response["error"]["code"], "job_not_found");
        let request = json!({"op":"reconnect","alias":"synthetic-host","plan_id":"00000000-0000-4000-8000-000000000002"});
        assert_eq!(
            envelope(controller.dispatch(request))["error"]["code"],
            "reconciliation_required"
        );
        assert_eq!(envelope(controller.dispatch(json!({"op":"execute","alias":"synthetic-host","plan_id":"00000000-0000-4000-8000-000000000002","approval":APPROVAL})))["error"]["code"], "reconciliation_required");
        assert!(!fs::read_to_string(temp.path().join("args"))
            .unwrap()
            .lines()
            .any(|v| v == "submit"));
    }

    #[test]
    fn malformed_new_approval_never_creates_task_or_invokes_ssh() {
        let (temp, controller) = fixture(RECEIPT);
        let id = "00000000-0000-4000-8000-000000000002";
        for approval in [
            "x".to_string(),
            "a".repeat(63),
            "a".repeat(65),
            "A".repeat(64),
            "g".repeat(64),
        ] {
            let result = controller.dispatch(
                json!({"op":"execute","alias":"synthetic-host","plan_id":id,"approval":approval}),
            );
            assert_eq!(result.unwrap_err().code, "invalid_approval");
            assert!(!controller.state.join("tasks").exists());
            assert!(!temp.path().join("args").exists());
        }
        let request =
            json!({"op":"execute","alias":"synthetic-host","plan_id":id,"approval":"a".repeat(64)});
        assert_eq!(
            controller.dispatch(request).unwrap()["data"]["status"],
            "completed"
        );
        assert_eq!(
            controller
                .dispatch(
                    json!({"op":"execute","alias":"synthetic-host","plan_id":id,"approval":"x"})
                )
                .unwrap()["data"]["id"],
            id
        );
        assert_eq!(
            fs::read_to_string(temp.path().join("args"))
                .unwrap()
                .lines()
                .filter(|arg| *arg == "submit")
                .count(),
            1
        );
    }

    #[test]
    fn new_reconnect_pins_current_runner_before_query_and_keeps_it_after_upgrade() {
        let (temp, controller) = fixture(RECEIPT);
        let original = json!("a".repeat(64));
        let binding = controller.state.join("bindings/synthetic-host.json");
        private_dir(binding.parent().unwrap()).unwrap();
        save(&binding, &json!({"digest":original})).unwrap();
        let id = "00000000-0000-4000-8000-000000000002";
        let request = json!({"op":"reconnect","alias":"synthetic-host","plan_id":id});
        assert_eq!(
            controller.dispatch(request.clone()).unwrap()["data"]["id"],
            id
        );
        let task = controller
            .state
            .join("tasks/synthetic-host")
            .join(format!("{id}.json"));
        assert_eq!(load(&task).unwrap()["runner_digest"], original);
        save(&binding, &json!({"digest":"b".repeat(64)})).unwrap();
        controller.dispatch(request).unwrap();
        controller.dispatch(json!({"op":"request","alias":"synthetic-host","request":{"command":"job","job_id":id}})).unwrap();
        let args = fs::read_to_string(temp.path().join("args")).unwrap();
        assert_eq!(
            args.lines()
                .filter(|arg| arg.contains(original.as_str().unwrap()))
                .count(),
            3
        );
        assert!(!args.contains(&"b".repeat(64)));
        assert!(!args.lines().any(|arg| arg == "submit"));
    }

    #[test]
    fn unrecorded_job_lookup_does_not_consume_submission_slot() {
        let (temp, controller) = fixture(&format!(
            "if grep -q '\"command\":\"job\"' input && ! test -f submitted; then\nprintf '%s\\n' '{{\"ok\":false,\"error\":{{\"code\":\"job_not_found\",\"message\":\"synthetic unsubmitted plan\"}}}}'; exit 1\nfi\ntouch submitted\n{RECEIPT}"
        ));
        let id = "00000000-0000-4000-8000-000000000002";
        let path = controller
            .state
            .join("tasks/synthetic-host")
            .join(format!("{id}.json"));
        for field in ["job_id", "plan_id"] {
            let mut request = json!({"command":"job"});
            request[field] = json!(id);
            let response = controller
                .dispatch(json!({"op":"request","alias":"synthetic-host","request":request}))
                .unwrap();
            assert_eq!(response["error"]["code"], "job_not_found");
            assert!(
                !path.exists(),
                "Read-only lookup allocated a submission record"
            );
        }
        let execute =
            json!({"op":"execute","alias":"synthetic-host","plan_id":id,"approval":APPROVAL});
        assert_eq!(
            controller.dispatch(execute.clone()).unwrap()["data"]["status"],
            "completed"
        );
        assert!(path.exists());
        assert_eq!(controller.dispatch(execute).unwrap()["data"]["id"], id);
        assert_eq!(
            fs::read_to_string(temp.path().join("args"))
                .unwrap()
                .lines()
                .filter(|arg| *arg == "submit")
                .count(),
            1
        );
    }

    #[test]
    fn receipt_identity_mismatch_never_changes_lookup_or_replays() {
        let (temp, controller) = fixture(
            r#"printf '%s\n' '{"ok":true,"data":{"id":"other-id","plan_id":"other-plan","status":"completed"}}'"#,
        );
        let request = json!({"op":"execute","alias":"synthetic-host","plan_id":"00000000-0000-4000-8000-000000000002","approval":APPROVAL});
        assert_eq!(
            envelope(controller.dispatch(request.clone()))["error"]["code"],
            "receipt_mismatch"
        );
        assert_eq!(
            envelope(controller.dispatch(request))["error"]["code"],
            "receipt_mismatch"
        );
        assert_eq!(
            load(
                &controller
                    .state
                    .join("tasks/synthetic-host/00000000-0000-4000-8000-000000000002.json")
            )
            .unwrap()["lookup_id"],
            "00000000-0000-4000-8000-000000000002"
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
            json!({"command":"inspect","environment_id":"malformed"}),
            json!({"command":"plan_show","plan_id":"malformed"}),
            json!({"command":"job","job_id":"malformed"}),
            json!({"command":"execute","plan_id":"x","approval":"x"}),
            json!({"command":"shell","argv":["bad"]}),
            json!({"command":"discover","shell":"bad"}),
            json!({"command":"plan_archive","environment_id":"00000000-0000-4000-8000-000000000001","categories":["sessions"],"selected_paths":["../escape.jsonl"]}),
            json!({"command":"work_preflight","environment_id":"00000000-0000-4000-8000-000000000001","categories":["memory"],"selected_paths":["projects/demo/session.jsonl"]}),
            json!({"command":"work_inventory","environment_id":"00000000-0000-4000-8000-000000000001","categories":["memory"],"offset":100}),
            json!({"command":"plan_policy","environment_id":"00000000-0000-4000-8000-000000000001","preset":"reduce","keep_remote_control":"false"}),
        ] {
            assert!(controller
                .dispatch(json!({"op":"request","alias":"synthetic-host","request":request}))
                .is_err());
        }
        assert!(!temp.path().join("args").exists());
    }
    #[test]
    fn outer_core_ids_are_rejected_before_records_or_ssh() {
        let (temp, controller) = fixture(RECEIPT);
        for id in ["plan-secret", "00000000000040008000000000000001"] {
            for op in ["execute", "reconnect"] {
                let mut request = json!({"op":op,"alias":"synthetic-host","plan_id":id});
                if op == "execute" {
                    request["approval"] = json!("synthetic");
                }
                let result = envelope(controller.dispatch(request));
                assert_eq!(result["error"]["code"], "invalid_request", "{result}");
                assert!(!controller.state.join("tasks").exists());
                assert!(!temp.path().join("args").exists());
            }
            let result = envelope(
                controller
                    .dispatch(json!({"op":"launch","alias":"synthetic-host","environment_id":id})),
            );
            assert_eq!(result["error"]["code"], "invalid_request", "{result}");
            assert!(!temp.path().join("args").exists());
        }
        let plan_id = "00000000-0000-4000-8000-000000000002";
        let (path, held) = controller.record("synthetic-host", plan_id).unwrap();
        save(
            &path,
            &json!({"plan_id":plan_id,"lookup_id":"plan-secret","status":"submission_unknown"}),
        )
        .unwrap();
        drop(held);
        let before = fs::read(&path).unwrap();
        let result = envelope(
            controller
                .dispatch(json!({"op":"reconnect","alias":"synthetic-host","plan_id":plan_id})),
        );
        assert_eq!(result["error"]["code"], "local_record_invalid", "{result}");
        assert_eq!(fs::read(&path).unwrap(), before);
        assert!(!temp.path().join("args").exists());
        let fresh = Controller {
            state: temp.path().join("fresh-state"),
            ..controller
        };
        let result =
            envelope(fresh.dispatch(
                json!({"op":"reconnect","alias":"synthetic-host","plan_id":"plan-secret"}),
            ));
        assert_eq!(result["error"]["code"], "invalid_request", "{result}");
        assert!(!fresh.state.exists());
    }
    #[test]
    fn services_use_finite_preview_schema_and_never_send_shell_or_mutation_bypass() {
        let (temp, controller) = fixture("printf '%s\\n' '{\"ok\":true,\"data\":{}}'");
        let inspect = json!({"command":"service_inspect","environment_id":"00000000-0000-4000-8000-000000000001","manager":"user","unit":"synthetic-target.service"});
        for unit in [
            "*.service",
            "../target.service",
            "--all.service",
            "x.service; touch /tmp/x",
            "target@.service",
        ] {
            let mut invalid = inspect.clone();
            invalid["unit"] = json!(unit);
            assert!(controller
                .dispatch(json!({"op":"request","alias":"synthetic-host","request":invalid}))
                .is_err());
        }
        let mut invalid = inspect.clone();
        invalid["manager"] = json!("other-manager");
        assert!(validate_request(&invalid).is_err());
        invalid = inspect.clone();
        invalid["shell"] = json!("stop");
        assert!(validate_request(&invalid).is_err());
        assert!(validate_request(&json!({"command":"service_stop","environment_id":"00000000-0000-4000-8000-000000000001","manager":"user","unit":"synthetic-target.service"})).is_err());
        assert!(!temp.path().join("args").exists());
        assert!(controller
            .dispatch(json!({"op":"request","alias":"synthetic-host","request":inspect}))
            .is_ok());
        assert!(validate_request(&json!({"command":"plan_service_quiesce","environment_id":"00000000-0000-4000-8000-000000000001","manager":"system","unit":"synthetic-target.service"})).is_ok());
        assert!(validate_request(
            &json!({"command":"plan_service_resume","job_id":"00000000-0000-4000-8000-000000000003"})
        )
        .is_ok());
        let args = fs::read_to_string(temp.path().join("args")).unwrap();
        assert!(!args.lines().any(|line| line == "submit"));
    }
    #[test]
    fn custom_policy_crosses_finite_schema_without_arbitrary_env() {
        let (temp, controller) = fixture("printf '%s\\n' '{\"ok\":true,\"data\":{}}'");
        let request = json!({"command":"plan_policy","environment_id":"00000000-0000-4000-8000-000000000001","preset":"custom","keep_remote_control":true,"trusted_devices":"not_required","custom_settings":{"DISABLE_ERROR_REPORTING":"disable","DISABLE_GROWTHBOOK":"remove","DISABLE_TELEMETRY":"keep"}});
        for choices in [
            json!(null),
            json!([]),
            json!({"API_KEY":"remove"}),
            json!({"DISABLE_TELEMETRY":"0"}),
            json!({"DISABLE_TELEMETRY":{"action":"remove"}}),
        ] {
            let mut invalid = request.clone();
            invalid["custom_settings"] = choices;
            assert!(controller
                .dispatch(json!({"op":"request","alias":"synthetic-host","request":invalid}))
                .is_err());
        }
        let mut invalid = request.clone();
        invalid["preset"] = json!("reduce");
        assert!(validate_request(&invalid).is_err());
        invalid = request.clone();
        invalid["release_settings"] = json!(["DISABLE_GROWTHBOOK"]);
        assert!(validate_request(&invalid).is_err());
        assert!(
            !temp.path().join("args").exists(),
            "invalid choices must fail before SSH"
        );
        assert_eq!(
            controller
                .dispatch(json!({"op":"request","alias":"synthetic-host","request":request}))
                .unwrap()["ok"],
            true
        );
        assert!(fs::read_to_string(temp.path().join("args"))
            .unwrap()
            .contains("request"));
    }
    #[test]
    fn shared_contract_plans_job_lookup_and_portable_archive_stay_on_stdin() {
        let (temp, controller) =
            fixture("printf '%s' '{\"ok\":true,\"data\":'; cat input; printf '}\\n'");
        let requests = [
            json!({"command":"inspect_components","environment_id":"00000000-0000-4000-8000-000000000001","project_cwd":"/synthetic/project"}),
            json!({"command":"work_inventory","environment_id":"00000000-0000-4000-8000-000000000001","categories":["memory"],"offset":100,"expected_digest":"a".repeat(64)}),
            json!({"command":"work_preflight","environment_id":"00000000-0000-4000-8000-000000000001","categories":["sessions"],"selected_paths":["projects/demo/session.jsonl"]}),
            json!({"command":"plan_archive","environment_id":"00000000-0000-4000-8000-000000000001","categories":["sessions"],"selected_paths":["projects/demo/session.jsonl"]}),
            json!({"command":"plan_preserve","environment_id":"00000000-0000-4000-8000-000000000001","categories":["sessions"],"selected_paths":["projects/demo/session.jsonl"]}),
            json!({"command":"plan_cleanup","environment_id":"00000000-0000-4000-8000-000000000001","recipe":"repair_login","writers_confirmed_stopped":true,"official_logout":false}),
            json!({"command":"plan_cleanup","environment_id":"00000000-0000-4000-8000-000000000001","recipe":"repair_login","writers_confirmed_stopped":true,"official_logout":false,"categories":[]}),
            json!({"command":"plan_show","plan_id":"00000000-0000-4000-8000-000000000002"}),
            json!({"command":"job","plan_id":"00000000-0000-4000-8000-000000000002"}),
            json!({"command":"plan_archive","environment_id":"00000000-0000-4000-8000-000000000001","categories":["memory"],"output_path":"/synthetic/export; touch bad.lintel-work"}),
            json!({"command":"plan_preserve","environment_id":"00000000-0000-4000-8000-000000000001","categories":["instructions","sessions"],"name":"Synthetic $(touch bad)"}),
            json!({"command":"archive_inspect","archive_path":"/synthetic/import; touch bad.lintel-work","archive_passphrase":"SECRET_PORTABLE_ARCHIVE_PASSPHRASE"}),
            json!({"command":"archive_read","archive_path":"/synthetic/import; touch bad.lintel-work","archive_passphrase":"SECRET_PORTABLE_ARCHIVE_PASSPHRASE","path":"memory/notes.md"}),
            json!({"command":"plan_import","environment_id":"00000000-0000-4000-8000-000000000001","archive_path":"/synthetic/import; touch bad.lintel-work","categories":["memory"],"archive_passphrase":"SECRET_PORTABLE_ARCHIVE_PASSPHRASE"}),
        ];
        for request in &requests {
            lintel_operations::validate(request).unwrap();
            let response = controller
                .dispatch(json!({"op":"request","alias":"synthetic-host","request":request}))
                .unwrap();
            assert_eq!(response["data"], *request);
            assert_eq!(
                serde_json::from_slice::<Value>(&fs::read(temp.path().join("input")).unwrap())
                    .unwrap(),
                *request,
            );
        }
        let args = fs::read_to_string(temp.path().join("args")).unwrap();
        assert_eq!(
            args.lines().filter(|arg| *arg == "request").count(),
            requests.len()
        );
        for forbidden in [
            "SECRET",
            "touch",
            "memory",
            "archive_path",
            "00000000-0000-4000-8000-000000000002",
            "00000000-0000-4000-8000-000000000001",
            "submit",
        ] {
            assert!(
                !args.contains(forbidden),
                "request data entered argv: {forbidden}"
            );
        }
        assert!(!temp.path().join("bad").exists());
        assert!(!controller.state.join("tasks").exists());
        assert_eq!(
            fs::read_dir(&controller.state)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["hosts.json".into(), "hosts.lock".into()]),
        );
    }

    #[test]
    fn shared_contract_rejects_ambiguous_sources_and_missing_confirmations_before_ssh() {
        let (temp, controller) = fixture(RECEIPT);
        for command in ["archive_inspect", "archive_read", "plan_import"] {
            let mut request = json!({"command":command,"archive_passphrase":"SECRET_PORTABLE_ARCHIVE_PASSPHRASE"});
            if command == "archive_read" {
                request["path"] = json!("memory/notes.md");
            }
            if command == "plan_import" {
                request["environment_id"] = json!("00000000-0000-4000-8000-000000000001");
                request["categories"] = json!(["memory"]);
            }
            // Neither source, both sources, and a relative path all violate the
            // same operation schema used by the named local CLI.
            for source in [
                json!({}),
                json!({"job_id":"00000000-0000-4000-8000-000000000003","archive_path":"/synthetic/archive"}),
                json!({"archive_path":"relative/archive"}),
            ] {
                let mut invalid = request.clone();
                invalid
                    .as_object_mut()
                    .unwrap()
                    .extend(source.as_object().unwrap().clone());
                assert!(lintel_operations::validate(&invalid).is_err());
                assert!(controller
                    .dispatch(json!({"op":"request","alias":"synthetic-host","request":invalid}))
                    .is_err());
            }
        }
        for request in [
            json!({"command":"job"}),
            json!({"command":"job","job_id":"00000000-0000-4000-8000-000000000003","plan_id":"00000000-0000-4000-8000-000000000002"}),
            json!({"command":"plan_policy","environment_id":"00000000-0000-4000-8000-000000000001","preset":"reduce"}),
            json!({"command":"plan_archive","environment_id":"00000000-0000-4000-8000-000000000001","categories":["memory"],"output_path":"relative/archive"}),
            json!({"command":"plan_archive","environment_id":"00000000-0000-4000-8000-000000000001","categories":[]}),
            json!({"command":"plan_preserve","environment_id":"00000000-0000-4000-8000-000000000001","categories":["memory"],"shell":"bad"}),
            json!({"command":"execute","plan_id":"00000000-0000-4000-8000-000000000002","approval":APPROVAL}),
            json!({"command":"launch","environment_id":"00000000-0000-4000-8000-000000000001"}),
            json!({"command":"launch_context","environment_id":"00000000-0000-4000-8000-000000000001"}),
        ] {
            assert!(controller
                .dispatch(json!({"op":"request","alias":"synthetic-host","request":request}))
                .is_err());
        }
        assert!(!temp.path().join("args").exists());
    }

    #[test]
    #[ignore = "requires tests/shared_remote_journey.py synthetic current HOME"]
    fn shared_contract_system_context_uses_current_home_and_explicit_resources() {
        let base = PathBuf::from(
            std::env::var_os("LINTEL_REMOTE_SYNTHETIC_FIXTURE")
                .expect("synthetic launcher required"),
        );
        let home = base.join("home");
        assert_eq!(std::env::var_os("HOME").unwrap(), home.as_os_str());
        assert_eq!(
            std::env::var_os("LINTEL_TEST_HOME").unwrap(),
            base.join("other-home").as_os_str()
        );
        let bundles = base.join("caller-supplied-bundles");
        let before_catalog = fs::read_dir(&base)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<BTreeSet<_>>();
        let catalog = operation_catalog();
        assert_eq!(catalog["operations"].as_array().unwrap().len(), 16);
        assert_eq!(
            fs::read_dir(&base)
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect::<BTreeSet<_>>(),
            before_catalog
        );
        let controller = Controller::system(bundles.clone()).unwrap();
        let state = std::env::var_os("LINTEL_STATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                if cfg!(target_os = "macos") {
                    home.join("Library/Application Support/Lintel")
                } else {
                    home.join(".local/share/lintel")
                }
            });
        assert_eq!(controller.state, state.join("remote"));
        assert_eq!(controller.config, home.join(".ssh/config"));
        assert_eq!(controller.bundles, bundles);
        assert_eq!(controller.transport.ssh, PathBuf::from("/usr/bin/ssh"));
        assert_eq!(controller.terminal.is_some(), cfg!(target_os = "macos"));
        assert_eq!(
            control(json!({"op":"aliases"}), bundles.clone())["data"]["aliases"],
            json!(["synthetic-current-home"])
        );
        assert_eq!(
            control(
                json!({"op":"add_host","alias":"synthetic-current-home"}),
                bundles.clone()
            )["data"]["alias"],
            "synthetic-current-home"
        );
        assert_eq!(
            control(json!({"op":"hosts"}), bundles)["data"]["hosts"],
            json!([{"alias":"synthetic-current-home"}])
        );
        assert!(state.join("remote/hosts.json").exists());
        assert!(!base.join("other-home/.local/share/lintel").exists());
    }

    #[test]
    fn cleanup_and_archive_payloads_keep_data_on_stdin_without_persistence() {
        let (temp, controller) = fixture("printf '%s\\n' '{\"ok\":true,\"data\":{}}'");
        for request in [
            json!({"command":"plan_cleanup","environment_id":"00000000-0000-4000-8000-000000000001","recipe":"reset_client","writers_confirmed_stopped":true,"official_logout":false,"categories":["memory"]}),
            json!({"command":"archive_inspect","job_id":"00000000-0000-4000-8000-000000000002","archive_passphrase":"SECRET_SYNTHETIC_PHRASE"}),
            json!({"command":"archive_read","job_id":"00000000-0000-4000-8000-000000000002","archive_passphrase":"SECRET_SYNTHETIC_PHRASE","path":"memory/notes.md"}),
            json!({"command":"plan_import","environment_id":"00000000-0000-4000-8000-000000000001","job_id":"00000000-0000-4000-8000-000000000002","categories":["memory"],"archive_passphrase":"SECRET_SYNTHETIC_PHRASE"}),
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

    #[test]
    fn removing_host_retains_pending_jobs_and_readding_cannot_replay() {
        let (temp, controller) = fixture(&format!(
            "if test ! -f called; then touch called; exit 255; fi\n{RECEIPT}"
        ));
        fs::create_dir_all(controller.config.parent().unwrap()).unwrap();
        fs::write(
            &controller.config,
            "Host synthetic-host\n  HostName synthetic.invalid\n",
        )
        .unwrap();
        let known_hosts = controller.config.parent().unwrap().join("known_hosts");
        fs::write(&known_hosts, "synthetic untouched host key").unwrap();
        let request = json!({"op":"execute","alias":"synthetic-host","plan_id":"00000000-0000-4000-8000-000000000002","approval":APPROVAL});
        assert_eq!(
            envelope(controller.dispatch(request.clone()))["error"]["code"],
            "transport_unknown"
        );
        let task = controller
            .state
            .join("tasks/synthetic-host/00000000-0000-4000-8000-000000000002.json");
        let before = fs::read(&task).unwrap();
        controller
            .dispatch(json!({"op":"add_host","alias":"other-host"}))
            .unwrap();
        assert_eq!(
            controller
                .dispatch(json!({"op":"remove_host","alias":"synthetic-host"}))
                .unwrap()["data"],
            json!({"alias":"synthetic-host","removed":true})
        );
        assert_eq!(
            controller
                .dispatch(json!({"op":"remove_host","alias":"synthetic-host"}))
                .unwrap()["data"]["removed"],
            false
        );
        assert_eq!(fs::read(&task).unwrap(), before);
        let listed = controller.dispatch(json!({"op":"hosts"})).unwrap();
        assert_eq!(listed["data"]["hosts"], json!([{"alias":"other-host"}]));
        assert_eq!(listed["data"]["tasks"][0]["alias"], "synthetic-host");
        assert_eq!(listed["data"]["tasks"][0]["status"], "submission_unknown");
        assert_eq!(
            envelope(controller.dispatch(request.clone()))["error"]["code"],
            "host_not_registered"
        );
        let queried = controller
            .dispatch(json!({"op":"reconnect","alias":"synthetic-host","plan_id":"00000000-0000-4000-8000-000000000002"}))
            .unwrap();
        assert_eq!(queried["data"]["status"], "completed");
        let queried = controller.dispatch(json!({"op":"request","alias":"synthetic-host","request":{"command":"job","job_id":"00000000-0000-4000-8000-000000000002"}})).unwrap();
        assert_eq!(queried["data"]["status"], "completed");
        assert_eq!(envelope(controller.dispatch(json!({"op":"request","alias":"synthetic-host","request":{"command":"job","job_id":"00000000-0000-4000-8000-000000000006"}})))["error"]["code"], "host_not_registered");
        assert_eq!(
            envelope(controller.dispatch(
                json!({"op":"reconnect","alias":"synthetic-host","plan_id":"00000000-0000-4000-8000-000000000006"})
            ))["error"]["code"],
            "host_not_registered"
        );
        controller
            .dispatch(json!({"op":"add_host","alias":"synthetic-host"}))
            .unwrap();
        assert_eq!(
            controller.dispatch(request).unwrap()["data"]["status"],
            "completed"
        );
        let args = fs::read_to_string(temp.path().join("args")).unwrap();
        assert_eq!(args.lines().filter(|line| *line == "submit").count(), 1);
        assert_eq!(args.lines().filter(|line| *line == "request").count(), 3);
        assert_eq!(
            fs::read_to_string(&controller.config).unwrap(),
            "Host synthetic-host\n  HostName synthetic.invalid\n"
        );
        assert_eq!(
            fs::read_to_string(known_hosts).unwrap(),
            "synthetic untouched host key"
        );
        assert!(controller
            .state
            .join("tasks/synthetic-host/00000000-0000-4000-8000-000000000002.lock")
            .exists());
    }

    #[test]
    fn removing_unsubmitted_alias_has_no_transport_or_task_side_effect() {
        let (temp, controller) = fixture(RECEIPT);
        let removed = controller
            .dispatch(json!({"op":"remove_host","alias":"synthetic-host"}))
            .unwrap();
        assert_eq!(removed["data"]["removed"], true);
        assert_eq!(
            controller.dispatch(json!({"op":"hosts"})).unwrap()["data"],
            json!({"hosts":[],"tasks":[],"installations":[],"launches":[]})
        );
        assert_eq!(
            controller
                .dispatch(json!({"op":"remove_host","alias":"never-added"}))
                .unwrap()["data"]["removed"],
            false
        );
        assert!(!temp.path().join("args").exists());
        assert!(!controller.state.join("tasks").exists());
    }

    #[test]
    fn ssh_diagnostics_classify_only_observed_causes() {
        for (stderr, expected) in [
            ("ssh: Could not resolve hostname synthetic.invalid: nodename nor servname provided", "dns"),
            ("ssh: connect to host synthetic.invalid port 22: Operation timed out", "timeout"),
            ("ssh: connect to host synthetic.invalid port 22: Connection refused", "connection_refused"),
            ("ssh: connect to host synthetic.invalid port 22: No route to host", "network_unreachable"),
            ("synthetic-user@synthetic.invalid: Permission denied (publickey).", "authentication_failed"),
            ("WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!", "host_key_changed"),
            ("No ED25519 host key is known for synthetic.invalid and you have requested strict checking.", "host_key_unknown"),
            ("Host key verification failed.", "host_key_verification_failed"),
            ("synthetic-config: line 4: Bad configuration option: syntheticbad", "config_invalid"),
            ("kex_exchange_identification: Connection closed by remote host", "ssh_unknown"),
        ] {
            let (_temp, controller) = fixture(&format!("test \"$LC_ALL\" = C || exit 8\nprintf '%s\\n' '{stderr}' >&2\nexit 255"));
            let response = envelope(controller.dispatch(json!({"op":"connect","alias":"synthetic-host"})));
            let diagnostic = &response["error"]["diagnostic"];
            assert_eq!(response["error"]["code"], "transport_unknown");
            assert_eq!(diagnostic["stage"], "ssh");
            assert_eq!(diagnostic["reason"], expected, "{response}");
            assert_eq!(diagnostic["exit_code"], 255);
            assert_eq!(diagnostic["submission_uncertain"], false);
            assert!(!response["error"]["message"].as_str().unwrap().contains("原任务"));
            assert!(!diagnostic["next_steps"].as_array().unwrap().is_empty());
        }
    }

    #[test]
    fn runner_and_response_diagnostics_distinguish_missing_binary_from_bad_protocol() {
        for (body, reason, stage) in [
            (
                "printf 'sh: lintel: command not found' >&2; exit 127",
                "runner_missing",
                "runner",
            ),
            (
                "printf 'sh: lintel: Permission denied' >&2; exit 126",
                "runner_not_executable",
                "runner",
            ),
            (
                "printf 'sh: some-other-program: command not found' >&2; exit 127",
                "abnormal_exit",
                "runner",
            ),
            ("printf 'a remote login banner'", "invalid_json", "response"),
            (
                "printf '%s' '{\"some\":\"json\"}'",
                "protocol_invalid",
                "response",
            ),
            (
                "printf '%s' '{\"ok\":true,\"data\":{}}'; exit 7",
                "abnormal_exit",
                "runner",
            ),
        ] {
            let (_temp, controller) = fixture(body);
            let response =
                envelope(controller.dispatch(json!({"op":"connect","alias":"synthetic-host"})));
            let diagnostic = &response["error"]["diagnostic"];
            assert_eq!(diagnostic["reason"], reason, "{response}");
            assert_eq!(diagnostic["stage"], stage);
            if reason == "runner_missing" {
                assert!(response["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("Claude Code 不代表"));
            }
        }
    }

    #[test]
    fn large_stderr_is_drained_bounded_and_control_sequences_never_reach_ui() {
        let (_temp, controller) = fixture(
            r"printf '\033[31mConnection refused\033[0m\r\001\033]0;hidden-title\007\n' >&2; head -c 2097152 /dev/zero | tr '\000' x >&2; exit 255",
        );
        let response =
            envelope(controller.dispatch(json!({"op":"connect","alias":"synthetic-host"})));
        let diagnostic = &response["error"]["diagnostic"];
        assert_eq!(diagnostic["reason"], "connection_refused");
        assert_eq!(diagnostic["stderr_truncated"], true);
        let excerpt = diagnostic["stderr_excerpt"].as_str().unwrap();
        assert!(excerpt.len() <= MAX_EXCERPT);
        assert!(!excerpt.contains("hidden-title"));
        assert!(excerpt
            .chars()
            .all(|ch| !ch.is_control() || ch == '\n' || ch == '\t'));
        // Filling stderr before consuming a large stdin cannot deadlock either pipe.
        let (temp, controller) = fixture(RECEIPT);
        fs::write(
            &controller.transport.ssh,
            format!(
                "#!/bin/sh\ncd '{}'\nhead -c 2097152 /dev/zero >&2\ncat > input\n{RECEIPT}\n",
                temp.path().display()
            ),
        )
        .unwrap();
        let request = json!({"command":"execute","approval":"x".repeat(512*1024)});
        assert_eq!(
            controller
                .transport
                .call("synthetic-host", &request, true)
                .unwrap()["ok"],
            true
        );
    }

    #[test]
    fn diagnostic_excerpt_redacts_echoed_payload_and_suppresses_authorization_output() {
        let (temp, controller) =
            fixture("cat input >&2; printf '\nPermission denied (publickey)' >&2; exit 255");
        let request = json!({"op":"request","alias":"synthetic-host","request":{"command":"register","name":"PRIVATE_NAME_WITH_\"QUOTE","root":"/private/synthetic/source"}});
        let response = envelope(controller.dispatch(request));
        let excerpt = response["error"]["diagnostic"]["stderr_excerpt"]
            .as_str()
            .unwrap();
        assert!(!excerpt.contains("PRIVATE_NAME"));
        assert!(!excerpt.contains("/private/synthetic/source"));
        assert_eq!(
            response["error"]["diagnostic"]["reason"],
            "authentication_failed"
        );
        let secret = "PRIVATE_ARCHIVE_PASSPHRASE";
        let response = envelope(controller.dispatch(json!({"op":"execute","alias":"synthetic-host","plan_id":"00000000-0000-4000-8000-000000000004","approval":APPROVAL,"archive_passphrase":secret})));
        assert_eq!(
            response["error"]["diagnostic"]["submission_uncertain"],
            true
        );
        assert!(response["error"]["diagnostic"]
            .get("stderr_excerpt")
            .is_none());
        assert!(!response.to_string().contains("PRIVATE"));
        assert!(!response.to_string().contains(APPROVAL));
        let record = fs::read_to_string(
            controller
                .state
                .join("tasks/synthetic-host/00000000-0000-4000-8000-000000000004.json"),
        )
        .unwrap();
        assert!(!record.contains("PRIVATE"));
        assert!(!record.contains(APPROVAL));
        assert!(!record.contains("diagnostic"));
        assert!(!record.contains("stderr"));
        assert!(fs::read_to_string(temp.path().join("input"))
            .unwrap()
            .contains(secret));
    }

    #[test]
    fn local_spawn_and_reconnect_errors_keep_action_specific_guidance() {
        let (_temp, mut controller) = fixture("printf 'Connection refused' >&2; exit 255");
        let response = envelope(controller.dispatch(
            json!({"op":"reconnect","alias":"synthetic-host","plan_id":"00000000-0000-4000-8000-000000000005"}),
        ));
        assert_eq!(
            response["error"]["diagnostic"]["reason"],
            "connection_refused"
        );
        assert!(response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("原任务"));
        controller.transport.interpreter = None;
        controller.transport.ssh = controller.state.join("missing-synthetic-ssh");
        let response =
            envelope(controller.dispatch(json!({"op":"connect","alias":"synthetic-host"})));
        assert_eq!(response["error"]["code"], "ssh_unavailable");
        assert_eq!(response["error"]["diagnostic"]["stage"], "local");
        assert_eq!(
            response["error"]["diagnostic"]["submission_uncertain"],
            false
        );
    }

    #[test]
    fn diagnostic_commands_are_safe_alias_checks_not_submission_replays() {
        for (body, submit) in [
            ("printf 'Connection refused' >&2; exit 255", false),
            ("printf 'Permission denied (publickey)' >&2; exit 255", true),
            ("printf '%s' '{\"ok\":false,\"error\":{\"code\":\"synthetic_rejected\",\"message\":\"synthetic error\"}}'; exit 1", true),
        ] {
            let (temp, controller) = fixture(body);
            let payload = if submit { json!({"command":"execute","plan_id":"00000000-0000-4000-8000-000000000002","approval":"SENSITIVE_APPROVAL"}) } else { json!({"command":"discover"}) };
            let response = envelope(controller.transport.call("synthetic-host", &payload, submit));
            let command = response["error"]["diagnostic"]["command"].as_str().unwrap();
            assert!(command.ends_with("synthetic-host 'command -v lintel'"));
            assert!(!command.contains("submit"));
            assert!(!command.contains("request"));
            assert!(!command.contains("SENSITIVE"));
            assert!(!command.contains("00000000-0000-4000-8000-000000000002"));
            let args = fs::read_to_string(temp.path().join("args")).unwrap();
            for flag in args.lines().filter(|arg| arg.starts_with('-')) { assert!(command.contains(flag), "missing {flag}: {command}"); }
        }
        let (_temp, mut controller) = fixture(RECEIPT);
        controller.transport.ssh = controller.state.join("nonexistent-ssh");
        controller.transport.interpreter = None;
        let response =
            envelope(controller.dispatch(json!({"op":"connect","alias":"synthetic-host"})));
        assert!(response["error"]["diagnostic"]["command"]
            .as_str()
            .unwrap()
            .contains("synthetic-host 'command -v lintel'"));
    }

    #[test]
    fn query_preserves_runner_diagnostic_without_replaying_on_retry() {
        let (temp, controller) = fixture("printf 'synthetic runner detail' >&2; printf '%s' '{\"ok\":false,\"error\":{\"code\":\"job_not_found\",\"message\":\"synthetic original job missing\"}}'; exit 1");
        for request in [
            json!({"op":"reconnect","alias":"synthetic-host","plan_id":"00000000-0000-4000-8000-000000000005"}),
            json!({"op":"execute","alias":"synthetic-host","plan_id":"00000000-0000-4000-8000-000000000005","approval":APPROVAL}),
        ] {
            let response = envelope(controller.dispatch(request));
            assert_eq!(response["error"]["code"], "reconciliation_required");
            let message = response["error"]["message"].as_str().unwrap();
            assert!(message.contains("job_not_found"));
            assert!(message.contains("synthetic original job missing"));
            assert!(message.contains("不会重新提交"));
            let diagnostic = &response["error"]["diagnostic"];
            assert_eq!(diagnostic["reason"], "runner_rejected");
            assert_eq!(diagnostic["exit_code"], 1);
            assert_eq!(diagnostic["stderr_excerpt"], "synthetic runner detail");
            assert!(diagnostic["command"]
                .as_str()
                .unwrap()
                .ends_with("synthetic-host 'command -v lintel'"));
        }
        let args = fs::read_to_string(temp.path().join("args")).unwrap();
        assert_eq!(args.lines().filter(|arg| *arg == "request").count(), 2);
        assert!(!args.lines().any(|arg| arg == "submit"));
        assert!(!fs::read_to_string(
            controller
                .state
                .join("tasks/synthetic-host/00000000-0000-4000-8000-000000000005.json")
        )
        .unwrap()
        .contains("diagnostic"));
    }

    #[test]
    fn launch_binding_resolution_distinguishes_pinned_path_from_absent() {
        let (temp, controller) = fixture(RECEIPT);
        fs::create_dir_all(controller.state.join("bindings")).unwrap();
        let request_id = "00000000-0000-4000-8000-0000000000aa";
        let dir = controller.state.join("launches/synthetic-host");
        fs::create_dir_all(&dir).unwrap();
        // Pinned to PATH: runner_digest is null but the record exists.
        save(
            &dir.join(format!("{request_id}.json")),
            &json!({"request_id":request_id,"runner_digest":Value::Null,"attempted":true,"status":"launch_intent"}),
        )
        .unwrap();
        let (bound, pinned) = controller
            .resolve_launch_binding("synthetic-host", request_id)
            .unwrap();
        assert!(
            pinned && bound.is_none(),
            "pinned PATH must resolve to (None, pinned=true)"
        );
        // Absent record falls back to the current binding (legacy), not a pin.
        let (bound2, pinned2) = controller
            .resolve_launch_binding("synthetic-host", "00000000-0000-4000-8000-0000000000bb")
            .unwrap();
        assert!(!pinned2 && bound2.is_none());
        let _ = temp;
    }

    #[test]
    fn launch_query_returns_binding_metadata_without_terminal() {
        let (temp, controller) = fixture(RECEIPT);
        let request_id = "00000000-0000-4000-8000-0000000000cc";
        let dir = controller.state.join("launches/synthetic-host");
        fs::create_dir_all(&dir).unwrap();
        let sha = "a".repeat(64);
        save(
            &dir.join(format!("{request_id}.json")),
            &json!({"request_id":request_id,"runner_digest":sha,"attempted":true,"status":"launch_intent","mode":"launch"}),
        )
        .unwrap();
        let response = controller
            .dispatch(json!({"op":"launch_query","alias":"synthetic-host","request_id":request_id}))
            .unwrap();
        assert_eq!(response["ok"], true, "{response}");
        assert_eq!(response["data"]["runner_digest"], sha);
        assert_eq!(response["data"]["binding_resolution"], "pinned_digest");
        // The Terminal opener was never invoked for a read-only query.
        assert!(!temp.path().join("open").exists());
    }

    #[test]
    fn launch_record_survives_alias_removal_and_list_uses_pinned_runner() {
        let (temp, controller) = fixture(RECEIPT);
        let request_id = "00000000-0000-4000-8000-0000000000dd";
        let sha = "b".repeat(64);
        let dir = controller.state.join("launches/synthetic-host");
        fs::create_dir_all(&dir).unwrap();
        save(
            &dir.join(format!("{request_id}.json")),
            &json!({"request_id":request_id,"runner_digest":sha,"attempted":true,"status":"launch_intent","mode":"launch"}),
        )
        .unwrap();
        // Removing the alias keeps the durable launch record queryable/listable.
        controller
            .dispatch(json!({"op":"remove_host","alias":"synthetic-host"}))
            .unwrap();
        let list = controller
            .dispatch(json!({"op":"launches","alias":"synthetic-host"}))
            .unwrap();
        assert_eq!(list["ok"], true, "{list}");
        let entries = list["data"]["launches"].as_array().unwrap();
        // The list is built from the local durable record (one entry) and uses its
        // pinned digest, whether or not the remote query succeeded.
        assert_eq!(entries.len(), 1, "{list}");
        assert_eq!(entries[0]["alias"], "synthetic-host");
        assert_eq!(entries[0]["runner_digest"], sha);
        let _ = temp;
    }
}

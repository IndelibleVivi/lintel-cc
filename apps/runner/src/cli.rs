//! Named agent surface over the existing core/host/egress execution owners.
use serde_json::{json, Value};
use std::{
    io::{self, IsTerminal, Read},
    time::{Duration, Instant},
};

pub const HELP: &str = concat!(
  "Lintel — Claude environment control\n\n",
  "  lintel version [--json]                     Static build/protocol identity\n",
  "  lintel capabilities [--environment ID]       Operation catalog; optional target inspection\n",
  "  lintel describe OPERATION [--json]           Parameters, effects, approval and recovery\n",
  "  lintel schema OPERATION                     JSON Schema envelope, no discovery\n",
  "  lintel context [--json]                      Readonly state/user/executable facts; no state made\n",
  "  lintel tasks [--json]                        The one finite six-task map; no state made\n",
  "  lintel help [GROUP]                         Static help; GROUP ∈ env/policy/work/job/restore/remote/launch/session\n",
  "  lintel env list | inspect ID | components ID [--project-cwd PATH] | create --name NAME | register --name NAME --root PATH\n",
  "  lintel policy plan --environment ID --preset reduce --keep-remote-control\n",
  "  lintel plan show ID                         Read original frozen plan\n",
  "  lintel job show ID | wait ID --timeout 30s   Query only; timeout never resubmits\n",
  "  lintel job submit --plan ID --approval HASH  Durable ACK; secret fields via JSON stdin\n",
  "  lintel restore plan --job ID                Prepare an independent restoration\n",
  "  lintel work archive plan --environment ID --categories instructions,memory,sessions\n",
  "  lintel work archive list | inspect | read   Use --job ID or --archive-path PATH\n",
  "  lintel work inventory --environment ID --categories ... [--offset N --expected-digest HEX]\n",
  "  lintel work preflight --environment ID --categories ... [--path RELATIVE_PATH ...]\n",
  "  lintel work archive|preserve plan ... [--path RELATIVE_PATH ...]  Exact original-file subset\n",
  "  lintel work session read                    Bounded paged read; passphrase via JSON stdin\n",
  "  lintel work preserve plan --environment ID --categories instructions,memory,sessions\n",
  "  lintel work import plan --environment ID --archive-path PATH --categories memory,sessions\n",
  "  lintel launch ID                            Legacy real TTY, root as cwd, no prompt\n",
  "  lintel launch request ID HASH               Frozen interactive start; no prompt; real TTY\n",
  "  lintel launch resume ID HASH                Frozen native resume; passphrase via no-echo TTY; real TTY\n",
  "  lintel launch query REQUEST_ID              Readonly: find one original startup request\n",
  "  lintel launch list                          Readonly: list durable startup requests\n",
  "  lintel browser operations [--instance ID]    Static actions; optional pairing/online facts\n",
  "  lintel browser instances | pair create | pair pending | pair approve --challenge ID\n",
  "  lintel browser submit | query | control    Finite host control; JSON stdin\n",
  "  lintel network serve --config PATH          Foreground owner, NDJSON, Ctrl-C stops\n",
  "  lintel network inspect                     Read shared host network metadata\n",
  "  lintel network probe [--ipv4-url URL --ipv6-url URL --proxy-url URL]\n",
  "  lintel network ipv6 plan --service-id ID --mode off|link_local [--baseline-id ID]\n",
  "  lintel network restore plan --job ID        Original full IPv6 config; separate approval\n",
  "  lintel remote operations                   Static finite SSH schemas, no state\n",
  "  lintel remote hosts | aliases | inspect ALIAS\n",
  "  lintel remote request ALIAS | submit ALIAS   JSON stdin; submit only execute\n",
  "  lintel remote job ALIAS PLAN_ID             Query original; no resubmission\n",
  "  lintel remote launch ALIAS ENVIRONMENT_ID    Real TTY, macOS Terminal, no prompt\n",
  "  lintel remote launch request ALIAS ID HASH   Approved frozen cwd/config; macOS Terminal\n",
  "  lintel remote launch resume ALIAS ID HASH    Finite resume; secret entered in SSH TTY\n",
  "  lintel remote launch query ALIAS ID          Query original runner; no Terminal\n",
  "  lintel remote launch list ALIAS              Local original-launch metadata; no SSH\n",
  "  lintel remote control [--bundles PATH]       Shared finite SSH/installation; JSON stdin\n",
  "  lintel call OPERATION                       Core requests via JSON stdin; interactive sessions use launch ID\n\n",
  "Legacy: request, submit, discover, inspect ID, jobs, job ID, launch ID, tui.\n",
  "Ordinary commands print one ok/data or ok/error envelope. Network serve is explicitly NDJSON.\n",
  "Passphrases never belong in argv or persistent request files. No blanket --yes.\n",
  "Static catalog commands do not initialize state. Protocol-1 request retains historical defaults.\n",
);

pub fn error(code: &str, message: impl AsRef<str>) -> Value {
    json!({"ok":false,"error":{"code":code,"message":message.as_ref()}})
}

pub fn stdin_json(required: bool) -> Result<Value, Value> {
    if io::stdin().is_terminal() {
        return if required {
            Err(error(
                "stdin_required",
                "请从 stdin 提供一个 JSON object；秘密不要放 argv",
            ))
        } else {
            Ok(json!({}))
        };
    }
    let mut bytes = vec![];
    if io::stdin()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() > 1024 * 1024
    {
        return Err(error("request_limit", "stdin 超过 1 MiB 或读取失败"));
    }
    if !required && bytes.iter().all(u8::is_ascii_whitespace) {
        return Ok(json!({}));
    }
    let v: Value = lintel_core::decode_request(&bytes)
        .map_err(|_| error("invalid_json", "stdin 不是一个有效 JSON object"))?;
    if !v.is_object() {
        return Err(error("invalid_request", "stdin 必须是 JSON object"));
    }
    Ok(v)
}

pub fn dispatch(request: Value) -> Value {
    match request["command"].as_str().unwrap_or("") {
        "version" => {
            json!({"ok":true,"data":{"product":"Lintel","version":env!("CARGO_PKG_VERSION"),"protocol":1,"catalog_version":1,"platform":std::env::consts::OS,"architecture":std::env::consts::ARCH}})
        }
        // Readonly execution context. Uses core's resolver, which constructs no
        // Engine: it never creates state or runs discovery.
        "context" => json!({"ok":true,"data":lintel_core::execution_context()}),
        // One finite task map shared with the frontend/website. Static metadata.
        "tasks" => json!({"ok":true,"data":lintel_operations::tasks()}),
        // Frozen launch/resume execution is only reachable through the dedicated
        // real-TTY CLI branch in main.rs (which calls core's interactive handler
        // directly). The generic static dispatcher — including hidden `lintel call`
        // or `request` JSON — must never open a Terminal or start a client.
        "launch_request" | "resume_request" => error(
            "terminal_required",
            "此启动只能通过真实交互终端入口（lintel launch request|resume <ID> <HASH>）执行；JSON/管道调用不会启动客户端。",
        ),
        "capabilities" => {
            let mut catalog = lintel_operations::catalog();
            catalog["remote_control"] = lintel_remote::operation_catalog();
            catalog["runner"] = json!({"detached_submission":true,"ack":"durable_acceptance","reboot_survival":false});
            catalog["adapters"][0]["implementation"] = json!(if cfg!(target_os = "macos") {
                "included"
            } else {
                "not_in_this_build"
            });
            if let Some(id) = request.get("environment_id") {
                let inspection_request = json!({"command":"inspect","environment_id":id});
                if let Err(e) = lintel_operations::validate(&inspection_request) {
                    return error("invalid_request", e);
                }
                let inspection = lintel_core::handle_request(inspection_request);
                if inspection["ok"] != true {
                    return inspection;
                }
                catalog["target"] = inspection["data"].clone();
            }
            json!({"ok":true,"data":catalog})
        }
        "describe" | "schema" => {
            let operation = request["operation"].as_str().unwrap_or("");
            if operation.starts_with("remote.") {
                let catalog = lintel_remote::operation_catalog();
                return catalog["operations"].as_array().unwrap().iter().find(|op|op["id"]==operation).map_or_else(||error("unknown_operation",operation),|op|json!({"ok":true,"data":if request["command"]=="schema"{op["request_schema"].clone()}else{op.clone()}}));
            }
            #[cfg(target_os = "macos")]
            if operation.starts_with("browser.") {
                let catalog = lintel_browser_host::operation_catalog();
                return catalog["operations"].as_array().unwrap().iter().find(|op|op["id"]==operation).map_or_else(
                    ||error("unknown_operation",operation),
                    |op|json!({"ok":true,"data":if request["command"]=="schema"{op["request_schema"].clone()}else{op.clone()}}));
            }

            let v = if request["command"] == "schema" {
                lintel_operations::schema(operation)
            } else {
                lintel_operations::describe(operation)
            };
            v.map_or_else(
                || error("unknown_operation", format!("没有 operation {operation}")),
                |data| json!({"ok":true,"data":data}),
            )
        }
        _ => {
            let discover = request["command"] == "discover";
            let mut response = lintel_core::handle_request(request);
            if discover {
                crate::submission::capability(&mut response);
            }
            response
        }
    }
}

fn flags(args: &[String], mut request: Value) -> Result<Value, Value> {
    // `--path` is repeatable only for the exact work-selection commands; every
    // other command keeps its existing single `--path` string (archive/session
    // read). `selected_paths` is name-gated by `named()` so it cannot leak into
    // an unsupported command's request.
    let repeats_path = matches!(
        request.get("command").and_then(Value::as_str),
        Some("work_preflight" | "plan_archive" | "plan_preserve")
    );
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].as_str();
        if flag == "--json" {
            i += 1;
            continue;
        }
        if ["--archive-passphrase", "--password", "--token"].contains(&flag) {
            return Err(error(
                "secret_stdin_required",
                "秘密字段只通过 JSON stdin 临时传入",
            ));
        }
        if [
            "--keep-remote-control",
            "--no-keep-remote-control",
            "--writers-confirmed-stopped",
            "--official-logout",
            "--no-official-logout",
        ]
        .contains(&flag)
        {
            let key = match flag {
                "--keep-remote-control" | "--no-keep-remote-control" => "keep_remote_control",
                "--writers-confirmed-stopped" => "writers_confirmed_stopped",
                _ => "official_logout",
            };
            if request.get(key).is_some() {
                return Err(error("duplicate_argument", key));
            }
            request[key] = json!(!flag.starts_with("--no-"));
            i += 1;
            continue;
        }
        if !flag.starts_with("--") || i + 1 >= args.len() {
            return Err(error("invalid_argument", format!("无效或缺少值: {flag}")));
        }
        let key = match flag {
            "--environment" => "environment_id",
            "--plan" => "plan_id",
            "--job" => "job_id",
            _ => flag.trim_start_matches("--"),
        };
        let key = key.replace('-', "_");
        if repeats_path && key == "path" {
            let raw = &args[i + 1];
            let list = request
                .as_object_mut()
                .unwrap()
                .entry("selected_paths")
                .or_insert_with(|| json!([]));
            let list = list
                .as_array_mut()
                .ok_or_else(|| error("invalid_argument", "selected_paths 必须是数组"))?;
            list.push(json!(raw));
            i += 2;
            continue;
        }
        if request.get(&key).is_some() {
            return Err(error("duplicate_argument", &key));
        }
        let raw = &args[i + 1];
        request[&key] = if ["categories", "release_settings"].contains(&key.as_str()) {
            json!(raw.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>())
        } else if key == "custom_settings" || key == "probe" {
            serde_json::from_str(raw)
                .map_err(|_| error("invalid_argument", "该字段需要 JSON object"))?
        } else if key == "offset" || key == "timeout_seconds" {
            json!(raw
                .parse::<u64>()
                .map_err(|_| error("invalid_argument", &format!("{key} 需要非负整数")))?)
        } else {
            json!(raw)
        };
        i += 2;
    }
    Ok(request)
}

fn named(command: &str, args: &[String], mut seed: Value, input: bool) -> Value {
    if command == "launch" {
        return error(
            "interactive_launch_required",
            "请在真实交互终端使用 lintel launch <environment-id> 启动；该入口核对 stdin/stdout TTY，并且不接受 prompt",
        );
    }
    // The operation is known before parsing argv. Repeatable --path belongs to
    // exact work selection; archive/session read retain their single path.
    if seed.get("command").is_none() {
        seed["command"] = json!(command);
    }
    let mut request = match flags(args, seed) {
        Ok(r) => r,
        Err(e) => return e,
    };
    if input {
        let fields = match stdin_json(false) {
            Ok(r) => r,
            Err(e) => return e,
        };
        for (key, value) in fields.as_object().unwrap() {
            if key == "command" && value == command {
                continue;
            }
            if request.get(key).is_some() {
                return error("duplicate_argument", key);
            }
            request[key] = value.clone();
        }
    }
    if request.get("command").is_some_and(|v| v != command) {
        return error("invalid_request", "stdin command 与命名入口不一致");
    }
    request["command"] = json!(command);
    if command == "plan_network_ipv6" && request.get("probe").is_none() {
        let mut probe = json!({});
        for key in ["ipv4_url", "ipv6_url", "proxy_url", "timeout_seconds"] {
            if let Some(value) = request.as_object_mut().unwrap().remove(key) {
                probe[key] = value;
            }
        }
        request["probe"] = probe;
    }
    if matches!(command, "network_probe" | "plan_network_ipv6") {
        let probe = if command == "network_probe" {
            &mut request
        } else {
            &mut request["probe"]
        };
        if let Some(probe) = probe.as_object_mut() {
            probe
                .entry("ipv4_url")
                .or_insert(json!("https://api.ipify.org"));
            probe
                .entry("ipv6_url")
                .or_insert(json!("https://api6.ipify.org"));
        }
    }
    if let Err(e) = lintel_operations::validate(&request) {
        return error("invalid_request", e);
    }
    if command == "execute" {
        crate::submission::submit(&request)
    } else {
        dispatch(request)
    }
}

pub fn wait(job: &str, timeout: Duration) -> Value {
    let request = json!({"command":"job","plan_id":job});
    if let Err(e) = lintel_operations::validate(&request) {
        return error("invalid_request", e);
    }
    let start = Instant::now();
    loop {
        let response = dispatch(request.clone());
        if response["ok"] != true {
            return response;
        }
        if !["accepted", "executing", "verifying", "pending"]
            .contains(&response["data"]["status"].as_str().unwrap_or(""))
        {
            return response;
        }
        if start.elapsed() >= timeout {
            return json!({"ok":false,"error":{"code":"wait_timeout","message":"原任务仍未结束；保留 plan_id，只查询原任务，不重新提交"},"plan_id":job,"data":response["data"]});
        }
        std::thread::sleep(Duration::from_millis(250).min(timeout.saturating_sub(start.elapsed())));
    }
}

fn timeout(raw: &str) -> Option<Duration> {
    let (raw, multiplier) = if let Some(n) = raw.strip_suffix("ms") {
        (n, 1)
    } else if let Some(n) = raw.strip_suffix('s') {
        (n, 1000)
    } else {
        (raw, 1000)
    };
    let milliseconds = raw.parse::<u64>().ok()?.checked_mul(multiplier)?;
    (milliseconds <= 3600 * 1000).then(|| Duration::from_millis(milliseconds))
}

/// Named subcommand help. Handled before any state initialization or parameter
/// execution, so `lintel <group> help` never creates state. Each entry lists
/// only real parser/schema operations.
const GROUP_HELP: &[(&str, &str)] = &[
    ("plan", "plan show ID\n  读回原冻结计划；不生成新的计划或授权。"),
    ("browser", "browser operations | instances | pair create|pending|approve | submit | query | control\n  明确实例与有限原生操作；清理要真正重启后独立确认。"),
    ("network", "network inspect\n  network probe [--ipv4-url HTTPS_URL --ipv6-url HTTPS_URL --proxy-url LOOPBACK_HTTP --timeout-seconds 10]\n  network ipv6 plan --service-id ID --mode off|link_local [同样的探测 flags] [--baseline-id ID]\n  network restore plan --job ID [--baseline-id ID]\n  network serve --config PATH\n  inspect 离线读取宿主共享状态；probe 显式访问 https://api.ipify.org 与 https://api6.ipify.org，可指定自己的 HTTPS 回显端点。IPv6 plan 会前测，批准后自动沿用同目标复测；macOS 可能申请系统网络配置授权。submit 需精确计划/hash，恢复另外批准。serve 是前台 NDJSON owner；Ctrl-C 只关闭自己的通道。"),
    ("env", "env list | inspect ID | components ID [--project-cwd PATH] | create --name NAME | register --name NAME --root PATH\n  list/inspect/components 只读；components 不运行认证或协调原任务；create/register 只建立或登记明确的配置目标。"),
    ("policy", "policy plan --environment ID --preset reduce|preserve|custom [--keep-remote-control]\n  预览精确的 user settings 字段写入；执行需要 approve 阶段。"),
    ("work", "work inventory --environment ID --categories instructions,memory,sessions [--offset N] [--expected-digest HEX]\n  work preflight --environment ID --categories ... [--path RELATIVE_PATH ...]\n  work archive plan|list|inspect|read | preserve plan | import plan | session read\n  inventory 返回有界只读原件元数据清单（分页/摘要绑定同一次扫描）；preflight 只读元数据容量预检。archive/preserve plan 可用可重复 --path 精确选择所选类别内的原件，排除未选的大会话而不截断。session read 需要 --job ID 或 --archive-path PATH 之一、--path PATH，以及 stdin 的 {\"archive_passphrase\":\"...\"}。"),
    ("job", "job show ID | job wait ID --timeout 30s | job submit --plan ID --approval HASH\n  show/wait 只查询原任务；submit 批准原计划；wait 超时不会重新提交。"),
    ("restore", "restore plan --job ID\n  为仍属于本工具的字段准备独立恢复预览。"),
    ("remote", "remote hosts | aliases | inspect ALIAS | request ALIAS | submit ALIAS | job ALIAS PLAN_ID | launch ALIAS ENVIRONMENT_ID | control\n  remote launch request|resume ALIAS REQUEST_ID APPROVAL；remote launch query ALIAS REQUEST_ID | remote launch list ALIAS。有限 OpenSSH controller；execute 只走 submit，恢复只查询原任务。"),
    ("launch", "launch ENVIRONMENT_ID\n  launch request REQUEST_ID APPROVAL | launch resume REQUEST_ID APPROVAL\n  launch query REQUEST_ID | launch list\n  真实 TTY 直接交互、无 prompt；旧入口以 root 为 cwd，新请求冻结独立 cwd。resume 的口令在该 TTY 无回显输入；请求以不可变 request ID 解析，重复只查询。"),
    ("session", "session read\n  work session read 的别名：需要 --job ID 或 --archive-path PATH、--path PATH 与 stdin 口令。"),
];

fn group_help(group: &str) -> Value {
    GROUP_HELP
        .iter()
        .find(|(name, _)| *name == group)
        .map_or_else(
            || error("unknown_command", format!("没有子命令组 {group}")),
            |(name, text)| {
                json!({"ok":true,"data":{"group":name,"help":text,"static":true,"notes":"此帮助不初始化 state、不执行操作。"}})
            },
        )
}

/// All named nested help is resolved before TTY, stdin or state initialization.
pub fn help_request(args: &[String]) -> Option<Value> {
    let first = args.first()?.as_str();
    let help = args
        .get(1)
        .is_some_and(|s| ["help", "--help", "-h"].contains(&s.as_str()))
        || args
            .last()
            .is_some_and(|s| ["help", "--help", "-h"].contains(&s.as_str()));
    (help && GROUP_HELP.iter().any(|(group, _)| *group == first)).then(|| group_help(first))
}

pub fn run(args: &[String]) -> Value {
    if let Some(help) = help_request(args) {
        return help;
    }
    let word = |i: usize| args.get(i).map(String::as_str).unwrap_or("");
    match word(0) {
        // `--help` / `-h` anywhere as the only argument prints static help before
        // touching state or parameters.
        "--help" | "-h" => json!({"ok":true,"data":{"help":HELP,"static":true}}),
        "help" => {
            if word(1).is_empty() {
                json!({"ok":true,"data":{"help":HELP,"static":true}})
            } else {
                group_help(word(1))
            }
        }
        // Any group's trailing `help` is answered before touching state/params.
        group
            if matches!(
                args.get(1).map(String::as_str),
                Some("help" | "--help" | "-h")
            ) =>
        {
            group_help(group)
        }
        "context" | "tasks" => {
            if args[1..].iter().any(|a| a != "--json") {
                return error("invalid_argument", "该静态命令只接受 --json");
            }
            dispatch(json!({"command":word(0)}))
        }
        "version" | "capabilities" => {
            let command = word(0);
            let r = match flags(&args[1..], json!({"command":command})) {
                Ok(r) => r,
                Err(e) => return e,
            };
            if r.as_object()
                .unwrap()
                .keys()
                .any(|k| k != "command" && (command != "capabilities" || k != "environment_id"))
            {
                return error("invalid_argument", "该静态命令不接受此参数");
            }
            dispatch(r)
        }
        "describe" | "schema" => {
            if word(1).is_empty() || args[2..].iter().any(|a| a != "--json") {
                return error("invalid_argument", "需要一个 operation ID");
            }
            dispatch(json!({"command":word(0),"operation":word(1)}))
        }
        "request" => match stdin_json(true) {
            Ok(r) => dispatch(r),
            Err(e) => e,
        },
        // Top-level alias for `work session read`.
        "session" if word(1) == "read" => named("session_read", &args[2..], json!({}), true),
        // Readonly launch/resume record queries (no approval, no passphrase, no
        // Terminal). `query` finds one original request; `list` shows all.
        "launch" if word(1) == "query" && !word(2).is_empty() => named(
            "launch_query",
            &args[3..],
            json!({"request_id":word(2)}),
            false,
        ),
        "launch" if word(1) == "list" => named("launches", &args[2..], json!({}), false),
        "call" => {
            if args.len() != 2 {
                return error(
                    "invalid_argument",
                    "lintel call OPERATION 从 stdin 接收字段",
                );
            }
            named(word(1), &[], json!({}), true)
        }
        "discover" | "jobs" => named(word(0), &args[1..], json!({}), false),
        "inspect" => {
            if word(1).is_empty() {
                error("invalid_argument", "需要 environment ID")
            } else {
                named(
                    "inspect",
                    &args[2..],
                    json!({"environment_id":word(1)}),
                    false,
                )
            }
        }
        "plan" if word(1) == "show" => {
            if word(2).is_empty() {
                error("invalid_argument", "需要 plan ID")
            } else {
                named("plan_show", &args[3..], json!({"plan_id":word(2)}), false)
            }
        }
        "env" => match word(1) {
            "list" => named("discover", &args[2..], json!({}), false),
            "inspect" if !word(2).is_empty() => named(
                "inspect",
                &args[3..],
                json!({"environment_id":word(2)}),
                false,
            ),
            "components" if !word(2).is_empty() => named(
                "inspect_components",
                &args[3..],
                json!({"environment_id":word(2)}),
                false,
            ),
            "register" => named("register", &args[2..], json!({}), false),
            "create" => named("create_environment", &args[2..], json!({}), false),
            _ => error(
                "invalid_argument",
                "env list | inspect ID | components ID [--project-cwd PATH] | register | create",
            ),
        },
        "policy" if word(1) == "plan" => named("plan_policy", &args[2..], json!({}), false),
        "restore" if word(1) == "plan" => named("plan_restore", &args[2..], json!({}), false),
        "job" => match word(1) {
            "submit" => named("execute", &args[2..], json!({}), true),
            "show" if !word(2).is_empty() => {
                named("job", &args[3..], json!({"plan_id":word(2)}), false)
            }
            "wait" if !word(2).is_empty() => {
                let r = match flags(&args[3..], json!({})) {
                    Ok(r) => r,
                    Err(e) => return e,
                };
                if r.as_object().unwrap().keys().any(|k| k != "timeout") {
                    return error("invalid_argument", "wait 只接受 timeout");
                }
                match timeout(r["timeout"].as_str().unwrap_or("30s")) {
                    Some(t) => wait(word(2), t),
                    None => error("invalid_argument", "timeout 需要 0–3600s 或毫秒值"),
                }
            }
            id if !id.is_empty() => named("job", &args[2..], json!({"plan_id":id}), false),
            _ => error("invalid_argument", "job show ID | wait ID | submit"),
        },
        "work" => {
            let (command, start, input) = match (word(1), word(2), word(3)) {
                ("archive", "plan", _) => ("plan_archive", 3, false),
                ("archive", "list", _) => ("jobs", 3, false),
                ("archive", "inspect", _) => ("archive_inspect", 3, true),
                ("archive", "read", _) => ("archive_read", 3, true),
                ("preflight", _, _) => ("work_preflight", 2, false),
                ("inventory", _, _) => ("work_inventory", 2, false),
                ("session", "read", _) => ("session_read", 3, true),
                ("preserve", "plan", _) => ("plan_preserve", 3, false),
                ("import", "plan", _) => ("plan_import", 3, true),
                _ => {
                    return error(
                        "invalid_argument",
                        "work inventory|preflight --environment ID --categories ... [--path REL ...] | archive plan/list/inspect/read | session read | preserve plan | import plan",
                    )
                }
            };
            let mut response = named(command, &args[start..], json!({}), input);
            if word(2) == "list" && response["ok"] == true {
                if let Some(jobs) = response["data"]["jobs"].as_array_mut() {
                    jobs.retain(|j| j["archive_path"].is_string());
                }
            }
            response
        }
        "remote" => remote(args),
        "browser" => browser(args),
        "network" if word(1) == "inspect" => named("network_inspect", &args[2..], json!({}), false),
        "network" if word(1) == "probe" => named("network_probe", &args[2..], json!({}), true),
        "network" if word(1) == "ipv6" && word(2) == "plan" => {
            named("plan_network_ipv6", &args[3..], json!({}), true)
        }
        "network" if word(1) == "restore" && word(2) == "plan" => {
            named("plan_network_restore", &args[3..], json!({}), true)
        }
        "network" if word(1) == "serve" => {
            let r = match flags(&args[2..], json!({})) {
                Ok(r) => r,
                Err(e) => return e,
            };
            if r.as_object().unwrap().len() != 1 || !r["config"].is_string() {
                return error("invalid_argument", "network serve --config PATH");
            }
            let runtime = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(_) => return error("runtime_unavailable", "无法启动前台通道"),
            };
            match runtime.block_on(lintel_egress::serve_config(std::path::Path::new(
                r["config"].as_str().unwrap(),
            ))) {
                Ok(()) => {
                    json!({"ok":true,"data":{"event":"stopped","owner":"foreground_process"}})
                }
                Err(e) => error("network_failed", e),
            }
        }
        _ => error(
            "unknown_command",
            "使用 lintel --help 或 lintel capabilities 查看操作入口",
        ),
    }
}

fn browser(args: &[String]) -> Value {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = args;
        error(
            "browser_component_unavailable",
            "Linux runner 不包含浏览器组件；在运行 profile 的主机使用独立 host CLI",
        )
    }
    #[cfg(target_os = "macos")]
    {
        let word = |i: usize| args.get(i).map(String::as_str).unwrap_or("");
        if word(1) == "operations" {
            let r = match flags(&args[2..], json!({})) {
                Ok(r) => r,
                Err(e) => return e,
            };
            if r.as_object().unwrap().keys().any(|k| k != "instance") {
                return error("invalid_argument", "operations 只接受 --instance ID");
            }
            let mut catalog = lintel_browser_host::operation_catalog();
            if let Some(id) = r.get("instance") {
                let instances = lintel_browser_host::control(json!({"op":"instances"}));
                if instances["ok"] != true {
                    return instances;
                }
                let Some(instance) = instances["data"]
                    .as_array()
                    .and_then(|values| values.iter().find(|v| v["instance_id"] == *id))
                else {
                    return error(
                        "browser_instance_missing",
                        "指定 profile 未配对；先完成扩展加载和配对",
                    );
                };
                catalog["target"] = instance.clone();
                catalog["can_queue"] =
                    json!(instance["paired"] == true && instance["conflict"] != true);
                catalog["permissions"] = json!("not_evaluated_until_extension_confirmation");
            }
            return json!({"ok":true,"data":catalog});
        }

        let (op, start, input) =
            match (word(1), word(2)) {
                ("instances", _) => ("instances", 2, false),
                ("pair", "create") => ("pair_create", 3, false),
                ("pair", "pending") => ("pair_pending", 3, false),
                ("pair", "approve") => ("pair_approve", 3, false),
                ("submit", _) => ("submit", 2, true),
                ("query", _) => ("query", 2, true),
                ("control", _) => ("", 2, true),
                _ => return error(
                    "invalid_argument",
                    "browser instances | pair create/pending/approve | submit | query | control",
                ),
            };
        let request = if input {
            match stdin_json(true) {
                Ok(r) => r,
                Err(e) => return e,
            }
        } else {
            json!({})
        };
        let mut request = match flags(&args[start..], request) {
            Ok(r) => r,
            Err(e) => return e,
        };
        if !op.is_empty() {
            if request.get("op").is_some_and(|v| v != op) {
                return error("invalid_request", "stdin op 与命名入口不一致");
            }
            request["op"] = json!(op);
        }
        lintel_browser_host::control(request)
    }
}

/// Run one read-only remote operation with the same runner-bundle resolution as
/// the other remote CLI paths. Used by `remote launch query|list`.
fn remote_pinned(payload: Value) -> Value {
    let bundles = std::env::var_os("LINTEL_RUNNER_BUNDLES")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_exe()
                .unwrap_or_default()
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("remote-runners")
        });
    lintel_remote::control(payload, bundles)
}

fn remote(args: &[String]) -> Value {
    let word = |i: usize| args.get(i).map(String::as_str).unwrap_or("");
    if word(1) == "operations" {
        return if args[2..].iter().all(|a| a == "--json") {
            json!({"ok":true,"data":lintel_remote::operation_catalog()})
        } else {
            error("invalid_argument", "remote operations 仅接受 --json")
        };
    }
    if word(1) == "launch" {
        // `remote launch request ALIAS ID HASH` / `remote launch resume ALIAS ID HASH` reuse
        // the same finite ID; `remote launch ALIAS ENVIRONMENT_ID` keeps the
        // legacy environment launch.
        // Readonly record queries (no TTY, no approval, no Terminal).
        if word(2) == "query" && !word(3).is_empty() && !word(4).is_empty() {
            return remote_pinned(
                json!({"op":"launch_query","alias":word(3),"request_id":word(4)}),
            );
        }
        if word(2) == "list" && !word(3).is_empty() {
            return remote_pinned(json!({"op":"launches","alias":word(3)}));
        }
        if word(2) == "request" || word(2) == "resume" {
            if args.len() != 6 {
                return error(
                    "invalid_argument",
                    "remote launch request|resume ALIAS REQUEST_ID APPROVAL；不接受额外字段",
                );
            }
            if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
                return error(
                    "interactive_launch_required",
                    "请在真实交互终端使用 lintel remote launch；不从隐藏管道打开 Terminal",
                );
            }
            let op = if word(2) == "resume" {
                "resume_request"
            } else {
                "launch_request"
            };
            // Only the validated alias/request-id/approval enter the payload; a
            // resume passphrase is entered again without echo in the remote TTY.
            let payload = json!({"op":op,"alias":word(3),"request_id":word(4),"approval":word(5)});
            let bundles = std::env::var_os("LINTEL_RUNNER_BUNDLES")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| {
                    std::env::current_exe()
                        .unwrap_or_default()
                        .parent()
                        .unwrap_or(std::path::Path::new("."))
                        .join("remote-runners")
                });
            return lintel_remote::control(payload, bundles);
        }
        if args.len() != 4 {
            return error(
                "invalid_argument",
                "remote launch ALIAS ENVIRONMENT_ID；不接受 prompt 或额外字段",
            );
        }
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return error("interactive_launch_required", "请在真实交互终端使用 lintel remote launch ALIAS ENVIRONMENT_ID；不从隐藏管道打开 Terminal");
        }
    }
    let mut payload = match word(1) {
        "control" => match stdin_json(true){Ok(r)=>r,Err(e)=>return e},
        "hosts"|"aliases" => json!({"op":word(1)}),
        "inspect" if !word(2).is_empty()=>json!({"op":"connect","alias":word(2)}),
        "request" if !word(2).is_empty()=> {
            let request=match stdin_json(true){Ok(r)=>r,Err(e)=>return e};
            json!({"op":"request","alias":word(2),"request":request})
        }
        "submit" if !word(2).is_empty()=> {
            let mut request=match stdin_json(true){Ok(r)=>r,Err(e)=>return e};
            if request["command"] != "execute" {return error("invalid_submission","remote submit 只接受 execute");}
            request.as_object_mut().unwrap().remove("command");
            request["op"]=json!("execute");request["alias"]=json!(word(2));request
        }
        "job" if !word(2).is_empty()&&!word(3).is_empty()=>json!({"op":"request","alias":word(2),"request":{"command":"job","job_id":word(3)}}),
        "launch" => json!({"op":"launch","alias":word(2),"environment_id":word(3)}),
        "query" if !word(2).is_empty()&&!word(3).is_empty()=>json!({"op":"launch_query","alias":word(2),"request_id":word(3)}),
        _=>return error("invalid_argument","remote control | hosts | aliases | inspect ALIAS | request ALIAS | submit ALIAS | job ALIAS PLAN_ID | launch ALIAS ENVIRONMENT_ID"),
    };
    if word(1) != "launch" && payload["op"] == "launch" {
        return error("interactive_launch_required", "remote control 不执行交互启动；请在真实 TTY 使用 lintel remote launch ALIAS ENVIRONMENT_ID");
    }
    let start = match word(1) {
        "hosts" | "aliases" | "control" => 2,
        "job" | "launch" => 4,
        _ => 3,
    };
    let mut config = match flags(&args[start..], json!({})) {
        Ok(r) => r,
        Err(e) => return e,
    };
    let bundles = config
        .as_object_mut()
        .unwrap()
        .remove("bundles")
        .and_then(|v| v.as_str().map(std::path::PathBuf::from))
        .or_else(|| std::env::var_os("LINTEL_RUNNER_BUNDLES").map(std::path::PathBuf::from))
        .unwrap_or_else(|| {
            std::env::current_exe()
                .unwrap_or_default()
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("remote-runners")
        });
    for (key, value) in config.as_object().unwrap() {
        if payload.get(key).is_some() {
            return error("duplicate_argument", key);
        }
        payload[key] = value.clone();
    }
    lintel_remote::control(payload, bundles)
}

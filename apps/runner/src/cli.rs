//! Named agent surface over the existing core/host/egress execution owners.
use serde_json::{json, Value};
use std::{
    io::{self, IsTerminal, Read},
    time::{Duration, Instant},
};

pub const HELP: &str = "Lintel — Claude environment control\n\n  lintel version [--json]                     Static build/protocol identity\n  lintel capabilities [--environment ID]       Operation catalog; optional target inspection\n  lintel describe OPERATION [--json]           Parameters, effects, approval and recovery\n  lintel schema OPERATION                     JSON Schema envelope, no discovery\n  lintel env list | inspect ID | create --name NAME | register --name NAME --root PATH\n  lintel policy plan --environment ID --preset reduce --keep-remote-control\n  lintel plan show ID                         Read original frozen plan\n  lintel job show ID | wait ID --timeout 30s   Query only; timeout never resubmits\n  lintel job submit --plan ID --approval HASH  Durable ACK; secret fields via JSON stdin\n  lintel restore plan --job ID                Prepare an independent restoration\n  lintel work archive plan --environment ID --categories instructions,memory,sessions\n  lintel work archive list | inspect | read   Use --job ID or --archive-path PATH\n  lintel work preserve plan --environment ID --categories instructions,memory,sessions\n  lintel work import plan --environment ID --archive-path PATH --categories memory,sessions\n  lintel browser operations [--instance ID]    Static actions; optional pairing/online facts\n  lintel browser instances | pair create | pair pending | pair approve --challenge ID\n  lintel browser submit | query | control    Finite host control; JSON stdin\n  lintel network serve --config PATH          Foreground owner, NDJSON, Ctrl-C stops\n  lintel remote operations                   Static finite SSH schemas, no state\n  lintel remote hosts | aliases | inspect ALIAS\n  lintel remote request ALIAS | submit ALIAS   JSON stdin; submit only execute\n  lintel remote job ALIAS PLAN_ID             Query original; no resubmission\n  lintel remote launch ALIAS ENVIRONMENT_ID    Real TTY, macOS Terminal, no prompt\n  lintel remote control [--bundles PATH]       Shared finite SSH/installation; JSON stdin\n  lintel call OPERATION                       Core requests via JSON stdin; interactive sessions use launch ID\n\nLegacy: request, submit, discover, inspect ID, jobs, job ID, launch ID, tui.\nOrdinary commands print one ok/data or ok/error envelope. Network serve is explicitly NDJSON.\nPassphrases never belong in argv or persistent request files. No blanket --yes.\nStatic catalog commands do not initialize state. Protocol-1 request retains historical defaults.\n";

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
        if request.get(&key).is_some() {
            return Err(error("duplicate_argument", &key));
        }
        let raw = &args[i + 1];
        request[&key] = if ["categories", "release_settings"].contains(&key.as_str()) {
            json!(raw.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>())
        } else if key == "custom_settings" {
            serde_json::from_str(raw)
                .map_err(|_| error("invalid_argument", "custom-settings 需要 JSON object"))?
        } else {
            json!(raw)
        };
        i += 2;
    }
    Ok(request)
}

fn named(command: &str, args: &[String], seed: Value, input: bool) -> Value {
    if command == "launch" {
        return error(
            "interactive_launch_required",
            "请在真实交互终端使用 lintel launch <environment-id> 启动；该入口核对 stdin/stdout TTY，并且不接受 prompt",
        );
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

pub fn run(args: &[String]) -> Value {
    let word = |i: usize| args.get(i).map(String::as_str).unwrap_or("");
    match word(0) {
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
            "register" => named("register", &args[2..], json!({}), false),
            "create" => named("create_environment", &args[2..], json!({}), false),
            _ => error(
                "invalid_argument",
                "env list | inspect ID | register | create",
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
                ("preserve", "plan", _) => ("plan_preserve", 3, false),
                ("import", "plan", _) => ("plan_import", 3, true),
                _ => {
                    return error(
                        "invalid_argument",
                        "work archive plan/list/inspect/read | preserve plan | import plan",
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
        "job" if !word(2).is_empty()&&!word(3).is_empty()=>json!({"op":"reconnect","alias":word(2),"plan_id":word(3)}),
        "launch" => json!({"op":"launch","alias":word(2),"environment_id":word(3)}),
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

//! Static operation contract shared by the agent CLI and finite transports.
//! Reading this catalog never opens a home, state, target, or executable.
use serde_json::{json, Map, Value};

const SETTINGS: &[&str] = &[
    "DISABLE_TELEMETRY",
    "DISABLE_ERROR_REPORTING",
    "DISABLE_FEEDBACK_COMMAND",
    "CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY",
    "DO_NOT_TRACK",
    "DISABLE_GROWTHBOOK",
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
];

fn text() -> Value {
    json!({"type":"string","minLength":1,"maxLength":4096})
}
fn choice(values: &[&str]) -> Value {
    json!({"type":"string","enum":values})
}
fn property(name: &str) -> Value {
    match name {
        "environment_id" | "plan_id" | "job_id" => {
            json!({"type":"string","format":"uuid","minLength":1,"maxLength":4096})
        }
        "keep_remote_control" | "writers_confirmed_stopped" | "official_logout" => {
            json!({"type":"boolean"})
        }
        "categories" => {
            json!({"type":"array","items":choice(&["instructions","memory","sessions"]),"uniqueItems":true})
        }
        "trusted_devices" => choice(&["unknown", "required", "not_required"]),
        "preset" => choice(&["preserve", "reduce", "custom"]),
        "manager" => choice(&["user", "system"]),
        "unit" => {
            json!({"type":"string","pattern":"^[A-Za-z0-9][A-Za-z0-9_.@-]*\\.service$","maxLength":240})
        }
        "archive_passphrase" => {
            json!({"type":"string","minLength":12,"writeOnly":true,"description":"仅通过 stdin 临时输入；不进入 argv、日志或持久请求文件"})
        }
        "release_settings" => {
            json!({"type":"array","items":choice(&["DISABLE_TELEMETRY","DO_NOT_TRACK","DISABLE_GROWTHBOOK","CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC"]),"uniqueItems":true})
        }
        "custom_settings" => {
            let properties: Map<String, Value> = SETTINGS
                .iter()
                .map(|key| (key.to_string(), choice(&["keep", "disable", "remove"])))
                .collect();
            json!({"type":"object","properties":properties,"additionalProperties":false})
        }
        "root" | "archive_path" | "output_path" => {
            json!({"type":"string","pattern":"^/","description":"目标主机上的准确绝对路径；执行器另行复查实际文件与 ownership"})
        }
        "proxy_url" => json!({"type":"string","description":"只接受 loopback HTTP 地址"}),
        _ => text(),
    }
}

/// Each row is an existing public operation, not an arbitrary RPC namespace.
fn fields(command: &str) -> Option<(&'static [&'static str], &'static [&'static str])> {
    Some(match command {
        "discover" | "jobs" | "export_support" => (&[], &[]),
        "register" => (&["name", "root"], &[]),
        "create_environment" => (&["name"], &[]),
        "inspect" => (&["environment_id"], &["trusted_devices"]),
        "drift"
        | "accept_drift"
        | "cleanup_inspect"
        | "auth_probe"
        | "reactivate_environment"
        | "launch_context" => (&["environment_id"], &[]),
        "launch" => (&["environment_id"], &["proxy_url"]),
        "plan_policy" => (
            &["environment_id", "preset", "keep_remote_control"],
            &["trusted_devices", "release_settings", "custom_settings"],
        ),
        "plan_reset" => (&["environment_id", "recipe", "categories"], &[]),
        "plan_archive" => (&["environment_id", "categories"], &["output_path"]),
        "plan_preserve" => (&["environment_id", "categories"], &["name"]),
        "plan_cleanup" => (
            &[
                "environment_id",
                "recipe",
                "writers_confirmed_stopped",
                "official_logout",
                "categories",
            ],
            &[],
        ),
        "service_inspect" | "plan_service_quiesce" => (&["environment_id", "manager", "unit"], &[]),
        "plan_restore" | "plan_service_resume" => (&["job_id"], &[]),
        "plan_show" => (&["plan_id"], &[]),
        "job" => (&[], &["job_id", "plan_id"]),
        "archive_inspect" => (&["archive_passphrase"], &["job_id", "archive_path"]),
        "archive_read" => (&["archive_passphrase", "path"], &["job_id", "archive_path"]),
        "plan_import" => (
            &["environment_id", "categories", "archive_passphrase"],
            &["job_id", "archive_path"],
        ),
        "execute" => (&["plan_id", "approval"], &["archive_passphrase"]),
        _ => return None,
    })
}

pub const COMMANDS: &[&str] = &[
    "discover",
    "register",
    "create_environment",
    "inspect",
    "plan_policy",
    "plan_reset",
    "plan_archive",
    "plan_preserve",
    "cleanup_inspect",
    "auth_probe",
    "plan_cleanup",
    "reactivate_environment",
    "service_inspect",
    "plan_service_quiesce",
    "plan_service_resume",
    "archive_inspect",
    "archive_read",
    "plan_import",
    "plan_restore",
    "plan_show",
    "execute",
    "jobs",
    "job",
    "drift",
    "accept_drift",
    "launch_context",
    "launch",
    "export_support",
];

pub fn schema(command: &str) -> Option<Value> {
    let (required, optional) = fields(command)?;
    let mut properties = Map::new();
    properties.insert("command".into(), json!({"const":command,"type":"string"}));
    for key in required.iter().chain(optional.iter()) {
        properties.insert(key.to_string(), property(key));
    }
    if command == "plan_reset" {
        properties.insert("recipe".into(), choice(&["rebuild"]));
    }
    if command == "plan_cleanup" {
        properties.insert(
            "recipe".into(),
            choice(&["repair_login", "reset_client", "retire"]),
        );
    }
    let mut required = required.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    required.insert(0, "command".into());
    let mut value = json!({"$schema":"https://json-schema.org/draft/2020-12/schema","title":command,"type":"object","properties":properties,"required":required,"additionalProperties":false});
    if ["archive_inspect", "archive_read", "plan_import", "job"].contains(&command) {
        let alternate = if command == "job" {
            "plan_id"
        } else {
            "archive_path"
        };
        value["oneOf"] = json!([{"required":["job_id"],"not":{"required":[alternate]}},{"required":[alternate],"not":{"required":["job_id"]}}]);
    }
    if command == "plan_policy" {
        value["allOf"] = json!([{"if":{"properties":{"preset":{"const":"custom"}}},"then":{"properties":{"release_settings":{"maxItems":0}}},"else":{"not":{"required":["custom_settings"]}}}]);
    }
    Some(value)
}

fn check_property(value: &Value, schema: &Value) -> bool {
    let typed = match schema["type"].as_str() {
        Some("string") => value.as_str().is_some_and(|s| {
            s.chars().count() >= schema["minLength"].as_u64().unwrap_or(0) as usize
                && s.chars().count() <= schema["maxLength"].as_u64().unwrap_or(u64::MAX) as usize
        }),
        Some("boolean") => value.is_boolean(),
        Some("array") => value.as_array().is_some_and(|items| {
            items.iter().all(|v| check_property(v, &schema["items"]))
                && (schema["uniqueItems"] != true
                    || items
                        .iter()
                        .enumerate()
                        .all(|(i, v)| !items[..i].contains(v)))
        }),
        Some("object") => value.as_object().is_some_and(|object| {
            object.iter().all(|(key, v)| {
                schema["properties"]
                    .get(key)
                    .is_some_and(|p| check_property(v, p))
            })
        }),
        _ => false,
    };
    typed
        && schema
            .get("enum")
            .is_none_or(|values| values.as_array().unwrap().contains(value))
}

/// Named/finite transports use this strict contract. Protocol-1 raw core callers
/// retain their historical defaults; schema does not silently rewrite them.
pub fn validate(request: &Value) -> Result<(), String> {
    let command = request["command"].as_str().ok_or("缺少 command")?;
    let s = schema(command).ok_or_else(|| format!("未支持的 operation: {command}"))?;
    let object = request.as_object().ok_or("请求必须是 JSON object")?;
    for key in s["required"].as_array().unwrap() {
        if !object.contains_key(key.as_str().unwrap()) {
            return Err(format!("缺少 {}", key.as_str().unwrap()));
        }
    }
    for (key, value) in object {
        let prop = s["properties"]
            .get(key)
            .ok_or_else(|| format!("未支持的字段 {key}"))?;
        if !check_property(value, prop) {
            return Err(format!("字段 {key} 的类型或取值无效"));
        }
        if ["root", "archive_path", "output_path"].contains(&key.as_str())
            && !value.as_str().unwrap().starts_with('/')
        {
            return Err(format!("{key} 需要绝对路径"));
        }
    }
    if let Some(branches) = s["oneOf"].as_array() {
        let satisfied = branches
            .iter()
            .filter(|branch| {
                branch["required"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|k| object.contains_key(k.as_str().unwrap()))
                    && !branch["not"]["required"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|k| object.contains_key(k.as_str().unwrap()))
            })
            .count();
        if satisfied != 1 {
            return Err("需要且只允许一个 job_id 或替代来源 ID/path".into());
        }
    }
    if command == "plan_policy" {
        if request.get("custom_settings").is_some() && request["preset"] != "custom" {
            return Err("非 custom 方案不接受 custom_settings".into());
        }
        if request["preset"] == "custom"
            && request["release_settings"]
                .as_array()
                .is_some_and(|a| !a.is_empty())
        {
            return Err("custom 不接受非空 release_settings".into());
        }
    }
    Ok(())
}

pub fn describe(command: &str) -> Option<Value> {
    let s = schema(command)?;
    let planning = command.starts_with("plan_") && command != "plan_show";
    let target = match command {
        "execute" => "frozen_plan_scope",
        "plan_archive" | "plan_preserve" | "plan_reset" | "plan_cleanup" => {
            "read_selected_work_for_frozen_plan"
        }
        "create_environment" => "create_config_root",
        "launch" => "start_target_process",
        "auth_probe" => "official_auth_status_process",
        "archive_read" => "read_work_text",
        "archive_inspect" | "plan_import" => "read_encrypted_work",
        _ => "inspect_or_no_target_write",
    };
    let state = match command {
        "discover" | "register" | "create_environment" | "reactivate_environment" => {
            "update_inventory"
        }
        "job" | "jobs" => "read_journal_may_reconcile_interrupted_job",
        "accept_drift" => "write_baseline",
        "execute" => "persist_receipt_and_recovery",
        _ if planning => "persist_frozen_plan",
        _ => "state_directory_and_operation_lock",
    };
    let transports = match command {
        "launch" | "launch_context" => vec!["core_json", "runner_json", "named_cli"],
        "execute" => vec!["core_json", "runner_json", "named_cli", "finite_ssh_submit"],
        _ => vec![
            "core_json",
            "runner_json",
            "named_cli",
            "finite_ssh_request",
        ],
    };
    Some(json!({
        "id":command,"protocol":1,"implementation":"implemented","transports":transports,
        "platforms":if command.contains("service") {vec!["linux"]} else {vec!["macos","linux"]},
        "target_conditions":if command.contains("service") {vec!["exact_root_bound_systemd_unit","user_manager_or_current_root"]} else {vec!["explicit_registered_environment_or_original_task_where_required"]},
        "applicability":{"status":"not_evaluated","reason":"静态描述不检查个人环境；运行相应 inspect 取得目标事实"},
        "effects":{"target":target,"lintel_state":state,"external":match command {"auth_probe"=>"official_auth_status_process","launch"=>"start_target_process","execute"=>"exact_plan_actions_may_include_official_logout",_=>"none"}},
        "requires_plan":command=="execute","approval":if command=="execute" {"exact_plan_hash_with_existing_user_authority"} else {"operation_specific_explicit_request"},
        "secret_fields":if s["properties"].get("archive_passphrase").is_some() {vec!["archive_passphrase"]} else {vec![]},
        "request_schema":s,
        "result":{"envelope":"ok/data or ok/error(code,message)","receipt_states":["accepted","executing","verifying","completed","partially_completed","needs_reconciliation","interrupted"],"recovery":["query_original","repreview","resolve_conflict"]},
        "launch_conditions":if command=="launch" {json!({"named_cli":"real_TTY_required_no_prompt","named_cli_entry":"lintel launch <environment-id>","core_json":"macos_Terminal_only","finite_ssh":"use_separate_remote_launch_control_macos_client"})}else{Value::Null},
        "compatibility":"协议 1 的 raw core request 保留历史可选字段/default；named CLI 与有限 SSH 使用明确字段合同"
    }))
}

pub fn catalog() -> Value {
    json!({"protocol":1,"catalog_version":1,"operations":COMMANDS.iter().filter_map(|c|describe(c)).collect::<Vec<_>>(),
    "adapters":[
        {"id":"browser","transport":"macos_browser_host_control","execution_owner":"paired_extension_native_browser_api","conditions":["component_available","profile_paired","extension_permission_and_approval"],"human_steps":["load_extension","pair_profile","approve_in_browser","full_restart_then_finishClear"]},
        {"id":"network","transport":"foreground_egress_ndjson","execution_owner":"calling_process","coverage":"proxy_connections_only","gui_owner":"desktop_app_process","shared_gui_control":false},
        {"id":"remote","transport":"finite_openssh","execution_owner":"bound_remote_runner","conditions":["registered_literal_alias","strict_host_key","runner_available"],"recovery":"query_original"}
    ]})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn schemas_validate_exact_sources_and_secret_fields() {
        assert!(validate(
            &json!({"command":"plan_policy","environment_id":"synthetic","preset":"reduce"})
        )
        .is_err());
        assert!(validate(&json!({"command":"job","job_id":"a","plan_id":"b"})).is_err());
        assert!(validate(&json!({"command":"archive_inspect","archive_path":"/tmp/synthetic.age","archive_passphrase":"synthetic-only"})).is_ok());
        assert!(validate(
            &json!({"command":"plan_preserve","environment_id":"a","categories":["credentials"]})
        )
        .is_err());
        assert_eq!(
            schema("archive_read").unwrap()["properties"]["archive_passphrase"]["writeOnly"],
            true
        );
        for command in COMMANDS {
            assert!(schema(command).is_some());
        }
    }
}

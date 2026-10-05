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
pub fn plan_hash_schema() -> Value {
    json!({"type":"string","pattern":"^[a-f0-9]{64}$","minLength":64,"maxLength":64,"writeOnly":true})
}
pub fn valid_plan_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn property(name: &str) -> Value {
    match name {
        "approval" => plan_hash_schema(),
        "environment_id" | "plan_id" | "job_id" => {
            json!({"type":"string","format":"uuid","minLength":1,"maxLength":4096})
        }
        "keep_remote_control" | "writers_confirmed_stopped" | "official_logout" => {
            json!({"type":"boolean"})
        }
        "categories" => {
            json!({"type":"array","items":choice(&["instructions","memory","sessions"]),"minItems":1,"uniqueItems":true})
        }
        "trusted_devices" => choice(&["unknown", "required", "not_required"]),
        "preset" => choice(&["preserve", "reduce", "custom"]),
        "manager" => choice(&["user", "system"]),
        "unit" => {
            json!({"type":"string","pattern":"^[A-Za-z0-9][A-Za-z0-9_.@-]*\\.service$","not":{"pattern":"@\\.service$|[^A-Za-z0-9_.@-]"},"maxLength":240})
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
        // Independent per-category activation choices for a migration. Absent
        // keys keep their legacy default; this is a finite object, not a free map.
        "activate" => {
            let properties: Map<String, Value> = ["instructions", "memory", "sessions"]
                .iter()
                .map(|key| (key.to_string(), json!({"type":"boolean"})))
                .collect();
            json!({"type":"object","properties":properties,"additionalProperties":false})
        }
        "mode" => choice(&["interactive"]),
        "input_reference" => {
            json!({"type":"object","properties":{"files":{"type":"array","maxItems":256,"items":{
                "type":"object","properties":{
                    "path":text(),"digest":json!({"type":"string","pattern":"^[a-f0-9]{64}$"}),
                    "package_digest":json!({"type":"string","pattern":"^[a-f0-9]{64}$"}),
                    "index":json!({"type":"integer","minimum":0})},
                "required":["path","digest"],"additionalProperties":false}}},"additionalProperties":false,"description":"有界来源元数据，仅记录路径与摘要；不含正文或秘密"})
        }
        "request_id" => json!({"type":"string","format":"uuid","minLength":1,"maxLength":4096}),
        "offset" => json!({"type":"integer","minimum":0,"maximum":9007199254740991i64}),
        "expected_digest" => json!({"type":"string","pattern":"^[a-f0-9]{64}$","minLength":64,"maxLength":64}),
        "root" | "archive_path" | "output_path" => {
            json!({"type":"string","pattern":"^/","description":"目标主机上的准确绝对路径；执行器另行复查实际文件与 ownership"})
        }
        "project_cwd" => {
            json!({"type":"string","pattern":"^/","description":"目标主机上独立于配置 root 的项目工作目录；执行端复查存在、可访问与身份，不为启动创建或修改"})
        }
        "proxy_url" => json!({"type":"string","description":"只接受 loopback HTTP 地址"}),
        _ => text(),
    }
}

/// Shared finite service-name contract; the core still checks service identity,
/// ownership and current systemd facts independently of input syntax.
pub fn valid_service_unit(unit: &str) -> bool {
    unit.len() <= 240
        && unit.ends_with(".service")
        && !unit.ends_with("@.service")
        && unit
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && unit
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.@-".contains(&c))
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
        "plan_preserve" => (&["environment_id", "categories"], &["name", "activate"]),
        "plan_cleanup" => (
            &[
                "environment_id",
                "recipe",
                "writers_confirmed_stopped",
                "official_logout",
            ],
            &["categories", "activate"],
        ),
        "plan_launch" => (&["environment_id", "project_cwd", "mode"], &["input_reference"]),
        "launch_request" => (&["request_id", "approval"], &[]),
        "resume_request" => (&["request_id", "approval"], &["archive_passphrase"]),
        "launch_query" => (&["request_id"], &[]),
        "launches" => (&[], &[]),
        "plan_resume" => (
            &["environment_id", "project_cwd", "path", "archive_passphrase"],
            &["job_id", "archive_path"],
        ),
        "service_inspect" | "plan_service_quiesce" => (&["environment_id", "manager", "unit"], &[]),
        "plan_restore" | "plan_service_resume" => (&["job_id"], &[]),
        "plan_show" => (&["plan_id"], &[]),
        "job" => (&[], &["job_id", "plan_id"]),
        "archive_inspect" => (&["archive_passphrase"], &["job_id", "archive_path"]),
        "archive_read" => (&["archive_passphrase", "path"], &["job_id", "archive_path"]),
        "session_read" => (
            &["archive_passphrase", "path"],
            &["job_id", "archive_path", "offset", "expected_digest"],
        ),
        "plan_import" => (
            &["environment_id", "categories", "archive_passphrase"],
            &["job_id", "archive_path", "activate"],
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
    "session_read",
    "plan_import",
    "plan_launch",
    "launch_request",
    "plan_resume",
    "resume_request",
    "launch_query",
    "launches",
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
        properties
            .get_mut("categories")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("minItems");
    }
    let mut required = required.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    required.insert(0, "command".into());
    let mut value = json!({"$schema":"https://json-schema.org/draft/2020-12/schema","title":command,"type":"object","properties":properties,"required":required,"additionalProperties":false});
    if ["archive_inspect", "archive_read", "session_read", "plan_import", "plan_resume", "job"]
        .contains(&command)
    {
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
    if command == "plan_cleanup" {
        value["allOf"] = json!([{"if":{"properties":{"recipe":{"const":"repair_login"}}},"then":{},"else":{"required":["categories"],"properties":{"categories":{"minItems":1}}}}]);
    }
    if ["plan_launch", "launch_request", "plan_resume"].contains(&command) {
        // These mutate nothing by themselves except launch (which is interactive
        // and no-prompt); validation still enforces the finite fields.
        value["x-lintel-interactive"] = json!(command == "launch_request");
    }
    Some(value)
}

pub fn valid_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if [8, 13, 18, 23].contains(&index) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

fn check_property(value: &Value, schema: &Value) -> bool {
    let typed = match schema["type"].as_str() {
        Some("string") => value.as_str().is_some_and(|s| {
            s.chars().count() >= schema["minLength"].as_u64().unwrap_or(0) as usize
                && s.chars().count() <= schema["maxLength"].as_u64().unwrap_or(u64::MAX) as usize
                && (schema["format"] != "uuid" || valid_uuid(s))
        }),
        Some("boolean") => value.is_boolean(),
        Some("array") => value.as_array().is_some_and(|items| {
            items.len() >= schema["minItems"].as_u64().unwrap_or(0) as usize
                && items.iter().all(|v| check_property(v, &schema["items"]))
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
        Some("integer") => value.as_u64().is_some(),
        _ => false,
    };
    typed
        && schema
            .get("enum")
            .is_none_or(|values| values.as_array().unwrap().contains(value))
        && schema
            .get("minimum")
            .is_none_or(|min| value.as_i64().is_some_and(|v| v >= min.as_i64().unwrap()))
        && schema.get("pattern").is_none_or(|pattern| {
            let pattern = pattern.as_str().unwrap();
            // The only patterns used here are the fixed plan-hash shape; keep the
            // check minimal and exact rather than pulling in a regex engine.
            if pattern == "^[a-f0-9]{64}$" {
                value
                    .as_str()
                    .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()))
            } else {
                value.as_str().is_some()
            }
        })
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
        if key == "unit" && !valid_service_unit(value.as_str().unwrap()) {
            return Err("unit 需要完整 service 名称；不接受 template、路径、glob 或命令".into());
        }
        if key == "approval" && !valid_plan_hash(value.as_str().unwrap()) {
            return Err("approval 需要原计划返回的 64 位小写 hex hash".into());
        }
        if ["root", "archive_path", "output_path", "project_cwd"].contains(&key.as_str())
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
    if command == "plan_cleanup"
        && request["recipe"] != "repair_login"
        && !request["categories"]
            .as_array()
            .is_some_and(|categories| !categories.is_empty())
    {
        return Err("reset_client/retire 需要至少一种工作类别".into());
    }
    // Activation is only meaningful for the instruction position this round.
    // memory/sessions cannot be registered as active native sessions; asking to
    // do so is an explicit unsupported request, not a silent drop.
    if let Some(activate) = request.get("activate").and_then(Value::as_object) {
        for key in ["memory", "sessions"] {
            if activate.get(key).and_then(Value::as_bool) == Some(true) {
                return Err("memory/sessions 本轮没有可注册的活动会话位置；activate 只支持 instructions".into());
            }
        }
    }
    Ok(())
}

pub fn describe(command: &str) -> Option<Value> {
    let s = schema(command)?;
    let planning = command.starts_with("plan_") && command != "plan_show";
    let target = match command {
        "execute" => "frozen_plan_scope",
        "launch_request" => "start_target_process",
        "launch_query" | "launches" => "readonly_launch_records",
        "plan_archive" | "plan_preserve" | "plan_reset" | "plan_cleanup" => {
            "read_selected_work_for_frozen_plan"
        }
        "plan_launch" | "plan_resume" => "read_target_for_frozen_launch",
        "create_environment" => "create_config_root",
        "launch" => "start_target_process",
        "auth_probe" => "official_auth_status_process",
        "archive_read" | "session_read" => "read_work_text",
        "archive_inspect" | "plan_import" => "read_encrypted_work",
        _ => "inspect_or_no_target_write",
    };
    let state = match command {
        "discover" | "register" | "create_environment" | "reactivate_environment" => {
            "update_inventory"
        }
        "job" | "jobs" => "read_journal_may_reconcile_interrupted_job",
        "launch_query" | "launches" => "read_launch_records_only",
        "accept_drift" => "write_baseline",
        "execute" => "persist_receipt_and_recovery",
        _ if planning => "persist_frozen_plan",
        _ => "state_directory_and_operation_lock",
    };
    let transports = match command {
        "launch" | "launch_context" | "plan_launch" | "launch_request" | "plan_resume" => {
            vec!["core_json", "runner_json", "named_cli"]
        }
        "execute" => vec!["core_json", "runner_json", "named_cli", "finite_ssh_submit"],
        _ => vec![
            "core_json",
            "runner_json",
            "named_cli",
            "finite_ssh_request",
        ],
    };
    let launch_conditions = match command {
        "launch" => json!({"named_cli":"real_TTY_required_no_prompt","named_cli_entry":"lintel launch <environment-id>","core_json":"macos_Terminal_only","finite_ssh":"use_separate_remote_launch_control_macos_client"}),
        "plan_launch" | "launch_request" | "plan_resume" => json!({"named_cli":"real_TTY_required_no_prompt","core_json":"macos_Terminal_only","finite_ssh":"bound_runner_request_id_only","prompt":false,"queue":false}),
        _ => Value::Null,
    };
    Some(json!({
        "id":command,"protocol":1,"implementation":"implemented","transports":transports,
        "platforms":if command.contains("service") {vec!["linux"]} else {vec!["macos","linux"]},
        "target_conditions":if command.contains("service") {vec!["exact_root_bound_systemd_unit","user_manager_or_current_root"]} else {vec!["explicit_registered_environment_or_original_task_where_required"]},
        "applicability":{"status":"not_evaluated","reason":"静态描述不检查个人环境；运行相应 inspect 取得目标事实"},
        "effects":{"target":target,"lintel_state":state,"external":match command {"auth_probe"=>"official_auth_status_process","launch"=>"start_target_process","launch_request"=>"start_target_process","execute"=>"exact_plan_actions_may_include_official_logout",_=>"none"}},
        "requires_plan":command=="execute","approval":if command=="execute" {"exact_plan_hash_with_existing_user_authority"} else {"operation_specific_explicit_request"},
        "secret_fields":if s["properties"].get("archive_passphrase").is_some() {vec!["archive_passphrase"]} else {vec![]},
        "request_schema":s,
        "result":{"envelope":"ok/data or ok/error(code,message)","receipt_states":["accepted","executing","verifying","completed","partially_completed","needs_reconciliation","interrupted"],"recovery":["query_original","repreview","resolve_conflict"]},
        "launch_conditions":launch_conditions,
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

/// The one maintained finite task map, embedded at compile time from the single
/// source `contracts/task-catalog.json` that the App help and website also import.
/// Metadata only: no home/state/executable is read. Each task names its stable
/// ID, zh label, App route, help anchor, related operations and runnable CLI
/// examples; `task_catalog_matches_schema` asserts the file parses and is finite.
const TASK_CATALOG_JSON: &str = include_str!("../../../contracts/task-catalog.json");

pub fn tasks() -> Value {
    serde_json::from_str(TASK_CATALOG_JSON).expect("embedded task catalog is valid JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_ids_match_the_published_uuid_format() {
        for (command, field) in [
            ("inspect", "environment_id"),
            ("plan_show", "plan_id"),
            ("job", "job_id"),
        ] {
            assert_eq!(
                schema(command).unwrap()["properties"][field]["format"],
                "uuid"
            );
            for id in [
                "00000000-0000-4000-8000-000000000001",
                "ABCDEF01-2345-4678-9ABC-DEF012345678",
            ] {
                assert!(
                    validate(&json!({"command":command,field:id})).is_ok(),
                    "{field}: {id}"
                );
            }
            for id in [
                "synthetic",
                "00000000000040008000000000000001",
                "00000000-0000-4000-8000-00000000000g",
                "00000000_0000-4000-8000-000000000001",
                "00000000-0000-4000-8000-000000000001\n",
            ] {
                assert!(
                    validate(&json!({"command":command,field:id})).is_err(),
                    "{field}: {id}"
                );
            }
        }
    }
    #[test]
    fn service_units_exclude_templates_paths_and_commands() {
        let unit_schema = &schema("service_inspect").unwrap()["properties"]["unit"];
        assert_eq!(
            unit_schema["not"]["pattern"],
            r"@\.service$|[^A-Za-z0-9_.@-]"
        );
        for command in ["service_inspect", "plan_service_quiesce"] {
            for unit in [
                "claude.service",
                "claude@synthetic.service",
                "a-b_1.service",
            ] {
                assert!(validate(&json!({"command":command,"environment_id":"00000000-0000-4000-8000-000000000001","manager":"user","unit":unit})).is_ok(), "{unit}");
            }
            for unit in [
                "claude@.service",
                "/tmp/claude.service",
                "*.service",
                "claude.service\n",
                "claude.socket",
                "claude;echo.service",
                "猫.service",
            ] {
                assert!(validate(&json!({"command":command,"environment_id":"00000000-0000-4000-8000-000000000001","manager":"user","unit":unit})).is_err(), "{unit}");
            }
            assert!(validate(&json!({"command":command,"environment_id":"00000000-0000-4000-8000-000000000001","manager":"user","unit":format!("{}.service", "a".repeat(240))})).is_err());
        }
    }
    #[test]
    fn work_selection_requires_at_least_one_category() {
        assert_eq!(
            schema("plan_archive").unwrap()["properties"]["categories"]["minItems"],
            1
        );
        for command in ["plan_archive", "plan_preserve"] {
            assert!(validate(
                &json!({"command":command,"environment_id":"00000000-0000-4000-8000-000000000001","categories":[]})
            )
            .is_err());
            assert!(validate(&json!({"command":command,"environment_id":"00000000-0000-4000-8000-000000000001","categories":["instructions"]})).is_ok());
        }
        assert!(validate(&json!({"command":"plan_policy","environment_id":"00000000-0000-4000-8000-000000000001","preset":"reduce","keep_remote_control":false,"release_settings":[]})).is_ok());
    }
    #[test]
    fn execute_approval_schema_matches_plan_hash_shape() {
        let mut request = json!({"command":"execute","plan_id":"00000000-0000-4000-8000-000000000002","approval":"x"});
        assert!(
            validate(&request).is_err(),
            "Malformed plan hash passed named validation"
        );
        for approval in [
            "a".repeat(63),
            "a".repeat(65),
            "A".repeat(64),
            "g".repeat(64),
        ] {
            request["approval"] = json!(approval);
            assert!(validate(&request).is_err());
        }
        request["approval"] = json!("a".repeat(64));
        assert!(validate(&request).is_ok());
        let approval = schema("execute").unwrap()["properties"]["approval"].clone();
        assert_eq!(approval["pattern"], "^[a-f0-9]{64}$");
        assert_eq!(approval["minLength"], 64);
        assert_eq!(approval["maxLength"], 64);
    }

    #[test]
    fn cleanup_work_selection_follows_the_recipe() {
        let base = json!({"command":"plan_cleanup","environment_id":"00000000-0000-4000-8000-000000000001","recipe":"repair_login","writers_confirmed_stopped":true,"official_logout":false});
        assert!(
            validate(&base).is_ok(),
            "Category-free repair-login is rejected"
        );
        let mut request = base.clone();
        request["categories"] = json!([]);
        assert!(validate(&request).is_ok());
        for recipe in ["reset_client", "retire"] {
            request["recipe"] = json!(recipe);
            assert!(validate(&request).is_err());
            request.as_object_mut().unwrap().remove("categories");
            assert!(validate(&request).is_err());
            request["categories"] = json!(["instructions"]);
            assert!(validate(&request).is_ok());
            request["categories"] = json!([]);
        }
        let schema = schema("plan_cleanup").unwrap();
        assert!(!schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("categories")));
        assert!(schema["properties"]["categories"]["minItems"].is_null());
        assert_eq!(
            schema["allOf"][0]["else"]["required"],
            json!(["categories"])
        );
        assert_eq!(
            schema["allOf"][0]["else"]["properties"]["categories"]["minItems"],
            1
        );
    }
    #[test]
    fn schemas_validate_exact_sources_and_secret_fields() {
        assert!(validate(
            &json!({"command":"plan_policy","environment_id":"00000000-0000-4000-8000-000000000001","preset":"reduce"})
        )
        .is_err());
        assert!(validate(&json!({"command":"job","job_id":"00000000-0000-4000-8000-000000000003","plan_id":"00000000-0000-4000-8000-000000000002"})).is_err());
        assert!(validate(&json!({"command":"archive_inspect","archive_path":"/tmp/synthetic.age","archive_passphrase":"synthetic-only"})).is_ok());
        assert!(validate(
            &json!({"command":"plan_preserve","environment_id":"00000000-0000-4000-8000-000000000001","categories":["credentials"]})
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

    #[test]
    fn session_read_is_bounded_and_exact_source() {
        let ok = json!({"command":"session_read","archive_path":"/tmp/synthetic.age","archive_passphrase":"synthetic-only","path":"projects/p/s.jsonl"});
        assert!(validate(&ok).is_ok(), "{ok}");
        // Exactly one source; both or neither is rejected.
        assert!(validate(&json!({"command":"session_read","job_id":"00000000-0000-4000-8000-000000000001","archive_path":"/tmp/a.age","archive_passphrase":"synthetic-only","path":"x"})).is_err());
        assert!(validate(&json!({"command":"session_read","archive_passphrase":"synthetic-only","path":"x"})).is_err());
        // offset bounded integer; digest lowercase hex only.
        assert!(validate(&json!({"command":"session_read","archive_path":"/tmp/a.age","archive_passphrase":"synthetic-only","path":"x","offset":-1})).is_err());
        assert!(validate(&json!({"command":"session_read","archive_path":"/tmp/a.age","archive_passphrase":"synthetic-only","path":"x","offset":10})).is_ok());
        assert!(validate(&json!({"command":"session_read","archive_path":"/tmp/a.age","archive_passphrase":"synthetic-only","path":"x","expected_digest":"A".repeat(64)})).is_err());
        assert!(validate(&json!({"command":"session_read","archive_path":"/tmp/a.age","archive_passphrase":"synthetic-only","path":"x","expected_digest":"a".repeat(64)})).is_ok());
        assert_eq!(schema("session_read").unwrap()["properties"]["archive_passphrase"]["writeOnly"], true);
    }

    #[test]
    fn launch_and_resume_are_finite() {
        let base = json!({"command":"plan_launch","environment_id":"00000000-0000-4000-8000-000000000001","project_cwd":"/tmp/project","mode":"interactive"});
        assert!(validate(&base).is_ok(), "{base}");
        assert!(validate(&json!({"command":"plan_launch","environment_id":"00000000-0000-4000-8000-000000000001","project_cwd":"tmp/relative","mode":"interactive"})).is_err());
        assert!(validate(&json!({"command":"plan_launch","environment_id":"00000000-0000-4000-8000-000000000001","project_cwd":"/tmp/project","mode":"shell"})).is_err());
        let reference = json!({"files":[{"path":"projects/p/s.jsonl","digest":"a".repeat(64),"package_digest":"b".repeat(64),"index":0}]});
        assert!(validate(&json!({"command":"plan_launch","environment_id":"00000000-0000-4000-8000-000000000001","project_cwd":"/tmp/project","mode":"interactive","input_reference":reference})).is_ok());
        assert!(validate(&json!({"command":"plan_launch","environment_id":"00000000-0000-4000-8000-000000000001","project_cwd":"/tmp/project","mode":"interactive","input_reference":{"body":"secret"}})).is_err());
        assert!(validate(&json!({"command":"launch_request","request_id":"00000000-0000-4000-8000-000000000001","approval":"a".repeat(64)})).is_ok());
        assert!(validate(&json!({"command":"launch_request","request_id":"not-a-uuid","approval":"a".repeat(64)})).is_err());
        assert!(validate(&json!({"command":"plan_resume","environment_id":"00000000-0000-4000-8000-000000000001","project_cwd":"/tmp/project","job_id":"00000000-0000-4000-8000-000000000002","archive_passphrase":"synthetic-only","path":"p/s.jsonl"})).is_ok());
        assert_eq!(schema("launch_request").unwrap()["x-lintel-interactive"], json!(true));
    }

    #[test]
    fn activation_is_finite_and_rejects_unavailable_positions() {
        let base = json!({"command":"plan_preserve","environment_id":"00000000-0000-4000-8000-000000000001","categories":["instructions"],"activate":{"instructions":true}});
        assert!(validate(&base).is_ok(), "{base}");
        assert!(validate(&json!({"command":"plan_preserve","environment_id":"00000000-0000-4000-8000-000000000001","categories":["instructions"],"activate":{"instructions":"yes"}})).is_err());
        assert!(validate(&json!({"command":"plan_preserve","environment_id":"00000000-0000-4000-8000-000000000001","categories":["instructions"],"activate":{"credentials":true}})).is_err());
        let bad = json!({"command":"plan_preserve","environment_id":"00000000-0000-4000-8000-000000000001","categories":["sessions"],"activate":{"sessions":true}});
        assert!(validate(&bad).is_err(), "activating an unavailable session position must be refused");
    }

    #[test]
    fn task_catalog_is_one_finite_map_of_six_tasks() {
        let catalog = tasks();
        assert_eq!(catalog["schema"], "lintel.tasks/1");
        assert_eq!(catalog["version"], 1);
        let list = catalog["tasks"].as_array().unwrap();
        assert_eq!(list.len(), 6);
        let ids: Vec<&str> = list.iter().map(|t| t["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["reduce_egress", "preserve_work", "repair_cleanup_retire", "browser_profile", "ssh_remote", "recover_results"]);
        // Every referenced operation must be a real operation this catalog owns.
        for task in list {
            for op in task["operations"].as_array().unwrap() {
                assert!(schema(op.as_str().unwrap()).is_some(), "task references unknown operation {op}");
            }
            assert!(task["route"].as_str().unwrap().starts_with('#'), "route");
            assert!(!task["help_anchor"].as_str().unwrap().is_empty(), "help_anchor");
            assert!(!task["label"].as_str().unwrap().is_empty(), "label");
            assert!(!task["summary"].as_str().unwrap().is_empty(), "summary");
            // Every CLI example must invoke `lintel` with a real top-level grammar.
            for example in task["cli_examples"].as_array().unwrap() {
                let example = example.as_str().unwrap();
                assert!(example.contains("lintel "), "cli example grammar: {example}");
            }
        }
    }
}

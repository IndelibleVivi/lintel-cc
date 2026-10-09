//! Pure metadata for the same finite operations accepted by the controller.
use super::{MAX_JSON, MAX_STDERR, REQUEST_COMMANDS};
use serde_json::{json, Map, Value};

const OPERATIONS: &[&str] = &[
    "aliases",
    "hosts",
    "add_host",
    "remove_host",
    "connect",
    "request",
    "execute",
    "reconnect",
    "prepare_runner",
    "install_runner",
    "query_install",
    "launch",
    "launch_request",
    "resume_request",
    "launch_query",
    "launches",
];

// Field ownership is shared by runtime exact-field checks and this catalog.
pub(super) fn operation_fields(
    op: &str,
) -> Option<(&'static [&'static str], &'static [&'static str])> {
    Some(match op {
        "aliases" | "hosts" => (&["op"], &[]),
        "add_host" | "remove_host" | "connect" | "prepare_runner" => (&["op", "alias"], &[]),
        "request" => (&["op", "alias", "request"], &[]),
        "execute" => (
            &["op", "alias", "plan_id", "approval"],
            &["archive_passphrase"],
        ),
        "reconnect" => (&["op", "alias", "plan_id"], &[]),
        "install_runner" => (&["op", "alias", "install_id", "approval"], &[]),
        "query_install" => (&["op", "alias", "install_id"], &[]),
        "launch" => (&["op", "alias", "environment_id"], &[]),
        "launch_request" => (&["op", "alias", "request_id", "approval"], &[]),
        // The archive passphrase is entered in the interactive Terminal (no-echo), never carried in the finite request fields.
        "resume_request" => (&["op", "alias", "request_id", "approval"], &[]),
        "launch_query" => (&["op", "alias", "request_id"], &[]),
        "launches" => (&["op", "alias"], &[]),
        _ => return None,
    })
}

fn request_schema() -> Value {
    let branches = REQUEST_COMMANDS
        .iter()
        .map(|command| {
            let mut schema = lintel_operations::schema(command).unwrap();
            // Runtime bounds UTF-8 bytes as well as shared field types. JSON
            // Schema lengths count characters; record the byte limit explicitly.
            for property in schema["properties"].as_object_mut().unwrap().values_mut() {
                if property["type"] == "string" {
                    let min = property["minLength"].as_u64().unwrap_or(1).max(1);
                    property["minLength"] = json!(min);
                    let max = property["maxLength"].as_u64().unwrap_or(4096).min(4096);
                    property["maxLength"] = json!(max);
                    property["x-maxUtf8Bytes"] = json!(4096);
                }
            }
            if ["service_inspect", "plan_service_quiesce"].contains(command) {
                schema["properties"]["unit"]["not"] =
                    json!({"anyOf":[{"pattern":"@\\.service$"},{"pattern":"[^A-Za-z0-9_.@-]"}]});
            }
            schema
        })
        .collect::<Vec<_>>();
    json!({"oneOf":branches,"x-maxEncodedBytesIncludingNewline":MAX_JSON})
}

fn property(op: &str, field: &str) -> Value {
    match field {
        "op" => json!({"type":"string","const":op}),
        "alias" => {
            json!({"type":"string","pattern":"^[A-Za-z0-9][A-Za-z0-9._-]*$","not":{"pattern":"[^A-Za-z0-9._-]"},"maxLength":128})
        }
        "plan_id" | "environment_id" => {
            json!({"type":"string","format":"uuid","minLength":36,"maxLength":36})
        }
        "request_id" => {
            json!({"type":"string","format":"uuid","minLength":36,"maxLength":36})
        }
        "install_id" => {
            json!({"type":"string","pattern":"^[A-Za-z0-9][A-Za-z0-9_-]*$","not":{"pattern":"[^A-Za-z0-9_-]"},"maxLength":160})
        }
        "request" => request_schema(),
        "approval" if op == "install_runner" => {
            json!({"type":"string","pattern":"^[a-f0-9]{64}$","minLength":64,"maxLength":64,"writeOnly":true,"description":"原安装预览返回的准确 approval；已持久化上传 intent 的重复调用只核对原 install_id"})
        }
        "approval" => {
            let mut property = lintel_operations::plan_hash_schema();
            property["description"] = json!("原远端计划返回的准确 approval；新提交在记录 intent 前验证格式，已有本地 plan_id 记录时只查询原任务，不重新提交");
            property
        }
        "archive_passphrase" => {
            json!({"type":"string","minLength":12,"writeOnly":true,"description":"只通过 SSH stdin 传递，不进入 argv、诊断 excerpt 或本地 intent；仅新提交需要验证"})
        }
        _ => unreachable!("finite field owner"),
    }
}

fn schema(op: &str) -> Value {
    let (required, optional) = operation_fields(op).unwrap();
    let properties: Map<String, Value> = required
        .iter()
        .chain(optional.iter())
        .map(|field| ((*field).into(), property(op, field)))
        .collect();
    json!({"$schema":"https://json-schema.org/draft/2020-12/schema","title":format!("remote.{op}"),"type":"object","properties":properties,"required":required,"additionalProperties":false})
}

fn describe(op: &str) -> Value {
    let (local, target, external) = match op {
        "aliases" => (
            "read_literal_aliases_from_current_HOME_ssh_config",
            "none",
            "none",
        ),
        "hosts" => (
            "create_state_directory_and_read_registry_tasks_installations",
            "none",
            "none",
        ),
        "add_host" => (
            "create_state_directory_and_register_literal_alias",
            "none",
            "none",
        ),
        "remove_host" => (
            "remove_registry_entry_only_retain_tasks_installations_dedup",
            "none",
            "none",
        ),
        "connect" => (
            "create_state_directory_and_read_runner_binding",
            "runner_discover_inventory",
            "fixed_openssh_request",
        ),
        "request" => (
            "create_state_directory_and_read_runner_binding",
            "effects_of_selected_allowed_core_operation",
            "fixed_openssh_request_and_selected_core_effects",
        ),
        "execute" => (
            "persist_original_plan_intent_before_sole_submit_and_record_receipt",
            "approved_frozen_plan_or_original_job_query",
            "fixed_openssh_submit_or_original_job_query",
        ),
        "reconnect" => (
            "read_or_persist_query_only_original_plan_record_with_current_runner_binding_and_record_receipt",
            "query_original_job_only",
            "fixed_openssh_request",
        ),
        "prepare_runner" => (
            "read_explicit_static_resources_and_persist_frozen_install_preview",
            "probe_linux_platform_tools_target_user_identity_and_PATH_runner",
            "fixed_openssh_probe_no_upload",
        ),
        "install_runner" => (
            "persist_install_intent_before_sole_upload_then_verified_binding",
            "approved_user_owned_digest_runner_install_or_original_install_query",
            "fixed_openssh_upload_verify_discover",
        ),
        "query_install" => (
            "read_original_install_record_and_may_record_verified_binding",
            "verify_original_digest_install_and_discover_no_upload",
            "fixed_openssh_probe_verify_discover_or_none_for_unsubmitted_preview",
        ),
        "launch" => (
            "persist_private_fixed_command_for_alias_environment_runner",
            "readonly_launch_context_then_interactive_runner_launch",
            "macos_Terminal_fixed_openssh_PTY_session",
        ),
        "launch_request" => (
            "persist_private_fixed_command_for_frozen_request_id_and_bound_runner",
            "readonly_frozen_plan_show_then_interactive_runner_launch_request",
            "macos_Terminal_fixed_openssh_PTY_session",
        ),
        "resume_request" => (
            "persist_private_fixed_command_and_local_attempt_before_opening_terminal",
            "interactive_runner_resume_request_private_running_copy_passphrase_entered_in_terminal",
            "macos_Terminal_fixed_openssh_PTY_session",
        ),
        "launch_query" => (
            "read_original_launch_record_via_bound_runner_no_terminal",
            "readonly_launch_record_query",
            "fixed_openssh_request",
        ),
        "launches" => (
            "read_local_original_launch_metadata_no_terminal",
            "readonly_launch_records_list",
            "none",
        ),
        _ => unreachable!("finite operation owner"),
    };
    let local_only = ["aliases", "hosts", "add_host", "remove_host", "launches"].contains(&op);
    let installation = ["prepare_runner", "install_runner", "query_install"].contains(&op);
    let registry_condition = match op {
        "query_install" => "registered_literal_alias_or_retained_original_install_query",
        "reconnect" => "registered_literal_alias_or_retained_original_job_reconnect",
        "request" => "registered_literal_alias_or_retained_original_job_lookup_only",
        "launch_query" | "launches" => "registered_literal_alias_or_retained_original_launch_query",
        "launch_request" | "resume_request" => {
            "registered_literal_alias_or_retained_attempt_query_only"
        }
        _ => "registered_literal_alias",
    };
    let conditions = if local_only {
        json!(["current_user_HOME_and_local_state_only"])
    } else if installation {
        json!([
            registry_condition,
            "strict_host_key_and_BatchMode",
            "Linux_x86_64_or_aarch64_target_and_current_SSH_user",
            "required_linux_install_tools_and_static_machine_id",
            "frozen_target_identity_and_digest_are_rechecked"
        ])
    } else {
        json!([
            registry_condition,
            "strict_host_key_and_BatchMode",
            "bound_digest_runner_or_retained_PATH_compatibility",
            "runner_specific_environment_plan_and_original_job_checks"
        ])
    };
    let approval = match op {
        "execute" => {
            json!({"kind":"exact_remote_plan_approval","required_for":"new_submission","requires_existing_user_authority":true})
        }
        "install_runner" => {
            json!({"kind":"exact_frozen_install_preview_approval","required_for":"previewed_install_only","recheck":["target_identity","resource_manifest","runner_digest","ownership"],"requires_existing_user_authority":true})
        }
        "request" => {
            json!({"kind":"selected_core_operation_explicit_request","execution":"use_remote.execute_for_plan_application"})
        }
        "launch" => {
            json!({"kind":"explicit_interactive_session_request","terminal_state":"user_observed"})
        }
        _ => json!({"kind":"explicit_operation_request","hash_required":false}),
    };
    let recovery = match op {
        "execute" | "reconnect" => {
            json!({"id_field":"plan_id","operation":"remote.reconnect","mode":"query_original_job_only","sole_submit":true,"receipt_matches_original_plan":true,"preserve_original_runner_digest":true,"retained_after_remove_host":true,"do_not_delete_dedup_to_retry":true})
        }
        "prepare_runner" | "install_runner" | "query_install" => {
            json!({"id_field":"install_id","operation":"remote.query_install","mode":"query_original_install_only_after_upload_intent","sole_upload":true,"preserve_frozen_digest_and_target":true,"retained_after_remove_host":true,"do_not_create_second_install_after_uncertainty":true})
        }
        "remove_host" => {
            json!({"mode":"registry_only","retained_queries":["remote.reconnect","remote.request.job","remote.query_install"],"retains":["tasks","installations","runner_bindings","deduplication"]})
        }
        "launch" => {
            json!({"mode":"terminal_session_observation","detached_job":false,"session_ends_with_SSH":true,"reboot_survival":false})
        }
        "request" => {
            json!({"mode":"selected_core_operation_specific","mutating_response_loss":"observe_target_before_repeating","does_not_submit_plans":true})
        }
        _ => json!({"mode":"no_submission_or_upload"}),
    };
    let mut value = json!({
        "id":format!("remote.{op}"),"op":op,"protocol":1,"implementation":"implemented",
        "transports":["shared_remote_control","named_remote_cli","desktop_remote_request"],
        "platforms":if op=="launch" {vec!["macos"]}else{vec!["macos","linux"]},
        "platform_scope":"controller_client",
        "target_conditions":conditions,
        "applicability":{"status":"not_evaluated","reason":"纯静态 catalog；不打开 HOME/state、读取 SSH 配置、定位资源或连接目标"},
        "effects":{"local_controller":local,"target":target,"external":external},
        "approval":approval,"recovery":recovery,
        "request_schema":schema(op),
        "secret_fields":match op {"execute"=>vec!["approval","archive_passphrase"],"install_runner"=>vec!["approval"],"request"=>vec!["request.archive_passphrase"],_=>vec![]},
        "result":{"envelope":"ok/data or ok/error(code,message,diagnostic?)"}
    });
    if op == "request" {
        value["allowed_core_operations"] = json!(REQUEST_COMMANDS);
        value["core_operation_metadata"] = json!(REQUEST_COMMANDS
            .iter()
            .map(|command| lintel_operations::describe(command).unwrap())
            .collect::<Vec<_>>());
        value["excluded_core_operations"] = json!(["execute", "launch", "launch_context"]);
        value["path_scope"] =
            json!("archive_path/output_path/root refer to the target host; no file transfer");
    }
    if installation {
        value["runner_resources"] = json!({"owner":"caller_supplied_App_origin_static_runner_resources","argument":"explicit_bundles_directory_not_a_JSON_field","required_for":"prepare_runner_and_previewed_install_runner; query_install_uses_original_record","manifest_protocol":1,"targets":["x86_64-unknown-linux-musl","aarch64-unknown-linux-musl"],"max_bundle_bytes":32*1024*1024,"missing_resources":"bundle_unavailable","download_or_remote_compile":false,"sudo_or_linger_or_login_policy_change":false});
        value["target_platforms"] = json!(["linux"]);
    }
    if op == "launch" {
        value["launch_conditions"] = json!({"client":"macOS_Terminal_only","preflight":"readonly_launch_context","tty":"fixed_openssh_PTY","environment":"validated_ID_and_runner_rechecks_root_and_executable","prompt_supported":false,"remote_returned_paths_in_local_shell":false,"detached_mutation_job":false});
    }
    if op == "execute" {
        value["validation"] = json!({"schema_scope":"new_submission_authorization_fields","existing_local_plan_record":"only_exact_field_names_alias_and_plan_id_are_checked_then_original_job_is_queried; authorization_values_are_not_reused"});
    }
    if op == "install_runner" {
        value["validation"] = json!({"schema_scope":"approval_of_frozen_preview","after_upload_intent":"only_original_install_is_queried; no_second_upload"});
    }
    value
}

/// Describe the finite controller operations without reading environment,
/// filesystem, bundled resources, SSH configuration, or runtime applicability.
pub fn operation_catalog() -> Value {
    json!({"protocol":1,"catalog_version":1,"operations":OPERATIONS.iter().map(|op|describe(op)).collect::<Vec<_>>(),"transport_limits":{"request_json_bytes_including_newline":MAX_JSON,"retained_stderr_bytes":MAX_STDERR},"trust_boundary":{"ssh":"fixed_system_OpenSSH_strict_host_key_BatchMode_no_ProxyCommand_RemoteCommand_PermitLocalCommand_forwarding","arbitrary_shell":false,"arbitrary_upload_download":false,"browser_storage":false,"state":"current_HOME_and_LINTEL_STATE_DIR_same_owner_as_shared_controller"}})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{exact_operation_fields, validate_request};
    use std::collections::BTreeSet;

    #[test]
    fn catalog_exact_fields_share_runtime_owner_for_all_operations() {
        let catalog = operation_catalog();
        let rows = catalog["operations"].as_array().unwrap();
        assert_eq!(rows.len(), OPERATIONS.len());
        assert_eq!(
            rows.iter()
                .map(|row| row["id"].as_str().unwrap().to_string())
                .collect::<BTreeSet<_>>(),
            OPERATIONS
                .iter()
                .map(|op| format!("remote.{op}"))
                .collect::<BTreeSet<_>>()
        );
        for row in rows {
            let op = row["op"].as_str().unwrap();
            let schema = &row["request_schema"];
            let (required, optional) = operation_fields(op).unwrap();
            assert_eq!(schema["required"], json!(required));
            assert_eq!(schema["additionalProperties"], false);
            assert_eq!(schema["properties"]["op"]["const"], op);
            assert_eq!(
                schema["properties"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>(),
                required.iter().chain(optional.iter()).copied().collect()
            );
            let mut request = json!({"op":op});
            for key in required.iter().filter(|key| **key != "op") {
                request[*key] = json!("synthetic");
            }
            assert!(exact_operation_fields(&request).is_ok());
            for key in required.iter().filter(|key| **key != "op") {
                let mut missing = request.clone();
                missing.as_object_mut().unwrap().remove(*key);
                assert!(
                    exact_operation_fields(&missing).is_err(),
                    "{op} missing {key}"
                );
            }
            request["shell"] = json!("bad");
            assert!(exact_operation_fields(&request).is_err());
        }
    }

    #[test]
    fn catalog_request_reuses_core_schemas_and_excludes_transport_bypasses() {
        let catalog = operation_catalog();
        let row = catalog["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["op"] == "request")
            .unwrap();
        assert_eq!(row["allowed_core_operations"], json!(REQUEST_COMMANDS));
        let branches = row["request_schema"]["properties"]["request"]["oneOf"]
            .as_array()
            .unwrap();
        assert_eq!(branches.len(), REQUEST_COMMANDS.len());
        for branch in branches {
            let command = branch["properties"]["command"]["const"].as_str().unwrap();
            let original = lintel_operations::schema(command).unwrap();
            assert_eq!(branch["required"], original["required"]);
            assert_eq!(
                branch["properties"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .collect::<Vec<_>>(),
                original["properties"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .collect::<Vec<_>>()
            );
            for key in ["oneOf", "allOf", "additionalProperties"] {
                assert_eq!(branch[key], original[key]);
            }
        }
        for command in ["execute", "launch", "launch_context"] {
            assert!(!branches
                .iter()
                .any(|branch| branch["properties"]["command"]["const"] == command));
            assert!(validate_request(&json!({"command":command,"plan_id":"00000000-0000-4000-8000-000000000002","environment_id":"00000000-0000-4000-8000-000000000001","approval":"synthetic"})).is_err());
        }
        let archive = branches
            .iter()
            .find(|branch| branch["title"] == "archive_inspect")
            .unwrap();
        assert_eq!(archive["properties"]["archive_passphrase"]["minLength"], 12);
        assert_eq!(
            archive["properties"]["archive_passphrase"]["writeOnly"],
            true
        );
        let service = branches
            .iter()
            .find(|branch| branch["title"] == "service_inspect")
            .unwrap();
        assert_eq!(
            service["properties"]["unit"]["not"]["anyOf"][0]["pattern"],
            "@\\.service$"
        );
        assert!(validate_request(&json!({"command":"service_inspect","environment_id":"00000000-0000-4000-8000-000000000001","manager":"user","unit":"synthetic@.service"})).is_err());
        // The static telemetry catalog is a readonly remote request: it never
        // creates a task record, needs no runner field and accepts no other key.
        assert!(REQUEST_COMMANDS.contains(&"telemetry_catalog"));
        assert!(validate_request(&json!({"command":"telemetry_catalog"})).is_ok());
        assert!(validate_request(&json!({"command":"telemetry_catalog","host":"api.anthropic.com"})).is_err());
        let telemetry = branches
            .iter()
            .find(|branch| branch["title"] == "telemetry_catalog")
            .unwrap();
        assert_eq!(
            telemetry["properties"]["command"]["const"],
            "telemetry_catalog"
        );
        assert_eq!(telemetry["required"], json!(["command"]));
    }

    #[test]
    fn catalog_records_approval_original_id_recovery_and_platform_limits() {
        let catalog = operation_catalog();
        let rows = catalog["operations"].as_array().unwrap();
        let row = |op: &str| rows.iter().find(|row| row["op"] == op).unwrap();
        assert_eq!(row("execute")["recovery"]["sole_submit"], true);
        assert_eq!(row("execute")["recovery"]["operation"], "remote.reconnect");
        let approval = &row("execute")["request_schema"]["properties"]["approval"];
        assert_eq!(approval["pattern"], "^[a-f0-9]{64}$");
        assert_eq!(approval["minLength"], 64);
        assert_eq!(approval["maxLength"], 64);
        assert_eq!(row("install_runner")["recovery"]["sole_upload"], true);
        assert_eq!(
            row("install_runner")["approval"]["kind"],
            "exact_frozen_install_preview_approval"
        );
        assert_eq!(
            row("query_install")["recovery"]["operation"],
            "remote.query_install"
        );
        assert_eq!(
            row("remove_host")["recovery"]["retains"],
            json!(["tasks", "installations", "runner_bindings", "deduplication"])
        );
        assert_eq!(row("launch")["platforms"], json!(["macos"]));
        assert_eq!(
            row("launch")["launch_conditions"]["prompt_supported"],
            false
        );
        assert_eq!(
            row("prepare_runner")["runner_resources"]["download_or_remote_compile"],
            false
        );
        assert_eq!(row("prepare_runner")["target_platforms"], json!(["linux"]));
        assert_eq!(
            row("request")["request_schema"]["properties"]["request"]["oneOf"][0]["properties"]
                ["command"]["x-maxUtf8Bytes"],
            4096
        );
    }
}

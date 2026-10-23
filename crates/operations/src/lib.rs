//! Static operation contract shared by the agent CLI and finite transports.
//! Reading this catalog never opens a home, state, target, or executable.
use serde_json::{json, Map, Value};
use std::{
    collections::HashSet,
    path::{Component, Path},
};

/// The seven verified privacy switches. They keep their historical action set
/// (`keep`/`disable`/`remove`): `enable` would be ambiguous because "on" means
/// the signal is off, and `set` has no meaning for a boolean switch.
const SWITCHES: &[&str] = &[
    "DISABLE_TELEMETRY",
    "DISABLE_ERROR_REPORTING",
    "DISABLE_FEEDBACK_COMMAND",
    "CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY",
    "DO_NOT_TRACK",
    "DISABLE_GROWTHBOOK",
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
];
/// The remaining v4 keys: the OTel telemetry/content/exporter/identity family,
/// the survey-over-OTel route and the WebFetch switch. They admit the full
/// five-action set.
const OTEL_KEYS: &[&str] = &[
    "CLAUDE_CODE_ENABLE_TELEMETRY",
    "OTEL_LOG_USER_PROMPTS",
    "OTEL_LOG_ASSISTANT_RESPONSES",
    "OTEL_LOG_TOOL_CONTENT",
    "OTEL_LOG_TOOL_DETAILS",
    "OTEL_LOG_RAW_API_BODIES",
    "OTEL_LOG_MANAGED_SETTINGS",
    "CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH",
    "OTEL_LOGS_EXPORTER",
    "OTEL_TRACES_EXPORTER",
    "OTEL_METRICS_EXPORTER",
    "OTEL_EXPORTER_OTLP_ENDPOINT",
    "OTEL_EXPORTER_OTLP_HEADERS",
    "OTEL_METRICS_INCLUDE_ACCOUNT_UUID",
    "OTEL_METRICS_INCLUDE_SESSION_ID",
    "OTEL_METRICS_INCLUDE_VERSION",
    "OTEL_METRICS_INCLUDE_ENTRYPOINT",
    "OTEL_METRICS_INCLUDE_REPOSITORY",
    "CLAUDE_CODE_ENABLE_FEEDBACK_SURVEY_FOR_OTEL",
    "CLAUDE_CODE_DISABLE_WEB_FETCH",
];

/// The credential-bearing key. The core refuses `set`/`enable` and any
/// `custom_values` entry for it (`masked_setting_not_writable`); the schema
/// admits only the actions whose mutation never has to carry its value.
const MASKED_KEY: &str = "OTEL_EXPORTER_OTLP_HEADERS";

/// Actions every non-privacy-switch field admits. The core additionally refuses
/// field/action combinations the product would ignore (`enable` on a free-value
/// field, `set` on a boolean), which the schema deliberately does not restate.
const ACTIONS: &[&str] = &["keep", "disable", "enable", "remove", "set"];
/// The masked key shares the switch action set for the same reason: its value is
/// never recorded, so it can be kept, removed or disabled, never written.
const SWITCH_ACTIONS: &[&str] = &["keep", "disable", "remove"];

fn action_list(key: &str) -> &'static [&'static str] {
    if key == MASKED_KEY {
        SWITCH_ACTIONS
    } else {
        ACTIONS
    }
}

/// The v5 recognized env keys: the privacy switches first, then the OTel family,
/// matching `crates/core/src/policy.rs` `catalog()` order. The list is duplicated
/// here because this crate is the static contract for named/finite transports and
/// must not open a home, state or executable.
fn settings() -> impl Iterator<Item = &'static str> {
    SWITCHES.iter().chain(OTEL_KEYS.iter()).copied()
}

fn text() -> Value {
    json!({"type":"string","minLength":1,"maxLength":4096})
}
fn choice(values: &[&str]) -> Value {
    json!({"type":"string","enum":values})
}

fn network_probe_fields() -> Value {
    json!({"type":"object","properties":{
        "ipv4_url":property("ipv4_url"),"ipv6_url":property("ipv6_url"),
        "proxy_url":property("proxy_url"),"timeout_seconds":property("timeout_seconds"),
        "proxy_binding":property("proxy_binding")},
        "required":["ipv4_url","ipv6_url"],"additionalProperties":false})
}

/// Static URL admission before a finite SSH request; the probe executor also
/// validates endpoints before connecting. HTTPS only, no credentials or redirects.
pub fn valid_probe_url(url: &str) -> bool {
    if url.len() > 2048
        || !url.bytes().all(|b| b.is_ascii_graphic())
        || url.contains(['?', '#', '@', '\\'])
    {
        return false;
    }
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    if rest
        .find('/')
        .is_some_and(|start| rest.len() - start > 1024)
    {
        return false;
    }
    let authority = rest.split('/').next().unwrap_or("");
    let (host, port) = if let Some(tail) = authority.strip_prefix('[') {
        let Some((ip, suffix)) = tail.split_once(']') else {
            return false;
        };
        if ip.parse::<std::net::Ipv6Addr>().is_err() {
            return false;
        }
        let port = if suffix.is_empty() {
            None
        } else {
            let Some(port) = suffix.strip_prefix(':') else {
                return false;
            };
            Some(port)
        };
        (ip, port)
    } else {
        let (host, port) = authority
            .split_once(':')
            .map_or((authority, None), |(h, p)| (h, Some(p)));
        // Match egress admission: inet_aton spellings must not be admitted as
        // DNS names by the CLI/SSH contract only to fail after remote dispatch.
        if host.parse::<std::net::Ipv4Addr>().is_err()
            && !host.is_empty()
            && host.split('.').all(|label| {
                match label
                    .strip_prefix("0x")
                    .or_else(|| label.strip_prefix("0X"))
                {
                    Some(hex) => !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit()),
                    None => !label.is_empty() && label.bytes().all(|b| b.is_ascii_digit()),
                }
            })
        {
            return false;
        }
        if host.is_empty()
            || host.len() > 253
            || !host.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
        {
            return false;
        }
        (host, port)
    };
    !host.is_empty() && port.is_none_or(|p| p.parse::<u16>().is_ok_and(|p| p != 0))
}

fn valid_probe_proxy(url: &str) -> bool {
    let Some(authority) = url.strip_prefix("http://") else {
        return false;
    };
    authority
        .parse::<std::net::SocketAddr>()
        .is_ok_and(|addr| addr.ip().is_loopback() && addr.port() != 0)
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

/// Canonical finite exact-path rule shared by strict transports and raw core.
/// It only checks syntax (relative, no absolute/`.`/`..`/empty/control/`\\`,
/// bounded UTF-8 length); the core still re-derives the category and on-disk
/// identity. The named/finite transport rejects a
/// malformed selection before the core, an SSH call or any mutation.
pub fn valid_selected_path(raw: &str) -> bool {
    !raw.is_empty()
        && raw.len() <= 4096
        && !raw.contains('\\')
        && !raw.contains('\0')
        && !raw.chars().any(char::is_control)
        && raw == raw.trim()
        && !raw.contains("//")
        && !raw.starts_with('/')
        && raw
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

pub fn valid_selected_paths(value: &Value) -> bool {
    value.as_array().is_some_and(|items| {
        let mut seen = HashSet::new();
        !items.is_empty()
            && items.len() <= 10000
            && items.iter().all(|item| {
                item.as_str()
                    .is_some_and(|path| valid_selected_path(path) && seen.insert(path))
            })
            && serde_json::to_vec(value).is_ok_and(|bytes| bytes.len() <= 512 * 1024)
    })
}

/// Explicit absolute residue paths inside the observed root. The core still
/// re-derives the location class and on-disk identity; this only rejects a
/// malformed or over-budget selection before it reaches the core or a transport.
pub fn valid_residue_paths(value: &Value) -> bool {
    value.as_array().is_some_and(|items| {
        let mut seen = HashSet::new();
        !items.is_empty()
            && items.len() <= 1000
            && items.iter().all(|item| {
                item.as_str().is_some_and(|path| {
                    path.starts_with('/')
                        && path.len() <= 4096
                        && !path.chars().any(char::is_control)
                        && seen.insert(path)
                })
            })
            && serde_json::to_vec(value).is_ok_and(|bytes| bytes.len() <= 512 * 1024)
    })
}
/// Static work-path categories shared by strict transports and core content admission.
/// This function inspects a relative path only, never a filesystem or credential.
pub fn work_path_category(relative: &Path) -> Option<&'static str> {
    let name = relative.file_name()?.to_str()?;
    if relative == Path::new("CLAUDE.md") {
        return Some("instructions");
    }
    // A reference instruction that an earlier migration left in the inactive
    // work area keeps the instructions category so a later archive still covers
    // it. The only files this can match are names Lintel itself produced in that
    // one slot: the reference `CLAUDE.md` and its collision disambiguations
    // (`lintel-<n>-CLAUDE.md`). This finite rule never promotes arbitrary
    // unknown files.
    if is_reference_instructions(relative) {
        return Some("instructions");
    }
    // Work previously preserved into lintel-imports keeps its category so a
    // later archive covers it again; instruction slots were handled above.
    let work_area =
        relative.starts_with("projects") || relative.starts_with("lintel-imports/projects");
    if work_area
        && relative.components().any(|c| c.as_os_str() == "memory")
        && relative.extension().is_some_and(|x| x == "md")
    {
        return Some("memory");
    }
    if work_area && name.ends_with(".jsonl") {
        return Some("sessions");
    }
    None
}

/// The inactive reference-rotation slot for the instructions category is
/// `lintel-imports/<name>`, where `<name>` is `CLAUDE.md` or a collision
/// disambiguation like `lintel-1-CLAUDE.md`. Only a file placed directly under
/// `lintel-imports` matches; deeper paths keep their own category rules.
fn is_reference_instructions(relative: &Path) -> bool {
    let mut components = relative.components();
    let first = match components.next() {
        Some(Component::Normal(part)) => part,
        _ => return false,
    };
    if first != "lintel-imports" || components.clone().count() != 1 {
        return false;
    }
    let name = match components.next() {
        Some(Component::Normal(name)) => match name.to_str() {
            Some(name) => name,
            None => return false,
        },
        _ => return false,
    };
    if name == "CLAUDE.md" {
        return true;
    }
    name.strip_prefix("lintel-")
        .and_then(|rest| rest.strip_suffix("-CLAUDE.md"))
        .is_some_and(|index| !index.is_empty() && index.bytes().all(|b| b.is_ascii_digit()))
}
fn property(name: &str) -> Value {
    match name {
        "query" => json!({"type":"string","maxLength":256}),
        "filter" => choice(&["all","attention","archives","network"]),
        "section" => choice(&["header","receipt","steps","warnings","coverage","task_result","result_coverage","next_actions","next_steps","error"]),
        "approval" => plan_hash_schema(),
        "environment_id"
        | "plan_id"
        | "job_id"
        | "baseline_id"
        | "other_environment_id"
        | "old_snapshot_id"
        | "new_snapshot_id"
        | "source_environment_id" => {
            json!({"type":"string","format":"uuid","minLength":1,"maxLength":4096})
        }
        "ipv4_url" | "ipv6_url" => {
            json!({"type":"string","minLength":1,"maxLength":2048,"description":"显式 HTTPS IP 回显目标；不接受 credentials、query、fragment 或 redirect"})
        }
        "timeout_seconds" => json!({"type":"integer","minimum":1,"maximum":15}),
        "service_id" => json!({"type":"string","minLength":1,"maxLength":128}),
        "proxy_binding" => json!({"type":"string","minLength":1,"maxLength":128}),
        "probe" => network_probe_fields(),
        "keep_remote_control"
        | "writers_confirmed_stopped"
        | "official_logout"
        | "reveal"
        | "archive_only"
        | "lightweight"
        | "delete_work"
        | "archive_before_clear"
        | "reset_identity" => {
            json!({"type":"boolean"})
        }
        "categories" => {
            json!({"type":"array","items":choice(&["instructions","memory","sessions"]),"minItems":1,"uniqueItems":true})
        }
        // Exact original-file selection. Additive: absent keeps the historical
        // whole-category behavior. Non-empty unique canonical relative paths;
        // string-level limits are enforced here, category and on-disk identity
        // by the core. The validator additionally bounds encoded bytes to 512 KiB.
        "selected_keys" | "selected_changes" => {
            json!({"type":"array","minItems":1,"maxItems":4096,"uniqueItems":true,"items":text()})
        }
        "identity_backup_paths" | "extra_roots" => {
            json!({"type":"array","maxItems":if name=="extra_roots"{8}else{1000},"uniqueItems":true,"items":property("config_path")})
        }
        "document" => json!({"type":"string","maxLength":1048576,"writeOnly":true}),
        "policy" => {
            let mut p = schema("plan_policy").unwrap();
            p["required"] = json!(["preset", "keep_remote_control"]);
            p["properties"].as_object_mut().unwrap().remove("command");
            p["properties"]
                .as_object_mut()
                .unwrap()
                .remove("environment_id");
            p
        }
        "selected_paths" => {
            json!({"type":"array","minItems":1,"maxItems":10000,"uniqueItems":true,"items":{"type":"string","minLength":1,"maxLength":4096}})
        }
        // Explicit absolute paths inside the observed config root whose residue
        // this cleanup may remove. Additive and only read by `clear_residue`; the
        // core re-derives the location class and on-disk identity.
        "residue_paths" => {
            json!({"type":"array","minItems":1,"maxItems":1000,"uniqueItems":true,"items":{"type":"string","minLength":1,"maxLength":4096,"pattern":"^/","description":"配置目录内的准确绝对路径；执行器另行复查所属位置类别"}})
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
            let properties: Map<String, Value> = settings()
                .map(|key| (key.to_string(), choice(action_list(key))))
                .collect();
            json!({"type":"object","properties":properties,"additionalProperties":false})
        }
        // Explicit values are a separate custom-only channel so a value can never
        // be applied without naming the field and the `set` action it belongs to.
        // The masked key is absent by construction: it cannot carry a value.
        "custom_values" => {
            let properties: Map<String, Value> = settings()
                .filter(|key| *key != MASKED_KEY)
                .map(|key| {
                    (
                        key.to_string(),
                        json!({"type":"string","minLength":1,"maxLength":4096}),
                    )
                })
                .collect();
            json!({"type":"object","properties":properties,"additionalProperties":false,"description":"仅与 action=set 配对；值为要写入该字段的字符串"})
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
                    "index":json!({"type":"integer","minimum":0}),
                    "offset":json!({"type":"integer","minimum":0,"maximum":268435456}),
                    "block_index":json!({"type":"integer","minimum":0,"maximum":1000000})},
                "required":["path","digest"],"dependentRequired":{"block_index":["offset"]},"additionalProperties":false}}},"additionalProperties":false,"description":"有界来源元数据；offset 是原文件字节位置，block_index 是原行内容块序号，index 保留旧页内序号。不含正文或秘密；旧引用不虚构精确位置。"})
        }
        "request_id" => json!({"type":"string","format":"uuid","minLength":1,"maxLength":4096}),
        "limit" => json!({"type":"integer","minimum":1,"maximum":200}),
        "modified_after" | "since" | "until" | "min_bytes" | "max_bytes" => {
            json!({"type":"integer","minimum":0})
        }
        "class" => choice(&["work", "residue", "identity", "config", "unclassified"]),
        "offset" => json!({"type":"integer","minimum":0,"maximum":9007199254740991i64}),
        "expected_digest" => {
            json!({"type":"string","pattern":"^[a-f0-9]{64}$","minLength":64,"maxLength":64})
        }
        "root" | "archive_path" | "output_path" | "config_path" => {
            json!({"type":"string","pattern":"^/","description":"目标主机上的准确绝对路径；执行器另行复查实际文件与 ownership"})
        }
        "project_cwd" | "other_project_cwd" => {
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
        "discover" | "jobs" | "export_support" | "telemetry_catalog" => (&[], &[]),
        "jobs_page" => (&[], &["offset", "limit", "expected_digest", "filter", "environment_id", "query"]),
        "job_detail_page" => (&["job_id"], &["section", "offset", "limit", "expected_digest"]),
        "network_inspect" => (&[], &[]),
        "network_probe" => (
            &["ipv4_url", "ipv6_url"],
            &["proxy_url", "timeout_seconds", "proxy_binding"],
        ),
        "plan_network_ipv6" => (&["service_id", "mode", "probe"], &["baseline_id"]),
        "plan_network_restore" => (&["job_id"], &["baseline_id", "probe"]),
        "register" => (&["name", "root"], &[]),
        "create_environment" => (&["name"], &[]),
        "inspect" => (&["environment_id"], &["trusted_devices"]),
        "inspect_components" => (&["environment_id"], &["project_cwd"]),
        "work_preflight" => (&["environment_id", "categories"], &["selected_paths"]),
        "work_inventory" => (
            &["environment_id", "categories"],
            &["offset", "expected_digest"],
        ),
        "accept_drift" => (&["environment_id"], &["selected_keys"]),
        "identity_inspect" => (&["environment_id"], &["reveal"]),
        "residue_inventory" => (&["environment_id"], &["extra_roots","expected_digest","offset","limit","category","class","project","modified_after","since","until","min_bytes","max_bytes"]),
        "config_inspect" => (&["environment_id"], &["project_cwd"]),
        "config_read" => (&["environment_id","config_path"], &[]),
        "plan_config_edit" => (&["environment_id","config_path","document","expected_digest"],&[]),
        "plan_config_apply" => (&["environment_id","source_environment_id","selected_keys"], &["config_path"]),
        "config_restore_inspect" => (&["environment_id","job_id"], &[]),
        "plan_config_restore" => (&["environment_id","job_id","selected_keys"], &[]),
        "plan_client_workflow" => (&["environment_id","writers_confirmed_stopped"], &["policy","residue_paths","archive_before_clear","reset_identity","identity_backup_paths","extra_roots","delete_work"]),
        "drift" => (&["environment_id"], &["lightweight"]),
        "residue_archive_inspect" => (&["archive_passphrase"], &["job_id","archive_path"]),
        "plan_residue_restore" => (&["environment_id","selected_paths","archive_passphrase"], &["job_id","archive_path"]),
        "cleanup_inspect"
        | "auth_probe"
        | "reactivate_environment"
        | "launch_context"
        // Bounded read-only observations: residue inventory, salted identity
        // presence, and the local snapshot record set.
        | "snapshot_list" => (&["environment_id"], &[]),
        "snapshot_save" => (&["environment_id"], &["project_cwd"]),
        "snapshot_accept" => (&["environment_id","old_snapshot_id","new_snapshot_id","selected_changes"], &[]),
        "snapshot_compare" => (
            &["environment_id", "old_snapshot_id", "new_snapshot_id"],
            &[],
        ),
        "env_compare" => (&["environment_id", "other_environment_id"], &["project_cwd", "other_project_cwd"]),
        "plan_identity_reset" => (
            &["environment_id", "writers_confirmed_stopped"],
            &["identity_backup_paths"],
        ),
        "launch" => (&["environment_id"], &["proxy_url"]),
        "plan_policy" => (
            &["environment_id", "preset", "keep_remote_control"],
            &[
                "trusted_devices",
                "release_settings",
                "custom_settings",
                "custom_values",
                "config_path",
            ],
        ),
        "plan_reset" => (&["environment_id", "recipe", "categories"], &[]),
        "plan_archive" => (
            &["environment_id", "categories"],
            &["output_path", "selected_paths"],
        ),
        "plan_preserve" => (
            &["environment_id", "categories"],
            &["name", "activate", "selected_paths"],
        ),
        "plan_cleanup" => (
            &[
                "environment_id",
                "recipe",
                "writers_confirmed_stopped",
                "official_logout",
            ],
            &["categories", "activate", "residue_paths", "archive_before_clear", "archive_only", "extra_roots", "delete_work"],
        ),
        "plan_launch" => (
            &["environment_id", "project_cwd", "mode"],
            &["input_reference"],
        ),
        "launch_request" => (&["request_id", "approval"], &[]),
        "resume_request" => (&["request_id", "approval"], &["archive_passphrase"]),
        "launch_query" => (&["request_id"], &[]),
        "launches" => (&[], &[]),
        "plan_resume" => (
            &[
                "environment_id",
                "project_cwd",
                "path",
                "archive_passphrase",
            ],
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
    "network_inspect",
    "network_probe",
    "plan_network_ipv6",
    "plan_network_restore",
    "telemetry_catalog",
    "discover",
    "register",
    "create_environment",
    "inspect",
    "inspect_components",
    "work_preflight",
    "work_inventory",
    "residue_inventory",
    "identity_inspect",
    "snapshot_save",
    "snapshot_list",
    "snapshot_compare",
    "snapshot_accept",
    "env_compare",
    "config_inspect",
    "config_read",
    "plan_config_edit",
    "plan_config_apply",
    "config_restore_inspect",
    "plan_config_restore",
    "plan_client_workflow",
    "residue_archive_inspect",
    "plan_residue_restore",
    "plan_policy",
    "plan_reset",
    "plan_archive",
    "plan_preserve",
    "cleanup_inspect",
    "auth_probe",
    "plan_cleanup",
    "plan_identity_reset",
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
    "jobs_page",
    "job_detail_page",
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
    if command == "plan_network_ipv6" {
        properties.insert("mode".into(), choice(&["off", "link_local"]));
    }
    if command == "plan_cleanup" {
        properties.insert(
            "recipe".into(),
            choice(&["repair_login", "reset_client", "retire", "clear_residue"]),
        );
        properties
            .get_mut("categories")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("minItems");
    }
    // A reset the user must first confirm no writer can race: the literal `true`
    // is part of the schema so an unconfirmed request is refused before the core.
    if command == "plan_identity_reset" {
        properties.insert(
            "writers_confirmed_stopped".into(),
            json!({"type":"boolean","const":true}),
        );
    }
    if command == "plan_residue_restore" {
        properties.insert("selected_paths".into(), property("residue_paths"));
    }
    if command == "jobs_page" {
        let mut environment=property("environment_id");
        environment["description"]=json!("目标环境记录与此主机共享网络记录；其他环境及无关 host-only 记录不匹配");
        properties.insert("environment_id".into(),environment);
    }
    if matches!(command, "jobs_page" | "job_detail_page") {
        properties.insert("limit".into(), json!({"type":"integer","minimum":1,"maximum":100}));
    }
    let mut required = required.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    required.insert(0, "command".into());
    let mut value = json!({"$schema":"https://json-schema.org/draft/2020-12/schema","title":command,"type":"object","properties":properties,"required":required,"additionalProperties":false});
    if [
        "residue_archive_inspect",
        "plan_residue_restore",
        "archive_inspect",
        "archive_read",
        "session_read",
        "plan_import",
        "plan_resume",
        "job",
    ]
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
        value["allOf"] = json!([
            {"if":{"properties":{"recipe":{"const":"clear_residue"}}},"then":{"required":["residue_paths"],"properties":{"official_logout":{"const":false}}}},
            {"if":{"properties":{"recipe":{"enum":["reset_client","retire"]}}},"then":{"required":["categories"],"properties":{"categories":{"minItems":1}}}}
        ]);
    }
    if [
        "plan_launch",
        "launch_request",
        "plan_resume",
        "resume_request",
    ]
    .contains(&command)
    {
        value["x-lintel-interactive"] =
            json!(matches!(command, "launch_request" | "resume_request"));
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
                && items.len() <= schema["maxItems"].as_u64().unwrap_or(u64::MAX) as usize
                && items.iter().all(|v| check_property(v, &schema["items"]))
                && (schema["uniqueItems"] != true
                    || items
                        .iter()
                        .enumerate()
                        .all(|(i, v)| !items[..i].contains(v)))
        }),
        Some("object") => {
            value.as_object().is_some_and(|object| {
                schema["required"].as_array().is_none_or(|keys| {
                    keys.iter()
                        .all(|key| object.contains_key(key.as_str().unwrap()))
                }) && schema["dependentRequired"]
                    .as_object()
                    .is_none_or(|dependencies| {
                        dependencies.iter().all(|(key, required)| {
                            !object.contains_key(key)
                                || required.as_array().unwrap().iter().all(|dependency| {
                                    object.contains_key(dependency.as_str().unwrap())
                                })
                        })
                    })
                    && object.iter().all(|(key, v)| {
                        schema["properties"]
                            .get(key)
                            .is_some_and(|p| check_property(v, p))
                    })
            })
        }
        Some("integer") => value.as_u64().is_some(),
        _ => false,
    };
    typed
        && schema
            .get("enum")
            .is_none_or(|values| values.as_array().unwrap().contains(value))
        && schema.get("const").is_none_or(|expected| expected == value)
        && schema
            .get("minimum")
            .is_none_or(|min| value.as_i64().is_some_and(|v| v >= min.as_i64().unwrap()))
        && schema
            .get("maximum")
            .is_none_or(|max| value.as_u64().is_some_and(|v| v <= max.as_u64().unwrap()))
        && schema.get("pattern").is_none_or(|pattern| {
            let pattern = pattern.as_str().unwrap();
            // The only patterns used here are the fixed plan-hash shape; keep the
            // check minimal and exact rather than pulling in a regex engine.
            if pattern == "^[a-f0-9]{64}$" {
                value.as_str().is_some_and(|s| {
                    s.len() == 64
                        && s.bytes()
                            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                })
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
        // The exact-path validators handle typed items, count, uniqueness and
        // encoding together without the generic O(n²) uniqueItems scan.
        if !["selected_paths", "residue_paths"].contains(&key.as_str())
            && !check_property(value, prop)
        {
            return Err(format!("字段 {key} 的类型或取值无效"));
        }
        if key == "unit" && !valid_service_unit(value.as_str().unwrap()) {
            return Err("unit 需要完整 service 名称；不接受 template、路径、glob 或命令".into());
        }
        if key == "approval" && !valid_plan_hash(value.as_str().unwrap()) {
            return Err("approval 需要原计划返回的 64 位小写 hex hash".into());
        }
        if [
            "root",
            "archive_path",
            "output_path",
            "project_cwd",
            "other_project_cwd",
            "config_path",
        ]
        .contains(&key.as_str())
            && !value.as_str().unwrap().starts_with('/')
        {
            return Err(format!("{key} 需要绝对路径"));
        }
        if key == "selected_paths"
            && !(if command == "plan_residue_restore" {
                valid_residue_paths(value)
            } else {
                valid_selected_paths(value)
            })
        {
            return Err(
                "selected_paths 需要非空、唯一、相对、无 . / .. 或控制字符的精确原件路径".into(),
            );
        }
        if key == "residue_paths" && !valid_residue_paths(value) {
            return Err("residue_paths 需要非空、唯一、绝对、无控制字符的配置目录内路径".into());
        }
    }
    if matches!(command, "network_probe" | "plan_network_ipv6")
        || (command == "plan_network_restore" && request.get("probe").is_some())
    {
        let probe = if command == "network_probe" {
            request
        } else {
            &request["probe"]
        };
        for key in ["ipv4_url", "ipv6_url"] {
            if !probe[key].as_str().is_some_and(valid_probe_url) {
                return Err(format!(
                    "{key} 需要有界 HTTPS IP 回显地址，不接受凭据、query 或 fragment"
                ));
            }
        }
        if let Some(proxy) = probe.get("proxy_url") {
            if !proxy.as_str().is_some_and(valid_probe_proxy) {
                return Err("proxy_url 只接受带明确端口的 loopback HTTP 地址".into());
            }
        }
    }
    if let Some(paths) = request["selected_paths"]
        .as_array()
        .filter(|_| command != "plan_residue_restore")
    {
        for path in paths {
            let category = work_path_category(Path::new(path.as_str().unwrap()))
                .ok_or("selected_paths 不属于受支持工作类别")?;
            if !request["categories"]
                .as_array()
                .is_some_and(|categories| categories.iter().any(|item| item == category))
            {
                return Err("selected_paths 类别不在显式 categories 内".into());
            }
        }
    }
    if matches!(command, "work_inventory" | "residue_inventory" | "jobs_page" | "job_detail_page")
        && request["offset"].as_u64().is_some_and(|offset| {
            offset > 9007199254740991 || (offset > 0 && request.get("expected_digest").is_none())
        })
    {
        return Err("后续 metadata 页需要原清单摘要和有限 offset".into());
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
    if command == "plan_client_workflow" {
        if request["writers_confirmed_stopped"] != true {
            return Err("需要确认写入者已停止".into());
        }
        if let Some(policy) = request.get("policy") {
            let mut p = policy.clone();
            p["command"] = json!("plan_policy");
            p["environment_id"] = request["environment_id"].clone();
            validate(&p)?;
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
        // `custom_values` is the only channel that carries a value, and it is
        // meaningful only next to the `set` action that names the field — both
        // directions: a value without its action, and an action without its value,
        // are refused here. The core re-derives the same rule, including the
        // masked-key refusal and the per-field value syntax.
        let choices = request.get("custom_settings").and_then(Value::as_object);
        if let Some(values) = request.get("custom_values") {
            if request["preset"] != "custom" {
                return Err("custom_values 只适用于 custom 方案".into());
            }
            let Some(values) = values.as_object().filter(|v| !v.is_empty()) else {
                return Err("custom_values 必须是非空的字段取值对象".into());
            };
            for (key, value) in values {
                if key == MASKED_KEY {
                    return Err("该字段通常承载接收端凭据；Lintel 不记录也不写入它的值".into());
                }
                if choices.and_then(|c| c.get(key)).and_then(Value::as_str) != Some("set") {
                    return Err("custom_values 只允许用于 action 为 set 的字段".into());
                }
                if !value.as_str().is_some_and(|text| !text.is_empty()) {
                    return Err("set 需要非空字符串值".into());
                }
            }
        }
        if let Some(choices) = choices {
            for (key, action) in choices {
                if action.as_str() == Some("set") && request["custom_values"].get(key).is_none() {
                    return Err("set 动作需要在 custom_values 中给出该字段的取值".into());
                }
            }
        }
    }
    if command == "plan_cleanup"
        && matches!(request["recipe"].as_str(), Some("reset_client" | "retire"))
        && !request["categories"]
            .as_array()
            .is_some_and(|categories| !categories.is_empty())
    {
        return Err("重建或退役配方需要至少一种工作类别".into());
    }
    if command == "plan_cleanup"
        && request["recipe"] == "clear_residue"
        && (!valid_residue_paths(&request["residue_paths"]) || request["official_logout"] != false)
    {
        return Err("清理运行残留需要明确的非空原件列表，且不执行注销".into());
    }
    // Activation is only meaningful for the instruction position this round.
    // memory/sessions cannot be registered as active native sessions; asking to
    // do so is an explicit unsupported request, not a silent drop.
    if let Some(activate) = request.get("activate").and_then(Value::as_object) {
        for key in ["memory", "sessions"] {
            if activate.get(key).and_then(Value::as_bool) == Some(true) {
                return Err(
                    "memory/sessions 本轮没有可注册的活动会话位置；activate 只支持 instructions"
                        .into(),
                );
            }
        }
    }
    Ok(())
}

pub fn describe(command: &str) -> Option<Value> {
    let s = schema(command)?;
    let planning = command.starts_with("plan_") && command != "plan_show";
    let target = match command {
        "telemetry_catalog" => "no_target_readonly_static_catalog",
        "network_probe" => "explicit_https_ip_echo_requests",
        "plan_network_ipv6" | "plan_network_restore" => "read_shared_host_network_for_frozen_plan",
        "execute" => "frozen_plan_scope",
        "launch_request" => "start_target_process",
        "resume_request" => "publish_private_running_copy_and_start_target_process",
        "launch_query" | "launches" => "readonly_launch_records",
        "inspect_components" => "read_finite_component_sources_and_original_records",
        "work_preflight" => "read_selected_work_metadata_only",
        "work_inventory" => "read_selected_work_metadata_inventory",
        "jobs_page" | "job_detail_page" => "read_bounded_journal_views",
        "residue_inventory" => "read_known_local_location_metadata_only",
        "identity_inspect" => "read_documented_identity_presence_or_explicit_private_userID",
        "snapshot_save" => "read_settings_startup_residue_and_identity_for_a_local_snapshot",
        "snapshot_list" | "snapshot_compare" => "read_local_snapshot_records_only",
        "env_compare" => "read_two_local_environment_snapshots_only",
        "config_inspect" => "read_finite_configuration_sources_without_secret_values",
        "config_read" => "private_explicit_config_document_read",
        "config_restore_inspect" => "read_original_config_field_recovery_availability",
        "plan_config_edit" | "plan_config_apply" | "plan_config_restore" => {
            "read_explicit_configuration_for_frozen_field_plan"
        }
        "plan_client_workflow" => "freeze_selected_configuration_residue_and_identity_steps",
        "plan_identity_reset" => "read_identity_material_for_frozen_plan",
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
        "telemetry_catalog" => "readonly_static_catalog_no_state_no_network",
        "network_inspect" => "readonly_host_network_metadata",
        "network_probe" => "persist_finite_probe_result_for_baseline_reuse",
        "discover" | "register" | "create_environment" | "reactivate_environment" => {
            "update_inventory"
        }
        "job" | "jobs" => "read_journal_may_reconcile_interrupted_job",
        "jobs_page" | "job_detail_page" => "read_journal_without_reconciliation_or_record_writes",
        "launch_query" | "launches" => "read_launch_records_only",
        "inspect_components" => "read_original_records_without_reconciliation",
        "work_preflight" => "readonly_metadata_scan_no_state_written",
        "work_inventory" => "readonly_metadata_scan_no_state_written",
        "residue_inventory" => "readonly_metadata_scan_no_state_written",
        "identity_inspect" => "readonly_presence_observation_no_state_written",
        "snapshot_accept" => "write_selected_semantic_baseline_without_restoration_authority",
        "snapshot_save" => "write_immutable_sanitized_snapshot_and_private_comparison_salt",
        "snapshot_list" | "snapshot_compare" | "env_compare" => "read_local_snapshot_records_only",
        "accept_drift" => "write_baseline",
        "execute" => "persist_receipt_and_recovery",
        "launch_request" | "resume_request" => "persist_original_launch_intent_and_result",
        _ if planning => "persist_frozen_plan",
        _ => "state_directory_and_operation_lock",
    };
    let transports = match command {
        "launch_context" => {
            vec!["core_json", "runner_json", "named_cli"]
        }
        "launch" | "launch_request" | "resume_request" => vec!["core_json", "named_cli"],
        "execute" => vec!["core_json", "runner_json", "named_cli", "finite_ssh_submit"],
        _ => vec![
            "core_json",
            "runner_json",
            "named_cli",
            "finite_ssh_request",
        ],
    };
    let launch_conditions = match command {
        "launch" => {
            json!({"named_cli":"real_TTY_required_no_prompt","named_cli_entry":"lintel launch <environment-id>","core_json":"macos_Terminal_only","finite_ssh":"use_separate_remote_launch_control_macos_client"})
        }
        "launch_request" | "resume_request" => {
            json!({"named_cli":"real_TTY_required_no_prompt","named_cli_entry":if command == "resume_request" {"lintel launch resume <request-id> <approval-hash>"} else {"lintel launch request <request-id> <approval-hash>"},"core_json":"macos_Terminal_only","runner_json":"unsupported_terminal_required","finite_ssh":"use_separate_remote_launch_control_macos_client_bound_original_runner","prompt":false,"queue":false,"archive_passphrase":if command == "resume_request" {"required_for_first_attempt_private_terminal_input"} else {"not_required"}})
        }
        _ => Value::Null,
    };
    Some(json!({
        "id":command,"protocol":1,"implementation":"implemented","transports":transports,
        "platforms":if command.contains("service") {vec!["linux"]} else if matches!(command,"plan_network_ipv6" | "plan_network_restore") {vec!["macos"]} else {vec!["macos","linux"]},
        "target_conditions":if command.contains("service") {vec!["exact_root_bound_systemd_unit","user_manager_or_current_root"]} else if command.contains("network") {vec!["explicit_host_shared_network_scope","probe_targets_visible_before_request"]} else {vec!["explicit_registered_environment_or_original_task_where_required"]},
        "applicability":{"status":"not_evaluated","reason":"静态描述不检查个人环境；运行相应 inspect 取得目标事实"},
        "effects":{"target":target,"lintel_state":state,"external":match command {"network_probe" | "plan_network_ipv6" | "plan_network_restore"=>"explicit_ip_echo_requests","auth_probe"=>"official_auth_status_process","launch" | "launch_request" | "resume_request"=>"start_target_process","execute"=>"exact_plan_actions_may_include_official_logout_or_shared_network_change_and_reprobe",_=>"none"}},
        "requires_plan":matches!(command,"execute" | "launch_request" | "resume_request"),"approval":if matches!(command,"execute" | "launch_request" | "resume_request") {"exact_plan_hash_with_existing_user_authority"} else {"operation_specific_explicit_request"},
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
    fn telemetry_catalog_is_static_and_rejects_all_fields() {
        assert!(COMMANDS.contains(&"telemetry_catalog"));
        assert!(validate(&json!({"command":"telemetry_catalog"})).is_ok());
        // No field is accepted: unknown actors cannot smuggle a host or path.
        for extra in [
            json!({"command":"telemetry_catalog","path":"/etc/hosts"}),
            json!({"command":"telemetry_catalog","host":"api.anthropic.com"}),
            json!({"command":"telemetry_catalog","ids":["datadog_logs_intake"]}),
            json!({"command":"telemetry_catalog","environment_id":"00000000-0000-4000-8000-000000000001"}),
        ] {
            assert!(validate(&extra).is_err(), "{extra}");
        }
        let describe = describe("telemetry_catalog").unwrap();
        assert_eq!(describe["requires_plan"], false);
        assert_eq!(describe["effects"]["external"], "none");
        assert_eq!(
            describe["effects"]["lintel_state"],
            "readonly_static_catalog_no_state_no_network"
        );
        assert_eq!(
            describe["effects"]["target"],
            "no_target_readonly_static_catalog"
        );
        // The static catalog must never claim SSH execution.
        let transports = describe["transports"].as_array().unwrap();
        assert!(transports.contains(&json!("named_cli")));
    }

    #[test]
    fn network_requests_freeze_finite_targets_before_transport() {
        let probe = json!({"ipv4_url":"https://api.ipify.org","ipv6_url":"https://api6.ipify.org","timeout_seconds":10});
        assert!(validate(&json!({"command":"network_probe","ipv4_url":probe["ipv4_url"],"ipv6_url":probe["ipv6_url"]})).is_ok());
        assert!(validate(&json!({"command":"plan_network_ipv6","service_id":"synthetic-service","mode":"off","probe":probe})).is_ok());
        for mode in ["automatic", "disable_all", "interactive"] {
            assert!(validate(&json!({"command":"plan_network_ipv6","service_id":"synthetic-service","mode":mode,"probe":probe})).is_err());
        }
        for url in [
            "http://example.invalid",
            "https://user:pass@example.invalid",
            "https://example.invalid/?secret=x",
            "https://example.invalid/#x",
            "https://[::1]junk]",
            "https://example.invalid:0",
            "https://example.invalid/ bad",
            "https://example.invalid/\r\nX:bad",
        ] {
            assert!(validate(&json!({"command":"network_probe","ipv4_url":url,"ipv6_url":"https://api6.ipify.org"})).is_err(), "{url}");
        }
        for patch in [
            json!({"timeout_seconds":16}),
            json!({"proxy_url":"http://example.invalid:7890"}),
            json!({"extra":"unsupported"}),
        ] {
            let mut wrong = probe.clone();
            wrong
                .as_object_mut()
                .unwrap()
                .extend(patch.as_object().unwrap().clone());
            assert!(validate(&json!({"command":"plan_network_ipv6","service_id":"synthetic-service","mode":"off","probe":wrong})).is_err());
        }
        assert!(validate(&json!({"command":"plan_network_restore","job_id":"00000000-0000-4000-8000-000000000001","probe":probe})).is_ok());
        assert_eq!(
            describe("network_probe").unwrap()["effects"]["external"],
            "explicit_ip_echo_requests"
        );
        assert_eq!(
            describe("plan_network_ipv6").unwrap()["platforms"],
            json!(["macos"])
        );
    }
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
        for command in ["plan_archive", "plan_preserve", "work_preflight"] {
            assert!(validate(
                &json!({"command":command,"environment_id":"00000000-0000-4000-8000-000000000001","categories":[]})
            )
            .is_err());
            assert!(validate(&json!({"command":command,"environment_id":"00000000-0000-4000-8000-000000000001","categories":["instructions"]})).is_ok());
        }
        assert!(validate(&json!({"command":"plan_policy","environment_id":"00000000-0000-4000-8000-000000000001","preset":"reduce","keep_remote_control":false,"release_settings":[]})).is_ok());
    }

    #[test]
    fn work_inventory_is_a_strict_readonly_operation() {
        let id = "00000000-0000-4000-8000-000000000001";
        assert!(COMMANDS.contains(&"work_inventory"));
        assert!(validate(
            &json!({"command":"work_inventory","environment_id":id,"categories":["memory"]})
        )
        .is_ok());
        // offset/expected_digest are the only optional fields and are finite.
        assert!(validate(
            &json!({"command":"work_inventory","environment_id":id,"categories":["memory"],"offset":0,"expected_digest":"a".repeat(64)})
        )
        .is_ok());
        for invalid in [
            json!({"command":"work_inventory","categories":["memory"]}),
            json!({"command":"work_inventory","environment_id":id}),
            json!({"command":"work_inventory","environment_id":id,"categories":[]}),
            json!({"command":"work_inventory","environment_id":id,"categories":["bogus"]}),
            json!({"command":"work_inventory","environment_id":"synthetic","categories":["memory"]}),
            json!({"command":"work_inventory","environment_id":id,"categories":["memory"],"offset":-1}),
            json!({"command":"work_inventory","environment_id":id,"categories":["memory"],"offset":"0"}),
            json!({"command":"work_inventory","environment_id":id,"categories":["memory"],"expected_digest":"short"}),
            json!({"command":"work_inventory","environment_id":id,"categories":["memory"],"extra":true}),
            json!({"command":"work_inventory","environment_id":id,"categories":["memory"],"approval":"x"}),
        ] {
            assert!(validate(&invalid).is_err(), "{invalid}");
        }
        let describe = describe("work_inventory").unwrap();
        assert_eq!(describe["requires_plan"], false);
        assert_eq!(
            describe["effects"]["target"],
            "read_selected_work_metadata_inventory"
        );
        assert_eq!(
            describe["effects"]["lintel_state"],
            "readonly_metadata_scan_no_state_written"
        );
        assert!(describe["secret_fields"].as_array().unwrap().is_empty());
    }

    #[test]
    fn selected_paths_are_finite_and_transport_validated() {
        let id = "00000000-0000-4000-8000-000000000001";
        for command in ["plan_archive", "plan_preserve", "work_preflight"] {
            assert!(validate(
                &json!({"command":command,"environment_id":id,"categories":["sessions"],"selected_paths":["projects/demo/session.jsonl"]})
            )
            .is_ok());
        }
        // A named transport must reject an empty or malformed selection before
        // the core; it never revisits whole-category behavior silently.
        for bad in [
            json!([]),
            json!([""]),
            json!(["/etc/hosts"]),
            json!(["../escape"]),
            json!(["./dot"]),
            json!(["a//b"]),
            json!(["a/./b"]),
            json!(["projects\\demo"]),
            json!([" spaced "]),
            json!(["a\nb"]),
            json!(["dup", "dup"]),
        ] {
            assert!(
                validate(&json!({"command":"plan_archive","environment_id":id,"categories":["sessions"],"selected_paths":bad})).is_err(),
                "{bad}"
            );
        }
        // `selected_paths` is not accepted by operations that do not select work.
        assert!(validate(
            &json!({"command":"work_preflight","environment_id":id,"categories":["sessions"],"selected_paths":["x"]})
        )
        .is_err());
        assert!(validate(
            &json!({"command":"plan_import","environment_id":id,"categories":["sessions"],"archive_passphrase":"twelve-chars!","selected_paths":["x"]})
        )
        .is_err());
        assert!(valid_selected_path("projects/demo/memory/MEMORY.md"));
        assert!(!valid_selected_path(""));
        assert!(!valid_selected_path("/abs"));
        assert!(!valid_selected_path("a/../b"));
        assert!(!valid_selected_path("猫".repeat(2000).as_str()));
        let encoded: Vec<String> = (0..200)
            .map(|index| format!("projects/demo/{index}-{}.jsonl", "\"".repeat(2000)))
            .collect();
        assert!(!valid_selected_paths(&json!(encoded)));
    }
    #[test]
    fn work_preflight_is_a_strict_readonly_operation() {
        let id = "00000000-0000-4000-8000-000000000001";
        assert!(COMMANDS.contains(&"work_preflight"));
        // Required fields: environment_id and a non-empty finite category set.
        assert!(validate(
            &json!({"command":"work_preflight","environment_id":id,"categories":["memory"]})
        )
        .is_ok());
        for invalid in [
            json!({"command":"work_preflight","categories":["memory"]}),
            json!({"command":"work_preflight","environment_id":id}),
            json!({"command":"work_preflight","environment_id":id,"categories":[]}),
            json!({"command":"work_preflight","environment_id":id,"categories":["bogus"]}),
            json!({"command":"work_preflight","environment_id":"synthetic","categories":["memory"]}),
            json!({"command":"work_preflight","environment_id":id,"categories":["memory"],"extra":true}),
            json!({"command":"work_preflight","environment_id":id,"categories":["memory"],"approval":"x"}),
        ] {
            assert!(validate(&invalid).is_err(), "{invalid}");
        }
        let describe = describe("work_preflight").unwrap();
        assert_eq!(describe["requires_plan"], false);
        assert_eq!(
            describe["effects"]["target"],
            "read_selected_work_metadata_only"
        );
        assert_eq!(
            describe["effects"]["lintel_state"],
            "readonly_metadata_scan_no_state_written"
        );
        assert!(describe["secret_fields"].as_array().unwrap().is_empty());
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
            schema["allOf"][1]["then"]["required"],
            json!(["categories"])
        );
        assert_eq!(
            schema["allOf"][1]["then"]["properties"]["categories"]["minItems"],
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
        assert!(validate(
            &json!({"command":"session_read","archive_passphrase":"synthetic-only","path":"x"})
        )
        .is_err());
        // offset bounded integer; digest lowercase hex only.
        assert!(validate(&json!({"command":"session_read","archive_path":"/tmp/a.age","archive_passphrase":"synthetic-only","path":"x","offset":-1})).is_err());
        assert!(validate(&json!({"command":"session_read","archive_path":"/tmp/a.age","archive_passphrase":"synthetic-only","path":"x","offset":10})).is_ok());
        assert!(validate(&json!({"command":"session_read","archive_path":"/tmp/a.age","archive_passphrase":"synthetic-only","path":"x","expected_digest":"A".repeat(64)})).is_err());
        assert!(validate(&json!({"command":"session_read","archive_path":"/tmp/a.age","archive_passphrase":"synthetic-only","path":"x","expected_digest":"a".repeat(64)})).is_ok());
        assert_eq!(
            schema("session_read").unwrap()["properties"]["archive_passphrase"]["writeOnly"],
            true
        );
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
        assert!(validate(
            &json!({"command":"launch_request","request_id":"not-a-uuid","approval":"a".repeat(64)})
        )
        .is_err());
        assert!(validate(&json!({"command":"plan_resume","environment_id":"00000000-0000-4000-8000-000000000001","project_cwd":"/tmp/project","job_id":"00000000-0000-4000-8000-000000000002","archive_passphrase":"synthetic-only","path":"p/s.jsonl"})).is_ok());
        assert_eq!(
            schema("launch_request").unwrap()["x-lintel-interactive"],
            json!(true)
        );
    }

    #[test]
    fn source_references_keep_optional_byte_and_block_coordinates_bounded() {
        let mut request = json!({"command":"plan_launch","environment_id":"00000000-0000-4000-8000-000000000001","project_cwd":"/tmp/project","mode":"interactive","input_reference":{"files":[{"path":"projects/p/s.jsonl","digest":"a".repeat(64),"index":0}]}});
        assert!(validate(&request).is_ok()); // Older, page-local reference remains legal.
        request["input_reference"]["files"][0]["offset"] = json!(230054);
        request["input_reference"]["files"][0]["block_index"] = json!(2);
        assert!(validate(&request).is_ok());
        for (field, bad) in [
            ("offset", json!(-1)),
            ("offset", json!(268435457u64)),
            ("offset", json!("0")),
            ("block_index", json!(1000001)),
            ("block_index", json!(-1)),
        ] {
            let mut invalid = request.clone();
            invalid["input_reference"]["files"][0][field] = bad;
            assert!(validate(&invalid).is_err(), "{invalid}");
        }
        let mut missing = request.clone();
        missing["input_reference"]["files"][0]
            .as_object_mut()
            .unwrap()
            .remove("offset");
        assert!(validate(&missing).is_err());
        missing["input_reference"]["files"][0]
            .as_object_mut()
            .unwrap()
            .remove("block_index");
        missing["input_reference"]["files"][0]
            .as_object_mut()
            .unwrap()
            .remove("digest");
        assert!(validate(&missing).is_err());
        request["input_reference"]["files"] =
            json!(vec![request["input_reference"]["files"][0].clone(); 257]);
        assert!(validate(&request).is_err());
    }

    #[test]
    fn launch_descriptions_separate_planning_from_interactive_execution() {
        for name in ["plan_launch", "plan_resume"] {
            let d = describe(name).unwrap();
            assert_eq!(d["requires_plan"], false);
            assert_eq!(d["effects"]["external"], "none");
            assert_eq!(d["effects"]["lintel_state"], "persist_frozen_plan");
            assert!(d["launch_conditions"].is_null());
            assert_eq!(d["request_schema"]["x-lintel-interactive"], false);
            for transport in ["runner_json", "finite_ssh_request", "named_cli"] {
                assert!(d["transports"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(transport)));
            }
        }
        for name in ["launch_request", "resume_request"] {
            let d = describe(name).unwrap();
            assert_eq!(d["requires_plan"], true);
            assert_eq!(
                d["approval"],
                "exact_plan_hash_with_existing_user_authority"
            );
            assert_eq!(d["effects"]["external"], "start_target_process");
            assert_eq!(
                d["effects"]["lintel_state"],
                "persist_original_launch_intent_and_result"
            );
            assert_eq!(d["request_schema"]["x-lintel-interactive"], true);
            assert_eq!(
                d["launch_conditions"]["named_cli"],
                "real_TTY_required_no_prompt"
            );
            assert_eq!(d["transports"], json!(["core_json", "named_cli"]));
        }
        assert_eq!(
            describe("resume_request").unwrap()["effects"]["target"],
            "publish_private_running_copy_and_start_target_process"
        );
    }

    #[test]
    fn activation_is_finite_and_rejects_unavailable_positions() {
        let base = json!({"command":"plan_preserve","environment_id":"00000000-0000-4000-8000-000000000001","categories":["instructions"],"activate":{"instructions":true}});
        assert!(validate(&base).is_ok(), "{base}");
        assert!(validate(&json!({"command":"plan_preserve","environment_id":"00000000-0000-4000-8000-000000000001","categories":["instructions"],"activate":{"instructions":"yes"}})).is_err());
        assert!(validate(&json!({"command":"plan_preserve","environment_id":"00000000-0000-4000-8000-000000000001","categories":["instructions"],"activate":{"credentials":true}})).is_err());
        let bad = json!({"command":"plan_preserve","environment_id":"00000000-0000-4000-8000-000000000001","categories":["sessions"],"activate":{"sessions":true}});
        assert!(
            validate(&bad).is_err(),
            "activating an unavailable session position must be refused"
        );
    }

    #[test]
    fn v5_custom_contract_exposes_all_27_keys_five_actions_and_set_values() {
        let settings = schema("plan_policy").unwrap()["properties"]["custom_settings"].clone();
        assert_eq!(
            settings["additionalProperties"], false,
            "custom_settings stays a finite map"
        );
        let actions = settings["properties"].as_object().unwrap();
        assert_eq!(actions.len(), 27, "the v5 catalog has 27 recognized keys");
        // The privacy switches keep their historical action set; the OTel family
        // admits the full five. The core re-derives the field/action pairing.
        for key in SWITCHES {
            assert_eq!(
                actions[*key]["enum"],
                json!(["keep", "disable", "enable", "remove", "set"]),
                "{key}"
            );
        }
        for key in OTEL_KEYS.iter().filter(|key| **key != MASKED_KEY) {
            assert_eq!(
                actions[*key]["enum"],
                json!(["keep", "disable", "enable", "remove", "set"]),
                "{key}"
            );
        }
        // The masked key can be kept, disabled or removed, never written.
        assert_eq!(
            actions[MASKED_KEY]["enum"],
            json!(["keep", "disable", "remove"])
        );
        // The value channel is finite too, and the masked key is absent from it by
        // construction: a value can never be applied without naming `set`.
        let values = schema("plan_policy").unwrap()["properties"]["custom_values"].clone();
        assert_eq!(values["additionalProperties"], false);
        assert_eq!(values["properties"].as_object().unwrap().len(), 26);
        assert!(values["properties"].get(MASKED_KEY).is_none());
        let id = "00000000-0000-4000-8000-000000000001";
        let set = json!({"command":"plan_policy","environment_id":id,"preset":"custom","keep_remote_control":true,
            "custom_settings":{"OTEL_LOGS_EXPORTER":"set"},"custom_values":{"OTEL_LOGS_EXPORTER":"none"}});
        assert!(validate(&set).is_ok(), "{set}");
        for bad in [
            json!({"command":"plan_policy","environment_id":id,"preset":"custom","keep_remote_control":true,"custom_values":{"OTEL_LOGS_EXPORTER":"none"}}),
            json!({"command":"plan_policy","environment_id":id,"preset":"preserve","keep_remote_control":false,"custom_values":{"OTEL_LOGS_EXPORTER":"none"}}),
            json!({"command":"plan_policy","environment_id":id,"preset":"custom","keep_remote_control":true,"custom_settings":{},"custom_values":{}}),
            json!({"command":"plan_policy","environment_id":id,"preset":"custom","keep_remote_control":true,"custom_settings":{},"custom_values":{"OTEL_EXPORTER_OTLP_HEADERS":"x"}}),
            json!({"command":"plan_policy","environment_id":id,"preset":"custom","keep_remote_control":true,"custom_settings":{},"custom_values":{"UNKNOWN":"x"}}),
            json!({"command":"plan_policy","environment_id":id,"preset":"custom","keep_remote_control":true,"custom_settings":{},"custom_values":{"OTEL_LOGS_EXPORTER":1}}),
        ] {
            assert!(validate(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn bounded_observation_and_snapshot_operations_are_finite() {
        let id = "00000000-0000-4000-8000-000000000001";
        for command in [
            "residue_inventory",
            "identity_inspect",
            "snapshot_save",
            "snapshot_list",
        ] {
            assert!(COMMANDS.contains(&command));
            assert!(validate(&json!({"command":command,"environment_id":id})).is_ok());
            for bad in [
                json!({"command":command}),
                json!({"command":command,"environment_id":"synthetic"}),
                json!({"command":command,"environment_id":id,"extra":true}),
                json!({"command":command,"environment_id":id,"salt":"caller-chosen"}),
            ] {
                assert!(validate(&bad).is_err(), "{bad}");
            }
        }
        let other = "00000000-0000-4000-8000-000000000002";
        assert!(validate(&json!({"command":"snapshot_compare","environment_id":id,"old_snapshot_id":id,"new_snapshot_id":other})).is_ok());
        assert!(validate(
            &json!({"command":"snapshot_compare","environment_id":id,"old_snapshot_id":id})
        )
        .is_err());
        assert!(validate(
            &json!({"command":"env_compare","environment_id":id,"other_environment_id":other})
        )
        .is_ok());
        assert!(validate(&json!({"command":"env_compare","environment_id":id})).is_err());
        assert!(validate(
            &json!({"command":"env_compare","environment_id":id,"other_environment_id":"synthetic"})
        )
        .is_err());
        // Residue and identity observations are metadata/presence only: no
        // operation description may claim a write.
        for command in ["residue_inventory", "identity_inspect", "snapshot_list"] {
            let described = describe(command).unwrap();
            assert_eq!(described["requires_plan"], false, "{command}");
            assert_eq!(described["effects"]["external"], "none", "{command}");
        }
    }

    #[test]
    fn identity_reset_requires_the_literal_writer_confirmation() {
        let id = "00000000-0000-4000-8000-000000000001";
        let confirmed = json!({"command":"plan_identity_reset","environment_id":id,"writers_confirmed_stopped":true});
        assert!(validate(&confirmed).is_ok(), "{confirmed}");
        assert_eq!(
            schema("plan_identity_reset").unwrap()["properties"]["writers_confirmed_stopped"]
                ["const"],
            true
        );
        for bad in [
            json!({"command":"plan_identity_reset","environment_id":id,"writers_confirmed_stopped":false}),
            json!({"command":"plan_identity_reset","environment_id":id,"writers_confirmed_stopped":"true"}),
            json!({"command":"plan_identity_reset","environment_id":id,"writers_confirmed_stopped":1}),
            json!({"command":"plan_identity_reset","environment_id":id}),
        ] {
            assert!(validate(&bad).is_err(), "{bad}");
        }
        assert_eq!(
            describe("plan_identity_reset").unwrap()["effects"]["external"],
            "none"
        );
    }

    #[test]
    fn client_control_strict_contract_carries_private_readers_and_selected_baselines() {
        let id = "00000000-0000-4000-8000-000000000001";
        let accepted = json!({"command":"snapshot_accept","environment_id":id,"old_snapshot_id":id,"new_snapshot_id":id,"selected_changes":["synthetic-change"]});
        assert!(validate(&accepted).is_ok());
        for selection in [json!([]), json!("all"), json!([1]), json!(["dup", "dup"])] {
            let mut bad = accepted.clone();
            bad["selected_changes"] = selection;
            assert!(validate(&bad).is_err(), "{bad}");
        }
        for command in ["residue_archive_inspect", "plan_residue_restore"] {
            let mut request = json!({"command":command,"archive_path":"/synthetic/residue.age","archive_passphrase":"synthetic-long-password"});
            if command == "plan_residue_restore" {
                request["environment_id"] = json!(id);
                request["selected_paths"] = json!(["/synthetic/debug/original"]);
            }
            assert!(validate(&request).is_ok(), "{request}");
            request["job_id"] = json!(id);
            assert!(validate(&request).is_err(), "ambiguous source: {request}");
        }
        assert!(
            validate(&json!({"command":"residue_inventory","environment_id":id,"offset":1}))
                .is_err()
        );
        assert!(validate(&json!({"command":"residue_inventory","environment_id":id,"offset":1,"expected_digest":"a".repeat(64)})).is_ok());
        assert!(
            validate(&json!({"command":"drift","environment_id":id,"lightweight":true})).is_ok()
        );
    }

    #[test]
    fn residue_clear_is_an_explicit_absolute_path_selection() {
        let id = "00000000-0000-4000-8000-000000000001";
        assert!(
            schema("plan_cleanup").unwrap()["properties"]["recipe"]["enum"]
                .as_array()
                .unwrap()
                .contains(&json!("clear_residue"))
        );
        let base = json!({"command":"plan_cleanup","environment_id":id,"recipe":"clear_residue","writers_confirmed_stopped":true,"official_logout":false});
        assert!(validate(&base).is_err(), "clear needs an exact selection");
        let mut with_paths = base.clone();
        with_paths["residue_paths"] = json!(["/synthetic/root/debug"]);
        assert!(validate(&with_paths).is_ok());
        let mut logout = with_paths.clone();
        logout["official_logout"] = json!(true);
        assert!(validate(&logout).is_err());
        for bad in [
            json!([]),
            json!([""]),
            json!(["relative/dir"]),
            json!(["dup", "dup"]),
            json!(["/a\nb"]),
            json!(["猫".repeat(2000).as_str()]),
        ] {
            let mut request = base.clone();
            request["residue_paths"] = bad.clone();
            assert!(validate(&request).is_err(), "{bad}");
        }
        // `residue_paths` belongs to the cleanup plan only.
        assert!(validate(
            &json!({"command":"work_preflight","environment_id":id,"categories":["sessions"],"residue_paths":["/a"]})
        )
        .is_err());
    }

    #[test]
    fn task_catalog_is_one_finite_map_of_six_tasks() {
        let catalog = tasks();
        assert_eq!(catalog["schema"], "lintel.tasks/1");
        assert_eq!(catalog["version"], 1);
        let list = catalog["tasks"].as_array().unwrap();
        assert_eq!(list.len(), 6);
        let ids: Vec<&str> = list.iter().map(|t| t["id"].as_str().unwrap()).collect();
        assert_eq!(
            ids,
            [
                "reduce_egress",
                "preserve_work",
                "repair_cleanup_retire",
                "browser_profile",
                "ssh_remote",
                "recover_results"
            ]
        );
        // Every referenced operation must be a real operation this catalog owns.
        for task in list {
            for op in task["operations"].as_array().unwrap() {
                assert!(
                    schema(op.as_str().unwrap()).is_some(),
                    "task references unknown operation {op}"
                );
            }
            assert!(task["route"].as_str().unwrap().starts_with('#'), "route");
            assert!(
                !task["help_anchor"].as_str().unwrap().is_empty(),
                "help_anchor"
            );
            assert!(!task["label"].as_str().unwrap().is_empty(), "label");
            assert!(!task["summary"].as_str().unwrap().is_empty(), "summary");
            // Every CLI example must invoke `lintel` with a real top-level grammar.
            for example in task["cli_examples"].as_array().unwrap() {
                let example = example.as_str().unwrap();
                assert!(
                    example.contains("lintel "),
                    "cli example grammar: {example}"
                );
            }
        }
    }
}

#[cfg(test)]
mod history_contract_tests {
    use super::*;
    #[test]
    fn bounded_history_requests_are_strict_and_readonly() {
        let id="00000000-0000-4000-8000-000000000001";
        for request in [json!({"command":"jobs_page"}),json!({"command":"jobs_page","filter":"archives","offset":100,"limit":100,"expected_digest":"a".repeat(64)}),json!({"command":"job_detail_page","job_id":id,"section":"steps"})] {validate(&request).unwrap();}
        for request in [json!({"command":"jobs_page","limit":101}),json!({"command":"jobs_page","filter":"secret"}),json!({"command":"job_detail_page","job_id":id,"section":"private_extra"}),json!({"command":"job_detail_page","job_id":id,"approval":"a".repeat(64)})] {assert!(validate(&request).is_err(),"{request}");}
        assert!(describe("jobs_page").unwrap().to_string().contains("read_journal_without_reconciliation_or_record_writes"));
    }
}

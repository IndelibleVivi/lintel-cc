//! Versioned, source-scoped privacy rules. Never runs Claude or probes an account.
use crate::{err, Result};
use serde_json::{json, Map, Value};
use std::path::Path;

pub(crate) const RULE: &str = "claude-privacy-v4-2026-10-16";
pub(crate) const FLAGS: [(&str, &str); 4] = [
    ("DISABLE_TELEMETRY", "产品指标"),
    ("DISABLE_ERROR_REPORTING", "错误回报"),
    ("DISABLE_FEEDBACK_COMMAND", "主动反馈 / bug / share"),
    ("CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY", "质量调查"),
];
pub(crate) const REMOTE_KEYS: [&str; 4] = [
    "DISABLE_TELEMETRY",
    "DO_NOT_TRACK",
    "DISABLE_GROWTHBOOK",
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
];
/// How a variable's value is read. `Boolean`/`Nonempty` are the verified
/// semantics of the privacy switches; the remaining families are the OTel
/// content-recording controls.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Semantics {
    /// 1/true/yes/on enables, empty/0/false/no/off disables, other spellings unknown.
    Boolean,
    /// Any non-empty string takes effect, including "0"/"false".
    Nonempty,
    /// Absent or empty means enabled; "false"/"0" excludes it.
    DefaultOn,
    /// Free-form value (URL, number, exporter selector).
    Value,
    /// Absent means off; "1" records inline (truncated); "file:<dir>" dumps the
    /// complete bodies into a local directory and references them by path.
    RawApiBodies,
}

impl Semantics {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Semantics::Boolean => "boolean",
            Semantics::Nonempty => "nonempty",
            Semantics::DefaultOn => "default_on",
            Semantics::Value => "value",
            Semantics::RawApiBodies => "raw_api_bodies",
        }
    }
}

/// Which settings sources can turn a variable on or off. Verified against the
/// official env-vars / monitoring docs: the OTel family is ignored in project
/// and local settings, "apart from the off values" — a repository may still
/// turn a signal off, but not on and not to a destination.
pub(crate) struct Sources {
    pub(crate) turn_on: &'static [&'static str],
    pub(crate) turn_off: &'static [&'static str],
}

pub(crate) struct FieldRule {
    pub(crate) key: &'static str,
    pub(crate) label: &'static str,
    pub(crate) category: &'static str,
    pub(crate) semantics: Semantics,
    pub(crate) sources: Sources,
    /// Official default, not a current observation: "off"/"on"/"redacted", plus
    /// the stated default for free-value fields.
    pub(crate) default_state: &'static str,
    /// What the variable records or routes, and to which receiver.
    pub(crate) impact: &'static str,
    pub(crate) version_floor: Option<&'static str>,
    /// Only emitted for content families; names the fields this one follows.
    pub(crate) depends_on: &'static [&'static str],
}

const USER_ONLY: &[&str] = &["shell", "user", "managed"];
const ANY_SOURCE: &[&str] = &["shell", "user", "managed", "project", "project_local"];
const USER_SOURCES: Sources = Sources {
    turn_on: USER_ONLY,
    turn_off: USER_ONLY,
};
const OTEL_SOURCES: Sources = Sources {
    turn_on: USER_ONLY,
    turn_off: ANY_SOURCE,
};
const ANY_SOURCES: Sources = Sources {
    turn_on: ANY_SOURCE,
    turn_off: ANY_SOURCE,
};

/// The credential-bearing header value. Its value never appears in any output.
pub(crate) const MASKED_KEY: &str = "OTEL_EXPORTER_OTLP_HEADERS";
const CONTENT_LIMIT_KEY: &str = "CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH";
/// Documented exporter selector tokens. Logs and traces use otlp/console/none;
/// metrics additionally allows prometheus. Each field admits only its own set so
/// a plan cannot record a selector the product would ignore.
const SIGNAL_EXPORTER_TOKENS: &[&str] = &["otlp", "console", "none"];
const METRICS_EXPORTER_TOKENS: &[&str] = &["otlp", "prometheus", "console", "none"];

const CATALOG: &[FieldRule] = &[
    FieldRule {
        key: "DISABLE_TELEMETRY",
        label: "产品指标",
        category: "privacy_switch",
        semantics: Semantics::Nonempty,
        sources: USER_SOURCES,
        default_state: "off",
        impact: "关闭 Claude Code 的产品指标上报（Anthropic 第一方统计）；nonempty 语义，“0”/“false” 也会关闭。",
        version_floor: None,
        depends_on: &[],
    },
    FieldRule {
        key: "DISABLE_ERROR_REPORTING",
        label: "错误回报",
        category: "privacy_switch",
        semantics: Semantics::Nonempty,
        sources: USER_SOURCES,
        default_state: "off",
        impact: "关闭错误回报：崩溃与错误诊断数据不再发往 Anthropic。",
        version_floor: None,
        depends_on: &[],
    },
    FieldRule {
        key: "DISABLE_FEEDBACK_COMMAND",
        label: "主动反馈 / bug / share",
        category: "privacy_switch",
        semantics: Semantics::Boolean,
        sources: USER_SOURCES,
        default_state: "off",
        impact: "关闭 /bug、/share 等主动反馈入口，用户不能主动把内容提交给 Anthropic。",
        version_floor: None,
        depends_on: &[],
    },
    FieldRule {
        key: "CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY",
        label: "质量调查",
        category: "privacy_switch",
        semantics: Semantics::Boolean,
        sources: USER_SOURCES,
        default_state: "off",
        impact: "关闭质量调查提示（问卷不再展示）。",
        version_floor: None,
        depends_on: &[],
    },
    FieldRule {
        key: "DO_NOT_TRACK",
        label: "跨工具 telemetry 开关",
        category: "privacy_switch",
        semantics: Semantics::Boolean,
        sources: USER_SOURCES,
        default_state: "off",
        impact: "跨工具 telemetry 约定开关；与 DISABLE_TELEMETRY 的影响范围重叠。",
        version_floor: None,
        depends_on: &[],
    },
    FieldRule {
        key: "DISABLE_GROWTHBOOK",
        label: "feature-flag 获取",
        category: "privacy_switch",
        semantics: Semantics::Boolean,
        sources: USER_SOURCES,
        default_state: "off",
        impact: "关闭 feature-flag 获取；该流量与 Remote Control 冲突，关闭后依赖远端开关的功能不可用。",
        version_floor: None,
        depends_on: &[],
    },
    FieldRule {
        key: "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
        label: "非必要流量总开关",
        category: "privacy_switch",
        semantics: Semantics::Nonempty,
        sources: USER_SOURCES,
        default_state: "off",
        impact: "非必要流量总开关：nonempty 语义，同时关闭自动更新、release notes 与 feature-flag 获取；与 Remote Control 冲突。",
        version_floor: None,
        depends_on: &[],
    },
    FieldRule {
        key: "CLAUDE_CODE_ENABLE_TELEMETRY",
        label: "OTel 数据收集总开关",
        category: "telemetry_core",
        semantics: Semantics::Boolean,
        sources: OTEL_SOURCES,
        default_state: "off",
        impact: "开启后 Claude Code 才收集 OTel 指标/日志/trace；后续 exporter 与内容记录都以此为前提。官方明确该变量在项目与本地设置中被忽略，只能由 shell、user 或 managed 设置提供。",
        version_floor: None,
        depends_on: &[],
    },
    FieldRule {
        key: "OTEL_LOG_USER_PROMPTS",
        label: "OTel 用户提示词记录",
        category: "content_prompts",
        semantics: Semantics::Boolean,
        sources: OTEL_SOURCES,
        default_state: "redacted",
        impact: "在 OTel 日志与 trace 中包含用户提示词原文，接收端为用户自己配置的 OTel collector（去向由 OTEL_EXPORTER_OTLP_ENDPOINT 决定）。默认 redacted：只记元数据，不含提示词正文。",
        version_floor: None,
        depends_on: &["CLAUDE_CODE_ENABLE_TELEMETRY"],
    },
    FieldRule {
        key: "OTEL_LOG_ASSISTANT_RESPONSES",
        label: "OTel 模型回复记录",
        category: "content_responses",
        semantics: Semantics::Boolean,
        sources: OTEL_SOURCES,
        default_state: "redacted",
        impact: "把模型回复原文写入 OTel 日志/trace，接收端同上。未设置时跟随 OTEL_LOG_USER_PROMPTS 的取值；显式 “0” 可以在提示词记录开启时仍保持回复 redacted。",
        version_floor: Some("2.1.193"),
        depends_on: &["OTEL_LOG_USER_PROMPTS"],
    },
    FieldRule {
        key: "OTEL_LOG_TOOL_CONTENT",
        label: "OTel 工具内容记录",
        category: "content_tool",
        semantics: Semantics::Boolean,
        sources: OTEL_SOURCES,
        default_state: "off",
        impact: "让 tool.output span 事件携带工具输出内容（可能包含文件内容与命令输出），接收端同上。依赖 tracing 已开启（OTEL_TRACES_EXPORTER 指向可用接收端）；未开启 tracing 时该开关不产生内容记录。",
        version_floor: None,
        depends_on: &["tracing"],
    },
    FieldRule {
        key: "OTEL_LOG_TOOL_DETAILS",
        label: "OTel 工具细节记录",
        category: "content_tool_details",
        semantics: Semantics::Boolean,
        sources: OTEL_SOURCES,
        default_state: "off",
        impact: "把工具入参、MCP server 名、原始错误串、refusal category 与真实 agent/skill/plugin 名写入 metrics/traces/logs；接收端为所有已启用的 exporter。比工具输出更细：包含调用参数与真实命名。",
        version_floor: None,
        depends_on: &["CLAUDE_CODE_ENABLE_TELEMETRY"],
    },
    FieldRule {
        key: "OTEL_LOG_RAW_API_BODIES",
        label: "原始 API 正文记录",
        category: "content_api_bodies",
        semantics: Semantics::RawApiBodies,
        sources: OTEL_SOURCES,
        default_state: "off",
        impact: "完整 Messages API 请求/响应 JSON（含完整会话历史）。“1” 为内联截断记录，上限由 CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH 控制（默认 61440 字节）；“file:<dir>” 把未截断正文写入本地目录，日志事件只带 body_ref 路径，目录内另有 index.jsonl（v2.1.274+）。注意：“file:<dir>” 会在本地落盘完整会话正文——正文落盘 ≠ 已上传，二者是不同事实；内联模式则会把正文发往用户配置的 OTel 接收端。",
        version_floor: None,
        depends_on: &["CLAUDE_CODE_ENABLE_TELEMETRY"],
    },
    FieldRule {
        key: "OTEL_LOG_MANAGED_SETTINGS",
        label: "OTel managed settings 记录",
        category: "content_managed_settings",
        semantics: Semantics::Boolean,
        sources: OTEL_SOURCES,
        default_state: "off",
        impact: "managed_settings_resolved 事件附带脱敏后的 managed settings 及其 SHA-256 digest；接收端为用户自己的 collector。记录的是组织下发的设置内容。",
        version_floor: Some("2.1.274"),
        depends_on: &["CLAUDE_CODE_ENABLE_TELEMETRY"],
    },
    FieldRule {
        key: CONTENT_LIMIT_KEY,
        label: "OTel 内容长度上限",
        category: "content_limit",
        semantics: Semantics::Value,
        sources: OTEL_SOURCES,
        default_state: "61440",
        impact: "内联记录正文的字节上限（默认 61440）；只影响 OTEL_LOG_RAW_API_BODIES 的 “1” 内联模式的正文截断，不影响 “file:<dir>” 落盘模式。",
        version_floor: Some("2.1.214"),
        depends_on: &["OTEL_LOG_RAW_API_BODIES"],
    },
    FieldRule {
        key: "OTEL_LOGS_EXPORTER",
        label: "OTel 日志 exporter",
        category: "exporter",
        semantics: Semantics::Value,
        sources: OTEL_SOURCES,
        default_state: "otlp",
        impact: "选择日志信号的接收端类型：otlp / console / none。改为 none 即关闭日志信号；项目与本地设置不能指定去向，但可以设为 none 关闭。",
        version_floor: None,
        depends_on: &[],
    },
    FieldRule {
        key: "OTEL_TRACES_EXPORTER",
        label: "OTel trace exporter",
        category: "exporter",
        semantics: Semantics::Value,
        sources: OTEL_SOURCES,
        default_state: "otlp",
        impact: "选择 trace 信号的接收端类型：otlp / console / none；OTEL_LOG_TOOL_CONTENT 依赖 tracing 已开启。",
        version_floor: None,
        depends_on: &[],
    },
    FieldRule {
        key: "OTEL_METRICS_EXPORTER",
        label: "OTel 指标 exporter",
        category: "exporter",
        semantics: Semantics::Value,
        sources: OTEL_SOURCES,
        default_state: "otlp",
        impact: "选择指标信号的接收端类型：otlp / prometheus / console / none；身份属性字段只影响指标。",
        version_floor: None,
        depends_on: &[],
    },
    FieldRule {
        key: "OTEL_EXPORTER_OTLP_ENDPOINT",
        label: "OTel 接收端地址",
        category: "exporter_destination",
        semantics: Semantics::Value,
        sources: OTEL_SOURCES,
        default_state: "unset",
        impact: "日志、指标与 trace 的接收端地址。managed settings 设置该值时是锁定值：Claude Code 启动时会移除冲突的开发者设置；在项目/本地设置中指定会被忽略。",
        version_floor: None,
        depends_on: &[],
    },
    FieldRule {
        key: MASKED_KEY,
        label: "OTel 接收端请求头",
        category: "exporter_destination",
        semantics: Semantics::Value,
        sources: OTEL_SOURCES,
        default_state: "unset",
        impact: "OTLP 请求头，通常承载接收端认证凭据；去向由 OTEL_EXPORTER_OTLP_ENDPOINT 决定。该值属于凭据：Lintel 一律遮蔽，不读取、不显示、也不写入任何计划输出。",
        version_floor: None,
        depends_on: &[],
    },
    FieldRule {
        key: "OTEL_METRICS_INCLUDE_ACCOUNT_UUID",
        label: "指标包含账号 UUID",
        category: "identity_attribute",
        semantics: Semantics::DefaultOn,
        sources: OTEL_SOURCES,
        default_state: "on",
        impact: "在导出的指标标签中加入账号 UUID（默认包含）；写成 “false” 可以排除，从而减少指标中的账号身份信息。",
        version_floor: None,
        depends_on: &[],
    },
    FieldRule {
        key: "OTEL_METRICS_INCLUDE_SESSION_ID",
        label: "指标包含 session ID",
        category: "identity_attribute",
        semantics: Semantics::DefaultOn,
        sources: OTEL_SOURCES,
        default_state: "on",
        impact: "在导出的指标标签中加入 session ID（默认包含）；写成 “false” 可以排除。",
        version_floor: None,
        depends_on: &[],
    },
    FieldRule {
        key: "OTEL_METRICS_INCLUDE_VERSION",
        label: "指标包含 CLI 版本",
        category: "identity_attribute",
        semantics: Semantics::Boolean,
        sources: OTEL_SOURCES,
        default_state: "off",
        impact: "在导出的指标标签中加入 CLI 版本号（默认不包含）。",
        version_floor: None,
        depends_on: &[],
    },
    FieldRule {
        key: "OTEL_METRICS_INCLUDE_ENTRYPOINT",
        label: "指标包含入口类型",
        category: "identity_attribute",
        semantics: Semantics::Boolean,
        sources: OTEL_SOURCES,
        default_state: "off",
        impact: "在导出的指标标签中加入入口类型（默认不包含）。",
        version_floor: Some("2.1.152"),
        depends_on: &[],
    },
    FieldRule {
        key: "OTEL_METRICS_INCLUDE_REPOSITORY",
        label: "指标包含仓库信息",
        category: "identity_attribute",
        semantics: Semantics::Boolean,
        sources: OTEL_SOURCES,
        default_state: "off",
        impact: "在导出的指标标签中加入仓库信息（vcs.* 属性，默认不包含）。",
        version_floor: Some("2.1.269"),
        depends_on: &[],
    },
    FieldRule {
        key: "CLAUDE_CODE_ENABLE_FEEDBACK_SURVEY_FOR_OTEL",
        label: "调查改走自设 OTel",
        category: "feedback",
        semantics: Semantics::Boolean,
        sources: OTEL_SOURCES,
        default_state: "off",
        impact: "在非必要流量被阻断时，把质量调查路由到用户自己的 OTel collector，而不是发往 Anthropic；接收端由用户自己的 exporter 决定。",
        version_floor: None,
        depends_on: &[],
    },
    FieldRule {
        key: "CLAUDE_CODE_DISABLE_WEB_FETCH",
        label: "关闭 WebFetch 工具",
        category: "webfetch",
        semantics: Semantics::Boolean,
        sources: ANY_SOURCES,
        default_state: "off",
        impact: "整个关闭 WebFetch 工具。取舍：WebFetch 的安全预检查（向 Anthropic 发送目标 hostname 以判断可抓取性）随之消失，关闭后没有本地替代检查。该变量在任何来源都生效。",
        version_floor: Some("2.1.285"),
        depends_on: &[],
    },
];

/// The complete versioned field catalog. lib.rs `inspect`/`baseline`/`drift`
/// iterate `fields()`, so a new entry is listed there without further wiring.
pub(crate) fn catalog() -> &'static [FieldRule] {
    CATALOG
}

pub(crate) fn fields() -> impl Iterator<Item = (&'static str, &'static str)> {
    CATALOG.iter().map(|r| (r.key, r.label))
}

fn rule_of(key: &str) -> Option<&'static FieldRule> {
    CATALOG.iter().find(|r| r.key == key)
}

/// Validate an explicit `set` value before it can reach a plan. Rejecting here
/// keeps the plan from carrying a value that the product would ignore.
fn validate_custom_value(rule: &FieldRule, text: &str) -> Result<()> {
    // No value may smuggle control characters or a newline into settings.json.
    if text.chars().any(|c| c.is_control()) {
        return Err(err("invalid_request", "set 值不能包含控制字符或换行"));
    }
    match rule.key {
        CONTENT_LIMIT_KEY => {
            if !text.bytes().all(|b| b.is_ascii_digit()) {
                return Err(err("invalid_request", "内容长度上限必须是纯数字（字节数）"));
            }
        }
        "OTEL_EXPORTER_OTLP_ENDPOINT" => {
            if text.trim() != text || text.contains(' ') {
                return Err(err(
                    "invalid_request",
                    "接收端地址不能含空白；请给出完整 URL（例如 https://collector.example:4318）",
                ));
            }
        }
        "OTEL_LOGS_EXPORTER" | "OTEL_TRACES_EXPORTER" | "OTEL_METRICS_EXPORTER" => {
            // Prometheus is documented for metrics only; admitting it for logs
            // or traces would write a selector the product ignores.
            let allowed: &[&str] = if rule.key == "OTEL_METRICS_EXPORTER" {
                METRICS_EXPORTER_TOKENS
            } else {
                SIGNAL_EXPORTER_TOKENS
            };
            if !allowed.contains(&text) {
                return Err(err(
                    "invalid_request",
                    if rule.key == "OTEL_METRICS_EXPORTER" {
                        "metrics exporter 只能取 otlp / prometheus / console / none"
                    } else {
                        "logs/traces exporter 只能取 otlp / console / none"
                    },
                ));
            }
        }
        "OTEL_LOG_RAW_API_BODIES" => {
            let dir = match text {
                "0" | "1" => return Ok(()),
                _ => text.strip_prefix("file:").unwrap_or(""),
            };
            if dir.is_empty() || dir.trim() != dir {
                return Err(err(
                    "invalid_request",
                    "原始正文记录只能取 0 / 1 / file:<本地目录>；目录不能为空或含首尾空白",
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

fn version(s: &str) -> Option<[u32; 3]> {
    let parts: Vec<_> = s.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.bytes().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    Some([
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ])
}

/// Read only known native-install / npm metadata. An arbitrary executable name
/// or the newest documentation does not identify the installed product version.
pub(crate) fn product(executable: Option<&str>) -> Value {
    if let Some(exe) = executable {
        let path = Path::new(exe);
        if path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|s| s == "versions")
            && path
                .parent()
                .and_then(Path::parent)
                .and_then(Path::file_name)
                .is_some_and(|s| s == "claude")
        {
            if let Some(v) = path
                .file_name()
                .and_then(|s| s.to_str())
                .filter(|v| version(v).is_some())
            {
                return json!({"version":v,"source":"native_version_path","executable":exe});
            }
        }
        if path.file_name().is_some_and(|s| s == "cli.js") {
            if let Some(dir) = path.parent() {
                let metadata = dir.join("package.json");
                if let Ok(doc) = crate::storage::load(&metadata) {
                    if doc["name"] == "@anthropic-ai/claude-code" {
                        if let Some(v) = doc["version"].as_str().filter(|v| version(v).is_some()) {
                            return json!({"version":v,"source":"npm_package","executable":exe});
                        }
                    }
                }
            }
        }
    }
    json!({"version":null,"source":"unknown","executable":executable})
}

pub(crate) fn environment(mut e: Value, executable: Option<String>) -> Value {
    let p = product(executable.as_deref());
    e["executable"] = json!(executable);
    e["product_version"] = p["version"].clone();
    e["product_evidence"] = p;
    e
}

/// The three verified nonempty switches: any non-empty spelling is in effect.
const NONEMPTY_KEYS: [&str; 3] = [
    "DISABLE_TELEMETRY",
    "DISABLE_ERROR_REPORTING",
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
];

/// Report whether a variable is in effect under its verified reading. The
/// direction is carried by `semantics`/`default_state`: for a privacy switch
/// "in effect" means protective, while for an OTel content field it means the
/// content or destination is being recorded. `default_state` is the official
/// default; `disabled` is the current observation.
///
/// None means an undocumented spelling; do not call it configured.
pub(crate) fn disabled(key: &str, value: &Value) -> Option<bool> {
    let text = value.as_str();
    let rule = rule_of(key);
    // The verified privacy switches keep their original reading byte for byte,
    // including a non-string value never counting as configured.
    if rule.is_none_or(|r| r.category == "privacy_switch") {
        let Some(s) = text else { return Some(false) };
        if NONEMPTY_KEYS.contains(&key) {
            return Some(!s.is_empty());
        }
        return match s.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "" | "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        };
    }
    let semantics = rule.unwrap().semantics;
    let Some(s) = text else {
        // An absent key follows its documented default. A non-string value is
        // neither absent nor a documented spelling, so it stays unverified.
        // (The settings reader refuses non-string catalog values before a plan
        // or assessment is built; this is the fail-closed backstop.)
        return if value.is_null() {
            Some(semantics == Semantics::DefaultOn)
        } else {
            None
        };
    };
    match semantics {
        // DefaultOn: empty means the default (on) is in effect.
        Semantics::DefaultOn => match s.to_ascii_lowercase().as_str() {
            "" | "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" => Some(false),
            _ => None,
        },
        // RawApiBodies: empty means the default (off). `file:<dir>` records
        // complete bodies locally; a bare `file:` names no directory and stays
        // an unverified spelling.
        Semantics::RawApiBodies => match s {
            "" | "0" => Some(false),
            "1" => Some(true),
            other => match other.strip_prefix("file:") {
                Some(dir) if !dir.is_empty() => Some(true),
                _ => None,
            },
        },
        // A free value carries content, not a switch; it is "set" or not.
        Semantics::Value => Some(!s.is_empty()),
        _ => match s.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "" | "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        },
    }
}
pub(crate) fn setting_status(key: &str, value: &Value) -> &'static str {
    match disabled(key, value) {
        Some(true) => "configured",
        Some(false) => "unchanged",
        None => "uncertain",
    }
}
fn modern(product: &Value) -> Option<bool> {
    let v = version(product["version"].as_str()?)?;
    // The verified rule family is Claude Code 2.1; unknown future families must
    // not inherit the newest compatibility promise automatically.
    if v[0] != 2 || v[1] != 1 {
        return None;
    }
    Some(v[2] >= 283)
}
fn telemetry_needed(product: &Value, trusted: &str) -> bool {
    modern(product) != Some(true) || trusted != "not_required"
}

pub(crate) fn assessment(doc: &Value, product: &Value, trusted: &str) -> Value {
    let mut blockers = vec![];
    for key in REMOTE_KEYS {
        let state = disabled(key, &doc["env"][key]);
        if state == Some(false) {
            continue;
        }
        let dependency = if [
            "DISABLE_GROWTHBOOK",
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
        ]
        .contains(&key)
        {
            "blocked"
        } else if !telemetry_needed(product, trusted) {
            continue;
        } else if modern(product) == Some(false) || trusted == "required" {
            "blocked"
        } else {
            "conditional"
        };
        let status = if state.is_none() {
            "conditional"
        } else {
            dependency
        };
        let reason = if state.is_none() {
            "此布尔值写法没有已核验语义，需要核对来源"
        } else if key == "DISABLE_GROWTHBOOK" || key == "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC" {
            "关闭 feature-flag 获取，与 Remote Control 冲突"
        } else if modern(product) == Some(false) {
            "2.1.283 之前的版本需要 feature-flag 获取；2.1.154 之前错误提示不同"
        } else if trusted == "required" {
            "已声明组织要求 Trusted Devices，需要解除该开关"
        } else {
            "产品版本或组织 Trusted Devices 条件未确认，不能保证保留 Remote Control"
        };
        // `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` is nonempty (even "0" or
        // "false" disables) and, per the official env-vars doc, also disables
        // auto-updates, release notes and feature flags. Record that collateral
        // impact on the plan/assessment without changing settings semantics.
        let collateral = if key == "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC" {
            json!([{
                "effect":"additional_impacts",
                "detail":"官方 env-vars 记载此变量为非空即生效（0/false 也会关闭），并同时关闭自动更新、release notes 与 feature flag 获取；关闭语义按 nonempty 解释，不改变 Remote Control 保证。",
                "source":"https://code.claude.com/docs/en/env-vars"
            }])
        } else {
            json!([])
        };
        blockers.push(json!({"key":key,"value":doc["env"][key],"status":status,"reason":reason,"source":"user_settings","collateral":collateral}));
    }
    let status = if blockers.iter().any(|b| b["status"] == "blocked") {
        "blocked"
    } else if !blockers.is_empty() || modern(product).is_none() {
        "conditional"
    } else {
        "configuration_compatible"
    };
    let summary = match status {
        "blocked" => "当前 user settings 存在 Remote Control 冲突；保留选项尚未满足。",
        "conditional" => "Remote Control 条件尚未确认；未知版本、取值或组织条件不能当作可用证明。",
        _ => "此版本条件下 user settings 未发现 Remote Control 开关冲突；账号、组织权限及实际运行仍未验证。",
    };
    let rules: Vec<_> = catalog()
        .iter()
        .map(|r| {
            let raw = &doc["env"][r.key];
            // Credential-bearing values are never echoed: presence is reported,
            // the value is not.
            let masked = r.key == MASKED_KEY;
            let mut item = json!({
                "key":r.key,"label":r.label,
                "value":if masked { Value::Null } else { raw.clone() },
                "disabled":disabled(r.key,raw),
                "semantics":r.semantics.as_str(),
                "scope":"registered_root_user_settings","effect_timing":"next_launch",
                "category":r.category,
                "sources":{"turn_on":r.sources.turn_on,"turn_off":r.sources.turn_off},
                "default_state":r.default_state,"impact":r.impact,
                "version_floor":r.version_floor,"masked":masked,
            });
            if masked {
                item["value_masked"] = json!(true);
            }
            if r.category.starts_with("content_") {
                item["depends_on"] = json!(r.depends_on);
            }
            // `file:<dir>` is a local fact worth naming: the complete bodies are
            // on this disk. That is not the same fact as "uploaded".
            if r.semantics == Semantics::RawApiBodies {
                if let Some(dir) = raw
                    .as_str()
                    .and_then(|s| s.strip_prefix("file:"))
                    .filter(|d| !d.is_empty())
                {
                    item["local_dump"] = json!(dir);
                    item["local_dump_note"] =
                        json!("“file:<dir>” 会把未截断的完整会话正文写入该本地目录（日志事件只带 body_ref 路径）；正文落盘 ≠ 已上传，二者是不同事实。");
                }
            }
            item
        })
        .collect();
    json!({"rule_version":RULE,"supported_presets":["preserve","reduce","custom"],"product":product,"rules":rules,"remote_control":{
        "status":status,"summary":summary,"blockers":blockers,"trusted_devices":trusted,
        "trusted_devices_source":if trusted=="unknown"{"unverified"}else{"user_declared"},
        "version_family":match modern(product){Some(true)=>"2.1.283+",Some(false)=>"before_2.1.283",None=>"unknown"},
        "runtime_verified":false,
        "unverified":["shell/project/managed settings and running processes","eligible claude.ai subscription and login","direct Anthropic endpoint","organization enablement and device enrollment"]
    }})
}

pub(crate) fn trusted_devices(request: &Value) -> Result<&str> {
    match request.get("trusted_devices") {
        None => Ok("unknown"),
        Some(v) => v
            .as_str()
            .filter(|s| ["unknown", "required", "not_required"].contains(s))
            .ok_or_else(|| {
                err(
                    "invalid_request",
                    "trusted_devices 必须是 unknown / required / not_required",
                )
            }),
    }
}

pub(crate) fn plan(
    doc: &Value,
    product: &Value,
    request: &Value,
    path: &Path,
) -> Result<(Value, Value)> {
    let preset = request["preset"].as_str().unwrap_or("");
    if !["preserve", "reduce", "custom"].contains(&preset) {
        return Err(err("invalid_preset", "未知保护方案"));
    }
    let custom = if preset == "custom" {
        let choices = request.get("custom_settings").cloned().unwrap_or(json!({}));
        let choices_object = choices
            .as_object()
            .ok_or_else(|| err("invalid_request", "custom_settings 必须是字段 action 对象"))?;
        for (key, action) in choices_object {
            let rule = rule_of(key)
                .ok_or_else(|| err("invalid_request", "custom_settings 包含未识别的字段"))?;
            // The credential-bearing field is readable-but-not-writable: this
            // architecture carries the mutation payload in the changes list that
            // is also the plan output, so a masked value could not be written
            // without printing it. Removal (a null write) is the one honest
            // mutation, and it is available.
            if rule.key == MASKED_KEY
                && !action
                    .as_str()
                    .is_some_and(|s| ["keep", "remove", "disable"].contains(&s))
            {
                return Err(err(
                    "masked_setting_not_writable",
                    "该字段通常承载接收端凭据；本工具不记录其值，因此不能写入或替换它，也不能把它放进变更清单。可用的动作是 keep、remove 或 disable（等同移除），新值请在该值的归属来源中提供。",
                ));
            }
            // The verified privacy switches keep their historical action set; a
            // DISABLE_* flag's "enable" would be ambiguous (on means off), so it
            // is still refused instead of being redefined here.
            if rule.category == "privacy_switch" {
                if !action
                    .as_str()
                    .is_some_and(|s| ["keep", "disable", "remove"].contains(&s))
                {
                    return Err(err(
                        "invalid_request",
                        "既有隐私开关的 action 必须是 keep / disable / remove",
                    ));
                }
                continue;
            }
            let action = action
                .as_str()
                .filter(|s| ["keep", "disable", "enable", "remove", "set"].contains(s));
            let Some(action) = action else {
                return Err(err(
                    "invalid_request",
                    "字段 action 必须是 keep / disable / enable / remove / set",
                ));
            };
            if action == "enable" && rule.semantics == Semantics::Value {
                return Err(err(
                    "invalid_request",
                    "自由取值字段不能 enable；请用 set 指定取值",
                ));
            }
            if action == "set"
                && !matches!(rule.semantics, Semantics::Value | Semantics::RawApiBodies)
            {
                return Err(err(
                    "invalid_request",
                    "布尔/默认开启字段不支持 set；请用 enable / disable",
                ));
            }
        }
        choices
    } else {
        if request.get("custom_settings").is_some() {
            return Err(err(
                "invalid_request",
                "custom_settings 只适用于 custom 方案",
            ));
        }
        json!({})
    };
    // Explicit values are a separate, custom-only channel: they are admitted
    // only for fields whose action is exactly `set`, so a value can never be
    // applied without naming the field and the action it belongs to.
    let mut custom_values: Map<String, Value> = Map::new();
    match (preset, request.get("custom_values")) {
        (_, None) => {}
        ("custom", Some(raw)) => {
            let values = raw
                .as_object()
                .filter(|v| !v.is_empty())
                .ok_or_else(|| err("invalid_request", "custom_values 必须是非空的字段取值对象"))?;
            for (key, value) in values {
                if key == MASKED_KEY {
                    return Err(err(
                        "masked_setting_not_writable",
                        "该字段通常承载接收端凭据；Lintel 不记录也不写入它的值。",
                    ));
                }
                let rule = rule_of(key)
                    .ok_or_else(|| err("invalid_request", "custom_values 包含未识别的字段"))?;
                if custom[rule.key].as_str() != Some("set") {
                    return Err(err(
                        "invalid_request",
                        "custom_values 只允许用于 action 为 set 的字段",
                    ));
                }
                let text = value
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| err("invalid_request", "set 需要非空字符串值"))?;
                validate_custom_value(rule, text)?;
                custom_values.insert(rule.key.to_string(), json!(text));
            }
        }
        _ => return Err(err("invalid_request", "custom_values 只适用于 custom 方案")),
    }
    // Every `set` needs its value, whether custom_values was supplied or not.
    for (key, action) in custom.as_object().unwrap() {
        if action == "set" && !custom_values.contains_key(key) {
            return Err(err(
                "invalid_request",
                "set 动作需要在 custom_values 中给出该字段的取值",
            ));
        }
    }
    let keep = match request.get("keep_remote_control") {
        None => false,
        Some(v) => v
            .as_bool()
            .ok_or_else(|| err("invalid_request", "keep_remote_control 必须是 boolean"))?,
    };
    let trusted = trusted_devices(request)?;
    let releases: Vec<&str> = match request.get("release_settings") {
        None => vec![],
        Some(v) => v
            .as_array()
            .ok_or_else(|| err("invalid_request", "release_settings 必须是字段列表"))?
            .iter()
            .map(|v| {
                v.as_str()
                    .filter(|k| REMOTE_KEYS.contains(k))
                    .ok_or_else(|| {
                        err(
                            "invalid_request",
                            "只能显式解除已识别的 Remote Control 开关",
                        )
                    })
            })
            .collect::<Result<_>>()?,
    };
    if preset == "custom" && !releases.is_empty() {
        return Err(err(
            "invalid_request",
            "custom 方案请通过字段 remove 明确删除，不接受 release_settings",
        ));
    }
    if !keep && !releases.is_empty() {
        return Err(err(
            "invalid_request",
            "解除功能冲突需要选择保留 Remote Control",
        ));
    }
    let current = assessment(doc, product, trusted);
    for key in &releases {
        if !current["remote_control"]["blockers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["key"] == *key)
        {
            return Err(err(
                "invalid_request",
                "此字段没有当前可解除的功能冲突，请重新预览",
            ));
        }
    }
    let mut after = doc.clone();
    if after.get("env").is_none() {
        after["env"] = json!({})
    }
    let mut changes = vec![];
    for rule in catalog() {
        let (key, label) = (rule.key, rule.label);
        // Presets only ever act on the verified privacy switches and on the
        // explicitly released Remote Control keys; every other field (including
        // the whole OTel family) is reachable through `custom` alone.
        let action = if preset == "custom" {
            custom[key].as_str().unwrap_or("keep")
        } else if releases.contains(&key) {
            "remove"
        } else if FLAGS.iter().any(|(k, _)| *k == key)
            && (preset == "reduce"
                || [
                    "DISABLE_ERROR_REPORTING",
                    "CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY",
                ]
                .contains(&key))
            && !(keep && key == "DISABLE_TELEMETRY" && telemetry_needed(product, trusted))
        {
            "disable"
        } else {
            "keep"
        };
        let present = doc["env"].get(key).is_some();
        let state = disabled(key, &doc["env"][key]);
        let value = match action {
            "keep" => continue,
            "remove" => {
                if !present {
                    continue;
                }
                Value::Null
            }
            "disable" => match rule.semantics {
                Semantics::DefaultOn => {
                    if state == Some(false) {
                        continue;
                    }
                    json!("false")
                }
                Semantics::RawApiBodies => {
                    if state != Some(true) {
                        continue;
                    }
                    json!("0")
                }
                // A free value has no "off" spelling; removing the key is the
                // only state that stops it.
                Semantics::Value => {
                    if !present {
                        continue;
                    }
                    Value::Null
                }
                _ => {
                    if state == Some(true) {
                        continue;
                    }
                    json!("1")
                }
            },
            "enable" => match rule.semantics {
                // Default-on fields are enabled by the absence of the key.
                Semantics::DefaultOn => {
                    if state == Some(true) {
                        continue;
                    }
                    Value::Null
                }
                _ => {
                    if state == Some(true) {
                        continue;
                    }
                    json!("1")
                }
            },
            "set" => {
                let value = custom_values[key].clone();
                if doc["env"][key] == value {
                    continue;
                }
                value
            }
            _ => continue,
        };
        let mut change =
            json!({"key":key,"label":label,"before":doc["env"][key],"after":value,"path":path});
        if rule.semantics == Semantics::DefaultOn && action == "enable" {
            change["restores_default"] = json!(true);
            change["label"] = json!(format!("{label}（删除该键，恢复默认开启）"));
        }
        if key == MASKED_KEY {
            // The value is never recorded. The deletion is real, the original
            // bytes are not, so say so instead of implying a restore.
            change["before"] = Value::Null;
            change["after"] = Value::Null;
            change["masked"] = json!(true);
            change["note"] =
                json!("该字段的当前值不会被 Lintel 记录，因此也无法据此恢复；移除后需要由该值的归属来源重新提供。");
        }
        changes.push(change);
        if value.is_null() {
            after["env"].as_object_mut().unwrap().remove(key);
        } else {
            after["env"][key] = value;
        }
    }
    let mut policy = assessment(&after, product, trusted);
    policy["preset"] = json!(preset);
    if preset == "custom" {
        policy["custom_settings"] = custom;
    }
    policy["keep_remote_control"] = json!(keep);
    policy["release_settings"] = json!(releases);
    policy["current_remote_control"] = current["remote_control"].clone();
    Ok((json!(changes), policy))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonessential_traffic_records_documented_collateral_impact() {
        // The total switch reports the official extra impacts; it stays
        // semver-nonempty (0/false disables) and does not change settings
        // semantics or Remote Control guarantees.
        for spelling in ["1", "0", "false", "true", "disable"] {
            let doc = json!({"env":{"CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC":spelling}});
            let a = assessment(&doc, &json!({"version":"2.1.283"}), "not_required");
            let rule = a["remote_control"]["blockers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|b| b["key"] == "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC")
                .unwrap();
            assert_eq!(rule["collateral"].as_array().unwrap().len(), 1, "{spelling}");
            assert_eq!(rule["collateral"][0]["effect"], "additional_impacts");
            assert_eq!(
                disabled("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", &json!(spelling)),
                Some(true),
                "{spelling}"
            );
        }
        // Other keys, including granular FLAGS, never inherit that collateral.
        let doc = json!({"env":{"DISABLE_TELEMETRY":"1","DISABLE_GROWTHBOOK":"1"}});
        let a = assessment(&doc, &json!({"version":"2.1.283"}), "not_required");
        for blocker in a["remote_control"]["blockers"].as_array().unwrap() {
            assert!(
                blocker["collateral"].as_array().unwrap().is_empty(),
                "{}",
                blocker["key"]
            );
        }
    }

    #[test]
    fn values_follow_each_rule() {
        // The verified privacy switches keep their original matrix exactly.
        for rule in catalog().iter().filter(|r| r.category == "privacy_switch") {
            let key = rule.key;
            for (value, on) in [
                (Value::Null, false),
                (json!(""), false),
                (json!("0"), true),
                (json!("false"), true),
                (json!("1"), true),
            ] {
                let nonempty = [
                    "DISABLE_TELEMETRY",
                    "DISABLE_ERROR_REPORTING",
                    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
                ]
                .contains(&key);
                assert_eq!(
                    disabled(key, &value),
                    Some(if nonempty { on } else { value == "1" }),
                    "{key}: {value}"
                );
            }
        }
        assert_eq!(disabled("DISABLE_GROWTHBOOK", &json!("TrUe")), Some(true));
        assert_eq!(disabled("DO_NOT_TRACK", &json!("other")), None);
    }
    #[test]
    fn otel_semantics_matrix_covers_every_new_family() {
        // DefaultOn: absent means enabled; only "false"/"0" exclude it.
        for key in [
            "OTEL_METRICS_INCLUDE_ACCOUNT_UUID",
            "OTEL_METRICS_INCLUDE_SESSION_ID",
        ] {
            for (value, expected) in [
                (Value::Null, Some(true)),
                (json!(""), Some(true)),
                (json!("false"), Some(false)),
                (json!("FALSE"), Some(false)),
                (json!("0"), Some(false)),
                (json!("1"), Some(true)),
                (json!("true"), Some(true)),
                (json!("YES"), Some(true)),
                (json!("On"), Some(true)),
                (json!("maybe"), None),
                (json!(0), None),
            ] {
                assert_eq!(disabled(key, &value), expected, "{key}: {value}");
            }
        }
        // RawApiBodies: absent is off, "1" records inline, "file:<dir>" dumps
        // complete bodies into a local directory.
        for (value, expected) in [
            (Value::Null, Some(false)),
            (json!(""), Some(false)),
            (json!("0"), Some(false)),
            (json!("1"), Some(true)),
            (json!("file:/tmp/otel-bodies"), Some(true)),
            (json!("file:"), None),
            (json!("2"), None),
            (json!("yes"), None),
        ] {
            assert_eq!(
                disabled("OTEL_LOG_RAW_API_BODIES", &value),
                expected,
                "{value}"
            );
        }
        // Content-recording booleans follow the documented spellings, and an
        // unverified spelling stays unknown rather than becoming "off".
        for key in [
            "CLAUDE_CODE_ENABLE_TELEMETRY",
            "OTEL_LOG_USER_PROMPTS",
            "OTEL_LOG_ASSISTANT_RESPONSES",
            "OTEL_LOG_TOOL_CONTENT",
            "OTEL_LOG_TOOL_DETAILS",
            "OTEL_LOG_MANAGED_SETTINGS",
            "OTEL_METRICS_INCLUDE_VERSION",
            "OTEL_METRICS_INCLUDE_ENTRYPOINT",
            "OTEL_METRICS_INCLUDE_REPOSITORY",
            "CLAUDE_CODE_ENABLE_FEEDBACK_SURVEY_FOR_OTEL",
            "CLAUDE_CODE_DISABLE_WEB_FETCH",
        ] {
            for (value, expected) in [
                (Value::Null, Some(false)),
                (json!(""), Some(false)),
                (json!("1"), Some(true)),
                (json!("TrUe"), Some(true)),
                (json!("yes"), Some(true)),
                (json!("on"), Some(true)),
                (json!("0"), Some(false)),
                (json!("false"), Some(false)),
                (json!("no"), Some(false)),
                (json!("off"), Some(false)),
                (json!("sometimes"), None),
            ] {
                assert_eq!(disabled(key, &value), expected, "{key}: {value}");
            }
        }
        // Free-value fields report set/unset, and never echo the content.
        for key in [
            "OTEL_LOGS_EXPORTER",
            "OTEL_TRACES_EXPORTER",
            "OTEL_METRICS_EXPORTER",
            "OTEL_EXPORTER_OTLP_ENDPOINT",
            MASKED_KEY,
            CONTENT_LIMIT_KEY,
        ] {
            for (value, expected) in [
                (Value::Null, Some(false)),
                (json!(""), Some(false)),
                (json!("none"), Some(true)),
                (json!("61440"), Some(true)),
                (json!("https://collector.example:4318"), Some(true)),
            ] {
                assert_eq!(disabled(key, &value), expected, "{key}: {value}");
            }
        }
    }
    #[test]
    fn catalog_carries_the_verified_metadata_for_every_field() {
        let keys: Vec<&str> = catalog().iter().map(|r| r.key).collect();
        assert_eq!(keys.len(), 27);
        let mut unique = keys.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), keys.len(), "catalog keys must be unique");
        // fields() is the shared listing surface; it must stay in step.
        assert_eq!(fields().count(), keys.len());
        for rule in catalog() {
            assert!(!rule.label.is_empty(), "{}", rule.key);
            assert!(!rule.impact.is_empty(), "{}", rule.key);
            assert!(!rule.category.is_empty(), "{}", rule.key);
            assert!(!rule.default_state.is_empty(), "{}", rule.key);
            assert!(
                !rule.sources.turn_on.is_empty() && !rule.sources.turn_off.is_empty(),
                "{}",
                rule.key
            );
            for source in rule
                .sources
                .turn_on
                .iter()
                .chain(rule.sources.turn_off.iter())
            {
                assert!(
                    ["shell", "user", "managed", "project", "project_local"].contains(source),
                    "{}: {source}",
                    rule.key
                );
            }
            // The OTel family is ignored in a repository apart from its off
            // values; the verified privacy switches remain user-settings-only.
            // WebFetch is the documented exception: it works from any source.
            if rule.category != "privacy_switch" && rule.key != "CLAUDE_CODE_DISABLE_WEB_FETCH" {
                assert_eq!(
                    rule.sources.turn_on,
                    ["shell", "user", "managed"],
                    "{}",
                    rule.key
                );
                assert_eq!(
                    rule.sources.turn_off,
                    ["shell", "user", "managed", "project", "project_local"],
                    "{}",
                    rule.key
                );
            }
            if rule.category == "privacy_switch" {
                assert_eq!(rule.sources.turn_on, rule.sources.turn_off, "{}", rule.key);
                assert_eq!(rule.sources.turn_on.len(), 3, "{}", rule.key);
            }
            if rule.key == "CLAUDE_CODE_DISABLE_WEB_FETCH" {
                assert_eq!(
                    rule.sources.turn_on,
                    ["shell", "user", "managed", "project", "project_local"],
                    "{}",
                    rule.key
                );
                assert_eq!(rule.sources.turn_on, rule.sources.turn_off, "{}", rule.key);
            }
        }
    }
    #[test]
    fn assessment_reports_catalog_metadata_and_flags_the_credential_field() {
        let secret = "Bearer synthetic-abcdef0123456789";
        let doc = json!({"env":{
            MASKED_KEY:secret,
            "OTEL_LOG_ASSISTANT_RESPONSES":"0",
            "OTEL_LOG_USER_PROMPTS":"1",
            "OTEL_LOG_RAW_API_BODIES":"file:/synthetic/otel-bodies",
            "OTEL_METRICS_INCLUDE_ACCOUNT_UUID":"false",
            "CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH":"61440",
        }});
        let a = assessment(&doc, &json!({"version":"2.1.283"}), "not_required");
        assert_eq!(a["rule_version"], RULE);
        let rules = a["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 27);
        // The credential value never appears anywhere in the assessment.
        assert!(!a.to_string().contains(secret));
        let headers = rules.iter().find(|r| r["key"] == MASKED_KEY).unwrap();
        assert!(headers["value"].is_null());
        assert_eq!(headers["value_masked"], true);
        assert_eq!(headers["masked"], true);
        assert_eq!(headers["category"], "exporter_destination");
        assert_eq!(headers["semantics"], "value");
        // Fallback semantics are stated as a dependency, not inferred.
        let responses = rules
            .iter()
            .find(|r| r["key"] == "OTEL_LOG_ASSISTANT_RESPONSES")
            .unwrap();
        assert_eq!(responses["depends_on"], json!(["OTEL_LOG_USER_PROMPTS"]));
        assert_eq!(responses["version_floor"], "2.1.193");
        assert_eq!(responses["category"], "content_responses");
        assert_eq!(responses["default_state"], "redacted");
        assert_eq!(responses["disabled"], false);
        let prompts = rules
            .iter()
            .find(|r| r["key"] == "OTEL_LOG_USER_PROMPTS")
            .unwrap();
        assert_eq!(prompts["category"], "content_prompts");
        assert_eq!(
            prompts["depends_on"],
            json!(["CLAUDE_CODE_ENABLE_TELEMETRY"])
        );
        let tool_content = rules
            .iter()
            .find(|r| r["key"] == "OTEL_LOG_TOOL_CONTENT")
            .unwrap();
        assert_eq!(tool_content["depends_on"], json!(["tracing"]));
        assert!(tool_content["impact"].as_str().unwrap().contains("tracing"));
        // A local dump is reported as a local fact, explicitly not an upload.
        let raw = rules
            .iter()
            .find(|r| r["key"] == "OTEL_LOG_RAW_API_BODIES")
            .unwrap();
        assert_eq!(raw["local_dump"], "/synthetic/otel-bodies");
        assert_eq!(raw["semantics"], "raw_api_bodies");
        assert!(raw.get("depends_on").is_some());
        assert!(raw["local_dump_note"].as_str().unwrap().contains("落盘"));
        // A default-on field reports its official default separately from the
        // current observation.
        let uuid = rules
            .iter()
            .find(|r| r["key"] == "OTEL_METRICS_INCLUDE_ACCOUNT_UUID")
            .unwrap();
        assert_eq!(uuid["semantics"], "default_on");
        assert_eq!(uuid["default_state"], "on");
        assert_eq!(uuid["disabled"], false);
        assert_eq!(
            rules
                .iter()
                .find(|r| r["key"] == "OTEL_METRICS_INCLUDE_ACCOUNT_UUID")
                .unwrap()["sources"]["turn_off"],
            json!(["shell", "user", "managed", "project", "project_local"])
        );
        // Absent DefaultOn reads as on; absent RawApiBodies is off with no dump.
        let absent = assessment(&json!({}), &json!({}), "unknown");
        let absent_rules = absent["rules"].as_array().unwrap();
        assert_eq!(
            absent_rules
                .iter()
                .find(|r| r["key"] == "OTEL_METRICS_INCLUDE_SESSION_ID")
                .unwrap()["disabled"],
            true
        );
        let absent_raw = absent_rules
            .iter()
            .find(|r| r["key"] == "OTEL_LOG_RAW_API_BODIES")
            .unwrap();
        assert_eq!(absent_raw["disabled"], false);
        assert!(absent_raw.get("local_dump").is_none());
        // A legacy privacy rule still reports its original semantics string.
        let legacy = rules
            .iter()
            .find(|r| r["key"] == "DISABLE_TELEMETRY")
            .unwrap();
        assert_eq!(legacy["semantics"], "nonempty");
        assert_eq!(legacy["category"], "privacy_switch");
        assert_eq!(legacy["default_state"], "off");
    }
    #[test]
    fn remote_version_and_organization_matrix() {
        for v in [
            None,
            Some("2.1.153"),
            Some("2.1.154"),
            Some("2.1.282"),
            Some("2.1.283"),
            Some("3.0.0"),
        ] {
            for trusted in ["unknown", "required", "not_required"] {
                for key in REMOTE_KEYS {
                    for val in [
                        Value::Null,
                        json!(""),
                        json!("0"),
                        json!("false"),
                        json!("1"),
                    ] {
                        let p = json!({"version":v});
                        let d = json!({"env":{key:val}});
                        let a = assessment(&d, &p, trusted);
                        let enabled = disabled(key, &d["env"][key]) == Some(true);
                        let expected = enabled
                            && ([
                                "DISABLE_GROWTHBOOK",
                                "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
                            ]
                            .contains(&key)
                                || telemetry_needed(&p, trusted));
                        assert_eq!(
                            !a["remote_control"]["blockers"]
                                .as_array()
                                .unwrap()
                                .is_empty(),
                            expected,
                            "{v:?} {trusted} {key} {val}"
                        );
                        assert_eq!(a["remote_control"]["runtime_verified"], false);
                    }
                }
            }
        }
    }
    #[test]
    fn reduce_then_keep_requires_explicit_release() {
        let p = json!({"version":"2.1.282"});
        let r = json!({"preset":"reduce","keep_remote_control":false});
        let (changes, _) = plan(&json!({}), &p, &r, Path::new("/synthetic/settings.json")).unwrap();
        let mut d = json!({"env":{"UNRELATED":"keep"}});
        for c in changes.as_array().unwrap() {
            d["env"][c["key"].as_str().unwrap()] = c["after"].clone();
        }
        let mut keep = json!({"preset":"reduce","keep_remote_control":true});
        let (changes, result) = plan(&d, &p, &keep, Path::new("/synthetic/settings.json")).unwrap();
        assert!(changes.as_array().unwrap().is_empty());
        assert_eq!(result["remote_control"]["status"], "blocked");
        keep["release_settings"] = json!(["DISABLE_TELEMETRY"]);
        let (changes, result) = plan(&d, &p, &keep, Path::new("/synthetic/settings.json")).unwrap();
        assert_eq!(changes[0]["before"], "1");
        assert!(changes[0]["after"].is_null());
        assert_eq!(
            result["remote_control"]["status"],
            "configuration_compatible"
        );
        d["env"]["DISABLE_TELEMETRY"] = json!("external");
        assert_eq!(
            plan(&d, &p, &keep, Path::new("/synthetic/settings.json"))
                .unwrap()
                .0[0]["before"],
            "external"
        );
    }
    #[test]
    fn custom_actions_preserve_values_and_only_change_selected_fields() {
        let doc = json!({"env":{
            "DISABLE_TELEMETRY":"false", "DISABLE_ERROR_REPORTING":"0",
            "DISABLE_FEEDBACK_COMMAND":"false", "CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY":"TrUe",
            "DO_NOT_TRACK":"off", "DISABLE_GROWTHBOOK":"yes", "UNRELATED":"保留原值"
        },"permissions":{"allow":["Read"]}});
        let choices = json!({
            "DISABLE_TELEMETRY":"disable", "DISABLE_ERROR_REPORTING":"disable",
            "DISABLE_FEEDBACK_COMMAND":"disable", "CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY":"keep",
            "DO_NOT_TRACK":"remove", "DISABLE_GROWTHBOOK":"remove",
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC":"remove"
        });
        let (changes, result) = plan(
            &doc, &json!({"version":"2.1.283"}),
            &json!({"preset":"custom","custom_settings":choices,"keep_remote_control":true,"trusted_devices":"not_required"}),
            Path::new("/synthetic/settings.json"),
        ).unwrap();
        assert_eq!(changes.as_array().unwrap().len(), 3);
        assert_eq!(changes[0]["key"], "DISABLE_FEEDBACK_COMMAND");
        assert_eq!(changes[0]["before"], "false");
        assert_eq!(changes[0]["after"], "1");
        assert_eq!(changes[1]["before"], "off");
        assert!(changes[1]["after"].is_null());
        assert_eq!(changes[2]["before"], "yes");
        assert!(changes[2]["after"].is_null());
        assert_eq!(result["preset"], "custom");
        assert_eq!(result["custom_settings"], choices);
        assert_eq!(
            result["remote_control"]["status"],
            "configuration_compatible"
        );
        assert_eq!(
            result["supported_presets"],
            json!(["preserve", "reduce", "custom"])
        );
        for key in [
            "DISABLE_TELEMETRY",
            "DISABLE_ERROR_REPORTING",
            "CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY",
        ] {
            let rule = result["rules"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["key"] == key)
                .unwrap();
            assert_eq!(rule["value"], doc["env"][key]);
        }
    }
    #[test]
    fn custom_missing_choices_keep_and_explicit_remote_choices_are_not_overridden() {
        let doc =
            json!({"env":{"DISABLE_GROWTHBOOK":"false","DISABLE_TELEMETRY":"","UNRELATED":"keep"}});
        for request in [
            json!({"preset":"custom"}),
            json!({"preset":"custom","custom_settings":{},"release_settings":[]}),
        ] {
            let (changes, policy) = plan(
                &doc,
                &json!({"version":"2.1.283"}),
                &request,
                Path::new("/synthetic/settings.json"),
            )
            .unwrap();
            assert!(changes.as_array().unwrap().is_empty());
            assert_eq!(policy["custom_settings"], json!({}));
        }
        for (version, trusted, expected) in [
            ("2.1.282", "not_required", "blocked"),
            ("2.1.283", "required", "blocked"),
            ("2.1.283", "unknown", "conditional"),
            ("2.1.283", "not_required", "configuration_compatible"),
        ] {
            let (changes, policy) = plan(&doc, &json!({"version":version}),
                &json!({"preset":"custom","keep_remote_control":true,"trusted_devices":trusted,"custom_settings":{"DISABLE_TELEMETRY":"disable"}}),
                Path::new("/synthetic/settings.json")).unwrap();
            assert_eq!(changes.as_array().unwrap().len(), 1);
            assert_eq!(changes[0]["key"], "DISABLE_TELEMETRY");
            assert_eq!(changes[0]["after"], "1");
            assert_eq!(
                policy["remote_control"]["status"], expected,
                "{version} {trusted}"
            );
            assert_eq!(policy["keep_remote_control"], true);
        }
    }
    #[test]
    fn custom_choices_reject_unsupported_shapes_actions_and_preset_mixups() {
        for choices in [
            Value::Null,
            json!([]),
            json!({"UNKNOWN":"disable"}),
            json!({"DISABLE_TELEMETRY":{"action":"disable"}}),
            json!({"DISABLE_TELEMETRY":"enable"}),
            json!({"DISABLE_TELEMETRY":true}),
        ] {
            let error = plan(
                &json!({}),
                &json!({"version":"2.1.283"}),
                &json!({"preset":"custom","custom_settings":choices}),
                Path::new("/synthetic/settings.json"),
            )
            .unwrap_err();
            assert_eq!(error.code, "invalid_request", "{choices}");
        }
        for preset in ["preserve", "reduce"] {
            for choices in [json!({}), json!({"DISABLE_TELEMETRY":"keep"})] {
                assert_eq!(
                    plan(
                        &json!({}),
                        &json!({}),
                        &json!({"preset":preset,"custom_settings":choices}),
                        Path::new("/synthetic/settings.json")
                    )
                    .unwrap_err()
                    .code,
                    "invalid_request"
                );
            }
        }
        for keep in [false, true] {
            assert_eq!(plan(&json!({"env":{"DISABLE_TELEMETRY":"1"}}), &json!({"version":"2.1.282"}),
                &json!({"preset":"custom","keep_remote_control":keep,"release_settings":["DISABLE_TELEMETRY"]}),
                Path::new("/synthetic/settings.json")).unwrap_err().code, "invalid_request");
        }
        // The extended action set is admitted only where its semantics are
        // defined; the original privacy switches keep their three actions.
        for choices in [
            json!({"OTEL_METRICS_INCLUDE_VERSION":"set"}),
            json!({"OTEL_LOGS_EXPORTER":"enable"}),
            json!({"CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH":"enable"}),
            json!({"CLAUDE_CODE_ENABLE_TELEMETRY":"set"}),
            json!({"DISABLE_TELEMETRY":"enable"}),
            json!({"DO_NOT_TRACK":"set"}),
        ] {
            let error = plan(
                &json!({}),
                &json!({"version":"2.1.283"}),
                &json!({"preset":"custom","custom_settings":choices}),
                Path::new("/synthetic/settings.json"),
            )
            .unwrap_err();
            assert_eq!(error.code, "invalid_request", "{choices}");
        }
        // `set` without its value, and values without `set`, are both refused.
        for request in [
            json!({"preset":"custom","custom_settings":{"OTEL_LOGS_EXPORTER":"set"}}),
            json!({"preset":"custom","custom_settings":{"OTEL_LOGS_EXPORTER":"set"},"custom_values":{}}),
            json!({"preset":"custom","custom_settings":{"OTEL_LOGS_EXPORTER":"keep"},"custom_values":{"OTEL_LOGS_EXPORTER":"otlp"}}),
            json!({"preset":"custom","custom_settings":{"OTEL_LOGS_EXPORTER":"set"},"custom_values":{"UNKNOWN":"otlp"}}),
            json!({"preset":"custom","custom_settings":{"OTEL_LOGS_EXPORTER":"set"},"custom_values":{"OTEL_LOGS_EXPORTER":""}}),
            json!({"preset":"custom","custom_settings":{"OTEL_LOGS_EXPORTER":"set"},"custom_values":{"OTEL_LOGS_EXPORTER":7}}),
            json!({"preset":"preserve","custom_values":{"OTEL_LOGS_EXPORTER":"otlp"}}),
            json!({"preset":"reduce","custom_values":{"OTEL_LOGS_EXPORTER":"otlp"}}),
        ] {
            let error = plan(
                &json!({}),
                &json!({"version":"2.1.283"}),
                &request,
                Path::new("/synthetic/settings.json"),
            )
            .unwrap_err();
            assert_eq!(error.code, "invalid_request", "{request}");
        }
        // Documented value domains are checked before a plan can carry them.
        for (key, value) in [
            ("CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH", "6144a"),
            ("CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH", "-1"),
            ("OTEL_LOGS_EXPORTER", "sidecar"),
            ("OTEL_TRACES_EXPORTER", "otlp "),
            ("OTEL_METRICS_EXPORTER", "OTEL"),
            ("OTEL_LOGS_EXPORTER", "prometheus"),
            ("OTEL_TRACES_EXPORTER", "prometheus"),
            (
                "OTEL_EXPORTER_OTLP_ENDPOINT",
                "https://collector.example/a b",
            ),
            ("OTEL_EXPORTER_OTLP_ENDPOINT", " https://collector.example"),
            ("OTEL_LOG_RAW_API_BODIES", "file:"),
            ("OTEL_LOG_RAW_API_BODIES", "file: /tmp/bodies"),
            ("OTEL_LOG_RAW_API_BODIES", "2"),
            ("OTEL_LOG_RAW_API_BODIES", "file:/tmp/bo\ndies"),
        ] {
            let request = json!({"preset":"custom",
                "custom_settings":{key:"set"},
                "custom_values":{key:value}});
            let error = plan(
                &json!({}),
                &json!({"version":"2.1.283"}),
                &request,
                Path::new("/synthetic/settings.json"),
            )
            .unwrap_err();
            assert_eq!(error.code, "invalid_request", "{key}: {value}");
        }
    }
    #[test]
    fn custom_otel_actions_produce_exact_changes_and_never_leak_the_credential() {
        let secret = "Bearer synthetic-secret-do-not-print";
        let doc = json!({"env":{
            MASKED_KEY:secret,
            "OTEL_METRICS_INCLUDE_ACCOUNT_UUID":"false",
            "OTEL_LOGS_EXPORTER":"console",
            "OTEL_LOG_RAW_API_BODIES":"1",
            "UNRELATED":"keep",
        }});
        let request = json!({"preset":"custom","custom_settings":{
            "OTEL_METRICS_INCLUDE_ACCOUNT_UUID":"enable",
            "OTEL_LOG_RAW_API_BODIES":"set",
            "OTEL_LOGS_EXPORTER":"disable",
            "OTEL_TRACES_EXPORTER":"set",
            "OTEL_LOG_TOOL_DETAILS":"enable",
            "CLAUDE_CODE_ENABLE_TELEMETRY":"enable",
            "CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH":"set",
        },"custom_values":{
            "OTEL_LOG_RAW_API_BODIES":"file:/synthetic/otel-bodies",
            "CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH":"122880",
            "OTEL_TRACES_EXPORTER":"otlp",
        }});
        let (changes, policy) = plan(
            &doc,
            &json!({"version":"2.1.283"}),
            &request,
            Path::new("/synthetic/settings.json"),
        )
        .unwrap();
        // Nothing anywhere in the result may carry the credential value, and no
        // action in the result may claim to restore it.
        assert!(!changes.to_string().contains(secret));
        assert!(!policy.to_string().contains(secret));
        let by_key = |key: &str| {
            changes
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["key"] == key)
                .unwrap()
                .clone()
        };
        // DefaultOn enable removes the key and says why in the label.
        let uuid = by_key("OTEL_METRICS_INCLUDE_ACCOUNT_UUID");
        assert_eq!(uuid["before"], "false");
        assert!(uuid["after"].is_null());
        assert_eq!(uuid["restores_default"], true);
        assert!(uuid["label"].as_str().unwrap().contains("恢复默认"));
        // RawApiBodies set writes the documented local-dump spelling.
        let raw = by_key("OTEL_LOG_RAW_API_BODIES");
        assert_eq!(raw["before"], "1");
        assert_eq!(raw["after"], "file:/synthetic/otel-bodies");
        // A free value with no "off" spelling is disabled by removal.
        let logs = by_key("OTEL_LOGS_EXPORTER");
        assert_eq!(logs["before"], "console");
        assert!(logs["after"].is_null());
        // An exporter is set by naming its documented token.
        let traces = by_key("OTEL_TRACES_EXPORTER");
        assert!(traces["before"].is_null());
        assert_eq!(traces["after"], "otlp");
        // An absent boolean simply becomes "1".
        assert_eq!(by_key("OTEL_LOG_TOOL_DETAILS")["after"], "1");
        assert_eq!(by_key("CLAUDE_CODE_ENABLE_TELEMETRY")["after"], "1");
        assert_eq!(
            by_key("CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH")["after"],
            "122880"
        );
        // The resulting assessment reflects the applied state.
        assert_eq!(policy["preset"], "custom");
        assert_eq!(policy["custom_settings"], request["custom_settings"]);
        let applied = policy["rules"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["key"] == "OTEL_LOG_RAW_API_BODIES")
            .unwrap();
        assert_eq!(applied["local_dump"], "/synthetic/otel-bodies");
        // Presets never touch the OTel family, even when it is currently on.
        for preset in ["preserve", "reduce"] {
            let (preset_changes, _) = plan(
                &doc,
                &json!({"version":"2.1.283"}),
                &json!({"preset":preset,"keep_remote_control":false}),
                Path::new("/synthetic/settings.json"),
            )
            .unwrap();
            for change in preset_changes.as_array().unwrap() {
                let key = change["key"].as_str().unwrap();
                let rule = catalog().iter().find(|r| r.key == key).unwrap();
                assert_eq!(rule.category, "privacy_switch", "{preset}: {key}");
            }
        }
        // The credential field is removable (a null write) but not writable, and
        // its removal never discloses the value it replaced.
        let (masked, _) = plan(
            &doc,
            &json!({"version":"2.1.283"}),
            &json!({"preset":"custom","keep_remote_control":true,
                "custom_settings":{MASKED_KEY:"remove"}}),
            Path::new("/synthetic/settings.json"),
        )
        .unwrap();
        assert!(!masked.to_string().contains(secret));
        assert_eq!(masked[0]["key"], MASKED_KEY);
        assert!(masked[0]["before"].is_null());
        assert!(masked[0]["after"].is_null());
        assert_eq!(masked[0]["masked"], true);
        for choices in [json!({MASKED_KEY:"set"}), json!({MASKED_KEY:"enable"})] {
            assert_eq!(
                plan(
                    &doc,
                    &json!({"version":"2.1.283"}),
                    &json!({"preset":"custom","custom_settings":choices}),
                    Path::new("/synthetic/settings.json")
                )
                .unwrap_err()
                .code,
                "masked_setting_not_writable",
                "{choices}"
            );
        }
        assert_eq!(
            plan(
                &doc,
                &json!({"version":"2.1.283"}),
                &json!({"preset":"custom","custom_settings":{MASKED_KEY:"set"},"custom_values":{MASKED_KEY:"x"}}),
                Path::new("/synthetic/settings.json")
            )
            .unwrap_err()
            .code,
            "masked_setting_not_writable"
        );
    }
    #[test]
    fn custom_otel_actions_are_noops_when_already_in_the_requested_state() {
        let doc = json!({"env":{
            "OTEL_METRICS_INCLUDE_ACCOUNT_UUID":"1",
            "OTEL_LOG_RAW_API_BODIES":"0",
            "OTEL_LOGS_EXPORTER":"none",
            "CLAUDE_CODE_ENABLE_TELEMETRY":"1",
        }});
        let request = json!({"preset":"custom","custom_settings":{
            "OTEL_METRICS_INCLUDE_ACCOUNT_UUID":"enable",
            "OTEL_LOG_RAW_API_BODIES":"disable",
            "OTEL_LOGS_EXPORTER":"set",
            "CLAUDE_CODE_ENABLE_TELEMETRY":"enable",
        },"custom_values":{"OTEL_LOGS_EXPORTER":"none"}});
        let (changes, _) = plan(
            &doc,
            &json!({"version":"2.1.283"}),
            &request,
            Path::new("/synthetic/settings.json"),
        )
        .unwrap();
        // Re-asserting the current state is not a change; only a real edit is.
        assert!(changes.as_array().unwrap().is_empty(), "{changes}");
    }
    #[test]
    fn raw_api_bodies_inline_and_file_modes_are_distinct_facts() {
        for (value, local) in [
            (json!("0"), None),
            (json!("1"), None),
            (json!("file:/synthetic/bodies"), Some("/synthetic/bodies")),
        ] {
            let doc = json!({"env":{"OTEL_LOG_RAW_API_BODIES":value}});
            let a = assessment(&doc, &json!({"version":"2.1.283"}), "not_required");
            let rule = a["rules"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["key"] == "OTEL_LOG_RAW_API_BODIES")
                .unwrap();
            match local {
                Some(dir) => {
                    assert_eq!(rule["local_dump"], dir);
                    assert_eq!(rule["disabled"], true);
                    let note = rule["local_dump_note"].as_str().unwrap();
                    assert!(note.contains("本地目录"));
                    assert!(note.contains("已上传") || note.contains("落盘"));
                }
                None => {
                    assert!(rule.get("local_dump").is_none());
                    assert_eq!(rule["disabled"], value == "1");
                    assert!(rule.get("local_dump_note").is_none());
                }
            }
        }
    }
    #[test]
    fn version_detection_is_static_and_specific() {
        let t = tempfile::tempdir().unwrap();
        let dir = t.path().join("claude/versions");
        std::fs::create_dir_all(&dir).unwrap();
        let native = dir.join("2.1.283");
        std::fs::write(&native, b"not executed").unwrap();
        assert_eq!(product(native.to_str())["version"], "2.1.283");
        assert!(product(Some("/synthetic/2.1.283"))["version"].is_null());
        let npm = t.path().canonicalize().unwrap().join("npm");
        std::fs::create_dir(&npm).unwrap();
        std::fs::write(
            npm.join("package.json"),
            r#"{"name":"@anthropic-ai/claude-code","version":"2.1.282"}"#,
        )
        .unwrap();
        assert_eq!(product(npm.join("cli.js").to_str())["version"], "2.1.282");
    }
}

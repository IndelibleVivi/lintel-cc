//! Versioned, source-scoped privacy rules. Never runs Claude or probes an account.
use crate::{err, Result};
use serde_json::{json, Value};
use std::path::Path;

pub(crate) const RULE: &str = "claude-privacy-v2-2026-10-03";
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
pub(crate) fn fields() -> impl Iterator<Item = (&'static str, &'static str)> {
    FLAGS.into_iter().chain([
        ("DO_NOT_TRACK", "跨工具 telemetry 开关"),
        ("DISABLE_GROWTHBOOK", "feature-flag 获取"),
        (
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
            "非必要流量总开关",
        ),
    ])
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

/// None means an undocumented boolean spelling; do not call it configured.
pub(crate) fn disabled(key: &str, value: &Value) -> Option<bool> {
    let Some(s) = value.as_str() else {
        return Some(false);
    };
    if [
        "DISABLE_TELEMETRY",
        "DISABLE_ERROR_REPORTING",
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
    ]
    .contains(&key)
    {
        return Some(!s.is_empty());
    }
    match s.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "" | "0" | "false" | "no" | "off" => Some(false),
        _ => None,
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
        blockers.push(json!({"key":key,"value":doc["env"][key],"status":status,"reason":reason,"source":"user_settings"}));
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
    let rules: Vec<_> = fields().map(|(key,label)| json!({
        "key":key,"label":label,"value":doc["env"][key],"disabled":disabled(key,&doc["env"][key]),
        "semantics":if ["DISABLE_TELEMETRY","DISABLE_ERROR_REPORTING","CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC"].contains(&key){"nonempty"}else{"boolean"},
        "scope":"registered_root_user_settings","effect_timing":"next_launch",
    })).collect();
    json!({"rule_version":RULE,"product":product,"rules":rules,"remote_control":{
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
    if !["preserve", "reduce"].contains(&preset) {
        return Err(err("invalid_preset", "未知保护方案"));
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
    for (key, label) in fields() {
        let release = releases.contains(&key);
        let set = FLAGS.iter().any(|(k, _)| *k == key)
            && (preset == "reduce"
                || [
                    "DISABLE_ERROR_REPORTING",
                    "CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY",
                ]
                .contains(&key))
            && !(keep && key == "DISABLE_TELEMETRY" && telemetry_needed(product, trusted));
        let value = if release {
            Value::Null
        } else if set && disabled(key, &doc["env"][key]) != Some(true) {
            json!("1")
        } else {
            continue;
        };
        changes.push(
            json!({"key":key,"label":label,"before":doc["env"][key],"after":value,"path":path}),
        );
        if release {
            after["env"].as_object_mut().unwrap().remove(key);
        } else {
            after["env"][key] = value;
        }
    }
    let mut policy = assessment(&after, product, trusted);
    policy["keep_remote_control"] = json!(keep);
    policy["release_settings"] = json!(releases);
    policy["current_remote_control"] = current["remote_control"].clone();
    Ok((json!(changes), policy))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn values_follow_each_rule() {
        for (key, _) in fields() {
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

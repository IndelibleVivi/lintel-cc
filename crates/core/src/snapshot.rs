//! Sanitized semantic environment snapshots and their local comparison.
//!
//! Every observation is injected by the caller: this module reads no file, runs
//! no client and never touches a credential or a raw identity value. A snapshot
//! carries switch values, finite declaration flags, a caller-salted identity
//! digest and residue counts; a comparison publishes verdicts only, never a
//! digest.
use crate::{id, now, policy, valstr};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

pub(crate) const SCHEMA: &str = "lintel.env-snapshot/1";

// Forward-compatible masking: the value of a forwarded OTLP header is replaced
// by null and marked, even while the installed rule list does not classify it.
const MASKED_KEYS: [&str; 1] = ["OTEL_EXPORTER_OTLP_HEADERS"];
// (startup_view field, snapshot field). Only these finite flags are projected.
const DECLARATION_FLAGS: [(&str, &str); 4] = [
    ("hooks_declared", "hooks"),
    ("mcp_declared", "mcp"),
    ("plugins_declared", "plugins"),
    ("api_key_helper_declared", "api_key_helper"),
];
// Keys owned by the residue section shape itself.
const RESIDUE_RESERVED: [&str; 2] = ["totals", "scan_complete"];

fn masked(key: &str, value: &Value) -> bool {
    MASKED_KEYS.contains(&key)
        || (key == "OTEL_EXPORTER_OTLP_ENDPOINT"
            && value
                .as_str()
                .is_some_and(|text| text.contains(['@', '?', '#'])))
}

/// One switch: the current string value (or null), its finite rule status and,
/// for a masked key, an explicit marker instead of the value.
fn settings_entry(key: &str, env: Option<&Value>) -> Value {
    let raw = env.and_then(|env| env.get(key));
    let absent = Value::Null;
    let value = raw.unwrap_or(&absent);
    let mut entry = json!({"value": valstr(value), "status": policy::setting_status(key, value)});
    if masked(key, value) && raw.is_some() {
        entry["value"] = Value::Null;
        entry["masked"] = json!(true);
    }
    if raw.is_some() && !value.is_string() {
        entry["uninterpretable"] = json!(true);
    }
    entry
}

fn capture_settings(settings: &Value) -> Value {
    let env = settings.get("env");
    let known: Vec<&str> = policy::fields().map(|(key, _)| key).collect();
    let extra: Vec<&str> = MASKED_KEYS
        .into_iter()
        .filter(|key| env.and_then(|env| env.get(*key)).is_some() && !known.contains(key))
        .collect();
    let mut out = Map::new();
    for key in known.into_iter().chain(extra) {
        out.insert(key.to_string(), settings_entry(key, env));
    }
    Value::Object(out)
}

/// Project only the finite declaration flags. A source that was not read as a
/// JSON object (missing, directory, access-limited, unparsed) cannot attest any
/// flag, so its flags stay null and the source is recorded as incomplete.
fn declarations(startup_view: Option<&Value>) -> (Vec<Value>, Vec<Value>) {
    let mut projected = vec![];
    let mut incomplete = vec![];
    let sources = startup_view
        .and_then(|view| view["sources"].as_array())
        .into_iter()
        .flatten();
    for source in sources {
        let state = source["state"].as_str().unwrap_or("unknown");
        let content_state = source["content_state"].as_str();
        let mut item = json!({
            "scope": source["scope"].as_str(),
            "path": source["path"].as_str(),
            "state": state,
        });
        if state == "not_found" {
            for (_, flag) in DECLARATION_FLAGS {
                item[flag] = json!(false);
            }
        } else if content_state == Some("observed") {
            for (field, flag) in DECLARATION_FLAGS {
                item[flag] = json!(source[field].as_bool().unwrap_or(false));
            }
        } else {
            for (_, flag) in DECLARATION_FLAGS {
                item[flag] = Value::Null;
            }
            incomplete.push(json!({
                "path": source["path"].as_str(),
                "state": state,
                "content_state": content_state,
            }));
        }
        projected.push(item);
    }
    (projected, incomplete)
}

/// The caller's salted digest is the only identity fact a snapshot may carry.
fn identity_digest<'a>(identity_binding: Option<&'a Value>) -> Option<&'a str> {
    identity_binding
        .and_then(|binding| binding.get("digests"))
        .and_then(|digests| digests.get("userID"))
        .and_then(Value::as_str)
        .filter(|digest| !digest.is_empty())
}

fn identity(identity_binding: Option<&Value>) -> Value {
    let digest = identity_digest(identity_binding);
    json!({"comparison_scope": identity_binding.and_then(|binding| binding.get("comparison_scope")), "userID": {
        "digest": digest,
        "state": if digest.is_some() { "observed" } else { "unknown" },
    }})
}

/// Residue per category, plus the reported totals and scan completeness. A
/// category may never take over the section's own keys.
fn residue_summary(residue: &Value) -> Value {
    let mut categories: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let locations = residue["locations"].as_array().into_iter().flatten();
    for location in locations {
        let Some(category) = location["category"].as_str() else {
            continue;
        };
        if RESIDUE_RESERVED.contains(&category) {
            continue;
        }
        let entry = categories.entry(category.to_string()).or_default();
        entry.0 += location["file_count"].as_u64().unwrap_or(0);
        entry.1 += location["total_bytes"].as_u64().unwrap_or(0);
    }
    let counted = categories.values().fold((0u64, 0u64), |sum, entry| {
        (sum.0 + entry.0, sum.1 + entry.1)
    });
    let totals = residue.get("totals");
    let mut out = Map::new();
    for (category, (count, bytes)) in &categories {
        out.insert(category.clone(), json!({"count": count, "bytes": bytes}));
    }
    out.insert(
        "totals".into(),
        json!({
            "file_count": totals.and_then(|totals| totals["file_count"].as_u64()).unwrap_or(counted.0),
            "total_bytes": totals.and_then(|totals| totals["total_bytes"].as_u64()).unwrap_or(counted.1),
        }),
    );
    out.insert(
        "scan_complete".into(),
        json!(residue["scan_complete"] == true),
    );
    Value::Object(out)
}

pub(crate) fn capture(
    environment_id: &str,
    product_version: Option<&str>,
    settings: &Value,
    startup_view: Option<&Value>,
    residue: Option<&Value>,
    identity_binding: Option<&Value>,
) -> Value {
    let (projected, incomplete) = declarations(startup_view);
    let residue_section = match residue {
        Some(residue) => residue_summary(residue),
        None => json!({}),
    };
    let startup_complete = startup_view.is_some_and(|view| {
        view["candidate_scan_complete"] == true && view["content_limited"] != true
    });
    json!({
        "schema": SCHEMA,
        "snapshot_id": id(),
        "environment_id": environment_id,
        "created_at": now(),
        "rule_version": policy::RULE,
        "product_version": product_version,
        "settings": capture_settings(settings),
        "declarations": projected,
        "identity": identity(identity_binding),
        "residue": residue_section,
        "coverage": {
            "settings": true,
            "startup": startup_view.is_some(),
            "residue": residue.is_some(),
            "identity": identity_binding.is_some(),
            "startup_complete": startup_complete,
            "declarations_incomplete": incomplete,
        },
    })
}

fn coverage_flag(snapshot: &Value, area: &str) -> bool {
    snapshot["coverage"][area].as_bool().unwrap_or(false)
}

fn digest_of(snapshot: &Value) -> Option<&str> {
    snapshot["identity"]["userID"]["digest"]
        .as_str()
        .filter(|digest| !digest.is_empty())
}

fn identity_verdict<'a>(a: &Value, b: &Value, different: &'a str) -> &'a str {
    // Missing scopes retain compatibility for the original injected snapshot
    // format. A bound snapshot cannot be compared against another salt domain
    // (or an unbound legacy observation) as if a changed salt were a changed ID.
    if a["identity"]["comparison_scope"] != b["identity"]["comparison_scope"] {
        return "unknown";
    }
    let compared = crate::residue::identity_compare(
        &json!({"digests":{"userID":digest_of(a)}}),
        &json!({"digests":{"userID":digest_of(b)}}),
    );
    match compared["userID"].as_str() {
        Some("same") => "same",
        Some("changed") => different,
        _ => "unknown",
    }
}

fn complete_startup(snapshot: &Value) -> bool {
    coverage_flag(snapshot, "startup")
        && snapshot["coverage"]["startup_complete"] == true
        && snapshot["coverage"]["declarations_incomplete"]
            .as_array()
            .is_some_and(Vec::is_empty)
}

fn complete_residue(snapshot: &Value) -> bool {
    coverage_flag(snapshot, "residue") && snapshot["residue"]["scan_complete"] == true
}

fn declaration_key(item: &Value) -> (String, String) {
    (
        item["scope"].as_str().unwrap_or("").to_string(),
        item["path"].as_str().unwrap_or("").to_string(),
    )
}

/// The union of both settings maps, in rule-list order first, then any extra
/// key (a forwarded header) sorted.
fn ordered_settings_keys(old: &Value, new: &Value) -> Vec<(String, &'static str)> {
    let sections = [old["settings"].as_object(), new["settings"].as_object()];
    let mut ordered: Vec<(String, &'static str)> = vec![];
    for (key, label) in policy::fields() {
        if sections.iter().flatten().any(|map| map.contains_key(key)) {
            ordered.push((key.to_string(), label));
        }
    }
    let mut rest: Vec<String> = vec![];
    for map in sections.iter().flatten() {
        for key in map.keys() {
            if !rest.contains(key) && !ordered.iter().any(|(known, _)| known == key) {
                rest.push(key.clone());
            }
        }
    }
    rest.sort();
    ordered.extend(rest.into_iter().map(|key| (key, "")));
    ordered
}

fn residue_categories(old: &Value, new: &Value) -> Vec<String> {
    let mut categories: Vec<String> = vec![];
    for snapshot in [old, new] {
        if let Some(map) = snapshot["residue"].as_object() {
            for key in map.keys() {
                if !RESIDUE_RESERVED.contains(&key.as_str()) && !categories.contains(key) {
                    categories.push(key.clone());
                }
            }
        }
    }
    categories.sort();
    categories
}

/// Same-environment before/after comparison. Rule versions are compared first:
/// a version change is not file drift, so settings differences stay reported
/// item by item and are marked as reinterpreted under the new rules.
pub(crate) fn compare(old: &Value, new: &Value) -> Value {
    let rule_changed = old["rule_version"] != new["rule_version"];
    let rule_change = rule_changed.then(|| {
        json!({
            "state": "rules_updated",
            "old": old["rule_version"],
            "new": new["rule_version"],
        })
    });
    let settings_covered = coverage_flag(old, "settings") && coverage_flag(new, "settings");
    let startup_covered = coverage_flag(old, "startup") && coverage_flag(new, "startup");
    let residue_covered = complete_residue(old) && complete_residue(new);
    // Undecidable coverage, not a change: it only decides the status when no
    // substantive change was found at all.
    let mut undecided = !(settings_covered
        && startup_covered
        && residue_covered
        && coverage_flag(old, "identity")
        && coverage_flag(new, "identity"));
    undecided |= !complete_startup(old) || !complete_startup(new);
    let mut changes = vec![];

    let mut settings_changes = 0usize;
    let mut uncertain_settings = 0usize;
    for (key, label) in ordered_settings_keys(old, new) {
        let left = old["settings"].get(&key).filter(|entry| entry.is_object());
        let right = new["settings"].get(&key).filter(|entry| entry.is_object());
        let masked_pair = left.is_some_and(|entry| entry["masked"] == true)
            || right.is_some_and(|entry| entry["masked"] == true);
        let mut item = json!({"area": "settings", "key": key, "label": label});
        if rule_changed {
            item["reinterpreted_under_new_rules"] = json!(true);
        }
        if !settings_covered {
            item["state"] = json!("incomplete");
            changes.push(item);
            continue;
        }
        // A masked key publishes no value, but a masked entry still attests
        // presence while an unmasked null attests absence; presence versus
        // absence stays a real fact. Two present sides with a hidden value
        // decide nothing.
        let state = match (left, right) {
            (None, None) => "unchanged",
            (None, Some(_)) => "added",
            (Some(_), None) => "removed",
            (Some(left), Some(right))
                if left["uninterpretable"] == true || right["uninterpretable"] == true =>
            {
                "uncertain"
            }
            (Some(left), Some(right)) if masked_pair => {
                match (left["masked"] == true, right["masked"] == true) {
                    (true, false) if right["value"].is_null() => "removed",
                    (false, true) if left["value"].is_null() => "added",
                    _ => "uncertain",
                }
            }
            (Some(left), Some(right)) if left["value"] == right["value"] => "unchanged",
            (Some(_), Some(_)) => "changed",
        };
        let hidden = masked_pair;
        let (previous, current) = if hidden {
            (Value::Null, Value::Null)
        } else {
            (
                left.map(|entry| entry["value"].clone())
                    .unwrap_or(Value::Null),
                right
                    .map(|entry| entry["value"].clone())
                    .unwrap_or(Value::Null),
            )
        };
        if matches!(state, "added" | "removed" | "changed") {
            settings_changes += 1;
        }
        if state == "uncertain" {
            uncertain_settings += 1;
            undecided = true;
        }
        if masked_pair {
            item["masked"] = json!(true);
        }
        item["state"] = json!(state);
        item["old"] = previous;
        item["new"] = current;
        changes.push(item);
    }

    let mut declaration_changes = 0usize;
    if !startup_covered {
        changes.push(json!({"area": "declarations", "state": "incomplete"}));
    } else {
        let none: Vec<Value> = vec![];
        let left_items = old["declarations"].as_array().unwrap_or(&none);
        let right_items = new["declarations"].as_array().unwrap_or(&none);
        let mut pairs: Vec<(Option<&Value>, Option<&Value>)> = vec![];
        for item in left_items {
            let key = declaration_key(item);
            let mate = right_items
                .iter()
                .find(|candidate| declaration_key(candidate) == key);
            pairs.push((Some(item), mate));
        }
        for item in right_items {
            let key = declaration_key(item);
            if !left_items
                .iter()
                .any(|candidate| declaration_key(candidate) == key)
            {
                pairs.push((None, Some(item)));
            }
        }
        for (left, right) in pairs {
            let key = left.or(right).map(declaration_key).unwrap_or_default();
            let mut item = json!({"area": "declarations", "scope": key.0, "path": key.1});
            match (left, right) {
                (Some(_), None) if !complete_startup(new) => {
                    item["state"] = json!("incomplete");
                    undecided = true;
                }
                (None, Some(_)) if !complete_startup(old) => {
                    item["state"] = json!("incomplete");
                    undecided = true;
                }
                (Some(left), None) => {
                    item["state"] = json!("disappeared");
                    item["old"] = left.clone();
                    item["new"] = Value::Null;
                    declaration_changes += 1;
                }
                (None, Some(right)) => {
                    item["state"] = json!("appeared");
                    item["old"] = Value::Null;
                    item["new"] = right.clone();
                    declaration_changes += 1;
                }
                (Some(left), Some(right)) => {
                    if DECLARATION_FLAGS
                        .iter()
                        .any(|(_, name)| left[name].is_null() || right[name].is_null())
                    {
                        item["state"] = json!("incomplete");
                        item["old"] = left.clone();
                        item["new"] = right.clone();
                        if left["state"] != right["state"] {
                            item["observation_change"] =
                                json!({"from":left["state"],"to":right["state"]});
                        }
                        undecided = true;
                        changes.push(item);
                        continue;
                    }
                    // A source state change is reported through the same finite
                    // flag list, so a source can never silently change meaning.
                    let mut flags = vec![];
                    let names = DECLARATION_FLAGS
                        .iter()
                        .map(|(_, name)| *name)
                        .chain(std::iter::once("state"));
                    for name in names {
                        if left[name] != right[name] {
                            flags.push(json!({
                                "flag": name,
                                "from": left[name],
                                "to": right[name],
                            }));
                        }
                    }
                    let state = if flags.is_empty() {
                        "unchanged"
                    } else {
                        declaration_changes += 1;
                        "flags_changed"
                    };
                    item["state"] = json!(state);
                    item["flags"] = json!(flags);
                    item["old"] = left.clone();
                    item["new"] = right.clone();
                }
                (None, None) => continue,
            }
            changes.push(item);
        }
    }

    let identity_state = identity_verdict(old, new, "changed");
    if identity_state == "unknown" {
        undecided = true;
    }
    let identity_changes = usize::from(identity_state == "changed");
    changes.push(json!({"area": "identity", "subject": "userID", "state": identity_state}));

    let mut residue_changes = 0usize;
    if !residue_covered {
        changes.push(json!({"area": "residue", "state": "incomplete"}));
    } else {
        for category in residue_categories(old, new) {
            let left = old["residue"]
                .get(&category)
                .filter(|entry| entry.is_object());
            let right = new["residue"]
                .get(&category)
                .filter(|entry| entry.is_object());
            let count =
                |entry: Option<&Value>| entry.and_then(|e| e["count"].as_i64()).unwrap_or(0);
            let bytes =
                |entry: Option<&Value>| entry.and_then(|e| e["bytes"].as_i64()).unwrap_or(0);
            let (old_count, new_count) = (count(left), count(right));
            let (old_bytes, new_bytes) = (bytes(left), bytes(right));
            let state = match (left, right) {
                (None, Some(_)) => "added",
                (Some(_), None) => "removed",
                _ if old_count == new_count && old_bytes == new_bytes => "unchanged",
                _ => "changed",
            };
            if state != "unchanged" {
                residue_changes += 1;
            }
            changes.push(json!({
                "area": "residue",
                "category": category,
                "state": state,
                "old_count": old_count,
                "new_count": new_count,
                "count_delta": new_count - old_count,
                "old_bytes": old_bytes,
                "new_bytes": new_bytes,
                "bytes_delta": new_bytes - old_bytes,
            }));
        }
    }

    // Additive source-aware semantics. Legacy snapshots cannot attest sources.
    if old.get("configuration_sources").is_some() || new.get("configuration_sources").is_some() {
        let covered = coverage_flag(old, "configuration_sources")
            && coverage_flag(new, "configuration_sources");
        if !covered {
            undecided = true;
            changes.push(
                json!({"area":"settings","subject":"configuration_sources","state":"incomplete"}),
            );
        } else {
            let mut left = BTreeMap::new();
            let mut right = BTreeMap::new();
            for (snapshot, rows) in [(old, &mut left), (new, &mut right)] {
                for row in snapshot["configuration_sources"]["settings"]
                    .as_array()
                    .into_iter()
                    .flatten()
                {
                    if row["scope"] == "user" {
                        continue;
                    }
                    rows.insert(
                        (
                            row["source"].as_str().unwrap_or(""),
                            row["key"].as_str().unwrap_or(""),
                        ),
                        row,
                    );
                }
            }
            let keys: std::collections::BTreeSet<_> =
                left.keys().chain(right.keys()).copied().collect();
            for key in keys {
                let a = left.get(&key);
                let b = right.get(&key);
                let hidden = a.is_some_and(|r| r["masked"] == true)
                    || b.is_some_and(|r| r["masked"] == true);
                let state = match (a, b) {
                    (None, Some(_)) => "added",
                    (Some(_), None) => "removed",
                    (Some(_), Some(_)) if hidden => "uncertain",
                    (Some(a), Some(b)) if a["value"] == b["value"] => "unchanged",
                    _ => "changed",
                };
                if state == "uncertain" {
                    undecided = true;
                } else if state != "unchanged" {
                    settings_changes += 1;
                }
                changes.push(json!({"area":"settings","key":key.1,"path":key.0,"source":key.0,"label":key.1,"state":state,"masked":hidden,"old":if hidden{Value::Null}else{a.map(|r|r["value"].clone()).unwrap_or(Value::Null)},"new":if hidden{Value::Null}else{b.map(|r|r["value"].clone()).unwrap_or(Value::Null)},"reinterpreted_under_new_rules":rule_changed}));
            }
        }
    }
    let substantive = settings_changes + declaration_changes + residue_changes + identity_changes;
    let status = if substantive > 0 {
        "changed"
    } else if rule_changed {
        "rules_updated"
    } else if undecided {
        "incomplete"
    } else {
        "unchanged"
    };
    let summary = match status {
        "changed" => format!(
            "发现 {substantive} 项实质变化（设置 {settings_changes}、来源声明 {declaration_changes}、身份 {identity_changes}、残留 {residue_changes}）。"
        ),
        "rules_updated" => format!(
            "规则版本由 {} 变为 {}：设置已按新规则逐项重新解释，未发现其他变化。",
            old["rule_version"].as_str().unwrap_or("未知"),
            new["rule_version"].as_str().unwrap_or("未知")
        ),
        "incomplete" => {
            "两侧快照存在未观测的区域，身份摘要或部分覆盖面不足以判定是否一致。".to_string()
        }
        _ if uncertain_settings == 0 => {
            "两侧快照的设置、来源声明、身份摘要与残留统计均未发现变化。".to_string()
        }
        _ => format!(
            "两侧快照的设置、来源声明、身份摘要与残留统计均未发现变化；{uncertain_settings} 项遮蔽值只能按可见值比较。"
        ),
    };
    for row in &mut changes {
        row["id"] = json!(crate::comparison_control::change_id(row));
    }
    json!({"status": status, "rule_change": rule_change, "changes": changes, "summary": summary})
}

/// Side-by-side comparison of two registered environments. It publishes local
/// facts only: the three-word identity verdict, per-key setting values, the
/// finite declaration alignment and both residue sections. No digest is ever
/// published.
pub(crate) fn compare_pair(a: &Value, b: &Value) -> Value {
    let identity = identity_verdict(a, b, "different");
    let mut settings = Map::new();
    let mut masked_settings = false;
    for (key, _) in ordered_settings_keys(a, b) {
        let left = a["settings"].get(&key).filter(|entry| entry.is_object());
        let right = b["settings"].get(&key).filter(|entry| entry.is_object());
        let masked_pair = left.is_some_and(|entry| entry["masked"] == true)
            || right.is_some_and(|entry| entry["masked"] == true);
        masked_settings |= masked_pair;
        // A masked entry never republishes its value, whatever the caller left
        // in the snapshot; the visible comparison then sees two nulls.
        let value = |entry: Option<&Value>| {
            entry
                .filter(|entry| entry["masked"] != true)
                .map(|entry| entry["value"].clone())
                .unwrap_or(Value::Null)
        };
        let same = if masked_pair
            || !coverage_flag(a, "settings")
            || !coverage_flag(b, "settings")
            || left.is_some_and(|entry| entry["uninterpretable"] == true)
            || right.is_some_and(|entry| entry["uninterpretable"] == true)
        {
            Value::Null
        } else {
            json!(left.is_some() && right.is_some() && value(left) == value(right))
        };
        let mut entry = json!({"a": value(left), "b": value(right), "same": same});
        if masked_pair {
            entry["masked"] = json!(true);
        }
        settings.insert(key, entry);
    }
    let none: Vec<Value> = vec![];
    let left_items = a["declarations"].as_array().unwrap_or(&none);
    let right_items = b["declarations"].as_array().unwrap_or(&none);
    let mut only_a = vec![];
    let mut only_b = vec![];
    let mut both = vec![];
    for item in left_items {
        let key = declaration_key(item);
        let aligned = json!({"scope": key.0, "path": key.1});
        if right_items
            .iter()
            .any(|candidate| declaration_key(candidate) == key)
        {
            both.push(aligned);
        } else {
            only_a.push(aligned);
        }
    }
    for item in right_items {
        let key = declaration_key(item);
        if !left_items
            .iter()
            .any(|candidate| declaration_key(candidate) == key)
        {
            only_b.push(json!({"scope": key.0, "path": key.1}));
        }
    }
    let mut note = String::from(
        "并排对照是本地事实比较：只比较两个已登记环境各自观测到的设置、来源声明、残留与身份摘要，不推断账号、平台或组织关联，也不代表同一登录、同一人或同一设备；same/different 仅表示两次观测的调用方加盐摘要是否一致。",
    );
    if !(coverage_flag(a, "startup") && coverage_flag(b, "startup")) {
        note.push_str("一侧的启动来源未观测，声明对齐只覆盖已观测部分。");
    }
    if masked_settings {
        note.push_str("存在遮蔽值：设置项只能按可见值比较，same 不表示被遮蔽内容真实一致。");
    }
    json!({
        "identity": {"userID": identity},
        "settings": settings,
        "declarations": {"only_a": only_a, "only_b": only_b, "both": both},
        "residue": {"a": a["residue"], "b": b["residue"]},
        "configuration_sources":{"a":a["configuration_sources"],"b":b["configuration_sources"]},
        "coverage": {"a":a["coverage"],"b":b["coverage"]},
        "note": note,
    })
}

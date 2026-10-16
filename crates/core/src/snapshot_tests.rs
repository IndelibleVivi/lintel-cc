//! Snapshot capture/comparison regressions. Every input is a synthetic
//! serde_json value; the module reads no file and never sees a raw identity.
use crate::policy;
use crate::snapshot::*;
use serde_json::{json, Value};

const ENV: &str = "11111111-2222-3333-4444-555555555555";
const OTHER_ENV: &str = "22222222-3333-4444-5555-666666666666";
const DIGEST: &str = "aaa1111111111111111111111111111111111111111111111111111111bbb";
const OTHER_DIGEST: &str = "fff0e1d2c3b4a5968778695a4b3c2d1e0f1a2b3c4d5e6f708192a3b4c5d6e7f8";
const SECRET_HEADER: &str = "syn-secret-header-777";

fn observed_source(scope: &str, path: &str, hooks: bool, mcp: bool, plugins: bool) -> Value {
    json!({"path":path,"scope":scope,"state":"observed","content_state":"observed",
        "hooks_declared":hooks,"mcp_declared":mcp,"plugins_declared":plugins,
        "api_key_helper_declared":false})
}

fn source_set(hooks: bool, mcp: bool, plugins: bool, helper: bool) -> Value {
    json!([
        json!({"path":"/synthetic/user/settings.json","scope":"user","state":"observed",
            "content_state":"observed","hooks_declared":hooks,"mcp_declared":mcp,
            "plugins_declared":plugins,"api_key_helper_declared":helper}),
        json!({"path":"/synthetic/user/CLAUDE.md","scope":"user_instructions","state":"not_found"}),
    ])
}

fn startup(sources: Value, candidates_complete: bool, content_limited: bool) -> Value {
    json!({"schema":"lintel.startup/1","sources":sources,
        "candidate_scan_complete":candidates_complete,"content_limited":content_limited})
}

fn full_startup() -> Value {
    let mut sources = source_set(true, true, false, false);
    sources.as_array_mut().unwrap().push(observed_source(
        "project",
        "/synthetic/project/settings.json",
        false,
        false,
        true,
    ));
    startup(sources, true, false)
}

fn location(category: &str, class: &str, files: u64, bytes: u64) -> Value {
    json!({"category":category,"class":class,"state":"observed",
        "file_count":files,"total_bytes":bytes})
}

fn residue(locations: Value, files: u64, bytes: u64, complete: bool) -> Value {
    json!({"schema":"lintel.residue/1","locations":locations,
        "totals":{"file_count":files,"total_bytes":bytes},"scan_complete":complete})
}

fn full_residue() -> Value {
    residue(
        json!([
            location("instructions", "file", 2, 120),
            location("sessions", "file", 3, 4096),
            location("sessions", "directory", 1, 0),
        ]),
        6,
        4216,
        true,
    )
}

fn switches() -> Value {
    json!({"env":{
        "DISABLE_TELEMETRY":"1","DISABLE_ERROR_REPORTING":"0",
        "DISABLE_FEEDBACK_COMMAND":"false","CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY":"原字节值",
        "DO_NOT_TRACK":"1","DISABLE_GROWTHBOOK":"1",
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC":"1","UNRELATED":"ignored"
    },"permissions":{"allow":["Read"]}})
}

fn settings_with_header() -> Value {
    let mut doc = switches();
    doc["env"]["OTEL_EXPORTER_OTLP_HEADERS"] = json!(SECRET_HEADER);
    doc
}

fn binding(digest: &str) -> Value {
    json!({"schema":"lintel.identity-binding/1","digests":{"userID":digest}})
}

fn capture_of(
    settings: &Value,
    startup_view: Option<&Value>,
    residue_view: Option<&Value>,
    identity: Option<&Value>,
) -> Value {
    capture(
        ENV,
        Some("2.1.283"),
        settings,
        startup_view,
        residue_view,
        identity,
    )
}

fn full_capture() -> Value {
    capture_of(
        &settings_with_header(),
        Some(&full_startup()),
        Some(&full_residue()),
        Some(&binding(DIGEST)),
    )
}

/// Recapture on the same environment and rule version with a different startup
/// observation, so only the injected fact differs between the compared pair.
fn recapture_startup(snapshot: &Value, startup_view: &Value) -> Value {
    let mut fresh = capture_of(
        &settings_with_header(),
        Some(startup_view),
        Some(&full_residue()),
        Some(&binding(DIGEST)),
    );
    fresh["environment_id"] = snapshot["environment_id"].clone();
    fresh["rule_version"] = snapshot["rule_version"].clone();
    fresh
}

/// The same environment without the forwarded header key at all.
fn without_header(snapshot: &Value) -> Value {
    let mut fresh = capture_of(
        &switches(),
        Some(&full_startup()),
        Some(&full_residue()),
        Some(&binding(DIGEST)),
    );
    fresh["environment_id"] = snapshot["environment_id"].clone();
    fresh["rule_version"] = snapshot["rule_version"].clone();
    fresh
}

fn settings_item<'a>(report: &'a Value, key: &str) -> &'a Value {
    report["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["area"] == "settings" && item["key"] == key)
        .unwrap_or_else(|| panic!("no settings change for {key}: {report}"))
}

fn declaration_item<'a>(report: &'a Value, path: &str) -> &'a Value {
    report["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["area"] == "declarations" && item["path"] == path)
        .unwrap_or_else(|| panic!("no declarations change for {path}: {report}"))
}

fn identity_item(report: &Value) -> &Value {
    report["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["area"] == "identity")
        .unwrap_or_else(|| panic!("no identity change: {report}"))
}

fn residue_item<'a>(report: &'a Value, category: &str) -> &'a Value {
    report["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["area"] == "residue" && item["category"] == category)
        .unwrap_or_else(|| panic!("no residue change for {category}: {report}"))
}

#[test]
fn capture_covers_every_section_and_masks_a_forwarded_header() {
    let snapshot = full_capture();
    assert_eq!(snapshot["schema"], SCHEMA);
    assert_eq!(snapshot["environment_id"], ENV);
    assert_eq!(snapshot["rule_version"], policy::RULE);
    assert_eq!(snapshot["product_version"], "2.1.283");
    assert!(uuid::Uuid::parse_str(snapshot["snapshot_id"].as_str().unwrap()).is_ok());
    assert!(!snapshot["created_at"].as_str().unwrap().is_empty());
    for (key, _) in policy::fields() {
        assert!(
            snapshot["settings"][key].is_object(),
            "missing switch {key}: {snapshot}"
        );
    }
    // Every rule-list switch, plus any masked key the rule list does not carry.
    let expected: std::collections::BTreeSet<&str> = policy::fields()
        .map(|(key, _)| key)
        .chain(std::iter::once("OTEL_EXPORTER_OTLP_HEADERS"))
        .collect();
    assert_eq!(
        snapshot["settings"].as_object().unwrap().len(),
        expected.len()
    );
    assert_eq!(snapshot["settings"]["DISABLE_TELEMETRY"]["value"], "1");
    assert_eq!(
        snapshot["settings"]["DISABLE_TELEMETRY"]["status"],
        "configured"
    );
    // This key's installed rule is nonempty-means-disabled: "0" is configured.
    assert_eq!(
        snapshot["settings"]["DISABLE_ERROR_REPORTING"]["status"],
        "configured"
    );
    assert_eq!(snapshot["settings"]["DO_NOT_TRACK"]["status"], "configured");
    // An undocumented boolean spelling is uncertain, never "configured".
    assert_eq!(
        snapshot["settings"]["CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY"]["status"],
        "uncertain"
    );
    assert_eq!(
        snapshot["settings"]["CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY"]["value"],
        "原字节值"
    );
    let masked = &snapshot["settings"]["OTEL_EXPORTER_OTLP_HEADERS"];
    assert!(masked["value"].is_null());
    assert_eq!(masked["masked"], true);
    assert!(
        !snapshot.to_string().contains(SECRET_HEADER),
        "a forwarded header value must never enter the snapshot: {snapshot}"
    );
    let declarations = snapshot["declarations"].as_array().unwrap();
    assert_eq!(declarations.len(), 3);
    assert_eq!(declarations[0]["scope"], "user");
    assert_eq!(declarations[0]["hooks"], true);
    assert_eq!(declarations[0]["mcp"], true);
    assert_eq!(declarations[0]["api_key_helper"], false);
    // A missing source declares no hook, MCP, plugin or helper.
    assert_eq!(declarations[1]["state"], "not_found");
    for flag in ["hooks", "mcp", "plugins", "api_key_helper"] {
        assert_eq!(declarations[1][flag], false, "{flag}");
    }
    assert_eq!(declarations[2]["scope"], "project");
    assert_eq!(declarations[2]["plugins"], true);
    assert_eq!(declarations[2]["mcp"], false);
    assert_eq!(snapshot["identity"]["userID"]["digest"], DIGEST);
    assert_eq!(snapshot["identity"]["userID"]["state"], "observed");
    assert_eq!(
        snapshot["residue"]["instructions"],
        json!({"count":2,"bytes":120})
    );
    assert_eq!(
        snapshot["residue"]["sessions"],
        json!({"count":4,"bytes":4096})
    );
    assert_eq!(snapshot["residue"]["totals"], full_residue()["totals"]);
    assert_eq!(snapshot["residue"]["scan_complete"], true);
    assert_eq!(snapshot["coverage"]["settings"], true);
    assert_eq!(snapshot["coverage"]["startup"], true);
    assert_eq!(snapshot["coverage"]["residue"], true);
    assert_eq!(snapshot["coverage"]["identity"], true);
    assert_eq!(snapshot["coverage"]["startup_complete"], true);
    assert!(snapshot["coverage"]["declarations_incomplete"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn capture_without_inputs_is_empty_but_never_absent() {
    let snapshot = capture("synthetic-env", None, &json!({}), None, None, None);
    assert_eq!(snapshot["schema"], SCHEMA);
    assert_eq!(snapshot["product_version"], Value::Null);
    assert_eq!(snapshot["declarations"], json!([]));
    assert_eq!(snapshot["residue"], json!({}));
    assert!(snapshot["identity"]["userID"]["digest"].is_null());
    assert_eq!(snapshot["identity"]["userID"]["state"], "unknown");
    assert_eq!(snapshot["coverage"]["settings"], true);
    for area in ["startup", "residue", "identity"] {
        assert_eq!(snapshot["coverage"][area], false, "{area}");
    }
    assert_eq!(snapshot["coverage"]["startup_complete"], false);
    // The rule list is still fully enumerated; an absent value is not a fact.
    assert_eq!(
        snapshot["settings"].as_object().unwrap().len(),
        policy::fields().count()
    );
    for (key, _) in policy::fields() {
        assert!(snapshot["settings"][key]["value"].is_null(), "{key}");
        // An absent value follows the key's documented default: default-on
        // keys read as configured rather than unchanged.
        assert_eq!(
            snapshot["settings"][key]["status"],
            policy::setting_status(key, &Value::Null),
            "{key}"
        );
    }
}

#[test]
fn capture_marks_unreadable_sources_without_claiming_a_flag() {
    let mut sources = source_set(true, false, false, false);
    sources.as_array_mut().unwrap().push(json!({
        "path":"/synthetic/managed/managed-settings.json","scope":"managed_candidate",
        "state":"access_limited","content_state":"unknown"}));
    sources.as_array_mut().unwrap().push(json!({
        "path":"/synthetic/user/rules","scope":"user_rules","state":"observed_directory"}));
    let view = startup(sources, false, true);
    let snapshot = capture_of(&switches(), Some(&view), None, None);
    let declarations = snapshot["declarations"].as_array().unwrap();
    assert_eq!(declarations[2]["state"], "access_limited");
    assert_eq!(declarations[3]["state"], "observed_directory");
    for item in [&declarations[2], &declarations[3]] {
        for flag in ["hooks", "mcp", "plugins", "api_key_helper"] {
            assert!(item[flag].is_null(), "{flag} in {item}");
        }
    }
    let incomplete = snapshot["coverage"]["declarations_incomplete"]
        .as_array()
        .unwrap();
    assert_eq!(incomplete.len(), 2);
    assert_eq!(
        incomplete[0]["path"],
        "/synthetic/managed/managed-settings.json"
    );
    assert_eq!(incomplete[0]["content_state"], "unknown");
    assert_eq!(incomplete[1]["state"], "observed_directory");
    // A truncated/limited scan is not a complete candidate scan.
    assert_eq!(snapshot["coverage"]["startup"], true);
    assert_eq!(snapshot["coverage"]["startup_complete"], false);
}

#[test]
fn capture_residue_never_lets_a_category_take_a_section_key() {
    let summary = residue(
        json!([
            location("instructions", "file", 1, 10),
            location("totals", "file", 99, 999),
            location("scan_complete", "file", 99, 999),
        ]),
        1,
        10,
        true,
    );
    let snapshot = capture_of(&switches(), None, Some(&summary), None);
    assert_eq!(
        snapshot["residue"]["instructions"],
        json!({"count":1,"bytes":10})
    );
    assert_eq!(
        snapshot["residue"]["totals"],
        json!({"file_count":1,"total_bytes":10})
    );
    assert_eq!(snapshot["residue"]["scan_complete"], true);
    assert_eq!(snapshot["residue"].as_object().unwrap().len(), 3);
}

#[test]
fn capture_without_reported_totals_uses_the_category_sum() {
    let summary = json!({"schema":"lintel.residue/1",
        "locations":[location("sessions","file",2,64)],"scan_complete":false});
    let snapshot = capture_of(&switches(), None, Some(&summary), None);
    assert_eq!(
        snapshot["residue"]["totals"],
        json!({"file_count":2,"total_bytes":64})
    );
    assert_eq!(snapshot["residue"]["scan_complete"], false);
}

#[test]
fn compare_identical_snapshot_is_unchanged() {
    let known = without_header(&full_capture());
    let report = compare(&known, &known);
    assert_eq!(report["status"], "unchanged", "{report}");
    assert!(report["rule_change"].is_null());
    for item in report["changes"].as_array().unwrap() {
        match item["area"].as_str().unwrap() {
            "identity" => assert_eq!(item["state"], "same"),
            // A masked value is never classified as an observed equality.
            "settings" if item["masked"] == true => assert_eq!(item["state"], "uncertain"),
            _ => assert_eq!(item["state"], "unchanged", "{item}"),
        }
    }
    assert_eq!(settings_item(&report, "DISABLE_TELEMETRY")["old"], "1");
    assert_eq!(settings_item(&report, "DISABLE_TELEMETRY")["new"], "1");
    assert!(settings_item(&report, "DISABLE_TELEMETRY")
        .get("reinterpreted_under_new_rules")
        .is_none());
    let summary = report["summary"].as_str().unwrap();
    assert!(summary.contains("未发现变化"), "{summary}");
    assert!(!summary.contains("遮蔽值"), "{summary}");
}

#[test]
fn compare_hidden_or_incomplete_observations_never_attest_complete_equality() {
    let hidden = full_capture();
    assert_eq!(compare(&hidden, &hidden)["status"], "incomplete");
    assert!(
        compare_pair(&hidden, &hidden)["settings"]["OTEL_EXPORTER_OTLP_HEADERS"]["same"].is_null()
    );
    let known = without_header(&hidden);
    let mut partial = known.clone();
    partial["residue"]["scan_complete"] = json!(false);
    partial["residue"]["instructions"] = json!({"count":0,"bytes":0});
    let report = compare(&known, &partial);
    assert_eq!(report["status"], "incomplete");
    assert!(report["changes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["area"] == "residue" && item["state"] == "incomplete"));
    partial = known.clone();
    partial["coverage"]["startup_complete"] = json!(false);
    assert_eq!(compare(&known, &partial)["status"], "incomplete");
}

#[test]
fn identity_digests_from_different_salt_domains_are_unknown() {
    let mut a = full_capture();
    let mut b = full_capture();
    a["identity"]["comparison_scope"] = json!("synthetic-scope-a");
    b["identity"]["comparison_scope"] = json!("synthetic-scope-b");
    b["identity"]["userID"]["digest"] = json!(OTHER_DIGEST);
    assert_eq!(identity_item(&compare(&a, &b))["state"], "unknown");
    assert_eq!(compare_pair(&a, &b)["identity"]["userID"], "unknown");
    b["identity"]["comparison_scope"] = json!("synthetic-scope-a");
    assert_eq!(identity_item(&compare(&a, &b))["state"], "changed");
}

#[test]
fn incomplete_declarations_do_not_become_absence_or_equality() {
    let known = without_header(&full_capture());
    let mut partial = known.clone();
    partial["coverage"]["startup_complete"] = json!(false);
    partial["declarations"].as_array_mut().unwrap().remove(0);
    let report = compare(&known, &partial);
    assert_eq!(
        declaration_item(&report, "/synthetic/user/settings.json")["state"],
        "incomplete"
    );
    assert_eq!(report["status"], "incomplete");
    let mut unknown = known.clone();
    unknown["declarations"][0]["hooks"] = Value::Null;
    assert_eq!(
        declaration_item(
            &compare(&unknown, &unknown),
            "/synthetic/user/settings.json"
        )["state"],
        "incomplete"
    );
    unknown["settings"]["OTEL_LOG_USER_PROMPTS"]["value"] = json!("1");
    assert_eq!(compare(&known, &unknown)["status"], "changed");
}

#[test]
fn exporter_credentials_and_unknown_typed_values_stay_private_or_uncertain() {
    let settings = json!({"env":{"OTEL_EXPORTER_OTLP_ENDPOINT":"https://synthetic-user:synthetic-password@collector.example.invalid/v1?token=synthetic-token", "OTEL_LOG_USER_PROMPTS":true}});
    let snap = capture_of(
        &settings,
        Some(&full_startup()),
        Some(&full_residue()),
        Some(&binding(DIGEST)),
    );
    assert_eq!(
        snap["settings"]["OTEL_EXPORTER_OTLP_ENDPOINT"]["masked"],
        true
    );
    for text in [
        snap.to_string(),
        compare(&snap, &snap).to_string(),
        compare_pair(&snap, &snap).to_string(),
    ] {
        assert!(!text.contains("synthetic-password"));
        assert!(!text.contains("synthetic-token"));
    }
    assert_eq!(
        settings_item(&compare(&snap, &snap), "OTEL_LOG_USER_PROMPTS")["state"],
        "uncertain"
    );
}

#[test]
fn compare_settings_change_removed_added_and_uncertain() {
    let old = full_capture();
    // The forwarded header disappears from the source. The rule list keeps the
    // key enumerated with a visible null, so masked presence versus observed
    // absence is still a removed setting, never a claimed value.
    let without = without_header(&old);
    let removed_report = compare(&old, &without);
    assert_eq!(removed_report["status"], "changed");
    let removed = settings_item(&removed_report, "OTEL_EXPORTER_OTLP_HEADERS");
    assert_eq!(removed["state"], "removed");
    assert_eq!(removed["masked"], true);
    assert!(removed["old"].is_null());
    assert!(removed["new"].is_null());

    let added_report = compare(&without, &old);
    let added = settings_item(&added_report, "OTEL_EXPORTER_OTLP_HEADERS");
    assert_eq!(added["state"], "added");

    let mut drifted = full_capture();
    drifted["settings"]["DISABLE_TELEMETRY"]["value"] = json!(Value::Null);
    drifted["settings"]["DO_NOT_TRACK"]["value"] = json!("other");
    let report = compare(&old, &drifted);
    assert_eq!(report["status"], "changed");
    let cleared = settings_item(&report, "DISABLE_TELEMETRY");
    assert_eq!(cleared["state"], "changed");
    assert_eq!(cleared["old"], "1");
    assert!(cleared["new"].is_null());
    let uncertain = settings_item(&report, "DO_NOT_TRACK");
    assert_eq!(uncertain["state"], "changed");
    assert_eq!(uncertain["old"], "1");
    assert_eq!(uncertain["new"], "other");
    assert_eq!(
        settings_item(&report, "DISABLE_ERROR_REPORTING")["state"],
        "unchanged"
    );
    assert_eq!(
        declaration_item(&report, "/synthetic/user/settings.json")["state"],
        "unchanged"
    );
    assert_eq!(identity_item(&report)["state"], "same");
    assert!(!report.to_string().contains(SECRET_HEADER));
}

#[test]
fn compare_rule_version_change_is_not_file_drift() {
    let old = full_capture();
    let mut new = old.clone();
    new["rule_version"] = json!("claude-privacy-v4-2026-10-20");
    let report = compare(&old, &new);
    assert_eq!(report["status"], "rules_updated");
    assert_eq!(report["rule_change"]["state"], "rules_updated");
    assert_eq!(report["rule_change"]["old"], policy::RULE);
    assert_eq!(report["rule_change"]["new"], "claude-privacy-v4-2026-10-20");
    for item in report["changes"].as_array().unwrap() {
        match (item["area"].as_str().unwrap(), item["masked"] == true) {
            ("settings", _) => assert_eq!(item["reinterpreted_under_new_rules"], true, "{item}"),
            ("identity", _) => assert_eq!(item["state"], "same"),
            _ => assert_eq!(item["state"], "unchanged", "{item}"),
        }
    }
    // A rules change never hides a real settings difference.
    let mut drifted = new.clone();
    drifted["settings"]["DISABLE_TELEMETRY"]["value"] = json!(Value::Null);
    let report = compare(&old, &drifted);
    assert_eq!(report["status"], "changed");
    assert_eq!(
        settings_item(&report, "DISABLE_TELEMETRY")["state"],
        "changed"
    );
    assert_eq!(
        settings_item(&report, "DISABLE_TELEMETRY")["reinterpreted_under_new_rules"],
        true
    );
    assert_eq!(report["rule_change"]["state"], "rules_updated");
    assert_eq!(report["rule_change"]["old"], policy::RULE);
}

#[test]
fn compare_declarations_appeared_disappeared_and_flags_changed() {
    let old = full_capture();

    let mut with_managed = full_startup();
    with_managed["sources"]
        .as_array_mut()
        .unwrap()
        .push(observed_source(
            "managed",
            "/synthetic/managed/managed-settings.json",
            false,
            false,
            false,
        ));
    let appeared_report = compare(&old, &recapture_startup(&old, &with_managed));
    assert_eq!(appeared_report["status"], "changed");
    let item = declaration_item(&appeared_report, "/synthetic/managed/managed-settings.json");
    assert_eq!(item["state"], "appeared");
    assert_eq!(item["scope"], "managed");
    assert!(item["old"].is_null());
    assert_eq!(item["new"]["mcp"], false);
    assert_eq!(item["new"]["plugins"], false);
    assert_eq!(item["new"]["hooks"], false);

    let mut without_project = source_set(true, true, false, false);
    without_project.as_array_mut().unwrap().push(
        json!({"path":"/synthetic/managed/other.json","scope":"managed","state":"not_found"}),
    );
    let disappeared_report = compare(
        &old,
        &recapture_startup(&old, &startup(without_project, true, false)),
    );
    let item = declaration_item(&disappeared_report, "/synthetic/project/settings.json");
    assert_eq!(item["state"], "disappeared");
    assert_eq!(item["old"]["plugins"], true);
    assert!(item["new"].is_null());

    let flipped_view = startup(source_set(false, true, true, false), true, false);
    let flipped_report = compare(&old, &recapture_startup(&old, &flipped_view));
    let item = declaration_item(&flipped_report, "/synthetic/user/settings.json");
    assert_eq!(item["state"], "flags_changed");
    let flags = item["flags"].as_array().unwrap();
    assert_eq!(flags.len(), 2);
    assert_eq!(flags[0]["flag"], "hooks");
    assert_eq!(flags[0]["from"], true);
    assert_eq!(flags[0]["to"], false);
    assert_eq!(flags[1]["flag"], "plugins");
    assert_eq!(flags[1]["from"], false);
    assert_eq!(flags[1]["to"], true);

    // An unreadable source cannot attest a declaration change. Preserve the
    // read-state transition separately instead of counting null as a new flag.
    let mut limited = source_set(true, true, false, false);
    limited.as_array_mut().unwrap()[0]["state"] = json!("access_limited");
    limited.as_array_mut().unwrap()[0]["content_state"] = json!("unknown");
    let limited_report = compare(
        &old,
        &recapture_startup(&old, &startup(limited, true, true)),
    );
    let item = declaration_item(&limited_report, "/synthetic/user/settings.json");
    assert_eq!(item["state"], "incomplete");
    assert_eq!(item["observation_change"]["from"], "observed");
    assert_eq!(item["observation_change"]["to"], "access_limited");
    assert!(item["new"]["hooks"].is_null());
    assert!(item["flags"].is_null());
}

#[test]
fn compare_identity_has_exactly_three_branches() {
    let old = full_capture();
    let same = compare(&old, &old);
    assert_eq!(identity_item(&same)["state"], "same");
    assert_eq!(identity_item(&same)["subject"], "userID");
    assert!(!same.to_string().contains(DIGEST));

    let other = capture_of(
        &settings_with_header(),
        Some(&full_startup()),
        Some(&full_residue()),
        Some(&binding(OTHER_DIGEST)),
    );
    let changed = compare(&old, &other);
    assert_eq!(changed["status"], "changed");
    assert_eq!(identity_item(&changed)["state"], "changed");

    let unknown = capture_of(
        &settings_with_header(),
        Some(&full_startup()),
        Some(&full_residue()),
        None,
    );
    let report = compare(&old, &unknown);
    assert_eq!(identity_item(&report)["state"], "unknown");
    assert_eq!(report["status"], "incomplete");
    for output in [&same, &changed, &report] {
        let text = output.to_string();
        assert!(!text.contains(DIGEST), "digest leaked: {text}");
        assert!(!text.contains(OTHER_DIGEST), "digest leaked: {text}");
    }
}

#[test]
fn compare_residue_reports_signed_deltas() {
    let old = full_capture();
    let mut shrink = old.clone();
    shrink["residue"]["sessions"] = json!({"count":1,"bytes":2048});
    shrink["residue"]["residue_cache"] = json!({"count":5,"bytes":900});
    let report = compare(&old, &shrink);
    assert_eq!(report["status"], "changed");
    let sessions = residue_item(&report, "sessions");
    assert_eq!(sessions["state"], "changed");
    assert_eq!(sessions["old_count"], 4);
    assert_eq!(sessions["new_count"], 1);
    assert_eq!(sessions["count_delta"], -3);
    assert_eq!(sessions["old_bytes"], 4096);
    assert_eq!(sessions["new_bytes"], 2048);
    assert_eq!(sessions["bytes_delta"], -2048);
    let added = residue_item(&report, "residue_cache");
    assert_eq!(added["state"], "added");
    assert_eq!(added["count_delta"], 5);
    assert_eq!(added["bytes_delta"], 900);
    assert_eq!(residue_item(&report, "instructions")["state"], "unchanged");
    assert_eq!(residue_item(&report, "instructions")["count_delta"], 0);
    // Totals and scan completeness are not categories.
    assert!(report["changes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| item["category"] != "totals" && item["category"] != "scan_complete"));

    let mut grow = old.clone();
    grow["residue"]["instructions"] = json!({"count":9,"bytes":200});
    let report = compare(&old, &grow);
    let instructions = residue_item(&report, "instructions");
    assert_eq!(instructions["count_delta"], 7);
    assert_eq!(instructions["bytes_delta"], 80);
    // Totals are reported per side, not summed into a fabricated category.
    assert!(report["changes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["area"] == "residue" && item["category"] == "instructions"));
}

#[test]
fn compare_without_a_section_is_incomplete_not_changed() {
    let old = full_capture();
    let bare = capture_of(
        &settings_with_header(),
        None,
        Some(&full_residue()),
        Some(&binding(DIGEST)),
    );
    let report = compare(&old, &bare);
    assert_eq!(report["status"], "incomplete");
    let declarations = report["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["area"] == "declarations")
        .unwrap();
    assert_eq!(declarations["state"], "incomplete");
    assert!(declarations["path"].is_null());
    for (key, _) in policy::fields() {
        // The masked header is present on both sides; two visible nulls cannot
        // attest equality, so it stays uncertain rather than unchanged.
        let expected = if key == "OTEL_EXPORTER_OTLP_HEADERS" {
            "uncertain"
        } else {
            "unchanged"
        };
        assert_eq!(settings_item(&report, key)["state"], expected, "{key}");
    }
    assert_eq!(identity_item(&report)["state"], "same");

    let mut no_identity = capture_of(
        &settings_with_header(),
        Some(&full_startup()),
        Some(&full_residue()),
        None,
    );
    no_identity["environment_id"] = old["environment_id"].clone();
    let report = compare(&old, &no_identity);
    assert_eq!(report["status"], "incomplete");
    assert_eq!(
        declaration_item(&report, "/synthetic/user/settings.json")["state"],
        "unchanged"
    );
    assert!(report["summary"].as_str().unwrap().contains("未观测"));
}

#[test]
fn compare_settings_coverage_gap_marks_items_incomplete() {
    let old = full_capture();
    let mut unread = old.clone();
    unread["coverage"]["settings"] = json!(false);
    let report = compare(&old, &unread);
    assert_eq!(report["status"], "incomplete");
    assert_eq!(
        settings_item(&report, "DISABLE_TELEMETRY")["state"],
        "incomplete"
    );
    // Only the uncovered area is undecided.
    assert_eq!(
        declaration_item(&report, "/synthetic/user/settings.json")["state"],
        "unchanged"
    );
    assert_eq!(identity_item(&report)["state"], "same");
    // An uncovered area never invents a verdict, and a real change still wins.
    let mut drifted = unread.clone();
    drifted["residue"]["instructions"] = json!({"count":8,"bytes":300});
    let report = compare(&old, &drifted);
    assert_eq!(report["status"], "changed");
    assert_eq!(
        settings_item(&report, "DISABLE_TELEMETRY")["state"],
        "incomplete"
    );
    assert_eq!(residue_item(&report, "instructions")["count_delta"], 6);
}

#[test]
fn compare_pair_aligns_identity_settings_and_declarations() {
    let a = full_capture();
    let mut b = full_capture();
    b["settings"]["DISABLE_TELEMETRY"]["value"] = Value::Null;
    b["settings"]["DO_NOT_TRACK"]["value"] = json!("1");
    b["declarations"].as_array_mut().unwrap().remove(1);
    b["declarations"]
        .as_array_mut()
        .unwrap()
        .push(observed_source(
            "managed",
            "/synthetic/managed/managed-settings.json",
            false,
            false,
            false,
        ));
    b["residue"]["instructions"] = json!({"count":1,"bytes":60});
    let report = compare_pair(&a, &b);
    assert_eq!(report["identity"]["userID"], "same");
    assert_eq!(report["settings"]["DISABLE_TELEMETRY"]["a"], "1");
    assert!(report["settings"]["DISABLE_TELEMETRY"]["b"].is_null());
    assert_eq!(report["settings"]["DISABLE_TELEMETRY"]["same"], false);
    assert_eq!(report["settings"]["DO_NOT_TRACK"]["same"], true);
    assert_eq!(
        report["declarations"]["only_a"][0]["path"],
        "/synthetic/user/CLAUDE.md"
    );
    assert_eq!(
        report["declarations"]["only_b"][0]["path"],
        "/synthetic/managed/managed-settings.json"
    );
    assert_eq!(report["declarations"]["only_b"][0]["scope"], "managed");
    assert_eq!(report["declarations"]["both"].as_array().unwrap().len(), 2);
    assert_eq!(
        report["declarations"]["both"][0]["path"],
        "/synthetic/user/settings.json"
    );
    assert_eq!(
        report["residue"]["a"]["instructions"],
        json!({"count":2,"bytes":120})
    );
    assert_eq!(
        report["residue"]["b"]["instructions"],
        json!({"count":1,"bytes":60})
    );
    assert!(report["note"].as_str().unwrap().contains("不推断账号"));
}

#[test]
fn compare_pair_identity_has_three_branches_and_never_publishes_a_digest() {
    let a = full_capture();
    assert_eq!(compare_pair(&a, &a)["identity"]["userID"], "same");

    let different = capture_of(
        &settings_with_header(),
        Some(&full_startup()),
        Some(&full_residue()),
        Some(&binding(OTHER_DIGEST)),
    );
    assert_eq!(
        compare_pair(&a, &different)["identity"]["userID"],
        "different"
    );

    let unknown = capture(OTHER_ENV, None, &switches(), None, None, None);
    assert_eq!(compare_pair(&a, &unknown)["identity"]["userID"], "unknown");
    assert_eq!(compare_pair(&unknown, &a)["identity"]["userID"], "unknown");

    // An empty digest map is an unobserved identity, not a shared one.
    let empty = capture(
        OTHER_ENV,
        None,
        &switches(),
        None,
        None,
        Some(&json!({"digests":{}})),
    );
    assert_eq!(compare_pair(&empty, &a)["identity"]["userID"], "unknown");

    for (left, right) in [(&a, &a), (&a, &different), (&a, &unknown), (&unknown, &a)] {
        for report in [compare(left, right), compare_pair(left, right)] {
            let text = report.to_string();
            assert!(!text.contains(DIGEST), "digest leaked: {text}");
            assert!(!text.contains(OTHER_DIGEST), "digest leaked: {text}");
        }
    }
    // The unknown-pair note admits the limited observation rather than guessing.
    assert!(compare_pair(&a, &unknown)["note"]
        .as_str()
        .unwrap()
        .contains("启动来源未观测"));
}

#[test]
fn compare_pair_never_publishes_a_masked_value() {
    let a = full_capture();
    // Even a caller-supplied value that escaped masking must not be republished.
    let mut b = full_capture();
    b["settings"]["OTEL_EXPORTER_OTLP_HEADERS"]["value"] = json!("leaked-visible-value");
    b["settings"]["OTEL_EXPORTER_OTLP_HEADERS"]["masked"] = json!(true);
    let report = compare_pair(&a, &b);
    let entry = &report["settings"]["OTEL_EXPORTER_OTLP_HEADERS"];
    assert_eq!(entry["masked"], true);
    assert!(entry["a"].is_null());
    assert!(entry["b"].is_null());
    assert!(report["note"].as_str().unwrap().contains("遮蔽值"));
    let text = report.to_string();
    assert!(!text.contains("leaked-visible-value"), "{text}");
    assert!(!text.contains(SECRET_HEADER), "{text}");
    // An unmasked side of the same key still keeps its ordinary comparison.
    let mut unmasked = full_capture();
    unmasked["settings"]["DISABLE_TELEMETRY"]["value"] = json!("0");
    assert_eq!(
        compare_pair(&a, &unmasked)["settings"]["DISABLE_TELEMETRY"]["same"],
        false
    );
}

#[test]
fn masked_entries_publish_no_value_in_any_public_output() {
    let with_secret = full_capture();
    let without = without_header(&with_secret);
    let reports = [
        compare(&with_secret, &without),
        compare_pair(&with_secret, &without),
    ];
    for report in reports {
        let text = report.to_string();
        assert!(!text.contains(SECRET_HEADER), "{text}");
        assert!(!text.contains(DIGEST), "{text}");
    }
}

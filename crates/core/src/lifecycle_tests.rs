use super::*;
use std::os::unix::fs::PermissionsExt;

fn fixture() -> (tempfile::TempDir, Engine, Value, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let home = base.join("home");
    let root = home.join("fixture");
    fs::create_dir_all(&root).unwrap();
    let engine = Engine::new(home, base.join("state")).unwrap();
    let e = engine.register("synthetic", &root, false).unwrap();
    (temp, engine, e, root)
}
fn data(engine: &Engine, r: Value) -> Value {
    let v = engine.request(r);
    assert_eq!(v["ok"], true, "{v}");
    v["data"].clone()
}
const PASS: &str = "synthetic archive passphrase";
fn write_login(root: &Path, token: &str) {
    save(
        &root.join(".credentials.json"),
        &json!({
            "claudeAiOauth":{"accessToken":token},
            "mcpOAuth":{"synthetic-server":{"accessToken":"SYNTHETIC_MCP_KEEP"}}
        }),
    )
    .unwrap();
}
fn clean_plan(engine: &Engine, e: &Value, recipe: &str, logout: bool) -> Value {
    data(
        engine,
        json!({"command":"plan_cleanup","environment_id":e["id"],"recipe":recipe,"writers_confirmed_stopped":true,"official_logout":logout,"categories":["instructions"]}),
    )
}
fn run(engine: &Engine, p: &Value) -> Value {
    data(
        engine,
        json!({"command":"execute","plan_id":p["id"],"approval":p["hash"],"archive_passphrase":PASS}),
    )
}

#[test]
fn cleanup_inspection_keeps_unreadable_credentials_metadata_only() {
    let (_temp, engine, environment, root) = fixture();
    let credentials = root.join(".credentials.json");
    write_login(&root, "SYNTHETIC_UNREADABLE_LOGIN");
    let original = fs::read(&credentials).unwrap();
    let frozen = clean_plan(&engine, &environment, "repair_login", false);
    fs::set_permissions(&credentials, fs::Permissions::from_mode(0)).unwrap();
    // Root can bypass Unix read permission bits; on ordinary supported desktop
    // and CI users this proves the body is actually unreadable, not mocked.
    let unreadable = unsafe { libc::geteuid() } != 0;
    if unreadable {
        assert_eq!(
            fs::read(&credentials).unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
    }
    let inspection = data(
        &engine,
        json!({"command":"cleanup_inspect","environment_id":environment["id"]}),
    );
    assert_eq!(
        inspection["files"],
        json!([
            {"path":credentials,"category":"credentials","present":true},
            {"path":root.join(".claude.json"),"category":"client_state","present":false}
        ])
    );
    assert!(!inspection
        .to_string()
        .contains("SYNTHETIC_UNREADABLE_LOGIN"));
    let inventory = data(
        &engine,
        json!({"command":"auth_inventory","environment_id":environment["id"]}),
    );
    let source = inventory["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["path"] == json!(credentials))
        .unwrap();
    assert_eq!(source["evidence"], "inaccessible");
    assert_eq!(source["presence"], "unknown");
    let preview = engine.request(json!({"command":"plan_cleanup","environment_id":environment["id"],"recipe":"repair_login","writers_confirmed_stopped":true,"auth_source_ids":[source["id"]]}));
    assert_eq!(
        preview["error"]["code"], "auth_source_not_removable",
        "{preview}"
    );
    let execute = engine
        .request(json!({"command":"execute","plan_id":frozen["id"],"approval":frozen["hash"]}));
    assert_eq!(
        execute["error"]["code"], "auth_source_unreadable",
        "{execute}"
    );
    assert!(!engine.path("jobs", frozen["id"].as_str().unwrap()).exists());
    assert_eq!(
        fs::metadata(&credentials).unwrap().permissions().mode() & 0o777,
        0
    );
    fs::set_permissions(&credentials, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(fs::read(&credentials).unwrap(), original);
}

#[test]
fn cleanup_inspection_observes_large_files_without_weakening_plan_admission() {
    let (_temp, engine, environment, root) = fixture();
    let credentials = root.join(".credentials.json");
    fs::File::create(&credentials)
        .unwrap()
        .set_len(8 * 1024 * 1024 + 1)
        .unwrap();
    let inspect = || {
        data(
            &engine,
            json!({"command":"cleanup_inspect","environment_id":environment["id"]}),
        )
    };
    assert_eq!(inspect()["files"][0]["present"], true);
    let repair = json!({"command":"plan_cleanup","environment_id":environment["id"],"recipe":"repair_login","writers_confirmed_stopped":true,"official_logout":false});
    let inventory = data(
        &engine,
        json!({"command":"auth_inventory","environment_id":environment["id"]}),
    );
    let source = inventory["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["path"] == json!(credentials))
        .unwrap();
    assert_eq!(source["reason"], "file_too_large");
    assert_eq!(source["presence"], "unknown");
    let mut selected = repair.clone();
    selected["auth_source_ids"] = json!([source["id"]]);
    let refused = engine.request(selected);
    assert_eq!(
        refused["error"]["code"], "auth_source_not_removable",
        "{refused}"
    );
    write_login(&root, "SYNTHETIC_BOUNDED_LOGIN");
    let state = root.join(".claude.json");
    fs::File::create(&state)
        .unwrap()
        .set_len(8 * 1024 * 1024 + 1)
        .unwrap();
    let observed = inspect();
    assert!(observed["files"]
        .as_array()
        .unwrap()
        .iter()
        .all(|file| file["present"] == true));
    // Unknown mixed state is retained even for reset_client; it does not force
    // an unsafe whole-file read, archive or deletion to clean a known field.
    let plan = data(
        &engine,
        json!({"command":"plan_cleanup","environment_id":environment["id"],"recipe":"reset_client","writers_confirmed_stopped":true,"official_logout":false,"categories":[]}),
    );
    let job = run(&engine, &plan);
    assert_eq!(job["local_cleanup"], "completed", "{job}");
    assert!(job["state_archive_path"].is_null());
    assert_eq!(fs::metadata(&state).unwrap().len(), 8 * 1024 * 1024 + 1);
    assert!(load(&credentials).unwrap().get("claudeAiOauth").is_none());
    assert_eq!(
        load(&credentials).unwrap()["mcpOAuth"]["synthetic-server"]["accessToken"],
        "SYNTHETIC_MCP_KEEP"
    );
    assert!(job["auth_verification"]["inventory"]["sources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["path"] == json!(state) && s["presence"] == "unknown"));
}

#[test]
fn cleanup_inspection_rejects_links_instead_of_reporting_absence() {
    let (_temp, engine, environment, root) = fixture();
    let credentials = root.join(".credentials.json");
    let target = root.join("unrelated-synthetic-file");
    std::os::unix::fs::symlink(&target, &credentials).unwrap();
    for target_exists in [false, true] {
        if target_exists {
            fs::write(&target, "unrelated bytes stay unchanged").unwrap();
        }
        let result =
            engine.request(json!({"command":"cleanup_inspect","environment_id":environment["id"]}));
        assert_eq!(result["error"]["code"], "symlink_target", "{result}");
        let inventory = data(
            &engine,
            json!({"command":"auth_inventory","environment_id":environment["id"]}),
        );
        let source = inventory["sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["path"] == json!(credentials))
            .unwrap();
        assert_eq!(source["presence"], "unknown");
        assert_eq!(source["removable"], false);
        let result = engine.request(json!({"command":"plan_cleanup","environment_id":environment["id"],"recipe":"repair_login","writers_confirmed_stopped":true,"auth_source_ids":[source["id"]]}));
        assert_eq!(
            result["error"]["code"], "auth_source_not_removable",
            "{result}"
        );
    }
    assert!(fs::symlink_metadata(&credentials)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(
        fs::read(&target).unwrap(),
        b"unrelated bytes stay unchanged"
    );
    assert_eq!(fs::read_dir(engine.state.join("plans")).unwrap().count(), 0);
    assert_eq!(fs::read_dir(engine.state.join("jobs")).unwrap().count(), 0);
}

#[test]
fn cleanup_preserves_work_and_mixed_state_without_archiving_or_replaying_credentials() {
    let (_tmp, engine, e, root) = fixture();
    write_login(&root, "SYNTHETIC_LOGIN_DO_NOT_ARCHIVE");
    fs::write(
        root.join(".claude.json"),
        r#"{"primaryApiKey":"SYNTHETIC_KEY_DO_NOT_ARCHIVE","mcpServers":{"example":{}},"syntheticSecret":"not-public"}"#,
    )
    .unwrap();
    fs::write(root.join("CLAUDE.md"), "Keep this instruction").unwrap();
    fs::write(
        root.join("settings.json"),
        r#"{"hooks":{"keep":"do not execute"}}"#,
    )
    .unwrap();
    let p = clean_plan(&engine, &e, "reset_client", false);
    let j = run(&engine, &p);
    assert_eq!(j["status"], "partially_completed");
    assert_eq!(j["local_cleanup"], "completed");
    let credentials = load(&root.join(".credentials.json")).unwrap();
    assert!(credentials.get("claudeAiOauth").is_none());
    assert_eq!(
        credentials["mcpOAuth"]["synthetic-server"]["accessToken"],
        "SYNTHETIC_MCP_KEEP"
    );
    let state = load(&root.join(".claude.json")).unwrap();
    assert!(state.get("primaryApiKey").is_none());
    assert_eq!(state["syntheticSecret"], "not-public");
    assert!(state["mcpServers"]["example"].is_object());
    assert!(root.join("CLAUDE.md").exists());
    assert!(root.join("settings.json").exists());
    assert!(j["state_archive_path"].is_null());
    let archive = data(
        &engine,
        json!({"command":"archive_inspect","job_id":j["id"],"archive_passphrase":PASS}),
    );
    assert_eq!(archive["files"].as_array().unwrap().len(), 1);
    assert_eq!(archive["files"][0]["path"], "CLAUDE.md");
    for value in [
        &p,
        &j,
        &load(&engine.path("plans", p["id"].as_str().unwrap())).unwrap(),
    ] {
        for secret in [
            "SYNTHETIC_LOGIN_DO_NOT_ARCHIVE",
            "SYNTHETIC_KEY_DO_NOT_ARCHIVE",
            "SYNTHETIC_MCP_KEEP",
            "not-public",
        ] {
            assert!(!value.to_string().contains(secret));
        }
    }
    write_login(&root, "new-login-must-survive");
    let newly_signed_in = fs::read(root.join(".credentials.json")).unwrap();
    assert_eq!(run(&engine, &p)["id"], j["id"]);
    assert_eq!(
        fs::read(root.join(".credentials.json")).unwrap(),
        newly_signed_in
    );
}

#[test]
fn existing_work_archive_blocks_cleanup_and_queries_do_not_retry_publication() {
    let (_tmp, engine, e, root) = fixture();
    write_login(&root, "SYNTHETIC_LOGIN_PRESERVE_ON_ARCHIVE_FAILURE");
    fs::write(root.join(".claude.json"), r#"{"synthetic":true}"#).unwrap();
    fs::write(root.join("CLAUDE.md"), "Keep this work").unwrap();
    let p = clean_plan(&engine, &e, "reset_client", false);
    let backup = engine
        .state
        .join("archives")
        .join(format!("{}.age", p["id"].as_str().unwrap()));
    // The work archive rejects an occupied destination before publication.
    // No raw credential backup is created to recover from that failure.
    fs::create_dir(&backup).unwrap();
    let j = run(&engine, &p);
    assert_eq!(j["status"], "needs_reconciliation", "{j}");
    assert!(j["archive_path"].is_null(), "{j}");
    assert_eq!(j["error"]["code"], "output_exists", "{j}");
    assert_eq!(j["local_cleanup"], "unverified");
    assert!(j["steps"]
        .as_array()
        .unwrap()
        .iter()
        .all(|step| !step["id"].as_str().unwrap().starts_with("auth_edit_")));
    assert!(j["state_archive_path"].is_null() && j["new_root"].is_null());
    assert!(root.join(".credentials.json").exists());
    assert!(root.join(".claude.json").exists());
    assert_eq!(
        fs::read_to_string(root.join("CLAUDE.md")).unwrap(),
        "Keep this work"
    );
    let stored = load(&engine.path("jobs", p["id"].as_str().unwrap())).unwrap();
    assert_eq!(stored["archive_path"], j["archive_path"]);
    let queried = data(&engine, json!({"command":"job","job_id":j["id"]}));
    assert_eq!(queried["archive_path"], j["archive_path"]);
    assert_eq!(queried["status"], "needs_reconciliation");
    assert_eq!(run(&engine, &p)["id"], j["id"]);
    assert!(
        backup.is_dir(),
        "query must not overwrite or remove the uncertain target"
    );
}

#[test]
fn changed_credentials_block_cleanup_before_acceptance() {
    let (_tmp, engine, e, root) = fixture();
    write_login(&root, "SYNTHETIC_OLD_GENERATION");
    let p = clean_plan(&engine, &e, "repair_login", false);
    write_login(&root, "SYNTHETIC_NEW_GENERATION");
    let new_bytes = fs::read(root.join(".credentials.json")).unwrap();
    let r = engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}));
    assert_eq!(r["error"]["code"], "stale_auth_source", "{r}");
    assert!(!engine.path("jobs", p["id"].as_str().unwrap()).exists());
    assert_eq!(fs::read(root.join(".credentials.json")).unwrap(), new_bytes);
}

fn auth_fixture() -> (tempfile::TempDir, Engine, Value, PathBuf) {
    let (temp, mut engine, mut environment, root) = fixture();
    engine.executable_search_path = Some("/usr/bin:/bin".into());
    let script = engine.home.join(".local/bin/claude");
    fs::create_dir_all(script.parent().unwrap()).unwrap();
    fs::write(
        &script,
        r##"#!/bin/sh
case "$2" in
status)
count=0
if test -f "$CLAUDE_CONFIG_DIR/probe-count"; then count=$(cat "$CLAUDE_CONFIG_DIR/probe-count"); fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDE_CONFIG_DIR/probe-count"
cat "$CLAUDE_CONFIG_DIR/synthetic-auth.json"
exit 0;;
logout)
touch "$CLAUDE_CONFIG_DIR/logout-attempted"
exit 9;;
esac
exit 9
"##,
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    environment["executable"] = json!(script);
    save(
        &engine.state.join("inventory.json"),
        &json!([environment.clone()]),
    )
    .unwrap();
    set_auth_status(&root, Some("account-a@example.invalid"));
    (temp, engine, environment, root)
}

fn set_auth_status(root: &Path, email: Option<&str>) {
    save(&root.join("synthetic-auth.json"), &json!({"configDirectory":root,"authMethod":"claude.ai","email":email,"orgId":"synthetic-org","privateExtra":"SYNTHETIC_AUTH_STATUS_DO_NOT_PUBLISH"})).unwrap();
}

#[test]
fn cleanup_auth_account_switch_does_not_expand_local_selection_or_claim_logout() {
    let (_temp, engine, environment, root) = auth_fixture();
    write_login(&root, "SYNTHETIC_SELECTED_LOGIN");
    let plan = data(
        &engine,
        json!({"command":"plan_cleanup","environment_id":environment["id"],"recipe":"repair_login","writers_confirmed_stopped":true,"verify_auth_status":true}),
    );
    assert!(
        !root.join("probe-count").exists(),
        "preview does not execute authentication"
    );
    set_auth_status(&root, Some("account-b@example.invalid"));
    let result = run(&engine, &plan);
    assert_eq!(result["local_cleanup"], "completed", "{result}");
    assert_eq!(
        result["auth_verification"]["runtime"]["observation"]["logged_in"],
        true
    );
    assert_eq!(
        result["auth_verification"]["server_revocation"],
        "unverified"
    );
    assert!(!result.to_string().contains("account-b@example.invalid"));
    assert!(load(&root.join(".credentials.json"))
        .unwrap()
        .get("claudeAiOauth")
        .is_none());
    assert!(!root.join("logout-attempted").exists());
}

#[test]
fn cleanup_auth_missing_subject_refuses_logout_preview_but_allows_local_cleanup() {
    let (_temp, engine, environment, root) = auth_fixture();
    write_login(&root, "SYNTHETIC_LOGIN_WITHOUT_ACCOUNT_METADATA");
    set_auth_status(&root, None);
    let probe = data(
        &engine,
        json!({"command":"auth_probe","environment_id":environment["id"]}),
    );
    assert_eq!(probe["identity_observed"], false);
    let result = engine.request(json!({"command":"plan_cleanup","environment_id":environment["id"],"recipe":"repair_login","writers_confirmed_stopped":true,"official_logout":true}));
    assert_eq!(
        result["error"]["code"], "official_logout_scope_unsafe",
        "{result}"
    );
    let local = clean_plan(&engine, &environment, "repair_login", false);
    assert_eq!(run(&engine, &local)["local_cleanup"], "completed");
    assert!(load(&root.join(".credentials.json"))
        .unwrap()
        .get("claudeAiOauth")
        .is_none());
    assert!(!root.join("logout-attempted").exists());
}

#[test]
fn cleanup_auth_changed_after_preservation_retains_work_without_logout() {
    let (_temp, engine, environment, root) = auth_fixture();
    fs::write(root.join("CLAUDE.md"), "Synthetic instructions retained").unwrap();
    write_login(&root, "SYNTHETIC_FROZEN_LOGIN");
    let plan = clean_plan(&engine, &environment, "reset_client", false);
    fn replace_after_preservation(path: &Path, _: &Path, phase: &str) {
        if phase == "prepared" {
            write_login(path.parent().unwrap(), "SYNTHETIC_CONCURRENT_LOGIN");
        }
    }
    auth_cleanup::set_edit_hook(Some(replace_after_preservation));
    let job = run(&engine, &plan);
    auth_cleanup::set_edit_hook(None);
    assert_eq!(job["error"]["code"], "stale_auth_source", "{job}");
    assert_eq!(job["status"], "needs_reconciliation");
    assert!(job["archive_path"].is_string());
    let target = PathBuf::from(job["new_root"].as_str().unwrap());
    assert_eq!(
        fs::read_to_string(target.join("CLAUDE.md")).unwrap(),
        "Synthetic instructions retained"
    );
    assert_eq!(
        load(&root.join(".credentials.json")).unwrap()["claudeAiOauth"]["accessToken"],
        "SYNTHETIC_CONCURRENT_LOGIN"
    );
    assert_eq!(
        job["auth_verification"]["local_sources"][0]["state"],
        "still_present"
    );
    assert!(job["state_archive_path"].is_null());
    assert!(!root.join("logout-attempted").exists());
}

#[test]
fn cleanup_auth_binding_is_private_and_runtime_observation_is_separate() {
    let (_temp, engine, environment, root) = auth_fixture();
    write_login(&root, "SYNTHETIC_PRIVATE_LOGIN_BODY");
    let plan = data(
        &engine,
        json!({"command":"plan_cleanup","environment_id":environment["id"],"recipe":"repair_login","writers_confirmed_stopped":true,"verify_auth_status":true}),
    );
    let internal = load(&engine.path("plans", plan["id"].as_str().unwrap())).unwrap();
    assert!(internal["extra"]["authentication"]["groups"][0]["snapshot"]["digest"].is_string());
    assert!(internal["extra"]["authentication"]["nonce"].is_string());
    assert!(!root.join("probe-count").exists());
    let job = run(&engine, &plan);
    assert_eq!(job["status"], "partially_completed", "{job}");
    assert_eq!(job["local_cleanup"], "completed");
    assert_eq!(
        job["auth_verification"]["runtime"]["observation"]["auth_method"],
        "claude.ai"
    );
    assert_eq!(
        job["auth_verification"]["runtime"]["observation"]["logged_in"],
        true
    );
    for value in [&plan, &internal, &job] {
        let text = value.to_string();
        assert!(!text.contains("account-a@example.invalid"));
        assert!(!text.contains("SYNTHETIC_AUTH_STATUS_DO_NOT_PUBLISH"));
        assert!(!text.contains("SYNTHETIC_PRIVATE_LOGIN_BODY"));
        assert!(!text.contains("SYNTHETIC_MCP_KEEP"));
    }
    assert!(!plan.to_string().contains("snapshot"));
    assert!(!plan.to_string().contains("nonce"));
    assert!(!job.to_string().contains("nonce"));
    assert!(!root.join("logout-attempted").exists());
}

#[test]
fn cleanup_auth_changed_selected_generation_blocks_and_unselected_fallback_is_preserved() {
    for had_credentials in [false, true] {
        let (_temp, engine, environment, root) = auth_fixture();
        if had_credentials {
            write_login(&root, "SYNTHETIC_TOKEN_A");
        }
        let plan = clean_plan(&engine, &environment, "repair_login", false);
        write_login(&root, "SYNTHETIC_TOKEN_B_OR_FALLBACK");
        let new_bytes = fs::read(root.join(".credentials.json")).unwrap();
        let result = engine
            .request(json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"]}));
        if had_credentials {
            assert_eq!(result["error"]["code"], "stale_auth_source", "{result}");
            assert!(!engine.path("jobs", plan["id"].as_str().unwrap()).exists());
        } else {
            assert_eq!(result["ok"], true, "{result}");
            assert!(result["data"]["auth_verification"]["local_sources"]
                .as_array()
                .unwrap()
                .is_empty());
            assert!(result["data"]["auth_verification"]["inventory"]["sources"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s["kind"] == "credential_field" && s["presence"] == "present"));
        }
        assert_eq!(fs::read(root.join(".credentials.json")).unwrap(), new_bytes);
        assert!(!root.join("logout-attempted").exists());
    }
}

#[test]
fn cleanup_auth_account_metadata_never_makes_broad_logout_safe() {
    let (_temp, engine, environment, root) = auth_fixture();
    write_login(&root, "SYNTHETIC_LOGIN_KEEP_ON_REFUSAL");
    let original = fs::read(root.join(".credentials.json")).unwrap();
    let mut status = load(&root.join("synthetic-auth.json")).unwrap();
    for organization in ["synthetic-org", "different-synthetic-org"] {
        status["orgId"] = json!(organization);
        save(&root.join("synthetic-auth.json"), &status).unwrap();
        let result = engine.request(json!({"command":"plan_cleanup","environment_id":environment["id"],"recipe":"reset_client","writers_confirmed_stopped":true,"official_logout":true,"categories":["instructions"]}));
        assert_eq!(
            result["error"]["code"], "official_logout_scope_unsafe",
            "{result}"
        );
    }
    assert_eq!(fs::read(root.join(".credentials.json")).unwrap(), original);
    assert!(!root.join("probe-count").exists());
    assert_eq!(fs::read_dir(engine.state.join("plans")).unwrap().count(), 0);
    assert_eq!(fs::read_dir(engine.state.join("jobs")).unwrap().count(), 0);
    assert!(!root.join("logout-attempted").exists());
}

#[test]
fn cleanup_auth_unattempted_legacy_logout_requires_new_preview_and_finished_jobs_stay_query_only() {
    let (_temp, engine, environment, root) = auth_fixture();
    write_login(&root, "SYNTHETIC_OLD_PLAN_LOGIN");
    let plan = clean_plan(&engine, &environment, "repair_login", false);
    let path = engine.path("plans", plan["id"].as_str().unwrap());
    let mut internal = load(&path).unwrap();
    let binding = internal["extra"]
        .as_object_mut()
        .unwrap()
        .remove("authentication")
        .unwrap();
    internal.as_object_mut().unwrap().remove("hash");
    internal["hash"] = json!(digest(&serde_json::to_vec(&internal).unwrap()));
    save(&path, &internal).unwrap();
    let result = engine
        .request(json!({"command":"execute","plan_id":internal["id"],"approval":internal["hash"]}));
    assert_eq!(result["error"]["code"], "stale_auth_plan", "{result}");
    assert!(!root.join("logout-attempted").exists());
    assert!(!engine.path("jobs", plan["id"].as_str().unwrap()).exists());
    internal["extra"]["authentication"] = binding;
    internal.as_object_mut().unwrap().remove("hash");
    internal["hash"] = json!(digest(&serde_json::to_vec(&internal).unwrap()));
    save(&path, &internal).unwrap();
    let job = run(&engine, &plan);
    internal["extra"]
        .as_object_mut()
        .unwrap()
        .remove("authentication");
    internal["extra"]["official_logout"] = json!(true);
    internal.as_object_mut().unwrap().remove("hash");
    internal["hash"] = json!(digest(&serde_json::to_vec(&internal).unwrap()));
    save(&path, &internal).unwrap();
    set_auth_status(&root, Some("account-b@example.invalid"));
    write_login(&root, "SYNTHETIC_NEW_LOGIN_AFTER_ACCEPTED_JOB");
    let new_bytes = fs::read(root.join(".credentials.json")).unwrap();
    let repeated = run(&engine, &internal);
    assert_eq!(repeated["id"], job["id"]);
    assert_eq!(fs::read(root.join(".credentials.json")).unwrap(), new_bytes);
    assert!(
        !root.join("logout-attempted").exists(),
        "an accepted original job must never replay logout"
    );
}

#[test]
fn retirement_and_reactivation_do_not_delete_work() {
    let (_tmp, engine, e, root) = fixture();
    fs::write(root.join("CLAUDE.md"), "work").unwrap();
    let p = clean_plan(&engine, &e, "retire", false);
    run(&engine, &p);
    let r = engine.request(json!({"command":"launch_context","environment_id":e["id"]}));
    assert_eq!(r["error"]["code"], "environment_retired");
    data(
        &engine,
        json!({"command":"reactivate_environment","environment_id":e["id"]}),
    );
    assert_eq!(
        engine.env(&json!({"environment_id":e["id"]})).unwrap()["status"],
        "discovered"
    );
    assert!(root.join("CLAUDE.md").exists());
}

#[test]
fn auth_probe_uses_target_config_and_logout_refuses_before_service_or_profile_side_effects() {
    let (_tmp, mut engine, mut e, root) = fixture();
    engine.executable_search_path = Some("/usr/bin:/bin".into());
    let script = engine.home.join(".local/bin/claude");
    fs::create_dir_all(script.parent().unwrap()).unwrap();
    fs::write(&script,r##"#!/bin/sh
case "$2" in
status)
if test -f "$CLAUDE_CONFIG_DIR/.credentials.json"; then
printf '{"configDirectory":"%s","authMethod":"claude.ai","email":"synthetic@example.invalid","privateExtra":"never-return"}' "$CLAUDE_CONFIG_DIR"
exit 0
else
printf '{"configDirectory":"%s","authMethod":"none"}' "$CLAUDE_CONFIG_DIR"
exit 1
fi;;
logout) touch "$CLAUDE_CONFIG_DIR/unexpected-logout"; exit 0;;
esac
exit 9
"##).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    e["executable"] = json!(script);
    save(&engine.state.join("inventory.json"), &json!([e.clone()])).unwrap();
    write_login(&root, "SYNTHETIC_TARGETED_LOGIN");
    let a = data(
        &engine,
        json!({"command":"auth_probe","environment_id":e["id"]}),
    );
    assert!(!a.to_string().contains("example.invalid"));
    assert!(!a.to_string().contains("never-return"));
    assert_eq!(a["config_directory"], json!(root));
    assert_eq!(a["context_scope"], "lintel_runner_selected_root");
    let shared = engine.home.join(".config/anthropic");
    fs::create_dir_all(&shared).unwrap();
    // An unrelated malformed service receipt would make service enumeration
    // fail. Unsafe broad logout must reject before reading it or running Claude.
    let unreadable_service = engine.state.join("jobs/unrelated-service.json");
    fs::write(&unreadable_service, "not JSON").unwrap();
    let blocked = engine.request(json!({"command":"plan_cleanup","environment_id":e["id"],"recipe":"repair_login","writers_confirmed_stopped":true,"official_logout":true}));
    assert_eq!(blocked["error"]["code"], "official_logout_scope_unsafe");
    assert!(
        root.join(".credentials.json").exists(),
        "logout must not run after scope changes"
    );
    fs::remove_file(&unreadable_service).unwrap();
    let p = clean_plan(&engine, &e, "repair_login", false);
    let j = run(&engine, &p);
    assert_eq!(j["status"], "partially_completed");
    assert_eq!(j["local_cleanup"], "completed");
    assert_eq!(j["remote_revocation"], "unverified");
    assert!(load(&root.join(".credentials.json"))
        .unwrap()
        .get("claudeAiOauth")
        .is_none());
    assert!(shared.is_dir());
    assert!(!root.join("unexpected-logout").exists());
}

#[test]
fn archive_reopens_imports_and_refuses_existing_or_escaping_targets() {
    let (_tmp, engine, e, root) = fixture();
    fs::write(root.join("CLAUDE.md"), "Synthetic instructions").unwrap();
    let p = data(
        &engine,
        json!({"command":"plan_reset","environment_id":e["id"],"recipe":"rebuild","categories":["instructions"]}),
    );
    let j = run(&engine, &p);
    let a = data(
        &engine,
        json!({"command":"archive_inspect","job_id":j["id"],"archive_passphrase":PASS}),
    );
    assert_eq!(a["files"].as_array().unwrap().len(), 1);
    let content = data(
        &engine,
        json!({"command":"archive_read","job_id":j["id"],"path":"CLAUDE.md","archive_passphrase":PASS}),
    );
    assert_eq!(content["text"], "Synthetic instructions");
    let wrong=engine.request(json!({"command":"archive_inspect","job_id":j["id"],"archive_passphrase":"incorrect synthetic password"}));
    assert_eq!(wrong["error"]["code"], "archive_locked");
    let dest = engine.create("destination", None).unwrap();
    let import = data(
        &engine,
        json!({"command":"plan_import","environment_id":dest["id"],"job_id":j["id"],"categories":["instructions"],"archive_passphrase":PASS}),
    );
    let receipt = run(&engine, &import);
    assert_eq!(receipt["status"], "completed");
    let conflict=engine.request(json!({"command":"plan_import","environment_id":dest["id"],"job_id":j["id"],"categories":["instructions"],"archive_passphrase":PASS}));
    assert_eq!(conflict["error"]["code"], "import_conflict");
    let payload = json!({"schema":"lintel.work/1","files":[{"path":"../CLAUDE.md","category":"instructions","data":[65],"digest":digest(b"A")}]});
    fs::write(
        j["archive_path"].as_str().unwrap(),
        archive::seal(&payload, PASS).unwrap(),
    )
    .unwrap();
    let invalid = engine
        .request(json!({"command":"archive_inspect","job_id":j["id"],"archive_passphrase":PASS}));
    assert_eq!(invalid["error"]["code"], "archive_path");
}

#[test]
fn cleanup_does_not_unlink_a_replacement_in_the_delete_window() {
    let (_tmp, _engine, _e, root) = fixture();
    let path = root.join(".credentials.json");
    fs::write(&path, "old-login").unwrap();
    let frozen = storage::snapshot(&path).unwrap();
    let replacement = root.join("replacement");
    fs::write(&replacement, "new-login").unwrap();
    fs::rename(&replacement, &path).unwrap();
    let result = cleanup::quarantine_remove(&path, &frozen, &root.join(".test-quarantine"));
    assert!(result.is_err());
    assert_eq!(fs::read_to_string(path).unwrap(), "new-login");
}

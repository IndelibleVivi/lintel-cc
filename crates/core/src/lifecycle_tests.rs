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
fn cleanup_preserves_work_archives_state_and_never_replays_new_login() {
    let (_tmp, engine, e, root) = fixture();
    fs::write(root.join(".credentials.json"), "synthetic-login").unwrap();
    fs::write(
        root.join(".claude.json"),
        r#"{"mcpServers":{"example":{}},"syntheticSecret":"not-public"}"#,
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
    assert!(!root.join(".credentials.json").exists());
    assert!(!root.join(".claude.json").exists());
    assert!(root.join("CLAUDE.md").exists());
    assert!(root.join("settings.json").exists());
    let encrypted = fs::read(j["state_archive_path"].as_str().unwrap()).unwrap();
    assert!(!String::from_utf8_lossy(&encrypted).contains("not-public"));
    let state = archive::unseal(&encrypted, PASS).unwrap();
    assert_eq!(state["schema"], "lintel.state/1");
    fs::write(root.join(".credentials.json"), "new-login-must-survive").unwrap();
    assert_eq!(run(&engine, &p)["id"], j["id"]);
    assert_eq!(
        fs::read_to_string(root.join(".credentials.json")).unwrap(),
        "new-login-must-survive"
    );
}

#[test]
fn failed_state_backup_keeps_its_publication_path_before_cleanup() {
    let (_tmp, engine, e, root) = fixture();
    fs::write(root.join(".credentials.json"), "synthetic-login").unwrap();
    fs::write(root.join(".claude.json"), r#"{"synthetic":true}"#).unwrap();
    fs::write(root.join("CLAUDE.md"), "Keep this work").unwrap();
    let p = clean_plan(&engine, &e, "reset_client", false);
    let backup = engine
        .state
        .join("archives")
        .join(format!("{}-state.age", p["id"].as_str().unwrap()));
    // An existing directory makes atomic publication fail after its intent is
    // journaled, without racing a worker or changing a real client root.
    fs::create_dir(&backup).unwrap();
    let j = run(&engine, &p);
    assert_eq!(j["status"], "needs_reconciliation", "{j}");
    assert_eq!(j["state_archive_path"].as_str(), backup.to_str());
    assert_eq!(j["error"]["step_id"], "state_backup");
    assert_eq!(j["steps"][0]["status"], "executing");
    assert!(j["archive_path"].is_null() && j["new_root"].is_null());
    assert!(root.join(".credentials.json").exists());
    assert!(root.join(".claude.json").exists());
    assert_eq!(
        fs::read_to_string(root.join("CLAUDE.md")).unwrap(),
        "Keep this work"
    );
    let stored = load(&engine.path("jobs", p["id"].as_str().unwrap())).unwrap();
    assert_eq!(stored["state_archive_path"], j["state_archive_path"]);
    let queried = data(&engine, json!({"command":"job","job_id":j["id"]}));
    assert_eq!(queried["state_archive_path"], j["state_archive_path"]);
    assert!(
        backup.is_dir(),
        "query must not overwrite or remove the uncertain target"
    );
}

#[test]
fn changed_credentials_block_cleanup_before_acceptance() {
    let (_tmp, engine, e, root) = fixture();
    fs::write(root.join(".credentials.json"), "old").unwrap();
    let p = clean_plan(&engine, &e, "repair_login", false);
    fs::write(root.join(".credentials.json"), "new").unwrap();
    let r = engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}));
    assert_eq!(r["error"]["code"], "stale_plan");
    assert!(!engine.path("jobs", p["id"].as_str().unwrap()).exists());
    assert_eq!(
        fs::read_to_string(root.join(".credentials.json")).unwrap(),
        "new"
    );
}

fn auth_fixture() -> (tempfile::TempDir, Engine, Value, PathBuf) {
    let (temp, mut engine, mut environment, root) = fixture();
    engine.executable_search_path = Some("/usr/bin:/bin".into());
    let script = engine.home.join(".local/bin/claude");
    fs::create_dir_all(script.parent().unwrap()).unwrap();
    fs::write(&script, r##"#!/bin/sh
case "$2" in
status)
count=0
if test -f "$CLAUDE_CONFIG_DIR/probe-count"; then count=$(cat "$CLAUDE_CONFIG_DIR/probe-count"); fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDE_CONFIG_DIR/probe-count"
if test "$count" -ge 3 && test -f "$CLAUDE_CONFIG_DIR/after-preserve-auth.json"; then
cat "$CLAUDE_CONFIG_DIR/after-preserve-auth.json"
else
cat "$CLAUDE_CONFIG_DIR/synthetic-auth.json"
fi
if test -f "$CLAUDE_CONFIG_DIR/signed-out"; then exit 1; else exit 0; fi;;
logout)
touch "$CLAUDE_CONFIG_DIR/logout-attempted" "$CLAUDE_CONFIG_DIR/signed-out"
printf '{"configDirectory":"%s","authMethod":"none"}' "$CLAUDE_CONFIG_DIR" > "$CLAUDE_CONFIG_DIR/synthetic-auth.json"
exit 0;;
esac
exit 9
"##).unwrap();
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
fn cleanup_auth_account_switch_without_local_file_change_blocks_before_acceptance() {
    let (_temp, engine, environment, root) = auth_fixture();
    let plan = clean_plan(&engine, &environment, "repair_login", true);
    set_auth_status(&root, Some("account-b@example.invalid"));
    let result =
        engine.request(json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"]}));
    assert_eq!(result["error"]["code"], "stale_auth", "{result}");
    assert!(!engine.path("jobs", plan["id"].as_str().unwrap()).exists());
    assert!(!root.join("logout-attempted").exists());
}

#[test]
fn cleanup_auth_missing_subject_refuses_logout_preview_but_allows_local_cleanup() {
    let (_temp, engine, environment, root) = auth_fixture();
    set_auth_status(&root, None);
    let probe = data(
        &engine,
        json!({"command":"auth_probe","environment_id":environment["id"]}),
    );
    assert_eq!(probe["identity_observed"], false);
    let result = engine.request(json!({"command":"plan_cleanup","environment_id":environment["id"],"recipe":"repair_login","writers_confirmed_stopped":true,"official_logout":true}));
    assert_eq!(
        result["error"]["code"], "auth_identity_unverified",
        "{result}"
    );
    let local = clean_plan(&engine, &environment, "repair_login", false);
    run(&engine, &local);
    assert!(!root.join("logout-attempted").exists());
}

#[test]
fn cleanup_auth_changed_after_preservation_retains_work_without_logout() {
    let (_temp, engine, environment, root) = auth_fixture();
    fs::write(root.join("CLAUDE.md"), "Synthetic instructions retained").unwrap();
    fs::write(
        root.join(".credentials.json"),
        "Synthetic credential generation",
    )
    .unwrap();
    let plan = clean_plan(&engine, &environment, "reset_client", true);
    let changed = json!({"configDirectory":root,"authMethod":"claude.ai","email":"account-b@example.invalid","orgId":"synthetic-org","privateExtra":"SYNTHETIC_AUTH_STATUS_DO_NOT_PUBLISH"});
    save(&root.join("after-preserve-auth.json"), &changed).unwrap();
    let job = run(&engine, &plan);
    assert_eq!(job["error"]["code"], "stale_auth", "{job}");
    assert!(job["archive_path"].is_string());
    let target = PathBuf::from(job["new_root"].as_str().unwrap());
    assert_eq!(
        fs::read_to_string(target.join("CLAUDE.md")).unwrap(),
        "Synthetic instructions retained"
    );
    assert!(root.join(".credentials.json").exists());
    assert!(!root.join("logout-attempted").exists());
}

#[test]
fn cleanup_auth_binding_is_private_and_unchanged_subject_can_finish() {
    let (_temp, engine, environment, root) = auth_fixture();
    let plan = clean_plan(&engine, &environment, "repair_login", true);
    let internal = load(&engine.path("plans", plan["id"].as_str().unwrap())).unwrap();
    assert!(internal["extra"]["auth_binding"]["status_digest"].is_string());
    assert!(internal["extra"]["auth_binding"]["nonce"].is_string());
    let job = run(&engine, &plan);
    assert_eq!(job["status"], "completed", "{job}");
    for value in [&plan, &internal, &job] {
        let text = value.to_string();
        assert!(!text.contains("account-a@example.invalid"));
        assert!(!text.contains("SYNTHETIC_AUTH_STATUS_DO_NOT_PUBLISH"));
    }
    assert!(!plan.to_string().contains("status_digest"));
    assert!(!job.to_string().contains("status_digest"));
    assert!(root.join("logout-attempted").exists());
}

#[test]
fn cleanup_auth_same_account_new_file_generation_or_fallback_blocks() {
    for had_credentials in [false, true] {
        let (_temp, engine, environment, root) = auth_fixture();
        if had_credentials {
            fs::write(root.join(".credentials.json"), "Synthetic token A").unwrap();
        }
        let plan = clean_plan(&engine, &environment, "repair_login", true);
        fs::write(
            root.join(".credentials.json"),
            "Synthetic token B or Keychain fallback",
        )
        .unwrap();
        let result = engine
            .request(json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"]}));
        assert_eq!(result["error"]["code"], "stale_plan", "{result}");
        assert!(!root.join("logout-attempted").exists());
        assert!(!engine.path("jobs", plan["id"].as_str().unwrap()).exists());
    }
}

#[test]
fn cleanup_auth_organization_change_blocks_even_with_same_email() {
    let (_temp, engine, environment, root) = auth_fixture();
    let plan = clean_plan(&engine, &environment, "repair_login", true);
    let mut status = load(&root.join("synthetic-auth.json")).unwrap();
    status["orgId"] = json!("different-synthetic-org");
    save(&root.join("synthetic-auth.json"), &status).unwrap();
    let result =
        engine.request(json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"]}));
    assert_eq!(result["error"]["code"], "stale_auth", "{result}");
    assert!(!root.join("logout-attempted").exists());
}

#[test]
fn cleanup_auth_unattempted_legacy_logout_requires_new_preview_and_finished_jobs_stay_query_only() {
    let (_temp, engine, environment, root) = auth_fixture();
    let plan = clean_plan(&engine, &environment, "repair_login", true);
    let path = engine.path("plans", plan["id"].as_str().unwrap());
    let mut internal = load(&path).unwrap();
    let binding = internal["extra"]
        .as_object_mut()
        .unwrap()
        .remove("auth_binding")
        .unwrap();
    internal.as_object_mut().unwrap().remove("hash");
    internal["hash"] = json!(digest(&serde_json::to_vec(&internal).unwrap()));
    save(&path, &internal).unwrap();
    let result = engine
        .request(json!({"command":"execute","plan_id":internal["id"],"approval":internal["hash"]}));
    assert_eq!(result["error"]["code"], "stale_auth", "{result}");
    assert!(!root.join("logout-attempted").exists());
    internal["extra"]["auth_binding"] = binding;
    internal.as_object_mut().unwrap().remove("hash");
    internal["hash"] = json!(digest(&serde_json::to_vec(&internal).unwrap()));
    save(&path, &internal).unwrap();
    let job = run(&engine, &plan);
    internal["extra"]
        .as_object_mut()
        .unwrap()
        .remove("auth_binding");
    internal.as_object_mut().unwrap().remove("hash");
    internal["hash"] = json!(digest(&serde_json::to_vec(&internal).unwrap()));
    save(&path, &internal).unwrap();
    fs::remove_file(root.join("logout-attempted")).unwrap();
    set_auth_status(&root, Some("account-b@example.invalid"));
    fs::remove_file(root.join("signed-out")).unwrap();
    let repeated = run(&engine, &internal);
    assert_eq!(repeated["id"], job["id"]);
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
fn official_logout_uses_target_config_and_sanitizes_status() {
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
logout) rm "$CLAUDE_CONFIG_DIR/.credentials.json"; exit 0;;
esac
exit 9
"##).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    e["executable"] = json!(script);
    save(&engine.state.join("inventory.json"), &json!([e.clone()])).unwrap();
    fs::write(root.join(".credentials.json"), "synthetic credential").unwrap();
    let a = data(
        &engine,
        json!({"command":"auth_probe","environment_id":e["id"]}),
    );
    assert!(!a.to_string().contains("example.invalid"));
    assert!(!a.to_string().contains("never-return"));
    let p = clean_plan(&engine, &e, "repair_login", true);
    let shared = engine.home.join(".config/anthropic");
    fs::create_dir_all(&shared).unwrap();
    // An unrelated malformed service receipt would make service enumeration
    // fail. Known shared auth must reject before reading it or running Claude.
    let unreadable_service = engine.state.join("jobs/unrelated-service.json");
    fs::write(&unreadable_service, "not JSON").unwrap();
    let blocked =
        engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}));
    assert_eq!(blocked["error"]["code"], "shared_auth_scope");
    assert!(
        root.join(".credentials.json").exists(),
        "logout must not run after scope changes"
    );
    fs::remove_file(&unreadable_service).unwrap();
    fs::remove_dir(&shared).unwrap();
    let j = run(&engine, &p);
    assert_eq!(j["status"], "completed");
    assert_eq!(j["remote_revocation"], "unverified");
    assert!(!root.join(".credentials.json").exists());
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

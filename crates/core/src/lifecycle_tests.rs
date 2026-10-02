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
    let (_tmp, engine, mut e, root) = fixture();
    let script = engine.home.join("fake-claude");
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
    let blocked =
        engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}));
    assert_eq!(blocked["error"]["code"], "shared_auth_scope");
    assert!(
        root.join(".credentials.json").exists(),
        "logout must not run after scope changes"
    );
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
    let dest = engine.create("destination").unwrap();
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

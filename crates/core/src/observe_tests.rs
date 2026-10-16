//! End-to-end Engine observations over synthetic homes only.
use super::*;
use std::os::unix::fs::PermissionsExt;

fn fixture() -> (tempfile::TempDir, Engine, Value, PathBuf) {
    let temporary = tempfile::tempdir().unwrap();
    let base = temporary.path().canonicalize().unwrap();
    let home = base.join("home");
    let root = home.join("root");
    fs::create_dir_all(&root).unwrap();
    save(&root.join("settings.json"), &json!({"env":{"OTEL_LOG_USER_PROMPTS":"1","OTEL_EXPORTER_OTLP_HEADERS":"synthetic-private-header"}})).unwrap();
    save(&root.join(".claude.json"), &json!({"userID":"synthetic-identity-private","oauthAccount":{"email":"synthetic@example.invalid"}})).unwrap();
    let mut engine = Engine::new(home, base.join("state")).unwrap();
    engine.executable_search_path = Some(std::ffi::OsString::new());
    let environment = engine.register("synthetic", &root, false).unwrap();
    (temporary, engine, environment, root)
}

fn ask(engine: &Engine, command: &str, environment: &Value, fields: Value) -> Value {
    let mut request = fields;
    request["command"] = json!(command);
    request["environment_id"] = environment["id"].clone();
    let response = engine.request(request);
    assert_eq!(response["ok"], true, "{response}");
    response["data"].clone()
}

fn no_private_values(value: &Value) {
    let text = value.to_string();
    for private in [
        "synthetic-private-header",
        "synthetic-identity-private",
        "synthetic@example.invalid",
        "comparison_scope",
        "digests",
    ] {
        assert!(
            !text.contains(private),
            "private field {private} escaped: {text}"
        );
    }
}

#[test]
fn observations_and_saved_summaries_hide_identity_and_credentials() {
    let (_temporary, engine, environment, root) = fixture();
    let presence = ask(&engine, "identity_inspect", &environment, json!({}));
    assert_eq!(presence["fields"][0]["present"], true);
    no_private_values(&presence);
    let a = ask(&engine, "snapshot_save", &environment, json!({}));
    let salt = read(&engine.state.join("identity-salt"), 1024).unwrap();
    let b = ask(&engine, "snapshot_save", &environment, json!({}));
    assert_eq!(
        read(&engine.state.join("identity-salt"), 1024).unwrap(),
        salt
    );
    let compared = ask(
        &engine,
        "snapshot_compare",
        &environment,
        json!({"old_snapshot_id":a["snapshot"]["snapshot_id"],"new_snapshot_id":b["snapshot"]["snapshot_id"]}),
    );
    assert!(compared["changes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["area"] == "identity" && item["state"] == "same"));
    for response in [
        &a,
        &b,
        &compared,
        &ask(&engine, "snapshot_list", &environment, json!({})),
    ] {
        no_private_values(response);
    }
    let snapshot_path = engine
        .state
        .join("snapshots")
        .join(environment["id"].as_str().unwrap())
        .join(format!(
            "{}.json",
            a["snapshot"]["snapshot_id"].as_str().unwrap()
        ));
    let stored = load(&snapshot_path).unwrap();
    assert!(!stored.to_string().contains("synthetic-private-header"));
    assert!(!stored.to_string().contains("synthetic-identity-private"));
    assert_eq!(
        load(&root.join(".claude.json")).unwrap()["userID"],
        "synthetic-identity-private"
    );
    assert_eq!(
        fs::metadata(engine.state.join("identity-salt"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[test]
fn unreadable_settings_do_not_discard_independent_observations() {
    let (_temporary, engine, environment, root) = fixture();
    fs::write(root.join("settings.json"), b"{synthetic-malformed}").unwrap();
    let saved = ask(&engine, "snapshot_save", &environment, json!({}));
    assert_eq!(saved["snapshot"]["coverage"]["settings"], false);
    assert_eq!(saved["snapshot"]["coverage"]["identity"], true);
    assert_eq!(saved["identity"]["fields"][0]["present"], true);
    no_private_values(&saved);
    let listed = ask(&engine, "snapshot_list", &environment, json!({}));
    assert_eq!(listed["listed"], 1);
}

#[test]
fn pair_comparison_is_readonly_and_uses_one_ephemeral_salt() {
    let (_temporary, engine, environment, root) = fixture();
    let other = root.parent().unwrap().join("other-root");
    fs::create_dir(&other).unwrap();
    save(
        &other.join(".claude.json"),
        &json!({"userID":"synthetic-identity-private"}),
    )
    .unwrap();
    let second = engine.register("synthetic-second", &other, false).unwrap();
    let response = ask(
        &engine,
        "env_compare",
        &environment,
        json!({"other_environment_id":second["id"]}),
    );
    assert_eq!(response["comparison"]["identity"]["userID"], "same");
    assert!(!engine.state.join("identity-salt").exists());
    assert!(!engine.state.join("snapshots").exists());
    no_private_values(&response);
    save(
        &other.join(".claude.json"),
        &json!({"userID":"synthetic-other"}),
    )
    .unwrap();
    let response = ask(
        &engine,
        "env_compare",
        &environment,
        json!({"other_environment_id":second["id"]}),
    );
    assert_eq!(response["comparison"]["identity"]["userID"], "different");
}

#[test]
fn invalid_salt_is_never_replaced_or_interpreted_as_identity_change() {
    let (_temporary, engine, environment, _) = fixture();
    let path = engine.state.join("identity-salt");
    atomic_new(&path, b"synthetic-invalid-salt", 0o600).unwrap();
    let saved = ask(&engine, "snapshot_save", &environment, json!({}));
    assert_eq!(saved["snapshot"]["coverage"]["identity"], false);
    assert_eq!(saved["identity"]["state"], "unknown");
    assert_eq!(read(&path, 1024).unwrap(), b"synthetic-invalid-salt");
    assert!(saved["snapshot"]["coverage"]["errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["area"] == "identity" && item["code"] == "salt_unreadable"));
}

#[test]
fn damaged_snapshot_does_not_hide_valid_neighbors_and_ids_stay_bound() {
    let (_temporary, engine, environment, _) = fixture();
    let saved = ask(&engine, "snapshot_save", &environment, json!({}));
    let dir = engine
        .state
        .join("snapshots")
        .join(environment["id"].as_str().unwrap());
    fs::write(dir.join("damaged.json"), b"{synthetic-malformed}").unwrap();
    let listed = ask(&engine, "snapshot_list", &environment, json!({}));
    assert_eq!(listed["listed"], 1);
    assert_eq!(listed["errors"].as_array().unwrap().len(), 1);
    let old_id = saved["snapshot"]["snapshot_id"].as_str().unwrap();
    let fake_id = id();
    let original = load(&dir.join(format!("{old_id}.json"))).unwrap();
    save(&dir.join(format!("{fake_id}.json")), &original).unwrap();
    let response = engine.request(json!({"command":"snapshot_compare","environment_id":environment["id"],"old_snapshot_id":old_id,"new_snapshot_id":fake_id}));
    assert_eq!(response["ok"], false);
    assert_eq!(response["error"]["code"], "invalid_snapshot");
}

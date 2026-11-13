use super::*;
use std::os::unix::fs::PermissionsExt;

const TOKEN: &str = "SYNTHETIC_SELECTED_TOKEN_NEVER_PERSIST";
const OTHER: &str = "SYNTHETIC_MCP_TOKEN_KEEP_IN_ORIGINAL_ONLY";

fn fixture(default_root: bool) -> (tempfile::TempDir, Engine, Value, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let home = base.join("home");
    let root = home.join(if default_root {
        ".claude"
    } else {
        "custom-root"
    });
    fs::create_dir_all(&root).unwrap();
    let mut engine = Engine::new(home, base.join("state")).unwrap();
    engine.executable_search_path = Some(root.join("empty-path").into_os_string());
    let e = engine.register("synthetic auth", &root, false).unwrap();
    (temp, engine, e, root)
}

fn ok(engine: &Engine, r: Value) -> Value {
    let response = engine.request(r);
    assert_eq!(response["ok"], true, "{response}");
    response["data"].clone()
}

fn seed(root: &Path) {
    save(&root.join(".credentials.json"), &json!({"claudeAiOauth":{"accessToken":TOKEN,"refreshToken":"SYNTHETIC_REFRESH"},"mcpOAuth":{"server":{"accessToken":OTHER}},"pluginSecrets":{"keep":"SYNTHETIC_PLUGIN"}})).unwrap();
}

fn plan(engine: &Engine, e: &Value, ids: Option<Vec<Value>>) -> Value {
    let mut r = json!({"command":"plan_cleanup","environment_id":e["id"],"recipe":"repair_login","official_logout":false,"writers_confirmed_stopped":true});
    if let Some(ids) = ids {
        r["auth_source_ids"] = json!(ids);
    }
    ok(engine, r)
}

fn execute(engine: &Engine, p: &Value) -> Value {
    ok(
        engine,
        json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}),
    )
}

fn assert_no_secret_files(path: &Path) {
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            assert_no_secret_files(&path);
        } else {
            let bytes = fs::read(&path).unwrap();
            for secret in [TOKEN, OTHER, "SYNTHETIC_REFRESH", "SYNTHETIC_PLUGIN"] {
                assert!(
                    !String::from_utf8_lossy(&bytes).contains(secret),
                    "private material copied into {path:?}"
                );
            }
        }
    }
}

#[test]
fn one_approval_removes_exact_sources_and_preserves_work_mcp_and_unselected_configuration() {
    let (_temp, engine, e, root) = fixture(false);
    seed(&root);
    save(&root.join(".claude.json"), &json!({"primaryApiKey":TOKEN,"mcpServers":{"keep":{"command":"never-run"}},"oauthAccount":{"email":"synthetic@example.invalid"}})).unwrap();
    save(&root.join("settings.json"), &json!({"env":{"CLAUDE_CODE_OAUTH_TOKEN":TOKEN,"KEEP":"unchanged"},"apiKeyHelper":"SYNTHETIC_HELPER_NEVER_RUN","hooks":{"keep":"never-run"}})).unwrap();
    let shell = engine.home.join(".zshrc");
    fs::write(
        &shell,
        format!(
            "# synthetic\nexport ANTHROPIC_API_KEY='{TOKEN}'\nexport PATH=\"/usr/bin:$PATH\"\n"
        ),
    )
    .unwrap();
    fs::write(root.join("CLAUDE.md"), "instruction bytes stay").unwrap();
    fs::create_dir(root.join("projects")).unwrap();
    fs::write(root.join("projects/session.jsonl"), "session bytes stay").unwrap();
    let inventory = ok(
        &engine,
        json!({"command":"auth_inventory","environment_id":e["id"]}),
    );
    let ids = inventory["sources"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["removable"] == true)
        .map(|s| s["id"].clone())
        .collect();
    let p = plan(&engine, &e, Some(ids));
    assert!(!p["warnings"]
        .to_string()
        .contains("这里只检查已登记 root 的 user settings"));
    assert!(p["warnings"].to_string().contains("本地认证来源"));
    assert_eq!(
        p["authentication"]["selected_sources"]
            .as_array()
            .unwrap()
            .len(),
        5
    );
    assert!(
        root.join(".credentials.json").exists(),
        "preview must not mutate"
    );
    let internal = load(&engine.path("plans", p["id"].as_str().unwrap())).unwrap();
    for value in [&inventory, &p, &internal] {
        let serialized = value.to_string();
        assert!(!serialized.contains(TOKEN));
        assert!(!serialized.contains(OTHER));
        assert!(!serialized.contains("SYNTHETIC_HELPER_NEVER_RUN"));
    }
    assert!(!p.to_string().contains("nonce"));
    let j = execute(&engine, &p);
    assert_eq!(j["local_cleanup"], "completed", "{j}");
    assert!(j["auth_verification"]["local_sources"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["state"] == "removed"));
    assert_eq!(j["auth_verification"]["runtime"]["state"], "not_requested");
    assert_eq!(j["auth_verification"]["server_revocation"], "unverified");
    let credentials = load(&root.join(".credentials.json")).unwrap();
    assert!(credentials.get("claudeAiOauth").is_none());
    assert_eq!(credentials["mcpOAuth"]["server"]["accessToken"], OTHER);
    assert_eq!(credentials["pluginSecrets"]["keep"], "SYNTHETIC_PLUGIN");
    let state = load(&root.join(".claude.json")).unwrap();
    assert!(state.get("primaryApiKey").is_none());
    assert!(state["mcpServers"].is_object());
    let settings = load(&root.join("settings.json")).unwrap();
    assert!(settings["env"].get("CLAUDE_CODE_OAUTH_TOKEN").is_none());
    assert!(settings.get("apiKeyHelper").is_none());
    assert_eq!(settings["env"]["KEEP"], "unchanged");
    assert_eq!(
        fs::read_to_string(shell).unwrap(),
        "# synthetic\nexport PATH=\"/usr/bin:$PATH\"\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("CLAUDE.md")).unwrap(),
        "instruction bytes stay"
    );
    assert_eq!(
        fs::read_to_string(root.join("projects/session.jsonl")).unwrap(),
        "session bytes stay"
    );
    assert!(j["state_archive_path"].is_null());
    assert_no_secret_files(&engine.state);
    assert!(fs::read_dir(&root).unwrap().all(|p| !p
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".lintel-auth-edit")));
}

#[test]
fn default_and_custom_roots_select_the_correct_saved_api_key_without_touching_neighbors() {
    for default_root in [true, false] {
        let (_temp, engine, e, root) = fixture(default_root);
        seed(&root);
        let global = if default_root {
            engine.home.join(".claude.json")
        } else {
            root.join(".claude.json")
        };
        save(
            &global,
            &json!({"primaryApiKey":TOKEN,"mcpServers":{"keep":{}}}),
        )
        .unwrap();
        let neighbor = engine.home.join("neighbor");
        fs::create_dir(&neighbor).unwrap();
        seed(&neighbor);
        let before = fs::read(neighbor.join(".credentials.json")).unwrap();
        let p = plan(&engine, &e, None);
        assert_eq!(p["authentication"]["local_source_count"], 2);
        let j = execute(&engine, &p);
        assert_eq!(j["local_cleanup"], "completed", "{j}");
        assert!(load(&global).unwrap().get("primaryApiKey").is_none());
        assert_eq!(
            fs::read(neighbor.join(".credentials.json")).unwrap(),
            before
        );
    }
}

#[test]
fn a_changed_shared_source_rejects_before_acceptance_and_a_fresh_plan_can_follow() {
    let (_temp, engine, e, _root) = fixture(false);
    let shell = engine.home.join(".bashrc");
    fs::write(
        &shell,
        format!("export CLAUDE_CODE_OAUTH_TOKEN='{TOKEN}'\n"),
    )
    .unwrap();
    let inv = ok(
        &engine,
        json!({"command":"auth_inventory","environment_id":e["id"]}),
    );
    let id = inv["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["path"] == json!(shell) && s["removable"] == true)
        .unwrap()["id"]
        .clone();
    let p = plan(&engine, &e, Some(vec![id.clone()]));
    fs::write(
        &shell,
        format!("# new external edit\nexport CLAUDE_CODE_OAUTH_TOKEN='{TOKEN}'\n"),
    )
    .unwrap();
    let rejection =
        engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}));
    assert_eq!(rejection["error"]["code"], "stale_auth_source");
    assert!(!engine.path("jobs", p["id"].as_str().unwrap()).exists());
    let fresh = plan(&engine, &e, Some(vec![id]));
    assert_eq!(execute(&engine, &fresh)["local_cleanup"], "completed");
    assert_eq!(fs::read_to_string(shell).unwrap(), "# new external edit\n");
}

#[test]
fn unknown_store_and_explicit_empty_selection_are_preserved_and_never_claim_erasure() {
    let (_temp, engine, e, root) = fixture(false);
    fs::write(
        root.join(".credentials.json"),
        "opaque synthetic legacy store",
    )
    .unwrap();
    let p = plan(&engine, &e, None);
    assert_eq!(p["authentication"]["local_source_count"], 0);
    let j = execute(&engine, &p);
    assert_eq!(j["auth_verification"]["coverage"], "selected_sources_only");
    assert_eq!(
        fs::read_to_string(root.join(".credentials.json")).unwrap(),
        "opaque synthetic legacy store"
    );
    seed(&root);
    let empty = plan(&engine, &e, Some(vec![]));
    let before = fs::read(root.join(".credentials.json")).unwrap();
    let empty_job = execute(&engine, &empty);
    assert!(empty_job["auth_verification"]["local_sources"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(fs::read(root.join(".credentials.json")).unwrap(), before);
}

fn held_writer(path: &Path, directory: &Path, phase: &str) {
    if phase == "held" {
        fs::write(
            directory.join("original"),
            "external writer still holds original",
        )
        .unwrap();
        fs::write(path, "new external live file").unwrap();
    }
}

#[test]
fn concurrent_original_and_live_writers_are_both_preserved_with_query_only_recovery() {
    let (_temp, engine, e, root) = fixture(false);
    seed(&root);
    let p = plan(&engine, &e, None);
    auth_cleanup::set_edit_hook(Some(held_writer));
    let j = execute(&engine, &p);
    auth_cleanup::set_edit_hook(None);
    assert_eq!(j["status"], "needs_reconciliation", "{j}");
    assert_eq!(j["error"]["code"], "auth_source_conflict");
    assert_eq!(
        fs::read_to_string(root.join(".credentials.json")).unwrap(),
        "new external live file"
    );
    let retained = &j["auth_verification"]["retained_sensitive_material"];
    assert_eq!(retained.as_array().unwrap().len(), 1);
    assert_eq!(
        fs::read_to_string(retained[0]["path"].as_str().unwrap()).unwrap(),
        "external writer still holds original"
    );
    assert_eq!(execute(&engine, &p)["id"], j["id"]);
    let before = fs::read(engine.path("jobs", j["id"].as_str().unwrap())).unwrap();
    let fresh = ok(
        &engine,
        json!({"command":"auth_verify","environment_id":e["id"],"job_id":j["id"]}),
    );
    assert_eq!(fresh["retained_sensitive_material"], *retained);
    assert_eq!(
        fs::read(engine.path("jobs", j["id"].as_str().unwrap())).unwrap(),
        before,
        "verification must not rewrite history"
    );
}

fn publish_writer(path: &Path, _directory: &Path, phase: &str) {
    if phase == "published" {
        save(path,&json!({"claudeAiOauth":{"accessToken":"SYNTHETIC_NEW_LOGIN"},"mcpOAuth":{"keep":"new-owner-value"}})).unwrap();
    }
}

#[test]
fn a_new_login_during_readback_stops_and_is_reported_as_still_present() {
    let (_temp, engine, e, root) = fixture(false);
    seed(&root);
    let p = plan(&engine, &e, None);
    auth_cleanup::set_edit_hook(Some(publish_writer));
    let j = execute(&engine, &p);
    auth_cleanup::set_edit_hook(None);
    assert_eq!(j["status"], "needs_reconciliation");
    assert_eq!(
        j["auth_verification"]["local_sources"][0]["state"],
        "still_present"
    );
    assert_eq!(
        j["auth_verification"]["retained_sensitive_material"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        load(&root.join(".credentials.json")).unwrap()["claudeAiOauth"]["accessToken"],
        "SYNTHETIC_NEW_LOGIN"
    );
    execute(&engine, &p);
    assert_eq!(
        load(&root.join(".credentials.json")).unwrap()["claudeAiOauth"]["accessToken"],
        "SYNTHETIC_NEW_LOGIN"
    );
}

fn interrupt_after_hold(_path: &Path, _directory: &Path, phase: &str) {
    if phase == "held" {
        panic!("synthetic interruption after durable original locator");
    }
}

#[test]
fn interrupted_original_is_query_only_and_fresh_verification_exposes_retained_credentials() {
    let (_temp, engine, e, root) = fixture(false);
    seed(&root);
    let p = plan(&engine, &e, None);
    auth_cleanup::set_edit_hook(Some(interrupt_after_hold));
    let interrupted =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute(&engine, &p)));
    auth_cleanup::set_edit_hook(None);
    assert!(interrupted.is_err());
    let reopened = Engine::new(engine.home.clone(), engine.state.clone()).unwrap();
    let queried = ok(&reopened, json!({"command":"job","job_id":p["id"]}));
    assert_eq!(queried["status"], "needs_reconciliation");
    assert!(!root.join(".credentials.json").exists());
    let fresh = ok(
        &reopened,
        json!({"command":"auth_verify","environment_id":e["id"],"job_id":p["id"]}),
    );
    assert_eq!(
        fresh["retained_sensitive_material"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(fs::read_to_string(
        fresh["retained_sensitive_material"][0]["path"]
            .as_str()
            .unwrap()
    )
    .unwrap()
    .contains(TOKEN));
    assert_eq!(execute(&reopened, &p)["id"], p["id"]);
    assert!(
        !root.join(".credentials.json").exists(),
        "original task must not replay or restore a login"
    );
}

#[test]
fn retained_other_authentication_is_visible_after_exact_local_cleanup_and_runtime_is_opt_in() {
    let (_temp, mut engine, mut e, root) = fixture(false);
    seed(&root);
    let exe = engine.home.join(".local/bin/claude");
    fs::create_dir_all(exe.parent().unwrap()).unwrap();
    fs::write(&exe,"#!/bin/sh\n[ \"$1 $2\" = 'auth status' ] || exit 9\nprintf '{\"configDirectory\":\"%s\",\"authMethod\":\"oauth_token\",\"email\":\"DO_NOT_PUBLISH\",\"token\":\"SYNTHETIC_RUNTIME_SECRET\"}' \"$CLAUDE_CONFIG_DIR\"\n").unwrap();
    fs::set_permissions(&exe, fs::Permissions::from_mode(0o700)).unwrap();
    engine.executable_search_path = Some("/usr/bin:/bin".into());
    e["executable"] = json!(exe);
    save(&engine.state.join("inventory.json"), &json!([e.clone()])).unwrap();
    let p = ok(
        &engine,
        json!({"command":"plan_cleanup","environment_id":e["id"],"recipe":"repair_login","official_logout":false,"writers_confirmed_stopped":true,"verify_auth_status":true}),
    );
    let j = execute(&engine, &p);
    assert_eq!(j["local_cleanup"], "completed", "{j}");
    assert_eq!(j["auth_verification"]["runtime"]["state"], "observed");
    assert_eq!(
        j["auth_verification"]["runtime"]["observation"]["logged_in"],
        true
    );
    assert_eq!(
        j["auth_verification"]["runtime"]["observation"]["auth_method"],
        "oauth_token"
    );
    assert!(!j.to_string().contains("DO_NOT_PUBLISH"));
    assert!(!j.to_string().contains("SYNTHETIC_RUNTIME_SECRET"));
    let fresh = ok(
        &engine,
        json!({"command":"auth_verify","environment_id":e["id"],"job_id":j["id"]}),
    );
    assert_eq!(fresh["runtime"]["state"], "not_requested");
}

#[test]
fn selected_source_missing_from_a_limited_scan_is_not_false_absence() {
    let (_temp, engine, e, root) = fixture(false);
    seed(&root);
    let p = plan(&engine, &e, None);
    let j = execute(&engine, &p);
    fs::File::create(root.join(".credentials.json"))
        .unwrap()
        .set_len(2 * 1024 * 1024)
        .unwrap();
    let v = ok(
        &engine,
        json!({"command":"auth_verify","environment_id":e["id"],"job_id":j["id"]}),
    );
    assert_eq!(v["local_sources"][0]["state"], "unknown");
    assert_eq!(
        j["auth_verification"]["local_sources"][0]["state"], "removed",
        "old receipt remains historical evidence"
    );
}

#[test]
fn verification_cannot_borrow_another_environment_or_an_unapproved_selection() {
    let (_temp, engine, e, root) = fixture(false);
    seed(&root);
    let p = plan(&engine, &e, None);
    let j = execute(&engine, &p);
    let other = engine.home.join("other");
    fs::create_dir(&other).unwrap();
    let neighbor = engine.register("neighbor", &other, false).unwrap();
    let result = engine
        .request(json!({"command":"auth_verify","environment_id":neighbor["id"],"job_id":j["id"]}));
    assert_eq!(result["error"]["code"], "auth_verification_scope");
    let invalid=engine.request(json!({"command":"plan_cleanup","environment_id":e["id"],"recipe":"repair_login","official_logout":false,"writers_confirmed_stopped":true,"auth_source_ids":["not-a-source"]}));
    assert_eq!(invalid["error"]["code"], "auth_source_not_removable");
}

fn occupy_original(_path: &Path, directory: &Path, phase: &str) {
    if phase == "prepared" {
        fs::write(directory.join("original"), "concurrent owner bytes").unwrap();
    }
}

#[test]
fn an_occupied_original_name_is_never_overwritten() {
    let (_temp, engine, e, root) = fixture(false);
    seed(&root);
    let p = plan(&engine, &e, None);
    let before = fs::read(root.join(".credentials.json")).unwrap();
    auth_cleanup::set_edit_hook(Some(occupy_original));
    let j = execute(&engine, &p);
    auth_cleanup::set_edit_hook(None);
    assert_eq!(j["status"], "needs_reconciliation");
    assert_eq!(fs::read(root.join(".credentials.json")).unwrap(), before);
    let retained = &j["auth_verification"]["retained_sensitive_material"];
    assert_eq!(retained.as_array().unwrap().len(), 1);
    assert_eq!(
        fs::read_to_string(retained[0]["path"].as_str().unwrap()).unwrap(),
        "concurrent owner bytes"
    );
}

fn replace_directory(_path: &Path, directory: &Path, phase: &str) {
    if phase == "prepared" {
        fs::rename(directory, directory.with_extension("outside-owner")).unwrap();
        fs::create_dir(directory).unwrap();
        fs::write(
            directory.join("replacement"),
            "new directory owner must survive Drop",
        )
        .unwrap();
    }
}

#[test]
fn replaced_quarantine_directory_and_staging_are_not_owned_by_pathname() {
    let (_temp, engine, e, root) = fixture(false);
    seed(&root);
    let p = plan(&engine, &e, None);
    let before = fs::read(root.join(".credentials.json")).unwrap();
    auth_cleanup::set_edit_hook(Some(replace_directory));
    let j = execute(&engine, &p);
    auth_cleanup::set_edit_hook(None);
    assert_eq!(j["status"], "needs_reconciliation");
    assert_eq!(j["error"]["code"], "auth_quarantine_changed");
    assert_eq!(fs::read(root.join(".credentials.json")).unwrap(), before);
    let retained = &j["auth_verification"]["retained_sensitive_material"][0];
    assert_eq!(retained["state"], "location_unknown");
    assert_eq!(
        fs::read_to_string(Path::new(retained["path"].as_str().unwrap()).join("replacement"))
            .unwrap(),
        "new directory owner must survive Drop"
    );
}

fn move_held_directory(_path: &Path, directory: &Path, phase: &str) {
    if phase == "held" {
        fs::rename(directory, directory.with_extension("outside-owner")).unwrap();
    }
}

#[test]
fn a_moved_held_directory_is_unknown_material_not_proof_of_erasure() {
    let (_temp, engine, e, root) = fixture(false);
    seed(&root);
    let p = plan(&engine, &e, None);
    auth_cleanup::set_edit_hook(Some(move_held_directory));
    let j = execute(&engine, &p);
    auth_cleanup::set_edit_hook(None);
    assert_eq!(j["status"], "needs_reconciliation");
    assert_eq!(j["local_cleanup"], "unverified");
    let retained = &j["auth_verification"]["retained_sensitive_material"][0];
    assert_eq!(retained["state"], "location_unknown");
    let moved = Path::new(retained["path"].as_str().unwrap()).with_extension("outside-owner");
    assert!(fs::read_to_string(moved.join("original"))
        .unwrap()
        .contains(TOKEN));
    assert!(!root.join(".credentials.json").exists());
    let fresh = ok(
        &engine,
        json!({"command":"auth_verify","environment_id":e["id"],"job_id":j["id"]}),
    );
    assert_eq!(
        fresh["retained_sensitive_material"][0]["state"],
        "location_unknown"
    );
    assert!(
        moved.join("original").exists(),
        "verification must not search for or mutate moved material"
    );
}

#[test]
fn changed_parent_cannot_turn_missing_paths_into_verified_source_removal() {
    let (_temp, engine, e, root) = fixture(false);
    seed(&root);
    let p = plan(&engine, &e, None);
    let j = execute(&engine, &p);
    fs::rename(&root, root.with_extension("moved")).unwrap();
    fs::create_dir(&root).unwrap();
    let fresh = ok(
        &engine,
        json!({"command":"auth_verify","environment_id":e["id"],"job_id":j["id"]}),
    );
    assert_eq!(fresh["local_sources"][0]["state"], "unknown");
}

fn leave_internal_temporary(_path: &Path, directory: &Path, phase: &str) {
    if phase == "held" {
        fs::remove_file(directory.join("original")).unwrap();
        fs::write(directory.join(".internal-atomic-staging"), OTHER).unwrap();
        panic!("synthetic interruption with retained internal temporary");
    }
}

#[test]
fn incomplete_owned_directory_keeps_internal_temporaries_in_coverage() {
    let (_temp, engine, e, root) = fixture(false);
    seed(&root);
    let p = plan(&engine, &e, None);
    auth_cleanup::set_edit_hook(Some(leave_internal_temporary));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute(&engine, &p)));
    auth_cleanup::set_edit_hook(None);
    assert!(result.is_err());
    let fresh = ok(
        &engine,
        json!({"command":"auth_verify","environment_id":e["id"],"job_id":p["id"]}),
    );
    let retained = &fresh["retained_sensitive_material"][0];
    assert_eq!(retained["state"], "retained_container");
    assert!(Path::new(retained["path"].as_str().unwrap())
        .join(".internal-atomic-staging")
        .exists());
    assert!(!fresh.to_string().contains(OTHER));
}

#[test]
fn credentials_recreated_by_status_are_observed_by_the_final_readback() {
    let (_temp, mut engine, mut e, root) = fixture(false);
    seed(&root);
    let exe = engine.home.join(".local/bin/claude");
    fs::create_dir_all(exe.parent().unwrap()).unwrap();
    fs::write(&exe, "#!/bin/sh\n[ \"$1 $2\" = 'auth status' ] || exit 9\nprintf '%s' '{\"claudeAiOauth\":{\"accessToken\":\"SYNTHETIC_REFRESH_DURING_STATUS\"}}' > \"$CLAUDE_CONFIG_DIR/.credentials.json\"\nprintf '{\"configDirectory\":\"%s\",\"authMethod\":\"oauth_token\",\"loggedIn\":true}' \"$CLAUDE_CONFIG_DIR\"\n").unwrap();
    fs::set_permissions(&exe, fs::Permissions::from_mode(0o700)).unwrap();
    engine.executable_search_path = Some("/usr/bin:/bin".into());
    e["executable"] = json!(exe);
    save(&engine.state.join("inventory.json"), &json!([e.clone()])).unwrap();
    let p = ok(
        &engine,
        json!({"command":"plan_cleanup","environment_id":e["id"],"recipe":"repair_login","official_logout":false,"writers_confirmed_stopped":true,"verify_auth_status":true}),
    );
    let j = execute(&engine, &p);
    assert_eq!(j["auth_verification"]["runtime"]["state"], "observed");
    assert_eq!(
        j["auth_verification"]["local_sources"][0]["state"],
        "still_present"
    );
    assert_eq!(j["local_cleanup"], "unverified");
    assert_eq!(j["status"], "needs_reconciliation");
    assert_eq!(j["error"]["code"], "auth_cleanup_unverified");
    assert!(!j.to_string().contains("SYNTHETIC_REFRESH_DURING_STATUS"));
}

fn install_marked_status_client(engine: &Engine) -> PathBuf {
    let exe = engine.home.join(".local/bin/claude");
    fs::create_dir_all(exe.parent().unwrap()).unwrap();
    fs::write(&exe, "#!/bin/sh\n[ \"$1 $2\" = 'auth status' ] || exit 9\nprintf ran > \"$CLAUDE_CONFIG_DIR/status-executed\"\nprintf '{\"configDirectory\":\"%s\",\"authMethod\":\"oauth_token\",\"loggedIn\":true}' \"$CLAUDE_CONFIG_DIR\"\n").unwrap();
    fs::set_permissions(&exe, fs::Permissions::from_mode(0o700)).unwrap();
    exe
}

#[test]
fn auth_runtime_missing_executable_cannot_follow_discovery_before_acceptance() {
    let (_temp, engine, e, root) = fixture(false);
    seed(&root);
    assert!(e["executable"].is_null());
    let before = fs::read(root.join(".credentials.json")).unwrap();
    let p = ok(
        &engine,
        json!({"command":"plan_cleanup","environment_id":e["id"],"recipe":"repair_login","official_logout":false,"writers_confirmed_stopped":true,"verify_auth_status":true}),
    );
    let exe = install_marked_status_client(&engine);
    let found = ok(&engine, json!({"command":"discover"}));
    assert!(found["environments"]
        .as_array()
        .unwrap()
        .iter()
        .any(|current| current["id"] == e["id"] && current["executable"] == json!(exe)));
    let response =
        engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}));
    assert_eq!(
        response["error"]["code"], "executable_changed",
        "{response}"
    );
    assert!(!engine.path("jobs", p["id"].as_str().unwrap()).exists());
    assert_eq!(fs::read(root.join(".credentials.json")).unwrap(), before);
    assert!(!root.join("status-executed").exists());
}

#[test]
fn auth_runtime_missing_executable_cannot_follow_discovery_when_rechecking_original() {
    let (_temp, engine, e, root) = fixture(false);
    seed(&root);
    let p = ok(
        &engine,
        json!({"command":"plan_cleanup","environment_id":e["id"],"recipe":"repair_login","official_logout":false,"writers_confirmed_stopped":true,"verify_auth_status":true}),
    );
    let j = execute(&engine, &p);
    assert_eq!(j["local_cleanup"], "completed");
    assert_eq!(
        j["auth_verification"]["runtime"]["error_code"],
        "executable_missing"
    );
    let journal = engine.path("jobs", j["id"].as_str().unwrap());
    let before = fs::read(&journal).unwrap();
    install_marked_status_client(&engine);
    ok(&engine, json!({"command":"discover"}));
    let fresh = ok(
        &engine,
        json!({"command":"auth_verify","environment_id":e["id"],"job_id":j["id"],"observe_runtime":true}),
    );
    assert_eq!(
        fresh["runtime"]["error_code"], "executable_changed",
        "{fresh}"
    );
    assert!(!root.join("status-executed").exists());
    assert_eq!(fs::read(&journal).unwrap(), before);
    // A new explicit observation has no old approval binding and may use the
    // newly discovered client; retaining an original must not disable that path.
    let current = ok(
        &engine,
        json!({"command":"auth_verify","environment_id":e["id"],"observe_runtime":true}),
    );
    assert_eq!(current["runtime"]["state"], "observed");
    assert!(root.join("status-executed").exists());
}

fn seed_cleanup_keychain(engine: &Engine, root: &Path) -> String {
    let service = crate::keychain::services(&engine.home, root)
        .last()
        .unwrap()
        .clone();
    save(&engine.home.join(".lintel-test-keychain.json"), &json!({"schema":"lintel.test-keychain/1","items":{&service:{"id":"synthetic-cleanup-item","data":{"claudeAiOauth":{"accessToken":TOKEN},"mcpOAuth":{"server":OTHER},"pluginSecrets":{"keep":"SYNTHETIC_PLUGIN"},"future":{"keep":[1,2]}}}}})).unwrap();
    service
}

#[test]
fn one_approval_cleans_file_and_keychain_preserving_all_container_neighbors() {
    for default in [true, false] {
        let (_temp, engine, e, root) = fixture(default);
        seed(&root);
        let service = seed_cleanup_keychain(&engine, &root);
        let p = plan(&engine, &e, None);
        assert_eq!(
            p["authentication"]["selected_sources"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert!(!p.to_string().contains("synthetic-cleanup-item"));
        let job = execute(&engine, &p);
        assert_eq!(job["local_cleanup"], "completed", "{job}");
        let data = load(&engine.home.join(".lintel-test-keychain.json")).unwrap()["items"]
            [&service]["data"]
            .clone();
        assert_eq!(
            data,
            json!({"mcpOAuth":{"server":OTHER},"pluginSecrets":{"keep":"SYNTHETIC_PLUGIN"},"future":{"keep":[1,2]}})
        );
        assert!(load(&root.join(".credentials.json"))
            .unwrap()
            .get("claudeAiOauth")
            .is_none());
        assert_no_secret_files(&engine.state);
    }
}

#[test]
fn keychain_update_uncertainty_keeps_original_job_without_replay() {
    let (_temp, engine, e, root) = fixture(false);
    let service = seed_cleanup_keychain(&engine, &root);
    let path = engine.home.join(".lintel-test-keychain.json");
    let mut data = load(&path).unwrap();
    data["after_write_error"] = json!("keychain_write_uncertain");
    save(&path, &data).unwrap();
    let p = plan(&engine, &e, None);
    let job = execute(&engine, &p);
    assert_eq!(job["local_cleanup"], "unverified", "{job}");
    assert_eq!(job["error"]["code"], "keychain_write_uncertain", "{job}");
    let mut after = load(&path).unwrap();
    assert!(after["items"][&service]["data"]
        .get("claudeAiOauth")
        .is_none());
    after.as_object_mut().unwrap().remove("after_write_error");
    after["items"][&service]["data"]["claudeAiOauth"] =
        json!({"accessToken":"recreated-after-uncertainty"});
    save(&path, &after).unwrap();
    let replay = execute(&engine, &p);
    assert_eq!(replay["id"], job["id"]);
    assert_eq!(load(&path).unwrap(), after);
    assert_no_secret_files(&engine.state);
}

#[test]
fn keychain_status_recreation_is_seen_by_independent_job_readback() {
    let (_temp, mut engine, mut e, root) = fixture(false);
    let service = seed_cleanup_keychain(&engine, &root);
    let fixture_path = engine.home.join(".lintel-test-keychain.json");
    let mut recreated = load(&fixture_path).unwrap();
    recreated["items"][&service]["data"]["claudeAiOauth"] =
        json!({"accessToken":"SYNTHETIC_REFRESH_DURING_STATUS"});
    let recreation_source = root.join("status-recreates-keychain.json");
    save(&recreation_source, &recreated).unwrap();
    let exe = engine.home.join(".local/bin/claude");
    fs::create_dir_all(exe.parent().unwrap()).unwrap();
    fs::write(&exe, format!("#!/bin/sh\n[ \"$1 $2\" = 'auth status' ] || exit 9\n/bin/cp '{}' '{}'\nprintf '{{\"configDirectory\":\"%s\",\"authMethod\":\"oauth_token\",\"loggedIn\":true}}' \"$CLAUDE_CONFIG_DIR\"\n", recreation_source.display(), fixture_path.display())).unwrap();
    fs::set_permissions(&exe, fs::Permissions::from_mode(0o700)).unwrap();
    engine.executable_search_path = Some("/usr/bin:/bin".into());
    e["executable"] = json!(exe);
    save(&engine.state.join("inventory.json"), &json!([e.clone()])).unwrap();
    let p = ok(
        &engine,
        json!({"command":"plan_cleanup","environment_id":e["id"],"recipe":"repair_login","writers_confirmed_stopped":true,"verify_auth_status":true}),
    );
    let job = execute(&engine, &p);
    assert_eq!(job["auth_verification"]["runtime"]["state"], "observed");
    assert_eq!(
        job["auth_verification"]["local_sources"][0]["state"],
        "still_present"
    );
    assert_eq!(job["local_cleanup"], "unverified");
    assert_eq!(job["error"]["code"], "auth_cleanup_unverified");
    let fresh = ok(
        &engine,
        json!({"command":"auth_verify","environment_id":e["id"],"job_id":job["id"]}),
    );
    assert_eq!(fresh["local_sources"][0]["state"], "still_present");
    assert!(!fresh
        .to_string()
        .contains("SYNTHETIC_REFRESH_DURING_STATUS"));
    assert_no_secret_files(&engine.state);
}

#[test]
fn stale_keychain_neighbor_rejects_approval_before_any_selected_file_edit() {
    let (_temp, engine, e, root) = fixture(false);
    seed(&root);
    let original_file = fs::read(root.join(".credentials.json")).unwrap();
    let service = seed_cleanup_keychain(&engine, &root);
    let p = plan(&engine, &e, None);
    let path = engine.home.join(".lintel-test-keychain.json");
    let mut external = load(&path).unwrap();
    external["items"][&service]["data"]["mcpOAuth"]["server"] = json!("outside-edit");
    save(&path, &external).unwrap();
    let response =
        engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}));
    assert_eq!(response["ok"], false, "{response}");
    assert_eq!(response["error"]["code"], "stale_auth_source", "{response}");
    assert_eq!(
        fs::read(root.join(".credentials.json")).unwrap(),
        original_file
    );
    assert_eq!(load(&path).unwrap(), external);
    assert!(!engine.path("jobs", p["id"].as_str().unwrap()).exists());
}

use super::*;

const PASS: &str = "synthetic-config-passphrase";
fn fixture() -> (tempfile::TempDir, Engine, Value, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let home = base.join("home");
    let root = home.join("root");
    fs::create_dir_all(&root).unwrap();
    save(&root.join("settings.json"),&json!({"env":{"OTEL_LOG_USER_PROMPTS":"1","UNRELATED":"preserve"},"apiKeyHelper":"synthetic-config-private-old","model":"original"})).unwrap();
    let mut engine = Engine::new(home, base.join("state")).unwrap();
    engine.executable_search_path = Some(std::ffi::OsString::new());
    let e = engine.register("synthetic", &root, false).unwrap();
    (temp, engine, e, root)
}
fn ask(engine: &Engine, command: &str, e: &Value, mut fields: Value) -> Value {
    fields["command"] = json!(command);
    fields["environment_id"] = e["id"].clone();
    let r = engine.request(fields);
    assert_eq!(r["ok"], true, "{r}");
    r["data"].clone()
}
fn edit(engine: &Engine, e: &Value, path: &Path, doc: Value) -> Value {
    let read = ask(engine, "config_read", e, json!({"config_path":path}));
    ask(
        engine,
        "plan_config_edit",
        e,
        json!({"config_path":path,"document":doc.to_string(),"expected_digest":read["digest"]}),
    )
}
fn execute(engine: &Engine, plan: &Value) -> Value {
    let response=engine.request(json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"],"archive_passphrase":PASS}));
    assert_eq!(response["ok"], true, "{response}");
    assert_eq!(response["data"]["status"], "completed", "{response}");
    response["data"].clone()
}
fn private(value: &Value) {
    for needle in [
        "synthetic-config-private-old",
        "synthetic-config-private-new",
    ] {
        assert!(
            !value.to_string().contains(needle),
            "private config escaped: {value}"
        );
    }
}

#[test]
fn masked_policy_changes_use_encrypted_config_preservation_and_restore() {
    let (_temp, engine, e, root) = fixture();
    let path = root.join("settings.json");
    let mut original = load(&path).unwrap();
    original["env"]["OTEL_EXPORTER_OTLP_HEADERS"] = json!("synthetic-config-private-old");
    original["env"]["OTEL_EXPORTER_OTLP_ENDPOINT"] =
        json!("https://synthetic-config-private-old@collector.invalid");
    save(&path, &original).unwrap();
    let plan = ask(
        &engine,
        "plan_policy",
        &e,
        json!({"preset":"custom","keep_remote_control":false,
        "custom_settings":{"OTEL_EXPORTER_OTLP_HEADERS":"remove","OTEL_EXPORTER_OTLP_ENDPOINT":"set"},
        "custom_values":{"OTEL_EXPORTER_OTLP_ENDPOINT":"https://collector.invalid"}}),
    );
    assert_eq!(plan["kind"], "config_edit");
    assert_eq!(plan["archive_passphrase_required"], true);
    private(&plan);
    let refused =
        engine.request(json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"]}));
    assert_eq!(refused["ok"], false);
    assert_eq!(load(&path).unwrap(), original);
    assert!(!engine.path("jobs", plan["id"].as_str().unwrap()).exists());
    let job = execute(&engine, &plan);
    private(&job);
    assert!(Path::new(job["config_archive_path"].as_str().unwrap()).exists());
    let mut current = load(&path).unwrap();
    assert!(current["env"].get("OTEL_EXPORTER_OTLP_HEADERS").is_none());
    current["unrelated_later"] = json!("retain");
    save(&path, &current).unwrap();
    let restore = ask(
        &engine,
        "plan_config_restore",
        &e,
        json!({"job_id":job["id"],"selected_keys":["/env/OTEL_EXPORTER_OTLP_HEADERS"]}),
    );
    private(&restore);
    execute(&engine, &restore);
    let restored = load(&path).unwrap();
    assert_eq!(
        restored["env"]["OTEL_EXPORTER_OTLP_HEADERS"],
        original["env"]["OTEL_EXPORTER_OTLP_HEADERS"]
    );
    assert_eq!(
        restored["env"]["OTEL_EXPORTER_OTLP_ENDPOINT"],
        "https://collector.invalid"
    );
    assert_eq!(restored["unrelated_later"], "retain");
}

#[test]
fn private_edit_backup_selected_restore_and_replay() {
    let (_temp, engine, e, root) = fixture();
    let path = root.join("settings.json");
    let mut next = load(&path).unwrap();
    next["env"]["OTEL_LOG_USER_PROMPTS"] = json!("0");
    next["apiKeyHelper"] = json!("synthetic-config-private-new");
    let plan = edit(&engine, &e, &path, next.clone());
    assert_eq!(plan["configuration"]["path"], json!(path));
    private(&plan);
    let job = execute(&engine, &plan);
    private(&job);
    assert_eq!(load(&path).unwrap(), next);
    let backup = archive::unseal(
        &read(
            Path::new(job["config_archive_path"].as_str().unwrap()),
            STATE_JSON_LIMIT,
        )
        .unwrap(),
        PASS,
    )
    .unwrap();
    assert_eq!(backup["schema"], "lintel.config/1");
    assert_eq!(
        backup["document"]["apiKeyHelper"],
        "synthetic-config-private-old"
    );
    next["later"] = json!({"unrelated":"preserve"});
    next["apiKeyHelper"] = json!("external-later-value");
    save(&path, &next).unwrap();
    let observed = ask(
        &engine,
        "config_restore_inspect",
        &e,
        json!({"job_id":job["id"]}),
    );
    private(&observed);
    assert!(observed["fields"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field["key"] == "/apiKeyHelper" && field["available"] == false));
    let restore = ask(
        &engine,
        "plan_config_restore",
        &e,
        json!({"job_id":job["id"],"selected_keys":["/env/OTEL_LOG_USER_PROMPTS"]}),
    );
    execute(&engine, &restore);
    next["env"]["OTEL_LOG_USER_PROMPTS"] = json!("1");
    assert_eq!(load(&path).unwrap(), next);
    let before = read(&path, CONFIG_BYTES).unwrap();
    let repeat=engine.request(json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"],"archive_passphrase":PASS}));
    assert_eq!(repeat["data"]["id"], job["id"]);
    assert_eq!(read(&path, CONFIG_BYTES).unwrap(), before);
    let conflict=engine.request(json!({"command":"plan_config_restore","environment_id":e["id"],"job_id":job["id"],"selected_keys":["/apiKeyHelper"]}));
    assert_eq!(conflict["error"]["code"], "restore_conflict");
}

#[test]
fn source_conflicts_managed_readonly_and_project_content_limits() {
    let (_temp, engine, e, root) = fixture();
    let project = root.parent().unwrap().join("project");
    fs::create_dir_all(project.join(".claude")).unwrap();
    let local = project.join(".claude/settings.local.json");
    save(&local, &json!({"env":{"OTEL_LOG_USER_PROMPTS":"0"}})).unwrap();
    let managed = engine.home.join("synthetic-managed");
    fs::create_dir(&managed).unwrap();
    save(
        &managed.join("managed-settings.json"),
        &json!({"env":{"OTEL_LOG_USER_PROMPTS":"1"}}),
    )
    .unwrap();
    let sources = ask(
        &engine,
        "config_inspect",
        &e,
        json!({"project_cwd":project}),
    );
    assert!(sources["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["key"] == "OTEL_LOG_USER_PROMPTS" && row["state"] == "conflicting"));
    assert!(sources["files"]
        .as_array()
        .unwrap()
        .iter()
        .any(
            |row| row["path"] == json!(managed.join("managed-settings.json"))
                && row["writable"] == false
        ));
    let bad=engine.request(json!({"command":"plan_config_edit","environment_id":e["id"],"config_path":local,"document":json!({"env":{"OTEL_LOG_RAW_API_BODIES":"1"}}).to_string(),"expected_digest":digest(&read(&local,CONFIG_BYTES).unwrap())}));
    assert_eq!(bad["error"]["code"], "config_source_not_permitted");
    let plan = edit(
        &engine,
        &e,
        &local,
        json!({"env":{"OTEL_LOG_RAW_API_BODIES":"0"}}),
    );
    let done = execute(&engine, &plan);
    assert_eq!(load(&local).unwrap()["env"]["OTEL_LOG_RAW_API_BODIES"], "0");
    assert_eq!(done["configuration"], Value::Null);
}

#[test]
fn new_project_source_freezes_parent_and_does_not_enable_recording() {
    let (_temp, engine, e, root) = fixture();
    let project = root.parent().unwrap().join("new-project");
    fs::create_dir(&project).unwrap();
    let path = project.join(".claude/settings.local.json");
    let forbidden=engine.request(json!({"command":"plan_config_edit","environment_id":e["id"],"config_path":path,"document":json!({"env":{"OTEL_LOG_RAW_API_BODIES":"file:/synthetic"}}).to_string(),"expected_digest":digest(b"")}));
    assert_eq!(forbidden["error"]["code"], "config_source_not_permitted");
    assert!(!path.parent().unwrap().exists());
    let plan = edit(
        &engine,
        &e,
        &path,
        json!({"env":{"OTEL_LOG_RAW_API_BODIES":"0"}}),
    );
    assert!(!path.exists());
    let job = execute(&engine, &plan);
    assert!(path.exists());
    assert_eq!(job["steps"].as_array().unwrap().len(), 3);
    let restore = ask(
        &engine,
        "plan_config_restore",
        &e,
        json!({"job_id":job["id"],"selected_keys":["/env/OTEL_LOG_RAW_API_BODIES"]}),
    );
    execute(&engine, &restore);
    assert_eq!(load(&path).unwrap(), json!({}));
}

#[test]
fn stale_inputs_fail_before_accept_and_identity_files_are_not_config() {
    let (_temp, engine, e, root) = fixture();
    let path = root.join("settings.json");
    let mut next = load(&path).unwrap();
    next["model"] = json!("changed");
    let plan = edit(&engine, &e, &path, next);
    let mut external = load(&path).unwrap();
    external["external"] = json!("preserve");
    save(&path, &external).unwrap();
    let rejected=engine.request(json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"],"archive_passphrase":PASS}));
    assert_eq!(rejected["error"]["code"], "stale_plan");
    assert!(!engine.path("jobs", plan["id"].as_str().unwrap()).exists());
    assert_eq!(load(&path).unwrap(), external);
    for name in [".claude.json", ".credentials.json", "managed-settings.json"] {
        let r = engine.request(
            json!({"command":"config_read","environment_id":e["id"],"config_path":root.join(name)}),
        );
        assert_eq!(r["error"]["code"], "config_readonly");
    }
    let read = ask(&engine, "config_read", &e, json!({"config_path":path}));
    let bad=engine.request(json!({"command":"plan_config_edit","environment_id":e["id"],"config_path":path,"expected_digest":read["digest"],"document":"{\"model\":1,\"model\":2}"}));
    assert_eq!(bad["ok"], false);
}

#[test]
fn cross_environment_apply_excludes_private_material_and_binds_target() {
    let (_temp, engine, e, root) = fixture();
    let target = root.parent().unwrap().join("target");
    fs::create_dir(&target).unwrap();
    save(
        &target.join("settings.json"),
        &json!({"env":{"UNRELATED":"target-only"}}),
    )
    .unwrap();
    let other = engine.register("target", &target, false).unwrap();
    let plan = ask(
        &engine,
        "plan_config_apply",
        &other,
        json!({"source_environment_id":e["id"],"selected_keys":["OTEL_LOG_USER_PROMPTS"]}),
    );
    assert_eq!(plan["environment_id"], other["id"]);
    execute(&engine, &plan);
    let after = load(&target.join("settings.json")).unwrap();
    assert_eq!(after["env"]["OTEL_LOG_USER_PROMPTS"], "1");
    assert_eq!(after["env"]["UNRELATED"], "target-only");
    assert!(after.get("apiKeyHelper").is_none());
    let bad=engine.request(json!({"command":"plan_config_apply","environment_id":other["id"],"source_environment_id":e["id"],"selected_keys":["OTEL_EXPORTER_OTLP_HEADERS"]}));
    assert_eq!(bad["error"]["code"], "private_setting_not_copyable");
    let wrong = engine.request(
        json!({"command":"config_restore_inspect","environment_id":e["id"],"job_id":plan["id"]}),
    );
    assert_eq!(wrong["error"]["code"], "environment_mismatch");
}

#[test]
fn crash_child() {
    let Some(base) = std::env::var_os("LINTEL_CONFIG_TEST_BASE") else {
        return;
    };
    let base = PathBuf::from(base);
    let mut engine = Engine::new(base.join("home"), base.join("state")).unwrap();
    engine.executable_search_path = Some(std::ffi::OsString::new());
    engine.settings_write_hook = Some(|phase| {
        if phase == std::env::var("LINTEL_CONFIG_TEST_PHASE").unwrap() {
            unsafe { libc::_exit(73) }
        }
    });
    let plan =
        load(&engine.path("plans", &std::env::var("LINTEL_CONFIG_TEST_PLAN").unwrap())).unwrap();
    engine.request(json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"],"archive_passphrase":PASS}));
    panic!("fault hook was not reached");
}

#[test]
fn interrupted_config_publication_requires_original_owned_evidence() {
    for phase in ["prepared", "published"] {
        let (_temp, engine, e, root) = fixture();
        let path = root.join("settings.json");
        let mut next = load(&path).unwrap();
        next["model"] = json!("changed");
        let plan = edit(&engine, &e, &path, next);
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "config_control::tests::crash_child",
                "--nocapture",
            ])
            .env("LINTEL_CONFIG_TEST_BASE", engine.home.parent().unwrap())
            .env("LINTEL_CONFIG_TEST_PLAN", plan["id"].as_str().unwrap())
            .env("LINTEL_CONFIG_TEST_PHASE", phase)
            .status()
            .unwrap();
        assert_eq!(result.code(), Some(73));
        let response = engine.request(json!({"command":"job","job_id":plan["id"]}));
        assert_eq!(response["data"]["status"], "needs_reconciliation");
        assert_eq!(
            response["data"]["restorable"],
            phase == "published",
            "{response}"
        );
        private(&response);
        let before = read(&path, CONFIG_BYTES).unwrap();
        let repeat=engine.request(json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"],"archive_passphrase":PASS}));
        assert_eq!(repeat["data"]["id"], plan["id"]);
        assert_eq!(read(&path, CONFIG_BYTES).unwrap(), before);
    }
}

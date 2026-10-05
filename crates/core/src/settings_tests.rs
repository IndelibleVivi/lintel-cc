//! Real subprocess exits at the two settings-publication journal boundaries.
use super::*;
use std::process::Command;

fn fixture() -> (tempfile::TempDir, Engine, Value, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let home = base.join("home");
    let root = home.join("synthetic-root");
    fs::create_dir_all(&root).unwrap();
    save(
        &root.join("settings.json"),
        &json!({"env":{"UNRELATED":"keep"}}),
    )
    .unwrap();
    let engine = Engine::new(home, base.join("state")).unwrap();
    let environment = engine.register("synthetic", &root, false).unwrap();
    let plan = engine.request(json!({"command":"plan_policy","environment_id":environment["id"],"preset":"custom","custom_settings":{"DISABLE_TELEMETRY":"disable"}}));
    assert_eq!(plan["ok"], true, "{plan}");
    (temp, engine, plan["data"].clone(), root)
}

fn crash(engine: &Engine, plan: &Value, phase: &str) {
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "settings_tests::crash_child", "--nocapture"])
        .env(
            "LINTEL_CORE_SETTINGS_CRASH_BASE",
            engine.home.parent().unwrap(),
        )
        .env(
            "LINTEL_CORE_SETTINGS_CRASH_PLAN",
            plan["id"].as_str().unwrap(),
        )
        .env("LINTEL_CORE_SETTINGS_CRASH_PHASE", phase)
        .status()
        .unwrap();
    assert_eq!(
        status.code(),
        Some(73),
        "the child must exit at the actual publication boundary"
    );
    let journal = load(&engine.path("jobs", plan["id"].as_str().unwrap())).unwrap();
    assert_eq!(journal["status"], "executing");
    assert_eq!(journal["restorable"], false);
    assert_eq!(journal["settings_write"]["state"], "prepared");
}

#[test]
fn crash_child() {
    let Some(base) = std::env::var_os("LINTEL_CORE_SETTINGS_CRASH_BASE") else {
        return;
    };
    let base = PathBuf::from(base);
    let mut engine = Engine::new(base.join("home"), base.join("state")).unwrap();
    engine.settings_write_hook = Some(|phase| {
        if std::env::var("LINTEL_CORE_SETTINGS_CRASH_PHASE").unwrap() == phase {
            // No unwinding or final receipt writes: exactly like abrupt process death.
            unsafe { libc::_exit(73) }
        }
    });
    let plan = load(&engine.path(
        "plans",
        &std::env::var("LINTEL_CORE_SETTINGS_CRASH_PLAN").unwrap(),
    ))
    .unwrap();
    engine.request(json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"]}));
    panic!("publication boundary was not reached");
}

#[test]
fn published_settings_recover_after_death_before_receipt_save() {
    let (_temp, engine, plan, root) = fixture();
    crash(&engine, &plan, "published");
    let before_query = snapshot(&root.join("settings.json")).unwrap();
    let job = engine.request(json!({"command":"job","job_id":plan["id"]}));
    assert_eq!(job["data"]["status"], "needs_reconciliation", "{job}");
    assert_eq!(job["data"]["settings_recovery"]["state"], "written");
    assert_eq!(job["data"]["restorable"], true);
    // Re-execution of the original approval is query-only.
    let repeat =
        engine.request(json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"]}));
    assert_eq!(repeat["data"], job["data"]);
    assert_eq!(snapshot(&root.join("settings.json")).unwrap(), before_query);
    let restore = engine.request(json!({"command":"plan_restore","job_id":plan["id"]}));
    assert_eq!(restore["ok"], true, "{restore}");
    assert_eq!(restore["data"]["changes"][0]["after"], Value::Null);
    let done = engine.request(json!({"command":"execute","plan_id":restore["data"]["id"],"approval":restore["data"]["hash"]}));
    assert_eq!(done["data"]["status"], "completed", "{done}");
    assert_eq!(
        load(&root.join("settings.json")).unwrap(),
        json!({"env":{"UNRELATED":"keep"}})
    );
}

#[test]
fn unwritten_intent_cannot_recover_a_later_equal_external_setting() {
    for query_before_external_write in [false, true] {
        let (_temp, engine, plan, root) = fixture();
        crash(&engine, &plan, "prepared");
        if query_before_external_write {
            let job = engine.request(json!({"command":"job","job_id":plan["id"]}));
            assert_eq!(job["data"]["settings_recovery"]["state"], "not_written");
        }
        let staged = fs::read_dir(&root)
            .unwrap()
            .map(|f| f.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".lintel-")
            })
            .unwrap();
        atomic(
            &root.join("settings.json"),
            &fs::read(staged).unwrap(),
            0o600,
        )
        .unwrap();
        let external = snapshot(&root.join("settings.json")).unwrap();
        let restore = engine.request(json!({"command":"plan_restore","job_id":plan["id"]}));
        assert_eq!(restore["error"]["code"], "not_restorable", "{restore}");
        let job = engine.request(json!({"command":"job","job_id":plan["id"]}));
        assert_eq!(
            job["data"]["settings_recovery"]["state"],
            if query_before_external_write {
                "not_written"
            } else {
                "ownership_unproven"
            }
        );
        assert_eq!(snapshot(&root.join("settings.json")).unwrap(), external);
    }
}

#[test]
fn interrupted_write_refuses_external_rewrite_or_changed_plan_or_root() {
    for replacement in ["external", "plan", "root"] {
        let (_temp, engine, plan, root) = fixture();
        crash(&engine, &plan, "published");
        match replacement {
            "external" => {
                // Even identical approved bytes in a replacement inode do not
                // prove which writer published the current settings.
                let bytes = fs::read(root.join("settings.json")).unwrap();
                atomic(&root.join("settings.json"), &bytes, 0o600).unwrap();
            }
            "plan" => {
                let path = engine.path("plans", plan["id"].as_str().unwrap());
                let mut frozen = load(&path).unwrap();
                frozen["changes"][0]["before"] = json!("tampered");
                save(&path, &frozen).unwrap();
            }
            _ => {
                fs::rename(&root, root.with_file_name("original-root")).unwrap();
                fs::create_dir(&root).unwrap();
                fs::copy(
                    root.with_file_name("original-root").join("settings.json"),
                    root.join("settings.json"),
                )
                .unwrap();
            }
        }
        let retained = snapshot(&root.join("settings.json")).unwrap();
        let job = engine.request(json!({"command":"job","job_id":plan["id"]}));
        assert_eq!(
            job["data"]["settings_recovery"]["state"], "ownership_unproven",
            "{job}"
        );
        assert_eq!(job["data"]["restorable"], false);
        assert_eq!(
            engine.request(json!({"command":"plan_restore","job_id":plan["id"]}))["error"]["code"],
            "not_restorable"
        );
        assert_eq!(snapshot(&root.join("settings.json")).unwrap(), retained);
    }
}

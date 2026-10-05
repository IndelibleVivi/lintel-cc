//! Core regression tests for independent work preservation and execution
//! semantics. Synthetic temporary roots only; no real Claude, account or
//! operator path is ever touched.
use super::*;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

const PASS: &str = "synthetic work-preservation passphrase";

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
fn ok(engine: &Engine, r: Value) -> Value {
    let v = engine.request(r);
    assert_eq!(v["ok"], true, "{v}");
    v["data"].clone()
}
fn err_code(engine: &Engine, r: Value) -> String {
    let v = engine.request(r);
    assert_eq!(v["ok"], false, "{v}");
    v["error"]["code"].as_str().unwrap().to_string()
}
fn seed_work(root: &Path) {
    fs::write(root.join("CLAUDE.md"), "Synthetic instruction only.").unwrap();
    fs::create_dir_all(root.join("projects/example/memory")).unwrap();
    fs::write(
        root.join("projects/example/memory/MEMORY.md"),
        "Synthetic memory only.",
    )
    .unwrap();
    fs::write(
        root.join("projects/example/session.jsonl"),
        "{\"synthetic\":true}\n",
    )
    .unwrap();
    fs::write(
        root.join(".credentials.json"),
        "SYNTHETIC_CREDENTIAL_DO_NOT_COPY",
    )
    .unwrap();
    fs::write(
        root.join("settings.json"),
        "{\"env\":{\"SYNTHETIC_LOGIN_FLAG\":\"keep\"}}",
    )
    .unwrap();
}

#[test]
fn archive_only_does_not_create_or_change_environment() {
    let (_t, engine, e, root) = fixture();
    seed_work(&root);
    let before_inventory = engine.request(json!({"command":"discover"}))["data"]["environments"]
        .as_array()
        .unwrap()
        .len();
    let p = ok(
        &engine,
        json!({"command":"plan_archive","environment_id":e["id"],"categories":["instructions","memory","sessions"]}),
    );
    assert_eq!(p["kind"], "archive");
    assert_eq!(p["outcome"], "archive_only");
    assert_eq!(p["file_count"], 3);
    let j = ok(
        &engine,
        json!({"command":"execute","plan_id":p["id"],"approval":p["hash"],"archive_passphrase":PASS}),
    );
    assert_eq!(j["status"], "completed");
    assert_eq!(j["outcome"], "archive_only");
    assert!(
        j["new_environment_id"].is_null(),
        "archive-only must not create a root"
    );
    assert!(root.join(".credentials.json").exists());
    assert!(root.join("settings.json").exists());
    assert_eq!(
        fs::read_to_string(root.join("settings.json")).unwrap(),
        "{\"env\":{\"SYNTHETIC_LOGIN_FLAG\":\"keep\"}}"
    );
    let after_inventory = engine.request(json!({"command":"discover"}))["data"]["environments"]
        .as_array()
        .unwrap()
        .len();
    assert_eq!(
        before_inventory, after_inventory,
        "no new environment was registered"
    );
    let encrypted = fs::read(j["archive_path"].as_str().unwrap()).unwrap();
    assert!(encrypted.starts_with(b"age-encryption.org/"));
    assert!(!String::from_utf8_lossy(&encrypted).contains("Synthetic instruction"));
    for path in fs::read_dir(engine.state.join("jobs")).unwrap() {
        let text = fs::read_to_string(path.unwrap().path()).unwrap();
        assert!(!text.contains(PASS));
    }
}

#[test]
fn archive_output_directory_replacement_rejects_before_publication() {
    let (_temp, engine, environment, root) = fixture();
    seed_work(&root);
    let directory = root.parent().unwrap().join("output");
    fs::create_dir(&directory).unwrap();
    let output = directory.join("carried.age");
    let plan = ok(
        &engine,
        json!({"command":"plan_archive","environment_id":environment["id"],"categories":["instructions"],"output_path":output}),
    );
    fs::rename(&directory, directory.with_file_name("original-output")).unwrap();
    fs::create_dir(&directory).unwrap();
    assert_eq!(
        err_code(
            &engine,
            json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"],"archive_passphrase":PASS})
        ),
        "stale_plan"
    );
    assert!(!output.exists());
    assert!(!directory
        .with_file_name("original-output")
        .join("carried.age")
        .exists());
    assert!(!engine.path("jobs", plan["id"].as_str().unwrap()).exists());
    assert_eq!(
        fs::read(root.join("CLAUDE.md")).unwrap(),
        b"Synthetic instruction only."
    );
    // The common writer must recheck the same identity at publication, even
    // when replacement occurs after the acceptance-time check.
    let frozen = load(&engine.path("plans", plan["id"].as_str().unwrap())).unwrap();
    let journal = engine.path("jobs", plan["id"].as_str().unwrap());
    let mut receipt = json!({"steps":[]});
    let failure = engine
        .write_archive(
            &environment,
            &frozen,
            &json!({"archive_passphrase":PASS}),
            &output,
            &mut receipt,
            &journal,
        )
        .unwrap_err();
    assert_eq!(failure.code, "stale_plan");
    assert!(!output.exists());
    assert!(!directory
        .with_file_name("original-output")
        .join("carried.age")
        .exists());
}

#[test]
fn archive_output_path_freezes_and_refuses_overwrite() {
    let (_t, engine, e, root) = fixture();
    seed_work(&root);
    let out = root.parent().unwrap().join("carried.age");
    let p = ok(
        &engine,
        json!({"command":"plan_archive","environment_id":e["id"],"categories":["instructions"],"output_path":out.to_str().unwrap()}),
    );
    assert_eq!(p["output_path"], out.to_str().unwrap());
    let plan_path = engine.path("plans", p["id"].as_str().unwrap());
    let frozen = load(&plan_path).unwrap();
    let mut legacy = frozen.clone();
    legacy["extra"]
        .as_object_mut()
        .unwrap()
        .remove("output_parent_identity");
    legacy.as_object_mut().unwrap().remove("hash");
    legacy["hash"] = json!(digest(&serde_json::to_vec(&legacy).unwrap()));
    save(&plan_path, &legacy).unwrap();
    assert_eq!(
        err_code(
            &engine,
            json!({"command":"execute","plan_id":p["id"],"approval":legacy["hash"],"archive_passphrase":PASS})
        ),
        "stale_plan"
    );
    assert!(!out.exists());
    assert!(!engine.path("jobs", p["id"].as_str().unwrap()).exists());
    save(&plan_path, &frozen).unwrap();
    let j = ok(
        &engine,
        json!({"command":"execute","plan_id":p["id"],"approval":p["hash"],"archive_passphrase":PASS}),
    );
    assert_eq!(j["archive_path"], out.to_str().unwrap());
    assert!(out.exists());
    assert_eq!(
        err_code(
            &engine,
            json!({"command":"plan_archive","environment_id":e["id"],"categories":["instructions"],"output_path":out.to_str().unwrap()})
        ),
        "output_exists"
    );
    assert_eq!(
        err_code(
            &engine,
            json!({"command":"plan_archive","environment_id":e["id"],"categories":["instructions"],"output_path":"/no/such/lintel/dir/x.age"})
        ),
        "invalid_output_path"
    );
}

#[test]
fn encrypted_package_travels_to_independent_install() {
    let (_t, engine, e, root) = fixture();
    seed_work(&root);
    let out = root.parent().unwrap().join("carried.age");
    let p = ok(
        &engine,
        json!({"command":"plan_archive","environment_id":e["id"],"categories":["instructions","memory","sessions"],"output_path":out.to_str().unwrap()}),
    );
    ok(
        &engine,
        json!({"command":"execute","plan_id":p["id"],"approval":p["hash"],"archive_passphrase":PASS}),
    );
    let package = archive::unseal(&fs::read(&out).unwrap(), PASS).unwrap();
    assert_eq!(package["generator"], "Lintel");
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let other_home = base.join("home");
    let dest_root = other_home.join("other-claude");
    fs::create_dir_all(&dest_root).unwrap();
    let other = Engine::new(other_home, base.join("state")).unwrap();
    let dest = other.register("independent", &dest_root, false).unwrap();
    let path = out.to_str().unwrap();
    let manifest = ok(
        &other,
        json!({"command":"archive_inspect","archive_path":path,"archive_passphrase":PASS}),
    );
    assert_eq!(manifest["generator"], "Lintel");
    assert_eq!(manifest["schema"], "lintel.work/1");
    assert_eq!(manifest["files"].as_array().unwrap().len(), 3);
    let read = ok(
        &other,
        json!({"command":"archive_read","archive_path":path,"archive_passphrase":PASS,"path":"CLAUDE.md"}),
    );
    assert_eq!(read["text"], "Synthetic instruction only.");
    let import = ok(
        &other,
        json!({"command":"plan_import","environment_id":dest["id"],"archive_path":path,"categories":["instructions","memory","sessions"],"archive_passphrase":PASS}),
    );
    let j = ok(
        &other,
        json!({"command":"execute","plan_id":import["id"],"approval":import["hash"],"archive_passphrase":PASS}),
    );
    assert_eq!(j["status"], "completed");
    assert_eq!(
        fs::read_to_string(dest_root.join("CLAUDE.md")).unwrap(),
        "Synthetic instruction only."
    );
    assert!(dest_root
        .join("lintel-imports/projects/example/session.jsonl")
        .exists());
    fs::create_dir_all(base.join("other-two")).unwrap();
    let dest2 = other
        .register("independent two", &base.join("other-two"), false)
        .unwrap();
    let import_mem = ok(
        &other,
        json!({"command":"plan_import","environment_id":dest2["id"],"archive_path":path,"categories":["memory"],"archive_passphrase":PASS}),
    );
    ok(
        &other,
        json!({"command":"execute","plan_id":import_mem["id"],"approval":import_mem["hash"],"archive_passphrase":PASS}),
    );
    assert!(
        !base.join("other-two/CLAUDE.md").exists(),
        "unselected category was not imported"
    );
    assert!(base
        .join("other-two/lintel-imports/projects/example/memory/MEMORY.md")
        .exists());
    assert_eq!(
        err_code(
            &other,
            json!({"command":"plan_import","environment_id":dest["id"],"archive_path":path,"categories":["instructions"],"archive_passphrase":PASS})
        ),
        "import_conflict"
    );
}

#[test]
fn stale_wrong_passphrase_and_corrupt_packages_are_refused() {
    let (_t, engine, e, root) = fixture();
    seed_work(&root);
    let out = root.parent().unwrap().join("carried.age");
    let p = ok(
        &engine,
        json!({"command":"plan_archive","environment_id":e["id"],"categories":["instructions"],"output_path":out.to_str().unwrap()}),
    );
    ok(
        &engine,
        json!({"command":"execute","plan_id":p["id"],"approval":p["hash"],"archive_passphrase":PASS}),
    );
    let path = out.to_str().unwrap();
    assert_eq!(
        err_code(
            &engine,
            json!({"command":"archive_inspect","archive_path":path,"archive_passphrase":"incorrect synthetic passphrase"}),
        ),
        "archive_locked"
    );
    fs::write(&out, b"not an age archive").unwrap();
    assert_eq!(
        err_code(
            &engine,
            json!({"command":"archive_inspect","archive_path":path,"archive_passphrase":PASS}),
        ),
        "invalid_archive"
    );
    let payload = json!({"schema":"lintel.work/1","files":[{"path":"../CLAUDE.md","category":"instructions","data":[65],"digest":digest(b"A")}]});
    fs::write(&out, archive::seal(&payload, PASS).unwrap()).unwrap();
    assert_eq!(
        err_code(
            &engine,
            json!({"command":"archive_inspect","archive_path":path,"archive_passphrase":PASS}),
        ),
        "archive_path"
    );
}

#[test]
fn preserve_completes_while_legacy_reset_stays_partial() {
    let (_t, engine, e, root) = fixture();
    seed_work(&root);
    let preserve = ok(
        &engine,
        json!({"command":"plan_preserve","environment_id":e["id"],"categories":["instructions","memory","sessions"],"name":"writing"}),
    );
    assert_eq!(preserve["kind"], "preserve");
    assert_eq!(preserve["outcome"], "preserve");
    let j = ok(
        &engine,
        json!({"command":"execute","plan_id":preserve["id"],"approval":preserve["hash"],"archive_passphrase":PASS}),
    );
    assert_eq!(j["status"], "completed", "{j}");
    assert_eq!(j["outcome"], "preserved");
    let new_root = PathBuf::from(j["new_root"].as_str().unwrap());
    assert!(new_root.join("CLAUDE.md").exists());
    assert!(new_root
        .join("lintel-imports/projects/example/session.jsonl")
        .exists());
    assert!(root.join(".credentials.json").exists());
    assert!(root.join("settings.json").exists());
    // The retained old environment is the desired outcome, reported as
    // `preserved` (not a misleading not_completed step) with explicit coverage.
    let steps = j["steps"].as_array().unwrap();
    let retained = steps.iter().find(|s| s["id"] == "status").unwrap();
    assert_eq!(retained["status"], "preserved", "{j}");
    assert_eq!(j["coverage"]["old_login"], "retained");
    assert_eq!(j["coverage"]["old_root"], "retained");
    assert_eq!(j["coverage"]["service_binding"], "unchanged");
    assert!(j["coverage"]["file_count"].as_u64().unwrap() >= 3);
    assert!(j["next_steps"].as_array().unwrap().len() >= 2);
    let legacy = ok(
        &engine,
        json!({"command":"plan_reset","environment_id":e["id"],"recipe":"rebuild","categories":["instructions"]}),
    );
    let lj = ok(
        &engine,
        json!({"command":"execute","plan_id":legacy["id"],"approval":legacy["hash"],"archive_passphrase":PASS}),
    );
    assert_eq!(lj["status"], "partially_completed", "{lj}");
}

#[test]
fn failure_after_verifying_records_real_phase() {
    let (_t, engine, e, root) = fixture();
    fs::write(
        root.join("settings.json"),
        "{\"env\":{\"DISABLE_TELEMETRY\":\"false\"}}",
    )
    .unwrap();
    let p = ok(
        &engine,
        json!({"command":"plan_policy","environment_id":e["id"],"preset":"reduce","keep_remote_control":false}),
    );
    // Make the post-write baseline save fail: the settings write itself succeeds
    // and the receipt reaches `verifying`, then the failure is recorded. Replace
    // the private baseline directory with a regular file so creating the baseline
    // record fails deterministically (no permission/root assumptions).
    let baselines = engine.state.join("baselines");
    fs::remove_dir_all(&baselines).unwrap();
    fs::write(&baselines, b"block").unwrap();
    let j = ok(
        &engine,
        json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}),
    );
    assert_eq!(j["status"], "needs_reconciliation", "{j}");
    assert_eq!(j["error"]["phase"], "verifying", "{j}");
    assert!(j["error"]["code"].is_string());
    assert!(j["error"]["recovery"].is_string());
    assert!(
        j["restorable"].as_bool().unwrap(),
        "written settings stay restorable"
    );
    assert!(root.join("settings.json").exists());
    // The settings write completed before the verifying-stage failure; the
    // receipt stays restorable and the file remains valid JSON.
    assert!(load(&root.join("settings.json")).unwrap()["env"].is_object());
}

#[test]
fn logout_failure_reports_ordered_steps_and_does_not_delete() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let home = base.join("home");
    let root = home.join("cc");
    fs::create_dir_all(&root).unwrap();
    let engine = Engine::new(home.clone(), base.join("state")).unwrap();
    let script = home.join("fake-claude");
    fs::write(
        &script,
        "#!/bin/sh\ncase \"$2\" in\n status) printf '{\"configDirectory\":\"%s\",\"authMethod\":\"claude.ai\"}' \"$CLAUDE_CONFIG_DIR\"; exit 0;;\n logout) exit 3;;\nesac\nexit 9\n",
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let mut e = engine.register("synthetic", &root, false).unwrap();
    e["executable"] = json!(script);
    save(&engine.state.join("inventory.json"), &json!([e.clone()])).unwrap();
    fs::write(root.join("CLAUDE.md"), "Synthetic instruction only.").unwrap();
    fs::write(root.join(".credentials.json"), "synthetic credential").unwrap();
    fs::write(root.join(".claude.json"), "{\"synthetic\":true}").unwrap();
    let p = ok(
        &engine,
        json!({"command":"plan_cleanup","environment_id":e["id"],"recipe":"reset_client","writers_confirmed_stopped":true,"official_logout":true,"categories":["instructions"]}),
    );
    let ids: Vec<String> = p["actions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap().to_string())
        .collect();
    let pos = |name: &str| ids.iter().position(|x| x == name).unwrap();
    // Canonical reset order in the frozen preview: archive, then the fresh root
    // + migration, then logout, then removals. No destructive action leads.
    assert!(pos("archive") < pos("logout"), "{ids:?}");
    assert!(pos("archive") < pos("rebuild"), "{ids:?}");
    assert!(pos("rebuild") < pos("logout"), "{ids:?}");
    assert!(pos("logout") < pos("credentials"), "{ids:?}");
    let j = ok(
        &engine,
        json!({"command":"execute","plan_id":p["id"],"approval":p["hash"],"archive_passphrase":PASS}),
    );
    assert_eq!(j["status"], "needs_reconciliation", "{j}");
    assert_eq!(j["error"]["code"], "logout_failed", "{j}");
    assert_eq!(j["error"]["phase"], "executing");
    assert_eq!(j["error"]["uncertain_side_effects"], true);
    assert_eq!(j["error"]["step_id"], "logout");
    assert!(j["error"]["recovery"].is_string());
    assert!(j["archive_path"].is_string());
    assert!(j["state_archive_path"].is_string());
    // The fresh root and its migration were produced BEFORE the failed logout and
    // are retained; the approved local files were not deleted.
    assert!(
        j["new_root"].is_string(),
        "fresh root must be retained across logout failure"
    );
    let new_root = PathBuf::from(j["new_root"].as_str().unwrap());
    assert_eq!(
        fs::read_to_string(new_root.join("CLAUDE.md")).unwrap(),
        "Synthetic instruction only."
    );
    assert!(root.join(".credentials.json").exists());
    assert!(root.join(".claude.json").exists());
    let steps = j["steps"].as_array().unwrap();
    assert_eq!(
        steps.iter().find(|s| s["id"] == "archive").unwrap()["status"],
        "completed"
    );
    assert_eq!(
        steps.iter().find(|s| s["id"] == "create").unwrap()["status"],
        "completed"
    );
    assert_eq!(
        steps.iter().find(|s| s["id"] == "migrate").unwrap()["status"],
        "completed"
    );
    assert_eq!(
        steps.iter().find(|s| s["id"] == "logout").unwrap()["status"],
        "executing"
    );
    // Ordered execution: create and migrate precede the logout step.
    let step_pos = |name: &str| steps.iter().position(|s| s["id"] == name).unwrap();
    assert!(step_pos("archive") < step_pos("create"), "{steps:?}");
    assert!(step_pos("create") < step_pos("logout"), "{steps:?}");
    // No local removal step was executed after the failed logout.
    assert!(steps.iter().all(|s| s["id"] != "credentials"), "{steps:?}");
    assert!(steps.iter().all(|s| s["id"] != "client_state"), "{steps:?}");
    let queried = ok(&engine, json!({"command":"job","job_id":p["id"]}));
    assert_eq!(queried["status"], "needs_reconciliation");
    assert_eq!(queried["error"]["code"], "logout_failed");
    assert_eq!(queried["new_root"], j["new_root"]);
    assert!(root.join(".credentials.json").exists());
    assert_eq!(queried["id"], j["id"]);
}

#[test]
fn accepted_after_failure_code_persists_across_engines() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let home = base.join("home");
    let root = home.join("cc");
    fs::create_dir_all(&root).unwrap();
    let state = base.join("state");
    let engine = Engine::new(home.clone(), state.clone()).unwrap();
    let e = engine.register("synthetic", &root, false).unwrap();
    fs::write(root.join("CLAUDE.md"), "instruction").unwrap();
    let p = ok(
        &engine,
        json!({"command":"plan_archive","environment_id":e["id"],"categories":["instructions"]}),
    );
    // Occupy the exact private archive destination with a directory so the
    // write fails only after the durable accept has been persisted.
    let dest = state
        .join("archives")
        .join(format!("{}.age", p["id"].as_str().unwrap()));
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("x"), "block").unwrap();
    let j = ok(
        &engine,
        json!({"command":"execute","plan_id":p["id"],"approval":p["hash"],"archive_passphrase":PASS}),
    );
    assert_eq!(j["status"], "needs_reconciliation", "{j}");
    let code = j["error"]["code"].as_str().unwrap().to_string();
    assert!(!code.is_empty());
    let reopened = Engine::new(home, state).unwrap();
    let q = ok(&reopened, json!({"command":"job","job_id":p["id"]}));
    assert_eq!(q["status"], "needs_reconciliation");
    assert_eq!(q["error"]["code"], code);
    assert!(q["error"]["message"].is_string());
    assert_eq!(q["error"]["phase"], "executing");
}

#[test]
fn plan_show_reads_frozen_plan_without_secrets() {
    let (_t, engine, e, root) = fixture();
    seed_work(&root);
    let out = root.parent().unwrap().join("x.age");
    let p = ok(
        &engine,
        json!({"command":"plan_archive","environment_id":e["id"],"categories":["instructions"],"output_path":out.to_str().unwrap()}),
    );
    let shown = ok(&engine, json!({"command":"plan_show","plan_id":p["id"]}));
    assert_eq!(shown["id"], p["id"]);
    assert_eq!(shown["hash"], p["hash"]);
    assert_eq!(shown["kind"], "archive");
    assert_eq!(shown["file_count"], 1);
    assert!(shown["archive_passphrase_required"].as_bool().unwrap());
    let text = shown.to_string();
    assert!(!text.contains(PASS));
    assert!(shown.get("snapshot").is_none());
    assert!(shown.get("root_identity").is_none());
    assert!(shown.get("extra").is_none());
    assert_eq!(
        err_code(
            &engine,
            json!({"command":"plan_show","plan_id":"00000000-0000-4000-8000-000000000009"})
        ),
        "plan_not_found"
    );
}

#[test]
fn publishing_new_files_keeps_concurrent_target_and_single_link() {
    use std::os::unix::fs::MetadataExt;
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let occupied = base.join("occupied.age");
    fs::write(&occupied, b"external content").unwrap();
    let failure = atomic_new(&occupied, b"approved package", 0o600).unwrap_err();
    assert_eq!(failure.code, "target_exists");
    assert_eq!(fs::read(&occupied).unwrap(), b"external content");
    let fresh = base.join("fresh.age");
    atomic_new(&fresh, b"complete package", 0o600).unwrap();
    assert_eq!(fs::read(&fresh).unwrap(), b"complete package");
    assert_eq!(fs::metadata(&fresh).unwrap().nlink(), 1);
    assert_eq!(
        fs::read_dir(&base).unwrap().count(),
        2,
        "temporary publication files remain"
    );
}

#[test]
fn portable_archive_generator_is_declared_metadata_or_unknown() {
    let (_temp, engine, _environment, root) = fixture();
    for (name, generator) in [("legacy", None), ("external", Some("Compatible exporter"))] {
        let mut package = json!({"schema":"lintel.work/1","files":[]});
        if let Some(generator) = generator {
            package["generator"] = json!(generator);
        }
        let path = root.join(format!("{name}.age"));
        atomic_new(&path, &archive::seal(&package, PASS).unwrap(), 0o600).unwrap();
        let inspected = ok(
            &engine,
            json!({"command":"archive_inspect","archive_path":path,"archive_passphrase":PASS}),
        );
        assert_eq!(inspected["generator"], json!(generator));
    }
}

#[test]
fn job_archive_rejects_replacement_but_explicit_path_remains_independent() {
    let (_temp, engine, environment, root) = fixture();
    seed_work(&root);
    let out = root.parent().unwrap().join("export.age");
    let plan = ok(
        &engine,
        json!({"command":"plan_archive","environment_id":environment["id"],"categories":["instructions"],"output_path":out}),
    );
    let receipt = ok(
        &engine,
        json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"],"archive_passphrase":PASS}),
    );
    let mut replacement = archive::unseal(&fs::read(&out).unwrap(), PASS).unwrap();
    let record_path = engine.path("jobs", receipt["id"].as_str().unwrap());
    let completed = load(&record_path).unwrap();
    let mut interrupted = completed.clone();
    interrupted
        .as_object_mut()
        .unwrap()
        .remove("archive_digest");
    interrupted["status"] = json!("needs_reconciliation");
    interrupted["steps"][0]["status"] = json!("executing");
    save(&record_path, &interrupted).unwrap();
    // A package published before the completion save remains discoverable by
    // its original job and is bound to the pre-publication ciphertext intent.
    ok(
        &engine,
        json!({"command":"archive_inspect","job_id":receipt["id"],"archive_passphrase":PASS}),
    );
    save(&record_path, &completed).unwrap();
    let bytes: &[u8] = b"synthetic replacement instruction";
    replacement["files"][0]["data"] = json!(bytes);
    replacement["files"][0]["digest"] = json!(digest(bytes));
    atomic(&out, &archive::seal(&replacement, PASS).unwrap(), 0o600).unwrap();
    let destination = root.parent().unwrap().join("destination");
    fs::create_dir(&destination).unwrap();
    let target = engine
        .register("synthetic target", &destination, false)
        .unwrap();
    for command in ["archive_inspect", "archive_read", "plan_import"] {
        let mut request =
            json!({"command":command,"job_id":receipt["id"],"archive_passphrase":PASS});
        if command == "archive_read" {
            request["path"] = json!("CLAUDE.md");
        }
        if command == "plan_import" {
            request["environment_id"] = target["id"].clone();
            request["categories"] = json!(["instructions"]);
        }
        assert_eq!(err_code(&engine, request), "stale_archive", "{command}");
    }
    let explicit = ok(
        &engine,
        json!({"command":"archive_read","archive_path":out,"archive_passphrase":PASS,"path":"CLAUDE.md"}),
    );
    assert_eq!(explicit["text"], "synthetic replacement instruction");
    // Legacy receipts without a digest retain their documented compatibility.
    let mut legacy = load(&record_path).unwrap();
    legacy.as_object_mut().unwrap().remove("archive_digest");
    save(&record_path, &legacy).unwrap();
    assert_eq!(
        err_code(
            &engine,
            json!({"command":"archive_inspect","job_id":receipt["id"],"archive_passphrase":PASS})
        ),
        "stale_archive"
    );
    legacy
        .as_object_mut()
        .unwrap()
        .remove("archive_intent_digest");
    save(&record_path, &legacy).unwrap();
    ok(
        &engine,
        json!({"command":"archive_inspect","job_id":receipt["id"],"archive_passphrase":PASS}),
    );
}

#[test]
fn cleanup_reopen_rejects_replacement_before_new_root_or_migration() {
    let (_temp, engine, environment, root) = fixture();
    seed_work(&root);
    let preview = ok(
        &engine,
        json!({"command":"plan_cleanup","environment_id":environment["id"],"recipe":"reset_client","writers_confirmed_stopped":true,"official_logout":false,"categories":["instructions"]}),
    );
    let plan = load(&engine.path("plans", preview["id"].as_str().unwrap())).unwrap();
    let journal = engine.path("jobs", preview["id"].as_str().unwrap());
    let mut receipt = json!({"id":preview["id"],"steps":[]});
    let request = json!({"archive_passphrase":PASS});
    engine
        .archive_work(&environment, &plan, &request, &mut receipt, &journal)
        .unwrap();
    let path = PathBuf::from(receipt["archive_path"].as_str().unwrap());
    let mut replacement = archive::unseal(&fs::read(&path).unwrap(), PASS).unwrap();
    let bytes: &[u8] = b"synthetic replacement instruction";
    replacement["files"][0]["data"] = json!(bytes);
    replacement["files"][0]["digest"] = json!(digest(bytes));
    atomic(&path, &archive::seal(&replacement, PASS).unwrap(), 0o600).unwrap();
    let failure = engine
        .migrate_to_new_root(&environment, &plan, &request, &mut receipt, &journal)
        .unwrap_err();
    assert_eq!(failure.code, "stale_archive");
    assert!(receipt.get("new_root").is_none());
    assert!(!receipt["steps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|step| step["id"] == "create" || step["id"] == "migrate"));
    assert_eq!(
        fs::read_to_string(root.join("CLAUDE.md")).unwrap(),
        "Synthetic instruction only."
    );
    assert!(root.join(".credentials.json").exists());
}

#[test]
fn portable_import_case_equivalence_rejects_before_content_writes() {
    let (_temp, engine, environment, root) = fixture();
    let marker = root.parent().unwrap().join("filesystem-case-check");
    fs::write(&marker, b"synthetic").unwrap();
    let folds_case = marker.with_file_name("FILESYSTEM-CASE-CHECK").exists();
    let package = json!({"schema":"lintel.work/1","files": [
        {"path":"projects/foo.jsonl","category":"sessions","data":b"lower","digest":digest(b"lower")},
        {"path":"projects/Foo.jsonl","category":"sessions","data":b"upper","digest":digest(b"upper")}
    ]});
    let archive_path = root.parent().unwrap().join("portable-case.age");
    atomic_new(
        &archive_path,
        &archive::seal(&package, PASS).unwrap(),
        0o600,
    )
    .unwrap();
    let plan = ok(
        &engine,
        json!({"command":"plan_import","environment_id":environment["id"],"archive_path":archive_path,"categories":["sessions"],"archive_passphrase":PASS}),
    );
    let receipt = ok(
        &engine,
        json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"],"archive_passphrase":PASS}),
    );
    if folds_case {
        assert_eq!(receipt["error"]["code"], "migration_path_conflict");
        assert_eq!(receipt["status"], "needs_reconciliation");
        assert!(!root.join("lintel-imports").exists());
    } else {
        assert_eq!(receipt["status"], "completed");
        assert_eq!(
            fs::read(root.join("lintel-imports/projects/foo.jsonl")).unwrap(),
            b"lower"
        );
        assert_eq!(
            fs::read(root.join("lintel-imports/projects/Foo.jsonl")).unwrap(),
            b"upper"
        );
    }
    assert!(fs::read_dir(&root).unwrap().all(|entry| !entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".lintel-path-check-")));
}

#[test]
fn portable_import_unwritable_parent_rejects_before_any_content_write() {
    let (_temp, engine, environment, root) = fixture();
    let package = json!({"schema":"lintel.work/1","files": [
        {"path":"CLAUDE.md","category":"instructions","data":b"first","digest":digest(b"first")},
        {"path":"projects/new.jsonl","category":"sessions","data":b"second","digest":digest(b"second")}
    ]});
    let archive_path = root.parent().unwrap().join("portable-permissions.age");
    atomic_new(
        &archive_path,
        &archive::seal(&package, PASS).unwrap(),
        0o600,
    )
    .unwrap();
    let parent = root.join("lintel-imports/projects");
    fs::create_dir_all(&parent).unwrap();
    let request = json!({"command":"plan_import","environment_id":environment["id"],"archive_path":archive_path,"categories":["instructions","sessions"],"archive_passphrase":PASS});
    let plan = ok(&engine, request.clone());
    let expected = if unsafe { libc::geteuid() } == 0 {
        std::os::unix::fs::chown(&parent, Some(65534), None).unwrap();
        "wrong_owner"
    } else {
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o500)).unwrap();
        "migration_destination_unwritable"
    };
    assert_eq!(err_code(&engine, request), expected);
    let receipt = ok(
        &engine,
        json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"],"archive_passphrase":PASS}),
    );
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
    if unsafe { libc::geteuid() } == 0 {
        std::os::unix::fs::chown(&parent, Some(0), None).unwrap();
    }
    assert!(
        !root.join("CLAUDE.md").exists(),
        "Known destination failure partially imported content"
    );
    assert_eq!(receipt["error"]["code"], expected);
    assert!(!parent.join("new.jsonl").exists());
    if fs::metadata("/").unwrap().uid() != unsafe { libc::geteuid() } {
        // Metadata-only exercise of the foreign-UID branch; no creation or
        // permission change occurs outside this test's temporary root.
        assert_eq!(
            work::preflight_import_parent(Path::new("/"), &root.join("metadata-only"))
                .unwrap_err()
                .code,
            "wrong_owner"
        );
    }
}

#[test]
fn portable_import_existing_ancestor_rejects_before_any_content_write() {
    let (_temp, engine, environment, root) = fixture();
    let package = json!({"schema":"lintel.work/1","files": [
        {"path":"CLAUDE.md","category":"instructions","data":b"first","digest":digest(b"first")},
        {"path":"projects/new.jsonl","category":"sessions","data":b"second","digest":digest(b"second")}
    ]});
    let archive_path = root.parent().unwrap().join("portable-ancestor.age");
    atomic_new(
        &archive_path,
        &archive::seal(&package, PASS).unwrap(),
        0o600,
    )
    .unwrap();
    let request = json!({"command":"plan_import","environment_id":environment["id"],"archive_path":archive_path,"categories":["instructions","sessions"],"archive_passphrase":PASS});
    let plan = ok(&engine, request.clone());
    fs::create_dir(root.join("lintel-imports")).unwrap();
    let blocker = root.join("lintel-imports/projects");
    fs::write(&blocker, b"existing ancestor file").unwrap();
    // The shared path guard inspects every actual ancestor before the import
    // loop, including a blocker introduced after preview.
    assert_eq!(err_code(&engine, request), "path_unreadable");
    let receipt = ok(
        &engine,
        json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"],"archive_passphrase":PASS}),
    );
    assert_eq!(receipt["status"], "needs_reconciliation");
    assert_eq!(receipt["error"]["code"], "path_unreadable");
    assert!(!root.join("CLAUDE.md").exists());
    assert_eq!(fs::read(&blocker).unwrap(), b"existing ancestor file");
    assert!(!receipt["steps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|step| step["id"] == "CLAUDE.md"));
    let queried = ok(&engine, json!({"command":"job","job_id":receipt["id"]}));
    assert_eq!(queried["id"], receipt["id"]);
    assert_eq!(queried["error"], receipt["error"]);
    assert!(!root.join("CLAUDE.md").exists());
}

#[test]
fn portable_import_preserves_existing_directory_modes_and_creates_private_parents() {
    let (_temp, engine, environment, root) = fixture();
    let imports = root.join("lintel-imports");
    let projects = imports.join("projects");
    fs::create_dir_all(&projects).unwrap();
    for (path, mode) in [(&root, 0o755), (&imports, 0o775), (&projects, 0o750)] {
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }
    let package = json!({"schema":"lintel.work/1","files": [
        {"path":"CLAUDE.md","category":"instructions","data":b"instruction","digest":digest(b"instruction")},
        {"path":"projects/new.jsonl","category":"sessions","data":b"session","digest":digest(b"session")},
        {"path":"projects/example/memory/MEMORY.md","category":"memory","data":b"memory","digest":digest(b"memory")}
    ]});
    let archive_path = root.parent().unwrap().join("portable-modes.age");
    atomic_new(
        &archive_path,
        &archive::seal(&package, PASS).unwrap(),
        0o600,
    )
    .unwrap();
    let plan = ok(
        &engine,
        json!({"command":"plan_import","environment_id":environment["id"],"archive_path":archive_path,"categories":["instructions","memory","sessions"],"archive_passphrase":PASS}),
    );
    let receipt = ok(
        &engine,
        json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"],"archive_passphrase":PASS}),
    );
    assert_eq!(receipt["status"], "completed");
    for (path, mode) in [(&root, 0o755), (&imports, 0o775), (&projects, 0o750)] {
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            mode,
            "{}",
            path.display()
        );
    }
    for path in [projects.join("example"), projects.join("example/memory")] {
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o700,
            "{}",
            path.display()
        );
    }
    for (relative, bytes) in [
        ("CLAUDE.md", b"instruction".as_slice()),
        ("lintel-imports/projects/new.jsonl", b"session".as_slice()),
        (
            "lintel-imports/projects/example/memory/MEMORY.md",
            b"memory".as_slice(),
        ),
    ] {
        let path = root.join(relative);
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

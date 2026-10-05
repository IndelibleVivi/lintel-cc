//! Baseline backend regressions: frozen new targets, structured results,
//! bounded session reading and the finite launch/resume paths. All fixtures are
//! synthetic temporary roots.
use crate::*;
use serde_json::{json, Value};
use std::fs;
use tempfile::TempDir;

const PASS: &str = "synthetic-baseline-passphrase";

fn setup() -> (TempDir, Engine, Value, std::path::PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let home = base.join("home");
    fs::create_dir(&home).unwrap();
    let root = home.join("cc");
    fs::create_dir(&root).unwrap();
    let mut engine = Engine::new(home, base.join("state")).unwrap();
    engine.launch_opener = Some(|path, content| {
        assert_eq!(fs::read_to_string(path)?, content);
        assert!(content.contains("CLAUDE_CONFIG_DIR="));
        assert!(content.contains("cd "));
        Ok(json!({"status":"launch_requested","message":"synthetic test opener"}))
    });
    let e = engine.register("fixture", &root, false).unwrap();
    (temp, engine, e, root)
}

fn data(engine: &Engine, request: Value) -> Value {
    let response = engine.request(request);
    assert_eq!(response["ok"], true, "{response}");
    response["data"].clone()
}

/// Install an inert synthetic Claude at this Engine home's native install path
/// (`~/.local/bin/claude`) and re-register the environment, so static discovery
/// resolves it without executing anything. Production discovery precedence is
/// unchanged; only the fixture home differs.
fn with_inert_claude(engine: &Engine, e: &Value, root: &std::path::Path) -> Value {
    let home = root.parent().unwrap();
    let native = home.join(".local/bin/claude");
    fs::create_dir_all(native.parent().unwrap()).unwrap();
    fs::write(&native, "#!/bin/sh\nexit 0\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&native, fs::Permissions::from_mode(0o700)).unwrap();
    // `discover` re-runs static discovery over the inventory; with the fixture
    // native install present it records the resolved executable.
    let _ = data(engine, json!({"command":"discover"}));
    engine.env(&json!({"environment_id": e["id"]})).unwrap()
}

/// Install an inert synthetic Claude at a **versioned native path**
/// (`<home>/.claude/versions/<version>/claude`) so static metadata reports a
/// declared version. Never executed.
fn with_versioned_claude(
    engine: &Engine,
    e: &Value,
    root: &std::path::Path,
    version: &str,
) -> Value {
    let home = root.parent().unwrap();
    // The real native layout is `<config>/versions/<version>`; the discovery
    // fallback is `~/.local/bin/claude`. A symlink makes discovery resolve to the
    // versioned path so static metadata reports a declared version.
    let versioned = home.join(format!("claude/versions/{version}"));
    fs::create_dir_all(versioned.parent().unwrap()).unwrap();
    fs::write(&versioned, "#!/bin/sh\nexit 0\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&versioned, fs::Permissions::from_mode(0o700)).unwrap();
    let link = home.join(".local/bin/claude");
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    let _ = fs::remove_file(&link);
    std::os::unix::fs::symlink(&versioned, &link).unwrap();
    let _ = data(engine, json!({"command":"discover"}));
    engine.env(&json!({"environment_id": e["id"]})).unwrap()
}

#[test]
fn preserve_plan_freezes_target_and_publishes_manifest_without_creating_root() {
    let (_t, engine, e, root) = setup();
    fs::write(root.join("CLAUDE.md"), "instruction").unwrap();
    fs::create_dir_all(root.join("projects/p/memory")).unwrap();
    fs::write(root.join("projects/p/memory/M.md"), "memory").unwrap();
    let plan = data(
        &engine,
        json!({"command":"plan_preserve","environment_id":e["id"],"categories":["instructions","memory"]}),
    );
    assert_eq!(plan["plan_revision"], "lintel.plan/2");
    let target = &plan["planned_target"];
    assert_eq!(target["create"], true);
    let new_root = target["new_root"].as_str().unwrap();
    assert!(
        !std::path::Path::new(new_root).exists(),
        "preview created the root"
    );
    assert_eq!(target["counts"]["instructions"], 1);
    assert_eq!(target["counts"]["memory"], 1);
    assert_eq!(target["purposes"]["instructions"], "instructions");
    assert_eq!(target["purposes"]["memory"], "reference");
    // Legacy default: instructions active, memory not.
    assert_eq!(target["activation"]["instructions"], true);
    assert_eq!(target["activation"]["memory"], false);
    // Destinations are the exact absolute final files.
    for file in target["files"].as_array().unwrap() {
        let dest = file["destination"].as_str().unwrap();
        assert!(dest.starts_with(new_root), "{dest}");
    }
    assert!(plan.get("root").is_none(), "public plan must not leak root");
    assert!(
        plan.get("extra").is_none(),
        "public plan must not leak extra"
    );
}

#[test]
fn occupied_planned_root_is_stale_plan_before_any_side_effect() {
    let (_t, engine, e, root) = setup();
    fs::write(root.join("CLAUDE.md"), "instruction").unwrap();
    let plan = data(
        &engine,
        json!({"command":"plan_preserve","environment_id":e["id"],"categories":["instructions"]}),
    );
    // Externally occupy the planned root after preview.
    let new_root = plan["planned_target"]["new_root"]
        .as_str()
        .unwrap()
        .to_string();
    fs::create_dir_all(&new_root).unwrap();
    fs::write(std::path::Path::new(&new_root).join("occupier"), "x").unwrap();
    let response = engine.request(json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"],"archive_passphrase":PASS}));
    assert_eq!(response["error"]["code"], "stale_plan", "{response}");
    // No new root was created by us; the occupier is untouched.
    assert!(std::path::Path::new(&new_root).join("occupier").exists());
    assert!(root.join("CLAUDE.md").exists());
}

#[test]
fn preserve_creates_the_frozen_root_only_on_execute() {
    let (_t, engine, e, root) = setup();
    fs::write(root.join("CLAUDE.md"), "instruction").unwrap();
    let plan = data(
        &engine,
        json!({"command":"plan_preserve","environment_id":e["id"],"categories":["instructions"]}),
    );
    let planned = plan["planned_target"]["new_root"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(!std::path::Path::new(&planned).exists());
    let receipt = data(
        &engine,
        json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"],"archive_passphrase":PASS}),
    );
    assert_eq!(
        receipt["new_root"], planned,
        "created root differs from the frozen one"
    );
    assert_eq!(
        receipt["new_environment_id"],
        plan["planned_target"]["new_environment_id"]
    );
    assert_eq!(receipt["task_result"]["outcome"], "completed");
    assert!(receipt["task_result"]["selected_steps"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["done"] == true));
}

#[test]
fn legacy_plan_without_frozen_target_is_stale_plan() {
    let (_t, engine, e, root) = setup();
    fs::write(root.join("CLAUDE.md"), "instruction").unwrap();
    let plan = data(
        &engine,
        json!({"command":"plan_reset","environment_id":e["id"],"recipe":"rebuild","categories":["instructions"]}),
    );
    // Strip the frozen target, simulating a pre-freeze legacy plan, and re-hash.
    let path = engine.path("plans", string(&plan, "id").unwrap());
    let mut stored = load(&path).unwrap();
    stored["extra"]
        .as_object_mut()
        .unwrap()
        .remove("frozen_target");
    stored.as_object_mut().unwrap().remove("hash");
    stored["hash"] = json!(digest(&serde_json::to_vec(&stored).unwrap()));
    save(&path, &stored).unwrap();
    let response = engine.request(json!({"command":"execute","plan_id":stored["id"],"approval":stored["hash"],"archive_passphrase":PASS}));
    assert_eq!(response["error"]["code"], "stale_plan", "{response}");
    // No root created.
    assert_eq!(
        fs::read_dir(engine.state.join("environments"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn local_cleanup_success_reports_selected_completion_and_coverage() {
    let (_t, engine, e, root) = setup();
    fs::write(root.join(".credentials.json"), "synthetic").unwrap();
    let plan = data(
        &engine,
        json!({"command":"plan_cleanup","environment_id":e["id"],"recipe":"repair_login","writers_confirmed_stopped":true,"official_logout":false}),
    );
    let receipt = data(
        &engine,
        json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"]}),
    );
    assert_eq!(receipt["status"], "partially_completed");
    let result = &receipt["task_result"];
    assert_eq!(result["primary"], "所选本地清理已完成", "{receipt}");
    let coverage = result["coverage"].as_array().unwrap();
    assert!(
        coverage
            .iter()
            .any(|c| c["scope"] == "official_logout" && c["state"] == "not_requested"),
        "{receipt}"
    );
    assert!(
        coverage
            .iter()
            .any(|c| c["scope"] == "server_state" && c["state"] == "not_checked"),
        "{receipt}"
    );
    // Receipt and job query agree on the structured result.
    let queried = data(&engine, json!({"command":"job","job_id":receipt["id"]}));
    assert_eq!(queried["task_result"], receipt["task_result"]);
}

#[test]
fn session_read_pages_bounded_and_binds_digest() {
    let (_t, engine, e, root) = setup();
    // Build a synthetic JSONL with a known record, an unknown record and an
    // opaque thinking block, plus a non-ASCII line.
    let mut body = String::new();
    body.push_str("{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"你好世界\"}]}}\n");
    body.push_str("{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"thinking\",\"thinking\":\"secret\",\"signature\":\"sig\"},{\"type\":\"text\",\"text\":\"answer\"}]}}\n");
    body.push_str("{\"type\":\"totally-unknown\",\"payload\":{\"k\":1}}\n");
    body.push_str("not json at all\n");
    fs::create_dir_all(root.join("projects/p")).unwrap();
    fs::write(root.join("projects/p/s.jsonl"), &body).unwrap();
    let archive = data(
        &engine,
        json!({"command":"plan_archive","environment_id":e["id"],"categories":["sessions"]}),
    );
    let receipt = data(
        &engine,
        json!({"command":"execute","plan_id":archive["id"],"approval":archive["hash"],"archive_passphrase":PASS}),
    );
    assert_eq!(receipt["status"], "completed");
    let manifest = data(
        &engine,
        json!({"command":"archive_inspect","job_id":receipt["id"],"archive_passphrase":PASS}),
    );
    let file = &manifest["files"][0];
    let digest = file["digest"].as_str().unwrap().to_string();
    let page = data(
        &engine,
        json!({"command":"session_read","job_id":receipt["id"],"archive_passphrase":PASS,"path":file["path"],"expected_digest":digest}),
    );
    assert_eq!(page["done"], true);
    assert_eq!(page["digest"], digest);
    assert_eq!(
        page["source"]["package_digest"],
        digest_of_package(&receipt)
    );
    let records = page["records"].as_array().unwrap();
    assert!(records
        .iter()
        .any(|r| r["kind"] == "user" && r["text"] == "你好世界"));
    assert!(
        records.iter().any(|r| r["opaque"] == true),
        "opaque thinking must be visible"
    );
    assert!(
        !records.iter().any(|r| r["text"] == "secret"),
        "opaque thinking must not leak"
    );
    assert!(
        records.iter().any(|r| r["unknown"] == true),
        "unknown records must stay visible"
    );
    // A digest that does not match is stale.
    let stale = engine.request(json!({"command":"session_read","job_id":receipt["id"],"archive_passphrase":PASS,"path":file["path"],"expected_digest":"a".repeat(64)}));
    assert_eq!(stale["error"]["code"], "stale_archive", "{stale}");
}

fn digest_of_package(receipt: &Value) -> String {
    receipt["archive_digest"].as_str().unwrap().to_string()
}

#[test]
fn session_read_normalizes_string_content_and_unknown_shape() {
    let (_t, engine, e, root) = setup();
    fs::create_dir_all(root.join("projects/p")).unwrap();
    // Common user JSONL uses a plain string content; ensure it is visible, not
    // classified as unknown.
    let body = concat!(
        "{\"type\":\"user\",\"timestamp\":\"2026-01-01T00:00:00Z\",\"message\":{\"content\":\"plain string content\"}}\n",
        "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"rich\"}]}}\n",
        "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"tool_use\",\"name\":\"Read\",\"input\":{\"file\":\"x\"}}]}}\n",
        "{\"type\":\"assistant\",\"message\":{\"content\":[{\"type\":\"thinking\",\"thinking\":\"opaque\",\"signature\":\"s\"}]}}\n",
    );
    fs::write(root.join("projects/p/s.jsonl"), body).unwrap();
    let archive = data(
        &engine,
        json!({"command":"plan_archive","environment_id":e["id"],"categories":["sessions"]}),
    );
    let receipt = data(
        &engine,
        json!({"command":"execute","plan_id":archive["id"],"approval":archive["hash"],"archive_passphrase":PASS}),
    );
    let page = data(
        &engine,
        json!({"command":"session_read","job_id":receipt["id"],"archive_passphrase":PASS,"path":"projects/p/s.jsonl"}),
    );
    let records = page["records"].as_array().unwrap();
    assert!(
        records
            .iter()
            .any(|r| r["kind"] == "user" && r["text"] == "plain string content"),
        "{page}"
    );
    assert!(records
        .iter()
        .any(|r| r["kind"] == "assistant" && r["text"] == "rich"));
    assert!(records
        .iter()
        .any(|r| r["kind"] == "tool_call" && r["name"] == "Read"));
    assert!(records.iter().any(|r| r["opaque"] == true));
    // Timestamp is surfaced for UI hierarchy on the string-content record.
    let user = records.iter().find(|r| r["kind"] == "user").unwrap();
    assert_eq!(user["timestamp"], "2026-01-01T00:00:00Z");
    // Raw bytes are preserved verbatim.
    assert_eq!(page["raw_text"], body);
}

#[test]
fn session_read_pages_entire_corpus_without_losing_records() {
    let (_t, engine, e, root) = setup();
    fs::create_dir_all(root.join("projects/p")).unwrap();
    // A corpus larger than one page, with a mix of ASCII, multibyte and a long
    // line, so paging must advance by complete lines and never lose or repeat.
    let mut body = String::new();
    let mut expected = vec![];
    for index in 0..(crate::session::PAGE_BYTES / 64 + 40) {
        let text = format!("记录 {index} 数据 padding-{index}");
        expected.push(text.clone());
        body.push_str(&format!(
            "{{\"type\":\"user\",\"message\":{{\"content\":\"{text}\"}}}}\n"
        ));
    }
    fs::write(root.join("projects/p/s.jsonl"), &body).unwrap();
    let archive = data(
        &engine,
        json!({"command":"plan_archive","environment_id":e["id"],"categories":["sessions"]}),
    );
    let receipt = data(
        &engine,
        json!({"command":"execute","plan_id":archive["id"],"approval":archive["hash"],"archive_passphrase":PASS}),
    );
    let mut offset = 0u64;
    let mut collected = vec![];
    let mut pages = 0;
    loop {
        let page = data(
            &engine,
            json!({"command":"session_read","job_id":receipt["id"],"archive_passphrase":PASS,"path":"projects/p/s.jsonl","offset":offset}),
        );
        pages += 1;
        assert!(pages < 1000, "paging did not terminate");
        for record in page["records"].as_array().unwrap() {
            if let Some(text) = record["text"].as_str() {
                collected.push(text.to_string());
            }
        }
        if page["done"] == true {
            break;
        }
        let next = page["next_offset"].as_u64().unwrap();
        assert!(
            next > offset,
            "paging did not make forward progress: {offset} -> {next}"
        );
        offset = next;
    }
    assert_eq!(
        collected, expected,
        "paged reading lost or reordered records"
    );
}

#[test]
fn launch_plan_freezes_one_id_and_request_replay_is_query_only() {
    let (_t, engine, registered, root) = setup();
    let e = with_inert_claude(&engine, &registered, &root);
    let project = root.parent().unwrap().join("project");
    fs::create_dir(&project).unwrap();
    let plan = data(
        &engine,
        json!({"command":"plan_launch","environment_id":e["id"],"project_cwd":project,"mode":"interactive"}),
    );
    // The frozen request id equals the plan id: one immutable ID.
    assert_eq!(plan["launch_request"]["id"], plan["id"]);
    assert_eq!(
        plan["launch_request"]["project_cwd"],
        project.to_string_lossy().as_ref()
    );
    // A non-TTY core-json launch must not silently succeed; it returns the
    // platform's terminal requirement (macOS opens Terminal, Linux does not).
    let first = engine.request(
        json!({"command":"launch_request","request_id":plan["id"],"approval":plan["hash"]}),
    );
    // The macOS opener is inert; Linux refuses the missing TTY. In either
    // case exactly one attempt is recorded under the durable intent.
    if cfg!(target_os = "linux") {
        assert_eq!(first["error"]["code"], "terminal_required", "{first}");
    }
    let recorded_path = engine
        .state
        .join("launches")
        .join(format!("{}.json", plan["id"].as_str().unwrap()));
    assert!(recorded_path.exists(), "no durable launch intent recorded");
    let recorded: Value = load(&recorded_path).unwrap();
    // Second call is a pure query over the same record: same status, marked replay.
    let second = engine.request(
        json!({"command":"launch_request","request_id":plan["id"],"approval":plan["hash"]}),
    );
    assert_eq!(second["data"]["replayed_query"], true, "{second}");
    assert_eq!(second["data"]["status"], recorded["status"], "{second}");
    assert_eq!(second["data"]["request_id"], plan["id"]);
}

#[test]
fn launch_rejects_wrong_approval_and_missing_project() {
    let (_t, engine, registered, root) = setup();
    let e = with_inert_claude(&engine, &registered, &root);
    let project = root.parent().unwrap().join("project");
    fs::create_dir(&project).unwrap();
    let bad_project = data(
        &engine,
        json!({"command":"plan_launch","environment_id":e["id"],"project_cwd":project,"mode":"interactive"}),
    );
    let mismatch = engine.request(json!({"command":"launch_request","request_id":bad_project["id"],"approval":"a".repeat(64)}));
    assert_eq!(mismatch["error"]["code"], "approval_mismatch", "{mismatch}");
    let missing = engine.request(json!({"command":"plan_launch","environment_id":e["id"],"project_cwd":"/definitely/not/here","mode":"interactive"}));
    assert_eq!(missing["error"]["code"], "project_missing", "{missing}");
}

#[test]
fn resume_rejects_unknown_and_prepares_private_copy_for_known_format() {
    let (_t, engine, registered, root) = setup();
    let e = with_versioned_claude(&engine, &registered, &root, "2.1.285");
    let project = root.parent().unwrap().join("project");
    fs::create_dir(&project).unwrap();
    // Known transcript + unknown file in one archive.
    fs::create_dir_all(root.join("projects/p")).unwrap();
    fs::write(
        root.join("projects/p/s.jsonl"),
        "{\"type\":\"user\",\"sessionId\":\"s\",\"cwd\":\"/p\",\"uuid\":\"u\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"hi\"}]}}\n",
    )
    .unwrap();
    fs::write(root.join("CLAUDE.md"), "just instruction text").unwrap();
    let archive = data(
        &engine,
        json!({"command":"plan_archive","environment_id":e["id"],"categories":["sessions","instructions"]}),
    );
    let receipt = data(
        &engine,
        json!({"command":"execute","plan_id":archive["id"],"approval":archive["hash"],"archive_passphrase":PASS}),
    );
    // Unknown member: specific unsupported reason, never fake green.
    let unknown = data(
        &engine,
        json!({"command":"plan_resume","environment_id":e["id"],"project_cwd":project,"job_id":receipt["id"],"archive_passphrase":PASS,"path":"CLAUDE.md"}),
    );
    assert_eq!(unknown["resume"]["supported"], false);
    assert!(unknown["resume"]["reason"].is_string(), "{unknown}");
    // Known transcript: supported, one immutable id, private copy planned.
    let known = data(
        &engine,
        json!({"command":"plan_resume","environment_id":e["id"],"project_cwd":project,"job_id":receipt["id"],"archive_passphrase":PASS,"path":"projects/p/s.jsonl"}),
    );
    assert_eq!(known["resume"]["supported"], true, "{known}");
    assert_eq!(known["resume"]["archive_unmodified"], true);
    assert_eq!(known["resume"]["auth_unverified"], true);
    let copy = known["resume"]["private_copy_path"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(!std::path::Path::new(&copy).exists());
    // The attempt may open Terminal (launch_requested) or fail in a sandbox; the
    // private copy and durable record exist either way, and the archive original
    // is untouched. The copy must be 0600 and byte-identical to the source.
    let _attempt = engine.request(json!({"command":"resume_request","request_id":known["id"],"approval":known["hash"],"archive_passphrase":PASS}));
    assert!(
        std::path::Path::new(&copy).exists(),
        "private running copy missing"
    );
    let record: Value = load(
        &engine
            .state
            .join("launches")
            .join(format!("{}.json", known["id"].as_str().unwrap())),
    )
    .unwrap();
    assert!(
        matches!(
            record["status"].as_str(),
            Some("launch_requested" | "resume_prepared" | "launch_failed")
        ),
        "{record}"
    );
    assert_eq!(record["private_copy_path"], copy);
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        std::fs::metadata(&copy).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        std::fs::read(&copy).unwrap(),
        std::fs::read(root.join("projects/p/s.jsonl")).unwrap(),
        "private copy differs from the archived transcript"
    );
    // A repeat is a query-only replay and never re-runs the Terminal attempt.
    let replay = engine.request(json!({"command":"resume_request","request_id":known["id"],"approval":known["hash"],"archive_passphrase":PASS}));
    assert_eq!(replay["data"]["replayed_query"], true, "{replay}");
    // The archive original is unchanged.
    let manifest = data(
        &engine,
        json!({"command":"archive_inspect","job_id":receipt["id"],"archive_passphrase":PASS}),
    );
    assert_eq!(manifest["files"].as_array().unwrap().len(), 2);
}

#[test]
fn activation_false_keeps_instructions_in_reference_area() {
    let (_t, engine, e, root) = setup();
    fs::write(root.join("CLAUDE.md"), "instruction-bytes").unwrap();
    let plan = data(
        &engine,
        json!({"command":"plan_preserve","environment_id":e["id"],"categories":["instructions"],"activate":{"instructions":false}}),
    );
    // The published manifest maps CLAUDE.md into the reference area, not the root.
    let dest = plan["planned_target"]["files"][0]["destination"]
        .as_str()
        .unwrap();
    assert!(dest.contains("lintel-imports"), "{dest}");
    assert!(dest.ends_with("lintel-imports/CLAUDE.md"), "{dest}");
    assert_eq!(plan["planned_target"]["activation"]["instructions"], false);
    assert_eq!(
        plan["planned_target"]["purposes"]["instructions"],
        "instructions"
    );
    let receipt = data(
        &engine,
        json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"],"archive_passphrase":PASS}),
    );
    assert_eq!(receipt["status"], "completed", "{receipt}");
    let new_root = std::path::PathBuf::from(receipt["new_root"].as_str().unwrap());
    assert!(
        !new_root.join("CLAUDE.md").exists(),
        "inactive instructions landed at root"
    );
    assert_eq!(
        fs::read(new_root.join("lintel-imports/CLAUDE.md")).unwrap(),
        b"instruction-bytes"
    );
}

#[test]
fn activation_true_places_instructions_at_root_with_exact_bytes() {
    let (_t, engine, e, root) = setup();
    fs::write(root.join("CLAUDE.md"), "active-bytes").unwrap();
    let plan = data(
        &engine,
        json!({"command":"plan_preserve","environment_id":e["id"],"categories":["instructions"],"activate":{"instructions":true}}),
    );
    let dest = plan["planned_target"]["files"][0]["destination"]
        .as_str()
        .unwrap();
    assert!(
        dest.ends_with("/CLAUDE.md") && !dest.contains("lintel-imports"),
        "{dest}"
    );
    let receipt = data(
        &engine,
        json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"],"archive_passphrase":PASS}),
    );
    let new_root = std::path::PathBuf::from(receipt["new_root"].as_str().unwrap());
    assert_eq!(
        fs::read(new_root.join("CLAUDE.md")).unwrap(),
        b"active-bytes"
    );
}

#[test]
fn launch_rechecks_stale_project_and_executable_before_intent() {
    let (_t, engine, registered, root) = setup();
    let e = with_inert_claude(&engine, &registered, &root);
    let project = root.parent().unwrap().join("project");
    fs::create_dir(&project).unwrap();
    let plan = data(
        &engine,
        json!({"command":"plan_launch","environment_id":e["id"],"project_cwd":project,"mode":"interactive"}),
    );
    // Now replace the project directory object (same path, new inode).
    fs::remove_dir(&project).unwrap();
    fs::create_dir(&project).unwrap();
    let request_id = plan["id"].as_str().unwrap().to_string();
    let result = engine.request(
        json!({"command":"launch_request","request_id":request_id,"approval":plan["hash"]}),
    );
    assert_eq!(result["error"]["code"], "stale_plan", "{result}");
    // No durable intent was written: no attempt happened.
    assert!(!engine
        .state
        .join("launches")
        .join(format!("{}.json", plan["id"].as_str().unwrap()))
        .exists());
}

#[test]
fn launch_query_and_list_are_readonly_and_find_request() {
    let (_t, engine, registered, root) = setup();
    let e = with_inert_claude(&engine, &registered, &root);
    let project = root.parent().unwrap().join("project");
    fs::create_dir(&project).unwrap();
    let plan = data(
        &engine,
        json!({"command":"plan_launch","environment_id":e["id"],"project_cwd":project,"mode":"interactive"}),
    );
    // Before any attempt, launch_query finds the frozen plan metadata only.
    let planned = data(
        &engine,
        json!({"command":"launch_query","request_id":plan["id"]}),
    );
    assert_eq!(planned["status"], "planned");
    assert_eq!(planned["observed"], "plan");
    assert_eq!(planned["request_id"], plan["id"]);
    assert_eq!(planned["root"], plan["launch_request"]["config_root"]);
    // Make an attempt (may fail in a sandbox), then query the durable record.
    let _ = engine.request(
        json!({"command":"launch_request","request_id":plan["id"],"approval":plan["hash"]}),
    );
    let observed = data(
        &engine,
        json!({"command":"launch_query","request_id":plan["id"]}),
    );
    assert_eq!(observed["observed"], "record");
    assert_eq!(observed["replayed_query"], true);
    // No secret/body leaks in the projected view.
    let encoded = serde_json::to_string(&observed).unwrap();
    assert!(!encoded.contains("archive_passphrase"));
    let list = data(&engine, json!({"command":"launches"}));
    assert!(list["launches"]
        .as_array()
        .unwrap()
        .iter()
        .any(|l| l["request_id"] == plan["id"]));
}

#[test]
fn legacy_portable_import_keeps_original_instruction_destination() {
    let (_t, engine, source, root) = setup();
    fs::write(root.join("CLAUDE.md"), b"legacy instruction bytes").unwrap();
    let archive = data(
        &engine,
        json!({"command":"plan_archive","environment_id":source["id"],"categories":["instructions"]}),
    );
    let receipt = data(
        &engine,
        json!({"command":"execute","plan_id":archive["id"],"approval":archive["hash"],"archive_passphrase":PASS}),
    );
    let target_root = root.parent().unwrap().join("import-target");
    fs::create_dir(&target_root).unwrap();
    let target = engine
        .register("old import target", &target_root, false)
        .unwrap();
    let plan = data(
        &engine,
        json!({"command":"plan_import","environment_id":target["id"],"job_id":receipt["id"],"archive_passphrase":PASS,"categories":["instructions"]}),
    );
    // Simulate a stored protocol-1 plan from before the additive fields, while
    // retaining its approved absolute destination and original ID.
    let plan_path = engine.path("plans", plan["id"].as_str().unwrap());
    let mut legacy = load(&plan_path).unwrap();
    for key in ["activate", "frozen_target", "work_purpose"] {
        legacy["extra"].as_object_mut().unwrap().remove(key);
    }
    legacy.as_object_mut().unwrap().remove("hash");
    legacy["hash"] = json!(digest(&serde_json::to_vec(&legacy).unwrap()));
    save(&plan_path, &legacy).unwrap();
    let imported = data(
        &engine,
        json!({"command":"execute","plan_id":plan["id"],"approval":legacy["hash"],"archive_passphrase":PASS}),
    );
    assert_eq!(imported["id"], plan["id"]);
    assert_eq!(
        fs::read(target_root.join("CLAUDE.md")).unwrap(),
        b"legacy instruction bytes"
    );
    assert!(!target_root.join("lintel-imports/CLAUDE.md").exists());
    let rejected =
        engine.request(json!({"command":"execute","plan_id":plan["id"],"approval":"wrong"}));
    assert_eq!(rejected["error"]["code"], "approval_mismatch");
    let query = data(&engine, json!({"command":"job","job_id":plan["id"]}));
    assert_eq!(query["id"], plan["id"]);
    let replay = data(
        &engine,
        json!({"command":"execute","plan_id":plan["id"],"approval":legacy["hash"]}),
    );
    assert_eq!(replay["id"], plan["id"]);
    assert_eq!(replay["status"], imported["status"]);
}

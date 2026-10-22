//! Synthetic regression tests for the independent `lintel.residue/1` reader and
//! restore path. Every fixture is a temporary directory; no real backup, secret
//! or personal path is read or written, and no process/service is touched.
use super::*;
use crate::archive::seal;
use crate::storage::{load, save};

const PASS: &str = "synthetic residue archive passphrase";
const OTHER_PASS: &str = "a different synthetic passphrase";

#[test]
fn hostile_residue_admission_stops_before_cipher_or_byte_array_allocation() {
    let fx = fixture();
    let oversized = fx.engine.state.join("archives/oversized.age");
    let file = fs::File::create(&oversized).unwrap();
    file.set_len(cipher_bound(RESIDUE_MAX_FILES, RESIDUE_TOTAL_LIMIT) + 1)
        .unwrap();
    assert_eq!(
        read_cipher(&oversized).unwrap_err().code,
        "residue_archive_limit"
    );
    let item = json!({"path":"/synthetic/original","role":"residue","bytes":0,"digest":digest(b""),"data":[]});
    let valid = json!({"schema":RESIDUE_SCHEMA,"files":[item.clone()]});
    assert!(serde_json::from_value::<DecodedPackage>(valid.clone()).is_ok());
    let mut too_many = valid.clone();
    too_many["files"] = json!(vec![item; RESIDUE_MAX_FILES + 1]);
    assert!(serde_json::from_value::<DecodedPackage>(too_many).is_err());
    // The finite decoder, unlike a generic Value array, refuses a ninth MiB
    // while reading the byte sequence. No archive or file publication occurs.
    let data = "0,".repeat(RESIDUE_FILE_LIMIT as usize) + "0";
    let text=format!("{{\"schema\":\"{}\",\"files\":[{{\"path\":\"/synthetic/original\",\"bytes\":{},\"digest\":\"{}\",\"data\":[{}]}}]}}",RESIDUE_SCHEMA,RESIDUE_FILE_LIMIT+1,digest(b""),data);
    assert!(serde_json::from_str::<DecodedPackage>(&text).is_err());
    assert!(serde_json::from_str::<DecodedPackage>(
        "{\"schema\":\"lintel.residue/1\",\"schema\":\"lintel.work/1\",\"files\":[]}"
    )
    .is_err());
    assert!(serde_json::from_str::<DecodedPackage>(&(valid.to_string() + " trailing")).is_err());
}

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    engine: Engine,
    environment_id: Value,
}

fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let home = base.join("home");
    let root = home.join("cc");
    fs::create_dir_all(&root).unwrap();
    let engine = Engine::new(home.clone(), base.join("state")).unwrap();
    let environment_id = engine.register("fixture", &root, false).unwrap();
    Fixture {
        _temp: temp,
        root,
        engine,
        environment_id,
    }
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn frozen(engine: &Engine, public: &Value) -> Value {
    let id = public["id"].as_str().unwrap();
    load(&engine.state.join("plans").join(format!("{id}.json"))).unwrap()
}

/// Write one `lintel.residue/1` package holding the given (path, role, bytes)
/// entries, exactly as the local-data recipes would.
fn seal_package(engine: &Engine, entries: &[(&Path, &str, &[u8])], pass: &str) -> PathBuf {
    let files: Vec<Value> = entries
        .iter()
        .map(|(path, role, data)| {
            json!({
                "path": path,
                "role": role,
                "bytes": data.len(),
                "digest": digest(data),
                "data": data,
            })
        })
        .collect();
    let package = json!({
        "schema": "lintel.residue/1",
        "generator": "Lintel",
        "created_at": now(),
        "environment_id": engine.inventory().unwrap()[0]["id"],
        "files": files,
    });
    let path = engine
        .state
        .join("archives")
        .join(format!("{}.age", uuid::Uuid::new_v4()));
    fs::write(&path, seal(&package, pass).unwrap()).unwrap();
    path
}

fn restore_plan(fixture: &Fixture, archive: &Path, selected: &[&Path]) -> Value {
    let selected: Vec<String> = selected
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    fixture
        .engine
        .plan_residue_restore(&json!({
            "environment_id": fixture.environment_id["id"],
            "archive_path": archive.to_string_lossy(),
            "archive_passphrase": PASS,
            "selected_paths": selected,
        }))
        .unwrap()
}

fn run_restore(fixture: &Fixture, plan: &Value) -> Value {
    let id = plan["id"].as_str().unwrap();
    let saved = frozen(&fixture.engine, plan);
    let env = fixture
        .engine
        .env(&json!({"environment_id": saved["environment_id"]}))
        .unwrap();
    let r = json!({"archive_passphrase": PASS});
    fixture.engine.check_residue_restore(&saved, &r).unwrap();
    let journal = fixture.engine.state.join("jobs").join(format!("{id}.json"));
    let mut receipt = json!({"id":id,"plan_id":id,"status":"executing","steps":[],"warnings":[]});
    save(&journal, &receipt).unwrap();
    fixture
        .engine
        .execute_residue_restore(&env, &saved, &r, &mut receipt, &journal)
        .unwrap();
    receipt
}

#[test]
fn unbound_residue_packages_are_readable_but_cannot_authorize_restoration() {
    let fx = fixture();
    let original = fx.root.join("debug/absent.log");
    fs::create_dir_all(original.parent().unwrap()).unwrap();
    for environment_id in [Value::Null, json!("not-a-valid-environment-id")] {
        let mut package = json!({
            "schema": RESIDUE_SCHEMA,
            "files": [{"path": original, "role": "residue", "bytes": 3,
                "digest": digest(b"old"), "data": b"old"}],
        });
        if !environment_id.is_null() {
            package["environment_id"] = environment_id;
        }
        let archive = fx
            .engine
            .state
            .join("archives")
            .join(format!("{}.age", uuid::Uuid::new_v4()));
        fs::write(&archive, seal(&package, PASS).unwrap()).unwrap();
        let request = json!({"archive_path": archive, "archive_passphrase": PASS,
            "environment_id": fx.environment_id["id"], "selected_paths": [original]});
        let report = fx.engine.residue_archive_inspect(&request).unwrap();
        assert_eq!(report["file_count"], 1);
        assert_eq!(report["files"][0]["restore_available"], false);
        assert_eq!(
            fx.engine.plan_residue_restore(&request).unwrap_err().code,
            "environment_binding_missing"
        );
        assert!(!original.exists());
    }
}

#[test]
fn residue_inspect_returns_a_body_free_manifest() {
    let fx = fixture();
    let original = fx.root.join("debug/old.log");
    let archive = seal_package(&fx.engine, &[(&original, "residue", b"12345")], PASS);
    let report = fx
        .engine
        .residue_archive_inspect(&json!({
            "archive_path": archive.to_string_lossy(),
            "archive_passphrase": PASS,
        }))
        .unwrap();
    assert_eq!(report["schema"], "lintel.residue/1");
    assert_eq!(report["file_count"], 1);
    assert_eq!(report["total_bytes"], 5);
    let file = &report["files"][0];
    assert_eq!(file["path"], original.to_string_lossy().as_ref());
    assert_eq!(file["bytes"], 5);
    assert_eq!(file["digest"], digest(b"12345"));
    // A manifest never carries file bodies.
    let text = serde_json::to_string(&report).unwrap();
    assert!(!text.contains("data"), "{text}");
    assert!(!text.contains("12345"), "{text}");

    // A wrong passphrase and a work-package schema are both refused.
    assert_eq!(
        fx.engine
            .residue_archive_inspect(&json!({
                "archive_path": archive.to_string_lossy(),
                "archive_passphrase": OTHER_PASS,
            }))
            .unwrap_err()
            .code,
        "residue_archive_locked"
    );
    let work = fx.engine.state.join("archives").join("work.age");
    fs::write(
        &work,
        seal(&json!({"schema":"lintel.work/1","files":[]}), PASS).unwrap(),
    )
    .unwrap();
    assert_eq!(
        fx.engine
            .residue_archive_inspect(&json!({
                "archive_path": work.to_string_lossy(),
                "archive_passphrase": PASS,
            }))
            .unwrap_err()
            .code,
        "residue_archive_schema"
    );
}

#[test]
fn legacy_numeric_byte_array_packages_remain_readable() {
    let fx = fixture();
    let original = fx.root.join("debug/legacy.log");
    fs::create_dir_all(original.parent().unwrap()).unwrap();
    // This is the historical wire shape used by earlier producers. Current
    // producers still emit it, so legacy fixtures need no format migration.
    let package = json!({
        "schema": RESIDUE_SCHEMA,
        "environment_id": fx.environment_id["id"],
        "files": [{
            "path": original,
            "role": "residue",
            "bytes": 3,
            "digest": digest(b"old"),
            "data": [111, 108, 100]
        }]
    });
    let archive = fx.engine.state.join("archives").join("legacy.age");
    fs::write(&archive, seal(&package, PASS).unwrap()).unwrap();
    let report = fx
        .engine
        .residue_archive_inspect(&json!({
            "archive_path": archive,
            "archive_passphrase": PASS,
        }))
        .unwrap();
    assert_eq!(report["file_count"], 1);
    assert_eq!(report["files"][0]["digest"], digest(b"old"));
}

#[test]
fn residue_restore_writes_only_selected_entries_to_their_original_paths() {
    let fx = fixture();
    let first = fx.root.join("debug/old.log");
    let second = fx.root.join("shell-snapshots/snap.sh");
    // The originals are gone (as after a clear) but their directories remain;
    // the backup holds the removed bytes.
    fs::create_dir_all(first.parent().unwrap()).unwrap();
    fs::create_dir_all(second.parent().unwrap()).unwrap();
    let archive = seal_package(
        &fx.engine,
        &[
            (&first, "residue", b"12345"),
            (&second, "residue", b"#snapshot"),
        ],
        PASS,
    );
    let plan = restore_plan(&fx, &archive, &[&first]);
    assert_eq!(plan["kind"], "residue_restore");
    // Preview is inert.
    assert!(!first.exists());
    let receipt = run_restore(&fx, &plan);
    assert_eq!(receipt["status"], "completed", "{receipt}");
    assert_eq!(receipt["residue_restore"]["restored"], 1);
    assert_eq!(fs::read(&first).unwrap(), b"12345");
    // The unselected entry is not restored.
    assert!(!second.exists(), "an unselected entry was restored");
}

#[test]
fn residue_restore_refuses_an_occupied_or_unknown_target() {
    let fx = fixture();
    let held = fx.root.join("debug/old.log");
    let archive = seal_package(&fx.engine, &[(&held, "residue", b"12345")], PASS);
    // The original path already holds newer content: no overwrite.
    write(&held, b"a newer external log line");
    assert_eq!(
        fx.engine
            .plan_residue_restore(&json!({
                "environment_id": fx.environment_id["id"],
                "archive_path": archive.to_string_lossy(),
                "archive_passphrase": PASS,
                "selected_paths": [held.to_string_lossy()],
            }))
            .unwrap_err()
            .code,
        "restore_conflict"
    );
    assert_eq!(fs::read(&held).unwrap(), b"a newer external log line");
    // An entry not present in the archive is refused, not silently skipped.
    let absent = fx.root.join("debug/other.log");
    assert_eq!(
        fx.engine
            .plan_residue_restore(&json!({
                "environment_id": fx.environment_id["id"],
                "archive_path": archive.to_string_lossy(),
                "archive_passphrase": PASS,
                "selected_paths": [absent.to_string_lossy()],
            }))
            .unwrap_err()
            .code,
        "invalid_selection"
    );
    // An empty selection never widens into "restore everything".
    assert_eq!(
        fx.engine
            .plan_residue_restore(&json!({
                "environment_id": fx.environment_id["id"],
                "archive_path": archive.to_string_lossy(),
                "archive_passphrase": PASS,
                "selected_paths": [],
            }))
            .unwrap_err()
            .code,
        "invalid_selection"
    );
}

#[test]
fn residue_restore_rejects_a_replaced_archive_before_writing() {
    let fx = fixture();
    let target = fx.root.join("debug/old.log");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    let archive = seal_package(&fx.engine, &[(&target, "residue", b"12345")], PASS);
    let plan = restore_plan(&fx, &archive, &[&target]);
    let saved = frozen(&fx.engine, &plan);
    // An external replacement of the archive after preview must be refused.
    fs::write(&archive, b"not the frozen ciphertext").unwrap();
    let r = json!({"archive_passphrase": PASS});
    let error = fx.engine.check_residue_restore(&saved, &r).unwrap_err();
    assert_eq!(error.code, "stale_residue_archive");
    assert!(!target.exists());
}

#[test]
fn residue_reader_refuses_unsafe_paths_and_bad_digests() {
    let fx = fixture();
    // A relative recorded path is refused.
    let relative = seal(
        &json!({"schema":"lintel.residue/1","files":[
            {"path":"relative/dir","role":"residue","bytes":1,"digest":digest(b"x"),"data":[120]}
        ]}),
        PASS,
    )
    .unwrap();
    let bad = fx.engine.state.join("archives").join("relative.age");
    fs::write(&bad, relative).unwrap();
    assert_eq!(
        fx.engine
            .residue_archive_inspect(&json!({
                "archive_path": bad.to_string_lossy(),
                "archive_passphrase": PASS,
            }))
            .unwrap_err()
            .code,
        "invalid_residue_archive"
    );
    // A digest that disagrees with the data is refused.
    let forged = seal(
        &json!({"schema":"lintel.residue/1","files":[
            {"path":"/tmp/synthetic-forged","role":"residue","bytes":1,"digest":digest(b"y"),"data":[120]}
        ]}),
        PASS,
    )
    .unwrap();
    let forged_path = fx.engine.state.join("archives").join("forged.age");
    fs::write(&forged_path, forged).unwrap();
    assert_eq!(
        fx.engine
            .residue_archive_inspect(&json!({
                "archive_path": forged_path.to_string_lossy(),
                "archive_passphrase": PASS,
            }))
            .unwrap_err()
            .code,
        "invalid_residue_archive"
    );
}

#[test]
fn residue_restore_requires_exactly_one_source() {
    let fx = fixture();
    let target = fx.root.join("debug/old.log");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    let archive = seal_package(&fx.engine, &[(&target, "residue", b"12345")], PASS);
    assert_eq!(
        fx.engine
            .residue_archive_inspect(&json!({
                "archive_path": archive.to_string_lossy(),
                "job_id": uuid::Uuid::new_v4().to_string(),
                "archive_passphrase": PASS,
            }))
            .unwrap_err()
            .code,
        "invalid_request"
    );
    assert_eq!(
        fx.engine
            .residue_archive_inspect(&json!({"archive_passphrase": PASS}))
            .unwrap_err()
            .code,
        "invalid_request"
    );
}

//! Synthetic regression tests for the local-data / identity write surface.
//! Every fixture is a temporary directory; no real `~/.claude*` path, credential,
//! login token or account object is ever read or written, and no process or
//! service is touched (the unit-test Engine refuses both).
use super::*;
use crate::archive::unseal;
use crate::storage::{load, save, snapshot};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;

const USER_ID: &str = "syn-identity-123";
const SECRET: &str = "synSecretValue999";
const PASS: &str = "synthetic passphrase for client-data tests";
const OTHER_PASS: &str = "a different synthetic passphrase";

struct Fixture {
    _temp: tempfile::TempDir,
    home: PathBuf,
    root: PathBuf,
    engine: Engine,
    environment_id: Value,
}

/// A synthetic home with a non-default root (`home/cc`), so the client-state
/// file resolves to `root/.claude.json` exactly as `cleanup.rs` would.
fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    // `/var` is a symlink on macOS; the engine refuses symlinked components.
    let base = temp.path().canonicalize().unwrap();
    let home = base.join("home");
    let root = home.join("cc");
    fs::create_dir_all(&root).unwrap();
    let engine = Engine::new(home.clone(), base.join("state")).unwrap();
    let environment_id = engine.register("fixture", &root, false).unwrap();
    Fixture {
        _temp: temp,
        home,
        root,
        engine,
        environment_id,
    }
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn fifo(path: &Path) {
    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    let rc = unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) };
    assert_eq!(rc, 0, "mkfifo failed: {}", std::io::Error::last_os_error());
}

/// Seed the residual, work and identity material the two recipes operate on.
fn seed(fx: &Fixture) {
    write(&fx.root.join("debug/old.log"), b"12345");
    write(&fx.root.join("debug/older.log"), b"0123456789");
    write(&fx.root.join("shell-snapshots/snap.sh"), b"#snapshot");
    write(
        &fx.root.join("projects/example/session.jsonl"),
        b"{\"work\":true}\n",
    );
    write(
        &fx.root.join("projects/example/memory/MEMORY.md"),
        b"keep memory",
    );
    write(&fx.root.join(".credentials.json"), b"SYNTHETIC_CREDENTIAL");
    write(
        &fx.root.join(".claude.json.backup"),
        b"{\"synthetic\":\"backup\"}",
    );
    write(
        &fx.root.join(".claude.json"),
        format!(
            "{{\"userID\":\"{USER_ID}\",\"private\":\"{SECRET}\",\"unrelated\":\"keep-me\",\"oauthAccount\":{{\"emailAddress\":\"synthetic@example.invalid\"}}}}"
        )
        .as_bytes(),
    );
}

// ---- helpers ---------------------------------------------------------------

/// Load the private frozen plan written by [`Engine::plan`], which
/// `public_plan` deliberately strips `extra` from before returning.
fn frozen(engine: &Engine, public: &Value) -> Value {
    let id = public["id"].as_str().unwrap();
    load(&engine.state.join("plans").join(format!("{id}.json"))).unwrap()
}

fn run_clear(fx: &Fixture, plan: &Value, pass: &str) -> Value {
    let id = plan["id"].as_str().unwrap();
    let saved = frozen(&fx.engine, plan);
    let env = fx
        .engine
        .env(&json!({"environment_id": saved["environment_id"]}))
        .unwrap();
    let r = json!({"archive_passphrase": pass});
    fx.engine.check_residue_clear(&saved, &r).unwrap();
    let journal = fx.engine.state.join("jobs").join(format!("{id}.json"));
    let mut receipt = json!({"id":id,"plan_id":id,"status":"executing","steps":[],"warnings":[]});
    save(&journal, &receipt).unwrap();
    fx.engine
        .clear_residue(&env, &saved, &r, &mut receipt, &journal)
        .unwrap();
    receipt
}

fn run_identity(fx: &Fixture, plan: &Value, pass: &str) -> Value {
    let id = plan["id"].as_str().unwrap();
    let saved = frozen(&fx.engine, plan);
    let env = fx
        .engine
        .env(&json!({"environment_id": saved["environment_id"]}))
        .unwrap();
    let r = json!({"archive_passphrase": pass});
    fx.engine.check_identity_reset(&saved, &r).unwrap();
    let journal = fx.engine.state.join("jobs").join(format!("{id}.json"));
    let mut receipt = json!({"id":id,"plan_id":id,"status":"executing","steps":[],"warnings":[]});
    save(&journal, &receipt).unwrap();
    fx.engine
        .identity_reset(&env, &saved, &r, &mut receipt, &journal)
        .unwrap();
    receipt
}

fn clear_plan(fx: &Fixture, paths: &[&Path], archive: bool) -> Value {
    let residue_paths: Vec<String> = paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    fx.engine
        .plan_clear_residue(&json!({
            "environment_id": fx.environment_id["id"],
            "writers_confirmed_stopped": true,
            "residue_paths": residue_paths,
            "archive_before_clear": archive,
        }))
        .unwrap()
}

fn identity_plan(fx: &Fixture, backups: &[&Path]) -> Value {
    let identity_backup_paths: Vec<String> = backups
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    fx.engine
        .plan_identity_reset(&json!({
            "environment_id": fx.environment_id["id"],
            "writers_confirmed_stopped": true,
            "identity_backup_paths": identity_backup_paths,
        }))
        .unwrap()
}

// ---- residue clearing ------------------------------------------------------

#[test]
fn clear_residue_removes_only_selected_originals_and_backs_them_up() {
    let fx = fixture();
    seed(&fx);
    let first = fx.root.join("debug/old.log");
    let second = fx.root.join("shell-snapshots/snap.sh");
    let plan = clear_plan(&fx, &[&first, &second], true);
    assert_eq!(plan["kind"], "cleanup");
    assert_eq!(plan["outcome"], "clear_residue");
    // The private frozen plan carries the exact originals; the public projection
    // hides `extra`, so the count is read from the frozen plan itself.
    let frozen_plan = frozen(&fx.engine, &plan);
    assert_eq!(frozen_plan["extra"]["files"].as_array().unwrap().len(), 2);
    assert_eq!(frozen_plan["extra"]["archive_before_clear"], true);
    // Preview is inert: nothing was removed by planning.
    assert!(first.exists() && second.exists());

    let receipt = run_clear(&fx, &plan, PASS);
    assert_eq!(receipt["status"], "completed", "{receipt}");
    assert_eq!(receipt["residue_clear"]["removed"], 2);
    assert!(!first.exists(), "selected residue was not removed");
    assert!(!second.exists());
    // Unselected residue, work content, credentials and the unselected backup
    // are byte-unchanged.
    assert_eq!(
        fs::read(fx.root.join("debug/older.log")).unwrap(),
        b"0123456789"
    );
    assert_eq!(
        fs::read(fx.root.join("projects/example/session.jsonl")).unwrap(),
        b"{\"work\":true}\n"
    );
    assert_eq!(
        fs::read(fx.root.join("projects/example/memory/MEMORY.md")).unwrap(),
        b"keep memory"
    );
    assert_eq!(
        fs::read(fx.root.join(".credentials.json")).unwrap(),
        b"SYNTHETIC_CREDENTIAL"
    );
    assert_eq!(
        fs::read(fx.root.join(".claude.json.backup")).unwrap(),
        b"{\"synthetic\":\"backup\"}"
    );

    // The residue archive is an independent, decryptable package with its own
    // schema, carrying exactly the removed bytes.
    let archive_path = PathBuf::from(receipt["residue_archive_path"].as_str().unwrap());
    let sealed = fs::read(&archive_path).unwrap();
    assert!(sealed.starts_with(b"age-encryption.org/"));
    let package = unseal(&sealed, PASS).unwrap();
    assert_eq!(package["schema"], "lintel.residue/1");
    assert_ne!(package["schema"], "lintel.work/1");
    let files = package["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
    let recorded: Vec<&str> = files.iter().map(|f| f["path"].as_str().unwrap()).collect();
    assert!(recorded.contains(&first.to_str().unwrap()));
    let data: Vec<u8> = files
        .iter()
        .flat_map(|f| f["data"].as_array().unwrap())
        .map(|b| b.as_u64().unwrap() as u8)
        .collect();
    assert!(data.windows(5).any(|w| w == b"12345"));
    // A different passphrase cannot open the residue archive.
    assert_eq!(
        unseal(&sealed, OTHER_PASS).unwrap_err().code,
        "archive_locked"
    );
}

#[test]
fn clear_residue_without_backup_removes_and_records_no_archive() {
    let fx = fixture();
    seed(&fx);
    let target = fx.root.join("debug/old.log");
    let plan = clear_plan(&fx, &[&target], false);
    let receipt = run_clear(&fx, &plan, PASS);
    assert_eq!(receipt["status"], "completed", "{receipt}");
    assert!(!target.exists());
    assert!(receipt["residue_archive_path"].is_null());
    // No archive was produced for a plan that did not request preservation.
    let archives = fs::read_dir(fx.engine.state.join("archives"))
        .unwrap()
        .count();
    assert_eq!(archives, 0);
}

#[test]
fn clear_residue_refuses_unsafe_or_empty_selections_before_any_write() {
    let fx = fixture();
    seed(&fx);
    let log = fx.root.join("debug/old.log");
    // Empty selection is never widened into a directory sweep.
    let empty = fx.engine.plan_clear_residue(&json!({
        "environment_id": fx.environment_id["id"],
        "writers_confirmed_stopped": true,
        "residue_paths": [],
    }));
    assert_eq!(empty.unwrap_err().code, "invalid_residue_paths");
    // A symlink is refused, never followed.
    let link = fx.root.join("debug/link.log");
    std::os::unix::fs::symlink(&log, &link).unwrap();
    let symlinked = fx.engine.plan_clear_residue(&json!({
        "environment_id": fx.environment_id["id"],
        "writers_confirmed_stopped": true,
        "residue_paths": [link.to_string_lossy()],
    }));
    assert_eq!(symlinked.unwrap_err().code, "symlink_target");
    assert!(log.exists());
    // A FIFO is refused without a blocking open.
    let pipe = fx.root.join("debug/pipe.log");
    fifo(&pipe);
    let special = fx.engine.plan_clear_residue(&json!({
        "environment_id": fx.environment_id["id"],
        "writers_confirmed_stopped": true,
        "residue_paths": [pipe.to_string_lossy()],
    }));
    assert_eq!(special.unwrap_err().code, "file_limit");
    // A multi-link file is refused: one name is not the whole object.
    let held = fx.root.join("debug/held.log");
    write(&held, b"shared");
    let hard = fx.root.join("debug/hard.log");
    fs::hard_link(&held, &hard).unwrap();
    let linked = fx.engine.plan_clear_residue(&json!({
        "environment_id": fx.environment_id["id"],
        "writers_confirmed_stopped": true,
        "residue_paths": [hard.to_string_lossy()],
    }));
    assert_eq!(linked.unwrap_err().code, "unsafe_file");
    // A relative path is refused.
    let relative = fx.engine.plan_clear_residue(&json!({
        "environment_id": fx.environment_id["id"],
        "writers_confirmed_stopped": true,
        "residue_paths": ["debug/old.log"],
    }));
    assert_eq!(relative.unwrap_err().code, "invalid_path");
    // Nothing was removed by any refused preview.
    assert!(log.exists());
    assert_eq!(fs::read(&held).unwrap(), b"shared");
}

#[test]
fn clear_residue_skips_a_replaced_original_and_completes_the_others() {
    let fx = fixture();
    seed(&fx);
    let changed = fx.root.join("debug/old.log");
    let untouched = fx.root.join("debug/older.log");
    let plan = clear_plan(&fx, &[&changed, &untouched], true);
    // An external writer replaces one selected original between preview and
    // execution. The frozen plan is the only authority, so that file is skipped
    // with a reason rather than deleted, and the other removal still runs.
    write(&changed, b"a newer external log line");
    let receipt = run_clear(&fx, &plan, PASS);
    assert_eq!(receipt["status"], "partially_completed", "{receipt}");
    assert_eq!(receipt["residue_clear"]["removed"], 1);
    assert_eq!(receipt["residue_clear"]["skipped"], 1);
    // The changed file is reported as a conflict with a reason, and it was not
    // backed up under the stale digest either.
    assert_eq!(
        receipt["residue_clear"]["conflicts"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(receipt["residue_backup"]["preserved"], 1);
    assert_eq!(
        fs::read(&changed).unwrap(),
        b"a newer external log line",
        "a changed file was deleted"
    );
    assert!(
        !untouched.exists(),
        "an unchanged selection blocked the others"
    );
}

#[test]
fn clear_residue_skips_an_already_absent_original_without_error() {
    let fx = fixture();
    seed(&fx);
    let gone = fx.root.join("debug/old.log");
    let other = fx.root.join("shell-snapshots/snap.sh");
    let plan = clear_plan(&fx, &[&gone, &other], false);
    fs::remove_file(&gone).unwrap();
    let receipt = run_clear(&fx, &plan, PASS);
    assert_eq!(receipt["status"], "completed", "{receipt}");
    assert_eq!(receipt["residue_clear"]["removed"], 1);
    assert_eq!(receipt["residue_clear"]["skipped"], 1);
    assert!(!other.exists());
}

// ---- identity reset --------------------------------------------------------

#[test]
fn identity_reset_removes_only_the_frozen_field_and_preserves_other_keys() {
    let fx = fixture();
    seed(&fx);
    let backup = fx.root.join(".claude.json.backup");
    let plan = identity_plan(&fx, &[&backup]);
    assert_eq!(plan["kind"], "cleanup");
    assert_eq!(plan["outcome"], "identity_reset");
    let receipt = run_identity(&fx, &plan, PASS);
    assert_eq!(receipt["status"], "completed", "{receipt}");
    assert_eq!(receipt["identity_reset"]["removed_keys"], json!(["userID"]));
    assert_eq!(receipt["identity_reset"]["removal_verified"], true);
    // Regeneration is unknowable here, so it is reported honestly as unknown.
    assert_eq!(receipt["identity_reset"]["regenerated"], "unknown");

    let doc = load(&fx.root.join(".claude.json")).unwrap();
    assert!(doc.get("userID").is_none(), "userID remained: {doc}");
    // Every other key keeps its value, including the account marker and a key
    // the rule list does not classify.
    assert_eq!(doc["private"], SECRET);
    assert_eq!(doc["unrelated"], "keep-me");
    assert_eq!(
        doc["oauthAccount"]["emailAddress"],
        "synthetic@example.invalid"
    );
    // The selected backup was preserved then removed; credentials and work
    // content are untouched, and no logout occurred.
    assert!(!backup.exists());
    assert_eq!(
        fs::read(fx.root.join(".credentials.json")).unwrap(),
        b"SYNTHETIC_CREDENTIAL"
    );
    assert_eq!(
        fs::read(fx.root.join("projects/example/session.jsonl")).unwrap(),
        b"{\"work\":true}\n"
    );
    assert!(receipt["steps"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["id"] != "logout"));
    // The identity archive independently decrypts to a lintel.residue/1 package
    // that still holds the original document bytes.
    let archive_path = PathBuf::from(receipt["identity_archive_path"].as_str().unwrap());
    let package = unseal(&fs::read(&archive_path).unwrap(), PASS).unwrap();
    assert_eq!(package["schema"], "lintel.residue/1");
    let state = package["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["role"] == "client_state")
        .unwrap();
    let restored: Vec<u8> = state["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b.as_u64().unwrap() as u8)
        .collect();
    assert!(String::from_utf8_lossy(&restored).contains(USER_ID));
}

#[test]
fn identity_reset_keeps_unselected_backups_and_refuses_a_stale_plan() {
    let fx = fixture();
    seed(&fx);
    let selected = fx.root.join(".claude.json.backup");
    let unselected = fx.root.join("debug/old.log");
    write(&unselected, b"not an identity backup");
    let plan = identity_plan(&fx, &[&selected]);
    // An external writer edits client_state after preview: the reset must refuse
    // and leave the new state and every backup untouched.
    write(
        &fx.root.join(".claude.json"),
        format!("{{\"userID\":\"{USER_ID}\",\"unrelated\":\"edited-after-preview\"}}").as_bytes(),
    );
    let stale = run_identity_expect_error(&fx, &plan, PASS);
    assert_eq!(stale.code, "stale_plan");
    assert!(load(&fx.root.join(".claude.json")).unwrap()["userID"].is_string());
    assert!(selected.exists());
    assert!(unselected.exists());
    // No archive was published by the refused execution.
    assert_eq!(
        fs::read_dir(fx.engine.state.join("archives"))
            .unwrap()
            .count(),
        0
    );
}

fn run_identity_expect_error(fx: &Fixture, plan: &Value, pass: &str) -> crate::Error {
    let id = plan["id"].as_str().unwrap();
    let saved = frozen(&fx.engine, plan);
    let env = fx
        .engine
        .env(&json!({"environment_id": saved["environment_id"]}))
        .unwrap();
    let r = json!({"archive_passphrase": pass});
    let journal = fx.engine.state.join("jobs").join(format!("{id}.json"));
    let mut receipt = json!({"id":id,"plan_id":id,"status":"executing","steps":[],"warnings":[]});
    save(&journal, &receipt).unwrap();
    fx.engine
        .identity_reset(&env, &saved, &r, &mut receipt, &journal)
        .unwrap_err()
}

#[test]
fn identity_reset_requires_a_verified_identifier() {
    let fx = fixture();
    write(
        &fx.root.join(".claude.json"),
        b"{\"oauthAccount\":{\"emailAddress\":\"synthetic@example.invalid\"}}",
    );
    assert_eq!(identity_plan_err(&fx).code, "identity_absent");
    // A malformed document is unknown, never guessed at.
    write(&fx.root.join(".claude.json"), b"{ not json");
    assert_eq!(identity_plan_err(&fx).code, "identity_unknown");
    // A symlinked client_state is refused as unreadable rather than followed.
    let outside = fx.home.join("outside.json");
    write(&outside, format!("{{\"userID\":\"{USER_ID}\"}}").as_bytes());
    fs::remove_file(fx.root.join(".claude.json")).unwrap();
    std::os::unix::fs::symlink(&outside, fx.root.join(".claude.json")).unwrap();
    assert_eq!(identity_plan_err(&fx).code, "identity_unknown");
}

fn identity_plan_err(fx: &Fixture) -> crate::Error {
    fx.engine
        .plan_identity_reset(&json!({
            "environment_id": fx.environment_id["id"],
            "writers_confirmed_stopped": true,
        }))
        .unwrap_err()
}

#[test]
fn identity_reset_removes_only_the_selected_backups() {
    let fx = fixture();
    seed(&fx);
    let selected = fx.root.join(".claude.json.backup");
    let also_selected = fx.root.join("shell-snapshots/snap.sh");
    write(&also_selected, b"#snapshot");
    let neighbor = fx.root.join("debug/old.log");
    let plan = identity_plan(&fx, &[&selected, &also_selected]);
    let receipt = run_identity(&fx, &plan, PASS);
    assert_eq!(receipt["status"], "completed", "{receipt}");
    assert_eq!(receipt["identity_reset"]["removed_backups"], 2);
    assert!(!selected.exists());
    assert!(!also_selected.exists());
    assert_eq!(
        fs::read(&neighbor).unwrap(),
        b"12345",
        "an unselected neighbour was affected"
    );
    let package = unseal(
        &fs::read(receipt["identity_archive_path"].as_str().unwrap()).unwrap(),
        PASS,
    )
    .unwrap();
    let roles: Vec<&str> = package["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["role"].as_str().unwrap())
        .collect();
    assert_eq!(roles.iter().filter(|r| **r == "identity_backup").count(), 2);
    assert_eq!(roles.iter().filter(|r| **r == "client_state").count(), 1);
}

// ---- masked identity inspection -------------------------------------------

#[test]
fn identity_inspect_is_presence_only_until_explicitly_revealed() {
    let fx = fixture();
    seed(&fx);
    let masked = fx
        .engine
        .identity_inspect(&json!({"environment_id": fx.environment_id["id"]}))
        .unwrap();
    assert_eq!(masked["schema"], "lintel.identity/1");
    assert_eq!(masked["state"], "observed");
    assert!(masked["revealed"].is_null());
    let text = serde_json::to_string(&masked).unwrap();
    assert!(!text.contains(USER_ID), "{text}");
    assert!(!text.contains(SECRET), "{text}");
    assert!(!text.contains("synthetic@example.invalid"), "{text}");
    let fields = masked["fields"].as_array().unwrap();
    assert_eq!(fields.len(), 2);
    assert!(fields.iter().all(|f| f["present"] == true));

    let revealed = fx
        .engine
        .identity_inspect(&json!({
            "environment_id": fx.environment_id["id"],
            "reveal": true,
        }))
        .unwrap();
    assert_eq!(revealed["revealed"][0]["value"], USER_ID);
    assert_eq!(revealed["reveal_scope"], "private_response_only");
    // The account marker is still never revealed, and no unrelated secret leaks.
    let text = serde_json::to_string(&revealed).unwrap();
    assert!(!text.contains("synthetic@example.invalid"), "{text}");
    assert!(!text.contains(SECRET), "{text}");
    assert!(revealed["revealed"].get("oauthAccount").is_none());
}

#[test]
fn identity_inspect_reveal_reports_no_value_for_unreadable_state() {
    let fx = fixture();
    let outside = fx.home.join("outside.json");
    write(&outside, format!("{{\"userID\":\"{USER_ID}\"}}").as_bytes());
    std::os::unix::fs::symlink(&outside, fx.root.join(".claude.json")).unwrap();
    let revealed = fx
        .engine
        .identity_inspect(&json!({
            "environment_id": fx.environment_id["id"],
            "reveal": true,
        }))
        .unwrap();
    assert_eq!(revealed["state"], "unreadable");
    assert!(revealed["revealed"].is_null());
}

/// The reset must actually change the frozen document, and keep the restrictive
/// mode, so a post-reset re-observation is seen as a change rather than a no-op.
#[test]
fn identity_reset_receipt_reports_the_observed_state() {
    let fx = fixture();
    seed(&fx);
    // Freeze one restrictive mode and prove the reset keeps it (only group/other
    // write bits are ever dropped, never a widening).
    fs::set_permissions(
        fx.root.join(".claude.json"),
        fs::Permissions::from_mode(0o640),
    )
    .unwrap();
    let plan = identity_plan(&fx, &[]);
    let before = snapshot(&fx.root.join(".claude.json")).unwrap();
    let receipt = run_identity(&fx, &plan, PASS);
    assert_eq!(receipt["identity_reset"]["state_after"], "observed");
    let after = snapshot(&fx.root.join(".claude.json")).unwrap();
    assert_ne!(before, after, "the client-state document did not change");
    assert_eq!(
        fs::metadata(fx.root.join(".claude.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o640
    );
}

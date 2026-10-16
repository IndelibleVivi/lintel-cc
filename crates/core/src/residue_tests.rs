//! Synthetic regression tests for the residue/identity observation surface.
//! Every fixture is a temporary directory; no real `~/.claude*` path, credential
//! or account object is ever read or written.
use super::*;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;

const SYNTHETIC_USER_ID: &str = "syn-user-123";
const SYNTHETIC_SECRET: &str = "synSecretValue999";

struct Fixture {
    _temp: tempfile::TempDir,
    home: PathBuf,
    root: PathBuf,
}

/// A synthetic home with a non-default root (`home/cc`), so the client-state file
/// resolves to `root/.claude.json` exactly as `cleanup.rs` would.
fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    // `/var` is a symlink on macOS; the observation functions refuse symlinked
    // path components, so start from the canonical base.
    let base = temp.path().canonicalize().unwrap();
    let home = base.join("home");
    let root = home.join("cc");
    fs::create_dir_all(&root).unwrap();
    Fixture {
        _temp: temp,
        home,
        root,
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

fn location<'a>(report: &'a Value, category: &str) -> &'a Value {
    report["locations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|location| location["category"] == category)
        .unwrap_or_else(|| panic!("no {category} location in {report}"))
}

/// The seeded fixture mixes regular files, an empty location, a deep location, a
/// FIFO, a symlink and a hard link, plus one directory that is not in the table.
fn seed(fixture: &Fixture) -> PathBuf {
    let root = &fixture.root;
    // Two ordinary log files, plus a special and a symlinked entry.
    write(&root.join("debug/a.log"), b"12345");
    write(&root.join("debug/b.log"), b"0123456789");
    fifo(&root.join("debug/pipe.log"));
    std::os::unix::fs::symlink(root.join("debug/a.log"), root.join("debug/link.log")).unwrap();

    // Present but empty: a complete observation of an empty location.
    fs::create_dir_all(root.join("todos")).unwrap();

    // Work content: every entry gets the shared static work category.
    write(
        &root.join("projects/example/session.jsonl"),
        b"{\"synthetic\":true}\n",
    );
    write(
        &root.join("projects/example/memory/MEMORY.md"),
        b"synthetic memory",
    );
    write(&root.join("projects/example/notes.txt"), b"synthetic note");

    // Identity material: presence only, never entered or read here.
    write(&root.join(".credentials.json"), b"SYNTHETIC_ONLY");
    write(
        &root.join(".claude.json"),
        format!(
            "{{\"userID\":\"{SYNTHETIC_USER_ID}\",\"private\":\"{SYNTHETIC_SECRET}\",\"oauthAccount\":{{\"emailAddress\":\"synthetic@example.invalid\"}}}}"
        )
        .as_bytes(),
    );
    write(
        &root.join(".claude.json.backup"),
        b"{\"synthetic\":\"backup\"}",
    );

    // More than the listed-row cap, so the location must report truncation while
    // its aggregates stay real.
    for index in 0..105 {
        write(
            &root.join(format!("shell-snapshots/snap-{index:03}.sh")),
            b"#",
        );
    }

    // Every name here shares one inode (nlink 2), so both are skipped as unsafe.
    write(&root.join("backups/config-a.json"), b"abc");
    fs::hard_link(
        root.join("backups/config-a.json"),
        root.join("backups/config-b.json"),
    )
    .unwrap();

    // Depth: directories up to 3 levels below the location are entered, so a file
    // at relative depth 4 is collected and the depth-4 directory is not entered.
    write(&root.join("session-env/a/b/c/shallow.txt"), b"shallow");
    write(&root.join("session-env/a/b/c/d/deep.txt"), b"unreachable");

    // Not in the table: must never appear in any output.
    write(&root.join("unknown-dir/secret.jsonl"), b"synthetic unknown");

    root.to_path_buf()
}

#[test]
fn inventory_covers_only_the_known_table_with_metadata() {
    let fixture = fixture();
    let root = seed(&fixture);
    let report = inventory(&fixture.home, &root).unwrap();
    assert_eq!(report["schema"], "lintel.residue/1");
    assert_eq!(report["root"], root.to_string_lossy().as_ref());
    assert_eq!(
        report["locations"].as_array().unwrap().len(),
        LOCATION_TABLE.len()
    );

    // debug/: two regular files, one FIFO and one symlink both skipped as special.
    let debug = location(&report, "debug_logs");
    assert_eq!(debug["state"], "present");
    assert_eq!(debug["class"], "residue");
    assert_eq!(debug["file_count"], 2);
    assert_eq!(debug["total_bytes"], 15);
    assert_eq!(debug["skipped_special"], 2);
    assert_eq!(debug["skipped_unsafe"], 0);
    assert_eq!(debug["truncated"], false);
    assert_eq!(debug["entries"].as_array().unwrap().len(), 2);
    // Canonical pages use stable path order.
    assert_eq!(debug["entries"][0]["bytes"], 5);
    assert_eq!(debug["entries"][1]["bytes"], 10);
    assert!(debug["newest_mtime"].as_i64().unwrap() > 0);

    // An empty location is a complete observation, not an absence.
    let todos = location(&report, "todo_state");
    assert_eq!(todos["state"], "present");
    assert_eq!(todos["file_count"], 0);
    assert_eq!(todos["total_bytes"], 0);
    assert_eq!(todos["newest_mtime"], Value::Null);

    // shell-snapshots/: real aggregates, capped list, explicit truncation.
    let snaps = location(&report, "shell_snapshots");
    assert_eq!(snaps["file_count"], 105);
    assert_eq!(snaps["total_bytes"], 105);
    assert_eq!(snaps["entries"].as_array().unwrap().len(), 105);
    assert_eq!(snaps["truncated"], false);

    // Hard links are skipped as unsafe, and counted.
    let backups = location(&report, "config_backups");
    assert_eq!(backups["file_count"], 0);
    assert_eq!(backups["skipped_unsafe"], 2);
    assert_eq!(backups["state"], "present");

    // Bounded depth: the depth-4 file is unreachable, so the location is truncated.
    let session_env = location(&report, "session_env");
    assert_eq!(session_env["file_count"], 1);
    assert_eq!(session_env["truncated"], true);

    // Work content is annotated with the shared static category rule.
    let projects = location(&report, "sessions");
    assert_eq!(projects["class"], "work");
    assert_eq!(projects["file_count"], 3);
    let category_of = |suffix: &str| {
        projects["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["path"].as_str().unwrap().ends_with(suffix))
            .unwrap()["work_category"]
            .clone()
    };
    assert_eq!(category_of("session.jsonl"), "sessions");
    assert_eq!(category_of("MEMORY.md"), "memory");
    assert_eq!(category_of("notes.txt"), Value::Null);

    // Identity locations: presence only, and never entered.
    for (category, expected_class) in [
        ("credentials", "identity"),
        ("client_state", "identity"),
        ("shared_profile", "identity"),
    ] {
        let entry = location(&report, category);
        assert_eq!(entry["class"], expected_class, "{category}");
        assert_eq!(entry["entries"].as_array().unwrap().len(), 0, "{category}");
    }
    assert_eq!(location(&report, "credentials")["state"], "present");
    assert_eq!(location(&report, "credentials")["file_count"], 1);
    assert_eq!(location(&report, "client_state")["state"], "present");
    // The absent shared profile is a complete not_found, not a gap.
    assert_eq!(location(&report, "shared_profile")["state"], "not_found");
    // The backup sibling is ordinary residue.
    let backup = location(&report, "client_state_backup");
    assert_eq!(backup["class"], "residue");
    assert_eq!(backup["state"], "present");
    assert_eq!(backup["file_count"], 1);

    // Totals are the sum of the per-location aggregates.
    let files: u64 = report["locations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|location| location["file_count"].as_u64().unwrap())
        .sum();
    assert_eq!(report["totals"]["file_count"], files);
    // Truncation anywhere means the whole scan is not complete.
    assert_eq!(report["scan_complete"], false);
}

#[test]
fn inventory_never_leaks_content_or_covers_unknown_paths() {
    let fixture = fixture();
    let root = seed(&fixture);
    let report = inventory(&fixture.home, &root).unwrap();
    let text = serde_json::to_string(&report).unwrap();
    // Metadata-only: no file body, and specifically none of the synthetic
    // identity/private values, may appear anywhere in the inventory.
    assert!(!text.contains(SYNTHETIC_USER_ID), "{text}");
    assert!(!text.contains(SYNTHETIC_SECRET), "{text}");
    assert!(!text.contains("synthetic memory"), "content leaked");
    assert!(
        !text.contains("SYNTHETIC_ONLY"),
        "credential content leaked"
    );
    // A directory outside the table is not covered and not reported.
    assert!(!text.contains("unknown-dir"), "{text}");
    for location in report["locations"].as_array().unwrap() {
        assert_ne!(
            location["path"].as_str().unwrap(),
            root.join("unknown-dir").to_string_lossy().as_ref()
        );
    }
}

#[test]
fn overlapping_explicit_roots_do_not_duplicate_totals_or_reclassify_identity() {
    let fixture = fixture();
    write(&fixture.root.join("debug/a.log"), b"synthetic");
    write(&fixture.root.join(".credentials.json"), b"synthetic-secret");
    write(
        &fixture.root.join(".claude.json"),
        b"{\"userID\":\"synthetic\"}",
    );
    let known = inventory(&fixture.home, &fixture.root).unwrap();
    let overlapping = inventory_request(
        &fixture.home,
        &fixture.root,
        &json!({"extra_roots":[fixture.root,fixture.root.join("debug")]}),
    )
    .unwrap();
    assert_eq!(known["totals"], overlapping["totals"]);
    assert_eq!(overlapping["rows"].as_array().unwrap().len(), 1);
    assert_eq!(overlapping["rows"][0]["class"], "residue");
    save(
        &fixture.root.join("settings.json"),
        &json!({"env":{"OTEL_LOG_RAW_API_BODIES":"file:relative-body-dir"}}),
    )
    .unwrap();
    let report = inventory(&fixture.home, &fixture.root).unwrap();
    assert_eq!(
        report["coverage"]["additional_sources"]["raw_body"]["state"],
        "relative_directory_unsupported"
    );
}

#[test]
fn inventory_requires_an_existing_root() {
    let fixture = fixture();
    let missing = fixture.home.join("absent");
    let error = inventory(&fixture.home, &missing).unwrap_err();
    assert_eq!(error.code, "root_missing");
}

#[test]
fn identity_observation_masks_values_and_binds_them_privately() {
    let fixture = fixture();
    let root = seed(&fixture);
    let salt = "synthetic-persistent-salt";
    let observed = identity_observation(&fixture.home, &root, salt).unwrap();
    let public = &observed["public"];
    assert_eq!(public["schema"], "lintel.identity/1");
    assert_eq!(public["state"], "observed");
    assert_eq!(
        public["path"],
        root.join(".claude.json").to_string_lossy().as_ref()
    );
    let fields = public["fields"].as_array().unwrap();
    assert_eq!(fields.len(), IDENTITY_KEYS.len());
    let user = fields
        .iter()
        .find(|field| field["key"] == "userID")
        .unwrap();
    assert_eq!(user["present"], true);
    assert_eq!(user["persistence"], "persistent_across_sessions");
    assert_eq!(user["storage"], "client_state");
    assert_eq!(user["masked"], true);
    let account = fields
        .iter()
        .find(|field| field["key"] == "oauthAccount")
        .unwrap();
    assert_eq!(account["present"], true);
    assert_eq!(account["persistence"], "session_account");
    assert_eq!(account["masked"], true);

    // The public projection is value-free; the salted digest only ever lives in
    // the private binding.
    let public_text = serde_json::to_string(public).unwrap();
    let all_text = serde_json::to_string(&observed).unwrap();
    assert!(!public_text.contains(SYNTHETIC_USER_ID), "{public_text}");
    assert!(!public_text.contains(SYNTHETIC_SECRET), "{public_text}");
    assert!(
        !public_text.contains("synthetic@example.invalid"),
        "{public_text}"
    );
    assert!(!all_text.contains(SYNTHETIC_USER_ID), "{all_text}");
    assert!(!all_text.contains(SYNTHETIC_SECRET), "{all_text}");

    let binding = &observed["binding"];
    assert_eq!(binding["schema"], "lintel.identity-binding/1");
    assert_eq!(binding["salt_schema"], "sha256(salt|key|value)");
    let expected = digest(format!("{salt}|userID|{SYNTHETIC_USER_ID}").as_bytes());
    assert_eq!(binding["digests"]["userID"], expected);
    assert_eq!(expected.len(), 64);
    // `oauthAccount` is presence-only: no digest is derived from it.
    assert!(binding["digests"].get("oauthAccount").is_none());
}

#[test]
fn identity_observation_reports_not_found_malformed_and_too_large() {
    let absent = fixture();
    let observed = identity_observation(&absent.home, &absent.root, "salt").unwrap();
    assert_eq!(observed["public"]["state"], "not_found");
    assert!(observed["public"]["fields"].as_array().unwrap().is_empty());
    assert!(observed["binding"]["digests"]
        .as_object()
        .unwrap()
        .is_empty());

    let malformed = fixture();
    write(&malformed.root.join(".claude.json"), b"{ not json");
    let observed = identity_observation(&malformed.home, &malformed.root, "salt").unwrap();
    assert_eq!(observed["public"]["state"], "unknown");
    assert!(observed["public"]["fields"].as_array().unwrap().is_empty());
    assert!(observed["binding"]["digests"]
        .as_object()
        .unwrap()
        .is_empty());

    // A duplicate key is as unparsable as text, so no field is guessed.
    let duplicated = fixture();
    write(
        &duplicated.root.join(".claude.json"),
        format!("{{\"userID\":\"{SYNTHETIC_USER_ID}\",\"userID\":\"other\"}}").as_bytes(),
    );
    assert_eq!(
        identity_observation(&duplicated.home, &duplicated.root, "salt").unwrap()["public"]
            ["state"],
        "unknown"
    );

    let oversized = fixture();
    let path = oversized.root.join(".claude.json");
    write(&path, b"");
    fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(IDENTITY_BYTES + 1)
        .unwrap();
    let observed = identity_observation(&oversized.home, &oversized.root, "salt").unwrap();
    assert_eq!(observed["public"]["state"], "too_large");
    assert!(observed["binding"]["digests"]
        .as_object()
        .unwrap()
        .is_empty());

    // A symlinked client-state file is refused rather than followed.
    let linked = fixture();
    let outside = linked.home.join("outside.json");
    write(
        &outside,
        format!("{{\"userID\":\"{SYNTHETIC_USER_ID}\"}}").as_bytes(),
    );
    std::os::unix::fs::symlink(&outside, linked.root.join(".claude.json")).unwrap();
    let observed = identity_observation(&linked.home, &linked.root, "salt").unwrap();
    assert_eq!(observed["public"]["state"], "unreadable");
    assert!(observed["binding"]["digests"]
        .as_object()
        .unwrap()
        .is_empty());
}

#[test]
fn identity_observation_follows_the_shared_client_state_location_rule() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let home = base.join("home");
    let root = home.join(".claude");
    fs::create_dir_all(&root).unwrap();
    write(
        &home.join(".claude.json"),
        format!("{{\"userID\":\"{SYNTHETIC_USER_ID}\"}}").as_bytes(),
    );
    let observed = identity_observation(&home, &root, "salt").unwrap();
    assert_eq!(observed["public"]["state"], "observed");
    assert_eq!(
        observed["public"]["path"],
        home.join(".claude.json").to_string_lossy().as_ref()
    );
    // The backup sibling follows the same base, so it is not reported inside root.
    let report = inventory(&home, &root).unwrap();
    assert_eq!(
        location(&report, "client_state_backup")["state"],
        "not_found"
    );
}

#[test]
fn identity_compare_never_reveals_a_digest() {
    let fx = fixture();
    let root = seed(&fx);
    // A salt must stay stable across snapshots: two observations taken with the
    // same salt compare meaningfully, while two different salts cannot be
    // compared at all — the same `userID` produces unrelated digests, so the only
    // honest answer is `unknown`. Persist the salt with the snapshot.
    let first = identity_observation(&fx.home, &root, "persistent-salt").unwrap();
    let second = identity_observation(&fx.home, &root, "persistent-salt").unwrap();
    let same = identity_compare(&first["binding"], &second["binding"]);
    assert_eq!(same, json!({"userID": "same"}));
    assert_eq!(same.as_object().unwrap().len(), 1);

    // A different persistent identity is a change, not a leak.
    write(
        &root.join(".claude.json"),
        format!("{{\"userID\":\"syn-user-456\",\"oauthAccount\":{{}}}}").as_bytes(),
    );
    let changed = identity_observation(&fx.home, &root, "persistent-salt").unwrap();
    let comparison = identity_compare(&first["binding"], &changed["binding"]);
    assert_eq!(comparison, json!({"userID": "changed"}));
    let text = serde_json::to_string(&comparison).unwrap();
    assert!(!text.contains("sha256"), "{text}");

    // A salt mismatch also shows up as `changed`: the same value under a
    // different salt digests differently, and the comparison has no way to tell
    // that apart from a real identity change. This is exactly why the caller must
    // persist one salt with the snapshot and compare only within it.
    let resalted = identity_observation(&fx.home, &root, "other-salt").unwrap();
    assert_ne!(
        first["binding"]["digests"]["userID"],
        resalted["binding"]["digests"]["userID"]
    );
    assert_eq!(
        identity_compare(&resalted["binding"], &first["binding"]),
        json!({"userID": "changed"})
    );

    // A missing digest on either side is `unknown`, never `changed`.
    let absent = fixture();
    let missing = identity_observation(&absent.home, &absent.root, "persistent-salt").unwrap();
    assert_eq!(
        identity_compare(&first["binding"], &missing["binding"]),
        json!({"userID": "unknown"})
    );
    assert_eq!(
        identity_compare(&missing["binding"], &missing["binding"]),
        json!({"userID": "unknown"})
    );
}

#[test]
fn identity_observation_ignores_unknown_and_mistyped_fields() {
    let fixture = fixture();
    write(
        &fixture.root.join(".claude.json"),
        format!(
            "{{\"userID\":\"{SYNTHETIC_USER_ID}\",\"someOtherSecret\":\"{SYNTHETIC_SECRET}\",\"oauthAccount\":\"synthetic@example.invalid\"}}"
        )
        .as_bytes(),
    );
    let observed = identity_observation(&fixture.home, &fixture.root, "salt").unwrap();
    assert_eq!(observed["public"]["state"], "observed");
    let fields = observed["public"]["fields"].as_array().unwrap();
    assert_eq!(fields.len(), IDENTITY_KEYS.len());
    // A non-object `oauthAccount` is not recognized as account-system evidence.
    assert_eq!(fields[1]["present"], false);
    let text = serde_json::to_string(&observed).unwrap();
    assert!(!text.contains(SYNTHETIC_SECRET), "{text}");
    assert!(!text.contains("someOtherSecret"), "{text}");
    assert!(!text.contains("synthetic@example.invalid"), "{text}");
}

#[test]
fn inventory_skips_foreign_owner_entries() {
    let fixture = fixture();
    let root = &fixture.root;
    write(&root.join("todos/owned.json"), b"owned");
    let report = inventory(&fixture.home, root).unwrap();
    assert_eq!(location(&report, "todo_state")["file_count"], 1);
    // Guard the fixture itself: the temporary home really is owned by this user,
    // so `skipped_unsafe` above measured hard links rather than ownership.
    let meta = fs::symlink_metadata(root.join("todos/owned.json")).unwrap();
    assert_eq!(meta.uid(), unsafe { libc::geteuid() });
    assert_eq!(meta.nlink(), 1);
    fs::set_permissions(
        root.join("todos/owned.json"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
}

// ---- request-driven paged inventory ---------------------------------------

fn row_paths(report: &Value) -> Vec<String> {
    report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["path"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn inventory_request_pages_deterministically_without_losing_rows() {
    let fixture = fixture();
    let root = seed(&fixture);
    let first = inventory_request(&fixture.home, &root, &json!({"limit": 3})).unwrap();
    assert_eq!(first["limit"], 3);
    assert_eq!(first["offset"], 0);
    let total = first["matched_rows"].as_u64().unwrap();
    assert!(total > 3, "fixture too small to test paging: {total}");
    assert_eq!(first["rows"].as_array().unwrap().len(), 3);
    assert_eq!(first["page_complete"], false);
    let next = first["next_offset"].as_u64().unwrap();
    assert_eq!(next, 3);

    // Walk every page and prove the union is exactly the total with no repeats:
    // the order is stable, so the pages never overlap or drop a row.
    let mut seen: Vec<String> = row_paths(&first);
    let mut offset = next;
    loop {
        let page = inventory_request(
            &fixture.home,
            &root,
            &json!({"limit": 3, "offset": offset,"expected_digest":first["digest"]}),
        )
        .unwrap();
        assert_eq!(page["matched_rows"].as_u64().unwrap(), total);
        seen.extend(row_paths(&page));
        if page["page_complete"] == true {
            assert!(page["next_offset"].is_null());
            break;
        }
        offset = page["next_offset"].as_u64().unwrap();
    }
    let unique: std::collections::HashSet<&String> = seen.iter().collect();
    assert_eq!(
        unique.len(),
        total as usize,
        "paging repeated or dropped a row: {seen:?}"
    );
    let mut sorted = seen.clone();
    sorted.sort();
    assert_eq!(seen, sorted);
}

#[test]
fn inventory_request_filters_by_class_category_path_time_and_size() {
    let fixture = fixture();
    let root = seed(&fixture);
    let work = inventory_request(
        &fixture.home,
        &root,
        &json!({"class": "work", "category": "sessions"}),
    )
    .unwrap();
    assert!(work["matched_rows"].as_u64().unwrap() >= 1);
    assert!(row_paths(&work).iter().all(|p| p.contains("projects")));

    let scoped = inventory_request(
        &fixture.home,
        &root,
        &json!({"project": "projects/example"}),
    )
    .unwrap();
    assert!(scoped["matched_rows"].as_u64().unwrap() >= 2);
    assert!(row_paths(&scoped)
        .iter()
        .all(|p| p.contains("projects/example")));

    let large = inventory_request(&fixture.home, &root, &json!({"min_bytes": 6})).unwrap();
    assert!(large["rows"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["bytes"].as_u64().unwrap() >= 6));
    assert!(row_paths(&large)
        .iter()
        .all(|p| !p.ends_with("debug/a.log")));

    // A future `since` matches nothing but still reports a real zero total.
    let future = inventory_request(&fixture.home, &root, &json!({"since": 4102444800i64})).unwrap();
    assert_eq!(future["matched_rows"], 0);
    assert!(future["rows"].as_array().unwrap().is_empty());
    // A mistyped filter is refused rather than silently widened.
    assert_eq!(
        inventory_request(&fixture.home, &root, &json!({"class": 7}))
            .unwrap_err()
            .code,
        "invalid_filter"
    );
}

#[test]
fn inventory_request_reports_raw_body_directory_without_echoing_the_value() {
    let fixture = fixture();
    let root = seed(&fixture);
    let dump = root.join("otel-bodies");
    write(&dump.join("index.jsonl"), b"{\"body_ref\":\"synthetic\"}\n");
    write(
        &root.join("settings.json"),
        format!(
            "{{\"env\":{{\"OTEL_LOG_RAW_API_BODIES\":\"file:{}\",\"OTEL_EXPORTER_OTLP_HEADERS\":\"syn-secret-header\"}}}}",
            dump.to_string_lossy()
        )
        .as_bytes(),
    );
    let report = inventory_request(&fixture.home, &root, &json!({})).unwrap();
    let raw_body = &report["coverage"]["additional_sources"]["raw_body"];
    assert_eq!(raw_body["key"], RAW_BODY_KEY);
    assert_eq!(raw_body["configured"], true);
    assert_eq!(raw_body["present"], true);
    assert_eq!(raw_body["value_echoed"], false);
    assert_eq!(raw_body["directory"], dump.to_string_lossy().as_ref());
    // The configured directory is observed as a residue location, and no secret
    // from the same settings document appears anywhere in the report.
    let text = serde_json::to_string(&report).unwrap();
    assert!(!text.contains("syn-secret-header"), "{text}");
    assert!(report["locations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|location| location["category"] == "raw_body_dump"));
    assert!(row_paths(&report)
        .iter()
        .any(|p| p.ends_with("otel-bodies/index.jsonl")));
}

#[test]
fn inventory_request_reports_a_bare_inline_value_as_unconfigured() {
    let fixture = fixture();
    let root = seed(&fixture);
    write(
        &root.join("settings.json"),
        b"{\"env\":{\"OTEL_LOG_RAW_API_BODIES\":\"1\"}}",
    );
    let report = inventory_request(&fixture.home, &root, &json!({})).unwrap();
    let raw_body = &report["coverage"]["additional_sources"]["raw_body"];
    assert_eq!(raw_body["configured"], false);
    assert!(raw_body["directory"].is_null());
    assert!(report["locations"]
        .as_array()
        .unwrap()
        .iter()
        .all(|location| location["category"] != "raw_body_dump"));
}

#[test]
fn inventory_request_accepts_explicit_roots_and_validates_them() {
    let fixture = fixture();
    let root = seed(&fixture);
    let extra = fixture.home.join("elsewhere");
    write(&extra.join("cache/blob.bin"), b"explicit bytes");
    let report = inventory_request(
        &fixture.home,
        &root,
        &json!({"extra_roots": [extra.to_string_lossy()], "class": "unclassified"}),
    )
    .unwrap();
    let row = report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["origin"] == "request_extra_root")
        .unwrap();
    assert_eq!(report["matched_rows"], 1);
    assert_eq!(row["class"], "unclassified");
    assert_eq!(row["adapter"], "unclassified");
    assert_eq!(
        row["path"],
        extra.join("cache/blob.bin").to_string_lossy().as_ref()
    );

    // A relative, non-directory, duplicated or over-limit list is refused.
    assert_eq!(
        inventory_request(
            &fixture.home,
            &root,
            &json!({"extra_roots": ["relative/dir"]})
        )
        .unwrap_err()
        .code,
        "invalid_extra_roots"
    );
    assert_eq!(
        inventory_request(
            &fixture.home,
            &root,
            &json!({"extra_roots": [root.join("debug/a.log").to_string_lossy()]})
        )
        .unwrap_err()
        .code,
        "invalid_extra_roots"
    );
    assert_eq!(
        inventory_request(
            &fixture.home,
            &root,
            &json!({"extra_roots": [extra.to_string_lossy(), extra.to_string_lossy()]})
        )
        .unwrap_err()
        .code,
        "invalid_extra_roots"
    );
    let many: Vec<String> = (0..9).map(|n| format!("/tmp/synthetic-{n}")).collect();
    assert_eq!(
        inventory_request(&fixture.home, &root, &json!({"extra_roots": many}))
            .unwrap_err()
            .code,
        "invalid_extra_roots"
    );
}

#[test]
fn inventory_request_rejects_bad_paging_and_reports_coverage_independently() {
    let fixture = fixture();
    let root = seed(&fixture);
    assert_eq!(
        inventory_request(&fixture.home, &root, &json!({"limit": 0}))
            .unwrap_err()
            .code,
        "invalid_limit"
    );
    assert_eq!(
        inventory_request(&fixture.home, &root, &json!({"limit": 201}))
            .unwrap_err()
            .code,
        "invalid_limit"
    );
    let first = inventory_request(&fixture.home, &root, &json!({"limit": 2})).unwrap();
    assert_eq!(
        inventory_request(&fixture.home, &root, &json!({"limit": 2, "offset": 100000}))
            .unwrap_err()
            .code,
        "invalid_offset"
    );
    // The seed truncates shell-snapshots (row cap) and session-env (depth), so
    // coverage is false even though this small page returned successfully.
    assert_eq!(first["coverage"]["scan_complete"], false);
    assert!(!first["coverage"]["truncated_locations"]
        .as_array()
        .unwrap()
        .is_empty());
}

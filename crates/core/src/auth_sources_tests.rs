use super::*;
use std::os::unix::fs::{symlink, PermissionsExt};

const MARKER: &str = "synthetic-auth-secret-MUST-NOT-EXPORT";

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let home = base.join("home");
    let root = home.join(".claude");
    fs::create_dir_all(&root).unwrap();
    (temp, home, root)
}

fn view(home: &Path, root: &Path) -> Value {
    scan(home, root, None, &|_| None).unwrap().1
}

fn source_named(view: &Value, kind: &str, name: &str) -> Value {
    view["sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|source| source["kind"] == kind && source["name"] == name)
        .unwrap()
        .clone()
}

fn selected(view: &Value, kinds: &[&str]) -> Vec<String> {
    view["sources"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|source| {
            source["removable"] == true && kinds.contains(&source["kind"].as_str().unwrap())
        })
        .map(|source| source["id"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn inventory_distinguishes_candidates_context_profiles_and_unknowns_without_values() {
    let (_temp, home, root) = fixture();
    fs::write(
        root.join(".credentials.json"),
        json!({"claudeAiOauth":{"accessToken":MARKER},"mcpOAuth":{"preserve":MARKER}}).to_string(),
    )
    .unwrap();
    fs::write(
        home.join(".claude.json"),
        json!({"primaryApiKey":MARKER,"mcpServers":{"keep":{}}}).to_string(),
    )
    .unwrap();
    fs::write(root.join("settings.json"),json!({"env":{"ANTHROPIC_API_KEY":MARKER,"ANTHROPIC_PROFILE":"name-must-not-export"},"apiKeyHelper":"secret-helper-command-must-not-export"}).to_string()).unwrap();
    fs::write(
        home.join(".zshrc"),
        format!("export CLAUDE_CODE_OAUTH_TOKEN='{MARKER}'\n"),
    )
    .unwrap();
    let profile = home.join("profiles");
    fs::create_dir_all(&profile).unwrap();
    let (_scan, report) = scan(&home, &root, None, &|name| match name {
        "CLAUDE_CODE_OAUTH_TOKEN" => Some(OsString::from(MARKER)),
        "CLAUDE_CONFIG_DIR" => Some(OsString::from("/different/synthetic/root")),
        "ANTHROPIC_CONFIG_DIR" => Some(profile.as_os_str().to_owned()),
        _ => None,
    })
    .unwrap();
    let serialized = report.to_string();
    for secret in [
        MARKER,
        "name-must-not-export",
        "secret-helper-command-must-not-export",
        "/different/synthetic/root",
    ] {
        assert!(!serialized.contains(secret));
    }
    assert_eq!(report["context"]["inherited_config_dir"], "different");
    assert_eq!(
        source_named(&report, "process_env", "CLAUDE_CODE_OAUTH_TOKEN")["evidence"],
        "observed_context"
    );
    assert_eq!(
        source_named(&report, "credential_field", "claudeAiOauth")["default_selected"],
        true
    );
    assert_eq!(
        source_named(&report, "client_state_field", "primaryApiKey")["default_selected"],
        true
    );
    assert_eq!(
        source_named(&report, "settings_env", "ANTHROPIC_API_KEY")["default_selected"],
        false
    );
    assert_eq!(
        source_named(&report, "settings_env", "ANTHROPIC_PROFILE")["removable"],
        false
    );
    assert_eq!(
        source_named(&report, "keychain", "Keychain")["evidence"],
        "unknown"
    );
    assert_eq!(
        source_named(&report, "keychain", "Keychain")["presence"],
        "unknown"
    );
    assert_eq!(report["limits"]["coverage_complete"], false);
    assert!(report["sources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["path"] == json!(profile) && s["presence"] == "present"));
}

#[test]
fn frozen_json_transforms_remove_only_selected_fields_and_preserve_mcp_and_work() {
    let (_temp, home, root) = fixture();
    let credentials = json!({"claudeAiOauth":{"accessToken":MARKER,"refreshToken":"also-secret"},"mcpOAuth":{"server":"keep"},"pluginSecrets":{"keep":"private-other"}});
    let settings = json!({"env":{"ANTHROPIC_API_KEY":MARKER,"KEEP":"keep","ANTHROPIC_PROFILE":"shared"},"apiKeyHelper":"private-helper","mcpServers":{"keep":{}},"hooks":{"keep":[]}});
    fs::write(root.join(".credentials.json"), credentials.to_string()).unwrap();
    fs::write(root.join("settings.json"), settings.to_string()).unwrap();
    let global = json!({"primaryApiKey":MARKER,"userID":"preserve-identity","projects":{"keep":{}},"mcpServers":{"keep":{}}});
    fs::write(home.join(".claude.json"), global.to_string()).unwrap();
    let inventory = view(&home, &root);
    let ids = selected(
        &inventory,
        &[
            "credential_field",
            "client_state_field",
            "settings_env",
            "api_key_helper",
        ],
    );
    let frozen = freeze(&home, &root, None, &ids, "nonce-one").unwrap();
    for value in [MARKER, "also-secret", "private-helper", "private-other"] {
        assert!(!frozen.to_string().contains(value));
    }
    check(&frozen["groups"], "nonce-one").unwrap();
    for group in frozen["groups"].as_array().unwrap() {
        let bytes = replacement(group, "nonce-one").unwrap();
        let doc: Value = serde_json::from_slice(&bytes).unwrap();
        let name = Path::new(group["path"].as_str().unwrap())
            .file_name()
            .unwrap();
        if name == ".credentials.json" {
            let mut expected = credentials.clone();
            expected.as_object_mut().unwrap().remove("claudeAiOauth");
            assert_eq!(doc, expected);
        } else if name == "settings.json" {
            let mut expected = settings.clone();
            expected["env"]
                .as_object_mut()
                .unwrap()
                .remove("ANTHROPIC_API_KEY");
            expected.as_object_mut().unwrap().remove("apiKeyHelper");
            assert_eq!(doc, expected);
        } else {
            let mut expected = global.clone();
            expected.as_object_mut().unwrap().remove("primaryApiKey");
            assert_eq!(doc, expected);
        }
        fs::write(group["path"].as_str().unwrap(), bytes).unwrap();
    }
    for item in frozen["sources"].as_array().unwrap() {
        assert_eq!(source_state(item).unwrap(), "removed");
    }
    assert!(check(&frozen["groups"], "nonce-one").is_err());
}

#[test]
fn shell_literals_in_ordinary_config_are_exact_and_unrelated_bytes_survive() {
    let (_temp, home, root) = fixture();
    let before=format!("# credentials\r\nsource ~/.local/shell-init\r\nexport PATH=\"$HOME/bin:$PATH\"\r\nexport CLAUDE_CODE_OAUTH_TOKEN='{MARKER}' # own token\r\nALIAS=keep\r\nANTHROPIC_API_KEY=another-synthetic-token\r\nexport CLAUDE_CODE_OAUTH_TOKEN=second-synthetic-token\r\n");
    let path = home.join(".zshrc");
    fs::write(&path, &before).unwrap();
    let report = view(&home, &root);
    let oauth = source_named(&report, "shell_assignment", "CLAUDE_CODE_OAUTH_TOKEN");
    assert_eq!(oauth["removable"], true);
    assert_eq!(oauth["locations"], json!([{"line":4},{"line":7}]));
    assert_eq!(oauth["scope"], "host_shared");
    let frozen = freeze(
        &home,
        &root,
        None,
        &[oauth["id"].as_str().unwrap().to_owned()],
        "nonce",
    )
    .unwrap();
    let group = &frozen["groups"][0];
    let after = replacement(group, "nonce").unwrap();
    assert_eq!(std::str::from_utf8(&after).unwrap(),"# credentials\r\nsource ~/.local/shell-init\r\nexport PATH=\"$HOME/bin:$PATH\"\r\nALIAS=keep\r\nANTHROPIC_API_KEY=another-synthetic-token\r\n");
    fs::write(&path, after).unwrap();
    assert_eq!(source_state(&oauth).unwrap(), "removed");
    let second = view(&home, &root);
    assert_eq!(
        source_named(&report, "shell_assignment", "ANTHROPIC_API_KEY")["id"],
        source_named(&second, "shell_assignment", "ANTHROPIC_API_KEY")["id"]
    );
}

#[test]
fn ambiguous_shell_never_becomes_an_automatic_edit() {
    let cases = [
        "export CLAUDE_CODE_OAUTH_TOKEN=$(fetch-token)\n",
        "export CLAUDE_CODE_OAUTH_TOKEN=literal; echo keep\n",
        "export CLAUDE_CODE_OAUTH_TOKEN=literal other-command\n",
        "if true; then\nexport CLAUDE_CODE_OAUTH_TOKEN=literal\nfi\n",
        "foo() {\nexport CLAUDE_CODE_OAUTH_TOKEN=literal\n}\n",
        "cat <<'EOF'\nexport CLAUDE_CODE_OAUTH_TOKEN=literal\nEOF\n",
        "VALUE='multi\nexport CLAUDE_CODE_OAUTH_TOKEN=literal\nline'\n",
        "printf value \\\nexport CLAUDE_CODE_OAUTH_TOKEN=literal\n",
        "eval \"$(setup)\"\nexport CLAUDE_CODE_OAUTH_TOKEN=literal\n",
        "export CLAUDE_CODE_OAUTH_TOKEN=\"${OLD_TOKEN}\"\n",
        "some_command &&\nexport CLAUDE_CODE_OAUTH_TOKEN=literal\nkeep_command\n",
        "printf source |\nexport CLAUDE_CODE_OAUTH_TOKEN=literal\nkeep_command\n",
        "time\nexport CLAUDE_CODE_OAUTH_TOKEN=literal\nkeep_command\n",
        "repeat 3\nexport CLAUDE_CODE_OAUTH_TOKEN=literal\nkeep_command\n",
    ];
    for text in cases {
        let parsed = shell_declarations(text);
        let declaration = parsed.declarations.get("CLAUDE_CODE_OAUTH_TOKEN").unwrap();
        assert!(
            !parsed.simple || !declaration.exact,
            "unexpected editable shell: {text}"
        );
    }
    let (_temp, home, root) = fixture();
    fs::write(home.join(".zshrc"), cases[3]).unwrap();
    let report = view(&home, &root);
    let item = source_named(&report, "shell_assignment", "CLAUDE_CODE_OAUTH_TOKEN");
    assert_eq!(item["removable"], false);
    assert_eq!(
        freeze(
            &home,
            &root,
            None,
            &[item["id"].as_str().unwrap().to_owned()],
            "nonce"
        )
        .unwrap_err()
        .code,
        "auth_source_not_removable"
    );
}

#[test]
fn external_edits_reject_frozen_content_even_with_structurally_stable_source_ids() {
    let (_temp, home, root) = fixture();
    let path = home.join(".bashrc");
    fs::write(&path, "export ANTHROPIC_API_KEY=old-token\n").unwrap();
    let report = view(&home, &root);
    let item = source_named(&report, "shell_assignment", "ANTHROPIC_API_KEY");
    let frozen = freeze(
        &home,
        &root,
        None,
        &[item["id"].as_str().unwrap().to_owned()],
        "nonce",
    )
    .unwrap();
    fs::write(&path, "export ANTHROPIC_API_KEY=new-token\n").unwrap();
    assert_eq!(
        item["id"],
        source_named(&view(&home, &root), "shell_assignment", "ANTHROPIC_API_KEY")["id"]
    );
    assert_eq!(source_state(&item).unwrap(), "still_present");
    assert!(check(&frozen["groups"], "nonce").is_err());
    assert!(replacement(&frozen["groups"][0], "nonce").is_err());
}

#[test]
fn exact_post_read_does_not_turn_access_failure_or_dynamic_shell_into_absence() {
    let (_temp, home, root) = fixture();
    let path = home.join(".bashrc");
    fs::write(&path, "export ANTHROPIC_API_KEY=old-token\n").unwrap();
    let item = source_named(&view(&home, &root), "shell_assignment", "ANTHROPIC_API_KEY");
    fs::write(&path, "source $(dynamic-profile)\n").unwrap();
    assert_eq!(source_state(&item).unwrap(), "unknown");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
    assert_eq!(source_state(&item).unwrap(), "unknown");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::remove_file(&path).unwrap();
    assert_eq!(source_state(&item).unwrap(), "removed");
}

#[test]
fn permission_links_limits_and_unknown_json_are_explicit_and_never_removable() {
    let (_temp, home, root) = fixture();
    let outside = home.join("secret-target");
    fs::write(&outside, format!("export ANTHROPIC_API_KEY={MARKER}\n")).unwrap();
    symlink(&outside, home.join(".zshrc")).unwrap();
    fs::hard_link(&outside, home.join(".bashrc")).unwrap();
    fs::write(home.join(".profile"), vec![b'x'; SHELL_BYTES as usize + 1]).unwrap();
    fs::write(
        root.join(".credentials.json"),
        format!("{{\"claudeAiOauth\":{{\"accessToken\":\"{MARKER}\"}},\"claudeAiOauth\":{{}}}}"),
    )
    .unwrap();
    let report = view(&home, &root);
    assert_eq!(report["limits"]["bounded_scan_complete"], false);
    assert_eq!(
        source_named(&report, "shell_assignment", "ANTHROPIC_API_KEY")["removable"],
        false
    );
    assert!(!report.to_string().contains(MARKER));
    assert!(report["sources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["path"] == json!(home.join(".zshrc")) && s["evidence"] == "inaccessible"));
    assert!(report["sources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["path"] == json!(home.join(".profile")) && s["reason"] == "file_too_large"));
    assert!(!report["sources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["kind"] == "credential_field" && s["removable"] == true));
    assert_eq!(
        fs::read_to_string(&outside).unwrap(),
        format!("export ANTHROPIC_API_KEY={MARKER}\n")
    );
}

#[test]
fn custom_root_global_location_and_project_declarations_are_kept_separate() {
    let (temp, home, _default_root) = fixture();
    let root = home.join("custom");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        home.join(".claude.json"),
        json!({"primaryApiKey":"neighbor-key"}).to_string(),
    )
    .unwrap();
    fs::write(
        root.join(".claude.json"),
        json!({"primaryApiKey":MARKER}).to_string(),
    )
    .unwrap();
    let project = temp.path().canonicalize().unwrap().join("project");
    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::write(
        project.join(".claude/settings.local.json"),
        json!({"env":{"CLAUDE_CODE_OAUTH_TOKEN":MARKER,"ANTHROPIC_API_KEY":MARKER}}).to_string(),
    )
    .unwrap();
    let report = scan(&home, &root, Some(&project), &|_| None).unwrap().1;
    assert_eq!(
        source_named(&report, "client_state_field", "primaryApiKey")["path"],
        json!(root.join(".claude.json"))
    );
    let project_token = source_named(&report, "settings_env", "CLAUDE_CODE_OAUTH_TOKEN");
    assert_eq!(project_token["scope"], "project");
    assert_eq!(project_token["removable"], false);
    assert_eq!(project_token["reason"], "project_token_ignored");
    assert!(!report.to_string().contains("neighbor-key"));
    assert!(!report.to_string().contains(MARKER));
}

#[test]
fn parent_replacement_is_rejected_but_same_inode_quarantine_is_comparable() {
    let (_temp, home, root) = fixture();
    let path = root.join(".credentials.json");
    fs::write(
        &path,
        json!({"claudeAiOauth":{"accessToken":MARKER}}).to_string(),
    )
    .unwrap();
    let ids = selected(&view(&home, &root), &["credential_field"]);
    let frozen = freeze(&home, &root, None, &ids, "nonce").unwrap();
    let group = &frozen["groups"][0];
    let held = home.join("held");
    fs::create_dir(&held).unwrap();
    fs::rename(&path, held.join("original")).unwrap();
    assert!(matches_snapshot(&held.join("original"), &group["snapshot"], "nonce").unwrap());
    fs::rename(held.join("original"), &path).unwrap();
    fs::rename(&root, home.join("old-root")).unwrap();
    fs::create_dir(&root).unwrap();
    fs::rename(home.join("old-root/.credentials.json"), &path).unwrap();
    assert!(matches_snapshot(&path, &group["snapshot"], "nonce").unwrap());
    assert!(check(&frozen["groups"], "nonce").is_err());
    assert!(replacement(group, "nonce").is_err());
}

#[test]
fn salted_freezes_empty_selection_and_missing_files_have_precise_semantics() {
    let (_temp, home, root) = fixture();
    fs::write(
        home.join(".lintel-test-keychain.json"),
        json!({"schema":"lintel.test-keychain/1","items":{}}).to_string(),
    )
    .unwrap();
    let report = view(&home, &root);
    assert_eq!(report["limits"]["bounded_scan_complete"], true);
    let empty = freeze(&home, &root, None, &[], "nonce").unwrap();
    assert_eq!(empty["groups"], json!([]));
    fs::write(
        root.join(".credentials.json"),
        json!({"claudeAiOauth":{"accessToken":MARKER}}).to_string(),
    )
    .unwrap();
    let ids = selected(&view(&home, &root), &["credential_field"]);
    let first = freeze(&home, &root, None, &ids, "nonce-one").unwrap();
    let second = freeze(&home, &root, None, &ids, "nonce-two").unwrap();
    assert_ne!(
        first["groups"][0]["snapshot"]["digest"],
        second["groups"][0]["snapshot"]["digest"]
    );
    assert_eq!(first["sources"], second["sources"]);
    assert!(!first.to_string().contains(MARKER));
    assert!(freeze(
        &home,
        &root,
        None,
        &[ids[0].clone(), ids[0].clone()],
        "nonce"
    )
    .is_err());
}

#[test]
fn empty_values_are_not_claimed_as_effective_auth_and_synthetic_profiles_do_not_escape() {
    let (_temp, home, root) = fixture();
    fs::write(
        root.join("settings.json"),
        json!({"env":{"ANTHROPIC_API_KEY":"","ANTHROPIC_AUTH_TOKEN":false}}).to_string(),
    )
    .unwrap();
    let report = scan(&home, &root, None, &|name| match name {
        "CLAUDE_CODE_OAUTH_TOKEN" => Some(OsString::new()),
        "ANTHROPIC_CONFIG_DIR" => Some(OsString::from(
            "/outside-synthetic-fixture-must-not-traverse",
        )),
        _ => None,
    })
    .unwrap()
    .1;
    let env = source_named(&report, "process_env", "CLAUDE_CODE_OAUTH_TOKEN");
    assert_eq!(env["presence"], "present");
    assert_eq!(env["value_state"], "empty");
    assert_eq!(env["actual_use"], "unverified");
    assert_eq!(
        source_named(&report, "settings_env", "ANTHROPIC_API_KEY")["value_state"],
        "empty"
    );
    assert_eq!(
        source_named(&report, "settings_env", "ANTHROPIC_AUTH_TOKEN")["value_state"],
        "non_string"
    );
    assert!(!report
        .to_string()
        .contains("outside-synthetic-fixture-must-not-traverse"));
    assert!(
        report["sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["kind"] == "profile"
                && source["reason"] == "invalid_profile_pointer")
    );
}

#[test]
fn a_variable_name_mentioned_as_text_is_not_an_observed_assignment() {
    let (_temp, home, root) = fixture();
    fs::write(home.join(".zshrc"), "printf 'ANTHROPIC_API_KEY=example'\n").unwrap();
    let report = view(&home, &root);
    let item = source_named(&report, "shell_assignment", "ANTHROPIC_API_KEY");
    assert_eq!(item["evidence"], "unknown");
    assert_eq!(item["removable"], false);
    fs::write(
        home.join(".zshrc"),
        "export ANTHROPIC_API_KEY=$(read-secret)\n",
    )
    .unwrap();
    let item = source_named(&view(&home, &root), "shell_assignment", "ANTHROPIC_API_KEY");
    assert_eq!(item["evidence"], "declared");
    assert_eq!(item["removable"], false);
}

fn seed_keychain(home: &Path, root: &Path, data: Value) -> String {
    let service = keychain::services(home, root).last().unwrap().clone();
    fs::write(home.join(".lintel-test-keychain.json"), json!({"schema":"lintel.test-keychain/1","items":{&service:{"id":"synthetic-original-item","data":data}}}).to_string()).unwrap();
    service
}

#[test]
fn keychain_default_and_custom_services_freeze_exact_container_without_exporting_secrets() {
    for custom in [false, true] {
        let (_temp, home, default) = fixture();
        let root = if custom { home.join("custom") } else { default };
        fs::create_dir_all(&root).unwrap();
        let before = json!({"claudeAiOauth":{"accessToken":MARKER},"mcpOAuth":{"server":"secret-mcp"},"pluginSecrets":{"x":"secret-plugin"},"unknown":{"future":[1,2,3]}});
        let service = seed_keychain(&home, &root, before.clone());
        let inventory = view(&home, &root);
        let source = source_named(&inventory, "keychain_field", "claudeAiOauth");
        assert_eq!(source["default_selected"], true);
        assert_eq!(source["service"], service);
        assert!(source.get("path").is_none());
        assert!(source.get("identity").is_none());
        let frozen = freeze(
            &home,
            &root,
            None,
            &selected(&inventory, &["keychain_field"]),
            "nonce",
        )
        .unwrap();
        for secret in [MARKER, "secret-mcp", "secret-plugin"] {
            assert!(!inventory.to_string().contains(secret));
            assert!(!frozen.to_string().contains(secret));
        }
        let group = &frozen["groups"][0];
        check(&frozen["groups"], "nonce").unwrap();
        let original = keychain_original(group, "nonce").unwrap();
        let replacement = keychain_replacement(group, &original).unwrap();
        let mut expected = before;
        expected.as_object_mut().unwrap().remove("claudeAiOauth");
        assert_eq!(
            serde_json::from_slice::<Value>(&replacement).unwrap(),
            expected
        );
        keychain::replace(&home, &service, &original, &replacement).unwrap();
        assert_eq!(source_state_bound(&source, group).unwrap(), "removed");
    }
}

#[test]
fn keychain_external_oauth_neighbor_or_item_changes_reject_original_approval() {
    for change in ["oauth", "mcp", "identity"] {
        let (_temp, home, root) = fixture();
        let service = seed_keychain(
            &home,
            &root,
            json!({"claudeAiOauth":{"accessToken":MARKER},"mcpOAuth":{"keep":"before"}}),
        );
        let inventory = view(&home, &root);
        let frozen = freeze(
            &home,
            &root,
            None,
            &selected(&inventory, &["keychain_field"]),
            "nonce",
        )
        .unwrap();
        let path = home.join(".lintel-test-keychain.json");
        let mut fixture: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        match change {
            "oauth" => {
                fixture["items"][&service]["data"]["claudeAiOauth"]["accessToken"] =
                    json!("new-oauth")
            }
            "mcp" => fixture["items"][&service]["data"]["mcpOAuth"]["keep"] = json!("new-mcp"),
            _ => fixture["items"][&service]["id"] = json!("replacement-item"),
        }
        fs::write(path, fixture.to_string()).unwrap();
        assert!(check(&frozen["groups"], "nonce").is_err(), "{change}");
        assert!(keychain_original(&frozen["groups"][0], "nonce").is_err());
    }
}

#[test]
fn keychain_locked_denied_and_unrecognized_containers_are_unknown_not_absent() {
    let (_temp, home, root) = fixture();
    for code in ["keychain_locked", "keychain_access_denied"] {
        fs::write(
            home.join(".lintel-test-keychain.json"),
            json!({"schema":"lintel.test-keychain/1","items":{},"error":code}).to_string(),
        )
        .unwrap();
        let inventory = view(&home, &root);
        let source = source_named(&inventory, "keychain", "Keychain");
        assert_eq!(source["presence"], "unknown");
        assert_eq!(source["reason"], code);
        assert!(selected(&inventory, &["keychain_field"]).is_empty());
    }
    seed_keychain(
        &home,
        &root,
        json!({"claudeAiOauth":"not-recognized","mcpOAuth":{}}),
    );
    let source = source_named(&view(&home, &root), "keychain_field", "claudeAiOauth");
    assert_eq!(source["removable"], false);
}

#[test]
fn keychain_verification_distinguishes_replacement_item_from_absent_login_field() {
    let (_temp, home, root) = fixture();
    let service = seed_keychain(
        &home,
        &root,
        json!({"claudeAiOauth":{"accessToken":MARKER},"mcpOAuth":{}}),
    );
    let inventory = view(&home, &root);
    let frozen = freeze(
        &home,
        &root,
        None,
        &selected(&inventory, &["keychain_field"]),
        "nonce",
    )
    .unwrap();
    let source = &frozen["sources"][0];
    let group = &frozen["groups"][0];
    let path = home.join(".lintel-test-keychain.json");
    let mut fixture: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    fixture["items"][&service]["id"] = json!("new-item");
    fixture["items"][&service]["data"]
        .as_object_mut()
        .unwrap()
        .remove("claudeAiOauth");
    fs::write(&path, fixture.to_string()).unwrap();
    assert_eq!(source_state_bound(source, group).unwrap(), "unknown");
    fixture["items"][&service]["data"]["claudeAiOauth"] = json!({"accessToken":"new-token"});
    fs::write(&path, fixture.to_string()).unwrap();
    assert_eq!(source_state_bound(source, group).unwrap(), "still_present");
}

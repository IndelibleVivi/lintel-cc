use super::*;
use crate::Engine;
use std::fs;

struct NativeScope;
impl Drop for NativeScope {
    fn drop(&mut self) {
        NATIVE_FIXTURE.with(|slot| *slot.borrow_mut() = None);
    }
}

fn ok(engine: &Engine, request: Value) -> Value {
    let response = engine.request(request);
    assert_eq!(response["ok"], true, "{}", response["error"]);
    response["data"].clone()
}

#[test]
fn keychain_service_paths_use_the_clients_nfc_normalization() {
    assert_eq!(
        macos::normalize("/synthetic/cafe\u{301}"),
        "/synthetic/café"
    );
    let home = Path::new("/synthetic/home");
    let default = services(home, &home.join(".claude"));
    assert_eq!(default.len(), 2);
    assert_eq!(default[0], SERVICE);
    let custom = services(home, &home.join("cafe\u{301}"));
    assert_eq!(
        custom,
        vec![format!(
            "{SERVICE}-{}",
            &storage::digest("/synthetic/home/café".as_bytes())[..8]
        )]
    );
}

#[test]
#[ignore = "requires an explicitly selected disposable GitHub-hosted macOS account"]
fn keychain_native_engine_journey() {
    // The normal local test group never creates or opens a Keychain. Creating
    // a legacy Keychain can affect its account's search list, so this fixture
    // belongs exclusively to the disposable hosted-runner account.
    assert_eq!(std::env::var("GITHUB_ACTIONS").as_deref(), Ok("true"));
    assert_eq!(
        std::env::var("RUNNER_ENVIRONMENT").as_deref(),
        Ok("github-hosted")
    );
    let temporary = tempfile::tempdir().unwrap();
    let base = temporary.path().canonicalize().unwrap();
    let home = base.join("home");
    fs::create_dir_all(&home).unwrap();
    let database_path = base.join("synthetic-login.keychain");
    let database = macos::TestKeychain::create(&database_path).unwrap();
    NATIVE_FIXTURE.with(|slot| *slot.borrow_mut() = Some((home.clone(), database_path.clone())));
    let scope = NativeScope;
    let mut engine = Engine::new(home.clone(), base.join("state")).unwrap();
    engine.executable_search_path = Some(base.join("empty-path").into_os_string());

    for (index, directory) in [".claude", "custom café"].iter().enumerate() {
        let root = home.join(directory);
        fs::create_dir_all(&root).unwrap();
        let original = json!({"claudeAiOauth":{"accessToken":"SYNTHETIC_NATIVE_LOGIN","refreshToken":"SYNTHETIC_NATIVE_REFRESH"},"mcpOAuth":{"keep":"SYNTHETIC_NATIVE_MCP"},"pluginSecrets":{"keep":"SYNTHETIC_NATIVE_PLUGIN"},"unknown":{"preserve":[1,true,"原文"]}});
        let service = services(&home, &root)[0].clone();
        database
            .insert(&service, &serde_json::to_vec(&original).unwrap())
            .unwrap();
        let before = read(&home, &service).unwrap().unwrap();
        storage::save(&root.join(".credentials.json"), &json!({"claudeAiOauth":{"accessToken":"SYNTHETIC_FILE_LOGIN"},"mcpOAuth":{"keep":"SYNTHETIC_FILE_MCP"}})).unwrap();
        let environment = engine
            .register(&format!("Synthetic native Keychain {index}"), &root, false)
            .unwrap();
        let inventory = ok(
            &engine,
            json!({"command":"auth_inventory","environment_id":environment["id"]}),
        );
        assert!(inventory["sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["kind"] == "keychain_field"
                && source["service"] == service
                && source["default_selected"] == true));
        let plan = ok(
            &engine,
            json!({"command":"plan_cleanup","environment_id":environment["id"],"recipe":"repair_login","official_logout":false,"writers_confirmed_stopped":true}),
        );
        assert_eq!(
            plan["authentication"]["selected_sources"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert!(!plan.to_string().contains("persistent_ref"));
        assert!(!plan.to_string().contains("SYNTHETIC_NATIVE_LOGIN"));
        let receipt = ok(
            &engine,
            json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"]}),
        );
        assert_eq!(receipt["local_cleanup"], "completed");
        let after = read_bound(&home, &service, &before.identity)
            .unwrap()
            .unwrap();
        assert_eq!(after.identity, before.identity);
        let actual = storage::parse(&after.bytes).unwrap();
        let mut expected = original.clone();
        expected.as_object_mut().unwrap().remove("claudeAiOauth");
        assert_eq!(actual, expected);
        assert_eq!(
            storage::load(&root.join(".credentials.json")).unwrap(),
            json!({"mcpOAuth":{"keep":"SYNTHETIC_FILE_MCP"}})
        );

        // An independent client writes a new login. An original-ID replay and
        // a fresh verification must observe it, never perform another update.
        let mut newer = actual;
        newer["claudeAiOauth"] = json!({"accessToken":"SYNTHETIC_NEW_NATIVE_LOGIN"});
        macos::update(
            &service,
            &after.identity,
            &serde_json::to_vec(&newer).unwrap(),
            Some(&database_path),
        )
        .unwrap();
        let replay = ok(
            &engine,
            json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"]}),
        );
        assert_eq!(replay["id"], receipt["id"]);
        assert_eq!(
            storage::parse(&read(&home, &service).unwrap().unwrap().bytes).unwrap(),
            newer
        );
        let fresh = ok(
            &engine,
            json!({"command":"auth_verify","environment_id":environment["id"],"job_id":receipt["id"]}),
        );
        assert!(
            fresh["local_sources"]
                .as_array()
                .unwrap()
                .iter()
                .any(|source| source["kind"] == "keychain_field"
                    && source["state"] == "still_present")
        );

        let stale = ok(
            &engine,
            json!({"command":"plan_cleanup","environment_id":environment["id"],"recipe":"repair_login","official_logout":false,"writers_confirmed_stopped":true}),
        );
        newer["mcpOAuth"]["external_edit"] = json!(true);
        macos::update(
            &service,
            &after.identity,
            &serde_json::to_vec(&newer).unwrap(),
            Some(&database_path),
        )
        .unwrap();
        let rejected = engine
            .request(json!({"command":"execute","plan_id":stale["id"],"approval":stale["hash"]}));
        assert_eq!(rejected["ok"], false);
        assert!(!engine.path("jobs", stale["id"].as_str().unwrap()).exists());
        assert_eq!(
            storage::parse(&read(&home, &service).unwrap().unwrap().bytes).unwrap(),
            newer
        );
    }

    for directory in ["plans", "jobs"] {
        for entry in fs::read_dir(engine.state.join(directory)).unwrap() {
            let text = fs::read_to_string(entry.unwrap().path()).unwrap();
            for secret in [
                "SYNTHETIC_NATIVE_LOGIN",
                "SYNTHETIC_NATIVE_REFRESH",
                "SYNTHETIC_NATIVE_MCP",
                "SYNTHETIC_NATIVE_PLUGIN",
                "SYNTHETIC_NEW_NATIVE_LOGIN",
                "SYNTHETIC_FILE_LOGIN",
                "SYNTHETIC_FILE_MCP",
            ] {
                assert!(
                    !text.contains(secret),
                    "credential body entered persisted metadata"
                );
            }
        }
    }
    drop(scope);
    database.close().unwrap();
    println!("PASS: isolated native Keychain + production Engine, default/custom roots, one approval, preserved neighbours, stale edits and original-ID no replay");
}

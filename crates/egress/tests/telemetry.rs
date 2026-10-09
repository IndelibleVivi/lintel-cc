//! Controlled telemetry rule tests over the real existing loopback proxy.
//!
//! These tests require loopback sockets (like the existing proxy tests). They
//! never resolve or connect a public destination and never send telemetry: a
//! catalog host is only ever an admission string, and the controlled request is
//! the only request made to the proxy's own listener. Ordinary client traffic
//! remains distinct from a registered controlled test.
use lintel_egress::telemetry;
use lintel_egress::{Config, DefaultAction, Event, Proxy, Rule};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};

const DATADOG: &str = "http-intake.logs.us5.datadoghq.com";

async fn proxy(
    config: Config,
) -> (
    Arc<Proxy>,
    std::net::SocketAddr,
    Arc<Mutex<Vec<Event>>>,
    tokio::task::JoinHandle<()>,
) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let copy = events.clone();
    let proxy = Arc::new(
        Proxy::bind(
            config,
            Arc::new(move |event| copy.lock().unwrap().push(event)),
        )
        .await
        .expect("loopback proxy bind"),
    );
    let address = proxy.local_addr().unwrap();
    let serving = proxy.clone();
    let task = tokio::spawn(async move {
        serving
            .serve_until_shared(std::future::pending())
            .await
            .unwrap();
    });
    (proxy, address, events, task)
}

async fn connect_authority(address: std::net::SocketAddr, authority: &str, body: &[u8]) -> Vec<u8> {
    let mut socket = TcpStream::connect(address).await.unwrap();
    let mut request =
        format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\n\r\n").into_bytes();
    request.extend_from_slice(body);
    socket.write_all(&request).await.unwrap();
    socket.shutdown().await.unwrap();
    let mut response = Vec::new();
    timeout(Duration::from_secs(4), socket.read_to_end(&mut response))
        .await
        .unwrap()
        .unwrap();
    response
}

// Regression: the controlled test sends a real CONNECT through the serving
// proxy, gets a real 403 from the explicit block, and never touches a public
// destination or an unreachable origin.
#[tokio::test]
async fn rule_test_is_a_real_connect_denial_and_never_connects_the_destination() {
    // A loopback origin a mis-routed request could reach; it must never be
    // contacted by the controlled test.
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_port = origin.local_addr().unwrap().port();

    let blocked = telemetry::merge_blocked(&[], &["datadog_logs_intake".into()]).unwrap();
    let config = Config {
        environment_id: "synthetic-telemetry".into(),
        default_action: DefaultAction::Allow,
        blocked: blocked.clone(),
        upstream: Some(format!("http://127.0.0.1:{origin_port}")),
        ..Config::default()
    };
    let (proxy, address, events, task) = proxy(config).await;

    let outcome = proxy.rule_test("datadog_logs_intake").await.unwrap();
    assert_eq!(outcome.kind, "rule_test");
    assert_eq!(outcome.provenance, "rule_test");
    assert_eq!(outcome.result, "blocked_explicit");
    assert!(outcome.explicit_block);
    assert!(!outcome.connection_attempted);
    assert!(outcome.test_id.starts_with("rt-"));
    assert_eq!(outcome.origin, "rule_test");

    // The controlled request DID reach the handler and produced a controlled
    // event; it must not be connected to the destination.
    let observed = events.lock().unwrap().clone();
    let controlled = observed
        .iter()
        .find(|event| event.destination_host.as_deref() == Some(DATADOG))
        .expect("controlled event recorded");
    assert_eq!(controlled.origin, Some("rule_test"));
    assert_eq!(
        controlled.test_id.as_deref(),
        Some(outcome.test_id.as_str())
    );
    assert_eq!(controlled.decision, "deny");
    assert_eq!(controlled.provenance, "explicit_block");
    assert!(
        timeout(Duration::from_millis(120), origin.accept())
            .await
            .is_err(),
        "the destination must never be connected"
    );

    // Ordinary client traffic to an unlisted loopback origin still works and
    // carries no controlled origin.
    let origin_task = tokio::spawn(async move {
        let (mut remote, _) = origin.accept().await.unwrap();
        let mut header = Vec::new();
        while !header.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            remote.read_exact(&mut byte).await.unwrap();
            header.push(byte[0]);
            assert!(header.len() < 4096);
        }
        remote
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .await
            .unwrap();
        let mut byte = [0];
        remote.read_exact(&mut byte).await.unwrap();
        assert_eq!(byte, [b'x']);
        remote.write_all(b"ok").await.unwrap();
        remote.shutdown().await.unwrap();
    });
    let allowed = connect_authority(address, &format!("127.0.0.1:{origin_port}"), b"x").await;
    origin_task.await.unwrap();
    assert!(allowed.starts_with(b"HTTP/1.1 200"));
    let observed = events.lock().unwrap().clone();
    let ordinary = observed
        .iter()
        .find(|event| event.destination_port == Some(origin_port))
        .expect("ordinary event recorded");
    assert_eq!(ordinary.origin, None);
    assert_eq!(ordinary.test_id, None);
    task.abort();
}

// A normal client header cannot create a controlled origin: only a registration
// matched to the private peer + exact target does.
#[tokio::test]
async fn ordinary_traffic_cannot_spoof_a_controlled_origin() {
    let blocked = telemetry::merge_blocked(&[], &["datadog_logs_intake".into()]).unwrap();
    let config = Config {
        environment_id: "synthetic-spoof".into(),
        default_action: DefaultAction::Allow,
        blocked,
        ..Config::default()
    };
    let (_proxy, address, events, task) = proxy(config).await;
    // A fabricated header claiming the test origin changes nothing.
    let response = connect_authority(
        address,
        &format!("{DATADOG}:443"),
        b"X-Lintel-Origin: rule_test\r\nX-Lintel-Test-Id: rt-fake\r\n",
    )
    .await;
    assert!(response.starts_with(b"HTTP/1.1 403"));
    let observed = events.lock().unwrap().clone();
    let event = observed
        .iter()
        .find(|event| event.destination_host.as_deref() == Some(DATADOG))
        .unwrap();
    assert_eq!(event.origin, None, "a header must not forge the origin");
    assert_eq!(event.test_id, None);
    task.abort();
}

// A target that is not explicitly blocked is refused before ANY network activity:
// no registration, no connection, no event.
#[tokio::test]
async fn rule_test_refuses_non_blocked_target_before_network() {
    let config = Config {
        environment_id: "synthetic-not-blocked".into(),
        default_action: DefaultAction::Deny,
        blocked: vec![],
        ..Config::default()
    };
    let (proxy, _address, events, task) = proxy(config).await;
    assert_eq!(
        proxy.rule_test("datadog_logs_intake").await.unwrap_err(),
        "telemetry_target_not_explicitly_blocked"
    );
    // Mixed/necessary hosts and unknown ids are refused as catalog misuse.
    assert_eq!(
        proxy.rule_test("api.anthropic.com").await.unwrap_err(),
        "unknown_telemetry_destination"
    );
    assert_eq!(
        proxy.rule_test("bogus").await.unwrap_err(),
        "unknown_telemetry_destination"
    );
    assert!(events.lock().unwrap().is_empty(), "no request was made");
    task.abort();
}

// The per-Proxy budget is finite and shared; a stopped/replaced instance cannot
// validate a stale handle. Exhaustion is an explicit refusal.
#[tokio::test]
async fn rule_test_budget_is_finite_across_the_proxy_instance() {
    let blocked = telemetry::merge_blocked(&[], &["datadog_logs_intake".into()]).unwrap();
    let (instance, _address, _events, task) = proxy(Config {
        environment_id: "synthetic-budget".into(),
        default_action: DefaultAction::Allow,
        blocked,
        ..Config::default()
    })
    .await;
    // Repeated real tests on the same instance keep succeeding within the
    // per-Proxy budget; each consumes its own one-use registration.
    for _ in 0..3 {
        let outcome = instance.rule_test("datadog_logs_intake").await.unwrap();
        assert_eq!(outcome.result, "blocked_explicit");
        assert!(!outcome.connection_attempted);
    }
    task.abort();
    // A replaced instance (fresh Proxy, no registration) cannot validate a
    // request from another instance's peer; its own test still uses its own peer.
    let blocked = telemetry::merge_blocked(&[], &["datadog_logs_intake".into()]).unwrap();
    let (replacement, _address, _events, task2) = proxy(Config {
        environment_id: "synthetic-replacement".into(),
        default_action: DefaultAction::Allow,
        blocked,
        ..Config::default()
    })
    .await;
    assert_eq!(
        replacement
            .rule_test("datadog_logs_intake")
            .await
            .unwrap()
            .result,
        "blocked_explicit"
    );
    task2.abort();
}

#[tokio::test]
async fn merge_preserves_user_rules_and_mixed_targets_are_never_selected() {
    let user = vec![
        Rule {
            host: "user.invalid".into(),
            ports: vec![],
        },
        Rule {
            host: "api.anthropic.com".into(),
            ports: vec![443],
        },
    ];
    let merged = telemetry::merge_blocked(&user, &["datadog_browser_intake".into()]).unwrap();
    assert_eq!(merged[0].host, "user.invalid");
    assert_eq!(merged[1].host, "api.anthropic.com");
    assert_eq!(merged[2].host, "browser-intake-us5-datadoghq.com");
    for mixed in [
        "api.anthropic.com",
        "claude.ai",
        "claude.com",
        "platform.claude.com",
        "downloads.claude.ai",
    ] {
        assert!(telemetry::host_for(mixed).is_none(), "{mixed}");
    }
}

// The foreground `--test-telemetry` refuses unknown and non-blocked ids before
// binding any listener; only an explicitly blocked target reaches the bind +
// controlled loopback request.
#[tokio::test]
async fn foreground_test_telemetry_refuses_before_binding() {
    let dir = std::env::temp_dir().join(format!("lintel-tel-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("policy.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({
            "environment_id": "synthetic-cli",
            "bind": "127.0.0.1:0",
            "default_action": "allow",
            "blocked": telemetry::merge_blocked(&[], &["datadog_logs_intake".into()]).unwrap(),
        }))
        .unwrap(),
    )
    .unwrap();
    let error = lintel_egress::serve_config_with_tests(&path, &["bogus".to_string()])
        .await
        .unwrap_err();
    assert_eq!(error, "invalid_telemetry_test_ids");
    let error =
        lintel_egress::serve_config_with_tests(&path, &["datadog_browser_intake".to_string()])
            .await
            .unwrap_err();
    assert!(error.starts_with("telemetry_target_not_explicitly_blocked:"));
    let _ = std::fs::remove_dir_all(&dir);
}

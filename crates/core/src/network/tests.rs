//! Synthetic evidence for the host network observe/probe/plan/execute/restore
//! path. Every case runs against an in-memory fixture and a substituted probe;
//! no real host interface, route, SystemConfiguration session or public
//! endpoint is touched. This module is `#[cfg(test)]` only and is never a
//! production fixture entry point.
use super::*;
use crate::{json, Engine, Value};
use std::sync::{Arc, Mutex};

/// A synthetic host network modeling one macOS service's IPv6 protocol. It can
/// simulate an external edit between preview and execute. `ServiceView` clones
/// never hold the lock across a nested call, so no test can self-deadlock.
#[derive(Clone)]
pub(crate) struct FakeHost {
    inner: Arc<Mutex<FakeState>>,
}

struct FakeState {
    platform: &'static str,
    /// The authoritative persisted IPv6 configuration dictionary.
    configuration: Value,
    enabled: bool,
    service_enabled: bool,
    /// When set, `apply_target` fails with this code.
    fail_apply: Option<String>,
    /// When true, `apply_target` succeeds but does not change state (models a
    /// write that does not read back as the frozen target).
    ignore_write: bool,
    /// Bumped by `external_edit`/mutation so observations can detect a race.
    revision_bump: u64,
    observe_calls: usize,
    fail_observe_after: Option<usize>,
}

impl FakeHost {
    pub(crate) fn macos(mode: &str) -> Self {
        FakeHost {
            inner: Arc::new(Mutex::new(FakeState {
                platform: "macos",
                configuration: manual_configuration(mode),
                enabled: mode != "off",
                service_enabled: true,
                fail_apply: None,
                ignore_write: false,
                revision_bump: 0,
                observe_calls: 0,
                fail_observe_after: None,
            })),
        }
    }
    pub(crate) fn linux() -> Self {
        let host = FakeHost::macos("automatic");
        host.inner.lock().unwrap().platform = "linux";
        host
    }
    pub(crate) fn external_edit(&self, mode: &str) {
        let mut state = self.inner.lock().unwrap();
        state.configuration = manual_configuration(mode);
        state.enabled = mode != "off";
        state.revision_bump = state.revision_bump.wrapping_add(1);
    }
    pub(crate) fn set_fail_apply(&self, code: &str) {
        self.inner.lock().unwrap().fail_apply = Some(code.to_string());
    }
    pub(crate) fn set_ignore_write(&self, ignore: bool) {
        self.inner.lock().unwrap().ignore_write = ignore;
    }
    fn view(&self) -> ServiceView {
        // Build the view without holding the lock across any other lock.
        let (configuration, enabled, service_enabled, bump) = {
            let state = self.inner.lock().unwrap();
            (
                state.configuration.clone(),
                state.enabled,
                state.service_enabled,
                state.revision_bump,
            )
        };
        let mode = if !enabled {
            "off"
        } else {
            classify(&configuration)
        };
        let mut view = ServiceView {
            service_id: "set-1:svc-uuid-1".into(),
            name: "Wi-Fi".into(),
            interface: "en0".into(),
            mode: mode.to_string(),
            service_enabled,
            ipv4_addresses: vec!["192.0.2.10".into()],
            ipv6_addresses: vec!["2001:db8::1".into()],
            ipv6_protocol: json!({
                "set_id": "set-1",
                "service_uuid": "svc-uuid-1",
                "enabled": enabled,
                "configuration": configuration,
            }),
        };
        // Fold the race counter into the observed addresses so a bump shows up
        // in the revision the same way a real route/address change would.
        if bump > 0 {
            view.ipv6_addresses.push(format!("2001:db8::{bump:x}"));
        }
        view
    }
}

fn manual_configuration(mode: &str) -> Value {
    match mode {
        "manual" => json!({
            "ConfigMethod": "Manual",
            "Addresses": ["2001:db8::1"],
            "PrefixLength": [64],
            "Router": "fe80::1",
            "LintelCustomKey": "preserve-me",
        }),
        "link_local" => json!({"ConfigMethod": "LinkLocal"}),
        "off" => json!({
            "ConfigMethod": "Manual",
            "Addresses": ["2001:db8::1"],
            "PrefixLength": [64],
            "Router": "fe80::1",
        }),
        _ => json!({"ConfigMethod": "Automatic"}),
    }
}
fn classify(configuration: &Value) -> &'static str {
    match configuration.get("ConfigMethod").and_then(Value::as_str) {
        Some("LinkLocal") => "link_local",
        Some("Manual") => "manual",
        Some("Automatic") => "automatic",
        _ => "unknown",
    }
}

impl HostNetwork for FakeHost {
    fn observe(&self) -> Result<Observation> {
        {
            let mut state = self.inner.lock().unwrap();
            state.observe_calls += 1;
            if state
                .fail_observe_after
                .is_some_and(|limit| state.observe_calls > limit)
            {
                return Err(err(
                    "network_interfaces_unavailable",
                    "synthetic observation failure",
                ));
            }
        }
        let services = vec![self.view()];
        let platform = self.inner.lock().unwrap().platform;
        let revision = revision_of(platform, &services);
        Ok(Observation {
            platform,
            services,
            interfaces: vec![json!({"interface":"en0", "flags":1, "up":true,
                "ipv4_addresses":["192.0.2.10"], "ipv6_addresses":["2001:db8::1"]})],
            revision,
            revision_complete: true,
            limitations: default_limitations(platform),
        })
    }

    fn apply_target(
        &self,
        service_id: &str,
        before: &ServiceView,
        after: &Value,
    ) -> Result<Applied> {
        // Compute the whole outcome under the lock, then release before any
        // further borrow (never call self.view() while holding it).
        let result = {
            let mut state = self.inner.lock().unwrap();
            if let Some(code) = state.fail_apply.clone() {
                return Err(err(&code, "synthetic apply failure"));
            }
            if service_id != "set-1:svc-uuid-1" {
                return Err(err("stale_network_plan", "synthetic unknown service"));
            }
            if before.ipv6_protocol["configuration"] != state.configuration
                || before.ipv6_protocol["enabled"].as_bool() != Some(state.enabled)
                || before.service_enabled != state.service_enabled
            {
                return Err(err("stale_network_plan", "synthetic external change"));
            }
            if !state.ignore_write {
                state.configuration = after["configuration"].clone();
                state.enabled = after["enabled"].as_bool().unwrap_or(false);
                state.revision_bump = state.revision_bump.wrapping_add(1);
            }
            (
                state.configuration.clone(),
                state.enabled,
                state.service_enabled,
                state.ignore_write,
            )
        };
        let (configuration, enabled, service_enabled, ignore_write) = result;
        let observed = ServiceView {
            service_id: "set-1:svc-uuid-1".into(),
            name: "Wi-Fi".into(),
            interface: "en0".into(),
            mode: if !enabled {
                "off".into()
            } else {
                classify(&configuration).into()
            },
            service_enabled,
            ipv4_addresses: vec!["192.0.2.10".into()],
            ipv6_addresses: vec!["2001:db8::1".into()],
            ipv6_protocol: json!({
                "set_id": "set-1",
                "service_uuid": "svc-uuid-1",
                "enabled": enabled,
                "configuration": configuration,
            }),
        };
        let verified = !ignore_write
            && observed.ipv6_protocol.get("configuration") == after.get("configuration")
            && observed.ipv6_protocol.get("enabled") == after.get("enabled");
        Ok(Applied {
            service: observed,
            configuration_verified: verified,
        })
    }
}

/// A substituted probe that fills real platform/revision/time and echoes the
/// spec, so baseline reuse and staleness behave as in production. Deterministic
/// and offline.
fn fake_probe(spec: &ProbeSpec, platform: &str, revision: &str) -> Result<Value> {
    let channel = spec.proxy_url.is_some();
    let cell = |path: &str, family: &str, ok: bool| {
        json!({
            "path": path,
            "family": family,
            "status": if ok { "ok" } else { "not_tested" },
            "public_ip": if ok { json!(if family == "ipv4" { "203.0.113.7" } else { "2001:db8::7" }) } else { Value::Null },
            "elapsed_ms": 5,
            "peer_family": if ok { json!(family) } else { Value::Null },
            "message": if ok { Value::Null } else { json!("未提供 loopback 通道；此路径未测试") },
        })
    };
    Ok(json!({
        "schema": "lintel.network-probe/1",
        "id": uuid::Uuid::new_v4().to_string(),
        "executed_at": crate::now(),
        "platform": platform,
        "execution_host": "synthetic-host",
        "network_revision": revision,
        "endpoints": {"ipv4": spec.ipv4_url, "ipv6": spec.ipv6_url},
        "proxy_url": spec.proxy_url,
        "proxy_binding": spec.proxy_binding,
        "timeout_seconds": spec.timeout_seconds,
        "cells": [
            cell("host_default", "ipv4", true),
            cell("host_default", "ipv6", true),
            cell("lintel_channel", "ipv4", channel),
            cell("lintel_channel", "ipv6", channel),
        ],
    }))
}

fn fixture(host: FakeHost) -> (tempfile::TempDir, Engine) {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let state = temp.path().join("state");
    let mut engine = Engine::new(home, state).unwrap();
    engine.network_fixture = Some(Box::new(host));
    engine.network_probe_hook = Some(fake_probe);
    (temp, engine)
}

fn probe_spec(proxy: Option<(&str, &str)>) -> Value {
    match proxy {
        Some((url, binding)) => json!({
            "ipv4_url": "https://api.ipify.org",
            "ipv6_url": "https://api6.ipify.org",
            "proxy_url": url,
            "proxy_binding": binding,
            "timeout_seconds": 10,
        }),
        None => json!({
            "ipv4_url": "https://api.ipify.org",
            "ipv6_url": "https://api6.ipify.org",
            "timeout_seconds": 10,
        }),
    }
}

fn data(response: Value) -> Value {
    assert_eq!(response["ok"], true, "{response}");
    response["data"].clone()
}

fn error_code(response: Value) -> String {
    assert_eq!(response["ok"], false, "{response}");
    response["error"]["code"].as_str().unwrap_or("").to_string()
}

#[test]
fn inspect_reports_services_observed_addresses_and_stable_revision() {
    let (_temp, engine) = fixture(FakeHost::macos("manual"));
    let first = data(engine.request(json!({"command":"network_inspect"})));
    assert_eq!(first["schema"], "lintel.network/1");
    assert_eq!(first["platform"], "macos");
    assert_eq!(first["services"][0]["service_id"], "set-1:svc-uuid-1");
    assert_eq!(first["services"][0]["mode"], "manual");
    assert_eq!(first["services"][0]["enabled"], true);
    // Observed addresses, not the static manual address list.
    assert_eq!(first["services"][0]["ipv6_addresses"][0], "2001:db8::1");
    assert!(first["limitations"].as_array().unwrap().len() >= 3);
    let second = data(engine.request(json!({"command":"network_inspect"})));
    assert_eq!(first["network_revision"], second["network_revision"]);
}

#[test]
fn disabled_protocol_reports_off_mode() {
    let (_temp, engine) = fixture(FakeHost::macos("off"));
    let inspect = data(engine.request(json!({"command":"network_inspect"})));
    assert_eq!(inspect["services"][0]["mode"], "off");
}

#[test]
fn probe_shapes_four_cells_and_marks_channel_not_tested() {
    let (_temp, engine) = fixture(FakeHost::macos("manual"));
    let probe = data(engine.request(json!({
        "command":"network_probe",
        "ipv4_url":"https://api.ipify.org",
        "ipv6_url":"https://api6.ipify.org",
    })));
    assert_eq!(probe["schema"], "lintel.network-probe/1");
    assert_eq!(probe["execution_host"], "synthetic-host");
    assert_eq!(probe["cells"].as_array().unwrap().len(), 4);
    assert_eq!(probe["cells"][0]["status"], "ok");
    assert_eq!(probe["cells"][2]["status"], "not_tested");
    assert_eq!(probe["proxy_binding"], Value::Null);
    assert_eq!(probe["timeout_seconds"], 10);
}

#[test]
fn probe_rejects_unsafe_endpoints_and_mismatched_binding() {
    let (_temp, engine) = fixture(FakeHost::macos("manual"));
    // Credentials in the URL.
    let bad = engine.request(json!({
        "command":"network_probe",
        "ipv4_url":"https://user:pass@api.ipify.org",
        "ipv6_url":"https://api6.ipify.org",
    }));
    assert_eq!(error_code(bad), "invalid_probe");
    // An instance binding without a channel is refused.
    let unbound = engine.request(json!({
        "command":"network_probe",
        "ipv4_url":"https://api.ipify.org",
        "ipv6_url":"https://api6.ipify.org",
        "proxy_binding":"chan-1",
    }));
    assert_eq!(error_code(unbound), "invalid_proxy_binding");
    // A non-loopback proxy is refused.
    let remote = engine.request(json!({
        "command":"network_probe",
        "ipv4_url":"https://api.ipify.org",
        "ipv6_url":"https://api6.ipify.org",
        "proxy_url":"http://203.0.113.9:8080",
        "proxy_binding":"b1",
    }));
    assert_eq!(error_code(remote), "invalid_proxy");
    // Unknown key.
    let unknown = engine.request(json!({
        "command":"network_probe",
        "ipv4_url":"https://api.ipify.org",
        "ipv6_url":"https://api6.ipify.org",
        "sneaky": true,
    }));
    assert_eq!(error_code(unknown), "invalid_probe");
}

#[test]
fn ipv6_plan_freezes_identity_full_config_and_before_probe() {
    let (_temp, engine) = fixture(FakeHost::macos("manual"));
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"off",
        "probe": probe_spec(None),
    })));
    assert_eq!(plan["kind"], "network_ipv6");
    assert_eq!(plan["network"]["scope"], "host_shared");
    assert_eq!(plan["network"]["service_id"], "set-1:svc-uuid-1");
    assert_eq!(plan["network"]["interface"], "en0");
    // `off` keeps the full original dictionary and only flips enabled.
    assert_eq!(plan["network"]["after"]["enabled"], false);
    assert_eq!(
        plan["network"]["after"]["configuration"]["ConfigMethod"],
        "Manual"
    );
    assert_eq!(
        plan["network"]["after"]["configuration"]["LintelCustomKey"],
        "preserve-me"
    );
    assert_eq!(
        plan["network"]["before_probe"]["cells"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    assert!(plan.get("extra").is_none());
}

#[test]
fn link_local_drops_only_manual_address_keys() {
    let (_temp, engine) = fixture(FakeHost::macos("manual"));
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"link_local",
        "probe": probe_spec(None),
    })));
    let configuration = &plan["network"]["after"]["configuration"];
    assert_eq!(configuration["ConfigMethod"], "LinkLocal");
    assert!(configuration.get("Addresses").is_none());
    assert!(configuration.get("PrefixLength").is_none());
    assert!(configuration.get("Router").is_none());
    // Unrelated keys survive.
    assert_eq!(configuration["LintelCustomKey"], "preserve-me");
}

#[test]
fn execute_writes_reads_back_and_reprobes() {
    let (_temp, engine) = fixture(FakeHost::macos("manual"));
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"link_local",
        "probe": probe_spec(None),
    })));
    let receipt = data(engine.request(json!({
        "command":"execute","plan_id":plan["id"],"approval":plan["hash"],
    })));
    assert_eq!(receipt["status"], "completed");
    assert_eq!(receipt["network_change"]["configuration_verified"], true);
    assert_eq!(receipt["network_change"]["scope"], "host_shared");
    assert_eq!(receipt["restorable"], true);
    assert_eq!(receipt["after_probe"]["cells"][0]["status"], "ok");
    assert!(receipt["task_result"]["coverage"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["scope"] == "host_shared_network" && c["state"] == "done"));
    // A repeat execute by the same id must query the original job, not rewrite.
    let repeat = data(engine.request(json!({
        "command":"execute","plan_id":plan["id"],"approval":plan["hash"],
    })));
    assert_eq!(repeat["id"], receipt["id"]);
    assert_eq!(repeat["status"], "completed");
}

#[test]
fn manual_roundtrip_preserves_custom_key_and_prefix() {
    // Start Automatic, change to LinkLocal, then restore the original Manual
    // dictionary (with PrefixLength/Router/custom key) exactly.
    let host = FakeHost::macos("manual");
    let (_temp, engine) = fixture(host.clone());
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"link_local",
        "probe": probe_spec(None),
    })));
    let receipt = data(engine.request(json!({
        "command":"execute","plan_id":plan["id"],"approval":plan["hash"],
    })));
    assert_eq!(receipt["restorable"], true);
    let restore = data(engine.request(json!({
        "command":"plan_restore","job_id":receipt["id"],
    })));
    let after = &restore["network"]["after"]["configuration"];
    assert_eq!(after["ConfigMethod"], "Manual");
    assert_eq!(after["Addresses"][0], "2001:db8::1");
    assert_eq!(after["PrefixLength"][0], 64);
    assert_eq!(after["Router"], "fe80::1");
    assert_eq!(after["LintelCustomKey"], "preserve-me");
    let restored = data(engine.request(json!({
        "command":"execute","plan_id":restore["id"],"approval":restore["hash"],
    })));
    assert_eq!(restored["network_change"]["configuration_verified"], true);
}

#[test]
fn disabled_initial_then_link_local_then_restore_off() {
    let host = FakeHost::macos("off");
    let (_temp, engine) = fixture(host.clone());
    // Initial observation: off.
    let inspect = data(engine.request(json!({"command":"network_inspect"})));
    assert_eq!(inspect["services"][0]["mode"], "off");
    // Turn link_local on.
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"link_local",
        "probe": probe_spec(None),
    })));
    let receipt = data(engine.request(json!({
        "command":"execute","plan_id":plan["id"],"approval":plan["hash"],
    })));
    assert_eq!(receipt["restorable"], true);
    // Restore back to off (original enabled=false, full dictionary preserved).
    let restore = data(engine.request(json!({
        "command":"plan_restore","job_id":receipt["id"],
    })));
    assert_eq!(restore["network"]["after"]["enabled"], false);
    let restored = data(engine.request(json!({
        "command":"execute","plan_id":restore["id"],"approval":restore["hash"],
    })));
    assert_eq!(restored["network_change"]["configuration_verified"], true);
    let final_state = data(engine.request(json!({"command":"network_inspect"})));
    assert_eq!(final_state["services"][0]["mode"], "off");
}

#[test]
fn restore_refuses_external_edit_and_stale_identity() {
    let host = FakeHost::macos("manual");
    let (_temp, engine) = fixture(host.clone());
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"off",
        "probe": probe_spec(None),
    })));
    let receipt = data(engine.request(json!({
        "command":"execute","plan_id":plan["id"],"approval":plan["hash"],
    })));
    host.external_edit("link_local");
    let blocked = engine.request(json!({
        "command":"plan_restore","job_id":receipt["id"],
    }));
    assert_eq!(error_code(blocked), "restore_conflict");
}

#[test]
fn stale_before_configuration_blocks_execute_without_a_job() {
    let host = FakeHost::macos("manual");
    let (_temp, engine) = fixture(host.clone());
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"off",
        "probe": probe_spec(None),
    })));
    // External edit between preview and execute.
    host.external_edit("link_local");
    let blocked = engine.request(json!({
        "command":"execute","plan_id":plan["id"],"approval":plan["hash"],
    }));
    assert_eq!(error_code(blocked), "stale_network_plan");
    // No job was created for the stale plan.
    let lookup = engine.request(json!({"command":"job","plan_id":plan["id"]}));
    assert_eq!(error_code(lookup), "job_not_found");
}

#[test]
fn readback_mismatch_is_not_restorable_and_needs_reconciliation() {
    let host = FakeHost::macos("manual");
    let (_temp, engine) = fixture(host.clone());
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"off",
        "probe": probe_spec(None),
    })));
    host.set_ignore_write(true);
    let receipt = data(engine.request(json!({
        "command":"execute","plan_id":plan["id"],"approval":plan["hash"],
    })));
    assert_eq!(receipt["status"], "needs_reconciliation");
    assert_eq!(receipt["restorable"], false);
    assert_eq!(receipt["network_change"]["configuration_verified"], false);
    // Reconciliation on query must not claim completion.
    let queried = data(engine.request(json!({"command":"job","job_id":plan["id"]})));
    assert_eq!(queried["restorable"], false);
}

#[test]
fn denied_authorization_is_a_definite_failure_with_no_mutation() {
    let host = FakeHost::macos("manual");
    let (_temp, engine) = fixture(host.clone());
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"off",
        "probe": probe_spec(None),
    })));
    host.set_fail_apply("network_authorization_denied");
    let receipt = data(engine.request(json!({
        "command":"execute","plan_id":plan["id"],"approval":plan["hash"],
    })));
    assert_eq!(receipt["status"], "failed");
    assert_eq!(receipt["restorable"], false);
    // The configuration is untouched: still Manual.
    let inspect = data(engine.request(json!({"command":"network_inspect"})));
    assert_eq!(inspect["services"][0]["mode"], "manual");
}

#[test]
fn interrupted_verified_write_stays_restorable_on_query() {
    // A verified write stays restorable; interrupted reprobe stays unverified.
    // Query persists that reconciliation and never repeats either operation.
    let host = FakeHost::macos("manual");
    let (_temp, engine) = fixture(host.clone());
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"off",
        "probe": probe_spec(None),
    })));
    // Simulate a crash after the write was verified but before completion:
    // execute once normally, then rewrite the persisted job's status back to
    // "verifying" with configuration_verified=true.
    data(engine.request(json!({
        "command":"execute","plan_id":plan["id"],"approval":plan["hash"],
    })));
    let job_path = engine
        .state
        .join("jobs")
        .join(format!("{}.json", plan["id"].as_str().unwrap()));
    let mut job: Value = serde_json::from_slice(&std::fs::read(&job_path).unwrap()).unwrap();
    job["status"] = json!("verifying");
    job["after_probe"] = Value::Null;
    set_step(&mut job, "network_reprobe", "pending", "not started");
    std::fs::write(&job_path, serde_json::to_vec(&job).unwrap()).unwrap();
    let queried = data(engine.request(json!({"command":"job","job_id":plan["id"]})));
    assert_eq!(queried["status"], "partially_completed");
    assert_eq!(queried["restorable"], true);
    assert!(queried["after_probe"].is_null());
    let persisted: Value = serde_json::from_slice(&std::fs::read(&job_path).unwrap()).unwrap();
    assert_eq!(persisted["status"], queried["status"]);
    assert_eq!(
        data(engine.request(json!({"command":"job", "job_id":plan["id"]})))["status"],
        "partially_completed"
    );
    assert_eq!(host.view().mode, "off");
}

#[test]
fn baseline_reuse_requires_same_binding_revision_and_target() {
    let (_temp, engine) = fixture(FakeHost::macos("manual"));
    let probe = data(engine.request(json!({
        "command":"network_probe",
        "ipv4_url":"https://api.ipify.org",
        "ipv6_url":"https://api6.ipify.org",
        "proxy_url":"http://127.0.0.1:55123",
        "proxy_binding":"chan-1",
    })));
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"off",
        "probe": probe_spec(Some(("http://127.0.0.1:55123", "chan-1"))),
        "baseline_id": probe["id"],
    })));
    assert_eq!(plan["network"]["before_probe"]["id"], probe["id"]);
    // A different binding on the same port forces a fresh pre-probe.
    let plan2 = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"off",
        "probe": probe_spec(Some(("http://127.0.0.1:55123", "chan-2"))),
        "baseline_id": probe["id"],
    })));
    assert_ne!(plan2["network"]["before_probe"]["id"], probe["id"]);
}

#[test]
fn expired_or_stale_baseline_is_not_reused() {
    let (_temp, engine) = fixture(FakeHost::macos("manual"));
    let probe = data(engine.request(json!({
        "command":"network_probe",
        "ipv4_url":"https://api.ipify.org",
        "ipv6_url":"https://api6.ipify.org",
    })));
    // Rewrite the stored baseline to be 10 minutes old.
    let path = engine
        .state
        .join("network-probes")
        .join(format!("{}.json", probe["id"].as_str().unwrap()));
    let mut stored: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    stored["executed_at"] =
        json!((chrono::Utc::now() - chrono::Duration::minutes(10)).to_rfc3339());
    std::fs::write(&path, serde_json::to_vec(&stored).unwrap()).unwrap();
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"off",
        "probe": probe_spec(None),
        "baseline_id": probe["id"],
    })));
    assert_ne!(plan["network"]["before_probe"]["id"], probe["id"]);
}

#[test]
fn post_probe_failure_keeps_verified_configuration() {
    // The configuration write succeeds and reads back; a failing post-probe must
    // not roll it back or mark it failed.
    let host = FakeHost::macos("manual");
    let (_temp, mut engine) = fixture(host.clone());
    engine.network_probe_hook = Some(fail_after_write);
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"off",
        "probe": probe_spec(None),
    })));
    let receipt = data(engine.request(json!({
        "command":"execute","plan_id":plan["id"],"approval":plan["hash"],
    })));
    assert_eq!(receipt["status"], "partially_completed");
    assert_eq!(receipt["task_result"]["outcome"], "partial");
    assert_eq!(receipt["network_change"]["configuration_verified"], true);
    assert_eq!(receipt["restorable"], true);
    assert_eq!(receipt["after_probe"], Value::Null);
    let reprobe = receipt["task_result"]["coverage"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["scope"] == "post_change_probe")
        .cloned()
        .unwrap();
    assert_eq!(reprobe["state"], "not_checked");
}

fn fail_after_write(spec: &ProbeSpec, platform: &str, revision: &str) -> Result<Value> {
    // The plan-time before-probe must succeed (so the plan freezes); only the
    // execute-time after-probe fails. Distinguish by a marker in the spec.
    if spec.proxy_binding.is_none() && spec.timeout_seconds == 10 {
        // Both calls look alike here; fail the second by counting invocations.
        static CALLS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if n >= 1 {
            return Err(err("network_probe_failed", "synthetic post-probe failure"));
        }
    }
    fake_probe(spec, platform, revision)
}

#[test]
fn execute_rejects_stale_approval_hash_without_a_job() {
    let (_temp, engine) = fixture(FakeHost::macos("manual"));
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"off",
        "probe": probe_spec(None),
    })));
    let wrong = engine.request(json!({
        "command":"execute","plan_id":plan["id"],"approval":"0".repeat(64),
    }));
    assert_eq!(error_code(wrong), "approval_mismatch");
}

#[test]
fn linux_observation_is_readonly_and_mutation_unsupported() {
    let (_temp, engine) = fixture(FakeHost::linux());
    let inspect = data(engine.request(json!({"command":"network_inspect"})));
    assert_eq!(inspect["platform"], "linux");
    let unsupported = engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"en0",
        "mode":"off",
        "probe": probe_spec(None),
    }));
    assert_eq!(error_code(unsupported), "network_mutation_unsupported");
}

#[test]
fn unit_test_engine_without_fixture_refuses_host_network() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let engine = Engine::new(home, temp.path().join("state")).unwrap();
    assert_eq!(
        error_code(engine.request(json!({"command":"network_inspect"}))),
        "network_test_fixture_required"
    );
}

#[test]
fn unknown_and_launch_kinds_are_not_admitted_by_generic_execute() {
    let (_temp, engine) = fixture(FakeHost::macos("manual"));
    // An unknown plan kind must be rejected as invalid_plan_kind.
    let unknown_plan = json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "environment_id": Value::Null,
        "kind": "network_teleport",
        "title": "x",
        "changes": [],
        "preserves": [],
        "warnings": [],
        "actions": [],
        "created_at": crate::now(),
        "status": "planned",
        "rule_version": crate::policy::RULE,
        "extra": {},
    });
    let mut value = unknown_plan.clone();
    value["hash"] = json!(crate::storage::digest(
        &serde_json::to_vec(&unknown_plan).unwrap()
    ));
    let pid = value["id"].as_str().unwrap().to_string();
    std::fs::write(
        engine.state.join("plans").join(format!("{pid}.json")),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    let response = engine.request(json!({
        "command":"execute","plan_id":pid,"approval":value["hash"].clone(),
    }));
    assert_eq!(error_code(response), "invalid_plan_kind");
}

#[test]
fn plan_show_preserves_the_network_projection() {
    let (_temp, engine) = fixture(FakeHost::macos("manual"));
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"off",
        "probe": probe_spec(None),
    })));
    let shown = data(engine.request(json!({"command":"plan_show","plan_id":plan["id"]})));
    assert_eq!(shown["network"]["service_id"], "set-1:svc-uuid-1");
    assert!(shown.get("extra").is_none());
}

#[test]
fn channel_bound_plan_freezes_binding_and_cli_execute_needs_no_binding() {
    // A plan previewed with an App channel freezes the instance binding, visible
    // in the public projection. A CLI/runners finite execute provides no
    // `proxy_binding` field, and core must not require one.
    let (_temp, engine) = fixture(FakeHost::macos("manual"));
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"off",
        "probe": probe_spec(Some(("http://127.0.0.1:55123", "chan-A"))),
    })));
    assert_eq!(plan["network"]["probe"]["proxy_binding"], "chan-A");
    assert_eq!(
        plan["network"]["probe"]["proxy_url"],
        "http://127.0.0.1:55123"
    );
    let receipt = data(engine.request(json!({
        "command":"execute","plan_id":plan["id"],"approval":plan["hash"],
    })));
    assert_eq!(receipt["status"], "completed");
}

#[test]
fn restore_with_explicit_new_probe_ignores_stopped_channel() {
    // The original plan used a channel; the channel has stopped. The restore
    // preview takes an explicit fresh probe with no proxy.
    let (_temp, engine) = fixture(FakeHost::macos("manual"));
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"link_local",
        "probe": probe_spec(Some(("http://127.0.0.1:55123", "chan-A"))),
    })));
    let receipt = data(engine.request(json!({
        "command":"execute","plan_id":plan["id"],"approval":plan["hash"],
    })));
    assert_eq!(receipt["restorable"], true);
    // A new explicit probe without any channel is accepted.
    let restore = data(engine.request(json!({
        "command":"plan_restore","job_id":receipt["id"],
        "probe": probe_spec(None),
    })));
    assert_eq!(restore["kind"], "network_restore");
    assert_eq!(restore["network"]["probe"]["proxy_url"], Value::Null);
    assert_eq!(restore["network"]["probe"]["proxy_binding"], Value::Null);
    assert_eq!(
        restore["network"]["before_probe"]["cells"][2]["status"],
        "not_tested"
    );
}

#[test]
fn restore_without_explicit_probe_reuses_original_target() {
    let (_temp, engine) = fixture(FakeHost::macos("manual"));
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"off",
        "probe": probe_spec(None),
    })));
    let receipt = data(engine.request(json!({
        "command":"execute","plan_id":plan["id"],"approval":plan["hash"],
    })));
    let restore = data(engine.request(json!({
        "command":"plan_restore","job_id":receipt["id"],
    })));
    assert_eq!(
        restore["network"]["probe"]["ipv4_url"],
        "https://api.ipify.org"
    );
    assert_eq!(restore["network"]["after"]["enabled"], true);
}

#[test]
fn plan_restore_needs_original_verified_job() {
    let (_temp, engine) = fixture(FakeHost::macos("manual"));
    // A non-existent job id.
    let missing = engine.request(json!({
        "command":"plan_restore","job_id": uuid::Uuid::new_v4().to_string(),
    }));
    assert_eq!(error_code(missing), "job_not_found");
}

#[test]
fn generic_plan_restore_routes_network_job_to_network_restore() {
    // The Records entry point (plan_restore) must route a network job to the
    // finite network restore path rather than settings restore.
    let (_temp, engine) = fixture(FakeHost::macos("manual"));
    let plan = data(engine.request(json!({
        "command":"plan_network_ipv6",
        "service_id":"set-1:svc-uuid-1",
        "mode":"off",
        "probe": probe_spec(None),
    })));
    let receipt = data(engine.request(json!({
        "command":"execute","plan_id":plan["id"],"approval":plan["hash"],
    })));
    let restore = data(engine.request(json!({
        "command":"plan_restore","job_id":receipt["id"],
    })));
    assert_eq!(restore["kind"], "network_restore");
}

#[test]
fn explicit_cli_loopback_probe_does_not_require_app_instance_binding() {
    let (_temp, engine) = fixture(FakeHost::macos("automatic"));
    let probe = data(engine.request(json!({"command":"network_probe",
        "ipv4_url":"https://echo4.invalid", "ipv6_url":"https://echo6.invalid",
        "proxy_url":"http://127.0.0.1:55123"})));
    assert_eq!(probe["cells"][2]["status"], "ok");
    assert!(probe["proxy_binding"].is_null());
}

#[test]
fn disabled_service_is_frozen_and_never_enabled_by_ipv6_write() {
    let host = FakeHost::macos("manual");
    host.inner.lock().unwrap().service_enabled = false;
    let (_temp, engine) = fixture(host.clone());
    let plan = data(engine.request(json!({"command":"plan_network_ipv6", "service_id":"set-1:svc-uuid-1", "mode":"off", "probe":probe_spec(None)})));
    assert_eq!(plan["network"]["service_enabled"], false);
    let receipt = data(
        engine.request(json!({"command":"execute", "plan_id":plan["id"], "approval":plan["hash"]})),
    );
    assert_eq!(receipt["network_change"]["configuration_verified"], true);
    assert!(!host.view().service_enabled);
    let restore =
        data(engine.request(json!({"command":"plan_network_restore", "job_id":receipt["id"]})));
    data(engine.request(
        json!({"command":"execute", "plan_id":restore["id"], "approval":restore["hash"]}),
    ));
    assert!(!host.view().service_enabled);
    assert_eq!(
        host.view().ipv6_protocol["configuration"],
        manual_configuration("manual")
    );
}

#[test]
fn service_enablement_change_invalidates_preview_before_new_job() {
    let host = FakeHost::macos("manual");
    let (_temp, engine) = fixture(host.clone());
    let plan = data(engine.request(json!({"command":"plan_network_ipv6", "service_id":"set-1:svc-uuid-1", "mode":"off", "probe":probe_spec(None)})));
    host.inner.lock().unwrap().service_enabled = false;
    assert_eq!(
        error_code(
            engine.request(
                json!({"command":"execute", "plan_id":plan["id"], "approval":plan["hash"]})
            )
        ),
        "stale_network_plan"
    );
    assert!(!engine.path("jobs", plan["id"].as_str().unwrap()).exists());
    assert_eq!(host.view().mode, "manual");
}

#[test]
fn failed_post_probe_observation_marks_result_stale_and_prevents_reuse() {
    let host = FakeHost::macos("automatic");
    host.inner.lock().unwrap().fail_observe_after = Some(1);
    let (_temp, engine) = fixture(host.clone());
    let probe = data(engine.request(json!({"command":"network_probe", "ipv4_url":"https://api.ipify.org", "ipv6_url":"https://api6.ipify.org"})));
    assert_eq!(probe["stale"], true);
    assert_eq!(probe["observation_error"], "network_interfaces_unavailable");
    assert!(engine
        .reusable_baseline(
            probe["id"].as_str().unwrap(),
            &ProbeSpec::from_value(&probe_spec(None)).unwrap(),
            probe["network_revision"].as_str().unwrap()
        )
        .is_none());
}

#[test]
fn verified_write_does_not_borrow_old_revision_when_metadata_read_fails() {
    let host = FakeHost::macos("manual");
    let (_temp, engine) = fixture(host.clone());
    let plan = data(engine.request(json!({"command":"plan_network_ipv6", "service_id":"set-1:svc-uuid-1", "mode":"off", "probe":probe_spec(None)})));
    host.inner.lock().unwrap().fail_observe_after = Some(3);
    let receipt = data(
        engine.request(json!({"command":"execute", "plan_id":plan["id"], "approval":plan["hash"]})),
    );
    assert_eq!(receipt["network_change"]["configuration_verified"], true);
    assert_eq!(receipt["restorable"], true);
    assert_eq!(receipt["after_probe"]["stale"], true);
    assert_eq!(receipt["after_probe"]["network_revision"], "");
    assert_eq!(
        receipt["after_probe"]["observation_error"],
        "network_interfaces_unavailable"
    );
}

#[test]
fn revisions_include_configuration_flags_new_addressless_tun_and_route_values() {
    let services = vec![FakeHost::macos("automatic").view()];
    let interfaces = json!({"en0":{"flags":1, "ipv4":["192.0.2.1"], "ipv6":[]}});
    let routes = json!({"vpn":{"Router":"192.0.2.2"}});
    let original = observed_revision("macos", &services, &interfaces, &routes);
    let mut changed = services.clone();
    changed[0].ipv6_protocol["enabled"] = json!(false);
    assert_ne!(
        original,
        observed_revision("macos", &changed, &interfaces, &routes)
    );
    let mut flags = interfaces.clone();
    flags["en0"]["flags"] = json!(0);
    assert_ne!(
        original,
        observed_revision("macos", &services, &flags, &routes)
    );
    let mut tun = interfaces.clone();
    tun["utun4"] = json!({"flags":1, "ipv4":[], "ipv6":[]});
    assert_ne!(
        original,
        observed_revision("macos", &services, &tun, &routes)
    );
    assert_ne!(
        original,
        observed_revision(
            "macos",
            &services,
            &interfaces,
            &json!({"vpn":{"Router":"192.0.2.3"}})
        )
    );
}

#[test]
fn route_parser_keeps_first_ipv6_route_and_rejects_incomplete_budget() {
    assert_eq!(
        normalize_routes(b"first  IPv6 route\nsecond route\n", false).unwrap(),
        vec!["first IPv6 route", "second route"]
    );
    assert_eq!(
        normalize_routes(b"Iface Destination\neth0  00000000\n", true).unwrap(),
        vec!["eth0 00000000"]
    );
    assert!(normalize_routes(&vec![b'x'; 1024 * 1024 + 1], false).is_err());
    assert!(normalize_routes("route\n".repeat(4097).as_bytes(), false).is_err());
}

#[test]
fn route_change_after_approval_requires_new_preview_without_a_job() {
    let host = FakeHost::macos("manual");
    let (_temp, engine) = fixture(host.clone());
    let plan = data(engine.request(json!({"command":"plan_network_ipv6", "service_id":"set-1:svc-uuid-1", "mode":"off", "probe":probe_spec(None)})));
    // Same static configuration; only observed network facts change.
    host.inner.lock().unwrap().revision_bump += 1;
    assert_eq!(
        error_code(
            engine.request(
                json!({"command":"execute", "plan_id":plan["id"], "approval":plan["hash"]})
            )
        ),
        "stale_network_plan"
    );
    assert!(!engine.path("jobs", plan["id"].as_str().unwrap()).exists());
    assert_eq!(host.view().mode, "manual");
}

//! Static, inspectable Claude Code telemetry-destination catalog and the
//! provenance-controlled rule-test handle used to prove an explicit block.
//!
//! This module reads one embedded contract file (`contracts/telemetry-destinations.json`)
//! and never opens a home, state directory, executable, or host network. Unknown
//! catalog IDs, unknown fields and wildcard hosts are rejected. A "blockable"
//! entry is eligible for optional selection into an existing blocked-rule draft;
//! an "informational" entry is a mixed/necessary host that must never be offered
//! as a one-click telemetry block.

use crate::{Destination, Event, Rule};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// The single maintained catalog source, shared by the crate, the desktop egress
/// module and every CLI (root and runner). Metadata only.
const CATALOG_JSON: &str = include_str!("../../../contracts/telemetry-destinations.json");

const MAX_ID_LEN: usize = 64;
const MAX_TEST_LIFETIME: Duration = Duration::from_secs(30);
const MAX_TEST_TOTAL: u64 = 16;
const MAX_TEST_CONCURRENT: usize = 2;
static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(1);

fn static_catalog() -> &'static serde_json::Value {
    static CATALOG: OnceLock<serde_json::Value> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(CATALOG_JSON).expect("valid embedded telemetry catalog")
    })
}

pub fn catalog() -> serde_json::Value {
    static_catalog().clone()
}

/// The canonical exact host for a catalog id, or None for an unknown id. The
/// match is against the static catalog only; it never resolves a host.
pub fn host_for(id: &str) -> Option<&'static str> {
    if id.is_empty() || id.len() > MAX_ID_LEN {
        return None;
    }
    static_catalog()["destinations"]
        .as_array()?
        .iter()
        .find(|entry| entry["id"] == id && entry["blockable"] == true && entry["port"] == 443)?
        ["host"]
        .as_str()
}

/// Accept one exact catalog id and build its canonical rule without duplicating
/// a rule for the same host/port. Unknown ids and a rule that already exists are
/// explicit failures, so a caller never silently changes the intended selection.
pub fn admitted_rule(existing: &[Rule], id: &str) -> Result<Rule, &'static str> {
    let host = host_for(id).ok_or("unknown_telemetry_destination")?;
    if existing
        .iter()
        .any(|r| r.host == host && (r.ports.is_empty() || r.ports.contains(&443)))
    {
        return Err("telemetry_destination_already_blocked");
    }
    Ok(Rule {
        host: host.to_string(),
        ports: vec![443],
    })
}

/// Build the egress `blocked` rule list for a finite set of catalog ids. The
/// user's existing blocked rules are preserved verbatim and every other Config
/// field stays the caller's. Unknown ids reject the whole selection.
pub fn merge_blocked(existing: &[Rule], ids: &[String]) -> Result<Vec<Rule>, &'static str> {
    // Reject the whole selection before mutating so a partial set never applies.
    let mut accepted: Vec<Rule> = existing.to_vec();
    for id in ids {
        accepted.push(admitted_rule(&accepted, id)?);
    }
    Ok(accepted)
}

/// One controlled rule test's verified outcome. Only the owning Proxy produces
/// this after a real loopback CONNECT through its own serving instance.
#[derive(Clone, Debug, Serialize)]
pub struct RuleTestOutcome {
    pub kind: &'static str,
    pub timestamp_unix_ms: u128,
    pub environment_id: String,
    pub destination_host: String,
    pub destination_port: u16,
    pub provenance: &'static str,
    /// `blocked_explicit` only when a real loopback request was refused by an
    /// explicit block. `failed` covers a broken proxy or an unexpected allow
    /// (which is itself refused closed) or a non-blocked target.
    pub result: &'static str,
    pub decision: &'static str,
    pub reason: &'static str,
    pub explicit_block: bool,
    /// The controlled request DID traverse the proxy's real parser+decision path
    /// but never attempted a destination connection.
    pub connection_attempted: bool,
    /// Owner-origin marker; present only for controlled tests.
    pub test_id: String,
    pub origin: &'static str,
    pub coverage: &'static str,
}

/// One in-flight controlled rule test, registered before the loopback connect
/// and consumed by the accepting handler. Keyed by the private client peer
/// address so another client cannot own the origin, and by the expected exact
/// host/port so a spoofed target cannot borrow a registration.
pub(crate) struct TestCompletion {
    pub test_id: String,
    pub sender: tokio::sync::oneshot::Sender<Event>,
}

#[derive(Debug)]
struct TestSlot {
    test_id: String,
    host: String,
    port: u16,
    completion: Option<tokio::sync::oneshot::Sender<Event>>,
    expires_at: Instant,
}

/// Removing only this lease's slot on Drop also handles cancellation of the
/// caller's future. A consumed slot still counts until its owner has the result.
#[derive(Debug)]
pub(crate) struct TestLease {
    registry: Arc<RuleTestRegistry>,
    peer: std::net::SocketAddr,
    pub test_id: String,
    pub completion: tokio::sync::oneshot::Receiver<Event>,
}
impl Drop for TestLease {
    fn drop(&mut self) {
        if let Ok(mut inner) = self.registry.inner.lock() {
            if inner
                .slots
                .get(&self.peer)
                .is_some_and(|slot| slot.test_id == self.test_id)
            {
                inner.slots.remove(&self.peer);
            }
        }
    }
}

/// Bounded, single-owner registry of controlled tests for one Proxy instance.
/// Count and lifetime are per Proxy, not reset per call. Cancellation/errors
/// remove only the caller's registration.
#[derive(Debug)]
pub struct RuleTestRegistry {
    inner: std::sync::Mutex<RegistryInner>,
}

#[derive(Debug)]
struct RegistryInner {
    slots: std::collections::HashMap<std::net::SocketAddr, TestSlot>,
    issued: u64,
}

impl Default for RuleTestRegistry {
    fn default() -> Self {
        Self {
            inner: std::sync::Mutex::new(RegistryInner {
                slots: std::collections::HashMap::new(),
                issued: 0,
            }),
        }
    }
}

impl RuleTestRegistry {
    /// Reserve a bounded test slot for a private loopback peer and exact target.
    /// Returns the opaque one-use `test_id`. Enforces lifetime and per-Proxy
    /// budget (issued count and concurrent in-flight count).
    pub(crate) fn register(
        self: &Arc<Self>,
        peer: std::net::SocketAddr,
        host: &str,
        port: u16,
        lifetime: Duration,
    ) -> Result<TestLease, &'static str> {
        let mut inner = self.inner.lock().map_err(|_| "rule_test_poisoned")?;
        let now = Instant::now();
        inner.slots.retain(|_, slot| slot.expires_at > now);
        if inner.issued >= MAX_TEST_TOTAL {
            return Err("rule_test_exhausted");
        }
        if inner.slots.len() >= MAX_TEST_CONCURRENT {
            return Err("rule_test_in_flight");
        }
        if inner.slots.contains_key(&peer) {
            return Err("rule_test_in_flight");
        }
        inner.issued += 1;
        let test_id = format!(
            "rt-{}-{}",
            std::process::id(),
            NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
        );
        let (sender, completion) = tokio::sync::oneshot::channel();
        inner.slots.insert(
            peer,
            TestSlot {
                test_id: test_id.clone(),
                host: host.to_ascii_lowercase(),
                port,
                completion: Some(sender),
                expires_at: now + lifetime.min(MAX_TEST_LIFETIME),
            },
        );
        Ok(TestLease {
            registry: self.clone(),
            peer,
            test_id,
            completion,
        })
    }

    /// Consume the registration for an accepted peer if it matches the exact
    /// target. One-use: a second accept from the same peer finds nothing.
    pub(crate) fn consume(
        &self,
        peer: std::net::SocketAddr,
        host: &str,
        port: u16,
    ) -> Option<TestCompletion> {
        let mut inner = self.inner.lock().ok()?;
        let now = Instant::now();
        inner.slots.retain(|_, slot| slot.expires_at > now);
        let slot = inner.slots.get_mut(&peer)?;
        if slot.expires_at <= now || slot.host != host.to_ascii_lowercase() || slot.port != port {
            return None;
        }
        Some(TestCompletion {
            test_id: slot.test_id.clone(),
            sender: slot.completion.take()?,
        })
    }
}

/// Validate the finite `--test-telemetry` CLI input: a non-empty list of distinct
/// catalog ids bounded by the finite set. No arbitrary host or wildcard is
/// accepted; the caller resolves each id against [`host_for`].
pub fn valid_telemetry_ids(ids: &[String]) -> bool {
    !ids.is_empty() && ids.len() <= 8 && ids.iter().all(|id| host_for(id).is_some()) && {
        let mut seen = std::collections::HashSet::new();
        ids.iter().all(|id| seen.insert(id.as_str()))
    }
}

/// Resolve a finite `--test-telemetry` id list into the canonical destinations
/// the foreground server should prove are explicitly blocked.
pub fn test_destinations(ids: &[String]) -> Result<Vec<Destination>, &'static str> {
    if !valid_telemetry_ids(ids) {
        return Err("invalid_telemetry_test_ids");
    }
    ids.iter()
        .map(|id| {
            host_for(id)
                .ok_or("unknown_telemetry_destination")
                .and_then(|host| Destination::raw(host, 443))
        })
        .collect()
}

/// The static catalog envelope served by catalog transports. Kept distinct from
/// probe output so nothing here is confused with an actual observation.
pub fn catalog_envelope() -> serde_json::Value {
    serde_json::json!({
        "ok": true,
        "data": catalog(),
        "implementation": "static_catalog_no_network_no_state",
        "transport": "readonly_static"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_finite_exact_and_dated() {
        let catalog = catalog();
        assert_eq!(catalog["schema"], "lintel.telemetry-destinations/1");
        assert_eq!(catalog["checked_date"], "2026-10-09");
        assert_eq!(
            catalog["source"],
            "https://code.claude.com/docs/en/network-config"
        );
        let entries = catalog["destinations"].as_array().unwrap();
        assert!(!entries.is_empty());
        let mut ids = std::collections::HashSet::new();
        for entry in entries {
            // Every entry carries the honest, required fields.
            for field in [
                "id",
                "purpose",
                "provider",
                "clients",
                "applicability",
                "version_applicability",
                "source_url",
                "checked_date",
                "collateral_impact",
                "host",
                "port",
                "blockable",
            ] {
                assert!(!entry[field].is_null(), "missing {field}: {entry}");
            }
            let id = entry["id"].as_str().unwrap();
            assert!(ids.insert(id.to_string()), "duplicate id {id}");
            // Exact hosts only: no wildcard, no path, no scheme.
            let host = entry["host"].as_str().unwrap();
            assert!(!host.contains('*'), "wildcard host {host}");
            assert!(!host.contains('/'), "path in host {host}");
            assert!(!host.contains("://"), "scheme in host {host}");
            assert!(entry["port"].as_u64().is_some());
            // Version applicability is honest: no invented minimum version.
            assert!(!entry["version_applicability"]
                .as_str()
                .unwrap()
                .contains("2.1."));
        }
        // The two verified Datadog intake hosts are the only blockable entries.
        let blockable: Vec<&str> = entries
            .iter()
            .filter(|e| e["blockable"] == true)
            .map(|e| e["host"].as_str().unwrap())
            .collect();
        assert_eq!(
            blockable,
            vec![
                "http-intake.logs.us5.datadoghq.com",
                "browser-intake-us5-datadoghq.com"
            ]
        );
        // Mixed/necessary hosts are informational only.
        for host in [
            "api.anthropic.com",
            "claude.ai",
            "claude.com",
            "platform.claude.com",
            "downloads.claude.ai",
        ] {
            let entry = entries.iter().find(|e| e["host"] == host).unwrap();
            assert_eq!(entry["blockable"], false, "{host}");
        }
        // No historical Sentry/Statsig lead is a current blockable entry.
        for legacy in ["sentry.io", "statsig.com", "statsigapi.net"] {
            assert!(
                !entries.iter().any(|e| e["host"] == legacy),
                "legacy lead offered: {legacy}"
            );
        }
    }

    #[test]
    fn unknown_ids_and_wildcards_reject() {
        assert!(host_for("datadog_logs_intake").is_some());
        assert!(host_for("").is_none());
        assert!(host_for("unknown_id").is_none());
        assert!(host_for("*.datadoghq.com").is_none());
        assert!(host_for(&"a".repeat(65)).is_none());
        // No catalog id can be a raw host or a wildcard string.
        assert!(admitted_rule(&[], "http-intake.logs.us5.datadoghq.com").is_err());
        assert!(admitted_rule(&[], "datadog_logs_intake").is_ok());
    }

    #[test]
    fn selection_preserves_user_rules_and_rejects_dupes() {
        let user = vec![
            Rule {
                host: "user.invalid".into(),
                ports: vec![],
            },
            Rule {
                host: "mixed.example".into(),
                ports: vec![443],
            },
        ];
        let merged = merge_blocked(&user, &["datadog_logs_intake".into()]).unwrap();
        assert_eq!(merged[0].host, "user.invalid");
        assert_eq!(merged[1].host, "mixed.example");
        assert_eq!(merged[2].host, "http-intake.logs.us5.datadoghq.com");
        assert_eq!(merged[2].ports, vec![443]);
        // Selecting the same destination twice is an explicit failure, not a
        // silent duplicate.
        assert_eq!(
            merge_blocked(
                &user,
                &["datadog_logs_intake".into(), "datadog_logs_intake".into()]
            )
            .unwrap_err(),
            "telemetry_destination_already_blocked"
        );
        // An unknown id rejects the whole selection and leaves the draft alone.
        assert_eq!(
            merge_blocked(&user, &["datadog_logs_intake".into(), "bogus".into()]).unwrap_err(),
            "unknown_telemetry_destination"
        );
        // A whole-host user rule already covers the target.
        let whole = vec![Rule {
            host: "browser-intake-us5-datadoghq.com".into(),
            ports: vec![],
        }];
        assert_eq!(
            merge_blocked(&whole, &["datadog_browser_intake".into()]).unwrap_err(),
            "telemetry_destination_already_blocked"
        );
    }

    #[test]
    fn test_ids_are_finite_distinct_and_known() {
        assert!(valid_telemetry_ids(&["datadog_logs_intake".into()]));
        assert!(!valid_telemetry_ids(&[]));
        assert!(!valid_telemetry_ids(&[
            "datadog_logs_intake".into(),
            "datadog_logs_intake".into()
        ]));
        assert!(!valid_telemetry_ids(&["nope".into()]));
        let ids: Vec<String> = (0..9).map(|i| format!("id{i}")).collect();
        assert!(!valid_telemetry_ids(&ids));
        assert_eq!(
            test_destinations(&["nope".into()]).unwrap_err(),
            "invalid_telemetry_test_ids"
        );
    }

    #[test]
    fn registry_is_one_use_peer_bound_and_cancellation_owned() {
        let registry = Arc::new(RuleTestRegistry::default());
        let peer = "127.0.0.1:41000".parse().unwrap();
        let lease = registry
            .register(
                peer,
                "http-intake.logs.us5.datadoghq.com",
                443,
                Duration::from_secs(5),
            )
            .unwrap();
        assert!(lease.test_id.starts_with("rt-"));
        assert!(registry
            .consume(
                "127.0.0.1:41001".parse().unwrap(),
                "http-intake.logs.us5.datadoghq.com",
                443
            )
            .is_none());
        assert!(registry.consume(peer, "api.anthropic.com", 443).is_none());
        let completion = registry
            .consume(peer, "http-intake.logs.us5.datadoghq.com", 443)
            .unwrap();
        assert_eq!(completion.test_id, lease.test_id);
        assert!(registry
            .consume(peer, "http-intake.logs.us5.datadoghq.com", 443)
            .is_none());
        assert_eq!(
            registry.inner.lock().unwrap().slots.len(),
            1,
            "consumed work still in flight"
        );
        drop(lease);
        assert!(
            registry.inner.lock().unwrap().slots.is_empty(),
            "cancelled future frees only its own lease"
        );
    }

    #[test]
    fn registry_enforces_issued_budget_concurrency_expiry_and_distinct_instances() {
        let registry = Arc::new(RuleTestRegistry::default());
        for index in 0..MAX_TEST_TOTAL {
            let peer = format!("127.0.0.1:{}", 42000 + index).parse().unwrap();
            let lease = registry
                .register(
                    peer,
                    "http-intake.logs.us5.datadoghq.com",
                    443,
                    Duration::from_secs(30),
                )
                .unwrap();
            drop(lease);
        }
        assert_eq!(
            registry
                .register(
                    "127.0.0.1:42999".parse().unwrap(),
                    "http-intake.logs.us5.datadoghq.com",
                    443,
                    Duration::from_secs(30)
                )
                .unwrap_err(),
            "rule_test_exhausted"
        );
        let registry = Arc::new(RuleTestRegistry::default());
        let mut leases = Vec::new();
        for index in 0..MAX_TEST_CONCURRENT {
            leases.push(
                registry
                    .register(
                        format!("127.0.0.1:{}", 43000 + index).parse().unwrap(),
                        "http-intake.logs.us5.datadoghq.com",
                        443,
                        Duration::from_secs(30),
                    )
                    .unwrap(),
            );
        }
        assert_eq!(
            registry
                .register(
                    "127.0.0.1:43999".parse().unwrap(),
                    "http-intake.logs.us5.datadoghq.com",
                    443,
                    Duration::from_secs(5)
                )
                .unwrap_err(),
            "rule_test_in_flight"
        );
        for slot in registry.inner.lock().unwrap().slots.values_mut() {
            slot.expires_at = Instant::now() - Duration::from_millis(1);
        }
        let replacement = registry
            .register(
                "127.0.0.1:43000".parse().unwrap(),
                "http-intake.logs.us5.datadoghq.com",
                443,
                Duration::from_secs(5),
            )
            .unwrap();
        let replacement_id = replacement.test_id.clone();
        drop(leases);
        assert_eq!(
            registry.inner.lock().unwrap().slots.len(),
            1,
            "expired owner's Drop cannot remove replacement"
        );
        let other = Arc::new(RuleTestRegistry::default());
        let other_lease = other
            .register(
                "127.0.0.1:43000".parse().unwrap(),
                "http-intake.logs.us5.datadoghq.com",
                443,
                Duration::from_secs(5),
            )
            .unwrap();
        assert_ne!(
            other_lease.test_id, replacement_id,
            "proxy replacements cannot reuse a test id"
        );
    }
}

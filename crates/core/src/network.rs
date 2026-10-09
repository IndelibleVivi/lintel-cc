//! Shared host (not per-Claude-environment) IPv4/IPv6 observation, the finite
//! egress probe, and exact-approved macOS IPv6 service changes with restoration.
//!
//! Ownership split: `crates/egress` owns the single finite HTTPS echo probe and
//! the connection-address-family constraint. This module owns host observation,
//! frozen plans, the macOS SystemConfiguration mutation, original-job records
//! and restoration. The host network is a shared, machine-scoped resource, so
//! these tasks carry `environment_id: null` and never depend on a Claude
//! environment or the App's lifetime.
use crate::{err, now, storage::*, Engine, Result, Value};
use serde_json::json;
use std::path::Path;

/// A finite, platform-neutral view of one host network service (or, on Linux,
/// one interface). `mode` is derived from the frozen configuration, never from
/// a display name. `service_enabled` and the IPv6 `enabled` are independent
/// facts; `observed_addresses` comes from the live interface listing, not the
/// static manual configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServiceView {
    pub service_id: String,
    pub name: String,
    pub interface: String,
    pub mode: String,
    pub service_enabled: bool,
    pub ipv4_addresses: Vec<String>,
    pub ipv6_addresses: Vec<String>,
    /// The complete frozen IPv6 protocol projection:
    /// `{set_id, service_uuid, enabled, configuration}`. `configuration` is the
    /// full SC `SCNetworkProtocolGetConfiguration` dictionary. Restoring writes
    /// exactly this. On Linux it stays `null` (mutation unsupported).
    pub ipv6_protocol: Value,
}

#[derive(Clone, Debug)]
pub(crate) struct Observation {
    pub platform: &'static str,
    pub services: Vec<ServiceView>,
    pub interfaces: Vec<Value>,
    /// A stable digest of the *relevant* configuration, observed addresses,
    /// interface flags and route facts. It does not change merely because time
    /// passes. Anything the platform cannot read is an explicit limitation.
    pub revision: String,
    pub revision_complete: bool,
    pub limitations: Vec<String>,
}

/// The mutation outcome for one service. Whether the running system reflects
/// the new configuration is a separate `configuration_verified` fact.
#[derive(Clone, Debug)]
pub(crate) struct Applied {
    pub service: ServiceView,
    pub configuration_verified: bool,
}

/// Platform adapter for host network observation and (macOS only) exact IPv6
/// mutation. Production code selects the real adapter; unit tests inject a
/// synthetic one so no host-managed network is ever read or written.
pub(crate) trait HostNetwork {
    fn observe(&self) -> Result<Observation>;
    /// Write the exact frozen target protocol `after`
    /// (`{set_id,service_uuid,enabled,configuration}`) to `service_id`,
    /// re-checking the frozen identity and full prior configuration under an
    /// exclusive lock. `after` is applied verbatim — including any
    /// property-list keys this build does not interpret — so a restore is a
    /// faithful round trip. Platforms that only observe return unsupported.
    fn apply_target(
        &self,
        service_id: &str,
        before: &ServiceView,
        after: &Value,
    ) -> Result<Applied>;
}

/// Select the production adapter for this host. On Linux this is read-only.
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn host_adapter() -> &'static dyn HostNetwork {
    #[cfg(target_os = "macos")]
    {
        &macos::MACOS
    }
    #[cfg(target_os = "linux")]
    {
        &LINUX
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        &UNSUPPORTED
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
static UNSUPPORTED: UnsupportedHost = UnsupportedHost;

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
struct UnsupportedHost;

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
impl HostNetwork for UnsupportedHost {
    fn observe(&self) -> Result<Observation> {
        Err(unsupported_platform())
    }
    fn apply_target(&self, _: &str, _: &ServiceView, _: &Value) -> Result<Applied> {
        Err(unsupported_platform())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn unsupported_platform() -> crate::Error {
    err(
        "network_platform_unsupported",
        "此平台未提供宿主网络观察；不会读取或修改网络配置",
    )
}

fn mutation_unsupported() -> crate::Error {
    err(
        "network_mutation_unsupported",
        "此平台只提供只读网络观察与显式出口实测；不支持系统网络变更，也不会尝试提权",
    )
}

/// The finite loopback proxy admission, mirrored from the shared operations
/// contract so the core refuses a non-loopback or portless channel even if a
/// transport skipped its own check.
fn valid_loopback_proxy(url: &str) -> bool {
    let Some(authority) = url.strip_prefix("http://") else {
        return false;
    };
    authority
        .parse::<std::net::SocketAddr>()
        .is_ok_and(|addr| addr.ip().is_loopback() && addr.port() != 0)
}

/// Ensure a private on-disk directory exists for a state file path.
fn ensure_private(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        private_dir(parent)?;
    }
    Ok(())
}

/// Stable digest of the relevant host network facts. Ordered and normalized so
/// it only changes when the configuration, addresses, interface flags or
/// routes change.
pub(crate) fn revision_of(platform: &str, services: &[ServiceView]) -> String {
    let mut normalized: Vec<Value> = services
        .iter()
        .map(|s| {
            json!({
                "service_id": s.service_id,
                "interface": s.interface,
                "mode": s.mode,
                "service_enabled": s.service_enabled,
                "ipv4": s.ipv4_addresses,
                "ipv6": s.ipv6_addresses,
                "protocol": s.ipv6_protocol,
            })
        })
        .collect();
    normalized.sort_by_key(|v| v["service_id"].as_str().unwrap_or("").to_string());
    digest(&serde_json::to_vec(&json!({"platform": platform, "services": normalized})).unwrap())
}

pub(crate) fn service_json(service: &ServiceView) -> Value {
    json!({
        "service_id": service.service_id,
        "name": service.name,
        "interface": service.interface,
        "mode": service.mode,
        "enabled": service.service_enabled,
        "ipv4_addresses": service.ipv4_addresses,
        "ipv6_addresses": service.ipv6_addresses,
        "ipv6_enabled": service.ipv6_protocol["enabled"],
    })
}

/// Observed process-host name, private alongside the explicit probe result.
/// It is independent from the caller's selected SSH alias.
fn execution_host() -> Option<String> {
    #[cfg(test)]
    {
        Some("synthetic-host".into())
    }
    #[cfg(not(test))]
    {
        let mut bytes = [0_u8; 256];
        if unsafe { libc::gethostname(bytes.as_mut_ptr().cast(), bytes.len()) } != 0 {
            return None;
        }
        let end = bytes.iter().position(|byte| *byte == 0)?;
        let name = std::str::from_utf8(&bytes[..end]).ok()?;
        (!name.is_empty() && !name.chars().any(char::is_control)).then(|| name.to_owned())
    }
}

/// Finite, machine-readable boundaries shown beside every observation.
pub(crate) fn default_limitations(platform: &str) -> Vec<String> {
    let mut limitations = vec![
        "这是宿主共享网络：一个服务上的变更影响使用它的其他应用，不属于某个 Claude 环境。".into(),
        "配置、地址观察与请求结果分别表达；有地址或路由不代表应用已经使用它。".into(),
        "单次请求不证明 DNS、UDP、QUIC、WebRTC 或其他应用流量的覆盖。".into(),
    ];
    if platform == "macos" {
        limitations.push("macOS 服务身份使用当前 SCNetworkSet 的 service UUID 与接口，不使用显示名称；地址与路由来自动态存储观察。".into());
    } else {
        limitations.push("Linux 仅提供只读观察与显式出口实测；系统级网络变更明确不受支持。".into());
    }
    limitations
}

// ---------------------------------------------------------------------------
// Core integration: probe persistence, plans, execute, restore.
// ---------------------------------------------------------------------------

/// Where a probe result is kept for finite baseline reuse. It lives in private
/// Engine state (never in the repository) and is keyed by a UUID the caller can
/// pass back as `baseline_id`.
const PROBE_DIR: &str = "network-probes";
const BASELINE_FRESH_SECONDS: i64 = 300;

impl Engine {
    fn network_path(&self, dir: &str, id: &str) -> std::path::PathBuf {
        self.state.join(dir).join(format!("{id}.json"))
    }

    /// The adapter for this host. A unit-test fixture always wins; a synthetic
    /// `LINTEL_TEST_HOME` (used by the CLI synthetic journey) fails closed so it
    /// can never read or write the real machine; production uses the real host.
    fn network(&self) -> Result<&dyn HostNetwork> {
        #[cfg(test)]
        {
            if let Some(fixture) = &self.network_fixture {
                return Ok(fixture.as_ref());
            }
            return Err(err(
                "network_test_fixture_required",
                "unit-test Engine 没有 synthetic network fixture；不访问宿主真实网络",
            ));
        }
        #[cfg(not(test))]
        {
            if std::env::var_os("LINTEL_TEST_HOME").is_some() {
                return Err(err(
                    "network_synthetic_home_unsupported",
                    "合成测试环境不读取或修改真实主机网络；请在实际主机运行此操作",
                ));
            }
            Ok(host_adapter())
        }
    }

    /// Run the finite probe. Tests substitute the probe output; production runs
    /// the real egress probe exactly once per explicit request.
    fn run_probe_for(&self, spec: &ProbeSpec, platform: &str, revision: &str) -> Result<Value> {
        #[cfg(test)]
        if let Some(hook) = self.network_probe_hook {
            return hook(spec, platform, revision);
        }
        execute_probe(spec, platform, revision)
    }

    pub(crate) fn network_inspect(&self, _r: &Value) -> Result<Value> {
        let observation = self.network()?.observe()?;
        let services: Vec<Value> = observation.services.iter().map(service_json).collect();
        Ok(json!({
            "schema": "lintel.network/1",
            "platform": observation.platform,
            "execution_host": execution_host(),
            "network_revision": observation.revision,
            "network_revision_complete": observation.revision_complete,
            "services": services,
            "interfaces": observation.interfaces,
            "limitations": observation.limitations,
        }))
    }
}

// ---------------------------------------------------------------------------
// Probe: the single explicit outbound act. Delegates bytes to lintel-egress
// and stores a private result keyed by UUID for finite baseline reuse.
// ---------------------------------------------------------------------------

/// The frozen, independently validated probe request: endpoints, optional
/// loopback channel, deadline and the App-instance `proxy_binding`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProbeSpec {
    pub ipv4_url: String,
    pub ipv6_url: String,
    pub proxy_url: Option<String>,
    pub proxy_binding: Option<String>,
    pub timeout_seconds: u64,
}

impl ProbeSpec {
    pub(crate) fn from_value(value: &Value) -> Result<Self> {
        if !value.is_object() {
            return Err(err("invalid_probe", "探测配置需要对象"));
        }
        if let Some(object) = value.as_object() {
            for key in object.keys() {
                if !matches!(
                    key.as_str(),
                    "ipv4_url" | "ipv6_url" | "proxy_url" | "proxy_binding" | "timeout_seconds"
                ) {
                    return Err(err("invalid_probe", "探测配置包含未知字段"));
                }
            }
        }
        let ipv4_url = crate::string(value, "ipv4_url")?.to_string();
        let ipv6_url = crate::string(value, "ipv6_url")?.to_string();
        if !lintel_operations::valid_probe_url(&ipv4_url)
            || !lintel_operations::valid_probe_url(&ipv6_url)
        {
            return Err(err(
                "invalid_probe",
                "探测目标需要是有界的 HTTPS IP 回显地址，不接受凭据、query、fragment 或跳转",
            ));
        }
        lintel_egress::probe::Endpoint::parse(&ipv4_url)
            .map_err(|_| err("invalid_probe", "IPv4 回显目标无效"))?;
        lintel_egress::probe::Endpoint::parse(&ipv6_url)
            .map_err(|_| err("invalid_probe", "IPv6 回显目标无效"))?;
        let proxy_url = match value.get("proxy_url") {
            None | Some(Value::Null) => None,
            Some(Value::String(text)) if !text.is_empty() => {
                if !valid_loopback_proxy(text) {
                    return Err(err(
                        "invalid_proxy",
                        "proxy_url 只接受带明确端口的 loopback HTTP 地址",
                    ));
                }
                Some(text.clone())
            }
            Some(_) => {
                return Err(err(
                    "invalid_proxy",
                    "proxy_url 只接受带明确端口的 loopback HTTP 地址",
                ))
            }
        };
        let proxy_binding = match value.get("proxy_binding") {
            None | Some(Value::Null) => None,
            Some(Value::String(text)) if !text.is_empty() && text.len() <= 128 => {
                Some(text.clone())
            }
            Some(_) => {
                return Err(err(
                    "invalid_proxy_binding",
                    "proxy_binding 需要不超过 128 字节的非空标识",
                ))
            }
        };
        if proxy_binding.is_some() && proxy_url.is_none() {
            return Err(err(
                "invalid_proxy_binding",
                "通道实例绑定需要同时提供 loopback 通道地址",
            ));
        }
        let timeout_seconds = match value.get("timeout_seconds") {
            None | Some(Value::Null) => 10,
            Some(number) => number
                .as_u64()
                .filter(|seconds| (1..=15).contains(seconds))
                .ok_or_else(|| err("invalid_timeout", "timeout_seconds 必须在 1 到 15 之间"))?,
        };
        Ok(ProbeSpec {
            ipv4_url,
            ipv6_url,
            proxy_url,
            proxy_binding,
            timeout_seconds,
        })
    }

    /// The public projection shown in plans and receipts. It retains
    /// `proxy_binding` and `timeout_seconds` so what was approved is visible.
    fn public(&self) -> Value {
        json!({
            "ipv4_url": self.ipv4_url,
            "ipv6_url": self.ipv6_url,
            "proxy_url": self.proxy_url,
            "proxy_binding": self.proxy_binding,
            "timeout_seconds": self.timeout_seconds,
        })
    }

    fn same_as(&self, other: &ProbeSpec) -> bool {
        self.ipv4_url == other.ipv4_url
            && self.ipv6_url == other.ipv6_url
            && self.proxy_url == other.proxy_url
            && self.proxy_binding == other.proxy_binding
            && self.timeout_seconds == other.timeout_seconds
    }
}

/// Production probe entry. Unit tests inject a synthetic probe so no real
/// endpoint is ever contacted; production always calls this.
fn execute_probe(spec: &ProbeSpec, platform: &str, revision: &str) -> Result<Value> {
    let request = lintel_egress::probe::ProbeRequest {
        ipv4_url: spec.ipv4_url.clone(),
        ipv6_url: spec.ipv6_url.clone(),
        proxy_url: spec.proxy_url.clone(),
        timeout_seconds: spec.timeout_seconds,
    };
    let channel = spec.proxy_url.as_ref().map(|url| {
        let authority = url.strip_prefix("http://").unwrap_or(url).to_string();
        lintel_egress::probe::ProxyChannel {
            authority,
            family: lintel_egress::AddressFamily::System,
        }
    });
    let cells = lintel_egress::probe::run_blocking(&request, channel).map_err(|code| {
        err(
            "network_probe_failed",
            &format!("探测请求未完成（{code}）；没有取得出口结论"),
        )
    })?;
    let cells: Vec<Value> = cells
        .iter()
        .map(|cell| {
            let status = serde_json::to_value(cell.status)
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .unwrap_or_else(|| "connection_failed".into());
            json!({
                "path": cell.path,
                "family": cell.family,
                "status": status,
                "public_ip": cell.public_ip,
                "elapsed_ms": cell.elapsed_ms,
                "peer_family": cell.peer_family,
                "message": cell.message,
            })
        })
        .collect();
    Ok(json!({
        "schema": "lintel.network-probe/1",
        "id": crate::id(),
        "executed_at": now(),
        "platform": platform,
        "execution_host": execution_host(),
        "network_revision": revision,
        "endpoints": {"ipv4": spec.ipv4_url, "ipv6": spec.ipv6_url},
        "proxy_url": spec.proxy_url,
        "proxy_binding": spec.proxy_binding,
        "timeout_seconds": spec.timeout_seconds,
        "cells": cells,
    }))
}

impl Engine {
    fn store_probe(&self, probe: &Value) -> Result<()> {
        let path = self.network_path(PROBE_DIR, crate::string(probe, "id")?);
        ensure_private(&path)?;
        save(&path, probe)
    }

    fn load_probe(&self, id: &str) -> Option<Value> {
        if uuid::Uuid::parse_str(id).is_err() {
            return None;
        }
        let path = self.network_path(PROBE_DIR, id);
        load(&path).ok().filter(Value::is_object)
    }

    /// Reuse a baseline only for the same target, channel, binding and deadline,
    /// with the current host revision, inside the finite freshness window. A
    /// stale-flagged probe is never reused.
    fn reusable_baseline(
        &self,
        baseline_id: &str,
        spec: &ProbeSpec,
        revision: &str,
    ) -> Option<Value> {
        let probe = self.load_probe(baseline_id)?;
        let endpoints = &probe["endpoints"];
        let stored = ProbeSpec {
            ipv4_url: endpoints["ipv4"].as_str()?.to_string(),
            ipv6_url: endpoints["ipv6"].as_str()?.to_string(),
            proxy_url: probe["proxy_url"].as_str().map(str::to_owned),
            proxy_binding: probe["proxy_binding"].as_str().map(str::to_owned),
            timeout_seconds: probe["timeout_seconds"].as_u64().unwrap_or(10),
        };
        if !stored.same_as(spec) {
            return None;
        }
        if probe["network_revision"].as_str() != Some(revision) {
            return None;
        }
        if probe["stale"].as_bool() == Some(true) {
            return None;
        }
        let executed = probe["executed_at"].as_str()?;
        let executed = chrono::DateTime::parse_from_rfc3339(executed).ok()?;
        let age = chrono::Utc::now().signed_duration_since(executed.with_timezone(&chrono::Utc));
        if age.num_seconds() < 0 || age.num_seconds() > BASELINE_FRESH_SECONDS {
            return None;
        }
        Some(probe)
    }

    /// Run one explicit probe for a frozen spec, marking it stale if the host
    /// revision moved between the observation and the request. Reuses a valid
    /// baseline when `allow_reuse` names one.
    fn probe_with_staleness(
        &self,
        spec: &ProbeSpec,
        platform: &str,
        revision: &str,
        allow_reuse: Option<&str>,
    ) -> Result<Value> {
        if let Some(baseline_id) = allow_reuse {
            if let Some(probe) = self.reusable_baseline(baseline_id, spec, revision) {
                return Ok(probe);
            }
        }
        let mut probe = self.run_probe_for(spec, platform, revision)?;
        match self.network().and_then(|adapter| adapter.observe()) {
            Ok(after) if after.revision != revision || !after.revision_complete => {
                probe["stale"] = json!(true);
                probe["network_revision_changed"] = json!(after.revision);
            }
            Err(failure) => {
                probe["stale"] = json!(true);
                probe["observation_error"] = json!(failure.code);
            }
            _ => {}
        }
        self.store_probe(&probe)?;
        Ok(probe)
    }

    pub(crate) fn network_probe(&self, r: &Value) -> Result<Value> {
        // The command request carries `command` alongside the probe fields; drop
        // only that envelope key before the strict probe validation.
        let mut probe = r.clone();
        if let Some(object) = probe.as_object_mut() {
            object.remove("command");
        }
        let spec = ProbeSpec::from_value(&probe)?;
        let observation = self.network()?.observe()?;
        let mut probe =
            self.probe_with_staleness(&spec, observation.platform, &observation.revision, None)?;
        if !observation.revision_complete {
            probe["stale"] = json!(true);
            probe["observation_error"] = json!("network_revision_incomplete");
            self.store_probe(&probe)?;
        }
        Ok(probe)
    }

    pub(crate) fn plan_network_ipv6(&self, r: &Value) -> Result<Value> {
        let mode = crate::string(r, "mode")?;
        if !matches!(mode, "off" | "link_local") {
            return Err(err(
                "invalid_mode",
                "只支持 off 或 link_local 两种 IPv6 变更",
            ));
        }
        let service_id = crate::string(r, "service_id")?;
        let probe_value = r
            .get("probe")
            .ok_or_else(|| err("invalid_probe", "缺少冻结的探测目标"))?;
        let spec = ProbeSpec::from_value(probe_value)?;
        let observation = self.network()?.observe()?;
        if observation.platform != "macos" {
            return Err(mutation_unsupported());
        }
        let before = observation
            .services
            .iter()
            .find(|service| service.service_id == service_id)
            .cloned()
            .ok_or_else(|| {
                err(
                    "network_service_missing",
                    "没有找到此网络服务；请重新观察后再预览",
                )
            })?;
        if before.ipv6_protocol.is_null() || before.ipv6_protocol["configuration"].is_null() {
            return Err(err(
                "network_ipv6_missing",
                "该服务没有可保真的 IPv6 协议配置；未生成变更计划",
            ));
        }
        let baseline = r.get("baseline_id").and_then(Value::as_str);
        let before_probe = self.probe_with_staleness(
            &spec,
            observation.platform,
            &observation.revision,
            baseline,
        )?;
        let after = ipv6_after(&before, mode);
        let warnings = vec![
            "此变更作用于宿主共享网络服务；其他使用它的应用和 IPv6-only 网络、依赖 IPv6 的 VPN 可能失去连接。".into(),
            "确认只修改所选服务的 IPv6 协议；不会改动 IPv4、系统代理、DNS 或其他服务。".into(),
            "批准包括写入读回后对相同目标与路径的自动复测；探测失败不会被当作防泄漏成功。".into(),
            "变更不随 Claude 环境或 App 退出而撤销；需要明确恢复。".into(),
        ];
        let extra = json!({
            "network": {
                "scope": "host_shared",
                "service_id": before.service_id,
                "service_name": before.name,
                "interface": before.interface,
                "service_enabled": before.service_enabled,
                "before": before.ipv6_protocol,
                "after": after,
                "probe": spec.public(),
                "before_probe": before_probe,
                "mode": mode,
            }
        });
        self.network_plan(
            "network_ipv6",
            match mode {
                "off" => "关闭该网络服务的 IPv6",
                _ => "将该网络服务设为仅链路本地",
            },
            extra,
            warnings,
            json!([
                {"id":"network_write","label":"在系统授权下写入并读回所选 IPv6 协议","reversible":true},
                {"id":"network_reprobe","label":"读回后用相同目标与路径自动复测","reversible":false}
            ]),
        )
    }

    pub(crate) fn plan_network_restore(&self, r: &Value) -> Result<Value> {
        let jid = crate::safe_id(r, "job_id")?;
        let job = self.dispatch(&json!({"command":"job","job_id":jid}))?;
        if job["network_change"].is_null() {
            return Err(err("not_restorable", "此任务没有可恢复的宿主网络改动"));
        }
        if job["restorable"] != true || job["network_change"]["configuration_verified"] != true {
            return Err(err(
                "not_restorable",
                "原任务的网络写入未被读回确认；只查询原任务，不据此恢复",
            ));
        }
        let original = self.network_original(&jid)?;
        let network = original["extra"]["network"].clone();
        let service_id = crate::string(&network, "service_id")?;
        let observation = self.network()?.observe()?;
        if observation.platform != "macos" {
            return Err(mutation_unsupported());
        }
        let current = observation
            .services
            .iter()
            .find(|service| service.service_id == service_id)
            .cloned()
            .ok_or_else(|| err("network_service_missing", "原服务在预览后消失"))?;
        if current.interface != crate::string(&network, "interface")?
            || Some(current.service_enabled) != network["service_enabled"].as_bool()
        {
            return Err(err(
                "restore_conflict",
                "该服务的接口在任务后变化；保留当前配置，不覆盖",
            ));
        }
        if current.ipv6_protocol != network["after"] {
            return Err(err(
                "restore_conflict",
                "该服务的 IPv6 配置在任务后被外部修改；保留后续编辑，不覆盖",
            ));
        }
        let spec = match r.get("probe") {
            Some(probe) if probe.is_object() => ProbeSpec::from_value(probe)?,
            _ => ProbeSpec::from_value(&network["probe"])?,
        };
        let baseline = r.get("baseline_id").and_then(Value::as_str);
        let before_probe = self.probe_with_staleness(
            &spec,
            observation.platform,
            &observation.revision,
            baseline,
        )?;
        let warnings = vec![
            "恢复只写回原任务记录的完整 IPv6 配置；后来发生的外部修改会被拒绝而不是覆盖。".into(),
            "恢复后使用相同目标与路径自动复测；原通道若已停止，可显式选择新的测试目标。".into(),
        ];
        let extra = json!({
            "network": {
                "scope": "host_shared",
                "service_id": current.service_id,
                "service_name": current.name,
                "interface": current.interface,
                "service_enabled": current.service_enabled,
                "before": current.ipv6_protocol,
                "after": network["before"],
                "probe": spec.public(),
                "before_probe": before_probe,
                "mode": "restore",
                "original_job": jid,
            }
        });
        self.network_plan(
            "network_restore",
            "恢复原 IPv6 配置",
            extra,
            warnings,
            json!([
                {"id":"network_write","label":"在系统授权下写回原 IPv6 协议并读回","reversible":true},
                {"id":"network_reprobe","label":"读回后用相同目标与路径自动复测","reversible":false}
            ]),
        )
    }

    fn network_plan(
        &self,
        kind: &str,
        title: &str,
        extra: Value,
        warnings: Vec<String>,
        actions: Value,
    ) -> Result<Value> {
        let network = &extra["network"];
        let changes = json!([{
            "scope": "host_shared",
            "service_id": network["service_id"],
            "service_name": network["service_name"],
            "interface": network["interface"],
            "service_enabled": network["service_enabled"],
            "before": network["before"],
            "after": network["after"],
        }]);
        let mut p = json!({
            "id": crate::id(),
            "environment_id": Value::Null,
            "kind": kind,
            "title": title,
            "changes": changes,
            "preserves": ["IPv4、系统代理、DNS 与其他服务", "当前登录、凭据与工作内容"],
            "warnings": warnings,
            "actions": actions,
            "created_at": now(),
            "status": "planned",
            "rule_version": crate::policy::RULE,
            "extra": extra,
        });
        p["hash"] = json!(digest(&serde_json::to_vec(&p)?));
        save(&self.path("plans", crate::string(&p, "id")?), &p)?;
        crate::public_plan(p)
    }

    fn network_original(&self, job_id: &str) -> Result<Value> {
        let plan = load(&self.path("plans", job_id))?;
        if !plan.is_object() || !plan["hash"].is_string() {
            return Err(err("invalid_plan", "原网络计划损坏"));
        }
        let mut unhashed = plan.clone();
        unhashed.as_object_mut().unwrap().remove("hash");
        if plan["hash"] != digest(&serde_json::to_vec(&unhashed)?) {
            return Err(err("plan_changed", "原网络计划已变化；不据此恢复"));
        }
        if plan["extra"]["network"].is_null() {
            return Err(err("invalid_plan", "原计划没有网络记录"));
        }
        Ok(plan)
    }

    /// Re-verify an approved plan's frozen facts immediately before a new job is
    /// admitted. A stale plan or changed identity produces no job.
    fn verify_network_ready(&self, p: &Value) -> Result<()> {
        if p["rule_version"] != crate::policy::RULE {
            return Err(err("rule_changed", "网络规则版本已更新，请重新预览"));
        }
        let network = &p["extra"]["network"];
        let spec = ProbeSpec::from_value(&network["probe"])?;
        if network["before_probe"]["cells"].as_array().map(Vec::len) != Some(4) {
            return Err(err("invalid_plan", "冻结的前测结果不完整"));
        }
        let observation = self.network()?.observe()?;
        if observation.platform != "macos" {
            return Err(mutation_unsupported());
        }
        if !observation.revision_complete
            || network["before_probe"]["id"]
                .as_str()
                .and_then(|id| self.reusable_baseline(id, &spec, &observation.revision))
                .is_none()
        {
            return Err(err(
                "stale_network_plan",
                "前测已经过期或网络观察变化／不完整；请重新预览，未生成新任务",
            ));
        }
        let service_id = crate::string(network, "service_id")?;
        let current = observation
            .services
            .iter()
            .find(|service| service.service_id == service_id)
            .ok_or_else(|| err("stale_network_plan", "预览后该网络服务消失；未生成新任务"))?;
        if current.interface != crate::string(network, "interface")?
            || current.name != crate::string(network, "service_name")?
            || Some(current.service_enabled) != network["service_enabled"].as_bool()
        {
            return Err(err(
                "stale_network_plan",
                "预览后服务身份或接口变化；请重新预览",
            ));
        }
        if current.ipv6_protocol != network["before"] {
            return Err(err(
                "stale_network_plan",
                "预览后 IPv6 配置被外部修改；保留外部改动，请重新预览",
            ));
        }
        Ok(())
    }
}

/// The full target protocol projection for a finite IPv6 mode, built from the
/// frozen projection so unrelated keys are preserved. `off` keeps the whole
/// original dictionary and only sets `enabled=false`; `link_local` selects the
/// LinkLocal method and drops only the manual-address keys that method excludes.
fn ipv6_after(before: &ServiceView, mode: &str) -> Value {
    let mut configuration = before
        .ipv6_protocol
        .get("configuration")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if !configuration.is_object() {
        configuration = json!({});
    }
    if mode != "off" {
        configuration["ConfigMethod"] = json!("LinkLocal");
        if let Some(object) = configuration.as_object_mut() {
            object.remove("Addresses");
            object.remove("PrefixLength");
            object.remove("Router");
        }
    }
    json!({
        "set_id": before.ipv6_protocol.get("set_id"),
        "service_uuid": before.ipv6_protocol.get("service_uuid"),
        "enabled": mode != "off",
        "configuration": configuration,
    })
}

impl Engine {
    /// Build the durable receipt and run an exact-approved network plan. This
    /// mirrors the generic executor's record shape (accepted -> executing ->
    /// completed/needs_reconciliation) but with no Claude environment. Frozen
    /// facts are re-verified before any new job is admitted.
    pub(crate) fn execute_network_plan(
        &self,
        p: &Value,
        pid: &str,
        journal: &Path,
    ) -> Result<Value> {
        // Pre-acceptance verification: a stale plan or changed identity produces
        // no job and no ACK.
        self.verify_network_ready(p)?;
        let network = &p["extra"]["network"];
        let mut j = json!({
            "id": pid,
            "plan_id": pid,
            "environment_id": Value::Null,
            "scope": "host_shared",
            "title": p["title"],
            "status": "accepted",
            "created_at": now(),
            "restorable": false,
            "warnings": p["warnings"],
            "steps": [],
            // Persist the complete frozen before/after with the acceptance so a
            // query after a kill still identifies the host-scoped change.
            "network_change": {
                "scope": "host_shared",
                "service_id": network["service_id"],
                "service_name": network["service_name"],
                "interface": network["interface"],
                "service_enabled": network["service_enabled"],
                "before": network["before"],
                "after": network["after"],
                "configuration_verified": false,
            },
            "before_probe": network["before_probe"],
            "after_probe": Value::Null,
        });
        if let Some(context) = &self.execution_context {
            j["execution"] = context.clone();
        }
        save(journal, &j)?;
        if let Some(hook) = self.accept_hook {
            hook(&j);
        }
        j["status"] = json!("executing");
        save(journal, &j)?;
        let result = self.execute_network(p, &mut j, journal);
        match result {
            Ok(()) => {
                let probe_done = j["steps"].as_array().is_some_and(|steps| {
                    steps.iter().any(|step| {
                        step["id"] == "network_reprobe" && step["status"] == "completed"
                    })
                });
                j["status"] = json!(if probe_done {
                    "completed"
                } else {
                    "partially_completed"
                });
            }
            Err(failure) => {
                // Pre-write denial is a definite failure; nothing was written.
                // Any uncertainty after the write intent is a reconciliation
                // state that keeps the original job.
                let phase = j["status"].as_str().unwrap_or("executing").to_string();
                let certain = failure.code == "network_authorization_denied"
                    || failure.code == "network_locked"
                    || failure.code == "network_prepare_failed"
                    || failure.code == "network_write_failed"
                    || failure.code == "network_test_fixture_required"
                    || failure.code == "network_synthetic_home_unsupported"
                    || failure.code == "network_mutation_unsupported";
                let certain = certain
                    || matches!(
                        failure.code.as_str(),
                        "stale_network_plan"
                            | "network_ipv6_missing"
                            | "invalid_plan"
                            | "invalid_service_id"
                    );
                if j["network_change"]["configuration_verified"] == true {
                    j["status"] = json!("partially_completed");
                    j["error"] = json!({"code":failure.code,"message":failure.message,"phase":phase,
                        "uncertain_side_effects":false,"recovery":"配置读回已确认；后测或记录未完成。保留原任务，不重复写入。"});
                } else if certain {
                    j["status"] = json!("failed");
                    j["restorable"] = json!(false);
                    j["error"] = json!({
                        "code": failure.code,
                        "message": failure.message,
                        "phase": phase,
                        "uncertain_side_effects": false,
                        "recovery": "未修改网络设置；可修正后重新预览。",
                    });
                } else {
                    j["status"] = json!("needs_reconciliation");
                    j["network_change"]["configuration_verified"] = json!(false);
                    j["error"] = json!({
                        "code": failure.code,
                        "message": failure.message,
                        "phase": phase,
                        "uncertain_side_effects": true,
                        "recovery": "查询此任务的原始记录（同一 ID），核对配置读回与探测结果；不要重复提交网络写入。",
                    });
                }
                j["warnings"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!(failure.message));
            }
        }
        j["task_result"] = crate::task_result(p, &j);
        save(journal, &j)?;
        Ok(j)
    }

    /// Execute one exact-approved network change. Intent is persisted before the
    /// attempt; the post-write reprobe is a separate outcome that never rolls
    /// back or misreports a verified configuration change.
    fn execute_network(&self, p: &Value, j: &mut Value, journal: &Path) -> Result<()> {
        let network = &p["extra"]["network"];
        let service_id = crate::string(network, "service_id")?;
        let spec = ProbeSpec::from_value(&network["probe"])?;
        let before = ServiceView {
            service_id: service_id.to_string(),
            name: crate::string(network, "service_name")?.to_string(),
            interface: network["interface"].as_str().unwrap_or("").to_string(),
            mode: "unknown".into(),
            service_enabled: network["service_enabled"]
                .as_bool()
                .ok_or_else(|| err("invalid_plan", "冻结计划缺少服务 enabled 状态"))?,
            ipv4_addresses: vec![],
            ipv6_addresses: vec![],
            ipv6_protocol: network["before"].clone(),
        };
        j["steps"] = json!([
            {"id":"network_write","label":"在系统授权下写入并读回所选 IPv6 协议","status":"executing","message":"写入意图已保存；中断后只查询原任务"},
            {"id":"network_reprobe","label":"读回后用相同目标与路径自动复测","status":"pending"}
        ]);
        save(journal, j)?;
        let applied = match self
            .network()?
            .apply_target(service_id, &before, &network["after"])
        {
            Ok(applied) => applied,
            Err(failure) => {
                // The write may or may not have landed; the caller maps a
                // pre-write denial vs. an uncertain post-intent failure.
                set_step(j, "network_write", "unknown", &failure.message);
                return Err(failure);
            }
        };
        j["network_change"]["configuration_verified"] = json!(applied.configuration_verified);
        j["network_change"]["observed"] = service_json(&applied.service);
        if !applied.configuration_verified {
            // The write did not read back as the frozen `after`; this is not a
            // verified change, so it is not restorable from this job.
            set_step(
                j,
                "network_write",
                "unknown",
                "配置写入命令已完成，但读回与预期不一致；请查询原任务核对，未确认为本任务的写入。",
            );
            return Err(err(
                "network_readback_mismatch",
                "写入后读回与冻结目标不一致；不认定为已完成的配置变更，未提供恢复",
            ));
        }
        set_step(
            j,
            "network_write",
            "completed",
            "配置已写入并读回一致；网络效果由复测单独呈现。",
        );
        j["restorable"] = json!(true);
        j["status"] = json!("verifying");
        save(journal, j)?;
        // The post-write reprobe is a separate outcome.
        let observed = self.network().and_then(|adapter| adapter.observe());
        let revision = observed.as_ref().map(|o| o.revision.as_str()).unwrap_or("");
        let after_probe = self.probe_with_staleness(&spec, "macos", revision, None);
        match after_probe {
            Ok(mut after_probe) => {
                if let Err(failure) = observed {
                    after_probe["stale"] = json!(true);
                    after_probe["observation_error"] = json!(failure.code);
                    self.store_probe(&after_probe)?;
                }
                j["after_probe"] = after_probe;
                set_step(
                    j,
                    "network_reprobe",
                    "completed",
                    "已用相同目标与路径复测；配置与出口结果分别呈现。",
                );
            }
            Err(failure) => {
                j["after_probe"] = Value::Null;
                set_step(j, "network_reprobe", "failed", &failure.message);
                j["warnings"].as_array_mut().unwrap().push(json!(
                    "配置变更已完成；后测请求未完成。这不改变已验证的配置，也不等于出口安全。"
                ));
            }
        }
        Ok(())
    }

    /// Query-time reconciliation for a network job whose execution was
    /// interrupted. It never re-runs a write or a probe; a persisted verified
    /// fact is preserved, and only an unfinished intent is marked uncertain.
    pub(crate) fn reconcile_network_job(&self, job: &mut Value) -> bool {
        if job["network_change"].is_null() {
            return false;
        }
        let unfinished = matches!(
            job["status"].as_str(),
            Some("accepted" | "executing" | "verifying")
        );
        if !unfinished {
            return false;
        }
        if job["network_change"]["configuration_verified"] == true {
            // A verified configuration write already survived; it stays
            // restorable even if the process died before the reprobe finished.
            let reprobe_done = job["steps"].as_array().is_some_and(|steps| {
                steps
                    .iter()
                    .any(|step| step["id"] == "network_reprobe" && step["status"] == "completed")
            });
            job["status"] = json!(if reprobe_done {
                "completed"
            } else {
                "partially_completed"
            });
            if !reprobe_done {
                set_step(
                    job,
                    "network_reprobe",
                    "unknown",
                    "后测执行中断；只查询原任务，未自动重跑公网请求。",
                );
            }
            return true;
        }
        job["status"] = json!("needs_reconciliation");
        job["restorable"] = json!(false);
        job["warnings"].as_array_mut().map(|warnings| {
            warnings.push(json!(
                "网络写入被中断且未经读回确认；配置是否生效未确认。查询此原任务，不要重复提交。"
            ))
        });
        true
    }
}

fn set_step(j: &mut Value, id: &str, status: &str, message: &str) {
    if let Some(steps) = j["steps"].as_array_mut() {
        if let Some(step) = steps.iter_mut().find(|step| step["id"] == id) {
            step["status"] = json!(status);
            step["message"] = json!(message);
        }
    }
}

// ---------------------------------------------------------------------------
// Linux: read-only getifaddrs + bounded /proc route reads. No mutation.
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
static LINUX: LinuxHost = LinuxHost;

#[cfg(target_os = "linux")]
struct LinuxHost;

#[cfg(target_os = "linux")]
impl HostNetwork for LinuxHost {
    fn observe(&self) -> Result<Observation> {
        let interfaces = interface_facts()?;
        let services = interfaces
            .iter()
            .map(|(name, facts)| ServiceView {
                service_id: name.clone(),
                name: name.clone(),
                interface: name.clone(),
                mode: "unknown".into(),
                service_enabled: facts.flags & libc::IFF_UP as u32 != 0,
                ipv4_addresses: facts.ipv4.clone(),
                ipv6_addresses: facts.ipv6.clone(),
                ipv6_protocol: Value::Null,
            })
            .collect::<Vec<_>>();
        let mut limitations = default_limitations("linux");
        let routes = match linux_routes() {
            Ok(routes) => json!(routes),
            Err(failure) => {
                limitations.push(format!(
                    "路由观察不完整（{}）；network_revision 不包含完整路由表。",
                    failure.code
                ));
                Value::Null
            }
        };
        let revision_complete = !routes.is_null();
        let revision = observed_revision("linux", &services, &json!(interfaces), &routes);
        Ok(Observation {
            platform: "linux",
            services,
            interfaces: interface_projection(&interfaces),
            revision,
            revision_complete,
            limitations,
        })
    }
    fn apply_target(&self, _: &str, _: &ServiceView, _: &Value) -> Result<Applied> {
        Err(mutation_unsupported())
    }
}

// Routes remain private revision facts, never pretend to be interface addresses.
#[cfg(target_os = "linux")]
fn linux_routes() -> Result<Vec<String>> {
    use std::io::Read;
    let mut routes = Vec::new();
    for (path, header) in [("/proc/net/route", true), ("/proc/net/ipv6_route", false)] {
        let file = std::fs::File::open(path)
            .map_err(|_| err("network_routes_unavailable", "无法读取完整路由表"))?;
        let mut bytes = Vec::new();
        file.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
        routes.extend(normalize_routes(&bytes, header)?);
    }
    routes.sort();
    Ok(routes)
}

#[cfg(any(target_os = "linux", test))]
fn normalize_routes(bytes: &[u8], header: bool) -> Result<Vec<String>> {
    if bytes.len() > 1024 * 1024 {
        return Err(err("network_routes_oversized", "路由表超过有限观察预算"));
    }
    let text =
        std::str::from_utf8(bytes).map_err(|_| err("network_routes_invalid", "路由表编码无效"))?;
    let routes: Vec<_> = text
        .lines()
        .skip(usize::from(header))
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect();
    if routes.len() > 4096 {
        return Err(err("network_routes_oversized", "路由条目超过有限观察预算"));
    }
    Ok(routes)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[derive(Default, serde::Serialize)]
struct InterfaceFacts {
    flags: u32,
    ipv4: Vec<String>,
    ipv6: Vec<String>,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn interface_facts() -> Result<std::collections::BTreeMap<String, InterfaceFacts>> {
    let mut interfaces = std::collections::BTreeMap::<String, InterfaceFacts>::new();
    unsafe {
        let mut head: *mut libc::ifaddrs = std::ptr::null_mut();
        if libc::getifaddrs(&mut head) != 0 {
            return Err(err(
                "network_interfaces_unavailable",
                "无法读取接口状态；没有取得观察结果",
            ));
        }
        let result = (|| {
            let mut cursor = head;
            let mut entries = 0;
            while !cursor.is_null() {
                entries += 1;
                if entries > 4096 {
                    return Err(err("network_interfaces_oversized", "接口记录超过观察预算"));
                }
                let entry = &*cursor;
                if !entry.ifa_name.is_null() {
                    let name = std::ffi::CStr::from_ptr(entry.ifa_name)
                        .to_string_lossy()
                        .into_owned();
                    let facts = interfaces.entry(name).or_default();
                    facts.flags = entry.ifa_flags;
                    if let Some(address) = address_of(entry) {
                        match address {
                            std::net::IpAddr::V4(v4) => facts.ipv4.push(v4.to_string()),
                            std::net::IpAddr::V6(v6) => facts.ipv6.push(v6.to_string()),
                        }
                    }
                }
                cursor = entry.ifa_next;
            }
            Ok(())
        })();
        libc::freeifaddrs(head);
        result?;
    }
    for facts in interfaces.values_mut() {
        facts.ipv4.sort();
        facts.ipv4.dedup();
        facts.ipv6.sort();
        facts.ipv6.dedup();
    }
    Ok(interfaces)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn interface_projection(
    interfaces: &std::collections::BTreeMap<String, InterfaceFacts>,
) -> Vec<Value> {
    interfaces.iter().map(|(name, facts)| json!({
        "interface": name, "flags": facts.flags, "up": facts.flags & libc::IFF_UP as u32 != 0,
        "ipv4_addresses": facts.ipv4, "ipv6_addresses": facts.ipv6,
    })).collect()
}

#[cfg(any(target_os = "macos", target_os = "linux", test))]
fn observed_revision(
    platform: &str,
    services: &[ServiceView],
    interfaces: &Value,
    routes: &Value,
) -> String {
    digest(&serde_json::to_vec(&json!({
        "configured": revision_of(platform, services), "interfaces": interfaces, "routes": routes,
    })).expect("finite network facts are JSON"))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn address_of(entry: &libc::ifaddrs) -> Option<std::net::IpAddr> {
    if entry.ifa_addr.is_null() {
        return None;
    }
    let family = unsafe { (*entry.ifa_addr).sa_family } as i32;
    match family {
        libc::AF_INET => {
            let addr = unsafe { &*(entry.ifa_addr as *const libc::sockaddr_in) };
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::from(
                u32::from_be(addr.sin_addr.s_addr).to_be_bytes(),
            )))
        }
        libc::AF_INET6 => {
            let addr = unsafe { &*(entry.ifa_addr as *const libc::sockaddr_in6) };
            Some(std::net::IpAddr::V6(std::net::Ipv6Addr::from(
                addr.sin6_addr.s6_addr,
            )))
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// macOS: SystemConfiguration set/service/interface identity, the complete IPv6
// protocol configuration, and the dynamic-store observation. Raw FFI to the
// installed SDK frameworks; no shell, no sudo, no privileged helper.
// ---------------------------------------------------------------------------
#[cfg(target_os = "macos")]
#[allow(dead_code)] // Always compiled; only reached from the non-test host adapter.
mod macos {
    use super::*;
    use std::ffi::{c_void, CStr, CString};

    pub(super) static MACOS: MacHost = MacHost;
    pub(super) struct MacHost;

    // --- CoreFoundation / SystemConfiguration / Security FFI surface --------
    type CFTypeRef = *const c_void;
    type CFPropertyListRef = *const c_void;
    type CFStringRef = *const c_void;
    type CFArrayRef = *const c_void;
    type CFDictionaryRef = *const c_void;
    type CFMutableDictionaryRef = *mut c_void;
    type CFIndex = isize;
    type CFAllocatorRef = *const c_void;
    type Boolean = u8;
    type SCPreferencesRef = *mut c_void;
    type SCDynamicStoreRef = *mut c_void;
    type SCNetworkSetRef = *const c_void;
    type SCNetworkServiceRef = *const c_void;
    type SCNetworkInterfaceRef = *const c_void;
    type SCNetworkProtocolRef = *const c_void;
    type AuthorizationRef = *mut c_void;

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
        fn CFRelease(cf: CFTypeRef);
        fn CFGetTypeID(cf: CFTypeRef) -> usize;
        fn CFStringGetTypeID() -> usize;
        fn CFArrayGetTypeID() -> usize;
        fn CFDictionaryGetTypeID() -> usize;
        fn CFBooleanGetTypeID() -> usize;
        fn CFNumberGetTypeID() -> usize;
        fn CFArrayGetCount(array: CFArrayRef) -> CFIndex;
        fn CFArrayGetValueAtIndex(array: CFArrayRef, index: CFIndex) -> *const c_void;
        fn CFArrayCreateMutable(
            alloc: CFAllocatorRef,
            capacity: CFIndex,
            callbacks: *const c_void,
        ) -> *mut c_void;
        fn CFArrayAppendValue(array: *mut c_void, value: *const c_void);
        fn CFDictionaryGetCount(dict: CFDictionaryRef) -> CFIndex;
        fn CFDictionaryGetKeysAndValues(
            dict: CFDictionaryRef,
            keys: *mut *const c_void,
            values: *mut *const c_void,
        );
        fn CFDictionaryCreateMutable(
            alloc: CFAllocatorRef,
            capacity: CFIndex,
            key_callbacks: *const c_void,
            value_callbacks: *const c_void,
        ) -> CFMutableDictionaryRef;
        fn CFDictionarySetValue(
            dict: CFMutableDictionaryRef,
            key: *const c_void,
            value: *const c_void,
        );
        fn CFNumberCreate(alloc: CFAllocatorRef, the_type: i64, value: *const c_void) -> CFTypeRef;
        fn CFNumberGetType(number: CFTypeRef) -> i64;
        fn CFNumberGetValue(number: CFTypeRef, the_type: i64, out: *mut c_void) -> Boolean;
        fn CFStringCreateWithCString(
            alloc: CFAllocatorRef,
            cstr: *const i8,
            encoding: u32,
        ) -> CFStringRef;
        fn CFStringGetLength(string: CFStringRef) -> CFIndex;
        fn CFStringGetMaximumSizeForEncoding(length: CFIndex, encoding: u32) -> CFIndex;
        fn CFStringGetCString(
            string: CFStringRef,
            buffer: *mut i8,
            size: CFIndex,
            encoding: u32,
        ) -> Boolean;
        fn CFBooleanGetValue(boolean: CFTypeRef) -> Boolean;
        static kCFBooleanTrue: CFTypeRef;
        static kCFBooleanFalse: CFTypeRef;
        static kCFTypeDictionaryKeyCallBacks: c_void;
        static kCFTypeDictionaryValueCallBacks: c_void;
        static kCFTypeArrayCallBacks: c_void;
    }

    // CFNumber type codes (CFNumber.h).
    const KCF_NUMBER_SINT64: i64 = 4;
    const KCF_NUMBER_FLOAT32: i64 = 5;
    const KCF_NUMBER_FLOAT64: i64 = 6;
    const KCF_NUMBER_LONG: i64 = 10;
    const KCF_NUMBER_LONGLONG: i64 = 11;
    const KCF_NUMBER_FLOAT: i64 = 12;
    const KCF_NUMBER_DOUBLE: i64 = 13;
    const KCF_NUMBER_CFINDEX: i64 = 14;
    const KCF_NUMBER_CGFLOAT: i64 = 16;

    #[link(name = "SystemConfiguration", kind = "framework")]
    extern "C" {
        fn SCPreferencesCreate(
            alloc: CFAllocatorRef,
            name: CFStringRef,
            prefs_id: CFStringRef,
        ) -> SCPreferencesRef;
        fn SCPreferencesCreateWithAuthorization(
            alloc: CFAllocatorRef,
            name: CFStringRef,
            prefs_id: CFStringRef,
            authorization: AuthorizationRef,
        ) -> SCPreferencesRef;
        fn SCError() -> i32;
        fn SCPreferencesLock(prefs: SCPreferencesRef, wait: Boolean) -> Boolean;
        fn SCPreferencesUnlock(prefs: SCPreferencesRef) -> Boolean;
        fn SCPreferencesCommitChanges(prefs: SCPreferencesRef) -> Boolean;
        fn SCPreferencesApplyChanges(prefs: SCPreferencesRef) -> Boolean;
        fn SCNetworkSetCopyCurrent(prefs: SCPreferencesRef) -> SCNetworkSetRef;
        fn SCNetworkSetCopyServices(set: SCNetworkSetRef) -> CFArrayRef;
        fn SCNetworkSetGetSetID(set: SCNetworkSetRef) -> CFStringRef;
        fn SCNetworkServiceCopy(
            prefs: SCPreferencesRef,
            service_id: CFStringRef,
        ) -> SCNetworkServiceRef;
        fn SCNetworkServiceGetServiceID(service: SCNetworkServiceRef) -> CFStringRef;
        fn SCNetworkServiceGetName(service: SCNetworkServiceRef) -> CFStringRef;
        fn SCNetworkServiceGetEnabled(service: SCNetworkServiceRef) -> Boolean;
        fn SCNetworkServiceGetInterface(service: SCNetworkServiceRef) -> SCNetworkInterfaceRef;
        fn SCNetworkServiceCopyProtocol(
            service: SCNetworkServiceRef,
            protocol_type: CFStringRef,
        ) -> SCNetworkProtocolRef;
        fn SCNetworkInterfaceGetBSDName(interface: SCNetworkInterfaceRef) -> CFStringRef;
        fn SCNetworkProtocolGetEnabled(protocol: SCNetworkProtocolRef) -> Boolean;
        fn SCNetworkProtocolGetConfiguration(protocol: SCNetworkProtocolRef) -> CFDictionaryRef;
        fn SCNetworkProtocolSetEnabled(protocol: SCNetworkProtocolRef, enabled: Boolean)
            -> Boolean;
        fn SCNetworkProtocolSetConfiguration(
            protocol: SCNetworkProtocolRef,
            config: CFDictionaryRef,
        ) -> Boolean;
        fn SCDynamicStoreCreate(
            alloc: CFAllocatorRef,
            name: CFStringRef,
            callout: *const c_void,
            context: *const c_void,
        ) -> SCDynamicStoreRef;
        fn SCDynamicStoreCopyValue(store: SCDynamicStoreRef, key: CFStringRef)
            -> CFPropertyListRef;
        fn SCDynamicStoreCopyKeyList(store: SCDynamicStoreRef, pattern: CFStringRef) -> CFArrayRef;

        static kSCNetworkProtocolTypeIPv6: CFStringRef;
    }

    #[link(name = "Security", kind = "framework")]
    extern "C" {
        fn AuthorizationCreate(
            rights: *const c_void,
            environment: *const c_void,
            flags: u32,
            authorization: *mut AuthorizationRef,
        ) -> i32;
        fn AuthorizationFree(authorization: AuthorizationRef, flags: u32) -> i32;
    }

    const CF_STRING_ENCODING_UTF8: u32 = 0x08000100;
    // AuthorizationFlags (Security/Authorization.h).
    const AUTH_INTERACTION_ALLOWED: u32 = 1 << 0;
    const AUTH_EXTEND_RIGHTS: u32 = 1 << 1;
    const AUTH_PRE_AUTHORIZE: u32 = 1 << 4;
    const AUTH_DEFAULTS: u32 = 0;

    impl HostNetwork for MacHost {
        fn observe(&self) -> Result<Observation> {
            let session = Session::open()?;
            let mut services = session.services()?;
            // Attach live interface addresses, keyed by interface BSD name. The
            // static manual configuration is not the observed address.
            let observed = interface_facts()?;
            for service in &mut services {
                if let Some(facts) = observed.get(&service.interface) {
                    service.ipv4_addresses = facts.ipv4.clone();
                    service.ipv6_addresses = facts.ipv6.clone();
                }
            }
            let mut limitations = default_limitations("macos");
            let route_facts = dynamic_store_facts();
            if route_facts.is_null() {
                limitations
                    .push("无法读取动态存储状态；network_revision 不包含路由/VPN 观察。".into());
            }
            let revision = observed_revision("macos", &services, &json!(observed), &route_facts);
            Ok(Observation {
                platform: "macos",
                services,
                interfaces: interface_projection(&observed),
                revision,
                revision_complete: !route_facts.is_null(),
                limitations,
            })
        }

        fn apply_target(
            &self,
            service_id: &str,
            before: &ServiceView,
            after: &Value,
        ) -> Result<Applied> {
            let target = after
                .get("configuration")
                .ok_or_else(|| err("invalid_plan", "冻结目标缺少 configuration"))?;
            let enabled_value = after
                .get("enabled")
                .and_then(Value::as_bool)
                .ok_or_else(|| err("invalid_plan", "冻结目标缺少 enabled"))?;
            let authorization = Authorization::interactive()?;
            let mut session = Session::open_with_authorization(authorization.ptr())
                .map_err(|_| preparation_error(1001))?;
            // Non-blocking lock; busy is an explicit state, never sudo.
            if !session.lock(false) {
                return Err(preparation_error(unsafe { SCError() }));
            }
            // Re-read under the lock and compare the FULL frozen identity.
            let current_services = session.services().map_err(|_| preparation_error(1001))?;
            let current = current_services
                .iter()
                .find(|service| service.service_id == service_id)
                .ok_or_else(|| err("stale_network_plan", "服务身份在写入前变化；未修改网络设置"))?;
            if current.interface != before.interface
                || current.name != before.name
                || current.service_enabled != before.service_enabled
                || current.ipv6_protocol != before.ipv6_protocol
            {
                return Err(err(
                    "stale_network_plan",
                    "预览后服务、接口或完整 IPv6 配置变化；保留当前配置，未写入",
                ));
            }
            // Own the service reference; the array is released immediately.
            let (_, uuid) = service_id
                .split_once(':')
                .ok_or_else(|| err("invalid_service_id", "服务身份格式无效"))?;
            let uuid_ref = unsafe { cf_string(uuid) };
            let service = unsafe { SCNetworkServiceCopy(session.prefs, uuid_ref) };
            unsafe { CFRelease(uuid_ref) };
            if service.is_null() {
                return Err(err("stale_network_plan", "写入前服务消失；未修改网络设置"));
            }
            let protocol =
                unsafe { SCNetworkServiceCopyProtocol(service, kSCNetworkProtocolTypeIPv6) };
            unsafe { CFRelease(service) };
            if protocol.is_null() {
                return Err(err(
                    "network_ipv6_missing",
                    "该服务没有 IPv6 协议实例；未添加或删除协议",
                ));
            }
            let target_dict = match unsafe { json_to_cf_dictionary(target) } {
                Ok(dict) => dict,
                Err(failure) => {
                    unsafe { CFRelease(protocol) };
                    return Err(failure);
                }
            };
            let config_ok =
                unsafe { SCNetworkProtocolSetConfiguration(protocol, target_dict) != 0 };
            let enabled_ok =
                unsafe { SCNetworkProtocolSetEnabled(protocol, u8::from(enabled_value)) != 0 };
            unsafe { CFRelease(target_dict) };
            unsafe { CFRelease(protocol) };
            if !config_ok || !enabled_ok {
                return Err(err(
                    "network_write_failed",
                    "SystemConfiguration 未接受 IPv6 变更；未修改任何网络设置",
                ));
            }
            // Commit then apply are separate; a failure after commit is an
            // uncertainty that keeps the original job.
            if unsafe { SCPreferencesCommitChanges(session.prefs) } == 0 {
                return Err(err(
                    "network_commit_uncertain",
                    "SystemConfiguration commit 未确认；配置是否落地未知，查询原任务",
                ));
            }
            if unsafe { SCPreferencesApplyChanges(session.prefs) } == 0 {
                return Err(err(
                    "network_apply_uncertain",
                    "配置已提交但应用未确认；查询原任务，不重复写入",
                ));
            }
            // Release the lock before a fresh readback session.
            session.unlock();
            // Readback from a fresh SCPreferences session: a cache read is not
            // persisted proof.
            let fresh = Session::open()?;
            let readback = fresh.services()?;
            let observed = readback
                .iter()
                .find(|service| service.service_id == service_id)
                .cloned()
                .ok_or_else(|| {
                    err(
                        "network_readback_missing",
                        "读回时服务消失；请查询原任务核对",
                    )
                })?;
            let verified = observed.interface == before.interface
                && observed.service_enabled == before.service_enabled
                && observed.ipv6_protocol == *after;
            let mut observed_with_addresses = observed;
            if let Ok(interfaces) = interface_facts() {
                if let Some(facts) = interfaces.get(&observed_with_addresses.interface) {
                    observed_with_addresses.ipv4_addresses = facts.ipv4.clone();
                    observed_with_addresses.ipv6_addresses = facts.ipv6.clone();
                }
            }
            Ok(Applied {
                service: observed_with_addresses,
                configuration_verified: verified,
            })
        }
    }

    fn preparation_error(status: i32) -> crate::Error {
        // SystemConfiguration.h: access error=1003; preferences busy=3002.
        match status {
            1003 => err(
                "network_authorization_denied",
                "系统网络配置授权被拒绝；未修改网络设置",
            ),
            3002 => err(
                "network_locked",
                "另一个进程持有 SystemConfiguration 锁；未修改网络设置",
            ),
            _ => err(
                "network_prepare_failed",
                &format!("无法准备系统网络配置会话（系统错误 {status}）；未提交网络设置"),
            ),
        }
    }

    struct Authorization(AuthorizationRef);
    impl Authorization {
        fn interactive() -> Result<Self> {
            let flags = AUTH_INTERACTION_ALLOWED | AUTH_EXTEND_RIGHTS | AUTH_PRE_AUTHORIZE;
            let mut reference: AuthorizationRef = std::ptr::null_mut();
            let status = unsafe {
                AuthorizationCreate(std::ptr::null(), std::ptr::null(), flags, &mut reference)
            };
            if status != 0 || reference.is_null() {
                // -60006 canceled, -60007 interaction not allowed, etc.
                return Err(err(
                    "network_authorization_denied",
                    "系统网络配置授权未取得；未修改任何网络设置",
                ));
            }
            Ok(Authorization(reference))
        }
        fn ptr(&self) -> AuthorizationRef {
            self.0
        }
    }
    impl Drop for Authorization {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // Defaults only: never destroy rights acquired elsewhere.
                unsafe {
                    AuthorizationFree(self.0, AUTH_DEFAULTS);
                }
            }
        }
    }

    struct Session {
        prefs: SCPreferencesRef,
        locked: bool,
    }
    impl Session {
        fn open() -> Result<Self> {
            Self::create(std::ptr::null_mut())
        }
        fn open_with_authorization(authorization: AuthorizationRef) -> Result<Self> {
            Self::create(authorization)
        }
        fn create(authorization: AuthorizationRef) -> Result<Self> {
            let name = unsafe { cf_string("Lintel network observer") };
            let prefs = if authorization.is_null() {
                unsafe { SCPreferencesCreate(std::ptr::null(), name, std::ptr::null()) }
            } else {
                unsafe {
                    SCPreferencesCreateWithAuthorization(
                        std::ptr::null(),
                        name,
                        std::ptr::null(),
                        authorization,
                    )
                }
            };
            unsafe { CFRelease(name) };
            if prefs.is_null() {
                return Err(err(
                    "network_inspect_failed",
                    "无法打开 SystemConfiguration preferences 会话",
                ));
            }
            Ok(Session {
                prefs,
                locked: false,
            })
        }
        fn lock(&mut self, wait: bool) -> bool {
            if self.locked {
                return true;
            }
            let ok = unsafe { SCPreferencesLock(self.prefs, u8::from(wait)) != 0 };
            if ok {
                self.locked = true;
            }
            ok
        }
        fn unlock(&mut self) {
            if self.locked {
                unsafe { SCPreferencesUnlock(self.prefs) };
                self.locked = false;
            }
        }

        fn services(&self) -> Result<Vec<ServiceView>> {
            let set = unsafe { SCNetworkSetCopyCurrent(self.prefs) };
            if set.is_null() {
                return Err(err(
                    "network_inspect_failed",
                    "没有当前 SCNetworkSet；无法确定服务身份",
                ));
            }
            let set_id = unsafe { cf_take_string(SCNetworkSetGetSetID(set)) }.unwrap_or_default();
            if set_id.is_empty() {
                unsafe { CFRelease(set) };
                return Err(err(
                    "network_inspect_failed",
                    "无法确定当前网络 set identity",
                ));
            }
            let array = unsafe { SCNetworkSetCopyServices(set) };
            unsafe { CFRelease(set) };
            if array.is_null() {
                return Ok(vec![]);
            }
            let mut services = Vec::new();
            let count = unsafe { CFArrayGetCount(array) };
            if !(0..=512).contains(&count) {
                unsafe { CFRelease(array) };
                return Err(err(
                    "network_services_oversized",
                    "网络服务列表超过有限观察预算",
                ));
            }
            for index in 0..count {
                let borrowed =
                    unsafe { CFArrayGetValueAtIndex(array, index) } as SCNetworkServiceRef;
                if borrowed.is_null() {
                    continue;
                }
                // Copy into an owned reference for the duration of the view; the
                // array is released after the loop.
                let service = unsafe { CFRetain(borrowed) } as SCNetworkServiceRef;
                let view = self.service_view(service, &set_id);
                unsafe { CFRelease(service) };
                if let Some(view) = view {
                    services.push(view);
                }
            }
            unsafe { CFRelease(array) };
            services.sort_by(|a, b| a.service_id.cmp(&b.service_id));
            Ok(services)
        }

        fn service_view(&self, service: SCNetworkServiceRef, set_id: &str) -> Option<ServiceView> {
            let service_uuid = unsafe { cf_take_string(SCNetworkServiceGetServiceID(service)) }
                .unwrap_or_default();
            if service_uuid.is_empty() {
                return None;
            }
            let name = unsafe { cf_take_string(SCNetworkServiceGetName(service)) }
                .unwrap_or_else(|| service_uuid.clone());
            let service_enabled = unsafe { SCNetworkServiceGetEnabled(service) != 0 };
            let interface_ref = unsafe { SCNetworkServiceGetInterface(service) };
            let interface = if interface_ref.is_null() {
                String::new()
            } else {
                unsafe { cf_take_string(SCNetworkInterfaceGetBSDName(interface_ref)) }
                    .unwrap_or_default()
            };
            let protocol =
                unsafe { SCNetworkServiceCopyProtocol(service, kSCNetworkProtocolTypeIPv6) };
            let (mode, protocol_projection) = if protocol.is_null() {
                ("unknown".into(), Value::Null)
            } else {
                let proto_enabled = unsafe { SCNetworkProtocolGetEnabled(protocol) != 0 };
                let config = unsafe { SCNetworkProtocolGetConfiguration(protocol) };
                let configuration = if config.is_null() {
                    Value::Null
                } else {
                    // A configuration type this build cannot faithfully
                    // represent must not be silently dropped; fail closed to
                    // `null` so no plan can freeze a lossy configuration.
                    match unsafe { cf_to_json_strict(config) } {
                        Some(value) => value,
                        None => {
                            unsafe { CFRelease(protocol) };
                            return Some(ServiceView {
                                service_id: format!("{set_id}:{service_uuid}"),
                                name,
                                interface,
                                mode: "unknown".into(),
                                service_enabled,
                                ipv4_addresses: vec![],
                                ipv6_addresses: vec![],
                                ipv6_protocol: Value::Null,
                            });
                        }
                    }
                };
                let mode = if !proto_enabled {
                    "off".to_string()
                } else {
                    configuration
                        .get("ConfigMethod")
                        .and_then(Value::as_str)
                        .map(classify_method)
                        .unwrap_or("unknown")
                        .to_string()
                };
                unsafe { CFRelease(protocol) };
                (
                    mode,
                    json!({
                        "set_id": set_id,
                        "service_uuid": service_uuid,
                        "enabled": proto_enabled,
                        "configuration": configuration,
                    }),
                )
            };
            Some(ServiceView {
                service_id: format!("{set_id}:{service_uuid}"),
                name,
                interface,
                mode,
                service_enabled,
                ipv4_addresses: vec![],
                ipv6_addresses: vec![],
                ipv6_protocol: protocol_projection,
            })
        }
    }
    impl Drop for Session {
        fn drop(&mut self) {
            self.unlock();
            if !self.prefs.is_null() {
                unsafe { CFRelease(self.prefs) };
            }
        }
    }

    fn classify_method(method: &str) -> &'static str {
        match method {
            "Automatic" | "RouterAdvertisement" | "6to4" => "automatic",
            "LinkLocal" => "link_local",
            "Manual" => "manual",
            _ => "unknown",
        }
    }

    // --- CoreFoundation helpers --------------------------------------------
    // Each Create/Copy result has one owner, including partially built trees.
    struct OwnedCf(CFTypeRef);
    impl OwnedCf {
        fn into_raw(mut self) -> CFTypeRef {
            let raw = self.0;
            self.0 = std::ptr::null();
            raw
        }
    }
    impl Drop for OwnedCf {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { CFRelease(self.0) };
            }
        }
    }

    unsafe fn cf_string(value: &str) -> CFStringRef {
        let Ok(cstr) = CString::new(value) else {
            return std::ptr::null();
        };
        CFStringCreateWithCString(std::ptr::null(), cstr.as_ptr(), CF_STRING_ENCODING_UTF8)
    }

    unsafe fn cf_take_string(value: CFStringRef) -> Option<String> {
        if value.is_null() || CFGetTypeID(value) != CFStringGetTypeID() {
            return None;
        }
        let length = CFStringGetLength(value);
        if !(0..=32768).contains(&length) {
            return None;
        }
        let size = CFStringGetMaximumSizeForEncoding(length, CF_STRING_ENCODING_UTF8) + 1;
        if size <= 1 {
            return Some(String::new());
        }
        let mut buffer = vec![0i8; size as usize];
        if CFStringGetCString(value, buffer.as_mut_ptr(), size, CF_STRING_ENCODING_UTF8) == 0 {
            return None;
        }
        let text = CStr::from_ptr(buffer.as_ptr()).to_str().ok()?;
        // C-string extraction must not truncate an embedded NUL.
        (text.encode_utf16().count() == length as usize).then(|| text.to_owned())
    }

    unsafe fn cf_to_json_strict(value: CFTypeRef) -> Option<Value> {
        cf_to_json_inner(value, 0, &mut 4096)
    }

    unsafe fn cf_to_json_inner(
        value: CFTypeRef,
        depth: usize,
        remaining: &mut usize,
    ) -> Option<Value> {
        if value.is_null() || depth > 12 || *remaining == 0 {
            return None;
        }
        *remaining -= 1;
        let type_id = CFGetTypeID(value);
        if type_id == CFStringGetTypeID() {
            return cf_take_string(value).map(Value::String);
        }
        if type_id == CFBooleanGetTypeID() {
            return Some(json!(CFBooleanGetValue(value) != 0));
        }
        if type_id == CFNumberGetTypeID() {
            return cf_number_to_json(value);
        }
        if type_id == CFArrayGetTypeID() {
            let count = CFArrayGetCount(value);
            if count < 0 || count as usize > *remaining {
                return None;
            }
            let mut items = Vec::with_capacity(count as usize);
            for index in 0..count {
                items.push(cf_to_json_inner(
                    CFArrayGetValueAtIndex(value, index),
                    depth + 1,
                    remaining,
                )?);
            }
            return Some(Value::Array(items));
        }
        if type_id == CFDictionaryGetTypeID() {
            let count = CFDictionaryGetCount(value);
            if count < 0 || count as usize > 512 || count as usize > *remaining {
                return None;
            }
            let mut keys = vec![std::ptr::null(); count as usize];
            let mut values = vec![std::ptr::null(); count as usize];
            CFDictionaryGetKeysAndValues(value, keys.as_mut_ptr(), values.as_mut_ptr());
            let mut map = serde_json::Map::new();
            for index in 0..count as usize {
                let key = cf_to_json_inner(keys[index], depth + 1, remaining)?;
                let key = key.as_str()?.to_owned();
                let item = cf_to_json_inner(values[index], depth + 1, remaining)?;
                if map.insert(key, item).is_some() {
                    return None;
                }
            }
            return Some(Value::Object(map));
        }
        // Unsupported CF property types must never become a lossy null field.
        None
    }

    unsafe fn cf_number_to_json(value: CFTypeRef) -> Option<Value> {
        let subtype = CFNumberGetType(value);
        if matches!(
            subtype,
            KCF_NUMBER_FLOAT32
                | KCF_NUMBER_FLOAT64
                | KCF_NUMBER_FLOAT
                | KCF_NUMBER_DOUBLE
                | KCF_NUMBER_CGFLOAT
        ) {
            let mut out: f64 = 0.0;
            if CFNumberGetValue(
                value,
                KCF_NUMBER_FLOAT64,
                &mut out as *mut f64 as *mut c_void,
            ) == 0
            {
                return None;
            }
            serde_json::Number::from_f64(out).map(Value::Number)
        } else {
            let mut out: i64 = 0;
            (CFNumberGetValue(
                value,
                KCF_NUMBER_SINT64,
                &mut out as *mut i64 as *mut c_void,
            ) != 0)
                .then(|| json!(out))
        }
    }

    unsafe fn json_to_cf_dictionary(value: &Value) -> Result<CFMutableDictionaryRef> {
        if !value.is_object() {
            return Err(err("invalid_plan", "IPv6 配置必须是字典"));
        }
        let raw = json_to_cf_value(value)
            .ok_or_else(|| err("network_write_failed", "配置包含不可保真写回的值"))?;
        Ok(raw as CFMutableDictionaryRef)
    }

    unsafe fn json_to_cf_value(value: &Value) -> Option<CFTypeRef> {
        json_to_cf_inner(value, 0, &mut 4096)
    }

    unsafe fn json_to_cf_inner(
        value: &Value,
        depth: usize,
        remaining: &mut usize,
    ) -> Option<CFTypeRef> {
        if depth > 12 || *remaining == 0 {
            return None;
        }
        *remaining -= 1;
        match value {
            Value::String(text) => {
                if text.encode_utf16().count() > 32768 {
                    return None;
                }
                let cf = cf_string(text);
                (!cf.is_null()).then_some(cf)
            }
            Value::Bool(value) => Some(CFRetain(if *value {
                kCFBooleanTrue
            } else {
                kCFBooleanFalse
            })),
            Value::Number(number) => {
                let cf = if let Some(integer) = number.as_i64() {
                    CFNumberCreate(
                        std::ptr::null(),
                        KCF_NUMBER_SINT64,
                        &integer as *const i64 as *const c_void,
                    )
                } else if number.is_f64() {
                    let float = number.as_f64()?;
                    CFNumberCreate(
                        std::ptr::null(),
                        KCF_NUMBER_FLOAT64,
                        &float as *const f64 as *const c_void,
                    )
                } else {
                    return None;
                };
                (!cf.is_null()).then_some(cf)
            }
            Value::Array(items) => {
                if items.len() > *remaining {
                    return None;
                }
                let array = OwnedCf(CFArrayCreateMutable(
                    std::ptr::null(),
                    0,
                    &kCFTypeArrayCallBacks as *const c_void,
                ));
                if array.0.is_null() {
                    return None;
                }
                for item in items {
                    let cf_item = OwnedCf(json_to_cf_inner(item, depth + 1, remaining)?);
                    CFArrayAppendValue(array.0 as *mut c_void, cf_item.0);
                }
                Some(array.into_raw())
            }
            Value::Object(object) => {
                if object.len() > 512 || object.len() > *remaining {
                    return None;
                }
                let dict = OwnedCf(CFDictionaryCreateMutable(
                    std::ptr::null(),
                    0,
                    &kCFTypeDictionaryKeyCallBacks as *const c_void,
                    &kCFTypeDictionaryValueCallBacks as *const c_void,
                ));
                if dict.0.is_null() {
                    return None;
                }
                for (key, item) in object {
                    let cf_key = OwnedCf(json_to_cf_inner(&json!(key), depth + 1, remaining)?);
                    let cf_value = OwnedCf(json_to_cf_inner(item, depth + 1, remaining)?);
                    CFDictionarySetValue(dict.0 as *mut c_void, cf_key.0, cf_value.0);
                }
                Some(dict.into_raw())
            }
            Value::Null => None,
        }
    }

    // Dynamic values (not just key names) detect changed VPN/service routes.
    // Unsupported or oversized snapshots become an explicit inspect limitation.
    fn dynamic_store_facts() -> Value {
        unsafe { dynamic_store_snapshot().unwrap_or(Value::Null) }
    }

    unsafe fn dynamic_store_snapshot() -> Option<Value> {
        let name = OwnedCf(cf_string("Lintel network revision"));
        let store = OwnedCf(SCDynamicStoreCreate(
            std::ptr::null(),
            name.0,
            std::ptr::null(),
            std::ptr::null(),
        ));
        if store.0.is_null() {
            return None;
        }
        let mut keys = std::collections::BTreeSet::from([
            "State:/Network/Global/IPv4".to_string(),
            "State:/Network/Global/IPv6".to_string(),
            "State:/Network/Global/DNS".to_string(),
        ]);
        for pattern in [
            "State:/Network/Interface/.*/IPv4",
            "State:/Network/Interface/.*/IPv6",
            "State:/Network/Service/.*/IPv4",
            "State:/Network/Service/.*/IPv6",
            "State:/Network/Service/.*/DNS",
        ] {
            let pattern = OwnedCf(cf_string(pattern));
            let list = OwnedCf(SCDynamicStoreCopyKeyList(
                store.0 as SCDynamicStoreRef,
                pattern.0,
            ));
            if list.0.is_null() {
                return None;
            }
            let converted = cf_to_json_strict(list.0)?;
            for key in converted.as_array()? {
                keys.insert(key.as_str()?.to_owned());
            }
            if keys.len() > 512 {
                return None;
            }
        }
        let mut facts = Vec::new();
        let mut bytes = 0;
        for key in keys {
            let cf_key = OwnedCf(cf_string(&key));
            let value = OwnedCf(SCDynamicStoreCopyValue(
                store.0 as SCDynamicStoreRef,
                cf_key.0,
            ));
            // Absent keys participate as absence, not silent removal of a bad value.
            let value = if value.0.is_null() {
                Value::Null
            } else {
                cf_to_json_strict(value.0)?
            };
            let fact = json!({"key":key,"value":value});
            bytes += serde_json::to_vec(&fact).ok()?.len();
            if bytes > 1024 * 1024 {
                return None;
            }
            facts.push(fact);
        }
        Some(json!(facts))
    }

    #[cfg(test)]
    mod cf_tests {
        use super::*;
        #[test]
        fn access_denial_and_busy_remain_distinct_prewrite_results() {
            assert_eq!(preparation_error(1003).code, "network_authorization_denied");
            assert_eq!(preparation_error(3002).code, "network_locked");
            assert_eq!(preparation_error(1001).code, "network_prepare_failed");
        }
        #[test]
        fn full_manual_configuration_roundtrips_through_cf() {
            // Constructed property lists only; no SCPreferences or host access.
            let value = json!({"ConfigMethod":"Manual", "Addresses":["2001:db8::1"],
                "PrefixLength":[64], "Router":"fe80::1", "Vendor": {"nested":[true, false, 9007199254740993_i64, 1.25, "中文"]}});
            unsafe {
                let cf = OwnedCf(json_to_cf_dictionary(&value).unwrap());
                assert_eq!(cf_to_json_strict(cf.0), Some(value));
            }
        }
        #[test]
        fn unrepresentable_values_fail_closed_without_dropping_fields() {
            unsafe {
                for value in [
                    json!({"x":null}),
                    json!({"x":"a\u{0}b"}),
                    json!({"x":u64::MAX}),
                    json!({"x":[true, null]}),
                ] {
                    assert!(json_to_cf_dictionary(&value).is_err());
                }
                let nan = f64::NAN;
                let number = OwnedCf(CFNumberCreate(
                    std::ptr::null(),
                    KCF_NUMBER_FLOAT64,
                    &nan as *const f64 as *const c_void,
                ));
                assert_eq!(cf_to_json_strict(number.0), None);
            }
        }
        #[test]
        fn property_list_budget_rejects_deep_and_oversized_values() {
            let mut deep = json!(true);
            for _ in 0..14 {
                deep = json!([deep]);
            }
            unsafe {
                assert!(json_to_cf_value(&deep).is_none());
                assert!(json_to_cf_value(&json!(vec![true; 4097])).is_none());
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;

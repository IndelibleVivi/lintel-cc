# IPv4 / IPv6 and controlled network channels

[简体中文 (canonical)](network.md) · English · [Documentation](index.en.md)

`lintel-egress` is a Rust loopback HTTP proxy implementing TCP CONNECT, limited ordinary HTTP forwarding, per-instance fixed environment/hostname rules, and HTTP/HTTPS upstreams. It constrains only connections entering the proxy. `HTTP_PROXY`/`HTTPS_PROXY`, a running proxy or one observed proxy event cannot prove that a target process cannot connect directly.

## IPv4 / IPv6 Leaf: test host and channel separately

In the App, open Environment details → Egress and permissions; without a Claude environment, use the home action to view this host's network. CLI entries are `network inspect`, `network probe`, `network ipv6 plan` and `network restore plan`. All four share `crates/core/src/network.rs`, schemas from `crates/operations`, and finite HTTPS requests implemented solely in `crates/egress/src/probe.rs`. The full target is in the [Leaf (Chinese)](specs/ip-network.md).

`network_inspect` reads the selected host's services/interfaces and addresses offline. Its network revision includes configuration, interface flags, addresses and dynamic route/VPN values to validate a baseline. Incomplete observations are explicit limitations and are not reused. New interfaces without a configuration service, including VPN/TUN, remain separately visible. The UI refreshes metadata on entry, refocus and every 30 seconds while visible, without public requests. Configuration, observation, actual requests and enforcement are reported separately.

Only selecting Test actual egress sends the four requests: host-default IPv4/IPv6 and selected Lintel-channel IPv4/IPv6. The host-default path retains system VPN/TUN effects. With no channel, its two cells are `not_tested`. Echoes must be bounded IPs matching the family. Results include selected target, hostname actually observed by the executing process, time, elapsed duration and failure reason; IPs can be copied, unavailable hostnames are never invented. `peer_family` describes only the actual first TCP hop, not routing beyond an upstream. Defaults are `https://api.ipify.org` and `https://api6.ipify.org`; you may explicitly select your own HTTPS echo endpoints. Endpoints observe this request's public IP. Credentials/query/fragment, redirects, custom CAs and unbounded responses are refused. Each cell has a complete 1–15 second deadline, default 10, and separate header/body bounds. DNS, connect, CONNECT, TLS, HTTP and family failures never collapse into a security conclusion.

macOS targets the exact current SCNetworkSet/service UUID/interface, never a guessed display name such as Wi-Fi. Preview freezes full IPv6 protocol configuration, protocol enabled state, service enabled state, service identity and baseline. Off disables that protocol; link-local selects LinkLocal. After approval, only that service's IPv6 changes, preserving IPv4, DNS, system proxies and other services. Under system authorization it writes, commits/applies, independently reads back, then retests the frozen endpoints/paths. This is a **shared host setting**, affecting all applications using the service. It persists through Claude environment switches and App exit; IPv6-only/VPN connectivity may be lost. A valid baseline requires identical revision/endpoints/channel instance/deadline and age ≤ five minutes. An expired baseline or changed network before approval rejects a new job under that plan; preview again.

Full restoration requires separate preview/approval, including manual addresses, prefixes and router. Later external edits or changed services/interfaces refuse overwrite. If the App's original channel stops, a new restoration preview can explicitly mark that path untested. Original task IDs are durable; lost responses/interruption query only the original task, without rewriting or repeating probes. Intent or equal readable values alone do not prove completion. Unconfirmed readback remains uncertain and cannot authorize automatic restoration. Confirmed configuration followed by interrupted/incomplete post-tests yields `partially_completed` with its confirmed restoration entry retained. Bounded post-test failures report each cell's reason, separately from configuration success. They neither roll back confirmed configuration nor imply zero leakage.

Linux `network inspect/probe` runs on the selected runner; Mac channels never substitute for remote ones. Only remote loopback HTTP channels are accepted; `network serve` remains owned by an independent foreground process. Host-wide Linux IPv6 changes are unsupported, with no sudo or management-connection changes. Namespace/systemd workload enforcement remains a separate SPEC target. `LINTEL_TEST_HOME` and development fixtures may not touch real host networks. This Leaf's system-configuration acceptance uses only cfg(test) adapters and synthetic invoke. Real macOS authorization/write/restore, VPN changes, public echo interoperability and Linux native runtime remain unverified.

System name resolution has a process-wide owner with at most 32 active system resolver calls. Saturation returns a DNS failure without an expanding queue. After probe timeout or cancellation, a late result cannot start a connection, and synchronous callers do not wait for blocked DNS during Tokio runtime shutdown. An OS resolver call itself cannot be forcibly canceled and keeps its slot until it returns; this does not guarantee DNS availability.

## Starting and stopping

Start an independent instance per environment and point that environment's controlled launch entry at the returned loopback address. All configuration examples are synthetic:

```sh
lintel network serve --config crates/egress/examples/policy.synthetic.json
# Independent existing executable uses the same serve_config owner:
cargo run -p lintel-egress -- --config crates/egress/examples/policy.synthetic.json
```

Both CLIs share `serve_config` and explicitly emit NDJSON. The first `listening` event reports `owner=foreground_process`, PID, normalized `active_config` and actual listener, such as `127.0.0.1:49152`. The OS assigns port `0`. Only IPv4/IPv6 loopback bindings are accepted, with no public listener or automatic shell/browser/service/route/firewall changes. Unconfigured clients do not automatically use the proxy. Other processes on the machine can connect to the port: a separate port scopes configuration; **it does not authenticate a process**.

Ctrl-C stops CLI admission, then closes existing tunnels/HTTP connections. Reload rules by explicitly stopping the old instance, waiting for completion, then starting a new one; old CONNECT tunnels cannot survive and bypass new rules. On proxy stop or upstream failure, the proxy never bypasses the specified upstream. Whether clients fall back to direct connections requires separate verification.

GUI and CLI share Config/Proxy but the App process owns GUI channels. CLI neither reads nor stops App channels; they have no shared cross-process lifecycle.

GUI/runner integration uses the same crate:

```rust,ignore
let proxy = lintel_egress::Proxy::bind(config, observer).await?;
let address = proxy.local_addr()?;
// shutdown may be a oneshot receiver wrapped as Future<Output = ()>.
proxy.serve_until(shutdown).await?;
// Only after return may the caller report the instance and connections stopped.
```

`Observer = Arc<dyn Fn(Event) + Send + Sync>` receives the fixed metadata schema below. Callbacks should return promptly; GUI should keep only a bounded in-memory event list. The crate creates no log files by default. CLI writes metadata to stdout, which callers must continuously consume; after shell redirection, capacity/retention/deletion belong to the caller. `serve()` is the persistent form without a stop signal. For lifecycle control, use `serve_until` and await completion rather than dropping its future.

## Desktop native bridge

Tauri registers `network_request`, invoked as `invoke("network_request", { payload })`, with standard `{ "ok": true, "data": ... }` / `{ "ok": false, "error": { "code", "message" } }` envelopes. This is the native source contract; whether a UI exposes it depends on that UI's actual implementation.

| `payload.op` | Other fields | Behavior and return |
| --- | --- | --- |
| `start` | `environment_id`; optional object `config` | Core `inspect` checks the registered environment first, then binds its independent proxy. `config` uses Config below; native overrides environment ID and `127.0.0.1:0` bind, preventing public listeners. Existing instances return `channel_exists`; stop before changing rules. |
| `status` | `environment_id` | Returns whether the task is running, address, adopted `active_config` and at most 200 connection metadata events, in this process only, without log files. Ended tasks have `running: false`, `active_config: null`; old addresses/events never imply running. |
| `stop` | `environment_id` | Closes listener/connections and waits for proxy completion, then returns stopped and releases environment events. No instance returns the same stopped state. Does not kill Claude or change system proxies. |
| `launch` | `environment_id` | Requires a running channel. Sends core `launch` with its `proxy_url`, returning the core envelope unchanged. It is an explicit user launch action, never automatic on `start`/`status`. |

The new App's Open Claude through channel uses a separate launch preview: enter project cwd; `plan_launch` through the `request` bridge freezes root, cwd and current channel address; approval uses the same `launch_request`. At preview and first attempt, `NetworkState::dispatch_core` verifies that this address belongs to this environment's live App channel and serializes stop races. Existing original launch records are query-only even after channel stop; a merely `planned` preview cannot skip live-channel checks. Older `network_request.launch` remains for existing callers.

`start`/`status`/`stop` data always includes:

```json
{
  "running": false,
  "address": null,
  "active_config": null,
  "events": [],
  "coverage": "proxy_connections_only",
  "direct_connections_enforced": false
}
```

When running, `address` is the assigned `127.0.0.1:port`; `events` contains typed Events, and some operations add an explanatory `message`. Explicit stop sets address/config null and clears events. Restart may assign a different port; existing clients' proxy contexts do not update automatically, so restart clients explicitly. `launch` uses the core schema; macOS normally returns `status: "launch_requested"` and `message`, meaning Terminal accepted the request, not observed target traffic.

Running `active_config` is the complete validated, normalized Config passed to `Proxy::bind`: `environment_id`, `bind`, `default_action`, `allowed`, `blocked`, `upstream`, `address_family` and three resource limits. It is not a request echo. Hosts become lowercase without trailing dots; IPs are canonical, explicit rule ports retained. Upstream URLs are validated without string rewriting. Native sets target environment ID and `127.0.0.1:0`; the actual port is separate `address`. Config port 0 is not a connectable address. Invalid configuration creates neither channel nor `active_config`.

The desktop panel separates active readback from next-start drafts. Reopening queries the channel first. After stopping, editing can continue from read-back default action, blocked/allowed rules/ports and upstream; restarting preserves read-back resource limits. Drafts are per-environment Webview memory only. Target switching clears old displays; delayed replies cannot replace the current panel. Rule text is one host per line, optionally space plus comma-separated ports, e.g. `example.invalid 443,8443`; no ports means all. An abnormally ended task requires clearing the old instance before restart.

On macOS, core sets only a newly launched context's `HTTP_PROXY`, `HTTPS_PROXY`, `http_proxy`, `https_proxy` to that loopback URL. It changes no other environment, global shell or existing process and does not remove client handling of `NO_PROXY`/other routing. Synthetic tests do not run actual macOS Terminal/Claude routing and cannot establish direct-connection blocking. Other platforms returning `terminal_required` have not launched the target.

Low-volume native verification:

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml
CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml network::tests
```

Synthetic bridge tests check inspect/launch request shapes, distinguish `planned` from attempted records, and cover frozen-address changes, refusal of new attempts after stop and original-request query. Real loopback checks exercise denial/stop. They invoke no actual Claude, Terminal, system-proxy switches or public probes.

## Rules and supported scope

See [synthetic configuration](../crates/egress/examples/policy.synthetic.json). `environment_id` accepts short alphanumeric/`-`/`_` identifiers, never paths/accounts. Each `allowed`/`blocked` entry is `{ "host": "example.invalid", "ports": [443] }`. Domains are case-insensitive exact hostnames/IPs, without wildcards. Canonical IP comparison normalizes IPv4-mapped IPv6 such as `::ffff:127.0.0.1` to embedded IPv4, so a rule on either spelling covers both. inet_aton-style `127.1`, decimal integers and `0x` hexadecimal spellings are refused in rules and destinations, preventing exact-IP bypass. Empty `ports` means all ports for that host. Priority is explicit `blocked`, then `allowed`, then `default_action` (`allow`/`deny`). `default_action` is required in JSON; omission refuses startup rather than silently allowing everything. Rules run before each request/tunnel, and domain decisions do not inspect TLS URLs.

The product's default posture is `allow`, without an unverified built-in telemetry blacklist, but the wire configuration must still state `default_action`. `api.anthropic.com` is mixed-use and retained by default. CONNECT never classifies its encrypted content as telemetry. Explicitly blocking any host or selecting a deny allowlist blocks the whole host/port, potentially affecting necessary features. Reducing optional product egress also relies on supported native product settings.

| Capability | Current behavior |
| --- | --- |
| CONNECT | Bidirectional TCP bytes; no TLS decryption or root certificate installation |
| Ordinary HTTP | HTTP/1.1 absolute-form URL; converts to origin-form for origin, checks Host against URL authority |
| Request bodies | Single Content-Length, at most 64 MiB; bodies not saved |
| Keepalive/pipelining | One request per client connection, forced `Connection: close`; remaining requests not forwarded |
| chunked upload / Upgrade / Expect | Explicit 501; clients needing these use an appropriate CONNECT/TLS channel |
| HTTP response | Streamed unchanged; server must obey `Connection: close`, otherwise closed at lifetime limit |
| HTTP/HTTPS upstream | `http://proxy.example.invalid:8080` or `https://proxy.example.invalid:8443`; rustls verifies HTTPS certificate/hostname with Mozilla roots |
| Upstream authentication/custom CA | Unsupported; URL userinfo refused; client Proxy-Authorization not forwarded to origin |
| SOCKS/PAC | Unsupported; SOCKS URLs return explicit errors; require a separately verified conversion adapter |
| IPv6 | `::1` bind and IPv6 authorities supported; no control over other process IPv6 egress |
| UDP/QUIC/DNS/WebRTC/direct sockets | Outside proxy coverage, with no automatic tests or zero-leakage conclusions |

Connect establishment, including DNS/HTTPS-upstream handshake, and headers use `connect_timeout_seconds`, default 10. Whole connections default to 3600 seconds, explicitly configurable up to 86400; long tunnels close at expiry. Concurrency defaults to 64, configurable 1–1024, with admission backpressure at capacity. Headers are limited to 32 KiB/100 fields. No background public probes, STUN, analytics, license requests or online rules. Explicit IP echoes and approved automatic retests follow the finite target contract above.

Additive `address_family` defaults to `system` when absent. `ipv4_only` permits only IPv4 sockets Lintel creates toward destinations/upstreams, never tries DNS-returned IPv6, and never falls back to IPv6 or direct connections after upstream failure. IPv4-mapped IPv6 normalizes to IPv4. No usable IPv4 yields connection failure. It controls neither upstream onward connections, client bypass, DNS request family nor other processes. Event `peer_family` comes from the socket and is null before establishment.

## Observation semantics and privacy

Events contain only timestamp, environment ID, destination host/port, `http`/`connect` channel, allow/deny/reject, rule provenance, completion/failure reason, classification limits and small byte counts. They contain no URL paths/queries, Authorization, Cookies, request/response headers/bodies or raw errors. Hostnames may be sensitive; preview raw events before treating them as public support material.

`coverage` is always `proxy_connections_only`; `direct_connections_enforced` always `false`. `tunnel_content_unclassified` means CONNECT content is unexamined: encrypted HTTPS cannot be classified and other tunnel protocols are not inferred. `byte_counts_complete: false` means interrupted/error counters are incomplete; zero does not prove no transfer. `completed` means proxy forwarding finished, not business success. Events occur on close/interruption; a live tunnel may have no completion event yet.

## Verification and release limitations

```sh
cargo test -p lintel-egress
```

Real synthetic localhost sockets test CONNECT allow/deny, HTTP requests/bodies, upstream absolute-form HTTP/CONNECT, no direct fallback on upstream failure, pipeline containment, explicit-stop tunnel closure and absence of synthetic secrets in logs, plus config/framing refusals. No public proxy, real Claude data or administrator privilege is needed. HTTPS upstreams use maintained [tokio-rustls](https://docs.rs/tokio-rustls/latest/tokio_rustls/) and [webpki-roots](https://docs.rs/webpki-roots/latest/webpki_roots/); deployed-upstream interoperability has no acceptance record yet.

macOS enforcement requires signing, entitlements, user approval and real Network Extension runtime, absent from this crate. Linux namespace enforcement is unimplemented/unverified; see [Linux state (Chinese)](../platform/linux/README.md). No platform may display enforcement active on this evidence. Full acceptance still requires [network cases (Chinese)](ACCEPTANCE.md#d-网络与真实覆盖).

## Exact telemetry destinations

[Telemetry](telemetry.en.md) describes the static catalog, settings observations and controlled tests. Selection merges only into the next-start blocked draft, preserving upstream/family/other rules; mixed model/login/download hosts are not recommended for one-click blocking. Tests use only the active proxy's loopback request, never public telemetry endpoints, and remain distinct from actual Claude traffic. App updating is independent; active proxies block App install/restart. See [App updates](app-updates.en.md).

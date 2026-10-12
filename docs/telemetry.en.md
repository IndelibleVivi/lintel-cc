# Telemetry destination catalog and controlled rule tests

[简体中文 (canonical)](telemetry.md) · English · [Documentation](index.en.md)

This document owns the implementation boundaries of the telemetry destination catalog: static catalog, App draft merging, readonly catalog access through CLI/finite SSH, and controlled rule tests. The product position and existing [controlled channel](network.en.md) remain unchanged. There are no online rules, imported third-party historical lists, wildcard expansion or all-domain blocklists.

## Purpose and distinct facts

Which hosts carry optional telemetry, whether a client can actually reach them, whether they are explicitly blocked now, and native settings readback are four distinct facts:

- **Catalog (proposed/informational):** a static list from official documentation, with stable ID, purpose, client, version applicability honestly bounded by current documentation without invented minimum versions, official source URL, review date, collateral effects, exact host/port and `blockable` status.
- **configured:** after selecting a `blockable` destination in the blocked-channel draft and explicitly starting it, that host appears in the channel's `blocked` rules. This is a draft/configuration fact.
- **observed blocked:** a real loopback proxy returns `explicit_block` for an actual CONNECT request. Only this proves that this channel blocked it now.
- **not observed:** no connection, a stopped/replaced channel, or a rule matching only `default_deny` does not mean that this telemetry destination was blocked.

Native settings readback establishes only that a frozen value was written and read back. It is neither global enforcement nor browser/system blocking.

## Static catalog source and entries

The sole source is `contracts/telemetry-destinations.json` (`lintel.telemetry-destinations/1`), embedded at compile time by `crates/egress/src/telemetry.rs`. App, root CLI and runner CLI share this file.

Official documentation (`https://code.claude.com/docs/en/network-config`, reviewed 2026-10-09) lists two optional Datadog operational telemetry endpoints. These are currently the only `blockable` destinations:

| ID | host:port | Purpose and applicability |
| --- | --- | --- |
| `datadog_logs_intake` | `http-intake.logs.us5.datadoghq.com:443` | Claude Code operational telemetry (Datadog log intake). **Only** Claude Code using the **direct Anthropic API**; Bedrock, Google Agent Platform and Foundry deployments do not use this endpoint. |
| `datadog_browser_intake` | `browser-intake-us5-datadoghq.com:443` | Operational error reporting (Datadog browser intake), for Claude Code using the direct Anthropic API, additionally subject to Anthropic's server-side rollout gate. Native `DISABLE_ERROR_REPORTING` or `DISABLE_TELEMETRY` disables it. |

`provider` records the telemetry sink (Datadog) and model provider (direct Anthropic API) as different facts; the sink is not the model provider.

Mixed/necessary hosts below are **informational** and never offered as one-click telemetry blocks:

| host:port | Purpose |
| --- | --- |
| `api.anthropic.com:443` | Mixed model requests, safety prechecks, feature flags and telemetry |
| `claude.ai:443`, `claude.com:443`, `platform.claude.com:443` | Authentication or mixed use |
| `downloads.claude.ai:443` | Installer/updates, version checks, plugin executables |

Old Sentry/Statsig rules are unverified historical clues, **not** current `blockable` entries; they are absent from the catalog. The catalog accepts exact hostnames only, without wildcards, paths or schemes. Unknown IDs, unknown fields and wildcards are rejected.

Provider/applicability conditions may change with official documentation. Recheck the sources and review date above before changing entries.

## App: merging into the existing blocked-channel draft

The entry remains the controlled-channel panel in Environment details → Egress and permissions; there is no second proxy or executor.

`TelemetryPanel` displays the shared catalog. Selecting a `blockable` destination only merges its exact host into the `blocked` rules of the **next-start** draft:

- Existing `blocked`/`allowed` rules, default action, upstream, address family and three resource limits are preserved.
- A same-host all-port rule or a rule containing 443 covers the destination; port 80 alone does not cover 443.
- Informational/mixed hosts are never selected.
- Only the draft changes. Startup remains explicit; the active channel is never edited in place.

Drafts remain per-environment in Webview memory, without disk persistence, like the other connection details. Pending merge selections and handwritten rules are retained separately; deselecting does not delete handwritten rules. `start` merges exact host/443 and passes it to the native bridge. `active_config` readback proves adoption; real connection events prove blocking.

For an SSH environment, the catalog comes from the remote runner's readonly catalog request. Selection and controlled tests are unavailable. Older runners without the catalog show explicit failure; a local catalog never impersonates remote support. Remote UI must not claim that the App's local proxy is effective remotely: it covers only connections through that local channel.

### Observed privacy settings (readback)

`TelemetryPanel` accepts optional `settings?: Inspection['settings']` and `onPrivacySettings?: () => void`, supplied by `App.tsx`. With a real `inspect` readback:

- It separately shows the **observed value** of `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` and the official env-vars update impact: any nonempty value takes effect, also disabling automatic updates, release notes and feature-flag retrieval.
- Fine controls—product metrics, error reports, quality surveys, proactive feedback, cross-tool telemetry and feature flags—are listed separately.
- Without readback it shows `not_read`, never disabled/enforced. Readback ≠ global enforcement ≠ proof of current runtime.

`onPrivacySettings` only opens the existing privacy settings; it never automatically edits settings or an active proxy.

## CLI and finite SSH

The readonly static catalog initializes no private state and accesses no real host network:

```sh
lintel telemetry catalog          # same as lintel network catalog
lintel network catalog
```

Root CLI, runner CLI and the `telemetry_catalog` operation in `lintel request` return the same `lintel.telemetry-destinations/1`. Finite SSH treats it as a readonly request: no task record, resubmission or runner-binding fields, and no fields other than `command`.

### Controlled rule tests: `--test-telemetry`

```sh
lintel network serve --config policy.synthetic.json --test-telemetry datadog_logs_intake
```

`--test-telemetry ID[,ID...]` is a finite deduplicated list of known catalog IDs, at most 8. Before startup, each ID resolves to its exact host and is admitted only if that host is **already explicitly blocked**:

- Any target not matching `explicit_block`, including one matching only `default_deny`, refuses the whole startup with `telemetry_target_not_explicitly_blocked:<host>`; no silent success.
- Unknown/wildcard IDs produce `invalid_telemetry_test_ids` / `unknown_telemetry_destination`.
- Refusal occurs **before listener binding**; invalid tests open no listener and connect to no target.
- After admission, the foreground server serves after its `listening` event, then makes one real loopback CONNECT to **its own** listener, without fabricated events. A successful `rule_test` requires a real 403 matching the completed original event (`provenance:explicit_block`, `outcome:blocked`), and reports `owner_origin: lintel_egress_foreground`, `outcome.result: blocked_explicit`, `outcome.connection_attempted: false`. It **never connects to the target** or sends telemetry bodies.

The proxy's own `RuleTestRegistry` constrains tests. Registration binds a **private loopback client's local endpoint + exact host/443 + one-use test ID** to that proxy instance, with cumulative bounds: at most 16 tests, concurrency ≤ 2, five-second deadline each. Only a connection matching that private peer and exact target receives controlled-origin attribution; registration is consumed once. Cancellation/errors remove only their own registration. Arbitrary client headers or UI timing inference cannot create that provenance.

### Native controlled tests (App)

The App uses native `network_request` with `rule_test` on the same active channel:

```json
{"op":"rule_test","environment_id":"<ID>","telemetry_id":"datadog_logs_intake","channel_binding":"<CURRENT_CHANNEL_BINDING>"}
```

- Only catalog `telemetry_id` values are accepted, never arbitrary hosts/ports/wildcards; unknown IDs return `rule_test_failed`.
- No channel, a stopped channel, or a mismatched original `channel_binding` returns `channel_missing` / `channel_stopped` / `channel_changed`, never success.
- The serving channel's own `Proxy::rule_test` first checks `explicit_block` admission; failure makes no network request. It then binds the private loopback client, registers the directed test ID and sends a real `CONNECT host:443` to that channel's own listener. The response uses `owner_origin: lintel_app_proxy` and `outcome.test_id` for attribution. Ordinary Claude/proxy traffic never carries them; event `origin`/`test_id` are null.
- Success requires 403 and a completed event matching original test ID, target, `decision:deny`, `provenance:explicit_block` and `outcome:blocked`. Results are `blocked_explicit` or `failed`. If a request is mistakenly allowed, the server **fails closed**, refusing it without connecting and reporting `failed`; it never fabricates `blocked_explicit`. It resolves no public DNS, connects to no target and uses no upstream.
- Stopping/replacing the channel invalidates tests from the old instance; UI clears those results and requires retesting the new instance.

CLI and App share the catalog and controlled proof: both `serve_config_with_tests` and native `rule_test` make real loopback CONNECT requests through that proxy instance's `RuleTestRegistry`.

## Observation semantics

Real client connection events and rule tests remain distinct:

- Client event: `provenance: explicit_block` from `Config::decide`, `channel: connect`/`http`, `outcome: blocked`. This is observed blocked.
- Default-deny refusal: `provenance: default_deny`, **not** explicit-block evidence.
- Rule test: `kind: rule_test`, `provenance: rule_test`, `connection_attempted: false`. It proves that the frozen draft explicitly blocks the exact target, not that a client attempted it.

`coverage` is always `proxy_connections_only`; `direct_connections_enforced` is always `false`. Neither catalog nor tests provide system/browser/desktop enforcement or cover other processes' direct connections.

## Relationship to policy evaluation

`CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` in `crates/core/src/policy.rs` retains **nonempty** semantics: `0`/`false` also disable it. Official `https://code.claude.com/docs/en/env-vars` says it also disables automatic updates, release notes and feature-flag retrieval. This collateral effect is recorded in each relevant blocker's `collateral` field. It changes neither approved settings semantics nor Remote Control guarantees. The `reduce` preset still uses fine-grained `FLAGS`; this entry does not change that approach.

## Verification and limitations

```sh
python3 tests/verify.py --checks cargo-workspace-test,desktop-rust-test,journey-agent-adapters
# Build the frontend first, then run the independent synthetic invoke UI journey.
python3 tests/verify.py --checks desktop-build,telemetry-ui
```

Controlled tests use the **real existing loopback proxy**. An explicitly blocked catalog destination must return 403 with no target connection; an unblocked loopback origin remains reachable, proving exact scope. Tests never resolve/connect to public destinations or send telemetry bodies. An environment without loopback permission reports an exact `PermissionDenied`/`listen_failed` environment failure rather than passing.

Real host settings, macOS authorization, VPN transitions, Linux enforcement and public echo connectivity each need independent acceptance. Catalog content is rechecked when official documentation changes.

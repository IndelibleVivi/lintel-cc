# Lintel engineering contract

- Product: Lintel; repository name: lintel-cc. Target: macOS GUI and headless Linux runner.
- docs/SPEC.md preserves the complete product target; docs/current-state.md records delivered versus unverified capabilities. Do not silently narrow the target.
- crates/core owns plans, filesystem mutations, receipts and restoration. GUI and CLI call the same core; browser storage is changed only through native browser APIs.
- Test mutations only on synthetic temporary roots. Never modify real Claude credentials, settings, browser profiles, services or network policy during development.
- No account actions, production/remote deployment, paid services, public publishing, activation bypass or third-party code/assets without explicit current authorization.
- Do not read private main-session context or continuity. Temporary workers do not delegate, use Oracle, or perform Git/publishing/account actions. Multiple workers share this tree: preserve others' edits, one writer per owned path.
- UI data must be real or visibly synthetic. Configured, effective, observed and enforced are different facts. Unsupported operations return a specific limitation.
- No generic cleaner, fingerprint spoofing or account-unban claims. No default analytics, license server, TLS interception or full-environment export.
- Public docs use repo-relative paths and synthetic examples only. Private notes live outside Git.
- Relevant verification: `python3 tests/verify.py` is the canonical synthetic entrypoint; it includes the root workspace plus standalone desktop/native-host crates, JS/Python, desktop build and CLI journeys. Real browser startup and Linux runtime evidence remain independent. Update README/operator/current-state surfaces when their claims change.
- Desktop remote.rs owns the finite native SSH bridge; runner submit owns durable ACK and detached execution. Remote execute must use submit, and reconnect only queries the original job.
- Removing an SSH alias changes only the Lintel registry; retain durable task records and query-only recovery. Never erase deduplication state to retry a submission.
- Desktop resources.rs opens only named documentation resources in the system browser. Keep its fixed HTTPS targets in sync with Resources.tsx; do not expose arbitrary URL or shell execution.
- Browser clear is preparation only until a real runtime.onStartup generation and separately approved finishClear. Do not substitute extension worker restart or synthetic generation for browser restart evidence.

- `crates/core/src/policy.rs` owns versioned variable semantics and Remote Control conditions. Identify versions through static installed metadata; never execute Claude to inspect its version. Product/rule changes after approval invalidate a policy plan.
- `remote_install.rs` owns fixed Linux probe/upload/verify scripts. Only App-bundled static runners may be uploaded after exact preview approval. Persist install intent before upload; repeated installation is query-only. Pin new task runner digests; retained PATH compatibility serves older unbound task records and the independent Python CLI.
- Remote resource source: root `target/<linux-musl-target>/release/lintel`; `prepare-remote-runners.mjs` generates ignored `runner-bundles` binaries/manifest, packaged as `remote-runners`. Missing resources are an explicit limitation, not permission to download or compile on a VPS.

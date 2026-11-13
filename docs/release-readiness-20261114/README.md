# Release readiness — 2026-11-14

Status: independent audit and candidate hardening in progress. Baseline: `73f34ed06b5d630d2a36910baf88ff5713aede66`, after PR #7. Public distribution remains App 0.1.1 Preview and CLI 0.1.0; this work does not publish a Release, activate an updater, deploy the website, or use personal credentials.

## Acceptance and ownership

The target is a usable, reviewable release candidate, with independent decisions for macOS App, macOS CLI and Linux CLI. Full SPEC remains the long-term target; release gates cover the capabilities actually offered by the next Preview.

- Dependency security: exact locked versions, actual target/feature reachability, compatible security patches and caller-level regressions.
- Authentication and Keychain: exact source attribution, preservation, concurrent changes, independent verification, actual client compatibility evidence.
- Execution and recovery: approved identity, original job ownership, bounded reads, restart and incomplete history.
- Distribution: clean source, real candidate bytes, executable modes, internal checksums, notices, versions, native installation/upgrade evidence and trust.
- Product integration: six task paths, bilingual UI, error recovery, online/source distinction, cross-platform CI and final decision.

The coordinator maintains this entry; scoped investigations live beside it. Runtime evidence is generated outside Git, with source revision and platform recorded. Tracked reports contain only synthetic examples and public engineering facts.

## Route

1. Reconcile PR #4–#7, public assets, online site and exact CI scope.
2. Reproduce important failures and repair their actual consumers.
3. Run focused regressions, built-App journeys and strict canonical checks.
4. Build an unpublished `0.2.0` Preview candidate in disposable native CI; inspect real packages and test existing-state upgrade on each native platform where available.
5. Freeze the evidence matrix, bilingual release notes and operator commands; deliver a PR and an explicit release decision.

Only release timing/channel, Developer ID/notarization credentials, production updater trust and production website activation require owner decisions. None is performed by this task.

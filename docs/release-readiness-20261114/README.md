# Release readiness — 2026-11-14

**Decision: 0.2.0 Preview, with separate platform decisions.** Linux x86_64/aarch64 CLI: GO. macOS arm64 CLI: CONDITIONAL GO pending browser-download/trust acceptance. General-user macOS App: NO-GO; limited technical Preview remains conditional on the final native task/upgrade and graphic-rights requirements as well as an explicit distribution-trust choice.

Baseline after PR #7: `73f34ed06b5d630d2a36910baf88ff5713aede66`. Final product source: **`353536cc7dc3c8c63baa628955ee66332d95bcbb`**, tree `7c12bee277e84c85052d146738bbb395bd9b25b7`. Code and review: [PR #8](https://github.com/IndelibleVivi/lintel-cc/pull/8). Final gate: passed, including the mandatory macOS production verification of the actual DMG and the same-source full canonical CI. The public App 0.1.1 Preview, CLI 0.1.0 and deployed website remain unchanged. This work does not publish a Release, activate an updater, deploy the website or use personal credentials.

PR #8 is merged as `448d92bced6e87d34842bb5c54407e675e741653`, with the same full tree as the product source. The separate PR CI 43526971709 also passed all 14 jobs. These final evidence updates are documentation only; the candidate's source identity remains unchanged.

## Start here

- [Engineering review and release handoff](handoff.md): final decision, evidence matrix, exact candidate hashes, upgrade/rollback and executable remaining native/site procedures.
- [Baseline and evidence](baseline-and-evidence.md): actual PR #4–#7, earlier full CI, current public bytes and live site.
- [Authentication security](auth-security.md): real client mechanism evidence, default/explicit namespaces, secure storage selection, Keychain identity, Linux writers and final native results.
- [Execution and recovery](execution-recovery.md): original IDs, incomplete records, filesystem fault/substitution experiments, remote install admission and proxy-stop evidence.
- [Dependency security](dependency-security.md): exact locked graph, actual TLS reachability, compatible updates, independently reproduced xattr issue and remaining scoped alerts.
- [Product acceptance](product-acceptance.md): six task paths, bilingual layout/recovery, built-App and final native window boundaries.
- [Release engineering](release-engineering.md): real bytes, modes/checksums, resources, trust, native old-version upgrades and mandatory full-CI gate.
- [Chinese release notes](release-notes.zh-CN.md) / [English release notes](release-notes.en.md): reviewable, unpublished Preview drafts.

The complete raw evidence and unchanged final GitHub candidate ZIP accompany the owner handoff. Tracked reports contain synthetic examples and public engineering facts; no vendor credential database, personal state or signing secret is included. Every source, CI, native, package, installed-copy and deployed claim remains distinct. Full SPEC is the long-term product target; it does not turn future feature opportunities into indefinite release blockers.

The owner decisions are the intended macOS audience/signing identity, independent rights for the relevant third-party graphics (or an original replacement), and which platforms to publish and when. The remaining native and site acceptance programs are engineering tasks for a controlled environment, not a request that the owner manually reconstruct the audit.

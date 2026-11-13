# Credential provenance and verified cleanup

Status: source-complete, exact-source full CI verified and merged via
[PR #4](https://github.com/IndelibleVivi/lintel-cc/pull/4), 2026-11-13.
Source head: `9a7ae90eaad76c52e500f557b42da4e91aac607c`.
Main merge: `57b23c41a59d81952f676b24c2695c09cf1e880a`.
Baseline: `77be003`. No release, installation or deployment is implied.

## Outcome

Develop the existing local/SSH environment manager with a finite authentication
source inventory, exact selected-source cleanup through the existing
plan/approval/job engine, and an independent post-operation coverage report.
GUI and named/raw CLI consume the same core contracts. Development and all
mutation checks use synthetic homes and inert clients only.

## Decisions and scope

- Metadata discovery remains separate from explicit bounded source reading and
  explicit execution-context `auth status`. Inventory performs no history scan,
  helper execution, whole environment export or guessed Keychain operation.
  Optional status is disclosed separately because the client may use a helper,
  refresh state or contact the network.
- New cleanup plans freeze known credential fields and safe literal shell
  declarations. Mixed credential/configuration files retain MCP, plugins and
  unrelated fields. Shared shell selections are explicit and host-scoped.
- Official documentation warns that logout can clear saved MCP/plugin secrets.
  The automatic logout route therefore fails closed until a versioned adapter
  can establish the required preservation scope. Existing accepted original
  jobs remain query-only; old unexecuted destructive plans need a new preview.
- Work archive/new-root/import precede mutation; authentication cleanup creates
  no plaintext credential backup and no new mixed-state credential archive.
  Existing portable archives, recovery and original job identities remain.
- Receipts distinguish exact local removal, the current runner context, other
  contexts and server revocation. A residual/unknown source is never a global
  success. Rechecking does not rewrite the historical receipt.

## Work and ownership

1. Official mechanism evidence and paired public guidance: documentation lane.
2. Finite inventory, source selection/freeze and exact transformations:
   `auth_sources.rs` lane.
3. Plan/check/transaction/receipt, operations, runner CLI and integration
   verification: coordinator.
4. Existing Cleanup/Components/environment/result presentation and bilingual
   UI evidence: desktop lane.

## Acceptance evidence

Verified through the repository entrypoint: `cargo-workspace-test`,
`auth-sources-core-test` (39 selected tests), `journey-auth-sources`,
`journey-client-control`, `shared-remote-journey`, `journey-portable-work`,
`journey-cli`, `journey-submission`, `python-ssh-test` (17 tests), desktop
typecheck/build, both i18n checks and `auth-sources-render-test` (six SSR tests).
The i18n/SSR group contains 29 passing tests. The verification/CI planner
self-test also passed. Paired website projection has 21 routes, 16 guides and
42 sitemap URLs; its 14 language/route tests pass.

Behavioral evidence covers coexistence, default/custom roots, exact mixed-file
preservation, bounded/unknown/dynamic sources, inherited authentication after
cleanup, no credential body persistence, stale plans, writers before/after
publication, interrupted query-only recovery, occupied or replaced quarantine
objects, lost original locations and credentials recreated by status itself.
Review findings were fixed with regressions: no-replace original movement,
identity-bound staging cleanup, known-container coverage, salted settings
bindings, parent identity before readback, consistent status parsing and final
local readback after optional status. Synthetic boundaries are checked before
credential reads.

[Full CI 43386040362](https://github.com/IndelibleVivi/lintel-cc/actions/runs/43386040362)
passed all 14 jobs on the final source. All 12 downloaded canonical reports
bind clean merge checkout `3390d4e15a6f58a373f5a4b145a09532874a0478`;
all 119 selected check executions passed, and the production aggregate was
independently rerun. Both platforms passed the focused 39-test authentication
suite and all 19 browser checks. Local macOS checks also passed the complete
Cargo workspace, authentication CLI and built-App Chromium journeys.

The integration review found and repaired a missing-executable approval gap:
absence is now frozen alongside path and identity, both before acceptance and
when rechecking an original task. Two synthetic regressions failed before the
fix and passed after it; a new explicit observation without an original job ID
remains available. Bilingual guidance and its generated mirrors describe this
boundary.

Earlier runs exposed an outdated website payload count, the auth-only VM
fixture's temporary-root boundary, and an assumption that admission always
acknowledges within the production 40-second window. The final VM actually
returned `submission_uncertain`, observed preaccept `job_not_found`, then
obtained the original receipt through 24 queries after exactly one submission.
Real reboot reconciliation and service restoration also passed. Production
timeouts, safety checks and no-replay rules were not relaxed. These are
synthetic/disposable-host results, not native WebKit or real credentials.

## Continue here

PR integration status and current source/release boundaries live in
[current-state](../current-state.md); use that entry instead of rerunning
superseded intermediate CI investigations. The next product increment is an attributed, version-bound
Keychain/profile adapter only when it can preserve MCP/plugin credentials,
followed by separate native acceptance. Do not infer universal erasure or
server revocation from this local source work. No App release, updater
activation or website deployment is part of this change.

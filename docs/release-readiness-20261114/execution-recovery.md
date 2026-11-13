# Execution, recovery and local-state acceptance — 2026-11-14

Baseline: `73f34ed06b5d630d2a36910baf88ff5713aede66`. This is the execution/recovery lane of the release-readiness audit. The release decision and final integrated source/CI revision belong to the main handoff. All new experiments used disposable synthetic files, inert SSH adapters and the local Linux runtime. No account, personal Claude root, production SSH host or browser profile was modified.

## Findings and changes

### ER-01 — unreadable remote installation journals bypassed the unresolved-install gate

**Priority: repair before the next candidate. Fixed.**

The original `install_inventory()` in `crates/remote/src/remote_install.rs` used `if let Ok(record) = load(...)` and discarded errors. `require_reconciled_installs()` relied on that filtered list to detect `needs_reconciliation`. If an original installation journal became unreadable or malformed, the gate could treat it as absent and allow a new installation preview, and an already prepared second plan could proceed toward another upload. The original file was not deleted; the faulty inference occurred in the consumer of the inventory.

The production-path regression `unreadable_original_installation_blocks_new_intent_without_ssh` reproduced this on the baseline: after replacing one original record with a deliberately incomplete JSON record, `prepare_runner` returned a successful new preview. The failing assertion printed that actual returned plan. This was an inert SSH fixture, not a simulated standalone predicate.

The inventory now checks the record's filename identity, alias and recognized installation status, and retains an explicit finite error when a record cannot be used. New installation preview and approval inspect errors for their selected alias and fail with `install_record_invalid` before a new SSH probe, upload intent or binding switch. The original bytes stay untouched. Existing known-ID query paths keep their no-replay behavior.

The regression covers both a new `prepare_runner` and approval of a second already prepared plan. It verifies unchanged SSH command logs, unchanged original corrupt bytes, visibility of the incomplete inventory and retention of the healthy second record. It passes after the repair.

An unreadable original is still a repair case: this change does not invent its missing identity or silently erase it. The diagnostic directs the operator to retain and inspect that original installation ID; deleting deduplication evidence is not a recovery procedure.

### ER-02 — remote history silently hid unreadable original tasks and launches

**Priority: repair before the next candidate. Fixed in shared controller; App integration is owned by the main audit.**

`Controller::dispatch({op:"hosts"})` skipped invalid/unreadable task records, while installation and launch inventories similarly omitted them. A returning user could receive an apparently complete list with a missing unfinished operation. Core's local history already had explicit `record_errors`; the remote controller did not.

The `hosts` response now adds:

```json
{
  "records_complete": false,
  "record_errors": [
    {
      "kind": "task",
      "alias": "synthetic-host",
      "id": "00000000-0000-4000-8000-000000000003",
      "code": "local_record_invalid"
    }
  ]
}
```

`kind` distinguishes `task`, `installation` and `launch`. Only validated aliases and canonical task/launch UUIDs or generated installation-ID shapes are emitted. Unknown identifiers become `null`; arbitrary filenames and journal bodies are not copied into diagnostics. The standalone `launches` entry point includes the same completeness fields. Existing healthy arrays and exact original-ID lookup behavior remain available; this is an additive response change.

Regressions:

- `remote_inventory_reports_bad_originals_without_hiding_healthy_tasks`: one healthy original, one corrupted original and one secret-shaped invalid filename; incomplete list is explicit, healthy receipt remains, secret sentinel is absent from output, original corrupt bytes remain.
- `launch_inventory_exposes_unreadable_original_in_both_entrypoints`: both `hosts` and `launches` retain a healthy launch and identify the unreadable original without exposing its body.
- Existing empty-inventory assertion now includes `records_complete:true` and an empty error list; all pre-existing host-removal and no-transport assertions remain.

### ER-03 — finite local readers could wait indefinitely on a FIFO

**Priority: bounded failure hardening before the next candidate. Fixed.**

The remote controller's JSON `load()` and static SSH alias reader used blocking `open()` before checking file type. A FIFO substituted for a journal or `.ssh/config` therefore waited for a writer; byte limits did not impose a time limit. `storage::read()` checked pathname type first but then performed a blocking `open(O_NOFOLLOW)`, leaving a regular-file-to-FIFO replacement window. A shared Core call can hold the engine operation lock during that read.

The new readers open with `O_NONBLOCK`, then require that the **opened descriptor** is a regular file before reading. JSON records and Core reads retain `O_NOFOLLOW`; the SSH config reader deliberately keeps its existing support for ordinary symlinked SSH config while rejecting a descriptor that resolves to a pipe/device. Limits, finite error codes and no-shell alias parsing remain intact.

The baseline remote FIFO test failed after its two-second deadline; its rescue writer released the blocked reader so the test failed finitely. The repaired test covers both JSON records and SSH aliases with no writer. The Core regression checks the same descriptor boundary, exact read-size limit and existing symlink refusal. No production fault hook or timer was introduced. The Core precheck replacement window was identified by source analysis; the deterministic before/after failure was demonstrated on the remote production readers.

### ER-04 — TUI cleanup coverage still described all Keychain state as external

**Priority: small explanatory correction. Fixed.**

The cleanup TUI in `apps/runner/src/main.rs` still named Keychain alongside existing processes and server revocation as outside its selected cleanup, after Keychain removal had joined the shared plan. The line now describes selected local file and Keychain sources, and separately names unselected sources, running contexts and server revocation. It does not broaden the cleanup scope or alter default selections.

### ER-05 — a landed settings rename could report `uncertain_side_effects:false`

**Priority: correct recovery diagnostics before the next candidate. Fixed.**

`Engine::execute` derived this error flag from phase `verifying` or a running/executing step. A policy write has no such step and remains in phase `executing` until `atomic_recorded` returns. If rename succeeds but its parent-directory fsync fails, the file has changed while the durable journal still records `settings_write.state:"prepared"`. The existing job correctly became `needs_reconciliation` and would not replay, but the auxiliary boolean understated the unresolved publication uncertainty.

The new `settings_rename_with_failed_parent_sync_reports_uncertain_effects` test injects one `EIO` immediately after the actual settings rename and before the parent durability barrier. This extends the existing test-only storage fault machinery with an exact-path, thread-local one-shot fault, allowing receipt writes to proceed. The fault is absent from production. Before the fix, the test first confirmed that the actual settings value changed, then failed with `uncertain_side_effects` false. After the fix, a prepared/written settings journal is sufficient to retain uncertainty. The same regression independently queries the original ID, verifies recovery identifies the published object as written, repeats the original approval and verifies unchanged descriptor snapshot/bytes and the same receipt.

This is a diagnostic correction, not a new replay defect or a previously false `completed` result. The new condition is deliberately conservative for failures after the prepared intent but before publication can be proven.

### ER-06 — a substituted installation parent escaped the leaf directory guard

**Priority: reject malformed installation state before the next candidate. Fixed.**

An alias directory replaced by a file or symlink already failed `install_root → private_dir` before mutation, even though the inventory silently hid it. However, an existing alias directory could resolve through a symlink replacing its `installations` parent: `private_dir` examined only the existing leaf and did not walk its ancestors. The actual controller regression reproduced a successful new preparation through this substituted parent.

`install_root` now validates the direct `installations` parent before the selected alias. The inventory uses `symlink_metadata` for that root, distinguishes missing from malformed state, and reports valid alias names whose directory is now a file/link as `local_state_invalid`. A broken root link no longer appears as a complete empty installation list. Other unrelated non-alias files retain their previous ignored status.

`malformed_installation_directories_block_intent_and_report_incomplete_inventory` exercises five finite substitutions: live/broken parent symlinks, alias file and live/broken alias symlinks. For each, both a new prepare and approval of a previously saved plan fail before any SSH command; the original saved bytes remain; the inventory marks itself incomplete without printing filesystem targets. This is a targeted parent/leaf guard, not an OS-level atomic directory lock against a continuously racing external writer.

### ER-07 — partially valid installation JSON could still crash remote history

**Priority: keep returning-user recovery usable. Fixed.**

The independent UI/backend review caught a second layer after JSON decoding: a record with only matching `install_id`, `alias` and a recognized status passed inventory validation, but `RemotePanel` dereferenced `record.bundle.version` and other required fields. An incomplete original could therefore still break the panel rather than appear among unreadable records.

The finite `valid_install_record` check is shared by inventory and exact-ID query/install. It validates the existing mandatory presentation fields, frozen target/digests, string effect list and optional error-message shape. It does not require the later `previous_binding` field, so the existing legacy query-only compatibility stays intact. `partial_json_installations_are_errors_not_presentable_or_actionable_records` reproduces the old complete-true result for partial valid JSON, then covers nine missing/mistyped shapes. Each now produces `install_record_invalid`, retains the healthy neighboring installation, preserves original bytes and prevents new preparation, either approval path or a malformed exact-ID response before SSH. The complete remote suite, including legacy installation behavior, passes.

## Existing mechanisms that passed this review

These conclusions refer to specific execution paths and existing tests, not merely design documentation.

| Boundary | Source mechanism and useful evidence |
| --- | --- |
| Approval and stale plans | `Engine::execute` checks saved plan hash and exact approval, validates current root and settings snapshot, checks kind-specific frozen inputs, and rejects unsupported generic execute kinds before writing a job. Existing root/product/rule/source-change tests exercise these checks. |
| Durable acceptance | Core saves the accepted original journal and factual runner execution context before invoking the runner's acceptance hook. The runner ACK therefore follows the durable journal boundary. |
| No replay after interruption | An existing original job takes the query path before a new mutation. A busy Core lock permits atomic journal reads without falsely reclassifying a live worker; after the owner is gone, `job` reports `needs_reconciliation`. Existing replay/interruption tests remain intact. |
| Interrupted settings publication | `atomic_recorded` persists the staged object's snapshot before rename. `reconcile_settings_write` checks the original plan hash, root and current descriptor identity/bytes against that saved object. It distinguishes written, not written and unproven ownership; equal bytes from an unrelated later writer are not sufficient authority. `settings_tests` use a child process that exits at actual prepared/published boundaries. |
| Field restoration | `plan_restore` and config-edit restoration preserve later unrelated fields and reject changed selected ownership. Whole-file rollback is not used as a shortcut. |
| Work packages and partial publication | Archive/import publication uses native no-replace rename and a destination-parent durability barrier. The published name, intent and original job survive uncertainty; `publish_staged_new_reports_parent_sync_failure_without_claiming_success` passed in this lane. `save` and `load` share the 16 MiB persisted JSON admission limit. |
| Remote submission | Local original intent precedes the sole SSH submission; reconnect queries the original lookup ID and pinned runner. Alias removal preserves task records. The full shared-remote unit suite passed with existing ACK-loss, concurrent-submit, original-binding, receipt-mismatch, query-only and secret-on-stdin assertions. |
| Runner continuation claims | `submission.rs` transfers caller environment through a private bounded stdin frame; `supervisor.rs` selects a system/user manager only from existing eligibility and checks the actual cgroup before acceptance. It never enables linger or retries via a second launcher after uncertain manager launch. Setsid and reboot limitations are explicitly recorded. |
| Browser mutation and receipt recovery | `engine.js` durably records running before browser mutation and never replays running/uncertain/completed IDs. Restart converts unfinished running state to uncertainty. ACK stamping is serialized against the latest record; finishClear propagates the child outcome to the original operation. `background.js` binds receipts to the original native instance and does not send old operations under a replacement identity. |
| Browser host state | The native host reads bounded nonblocking regular files, validates state before mutation, and shares a 32 MiB read/write budget. A 1000-operation admission cap refuses new work while preserving original-ID queries. This protects deduplication but is a real long-term capacity limit, not journal retirement. |

## Verification in this lane

Commands were executed from the repository with the available Rust toolchain and locked offline dependency cache:

```sh
cargo test -p lintel-remote --locked --offline -- --test-threads=2
cargo test -p lintel-core storage:: --locked --offline -- --test-threads=2
cargo test -p lintel-core settings_tests:: --locked --offline -- --test-threads=2
```

Final focused result: **62 remote tests passed; 2 explicitly ignored integration entry points; 6 Core storage tests passed; 5 Core settings tests passed (including the child-process fixture entry).** The two remote ignores are the real Linux OpenSSH journey and the synthetic-current-HOME shared-controller journey, each intentionally invoked by its own canonical launcher. They are not counted as passed. Full workspace/canonical UI/CI integration and platform release evidence are recorded by the coordinating audit after its last combined edit.

Baseline failures were observed before the fixes for ER-01, the remote part of ER-03, ER-02's missing completeness signal, ER-05's false uncertainty flag, ER-06's substituted installation parent and ER-07's partial JSON admission. The additional launch test extends the same repaired inventory contract. No test count is treated as native macOS, installed-App or production-host evidence.

The new final-package CLI acceptance harness received an independent source review in this lane. Two evidence improvements were sent to the release engineering author and incorporated: the replay check now compares actual encrypted archive inventory/digests before and after the repeated original approval, and `upgrade_tested` explicitly distinguishes a distinct previous package from same-binary continuity. The harness's declared scope is native execution from the final extracted archive; it does not claim installer activation, OS distribution trust or backward writes by an old version. Its actual execution evidence belongs to the release engineering handoff.

## Remaining acceptance boundaries

- The Linux unit suite executes real local filesystem/pipe behavior with synthetic adapters. It does not replace hosted macOS, actual Security.framework, native WebKit, App upgrade or packaged-artifact acceptance.
- Browser deduplication records must not be deleted to work around the 1000-operation cap. A compatible retirement/export policy can follow this release; the current explicit refusal and original-ID queries should be described as the supported boundary.
- Noncooperating external writers can still change files between checks. Existing approved writers-stopped conditions and conflict/uncertainty outcomes remain necessary; this patch does not claim OS-level compare-and-swap.

The repaired execution paths are suitable for integration into the next candidate. These findings do not justify a new recovery framework, state migration or broad UI redesign.

# Exact systemd service pause and restoration

[简体中文 (canonical)](services.md) · English · [Documentation](index.en.md)

The Linux runner can preview and pause one systemd service explicitly bound to a registered configuration root, then restore its original startup state after separate approval. GUI remote service actions, CLI JSON and TUI all call `crates/core/src/service.rs`. Pausing does not log out accounts, clean credentials or replace a separately previewed cleanup.

## Supported targets

Requests explicitly provide environment ID, `manager: "user" | "system"` and full service unit name, such as `example.service`. `user` operates only the current UID's user manager; `system` mutations require the runner already executing as root. Lintel never invokes sudo, adjusts linger/PAM/login policy, or stops a manager, global supervisor or SSH.

Name schemas and named CLI/core share the finite `crates/operations` rule: full instantiated names such as `example@synthetic.service` are allowed; uninstantiated `example@.service`, paths, globs, newlines and commands are refused. Valid names must still meet the actual unit conditions below.

Supported targets are directly loaded persistent simple, exec or notify services, with unified cgroup v2, `KillMode=control-group`, and no cgroup delegation. The unit must directly and uniquely set `CLAUDE_CONFIG_DIR=<registered root>`, without unknown overrides through EnvironmentFile, PassEnvironment or UnsetEnvironment. For active services, MainPID and every cgroup process must belong to the runner user and `/proc` environment evidence must match the root; the root must also belong to that user. Configuration sources must be regular files owned by root/current user, not shared-writable, with no path symlinks.

Aliases, globs, bare service prefixes, uninstantiated templates, transient/generated/masked units, custom search directories, transitioning or failed states are unsupported. Stop propagation; other units' Requires/BindsTo/PartOf relationships to the target; custom stop/success/failure actions; host actions; and dependencies that may start/stop neighboring units are explicitly refused. Standard system/user slices and base-target dependencies may remain. An independent service with `Restart=always` or exact timer/socket triggers can be paused; trigger units themselves are not stopped.

CLI JSON, plans, receipts and cleanup inspections retain only root binding and necessary service state, never complete process environments, API keys or unit source text. Plans freeze unit/drop-in objects, byte digests, manager configuration and startup state to detect edits after preview.

The adapter checks `EnvironmentFiles`, `ExecStop` and `ExecStopPost` arrays through D-Bus queries with exact signatures. Only explicit empty arrays pass; missing properties, unknown formats and nonempty values refuse. systemd 255 `systemctl show --all` omits entire lines for such empty arrays, so missing lines are not emptiness evidence. `busctl get-property --json=short` returns the property itself in `data`; root environment, drop-ins, conditions and search directories use this structure.

Thirteen queried relationship properties have unordered-member semantics: `RequiredBy`, `BoundBy`, `ConsistsOf`, `PropagatesStopTo`, `TriggeredBy`, `OnFailure`, `OnSuccess`, `Requires`, `Requisite`, `BindsTo`, `Wants`, `Upholds`, `Conflicts`. At read time the adapter splits complete tokens on ASCII spaces in systemctl's printed format and sorts before freezing. It interprets neither quotes nor escapes and does not deduplicate. Valid unit IDs have no literal whitespace; encoded names remain one complete token. Reordering alone after reload is not an edit; membership changes still refuse. Other properties, source lists, DropInPaths and Conditions preserve order; source objects/bytes are compared exactly. Existing `stale_service_plan` checks require a new preview when an old snapshot differs from the current canonical form; old plans/approvals are never rewritten.

## Operations and approval

Register the actual configuration root first. Replace the UUID below with its registered environment ID; operators explicitly select all unit names and state directories.

```json
{"command":"service_inspect","environment_id":"<registered UUID>","manager":"user","unit":"example.service"}
```

Readonly `service_inspect` shows root binding, active/sub state, enablement, Restart, MainPID, cgroup, triggers, and whether the original pause job is actually loaded by the current manager. `quiesced: true` requires the exact blocker loaded, no permit, inactive service, MainPID zero and empty cgroup. An old completed receipt proves only the result at that time.

```json
{"command":"plan_service_quiesce","environment_id":"<registered UUID>","manager":"user","unit":"example.service"}
```

Preview contains manager, unit, root, original startup state, full blocker path and scope. `execute` requires exact plan `id` and `hash`. Remote mutation still uses durable runner `submit`; reconnect queries the original job rather than replacing an uncertain job with a new plan. TUI `s` opens inspect/quiesce/resume and requires typing `apply` to approve the specific plan.

Core persists the job and blocker-write intent before creating a unique file in the unit's default persistent configuration directory:

- User manager: `.config/systemd/user/<unit>.d/90-lintel-<plan UUID>.conf` in the current Engine home.
- System manager: `/etc/systemd/system/<unit>.d/90-lintel-<plan UUID>.conf`.

The blocker adds a non-trigger `ConditionPathExists` pointing to this job's unique `.service-resume-permit` in state, which is never created. Without the permit every startup source skips the target; temporarily unavailable/late-mounted state still blocks startup. Core reloads the manager, verifies the exact loaded condition through structured D-Bus `Conditions`, stops only the approved unit, and verifies an empty cgroup. Enablement, original unit, other drop-ins and neighbors are unchanged.

ConditionPathExists uses a raw absolute path without JSON/shell quotes; `%` becomes `%%` to prevent specifier expansion. State paths must be UTF-8 without control characters; inexpressible paths refuse during preview. Execution also compares the frozen blocker with current generation rules. Older pause plans storing different semantics return `stale_service_plan` and require preview again. File existence or successful daemon-reload cannot replace exact loaded-condition evidence.

Pause persists until independently approved restoration. Window closure, SSH logout and client reopening do not restore automatically; the disk blocker can survive reboot. User-manager lifetime after logout, runner-job completion and correct loaded state after reboot need independent real evidence, never inference from configuration/old receipts.

## Restoration and conflicts

```json
{"command":"plan_service_resume","job_id":"<original quiesce job UUID>"}
```

The original durable plan/job authorizes restoration, not a new registry. Preview rechecks root object, unit sources/drop-ins, exact blocker, permit and current processes. Edited/replaced/rebound configuration, a removed/changed blocker or overridden conditions yield a specific conflict/limitation and preserve outside edits. Inspect sources and preview again; restoration never rewrites the whole unit.

Approval removes only the original job's still-owned blocker. Existing quarantine binds deletion to the frozen file object; a replacement appearing during deletion is preserved and the recovery directory recorded. Core reloads and verifies configuration. Only originally active services start; originally inactive services stay inactive. If interrupted stop left the same originally running binding active, independent restoration removes the blocker without start/stop. Restoration has its own durable job; repeated execute never repeats start.

Interruption/command failure becomes `needs_reconciliation`. Repeated execute returns the original job and never automatically resends stop, cleanup or start. Unloaded/incomplete blockers, missing state or unexplained current configuration may prevent a restore plan. Retain state and receipts and inspect the exact unit/manager; never delete journal/deduplication to retry.

If the exact blocker is loaded after reload but configuration excluding the owned hold differs from preview, receipt `service.configuration_conflict` retains the differing keys actually compared. Fixed safety properties include `expected/current`; environment assignments, User, condition arguments and source paths expose keys only, without values. Sources report frozen-list index and digest/device/inode/mode/owner, never paths/text. Long values and more than 32 file differences are explicitly truncated. Diagnostics use normalized relationships and other frozen configuration. Unexplained differences still refuse before stop and retain blocker/journal for outside-edit inspection.

Cleanup still requires confirmation that non-systemd writers stopped. It scans accessible managers' loaded/installed direct root bindings and rechecks original hold evidence. A service bound to the root without an approved hold prevents cleanup plans even with `writers_confirmed_stopped: true`. Linux does not treat Claude processes explicitly belonging to another root as target writers. Unknown supervisors, wrappers, IDEs, manual processes and script/EnvironmentFile-hidden bindings require independent inspection; the adapter provides no whole-host writer discovery.

## Independent acceptance

Default synthetic verification and real systemd acceptance are separate:

```sh
cargo test -p lintel-core service::tests::
cargo build -p lintel-runner
python3 tests/service_journey.py target/debug/lintel
```

macOS checks schema, approval, journal, conflicts and synthetic transitions, without Linux systemd/cgroup runtime evidence. An explicitly selected disposable Linux test system needs real systemd, cgroup v2, Python 3 and current root privileges. The script creates only unique inert fixtures, never real Claude, accounts, personal homes or network services.

```sh
python3 tests/service_systemd_journey.py --runner /absolute/test-runner/lintel --json /var/tmp/service-report.json
```

Full mode checks ordinary local `/etc/systemd/system` units, Restart=always, actual timer/manual activation, exact-root/alias refusal, a continuously writing neighbor with unchanged InvocationID, durable query/replay, outside-edit conflicts, independent restoration and originally inactive state. Completed checks are saved immediately. After timer/manual blocking succeeds, the script stops its own 200 ms trigger timer while preserving enablement/source, waits for inactive/dead, MainPID zero and no Job, then separately checks external-edit conflict. During restoration it also stops that timer after observing held manual activation, verifies stable state, then requests product inspect/resume. Reports retain activation/timer phases, bounded state and exact errors before/after outside-edit requests. Pending/failed refusals never become outside-edit conflicts; no reset-failed or repeated Lintel mutation. Cleanup removes only owned fixtures.

The first stable `service_inspect` after pause likewise follows live timer/manual proof and owned-timer settlement, avoiding competition with continuously queued skipped starts. Pause receipts are saved before inspect, preserving original completion and later failure if inspect fails. Production pending/failed states still refuse and never trigger mutation replay.

To retain fixtures for real reboot:

```sh
python3 tests/service_systemd_journey.py --runner /absolute/test-runner/lintel --json /var/tmp/service-prepare.json --phase prepare
# Reboot the explicitly authorized disposable Linux test system separately.
python3 tests/service_systemd_journey.py --runner /absolute/test-runner/lintel --json /var/tmp/service-recover.json --phase recover --previous-report /var/tmp/service-prepare.json
```

Prepare saves synthetic fixture identity, home/state/root, unit, original quiesce job, preboot inspect, hold path and boot ID. Before first inspect it records finite raw show/JSON for fixture `EnvironmentFiles`, `ExecStop`, `ExecStopPost`, including failures in `--json`. Incomplete execute retains original `failed_service_receipt` and bounded configuration diagnostics; reports also include owned hold text, loaded Conditions/DropInPaths, service state and the unit's latest 30 capacity-bounded journal entries, before recovery/fixture cleanup. The retained timer is stopped but enabled; after real reboot it must become active again. Recover queries the same job, checks no target rewrites through reboot, actual loaded blocker and running neighbor, then separately approves restoration. `persistent_hold_survived_real_reboot` requires an actual boot-ID change. The script never reboots itself or changes login policy; this does not accept full G03 SSH/PAM/logout/job survival.

Clean `8c9d85c` [CI37177180828](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37177180828) ran full mode and prepare → real reboot → recover on disposable Ubuntu 24.04.4/systemd 255 root system manager. While paused, the target was inactive/dead, MainPID zero and cgroup empty; timer/manual attempts could not rewrite, while the neighbor kept running without restart. Outside edits returned `service_restore_conflict` and retained the blocker. Independent restoration, original inactive state and no restart on repeat passed. Changed boot ID, loaded hold after reboot and same quiesce job were checked. These results do not accept user-manager lifecycle, other supervisors or production VPS.

Mechanism references: [unit conditions](https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html), [service Restart](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html), [systemctl stop/mask](https://www.freedesktop.org/software/systemd/man/latest/systemctl.html), [D-Bus Conditions/dependencies](https://www.freedesktop.org/software/systemd/man/latest/org.freedesktop.systemd1.html). Empty-array/JSON behavior follows v255 [systemctl printing](https://github.com/systemd/systemd/blob/v255/src/systemctl/systemctl-show.c#L1168-L1309) and [busctl get-property](https://github.com/systemd/systemd/blob/v255/src/busctl/busctl.c#L1983-L2019). Condition path syntax follows [condition parser](https://github.com/systemd/systemd/blob/v255/src/core/load-fragment.c#L2795-L2844) and [specifier escaping](https://github.com/systemd/systemd/blob/v255/src/shared/specifier.c#L49-L87). Unordered relationships/format follow [dependency getter](https://github.com/systemd/systemd/blob/v255/src/core/dbus-unit.c#L133-L167), [getter mappings](https://github.com/systemd/systemd/blob/v255/src/core/dbus-unit.c#L790-L815), [string-array printing](https://github.com/systemd/systemd/blob/v255/src/shared/bus-print-properties.c#L217-L250) and [unit-ID character rules](https://github.com/systemd/systemd/blob/v255/src/basic/unit-name.c#L21-L81). Runtime masks' limits on higher-priority local units are why this uses an independent persistent drop-in.

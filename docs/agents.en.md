# Lintel Agent CLI

[简体中文 (canonical)](agents.md) · English · [Documentation](index.en.md)

For coding agents/script authors with existing operation authorization. Lintel 0.1.0 Preview is a source development preview, protocol 1/catalog 1, without signed formal distribution/public Release downloads. CLI is GUI-independent; configuration/plans/execution/archive/restoration share core. Self-description is the current binary's contract; inspect target applicability rather than infer permission from implementation.

Start with [English quickstart](quickstart.en.md) / [中文 quickstart](quickstart.md) for disposable synthetic preservation/original-query/independent package reading, then prepare real operations here.

## From App to terminal or Agent

Terminal and Agent accepts an explicitly chosen full CLI path, checking native platform/architecture, protocol, finite static interfaces and HOME/UID/state. Without CLI, build below or obtain a verified platform candidate. No formal download link; website/App main-doc links mean latest development documentation.

On macOS, reading a CLI from a protected location such as a removable volume may first wait for system file-access authorization. The CLI subprocess deadline does not include that OS wait. If checking remains pending, inspect the native permission prompt. Alternatively, copy the verified candidate, preserving its directory structure, into a new local version/test directory with existing access and select its `bin/lintel` again. System permission remains the user’s decision.

App never downloads silently, changes PATH or copies journals. Missing sidecar means unknown identity; byte agreement authenticates neither signatures nor source. Wrong architecture/protocol/digest/static-probe failure shows specific errors.

Handoff must use original task state. Accepted/uncertain jobs query the original ID; launches use readonly original-request query. Different CLI does not mean different state or rebind remote runner. Handoff text confers no new authorization.

<a id="install-candidate"></a>
## Installing a candidate package

Obtain platform archive/index from a trusted provider, verify identity/platform, unpack with system tools into a new version. Identity below is synthetic; replace with actual index. Create only parent first; ordinary mkdir fails if the **new identity directory** exists, preventing overwrite:

```sh
parent="${LINTEL_CLI_ROOT:-$HOME/.local/share/lintel-cli}"
mkdir -p "$parent"
# Replace with actual index identity: <version>-candidate-<first 12 revision characters>
identity="0.1.0-candidate-a239123903b6"
dest="$parent/$identity"
mkdir "$dest" && tar -xzf "candidate-packages/lintel-cli-$identity-macos-arm64.tar.gz" -C "$dest"
```

Verify installed bytes from selected directory; **targets need no Node**. System shasum/sha256sum checks SHA256SUMS covering payload/candidate.json/README.txt, excluding itself. This proves bytes, not signature/trusted provenance; then run absolute executable:

```sh
cd "$dest"
shasum -a 256 -c SHA256SUMS      # macOS
# On Linux use: sha256sum -c SHA256SUMS
"$dest/bin/lintel" version --json
"$dest/bin/lintel" capabilities --json
```

Upgrade into **another** identity directory, repeat checks/version/capabilities, explicitly switch caller path. No old executable replacement or PATH/rc/App/settings/service/Claude/credentials/browser-registration changes. State/IDs remain. Candidates are unsigned; Linux archives checked on Mac have format/byte evidence only, no Linux runtime. Execute locally only on matching architecture.

Uninstall after ceasing use/checking no executing tasks, deleting only your unpacked version directory. Retain state/packages/IDs. Removing executable neither restores approved config nor removes independent hosts/runners. Their restoration/management remains separate; never delete state instead.

<a id="install-version"></a>
## Independent source installation

From reviewed checkout, install into an explicitly chosen user directory with Rust stable/native toolchain, without Node/Tauri/GUI/App:

```sh
cargo install --path apps/runner --locked --root "$HOME/.local/share/lintel-cli/chosen-candidate"
"$HOME/.local/share/lintel-cli/chosen-candidate/bin/lintel" version --json
"$HOME/.local/share/lintel-cli/chosen-candidate/bin/lintel" capabilities --json
```

Prefer absolute paths. A shorthand lintel requires explicitly adding this bin to your PATH. No automatic rc/services/Claude/profiles/App edits. Linux compiles no Tauri/browser; macOS includes finite host-control library.

Upgrade reviewed source into **another version directory**, check version/capabilities/schema, then select new executable. Equal 0.1.0 does not identify equal builds; catalog owns support. Old tasks query original state/ID/pinned runner despite upgrades.

## Discover interfaces, then targets

```sh
lintel --help
lintel version --json
lintel capabilities --json
lintel context --json
lintel tasks --json
lintel work session read --help
lintel launch query --help
lintel describe plan_policy --json
lintel schema plan_policy
lintel env list --json
lintel env inspect <environment-id> --json
lintel env components <environment-id> --project-cwd /synthetic/project --json
lintel work preflight --environment <environment-id> --categories instructions,memory,sessions --json
lintel work inventory --environment <environment-id> --categories memory,sessions --json
lintel work archive plan --environment <environment-id> --categories memory,sessions --path projects/synthetic/memory/MEMORY.md --path projects/synthetic/session.jsonl --json
lintel capabilities --environment <environment-id>
```

Static help/version/capabilities/describe/schema/tasks/context never runs Claude/discovers personal roots/initializes state. describe returns stable operation ID, finite transports, schema, platform/target conditions, target/state effects, approval, secrets, results/recovery. Target capabilities adds real inspection; service unit/manager still service_inspect, browser still profile inspection.

Units require exact .service, e.g. claude.service/claude@synthetic.service, never uninstantiated template/path/glob/command. Shared names validate before target-state initialization; core still verifies actual ownership/binding/manager.

inspect_components returns finite exact-root sources/original records, optional target-host cwd, without Claude/credential bodies/job reconciliation. It checks ≤8 recorded units, not global supervisors; ≤50 records with scan/display gaps separate. Historical coverage is not live status. Browser ownership is independent of root.

work_preflight metadata reports count/bytes/limits/blocking relative paths/completeness. eligible means current admission only, no freeze/approval. Links/restricted/budget gaps remain unknown/incomplete; truncated blockers never pass. Actual archive/preserve fully reads/digests/rechecks. Streaming preserves sessions within runner limits; larger refusals remain. Resume's ≤8 MiB assessment is independent of archives/read pages.

work_inventory returns exact root/files path/category/bytes/total/complete/digest/next_offset. Continue with offset and same expected_digest; stale_inventory prevents mixed snapshots. Digest binds metadata only, never content/approval.

Repeatable --path on preflight/archive plan/preserve plan is additive selected_paths. Omission retains categories; provided arrays are nonempty/unique/canonical relative/category-matching, no dot/parent/absolute/control/fuzzy paths. ≤4096 UTF8 bytes each, 10,000 entries, 512 KiB serialized array. Core checks originals/ancestors/bounds and reads selected only; missing/special objects refuse, no category fallback. Public work_selection/planned_target shows exact selection/full destinations. Execute rechecks selected manifest; other changes cannot expand it. Cleanup/reset/import reject selected_paths; reader --path still means package entry.

Named/finite environment_id/plan_id/job_id follow UUID 8-4-4-4-12 hex, either case, invalid_request before state/SSH. Always use returned originals; raw protocol-1 retains rules. Outer remote execute/reconnect plan_id and launch environment_id share UUID checks before records/SSH. install_id has separate bounded rules and comes from preview.

Named execute/new finite remote approval requires 64 lowercase hex. Bad syntax refuses before state/intent/SSH; valid syntax still exact hash/target checked by core. Existing remote execute stays query-only without deletion/replay, reconnect is recovery. TTY launch/target capabilities also validate IDs before core; launch first requires stdin/stdout TTY and no prompt.

env list/discover may register roots/save inventory; job may persist interruption. auth_probe explicitly runs official status, returning identity_observed; missing logout subject refuses, changed auth stales approval, unattempted legacy needs preview, accepted query-only. archive_read returns text. These are not all effect-free metadata. discover shortcut/request share capabilities.

Ordinary stdout is one envelope, ok:false nonzero, stderr diagnostics. Launch takes interactive process control with stderr failures. network serve is the explicit long-running NDJSON exception. ok:true is request success; data.status/steps/coverage/error determine task completion.

## Local inventory and sanitized snapshots

These source-level observations share core. For SSH, check the target runner's `describe` and `schema` first; upgrading the App does not upgrade an older runner.

```sh
lintel env residue '<environment-id>'
lintel env identity '<environment-id>'
lintel env compare '<environment-id>' --other-environment-id '<other-environment-id>'
lintel snapshot save --environment '<environment-id>'
lintel snapshot list --environment '<environment-id>'
lintel snapshot compare --environment '<environment-id>' --old-snapshot-id '<old-snapshot-id>' --new-snapshot-id '<new-snapshot-id>'
```

`residue_inventory` reads metadata at known locations. Uncovered locations, scan limits and read failures remain explicit. `identity_inspect` returns recognized field presence by default. All six commands also support `lintel call OPERATION` with JSON stdin and the shared finite SSH contract; the current runner's schema defines the accepted fields.

`snapshot_save` stores an immutable sanitized observation and returns its ID and public summary. `snapshot_list` displays up to 200 records and inspects up to 2,000 directory entries per call; `scan_complete` and item errors state coverage. One damaged record does not hide readable neighbors. Snapshots retain ordinary settings, finite declarations and residue counts. Headers, credential-bearing collector addresses and raw identities are excluded. Identity comparison material stays in private local state; responses expose only same/changed/unknown, and different salt domains cannot attest identity changes. Immediate `env_compare` shares one temporary salt and saves neither a snapshot nor a persistent salt.

Missing, malformed or unreadable configuration never becomes an off verdict: that area is incomplete while independent observations remain available. Hidden values cannot attest equality. Sanitized snapshots support comparison; restoration still needs the original task or an encrypted backup and a new approval. A digest cannot reconstruct content. Ordinary state/lock behavior remains; these observations do not start Claude, run hooks or contact public endpoints.

`lintel snapshot accept --environment '<environment-id>' --old-snapshot-id '<old-snapshot-id>' --new-snapshot-id '<new-snapshot-id>' --selected-changes '["<returned-change-id>"]'` accepts only selected `changes[].id` values returned by `snapshot compare` as a comparison baseline, without editing configuration. It rechecks the current selected values; stale observations and masked values cannot be accepted. Once a semantic baseline exists, legacy `accept_drift` returns `semantic_baseline_selection_required`; continue with per-item `snapshot accept`. Legacy field baselines remain only for callers that have not adopted a semantic baseline.

### Writable advanced configuration, local data and recipe entries

The named entries below all use the same plan/approval/job/receipt chain and write only after an explicit approval; secrets (archive passphrases) still travel only through JSON stdin, never argv.

```sh
lintel config inspect --environment '<environment-id>' [--project-cwd /abs/project]
lintel config read --environment '<environment-id>' --config-path /abs/settings.json
lintel config edit plan --environment '<environment-id>' --config-path /abs/settings.json   # stdin: {"document":"<JSON text>","expected_digest":"<current digest>"}
lintel config apply plan --environment '<environment-id>' --source-environment-id '<other-id>' --selected-keys '["OTEL_LOG_USER_PROMPTS"]'
lintel config restore inspect --environment '<environment-id>' --job '<original-job-id>'
lintel config restore plan --environment '<environment-id>' --job '<original-job-id>' --selected-keys '["/env/OTEL_LOG_USER_PROMPTS"]'
lintel data clear plan --environment '<environment-id>' --writers-confirmed-stopped   # stdin: {"residue_paths":[...],"archive_before_clear":true,"delete_work":false}
lintel data identity-reset plan --environment '<environment-id>'   # stdin: {"writers_confirmed_stopped":true,"identity_backup_paths":[...]}
lintel data workflow plan --environment '<environment-id>'   # stdin: {"writers_confirmed_stopped":true,"policy":{...},"reset_identity":true,"residue_paths":[...]}
```

`plan_config_edit` needs a strict JSON `document` plus the read-time `expected_digest` and computes a per-field diff; an unrecognized or credential value (`OTEL_EXPORTER_OTLP_HEADERS`, a credential-bearing receiver URL) is only `masked: true` with `before`/`after` null. Execution seals an independent age `lintel.config/1` backup (`config_archive_path`) before writing and reads it back; `plan_config_restore` touches only the fields the original task changed and requires the current value to still equal the written value (else `restore_conflict`), preserving unrelated later edits. `config_inspect` reads finite configuration sources and masks public values without reading identity/credential-file bodies. Explicit `config_read` returns the selected configuration text; use a private terminal/editor and keep its response out of ordinary logs. Managed and identity files cannot be advanced-edit write targets.

`plan_cleanup`'s `clear_residue` needs non-empty `residue_paths`; both operations require `writers_confirmed_stopped: true`. Clearing uses `archive_before_clear` to select a backup before deletion or direct deletion; `archive_only: true` archives without deletion. `plan_identity_reset` always encrypts the complete original client state and selected backups into `lintel.residue/1` before removing only `userID`, without logging out or clearing credentials. The backup can contain sensitive original text; keep it encrypted. Its manifest and restoration open independently:

```sh
# every field comes through stdin; secrets never travel in argv
python3 -c 'import getpass,json; print(json.dumps({"job_id":"<original-job-id>","archive_passphrase":getpass.getpass("Passphrase: ")}))' |
  lintel call residue_archive_inspect
python3 -c 'import getpass,json; print(json.dumps({"environment_id":"<id>","job_id":"<original-job-id>","selected_paths":["/abs/original"],"archive_passphrase":getpass.getpass("Passphrase: ")}))' |
  lintel call plan_residue_restore
```

`lintel.residue/1` is separate from the work package: a valid original-environment binding is required for a new restore plan, while older unbound material remains manifest-readable. It restores only the recorded original paths and refuses to overwrite an existing file (`restore_conflict`), never as a work import. `plan_client_workflow` composes configuration edits, residue clearing and identity reset into one plan and one approval; a file cannot belong to two steps (`overlapping_actions`), and an interruption is queried by the original ID only. The App side is Local data and identity, Changes and comparison, Advanced configuration editing and the three quick recipes; see the [desktop guide](desktop.en.md) and [operator guide](operator-guide.en.md#recovery).

## Complete preview, approval, query and restoration

Use returned IDs/hash from **current host/target**, registering exact config root, never project code directory:

```sh
lintel env register --name 'Example workspace' --root /absolute/claude-config
lintel policy plan --environment <environment-id> --preset reduce --no-keep-remote-control
lintel plan show <returned-plan-id>
lintel job submit --plan <returned-plan-id> --approval <returned-plan-hash>
lintel job show <returned-plan-id>
lintel job wait <returned-plan-id> --timeout 30s
lintel restore plan --job <original-job-id>
lintel plan show <restore-plan-id>
lintel job submit --plan <restore-plan-id> --approval <restore-plan-hash>
lintel job wait <restore-plan-id> --timeout 30s
```

Policy must explicitly choose keep/no-keep Remote Control, optional declared trusted-devices required/not_required/unknown. Custom uses --preset custom --custom-settings '{"DISABLE_TELEMETRY":"disable"}', 27 schema fields with keep/enable/disable/remove/set; masked keys admit only keep/disable/remove, and set requires paired custom_values. Noncustom rejects custom_settings; custom rejects nonempty release_settings. Version/policy/target changes invalidate approval.

Plans persist frozen previews without settings edits; plan show reads public frozen scope without refresh. Approval needs exact hash and existing user authority for those actions, no generic --yes. Restoration preserves later edits/refuses conflicts, never forced overwrite.

Generic execute/job submit admits supported configuration/preservation/archive/cleanup/import/restoration/service mutation kinds only. Launch/resume/unknown invalid_plan_kind before new job/ACK; existing IDs remain query-only without record erasure/retry. Use exact separate launch/resume entries.

Submit is durable ACK, usually accepted, not completion. Independent processes with same HOME/state can query. wait validates UUID before polling/state; invalid_request nonzero. Default 30s, 0–3600s or milliseconds; timeout returns nonzero wait_timeout/original plan_id/latest data without resubmit. needs_reconciliation/interrupted are terminal attention states, without automatic restore.

Accepted error persists code/message/phase/recovery and completed artifacts. Query, never parse Chinese warnings/reexecute. Check actual execution hosting; disconnect/parent exit/logout/reboot differ, ACK has no reboot guarantee, runner no sudo/linger/login edits. Settings interruptions query original settings_recovery: written staged-object/plan proof permits new restore, not_written/ownership_unproven retain files without replay. Normal field restoration preserves unrelated edits.

## All core operations and legacy callers

Other operations use finite call OPERATION with stdin params/shared schema. call launch/launch_request/resume_request and JSON cannot bypass real TTY; App uses finite native adapter:

```sh
lintel call cleanup_inspect <<'JSON'
{"environment_id":"<environment-id>"}
JSON
lintel call service_inspect <<'JSON'
{"environment_id":"<environment-id>","manager":"user","unit":"example.service"}
JSON
```

Describe/schema then real inspect before plans. Cleanup order backup/archive → new-root import(reset_client) → approved logout → exact files → selected retirement. No full-environment export/global killing/restore-all. Service pause/resume separately approved, no auto root rebind.

Legacy request/submit/discover/inspect ID/jobs/job ID/launch ID/tui remain. Raw protocol-1 defaults stay; named/SSH strict fields never silently tighten raw callers. request execute foreground, submit detached/original query. launch TTY/no prompt, not background mutation. Old rebuild/partial receipts never become preservation outcomes.

<a id="portable-work"></a>
## Secret stdin and portable work

Secrets use one-time JSON stdin, never argv/history/permanent request files/plans/receipts. Manual Python getpass reads no-echo control terminal, or use an approved secret pipe; disable input-printing shell tracing:

```sh
lintel work archive plan --environment <source-id> --categories instructions,memory,sessions --output-path /absolute/new-work-package.age
lintel plan show <archive-plan-id>
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' |
  lintel job submit --plan <archive-plan-id> --approval <archive-plan-hash>
lintel job wait <archive-plan-id> --timeout 30s
lintel work archive list
```

Optional output_path otherwise private state. Explicit parent must exist/destination absent; parent device/inode freezes/rechecks, replacement/legacy missing identity stale_plan. Original receipts query-only. Archive-only touches no root/login/original. Separate preserve creates new environment and completes outcome preserved, retains old login, then protect/login/launch normally. lintel.work/1 retains category/path/bytes/digest without credentials/settings/hooks/MCP.

Source limits 256 MiB/file, 1 GiB total, 10,000 files; actual runner preflight may report older lower limits. Persisted JSON ≤16 MiB, state_limit before saving; body preflight does not guarantee full plan, narrow exact selection. Streaming fully decodes/verifies then imports frozen mapping; pages ≤256 KiB without full body residency. Each read still verifies whole package, state disk needs expansion and target cipher/temp space. Normal cleanup removes private staging; kills may retain .lintel-work-stage-* without sweeping. scrypt log N≤20, archive_limit without weakened decryption. Same format does not guarantee old-reader large capacity. SSH 60s/2MiB bounds remain; slow preview/read may transport_unknown, inspect with target CLI and query accepted originals.

Transfer **ciphertext** to independent machine/install without source inventory/job:

```sh
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' |
  lintel work archive inspect --archive-path /absolute/carried-work-package.age
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' |
  lintel work archive read --archive-path /absolute/carried-work-package.age --path CLAUDE.md
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' |
  lintel work import plan --environment <destination-id> --archive-path /absolute/carried-work-package.age --categories instructions,memory
```

Inspect/read/import require exactly --job or --archive-path (file already on target). Import preview/execution checks all parents/current UID/access/write before bodies without chmod/chown. Freeze ciphertext/targets; approved execute gets same passphrase stdin. Changed/corrupt/bad-passphrase/oversized/conflicting targets refuse without overwrite. Explicit SSH/SFTP/scp/removable transfer: [operator work](operator-guide.en.md#work), no auto transfer/full-state upload.

Preserve/rebuild new_root/new_environment_id persist before mkdir, intent only until create completes. Failures query original/path/ID, no recreate/auto-clean. Atomic no-replace Mac/Linux publication unsupported returns atomic_publication_unsupported, no hardlink/overwrite. archive_path is intent; completed step/archive_digest is readback proof. App task-archive selection requires proof; independent paths core verifies.

Named/finite preserve/archive/import/rebuild/cleanup reset_client/retire need ≥1 category; empty arrays/categories refuse. repair_login can omit/empty; raw defaults stay.

Import approval manifest package format/generator/sha256 and files source/destination/category/size/sha256 retains legacy relative fields; planned_target adds full root/destinations/purpose/activation, shared App/CLI and allocated collisions. No text/passphrase. Independent planned_target:done requires every frozen file's publication/parent sync/readback completed records, not frozen list/readable bytes/overall status. Sync uncertainty retains IDs/destinations without done/reimport.

## Configuration roots, project directories and session input

CLAUDE_CONFIG_DIR is registered root; project_cwd is existing usable target-host project. They differ; Lintel creates no project/permissions. New launch previews exact directories/program/static version then immutable approval. plan_launch/plan_resume only reads/freezes, no external execution/TTY, via JSON/named/finite SSH. launch_request/resume_request need exact plan/hash, real TTY or macOS core Terminal adapter, never runner JSON/finite requests. Remote interactive control pins request/runner. Resume additionally publishes private copy, a mutation.

startup lists finite candidates/declarations/auth metadata without commands/tokens/instructions; actual_loaded/auth unknown. Changes/legacy missing binding need preview; original records query-first. Terminal shell/Keychain/organization/outside-cwd imports need separate checks; user /status and /mcp in target terminal.

Directories freeze device/inode/owner/birth time against immediate Linux reuse, without freezing ordinary project contents. Missing birth time directory_identity_unsupported/no launch; legacy unattempted repreview, originals query-only.

```sh
lintel call plan_launch <<'JSON'
{"environment_id":"<environment-id>","project_cwd":"/absolute/project","mode":"interactive"}
JSON
lintel plan show <launch-request-id>
# Run in a real terminal; no prompt argument:
lintel launch_request <launch-request-id> <launch-plan-hash>
lintel launch query <launch-request-id>
lintel launch list
```

Old lintel launch ID keeps root-as-cwd; new requests explicit cwd. App copies the reviewed draft before opening, copy failure blocks launch. User separately pastes/sends/verifies model receipt; no ordinary journal text.

Readonly session pages continue prior next_offset, re-unlock on source changes without mapping old fragments. content_kind:text gives raw_text/no records; message pages arrays, legacy missing arrays show limit with raw text available:

```sh
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' |
  lintel work session read --archive-path /absolute/work.age --path projects/example/session.jsonl
```

Continue --offset/optional expected-digest. Unknown/nonASCII/newlines retain original bytes; text/structure are views, no transcript edits. Opaque thinking/signature never auto handoff. index is page-local; offset original record/fragment byte; block_index zero-based original content array including opaque/unknown. Single/unknown block is 0; bounded long-line fragments do not prove full parse. Handoff/plan_launch.input_reference.files retain path/file/package digests/offset/block without text. Optional offset integer 0–268435456, block_index 0–1000000 requires offset; old imprecise references remain without invented precision, App labels missing positions.

Native resume separately assesses ≤8 MiB transcripts; larger remain archived/readable but unsupported for resume. call plan_resume stdin passes exact source/path/environment/project/getpass; describe/schema first, then real TTY resume_request REQUEST_ID HASH. Preview shows support/private copy/unverified auth/original session. Absolute copy/fixed resume/fork flags, no ID/signature/index edits. Copy/inert args do not prove authenticated recovery. Original query needs no approval/passphrase/Terminal/copy; remote query pins original runner across upgrades.

<a id="browser-adapter"></a>
## Browser: finite adapter and human waits

macOS CLI:

```sh
lintel browser operations --json
lintel describe browser.submit
lintel schema browser.submit
lintel browser instances
lintel browser operations --instance <paired-instance-id>
lintel browser pair create
lintel browser pair pending
lintel browser pair approve --challenge <returned-challenge>
```

Static operations opens no journal; targeted instances return real pairing/conflict/online, permissions still extension-confirmed. Run on profile host; Linux browser_component_unavailable. Installation/loading/pairing remain human, not implied by CLI success. Submit/query stdin finite host objects; generate/retain original operation_id/exact instance_id/action schema. Host_path control remains independent, no copied App locator/preferences writes.

```sh
lintel browser query <<'JSON'
{"instance_id":"<paired-instance-id>","operation_id":"<original-operation-id>"}
JSON
```

Handle unpaired/offline/confirmation/restart/uncertain/rejected/completed separately. Clear preparation then full quit/native onStartup/new finishClear approval; no worker restart/generation fabrication/uncertain-delete replay. Expiry/startup rule restoration shares serialization, consumes matching record/ID only, legacy full-record matching retained. Changed rules/uncertain readback preserve records without overwriting newer pause.

IDs 8–80 ASCII alphanumeric/hyphen/underscore; Chrome/Edge IDs 32 a–p, Firefox lintel@lintel.local. Manifest chromium name implies no installer path; installer only Chrome/Edge/Firefox.

## Network: explicit foreground ownership

```sh
lintel network serve --config /absolute/reviewed-channel.json
```

[Config](network.en.md) requires default_action/exact rules. First NDJSON listening returns owner/PID/actual address/normalized config; Ctrl-C ends CLI channel. Proxy coverage only, no all-process enforcement. App independently owns its channel; CLI cannot query/stop it.

### Telemetry catalog and current-proxy proof

```sh
lintel telemetry catalog
lintel network catalog
lintel describe telemetry_catalog
# reviewed-channel.json must explicitly block the catalog host:443; test only this proxy.
lintel network serve --config /absolute/reviewed-channel.json --test-telemetry datadog_logs_intake
```

telemetry_catalog takes no params/state/network, through request/finite remote. Direct-API Datadog vs mixed model/auth/download hosts, applicability/source date never runtime proof. Original Proxy requires explicit block before startup and actual matching refusal/event afterward, no public DNS/target. App/CLI independent; App binds instance. Client/test origins separate. Fine presets retain Claude updates; any nonempty NONESSENTIAL, including 0/false, disables updates/release notes/flags. Readback never proves external loading. [Telemetry](telemetry.en.md), independent [App updates](app-updates.en.md), no CLI auto App/component/runner upgrades.

### Finite IP tests and shared IPv6 plans

```sh
lintel network inspect
# Explicit public request; defaults to two ipify HTTPS echoes, URLs can be overridden.
lintel network probe --timeout-seconds 10
# Use an already-running same-host CLI channel and its actual read-back port.
lintel network probe --proxy-url http://127.0.0.1:49152
# macOS: use inspect-returned IDs, not display names; preview includes a baseline.
lintel network ipv6 plan --service-id '<SET_ID>:<SERVICE_UUID>' --mode off --baseline-id '<PROBE_ID>'
lintel job submit --plan '<PLAN_ID>' --approval '<EXACT_PLAN_HASH>'
lintel job show '<ORIGINAL_JOB_ID>'
lintel network restore plan --job '<ORIGINAL_JOB_ID>'
```

Replace placeholders. Probe/ipv6 plan accept ipv4-url/ipv6-url/proxy-url/timeout-seconds; --probe JSON chooses nested spec, also restoration. Baseline ≤5 minutes same revision/endpoints/instance/deadline. Finite HTTPS endpoint/literal same-host HTTP loopback, strict pretransport fields/targets. Describe network_probe/plan_network_ipv6; raw validates too. environment_id:null/no root. Preview no system writes; approval includes write/full readback/same retest, durable intent then query on interruption. Restore separate/outside-edit refusal. Native App owns binding; CLI only own live channel. Linux inspect/probe/serve ipv4_only, no host mutation. Echo IP exposed, private target state requires review before support sharing; synthetic no host actions. [Network](network.en.md).

<a id="ssh-controller"></a>
## SSH: one finite controller

```sh
lintel remote aliases
lintel remote control <<'JSON'
{"op":"add_host","alias":"approved-alias"}
JSON
lintel remote inspect approved-alias
lintel remote hosts
```

Aliases reads current HOME config; add/remove registry only. inspect connects registered literal/discover, no Claude changes. Fixed strict options/no arbitrary paths/programs/scripts/key bypass. remote request stdin finite schema; mutations require remote submit execute with intent before runner. remote job readonly request.job uses original pin/deduplication; unknown IDs create no records/consume no submission. Removed originals remain.

Unknown lookup request job accepts exactly job_id or plan_id, current runner without intent. Explicit reconnect saves query-only intent/current digest if absent, then pins it despite upgrades; not for speculative unsubmitted plans. Use job/request.job there. Legacy PATH remains, never delete/resend uncertain records.

```sh
lintel remote request approved-alias <<'JSON'
{"command":"plan_policy","environment_id":"<remote-id>","preset":"reduce","keep_remote_control":false}
JSON
lintel remote submit approved-alias <<'JSON'
{"command":"execute","plan_id":"<returned-remote-plan-id>","approval":"<returned-remote-hash>"}
JSON
lintel remote job approved-alias <original-remote-plan-id>
```

Remote new plan_launch specifies environment/cwd/interactive, then real macOS TTY remote launch request ALIAS REQUEST_ID HASH. Resume plan then launch resume, passphrase reentered SSH no-echo. Repeat queries/pins. Query/list readonly without TTY. Legacy remote launch ALIAS ENV_ID keeps root-as-cwd, both TTY/exact args/no prompt, readonly preflight then Terminal. JSON interactive controls refuse hidden-pipe sessions.

prepare/install/query share App frozen approval/resources/prior registration/binding. Unresolved blocks both prepare/upload with original install_id. Activation requires previous_binding; superseded retains newer, verified legacy unbound; missing-preview evidence requires preparation. [Remote](remote.en.md)/schema own fields. Legacy Python subset retains callers, new features canonical Rust. Synthetic evidence never replaces real auth/browser/WebKit/VPS/logout/reboot; [state (Chinese)](current-state.md)/[SPEC (Chinese)](SPEC.md).

migration_path_conflict means actual case/Unicode/path equivalence; approved empty-file preflight rejects before root/body, preview readonly. Existing nondirectory ancestors path_unreadable before body. Existing root/parent modes remain, new dirs0700/files0600. Packages remain readable; reorganize or different filesystem, never original replay. migration_probe path/status/executing step persists before temp creation; removed after confirmed cleanup, retained stops bodies. Killed executing retains path, query never deletes/retests. App exposes pending directory; only owned empty scope, extras retained, not new environment.

## Maintainer appendix: building and packaging candidates

### Portable package creation and verification

package-cli.mjs packages **explicitly supplied canonical outputs**: arm64 macOS Mach-O, x86_64/aarch64 static Linux musl ELF and canonical remote-runners. No compile/download/signing; missing/wrong-arch/dynamic/mismatched Linux refuses. Linux CLI inputs must byte-match canonical runner, no mixed candidate. Redistribution preparation supplies `--notices` with the complete notice directory matching the binaries’ build toolchains; see [distribution preparation](app-updates.en.md#local-unsigned-dmg-and-third-party-notices). The finite inventory-listed licenses and notice texts are retained under `third-party-notices/` and covered by candidate file metadata and `SHA256SUMS`. Existing synthetic callers may omit the option; those packages explicitly record `notices_included:false` and are for local testing only. The notice collection is a dependency superset, not an exact linked SBOM.

Flat runtime: bin/lintel, bin/remote-runners/manifest.json, bin/remote-runners/<triple>/lintel (both), candidate.json/SHA256SUMS/README.txt. Lookup is dirname(executable)/remote-runners, not archive root. Verify/archive same-read manifest bytes to avoid resource rebuild mixing.

Each runner ≤32 MiB installer upload, runner_too_large before any archives/index. Mac needs full 64-bit header/arm64/MH_EXECUTE. ELF64 executable/static PIE requires nonzero entry in file-backed executable segment, file size≤memory and no PT_INTERP/DT_NEEDED. Truncated/object/dylib/no-entry shared objects refuse; formats are not other-architecture runtime. Probed/declared versions share safe syntax before filenames.

Mach-O validates command counts/bounds/segments/sections and LC_MAIN or ARM64 LC_UNIXTHREAD mapped executable entry. Header-only/unmapped/nonexecutable refuses. Publication failure retracts only this attempt's unchanged-identity/size/mtime links, preserving preexisting/outside edits/replacements, allowing retry; abrupt kills have no automatic transaction recovery. Mac platform LC_BUILD_VERSION or legacy LC_VERSION_MIN_MACOSX required, others/missing refuse. Child TAR_OPTIONS removed; exact declared archive members verified to prevent inherited omission/rename.

Mac input target/release/lintel; cross inputs build from source. Linux CLI/canonical runner:

```sh
rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl
cargo zigbuild --locked --release -p lintel-runner --bin lintel \
  --target x86_64-unknown-linux-musl --target aarch64-unknown-linux-musl
# Generate canonical resources in ignored runner-bundles.
node apps/desktop/scripts/prepare-remote-runners.mjs
```

Outputs target/<musl-triple>/release/lintel. Mac cargo build --locked --release -p lintel-runner --bin lintel produces target/release/lintel. [Resource preparation](remote.en.md#prepare-app-resources). Package from root into ignored/caller temp directory:

```sh
node scripts/package-cli.mjs \
  --macos-arm64 target/release/lintel \
  --linux-x86_64 target/x86_64-unknown-linux-musl/release/lintel \
  --linux-aarch64 target/aarch64-unknown-linux-musl/release/lintel \
  --linux-runners apps/desktop/src-tauri/runner-bundles \
  --notices apps/desktop/src-tauri/third-party-notices \
  --revision "$(git rev-parse HEAD)" \
  --out candidate-packages
```

Identity `<version>-candidate-<first 12 revision characters>`, never bare 0.1.0; candidates-<identity>.json separate indices. --revision exact lowercase 40/64 hex Git object declaration, not digest-proven source. candidate.json product/version/protocol/catalog/revision/target/payload bytes/SHA/signed:false/release:false/limits/per-target identity_source: macos_input_executed only successful Mac probe, Linux declared_static. Probed Mac version authoritative/matches manifest. Failed/no probe (e.g. Linux packaging) needs explicit --version with identity_verified_executed:false or refuses. Existing archives/dangling links/same-identity index refuse before writes.

Debug source cargo build -p lintel-runner → target/debug/lintel. Native host installed separately; Mac browser control requires exact host_path, no GUI/PATH assumptions. Canonical extension source/[browser guide](browser.en.md). Remote resources share [generation](remote.en.md#prepare-app-resources); explicit reviewed --bundles or LINTEL_RUNNER_BUNDLES, default beside executable remote-runners. Missing refuses without download/VPS compilation. Reading grants no real-host installation/update.

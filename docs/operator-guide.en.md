# Lintel operator guide

[简体中文 (canonical)](operator-guide.md) · English · [Documentation](index.en.md)

Applies to Lintel 0.1.0 Preview/development preview, based on current source/module contracts. These instructions precede formal release; [current state (Chinese)](current-state.md) owns candidate/runtime status. For a first trial use the [English quickstart](quickstart.en.md) / [中文 quickstart](quickstart.md).

Lintel handles specific Claude environments: reduce selected optional egress, manage browser permissions/site state, preserve work, clean identified old state and manage similar Linux VPS tasks from Mac. Identify the target, review actual changes, then use receipts to continue or restore.

**0.1.0 Preview** has the connected paths below. Formal browser installation, real authentication, production VPS, real first-use and non-developer installation still have acceptance gaps. A module test does not verify the whole environment.

## Getting started

With a locally built App, use the task map below. Signing/notarization/non-developer installation remain incomplete. To build, follow [README local build and trial](../README.en.md#local-build-and-trial) for Rust/Node/macOS requirements.

Linux uses standalone runner/CLI without desktop. Mac can approve bundled Linux runner installation in its host panel (§5). Matching macOS arm64/Linux x86_64/aarch64 CLI packages need no Rust/GUI: [verify/unpack into a new version](agents.en.md) and explicitly select executable, preserving old versions/tasks. Source `cargo build -p lintel-runner` produces target/debug/lintel. An installed App does not put lintel in terminal PATH; check the binary. For practice, README dev:synthetic clearly labels test space.

App source supports Simplified Chinese/English, follows system by default with local Settings selection. This edition uses English descriptions of controls; raw paths/commands/work/diagnostics remain unchanged. Switching retains target/forms/approval without submitting. Old candidates do not update with source; see [state (Chinese)](current-state.md).

## Choose one task

| Goal | Entry | Expected result |
| --- | --- | --- |
| Reduce egress, continue using Claude | Home Start a task → Protect plan, or sidebar | Exact settings/readback, then a new selected-environment session |
| Browser exposure/permissions/old login | Home Browser | Selected-profile pairing/settings/clear receipts |
| Preserve work/change environment | Work preservation → archive only or new environment | Encrypted archive; new root/import when selected |
| Login repair/old state/retirement | Cleanup and rebuild recipe | Exact files/results, optional new root/retirement |
| VPS/background services from Mac | Top host or Settings → Hosts and SSH | Remote workspace, original-query receipts, separate service restore |
| Drift/restoration/interruption | Environment details → Check changes; Records and recovery | Differences, accept/restore decision, latest original result |

Home quietly centers Clawd/original time greetings. Explicit Start a task opens six paths; Continue last goal returns workspace, pending entries query originals only. Sidebar retains professional areas plus Sessions and materials/Terminal and Agent. Current goal remains visible; navigation neither submits nor calls models. Names share the finite lintel tasks --json catalog.

## Identify three different targets

**Host** determines execution: Local is this Mac; SSH alias means that host's current login user. Remote banner says environments/plans/records belong there. Empty inventories retain top/Settings host controls.

**Environment** is primarily a registered Claude Code configuration directory, not project code or a credential-isolated account container. Overview → Configuration sources and verification scope shows root; Launch and sources shows executable/root. Confirm host/name/full path together.

The toolbar keeps the host and environment name visible. A separate lower row holds an orange terminal icon with a slow-blinking cursor. Click it to expand the full root, environment ID and client path downward, with path-copy controls; the icon, Close or Escape collapses it. Content moves below the details, outside interaction retains them, and changing target collapses them. Collapsed paths do not change exact target review in approval previews.

**Browser profile** is independently authorized on this App's Mac. SSH never turns it/local proxy into VPS components. Root/profile do not automatically establish identity isolation.

Components inspects finite root sources/static CLI/auth locations/exact recorded services. Optional target-host absolute cwd and Refresh components checks project/local/MCP without creating/starting. Account unknown, Desktop/IDE unsupported, browser independent. Recorded units refresh; unverified stays unknown, old pause is not live state. Check original task queries only this host's original ID.

### Register an existing environment or create an empty one

Add existing environment/sidebar plus: supply name/full config-root path; registration itself edits no settings. New environment → create with name makes a new root without login/settings/hooks/MCP, then choose protection/open/login normally. Empty lists keep both actions; work transfer uses §3.

A new directory separates local state but cannot prevent same-user file access/shared outside authentication. Test-space labels describe synthetic data, never personal scans. Ordinary web preview saying start from desktop has no native execution channel.

<a id="protect"></a>
## 1. Reduce egress and continue using Claude

### 1.1 Choose feature tradeoffs

1. Select host/environment, open Protect plan from sidebar/home task. Use full page for Remote Control/custom controls.
2. Choose Keep features/Reduce egress/Custom. This edits host/environment draft only, never applies on switching.
3. For Remote Control, select retention and declare Trusted Devices requirement, leaving unknown if unsure. Static installed version plus your declaration determines current/expected configuration conditions.
4. Decide whether to remove listed conflicts. Ordinary presets explicitly include selected field removals; Custom uses Remove environment override.
5. Preview changes, check fields/preservation/steps/target and approve once, or return to adjust.

| Preset | Current action | Use when |
| --- | --- | --- |
| Keep features | Disable errors/surveys, retain current metrics/proactive feedback | Fewer edits/current tradeoffs |
| Reduce egress | Control metrics/errors/feedback/surveys separately, handle declared Remote Control conflicts | Explicitly reduce these optional flows |
| Custom | 27 v5-rule controls with keep/enable/disable/remove/set choices | Specific controls/your own tradeoffs |

The custom list is driven by the current core v5 rule and holds 27 recognized fields: the seven privacy switches (metrics, errors, proactive feedback, surveys, cross-tool telemetry, feature flags, nonessential blanket switch) plus OTel collection, content recording, exporters/receivers, metric identity attributes, survey-over-OTel and WebFetch control. The list shows all fields and supports keyword filtering and batch actions; each row expands to the exact variable, current value, Boolean semantics, source, receiver or on-disk destination, official default, version evidence and dependency fields. Some nonempty variables disable even with 0/false; follow per-variable explanations. `OTEL_EXPORTER_OTLP_HEADERS` shows only "masked (credential)" and its value never reaches ordinary output.

Keep preserves fields; all-keep leaves settings bytes. Enable deletes the key for a DISABLE_* switch and for a default_on field. Disable writes the off value per semantics: privacy switches `"1"`, positive Boolean content flags `"0"`, default_on `"false"`, an exporter `"none"`; any other free-value field can only be removed. Remove deletes only this root's user override; shell/project/organization sources still matter. Set an explicit value goes through a separate `custom_values` channel carrying an admitted string, and the masked key never admits it. Removing or replacing an existing masked value requires a passphrase in the preview, encrypts the original configuration first, and supports selected-field restoration from the original task. Retain Remote Control never secretly overrides custom choices. WebFetch tool control now enters the catalog as `CLAUDE_CODE_DISABLE_WEB_FETCH`; official updates and marketplace automatic installation remain outside this editor, and custom OTel/general proxy/execution config are not wildcard-cleared.

### 1.2 Read back, then open a new session

Inspect steps/fresh-launch requirements. **Configured/read back means values were handled, not that existing Claude exited/reloaded.** Open Claude in receipt or Launch and sources uses selected root. Missing executable disables it; install Claude normally then recheck. macOS launch requested means Terminal acceptance; verify actual login/session/IDE/service behavior separately. Readback is not automatic network-effect proof.

To restore, find original receipt → Preview restoration → exact fields → approve. Later conflicting edits remain; inspect sources/select current policy rather than overwrite whole old root.

### 1.3 Control proxy-path connections

Local Environment details → Egress and permissions → Controlled channel. Edit next-start default allow/deny, exact blocked/allowed hosts and HTTP(S) upstream. One host per line with optional comma-separated ports:

```text
example.invalid 443,8443
```

This is syntax only. Omitted ports means all; no automatic subdomains. Explicit block precedes allow/default. No blanket telemetry blacklist; mixed API hosts may lose necessary requests.

1. Start explicitly after editing.
2. Check adopted configuration/actual local address from live readback.
3. Open Claude through channel for a new proxy context.
4. Refresh connections; absent completion events do not mean no traffic—live connections may have none.

For changes stop/wait/edit/start/reopen clients. Ports may change; old clients do not update. Ended tasks need old-channel clearing. App exit stops channel without closing Claude. Direct sockets/UDP/all DNS/NO_PROXY/other processes/client bypass remain outside proof, without process enforcement. SSH cannot start remote proxy here. Browser site proxy uses an actually running local port (§2).

### 1.4 IP egress and shared settings

Open same panel or home host-network action. Review HTTPS endpoints/paths, explicitly Test actual egress for four cells of host/channel IP/duration/failure. Missing channel untested. Request reveals egress IP, without background tests or other-app routing proof.

Mac Off/link-local previews freeze exact service/full config/baseline. Review host-shared impacts and approve write/same-target retest, possibly OS authorization. Other apps affected; exit does not restore. Configuration/pre-post results separate; timeout/IPv4 success is not no-leak proof. Restore independently previews/approves full original, preserving conflicting later edits. Lost responses query original.

Network/channel changes, same-port restart, changed targets or age >5 minutes invalidate baseline. Preview retests. Strict IPv4 is next-start channel choice, verified after stop/start; only Lintel-created target/upstream sockets. Linux readonly target-runner observations/tests, no host-off action. [Network guide](network.en.md).

<a id="browser"></a>
## 2. Browser exposure, permissions and old login

### 2.1 First profile connection

Home Browser → Connect a new profile uses bundled extension/host, without source/terminal build.

**Prepare/load:** choose browser, preview version/files/directory, approve, reveal/copy (Firefox manifest). Load in actual profile:

| Browser | Page | Action |
| --- | --- | --- |
| Chrome | chrome://extensions | Developer mode → Load unpacked → prepared directory |
| Edge | edge://extensions | Developer mode → Load unpacked → prepared directory |
| Firefox | about:debugging#/runtime/this-firefox | Load Temporary Add-on → manifest.json |

Chrome/Edge share stable files but each profile loads/pairs separately. Firefox temporary loading disappears on exit; lasting signing unavailable. Developer mode/formal non-developer journeys remain unaccepted.

**Local connection:** exact management-page extension ID (Firefox fixed), preview browser/authorization/registration, approve stable current-user installation independent of App moves.

**Pair profile:** generate/copy 12-character code; extension Connect desktop takes code/profile label, submits; App checks pending name/instance/codes, then approves matching codes.

Completion means paired plus recent 20-second polls. Paired can be offline; authorization and connectivity differ from prepared files/registered component. On failure verify actual load/exact ID; regenerate expired codes. Duplicate conflict: extension generates new identity/re-pairs. Owned extension/host updates need preview/approval/browser reload. Outside/edited/inconsistent resources refuse; interrupted preparation has separate recovery approval. Installer only Chrome/Edge/Firefox, never inferred AdsPower/other Chromium support.

### 2.2 Choose a needed action

Select paired/online/nonconflicting profile. Send to browser preview submits only; extension opens desktop proposals, reviews actual impact, confirms scope, grants permissions/executes once.

| Action | Scope/tradeoff | Restore/continue |
| --- | --- | --- |
| WebRTC | Whole profile; non-proxied UDP limits may affect calls | Separate restore/extension approval; controller/value conflicts |
| Permissions | Selected-site location/camera/microphone/notifications block | Owned-rule restoration; Firefox browser panel manually |
| Site proxy | Selected sites/local 127.0.0.1 HTTP port/no DIRECT | Proxy must run; separate restore; Firefox unavailable |
| Block sites | All selected-site features | Restore or pause |
| Pause rules | Owned blocking 1–60 min | Expiry/next startup if offline; no clear-isolation release |
| Whole-profile cache | All HTTP cache including other sites | Separate irreversible confirmation |
| Clear site data | Selected sites/types | Preparation/full restart/second approval; irreversible |

Default claude.ai, optional console.anthropic.com, no email/Google/Apple/SSO. Browser readback is not device-wide IP hiding or full DNS/WebRTC/program coverage.

### 2.3 Site clear: two approvals around a full restart

1. Select sites/types; cookies affect login and Chromium may expand to registrable domains disclosed in preview.
2. Submit/review/approve in extension: isolate, close targets/iframe hosts, unregister SWs.
3. Awaiting restart means **other storage is not yet deleted**. Fully quit/restart, not just window/worker.
4. Preview continuation from original preparation receipt/App and separately approve extension deletion.
5. Query/read deletion receipt, then independently release isolation from that receipt and manually revisit/login.

Pause rules never releases isolation. Continued isolation avoids premature revisit; original queries never redelete new login. Missing reload/startup generation refuses; reconnect while retaining preparation. Firefox lacks site CacheStorage/HTTP cache and verified container writers; temporary removal on exit is not accepted persistent restart clear. Independent Chromium real restart/synthetic storage does not accept formal browsers/real claude.ai auth. Lost replies query original/older browser records, never new uncertain deletion. Restoration limits: §6.

<a id="work"></a>
## 3. Preserve work or change configuration environment

### 3.1 Archive only

Home/sidebar share Work preservation. Default archive only selects instructions/memory/sessions, reviews metadata then preview; overview is not backup. Preflight checks metadata for >256 MiB/file, >1 GiB selection, >10,000 files or incomplete scans. Blocked previews require narrowing categories/exact originals/recheck without deletion/truncation. Real plans fully read content. Frozen metadata has independent 16 MiB record budget; deep paths/destinations may refuse before save/acceptance. Runner-reported older limits govern current target.

Project/session/file selection loads metadata pages, initially all current-category originals. Path groups select their files; standard sessions group subagents, others exact paths, individuals adjustable. Exclude oversized deselects only above reported single-file limit, unavailable without report. Grouping reads no text/guesses no cwd. Search changes display; searched group toggles matching files only, while select-all affects whole category.

Selection remains only this host/environment page; leave/reselect, no passphrase/content drafts. Refresh never adds new files; missing selected paths need removal after inspection. Incomplete scan/old unsupported inventory blocks exact approval; explicitly choose whole-category if desired. Exact preview shows original scope, same frozen list for archive/new root. Selected changes stale approval; unselected changes never expand. Links/restricted/special/budget gaps are not empty/complete.

Optional full output path belongs to **target host**, existing non-symlink parent/unused filename. No overwrite. Preview freezes destination/count; replaced parent/remount requires preview. Old explicit-output plans without identity repreview; completed queries remain. Passphrase ≥12, repeated, user-held/unrecoverable if lost. Approve once. Completion provides path/categories/count without root/login/settings/original changes.

### 3.2 Preserve and prepare a new environment

Choose new-environment goal/categories/optional name. Preview ordered archive/create/import and frozen ID/full root/file destinations without creating. Outside occupancy/changed parent requires preview. Execute archives first then creates/imports/readback; success completes this goal.

App defaults references, explicit reviewed-instruction activation places CLAUDE.md in active root. Raw protocol-1 defaults retained. Memory/session projects enter lintel-imports. Existing references rearchive without nesting; active/reference instructions both retained/category preserved, no implicit activation. Collisions retain both with lintel-N filenames; independent import uses frozen same allocation, refuses existing files. Approved empty-file filesystem preflight rejects case/Unicode collisions before root/body with migration_path_conflict. Package remains readable; reorganize/rearchive or different filesystem. No credentials/settings/hooks/MCP/plugins/automatic old-session claims; old root/login/processes retained.

Result offers new-root protection, then normal login/launch/runtime checks. New root **does not rebind systemd**; old service stays old-root, independent restore/new configuration. Legacy reset/TUI rebuild remains partial historical cleanup, never rewritten; App uses preservation semantics.

### 3.3 Read or import an existing package

Approval's file destinations lists source/full target (legacy relative to root), final collision allocation and expandable file/package digests. Unlocked inventory alone is not final mapping. Sessions and materials/work-page/records all use one reader; task view selects current package.

1. Select target-host task archive or independent absolute .age path; source job is unnecessary.
2. Passphrase/unlock, review inventory/category/capacity, read on demand. Structured roles/time/text/tools and raw fallback; ordinary text raw-only, legacy missing records explicit limit. Pages show byte positions, retain complete originals; file count is not resumable-session count. Initial status immediately says full verification/read. Paging retains previous page with explicit label until replacement. Global status shows verifying host/operation/queue waits. Leaving clears page passphrase/text/selection, but submitted read continues and same-queue work waits; late results cannot reopen reader.
3. Select registered target/categories/explicit instruction activation, preview. Existing root/parents need current executor ownership/access/write; obstacles refuse all bodies without permission edits. Review root/list/frozen package.
4. Approve with same passphrase. Changed package/conflicting filename refuses. planned-target completion requires every publication/parent sync/readback; paths/readable bytes are insufficient, uncertainty query-only. Batch parent conflicts refuse before bodies. Existing modes stay, new dirs0700/files0600. Choose dedicated target/narrow categories then new preview for conflicts.

Interrupted root creation/registration: original new_root/ID are exact intents, create completion proves registration; possibly existing unregistered roots offer no protection action. Inspect path/receipt, never recreate. Archive/state paths may be intents; UI marks pending and offers task-reader only after readback. Existing artifacts can be independently inspected; query never republishes. Unsupported no-replace rename returns atomic_publication_unsupported with originals retained. Passphrase stays transient until close; unlocking executes no content. lintel.work/1/old legitimate packages remain compatible.

### 3.4 Same-host and Mac ↔ VPS import

Same-host uses job/path. Cross-host explicitly transfer **ciphertext** with trusted SSH/SFTP/scp/removable storage, switch host and use its absolute path. Never entire state/credentials/plain requests; App does no transfer/path mapping. After verifying alias/destination, system-terminal example:

```sh
scp -o StrictHostKeyChecking=yes /absolute/path/work-package.age approved-alias:/absolute/destination/work-package.age
```

Choose unused filename/check transfer independently; system-tool overwrite/execution is outside frozen import. Target inspect/verify/select/approve follows. [Secret stdin](agents.en.md#portable-work); browser/proxy do not migrate.

Oversize/budget errors refuse complete archiving, retain originals, narrow categories/files/exclude session. Mid-operation inspect original steps/artifacts without recreating/import replay. Executing archive records path but only readback proves completion; existing package can unlock by original job.

Each unlock/list/read/import fully authenticates every file/tail even for one selected category/page, then reads finite staging windows. Large pages still cost whole decrypt/time. State needs expanded originals, target cipher/temp disk. Normal errors/exits clean staging; kills may leave private .lintel-work-stage-* without historical sweeping, not durable backup. SSH 60s/2MiB may transport_unknown without corruption; target CLI checks, accepted original query-only. Reader bounds metadata/cipher/scrypt log N≤20, refusing expensive packages without weakening; age KDF still consumes memory. Older readers may reject new capacity; use matching newer version.

State backup is distinct, no automatic mixed-state import or full auth/config/cloud restoration. Its path persists before writing; only step readback completes. Interruptions query/check, never repeat cleanup.

### 3.5 Build new context from originals

Select user/Claude/tool/unknown fragments in reader, generate editable handoff with goal/constraints/decisions/files/state/next/verification/source summaries, fill/review. Deterministic/manual only, no external model/JSONL rewrite/fabricated transcript. Opaque/redacted thinking/signatures remain originals, excluded from new context.

References retain package/file digests/original byte/block positions, not page-local record-zero collisions. Zero-based blocks include opaque positions; hiding never renumbers. Old missing positions visibly imprecise; long fragments not full-line parsing. Text is explicit interaction/copy only; durable launch references metadata.

Select continuation environment, review draft/target and **target-host project cwd** separately from CLAUDE_CONFIG_DIR root. Directory existence/access/identity checked without project creation/code migration/permission change; remembered paths recheck every time. Source preview lists root/cwd/ancestors/managed/drop-ins, presence/hook/MCP/helper/plugin/auth-name declarations without bodies/commands/tokens. Checked-absent expand; restricted/overlimit/directory-assets/unknown separate. New root cannot remove project sources; candidates do not prove loading. Old runners need update/projection.

Before intent/copy/Terminal, first launch/resume rechecks private binding; new/deleted/edited/replaced relevant objects stale approval, unrelated project bodies not frozen. Organization/MDM/worktree main/imports/directory content/Keychain/shell remain unknown; not external-writer OS CAS. Attempted IDs remain query-only despite later changes. On error check original. Only proven unattempted plans permit recheck/new ID/review retaining draft/cwd; intents/results never replay. In Terminal /status checks config/Login and /mcp connections; login explicitly, no auto draft send.

Preview needs birth time against replaced/reused-inode directories. directory_identity_unsupported: choose supporting project/config filesystem, reading still available. Legacy unattempted repreview, attempted query-only. Review program/version/project/root/draft then copy/open. Copy failure blocks launch/retains draft; copy success/open failure separate. Uncertainty original query without copying/second launch. Draft/target/cwd changes invalidate review. Ordinary launch has no prompt, user pastes/sends; no model-received claim. Preview needs no TTY/client, execution exact plan/hash/real Terminal. Generic submit rejects new launch/resume invalid_plan_kind before acceptance; original jobs query-only.

Passphrase/text/draft are transient, clear on leave/lock/source switch. Explicit clipboard copy only, no background clipboard handling or later-content clearing. Text may contain secrets; selection is not redaction. Journals/Agent operation handoff hold metadata only.

### 3.6 Independently inspect native resume

JSONL action checks native resume without draft. Select environment/cwd/review target; preview format/static version/finite adapter/private-copy/state effects. Unknown combos explicit limits with reader/new context still available. Assessment ≤8 MiB; larger fully preserved/readable, never inferred from truncated prefix.

Approval prepares private/read-back copy and interactive request; import/preserve never resume. Original bytes retained; copy existence is not real recovery. Client may update config/permission/model state/reread project/hooks/MCP; auth/original activity separate. Repeats/lost replies original query, no silent mode change. SSH reenters no-echo passphrase; App passphrase never script/argv/journal. Open-window/still-planned: inspect that terminal, never reopen.

No real login in this development. Official entry/inert argument correctness/real recovery are three evidence layers; only last verifies actual continuation. [State (Chinese)](current-state.md)/[18 checks (Chinese)](specs/product-baseline-status.md). [Preserved thinking](https://platform.claude.com/docs/en/build-with-claude/preserved-thinking), reviewed 2026-10-05, separates compatibility/prefix/model-account constraints. Sonnet 5.5 originating/linked-account rules affect thinking, not automatic old authentication. Lintel retains opaque originals without signature repair; checksums only bytes. New roots/IDs/removed fields prove no independent/unlinkable identity.

<a id="cleanup"></a>
## 4. Login repair, old state and retirement

Choose exact scope. Network errors/failed responses are not automatically bans; local identified-state handling never changes server account status.

| Recipe | Actual scope | Remaining checks |
| --- | --- | --- |
| Repair local login | Previewed credential files, optional official logout | Keychain/outside auth/server revocation |
| Clean and rebuild | Encrypted work/mixed-state backup, exact files, new root/import | Launch/login/browser/independent scopes |
| Retire | Exact state plus disabled Lintel launch entry, originals retained | Not uninstall/whole-root deletion/deleted-credential recovery |

Repair targets .credentials.json; others also previewed .claude.json, differing default/dedicated roots. No whole-machine Claude cleaner.

Local-scope rows initially show a filename and presence. Expand a row for the full path and confirm the exact target before preview.

1. Confirm host/environment/root, choose recipe.
2. Close target terminals/IDE/restarters; systemd requires §5.3 independent pause.
3. Refresh exact files/existence/writers, confirm stopped. Running/unassignable writers block; checkbox never bypasses checks.
4. Optional explicit auth inspection runs official command, unlike discovery. Optional official logout may contact server; shared/external/unverifiable sources block.
5. Rebuild/retire need ≥1 work category/passphrase; empty selection blocks. Repair no archive required. Preview exact handled/retained/irreversible scope then approve.
6. Read steps/local/server/archive/new-root. Selected local completion can be main success while coverage says logout not requested/server unchecked and raw status remains. Official logged-out is still unverified server token revocation.
7. Browser login separately §2.3; normal official new-root login by user.

Official claude.ai logout needs nonempty email or auth_identity_unverified. You may omit logout/local files only. Private salted full-state digest stores no email/raw output; UI only subject-observable. Before execute/after preservation/prelogout rechecks auth/files: account/organization/any observed field changes stale_auth, credentials/new fallback stale_plan. Completed artifacts stay. Old unattempted bindings need preview, accepted originals query without logout replay. Status consistency is not provenance/Keychain-only same-account token generation; last-check/logout has no complete external-writer atomic guarantee. Real Keychain/fallback/revocation separately accepted.

Order: state/work backups → rebuild create/import → chosen logout/verification → exact removals → retirement entry disable. Late logout failure retains archive/root/completed steps/code/phase/recovery and stops deletion. Original query never recreates/logs out. Changed preview objects/program/auth require repreview; uncertain accepted IDs query first. New/reappearing state stays and stops actions. Quarantine paths remain for inspection, never overwrite replacement files to restore.

Settings restore separately; deleted credentials/browser data no undo. Reactivate restores registry/launch eligibility, not credentials; login again if needed. Paused service independently restores, never on window/cleanup completion. Full Desktop/IDE/macOS-service/other-supervisor discovery/stop is incomplete; independently inspect actual chain.

<a id="remote"></a>
## 5. Manage VPS and services from Mac

### 5.1 Connect the correct SSH user

Hosts and SSH → prerequisites: existing alias/trusted key/OpenSSH-agent keys; BatchMode has no password dialog.

1. Register alias such as writing-server, never user@host/commands/options.
2. Existing PATH runner connects; Claude alone cannot.
3. Check/prepare runner, review OS/CPU/user/version/bytes/details.
4. Approve bundled upload, verify bytes/permissions/capabilities → ready.
5. Connect, then select/register that user's config root.

Scope is **current SSH user**; root does not manage all users automatically. Linux x86_64/arm64 resources, missing conditions explicit. No PATH/rc/sshd/Claude/compile/sudo changes. Old verified/superseded records never switch newer binding; verified/unbound legacy needs new preview/approval.

Interruptions reconcile original install/same file without upload, including direct error action. Earlier second previews still blocked by unresolved original; removing/reregistering cannot bypass. Updated aliases leave old tasks pinned, legacy unbound PATH remains. Connection errors phase/reason/stderr/manual summary, then explicit recheck. Unknown/changed keys trusted system process first. Recovered connection never reexecutes accepted original.

### 5.2 Execute, query and open sessions

Same UI uses remote-generated plans/alias-visible approvals. Durable acceptance then background work; accepted is not completion. Receipt shows actual systemd task or unguaranteed setsid, no reboot survival.

After disconnect/reopen: host durable records → original alias/ID query → full receipt/recovery → inspect accepted/running/uncertain original until concrete steps → separately approve settings/service restoration. Result query uses reconnect; manual IDs/CLI job use readonly lookup preserving existing pins. Unknown lookups create no intent/consume no submission; legacy PATH stays, uncertain never erased/replayed. Remove registry retains original tasks/config/files, undo reregisters; full restoration needs reregister, not cancellation.

Open Claude from Launch and sources/terminal-state receipt asks Terminal strict same alias/pin/PTY without prompt. **Interactive sessions end with SSH**, unlike jobs; verify actual Terminal. Isolated OpenSSH/VM confirms eligible existing system/user logout continuation and reboot query; ineligible setsid limited. Production VPS/aarch64/all managers unaccepted. No automatic linger/login-policy/second launcher.

### 5.3 Pause one exact bound systemd service

Cleanup repair/rebuild/retire exposes Target background service, not preservation-only.

1. Current-user manager, or system only already-root.
2. Full unit e.g. example.service, readonly inspect, no glob/commands.
3. Check root/restart/MainPID/triggers.
4. Preview exact unit/original state/persistent hold, approve.
5. Verify then return cleanup; still stop terminals/IDE/other writers.

Hold prevents starts with actual state/cgroup readback, persistent until separate restore across App/disconnect/window close. Finite exact bindings only; unknown/neighbor-impact/unsupported sources refuse without widening. Restore original receipt or inspect: remove only original hold, start only originally active, retain inactive. Outside conflicts preserve configuration/blocker, never erase records. New root does not rebind old service; restore remains original. Choose old/new service explicitly; automated rebuild/rebind/new-root acceptance loop is incomplete.

<a id="recovery"></a>
## 6. Drift, restoration and result recovery

### 6.1 Manual drift checks

Environment details → Changes since last check → Check changes compares managed baseline. Reapply policy re-previews/approves; Accept current updates baseline without rewrites. No differences means inspected scope only, not whole machine/browser/network. Background drift/menu-bar/automatic rules are not enabled; recheck after upgrades/entry switches/external edits.

### 6.2 Restore by original task

Pending collects running/interrupted/incomplete tasks. Settings publication proof gates restoration: confirmed original write offers preview, absent/unproven retains file, no automatic restore. Next steps follow actual receipt. Filters all/current/pending and search remain current host, then view steps/actions.

| Restore | Entry | Boundary |
| --- | --- | --- |
| Edited/deleted settings | Original restore preview | Owned fields/later edits protected |
| Work | Archive unlock/import preview | Categories/target/no overwrite, no credentials/config |
| Service | Original pause restore | Exact unit/owned hold/original state |
| Retired launch eligibility | Reactivate environment | Registry only, no deleted credentials |
| Browser reversible settings | Restore preview/extension approval | Current controller/value/owned changes |
| Cookies/deleted data/logout | No generic undo | Normal login, no restoration promises |

### 6.3 Interpret states

| State | Meaning | Next |
| --- | --- | --- |
| Configured/read back | Exact values handled/read | Fresh-launch requirements, no global runtime inference |
| Accepted | Original durable target task | Query completion |
| Running/verifying | Execution/verification | Keep/query ID |
| Completed | Listed steps finished | Scope/limits/start requirements |
| Partial | Requested step incomplete or historical status | Separate goal/coverage/raw status; retained old login does not fail preservation |
| Incomplete | At least some failure | Completed steps/errors/recoverable parts |
| Needs reconciliation | Request/effects unknown | Original query, no duplicate cleanup |
| Awaiting browser restart | Preparation only | Full restart/separate deletion approval |
| Launch requested | Terminal accepted | Verify real launch/login |

Some raw labels remain English; color is not proof. Latest result queries original. Closed modal can recover by original plan/job ID in interruption lookup; remote reopen §5.2. Launch records are separate, query ID/list without Terminal replay; intent survives panels. Agent launch query/remote launch query retains original.

### 6.4 Support data

Preview support data locally before copying/saving JSON. Allowlisted platform/version/count/capability only, without environment names/directories/config/tokens/sessions; user decides sharing, no upload. SSH summary separate manual action. Help offers GitHub/source/VPS 101/SSH guides. Themes/Clawd/play/album/game never execute environment work.

### 6.5 Manage local data, identity and configuration, then run a recipe

This section folds the individual goals into one ordinary journey: choose an environment → turn off unwanted content recording → preserve and clear selected residue → execute and review → continue launching. A normal same-target task gets one complete preview and one approval; backups, edits and verification in between are done by the tool.

**Local data and identity:** Environment details → Local state → Local data and identity lists work content, runtime residue, identity material and plugin configuration in the known-location table plus up to eight explicitly added absolute directories, reading only path, category, size and mtime and filtering/paging by class, project fragment, time and size. Only clearable runtime residue is selected by default; work content is user-chosen, `unclassified` locations are not deleted by default, and identity locations report presence only and are not a clear target. Selected items support Clear exactly or Archive and clear, the latter sealing an independent `lintel.residue/1` encrypted backup first. A selected original that belongs to work content needs the explicit "allow selecting work originals for deletion", or switch to work preservation. Identity is masked by default; "Show raw value" is what reads `userID` into this page's memory, never to localStorage, ordinary snapshots, support exports or logs. "Reset local client identity" encrypts the complete original client state and the backups you select, then removes `userID` and keeps every other key value-by-value, without logging out or clearing credentials. An existing `lintel.residue/1` backup opens through "Read and restore residue backups", restoring only to the recorded original paths without overwriting an existing file, never as a work import.

**Optional: one plan, one approval.** The local-data panel can also check "reduce body recording" and "reset local client identity", folding the content-recording change, the identity reset and the selected residue into a single `client_workflow` list; one file cannot belong to the config, identity and residue steps at once. Execution is still one preview and one approval, and each step's completion or skip is recorded separately.

**Changes and comparison:** Environment details → Changes and comparison saves a sanitized semantic snapshot and compares two snapshots or two registered environments, filterable by added/removed/changed. Accepting selected current values only forms a comparison baseline, not a restoration source; restoring original values still uses the original task or an encrypted backup. "Apply ordinary configuration to the target environment" copies only recognized ordinary fields and shows the target diff; login credentials and local identity are not carried by an ordinary configuration plan. The lightweight change check reads finite configuration sources only when an environment opens or a Lintel launch happens, leaving a tip for a real change or an incomplete read, with no resident watcher.

**Advanced configuration editing:** For an explicitly chosen user/project-local JSON file it offers strict validation, a per-field masked diff and original-task restoration. Writes reuse the existing plan chain: the digest is rechecked before approval, an age `lintel.config/1` backup is sealed before writing, then the file is read back; restoration changes only the original task's fields and retains unrelated later edits, and credential values (such as `OTEL_EXPORTER_OTLP_HEADERS` or a credential-bearing receiver URL) and unrecognized keys stay masked by default. Organization-enforced configuration retains its permission boundary.

**Quick recipes:** The Protect plan and Cleanup and rebuild pages offer three editable recipes. "Reduce body recording" disables the known content-recording fields while keeping metrics; "clear runtime residue" uses the residue currently selected in the local-data view; "preserve work and rebuild" archives and imports the selected work content. Recipes store only non-secret intent such as the preset and finite field choices; drafts and original tasks do not drift across host or environment switches, and every run regenerates a fresh plan for the current host and environment, never replaying an old approval.

## Hand off from App to Agent

Terminal and Agent/environment/plan/receipt handoffs retain exact target. Select full CLI → check context → review → copy. Fixed static argv/bounded time-output/no shell/personal discovery/state/Claude. Check product/protocol/platform/candidate/App-vs-CLI HOME/state/UID/euid. Missing sidecar unknown, checksum not provenance. Original objects require original App state/user/IDs/pins, no migration/PATH changes. Independent CLI state only when no original object.

Handoff contains goal/target/executor/state/original ID/hash/observations/exact query. Accepted/uncertain query only, no new submit. No passphrase/work/full state/credentials; copying grants no authority. Work context separately §3.5. [Agent guide](agents.en.md). Bundled short help shows version/docs revision; clean full-doc links pin commit, dirty/unknown latest-development label.

## Terminal-only CLI

No GUI/Tauri requirement; [Agent guide](agents.en.md) owns package/source/version/schema/secret/approval/wait/restoration/browser/SSH/network. Candidates explicitly selected, without automatic update/formal signing:

```sh
lintel version --json
lintel capabilities --json
lintel describe plan_policy --json
lintel env list --json
lintel policy plan --environment <environment-id> --preset reduce --no-keep-remote-control
lintel plan show <returned-plan-id>
lintel job submit --plan <returned-plan-id> --approval <returned-plan-hash>
lintel job wait <returned-plan-id> --timeout 30s
lintel restore plan --job <original-job-id>
```

Static version/capabilities/schema/describe discovers/runs/initializes nothing. env list/discover may register/save, shortcut/raw same capability. Ordinary stdout one envelope/nonzero refusal; ACK needs status query. Accepted errors retain ID/code, never replay reason. wait_timeout retains plan/no resend. network serve foreground NDJSON independent App. Legacy request/submit/inspect/jobs/job/tui/launch stays; TUI apply explicit, passphrase no-echo TTY, launch exact TTY/no prompt/not mutation. No --yes/restore --all.

## Reading evidence

These are connected source paths. The table reports repository evidence, **not freshly rerun checks at this reviewed HEAD**:

| Scope | Recorded proof | Cannot infer |
| --- | --- | --- |
| Config/archive/cleanup/restore | Real synthetic core/CLI/conflicts/encryption/TUI | Real login/Keychain/all Desktop/IDE |
| Browser | Independent Chromium/native persistence/startup/synthetic sites/Native Messaging | Formal browsers/AdsPower/real auth/non-developer installation |
| Network | Real loopback rules/stop/native readback | Claude public effects/no direct bypass/process enforcement |
| Remote | Isolated x86_64 SSH installation/query/PTY, real VM services/eligible PAM continuation/reboot reconciliation | Production/aarch64/all user managers |
| UI | Built pages/real core-host with synthetic invoke/SSH/clipboard, historical native observations | All latest WebKit/Finder/Terminal/accounts |

Full targets still include more auth/entry adapters, system enforcement, persistent/dedicated browsers, service migration loop, timed drift/menu bar, upgrade/uninstall/signed distribution. Removing them from guides does not complete the product.

migration_probe path/status/executing migration_preflight persists before temp creation. Confirmed cleanup removed; retained stops bodies. Interrupted executing remains original path, query never deletes/reruns. App shows pending directory; only own empty files, extras retained, not config root.

## Telemetry destinations and App updates

Egress draft selects exact official-catalog telemetry targets into existing blocked rules, then explicit start. Tests identify Lintel controlled requests; no observed Claude traffic is not zero telemetry. Fine switches/NONESSENTIAL update/flag effects separately checked; [telemetry](telemetry.en.md).

Settings → Lintel version and updates manages this App. Background off, errors not up-to-date, verified download/version then reviewed install/saved-work explicit restart. Native rejects active operations/proxies. Interrupted original records never replay; old App needs manual update once. Source has no formal public key/feed/Release activation; [preparation/order](app-updates.en.md). CLI/browser/runners are not auto-installed by updater.

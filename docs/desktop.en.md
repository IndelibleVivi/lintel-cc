# Desktop usage and interface conventions

[简体中文 (canonical)](desktop.md) · English · [Documentation](index.en.md)

Lintel desktop uses Tauri 2 + React. `src/App.tsx` manages host/environment selection, drafts, plans and receipts; `src/api.ts` sends the same typed core requests locally or to the selected SSH host. Browser/local network channels use their own finite native commands. The UI never fabricates logged-out, protection-success or network-enforcement states.

The [operator guide](operator-guide.en.md) covers goals through results. App help provides six task paths, scenario guides and [Agent CLI](agents.en.md). Links open only fixed named HTTPS documents.

## Everyday use

1. Confirm host/environment at the top. Home is a quiet companion space; Start a task opens six goals. The current goal remains visible; empty environments retain host/create/register/browser/CLI entries. Preview in the professional workspace without model calls. Protect plan controls feature tradeoffs, statically identifies installed versions, explains variables and Remote Control version/Trusted Devices conditions. Organization conditions are user declarations, never account reads; unknowns prove no availability. Conflicting existing values require explicit removal selection and enter the exact approved diff.
2. Compare changes, preserved items and actual steps before one approval. Results distinguish completed, partial, accepted and reconciliation; field readback does not verify runtime effects.
3. Environment details opens overview, egress/permissions, local state, launch/sources, components and check time. Home/side Work preservation share one task. Sessions and materials has an independent reader; Terminal and Agent checks the executor and hands off operation metadata. Sidebar opens protect, cleanup and records. View/copy full paths; manual refresh rereads state. Drift lists differences and permits reapplying/accepting baseline without automatic cleanup.
4. Records and recovery retains real receipts, filtered by environment/text. Restoration checks subsequent edits. On interruption query the original ID; absent response does not mean unexecuted.
5. Preview redacted support data locally, then copy/save JSON. No automatic upload.

Drafts are host/environment-separated in current-Webview localStorage; editing does not directly publish configuration. The owning host's core retains tasks/restoration after window closure. App exit stops its local proxies, without closing launched clients.

The main toolbar row keeps the host, environment, language and three appearance icons visible. A minimal orange `>_` before the host selector has no surrounding box or extra arrow; its cursor is still by default. It expands Current target downward in normal flow, moving content below it without a floating card. Details include host, full environment name, root, environment ID and detected client path, with path-copy controls. Paths are collapsed by default. The icon, Close or Escape collapses the details; Escape returns focus to the icon, while an open modal receives Escape first. Outside interaction and focus leaving retain the disclosure to avoid unexpected reflow; changing host or environment collapses it. Appearance → Motion effects independently controls interface animation, logo blink and terminal blink. Animation and logo blink default on; terminal blink defaults off. Choices stay on this device. System reduced motion overrides all three; turning off interface animation retains separate cursor choices. Dragging and games you start retain controls, with static runner scenery and no particles or decorative gait. Disclosure only changes presentation; approval previews still show the complete frozen target and destinations.

### Interface language

App supports Simplified Chinese/English and follows system by default. The top selector shows actual `EN`/`中文` and an arrow; its native menu exposes Follow system/Simplified Chinese/English, avoiding confusion with appearance System. Follow-system still shows actual language; Settings shows full mode/labels/explanation with visible focus. Both selectors retain manual choice only in this Webview's local storage. Native `app_preferred_languages` readonly fetches at most eight bounded CoreFoundation identifiers; Info.plist declares `en`/`zh-Hans`. Browser/synthetic runs fall back to browser languages; unsupported languages resolve to English. `app_set_language` accepts only `en`/`zh-CN` and updates ordinary macOS menus without changing actions/shortcuts. No Claude/account/work reads or system-language edits occur.

Switching immediately updates UI/`html.lang`, retaining page, target, drafts, inputs and frozen plan/hash without submission/approval. Clawd keeps the greeting selected for this opening and its translation, without rerandomizing/date-event reconsumption. Dates/numbers follow locale. Bundled translations need no network. `i18n-presentation.ts` displays verified core titles/steps/preservation/capabilities; unknown/old-runner text stays raw, original plans/receipts unchanged. UI errors retain keys rendered in current locale. User names, paths, commands, settings, task IDs, work text and raw diagnostics are never translated. CLI, popup, website and external docs have independent language contracts; [current state (Chinese)](current-state.md) records candidate inclusion.

### Custom protection

Custom in Protect plan groups the 27 fields recognized by the v5 rule into privacy/outbound, content recording, OTel/destinations, and identity/features. Each row offers its action directly and distinguishes the environment configuration from pending review. Ordinary editing has no bulk-selection checkboxes. Search, filter by group or changes only, undo one edit or discard all. Preview is disabled when there are no pending edits or a required value is missing. Details & original value reveals exact values, sources and destinations; rule details hold full semantics, defaults, version evidence and dependencies. Current readback and recipes/personal defaults have separate disclosures rather than duplicating the editor.

Enter Bulk edit, select individual settings or Select visible settings, choose a common applicable action and Update N draft settings. Filtering preserves other selections and explicitly counts those outside the results. The whole selection updates one draft for the next review. Finish selecting exits selection mode without discarding applied draft edits; individual undo and discard-all remain available.

- **Leave unchanged:** preserve the original field. App does not enable an empty review when everything is unchanged.
- **Enable/disable:** describe the feature or signal, not a universal meaning of `0`/`false`. DISABLE_* fields disable with `"1"` and enable by deleting the disabling value; positive Booleans disable with `"0"` and enable with `"1"`; default_on fields disable with `"false"` and enable by restoring absence; exporters disable with `"none"`. WebFetch uses a positive tool name.
- **Remove this environment’s override:** delete only the exact user-settings field. Other sources/defaults may take effect; removal does not guarantee a feature is off. Addresses, headers and the length limit have no separate off spelling, so the UI offers one removal action. Equivalent old draft actions remain compatible.
- **Enter a value:** addresses, numbers, exporters and advanced raw variables have appropriate inputs and inline errors. New `OTEL_EXPORTER_OTLP_HEADERS` values are not accepted. An existing credential-bearing endpoint stays masked, but a blank input permits replacement by a plain address. New addresses containing credentials or query material require private advanced editing under Environment details → Changes and comparison, outside ordinary plan output.

Raw API body recording directly offers off, inline recording or a local directory. Directory mode asks only for the full path and generates the core’s `file:<dir>` value; inline mode needs no implementation string. Their destinations differ; full originals/rules remain available. A local file does not establish upload.

Preview shows exact changes and expected Remote Control conditions. The retention option under Remote Control & version conditions never overrides explicit custom choices. Removing/replacing masked content requires a passphrase and encrypted preservation; restoration retains original-task/selected-field semantics. Preset switching preserves the custom draft, with separate host/environment scopes. Personal defaults save only supported ordinary intent/finite values, excluding addresses, paths and credential values. Omitted fields are named without replacing the live draft, and storage failure is reported. Applying a default still makes a fresh plan. Old runners without `supported_presets: custom` preserve drafts but disable custom review; recheck after updating. Rule upgrades require new pending previews while original jobs/restoration remain.

Local/SSH share the form; remote uses fixed JSON/durable submit. WebFetch tool control now enters the catalog as `CLAUDE_CODE_DISABLE_WEB_FETCH`; official updates and marketplace automatic installation remain outside this editor; see [core scope](core.en.md).

## Cleanup, archives and import

Components reads finite sources independently even with damaged settings, showing hook/MCP declarations without commands/credentials. Original records/current observations are separate. Host switches discard late results; missing old-runner operations show errors. Capacity preflight refreshes with categories; blockers disable preservation preview until selection is narrowed/rechecked, without deletion/truncation.

Work preservation offers encrypted archive only or preservation/new root. Both succeed against their own goal, retaining old login. Archives open from tasks or independent absolute paths, without source-state dependency; users explicitly transfer ciphertext using system tools. See [work workflow](operator-guide.en.md#work).

`WorkFileSelection.tsx` provides project-path/session-file groups and individual files, loads bounded metadata pages and refuses missing/changed pages without transcript reads. Search affects display only; oversized files can be excluded explicitly. Selected paths are not persisted in drafts, new files never silently selected. `WorkCapacity` preflights the same selection; `PlanReview` shows originals/frozen destinations. Without confirmed exact-selection support, approval is unavailable; old-runner errors never silently fall back to categories.

Cleanup has three paths:

| Path | Current scope |
| --- | --- |
| Repair local login | Verify writers stopped, handle previewed local credential files; official logout optional. |
| Clean and rebuild | Archive work/mixed client state, create new root/import, then selected official logout and exact removals. |
| Retire environment | Handle exact state scope and disable this environment's launch entry; original work root remains. Reenable never restores deleted credentials. |

All require confirming target terminals/IDEs/automatic restarters closed. Core rechecks writers; unassignable Claude writers block cleanup without global killing. UI shows files/existence/coverage, never credential bodies.

Recognized local scope first shows category, filename and presence; expand a file row for its exact full path, or Refresh scope to observe again. Session launch records accepts an original launch/resume request ID for a read-only result check, without starting another session or copying work content.

Only Explicitly inspect authentication sources runs target Claude auth status. Official logout invokes that environment's `auth logout` and may contact servers; unverifiable shared auth/schema refuses. Without logout, Keychain/outside-root auth remains unknown; file completion is not full logout. Browser/Desktop/IDE/supervisor scopes remain separate.

Archives require passphrases ≥12 characters, temporarily in form/request, never drafts/localStorage/journal. Records → Work archive or result View work archive unlocks inventory and paged structured messages/tools/unknown records/raw text. Category import needs a separate plan/approval and refuses same-name overwrite. Default materials enter references; explicitly enabling reviewed personal instructions chooses client-discovered instruction paths. Sessions/memories enter `lintel-imports`, without hooks/MCP activation or continuation claims.

Launch/resume share source checks: root, explicit cwd/bounded ancestors, managed files/drop-ins and hook/MCP/helper/plugin declarations. Present/restricted sources are visible; checked-absent entries expand. Old runners lacking projection cannot approve new App launch. Auth/shared-profile metadata only; Keychain/account/Terminal shell/worktree main checkout/actual loading remain unverified. Changes require new previews; original attempts are query-only. Archive/import never auto-launch/login.

Launch errors retain the original ID for query. Only a core-read unattempted original plan permits rechecking launch target: retain draft/cwd, create new ID and withdraw old review. Existing intent/results remain query-only.

## Local data, identity, changes and recipes

Local state → Local data and identity lists work content, runtime residue, identity/login material and plugin configuration through a finite known-location table plus up to eight explicitly added absolute directories, reading only path, category, size and mtime. It filters by class, project fragment, time and size and pages over one scan; only clearable runtime residue is selected by default, work content is user-chosen, `unclassified` locations are not deleted by default, and identity locations report presence only and are never a clear target. Selected items support Clear exactly or Archive and clear, the latter writing an independent `lintel.residue/1` encrypted backup before removal. A composed plan can fold "reduce body recording", "reset local client identity" and the selected residue into one plan and one approval.

Identity and login material recognizes only the verified `userID` and account trace `oauthAccount`, showing presence by default; "Show raw value" is what reads `userID` into this page's memory, and even then it is never written to localStorage, ordinary snapshots, support exports or logs. "Reset local client identity" encrypts the complete original client state and the backups you explicitly select, then removes `userID` and keeps every other key value-by-value; it never logs out or clears credentials, and whether the client regenerates the identifier is shown from a recheck. An existing `lintel.residue/1` backup opens through "Read and restore residue backups", restores only to its original recorded paths, refuses to overwrite an existing file and is never a work import.

Changes and comparison saves a sanitized semantic snapshot and compares two snapshots or two registered environments, filterable by added/removed/changed; "accept the selected current values" only forms a comparison baseline, not a restoration source. Cross-environment "apply ordinary configuration to the target environment" copies only recognized ordinary fields and shows the target diff, never login credentials or local identity. The lightweight change check reads finite configuration sources only when an environment opens or a Lintel launch happens, leaving a tip for a real change or an incomplete read, with no resident watcher.

Advanced configuration editing offers strict validation, a per-field masked diff and original-task restoration for an explicitly chosen user/project-local JSON file: writes reuse the existing plan chain, recheck the digest before approval, seal an age `lintel.config/1` backup before writing, and restore only the original task's fields while retaining unrelated later edits; credential values and unrecognized keys stay masked by default.

The Protect plan and Cleanup and rebuild pages hold three editable quick recipes: "reduce body recording" disables the known content-recording fields while keeping metrics, "clear runtime residue" uses the residue currently selected in the local-data view, and "preserve work and rebuild" archives and imports the selected work content. Recipes store only non-secret intent such as the preset and finite field choices; drafts and original tasks do not drift across host or environment switches, and every run regenerates a fresh plan for the current host and environment, never replaying an old approval.

Each row groups its name, description, target and option summary, with one Apply recipe action on the right. Edit and Save personal default sit in a secondary action row. When the personal default saved for a host/environment differs from the built-in preset (for example, enabling content recording in the "reduce body recording" slot), the row instead shows "Personal default · preset name" plus a description generated from the actual intent, instead of the original preset promise; the built-in name returns when the intent matches or no intent is saved. The current page's target has a quiet emphasis; other targets remain available and open their corresponding page. `Recipes.css` is the single styling entry for this layout.

## Browser and network

Browser workspace from home/cleanup offers pairing, WebRTC, clear, permissions, site proxy/blocking, temporary pause and separate whole-profile HTTP cache clear. Unsupported Firefox permissions/proxy/CacheStorage display limits.

New-profile connection has three steps. Choose browser, preview version/file scope/current-user directory, approve fixed-directory preparation, then load it following adjacent instructions. App carries developer packages requiring no source/build. Chrome/Edge share Chromium directory; Firefox temporarily loads manifest. Owned updates keep paths and require browser reload; outside/edited files refuse overwrite, browser changes withdraw preview. Enter exact extension ID and separately preview/approve local connection. Bundled host preview includes version, bytes, stable storage, registration/authorized ID, expandable manifest/SHA-256. Installation uses current-user fixed Lintel storage independent of App location. Matching registrations can be checked; installer-owned updates are reviewed, other conflicts preserved. Whole-plan recheck precedes writes.

Installation establishes component/ID authorization, not pairing/online. Create request, copy 12-character code to extension, compare codes and separately approve in App. Missing resources show precise limits. Extensions remain unsigned developer installations. Independent Chromium persistent-install/full-restart clear passed; formal Chrome/Edge/Firefox/AdsPower/non-developer journeys remain unaccepted. See [browser guide](browser.en.md).

Desktop submission prepares proposals; target extension still reviews/approves. Clear isolates requests, closes relevant pages and unregisters SWs, then waits for full browser restart and second approval before storage deletion. Deleted data is irrecoverable; isolation release is separate. Reversible WebRTC/permission/proxy/blocking settings have restoration previews. Persist operation IDs before submission; lost replies query originals without automatic new IDs.

Local Egress and permissions shows adopted default action/rules/ports/HTTP(S) upstream, channel start/stop/launch and real event refresh. Native reads back actual config. Next-start drafts remain per-environment Webview memory; stop then edit/readback then explicitly start. Old responses cannot cross environments. No system-proxy edits/TLS decryption/all direct sockets/DNS/UDP/other-process coverage. SSH workspace never labels local proxy as remote protection.

### IPv4 / IPv6 and shared networking

`IpNetworkPanel` offers four-cell tests, IP copy, bounded refresh, baseline validity, macOS service preview and original-task restoration. `NetworkResults`/`NetworkEvidence` share plan/receipt display. Home without environment opens the same panel/executor. `NetworkState::dispatch_core` binds previews/probes to actual App channel instance; even a new same-port instance needs preview again. Original queries/restoration survive channel exit. Full configuration/auto-reprobe uses normal approval; `Flow.environment:null` acquires no Claude root or launch action. Exit stops channel without reversing host IPv6. Linux SSH readonly tests execute remotely, never Mac probes. Synthetic bridge refuses real host actions; `network-ui` config/echo/invoke are synthetic, not WebKit/OS-write acceptance.

## SSH workspace

Host selector/Settings reads system Host aliases and saves connection references; user initiates connection. It needs **Lintel runner**, not merely Claude Code; missing Linux runners can be previewed/approved below. Preparation/SSH guidance is available. Register/connect never installs/updates software or edits SSH configuration, and collects no passwords/keys. Resolve strict host-key failure using system SSH first.

Remove/Undo remove changes only Lintel registry, never `~/.ssh/config`, known_hosts or remote files. Removing current host returns local; durable submissions remain queryable. Reregister never resubmits. Failed removal retains entry/reason.

Connection errors show alias, phase, reason, suggestions and exit code. Expand bounded stderr only for requests without passphrases/approvals; raw errors never enter journals. Copy troubleshooting summary excludes stderr to avoid copying embedded hosts/paths. Recheck is explicit readonly connectivity/environment retrieval, never automatic mutation retry.

Connected environments/plans/cleanup/archives/records belong to that host in the same UI. Execute uses fixed `lintel submit`, separating acceptance/completion. Latest result and durable host records query originals. On App reopen query original task → full receipt/recovery, readonly check original environment and enter its host result/records. Settings/service restoration still need separate approval. Removed aliases allow original queries only; reregister before restoration. Late replies cannot reopen closed flows/cross hosts. Receipt shows actual system/user transient service or setsid with limits/unit details. Approval explains conditions; old receipts never invent supervision. Worker survival depends on OS/manager lifecycle, separately from synthetic/fake-SSH proof; see [remote](remote.en.md).

Drift results/errors/waits/Accept current values bind to each host/environment check. Switching discards late replies; returning cannot revive old checks. Acceptance applies only to the displayed target.

Browser pairing/local channels stay on the desktop host, never move with SSH. Environment Launch and sources and completed receipts offer Open Claude. Remote `plan_launch` freezes root/independent cwd/executable; approval asks macOS Terminal to use the same strict alias, pinned runner and real PTY for `launch_request ID HASH`. Resume uses separate `plan_resume`/`resume_request` with passphrase reentered without echo in Terminal. Controller records one attempt first; lost replies query original launch. Legacy `launch ENV_ID` retains root-as-cwd. Remote core rechecks identity; App injects no prompt. SSH loss ends interactive sessions, unlike detached jobs. `launch_requested` means Terminal accepted; actual login/runtime is checked there. accepted/executing/verifying task receipts keep launch disabled until original query reaches terminal state. Failures scroll into view.

## Help and developer information

Offline sidebar help has six goals, goal/preview/approval/original-query steps, reader/CLI handoff and fixed Faye GitHub/Lintel/ [Infra Field Guide](https://github.com/IndelibleVivi/infra-field-guide) VPS 101/SSH troubleshooting links. The independent project provides source/usage/current acceptance. Frontend/native share `contracts/documentation-resources.json`; `resources.rs` opens only finite resource-ID HTTPS links. Clean builds pin only Lintel docs to the same revision; dirty/unknown builds label latest development docs. Independent guides retain their own main/latest-guide label. No arbitrary URLs or attached configuration/errors/credentials. Web preview uses ordinary links; App uses system browser. Failures preserve copyable links.

## Remote runners

Check and prepare runner supports Linux x86_64/arm64. The card shows target/SSH user/bytes/status/primary action, with full path/SHA-256/impact expandable. UID 0 is root; use the alias of the user running Claude. Only approval uploads App files; executable/capability checks precede Connect and manage. Lost replies become reconciliation of the same persistent installation even after panel close. PATH runners still connect directly. Scope/resources/runtime limits: [remote](remote.en.md).

## Appearance and local fonts

Day/Night/System is remembered in this Webview. CSS gradients supply fixed ambient morning light/green or moonlight/static stars without images/content scrolling. Home centers Clawd, original time-based greetings and space, with explicit task expansion, glow/soft shadow withdrawn during hiding/flight. Choices use two columns; default home retains last-goal/reconciliation-count entries. Professional editing uses continuous groups; cards/plans/records/modals share soft shadows/fine lines/visible focus.

Character taps reveal hearts, tea, flowers, books, naps, birthday hats, stars, companions, moon/snacks. Hold hugs, drag lifts, fast release flings; three rapid pokes show `><`, six hide leaving clickable feet, or coax back in pocket. Play opens tea/jump/moon/rest/play; moon response resets on interaction/Escape, focused Up jumps. No models/config changes. Blink/float/flight/jump respect reduced motion.

At home top, pull blank space down past threshold or overscroll upward to lift Clawd/open the pocket menu. Insufficient/cancelled gestures/Escape retract. Play also supports click/keyboard; menu floats near character without moving workspace. Pocket opens interactions/album/game. Closing returns focus to Play. Selection/forms/details cannot initiate pull-down.

The album has four original character-grid landscapes: moonlit mountain lake, rainy cabin, coastal lighthouse, starfield campfire. Character shades draw cloud/mountain/water/buildings; separate orange Clawd, eyes and four single-row short feet preserve silhouette. Whole art scales proportionally. Collect stars; album has independent warm/night themes. [Claude Code night-scene reference](https://miro.medium.com/v2/resize%3Afit%3A1358/format%3Awebp/1%2AgcjPv1ITZ6YpPwlMmveADA.png) informs negative space/layers/relationships; local code draws the art without distributing it. Pull-menu implementation makes no frame-perfect official-mobile claim.

The game shares one implementation with the website. Clawd is one continuous square horizontal body with four short straight attached legs, without separate feet. Running stays level; leg pairs alternate while attached, without horizontal toes. Jumping stretches the body and landing briefly compresses it. In current source, pressing the track or Jump takes off immediately. Short taps keep the original height; holding adds lift during that same ascent and release only ends the lift. Space/Up work the same way. The lift window caps at 0.26 seconds; holding cannot repeat a jump on landing. Ordinary stars follow the tap arc with small height variation, moving upward with taller obstacles so a normal clearance can collect them (+25 each); every fourth star is larger and higher (+50), requiring a higher jump. The track keeps the original monochrome pixel/ASCII-like hills, clouds and sun/moon, adding forest and courtyard marks in the same theme ink. Stones, books, pots and fences vary in size; spacing shifts between closer and wider gaps, with recovery distance after taller/wider obstacles. Ordinary hops can still clear obstacles; holding reaches high stars. P/button pauses, you can restart after a collision, starting speed is 260px/s and increases continuously to 350px/s. Brief prelanding input queues the next jump. Pause, focus/window loss, hidden/offscreen state, pointer cancellation/lost capture and disposal clear pending input and held lift. Touch scrolling remains possible through pointer cancellation. Switching to the album or closing disposes listeners/animation; reopening starts one instance. Themes follow App. Reduced motion disables background/dust/leg/stretch/landing scale while retaining deliberate jumps. The App best-score key remains `lintel.clawd.runner.best`; website scores remain separately local. No network, models or external services are used.

Immediate press/held lift, high stars, three monochrome places and four obstacle families belong to the current shared source. The existing `e1a2df4` App candidate retains the earlier game until separately rebuilt. Pure logic and built-App Chromium/synthetic UI evidence do not replace native WebKit acceptance; production website state is separately recorded in [current state (Chinese)](current-state.md).

Home uses fixed SVG geometry; game shares it, album uses fixed grids independent of fonts. [Clawd reference](https://pbs.twimg.com/tweet_video_thumb/G2C8pCLaMAAy2P1.jpg) and [TUI screenshot](https://zenn.dev/tutupizizizi/articles/claude-code-clawd-mascot) inform original rendering/scenes without downloaded product assets. Clawd belongs to Anthropic; Lintel is independent.

`src/styles.css` owns layout/style, `Clawd.tsx` character/emotion, `ClawdFlightMenu.tsx` menu, `ClawdPlayroom.tsx` React album/game mounting/disposal, `clawd-landscapes.ts` four artworks. [`apps/site/clawd-game.mjs`](../apps/site/clawd-game.mjs)/CSS is the only runner, loaded directly by site/bundled by Vite; old React runner is removed. Running chest stays level with alternating ground-contact legs; tap jumps retain their ballistic arc, holding adds finite lift, and real landing settles briefly. Pause/reduced motion use static silhouette without changing play/star heights/scores. Character/album styles retain matching prefixes.

Optional local Anthropic Sans/Serif/Mono have system fallback and no remote requests. Redistribution rights are unestablished, so files stay ignored under `public/local-fonts/`. Explicit local preparation:

```sh
cd apps/desktop
node scripts/local-fonts.mjs /absolute/path/to/authorized-font-directory
```

Only six exact filenames are read; no source-project scan. Dev can use local fonts. **Default `npm run build`/`npm run desktop:build` excludes `public/` (currently only optional fonts) and font-loading lists**, preserving files and using fallback. Explicit `npm run build -- --mode local-candidate` includes them only in a local candidate, never a public distribution package. This resolves font inclusion, not incomplete signing/notarization/extension publication/distribution.

Icon master is `src/brand.svg` with five supplied PNG/ICNS/ICO exports: terracotta lintel, neutral supports, lower-right cursor. `Brand()` shares geometry/themes; sidebar `lintel_`, divider and 2.6-second blink remain, static under reduced motion. [Visual identity (Chinese)](visual-language.md) owns editable assets/boundaries. Bundled icons do not accept macOS/Linux launchers.

## Development and verification

Cleanup's Target background service takes exact `.service`, current-user/system manager (root required), readonly inspection, then explicit binding verification/preview/pause approval. Receipt or live inspection permits separate restoration; external edits block/preserve. Return to cleanup and still close terminal/IDE/other writers. This finite Linux scope never adds sudo/neighbor changes/enablement. [Service guide](services.en.md) covers sources/holds/CLI. Lost replies query originals. Synthetic manager UI is distinct from VM/VPS/WebKit evidence.

```sh
cargo build -p lintel-runner
cd apps/desktop
npm ci
npm run dev:synthetic
npm run typecheck
npm run build
npm run desktop:build
```

Prebuild generates non-fixture Chromium/Firefox resources from `extensions/browser/src` into `browser-extension-bundle` and compiles standalone host. Native matched-Rust-host builds only; cross/universal refuses. `prepare:browser-extension`/`prepare:browser-host` generates resources only; copy/register still needs App approval. Frontend build runs no installers.

Concurrent development may use `LINTEL_FIXTURE_PORT=1422 npm run dev:synthetic`, default 1420; port/Origin/Host share config. Synthetic state/discovery stays temporary with visible test-space labels; native browser/SSH/auth/network never fabricate success. New requests need allowlisting; registered paths cannot escape synthetic home.

Primary size 1120×760, minimum 900×640, collapsible sidebar. Browser renders do not replace native WebKit/hosts/formal browser acceptance. [Current state (Chinese)](current-state.md) owns incomplete installation/release/full capabilities; source integration is not release.

## Local Preview candidates

Providers supply exact revision/platform/build notes. Current candidate is Apple Silicon without formal distribution signing/notarization; download restrictions may block launch. Local startup is not ordinary-user download acceptance. Unpack into a new version directory, open Lintel.app, preserve old candidates. Bundled natives need no Rust/Node/dev server on trial machines. Use synthetic materials and [quickstart](quickstart.en.md) preview/approve/original-query. After scripted launch bring window foreground: observed temporary blank background windows recovered by Dock focus. Never delete state/resubmit; query originals after display recovery. This observation is separate from full native/Gatekeeper matrices.

Upgrade by quitting old App, retaining version/state and opening new directory; never run two versions on one task concurrently. Remove only your unpacked directory after quitting. Removal does not revert approved configuration, change Claude, delete packages or independent hosts/runners. Restore through each approved path first, retaining receipts/IDs.

## Approval and result continuation

Import approval shows frozen source-relative path, full root/final destination, category, activation purpose and bytes. Legacy import_manifest remains relative; digest/package identity expands. Long paths wrap without byte changes. Settings/file changes remain separate. Modal title/target/actions stay visible with independent body scrolling and wrapping receipt actions.

Records retain environment/search and Pending for running/interrupted/incomplete outcomes. Interrupted-settings explanation, restoration and next_steps follow actual receipts; unresolved installs query original ID. Superseded and verified-but-unbound installs differ, without implying current binding changed.

## Page/component ownership

`contracts/task-catalog.json` owns six goals/names/descriptions/actions/help anchors consumed directly by home/help/CLI. `ArchivePanel.tsx` is the only reader (index/read/draft panes); `LaunchPanel.tsx` handles cwd/review/copy/results; `AgentPanel.tsx` only finite metadata; `PlanReview.tsx` frozen destinations; `TaskOutcome.tsx` goal/coverage separate with raw receipt compatibility. `ComponentPanel.tsx` finite projection and `WorkCapacity.tsx` metadata admission bind exact targets. `ResiduePanel.tsx` owns local data and identity, identity reveal/copy, composed recipes and the Finder reveal entry; `ResidueArchivePanel.tsx` reads `lintel.residue/1` backups only; `SnapshotPanel.tsx` owns snapshot save/compare/accept and side-by-side environment comparison; `ConfigEditor.tsx` owns advanced configuration read/diff/restore; `Recipes.tsx` and `recipeIntent.ts` store only the non-secret intent of the three recipes; `local_files.rs` handles the user-clicked local Finder reveal. `taskDrafts.ts` isolates nonsecret host/environment drafts. `homeGreetings.ts` owns 56 greetings/date/minute/rare events; `main.tsx` chooses before React mount, special dates only first daily opening and localStorage retains only last shown date. `ui.tsx` `WorkspaceActions` owns the single dock outside scrolling body for protect/preserve/cleanup; help/settings do not scroll with environments.

[Visual language (Chinese)](visual-language.md) retains identity authority. Selen can refine optical spacing/typography/lines/shadow/motion; wired buttons/full states/window/keyboard semantics cannot regress into mockups. Chromium/WebKit/Terminal/client evidence is in [18 checks (Chinese)](specs/product-baseline-status.md).

## Telemetry and App updates

Egress and permissions retains IP/channel checks. [Telemetry catalog](telemetry.en.md) offers exact optional targets, separating settings readback/tests/traffic. App updates live in Settings and modules → Lintel version and updates, unconfigured by default in source with background checks off. `AppUpdates.tsx`/native `app_updates.rs` wire download verification/approved install/explicit restart; `activity.rs`/active channels block install/restart during operations. Browser/VPS runners retain independent reviewed installers. See [App updates](app-updates.en.md) and [current state (Chinese)](current-state.md).

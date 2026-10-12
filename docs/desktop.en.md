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

### Interface language

App supports Simplified Chinese/English and follows system by default. The top selector shows actual `EN`/`中文` and an arrow; its native menu exposes Follow system/Simplified Chinese/English, avoiding confusion with appearance System. Follow-system still shows actual language; Settings shows full mode/labels/explanation with visible focus. Both selectors retain manual choice only in this Webview's local storage. Native `app_preferred_languages` readonly fetches at most eight bounded CoreFoundation identifiers; Info.plist declares `en`/`zh-Hans`. Browser/synthetic runs fall back to browser languages; unsupported languages resolve to English. `app_set_language` accepts only `en`/`zh-CN` and updates ordinary macOS menus without changing actions/shortcuts. No Claude/account/work reads or system-language edits occur.

Switching immediately updates UI/`html.lang`, retaining page, target, drafts, inputs and frozen plan/hash without submission/approval. Clawd keeps the greeting selected for this opening and its translation, without rerandomizing/date-event reconsumption. Dates/numbers follow locale. Bundled translations need no network. `i18n-presentation.ts` displays verified core titles/steps/preservation/capabilities; unknown/old-runner text stays raw, original plans/receipts unchanged. UI errors retain keys rendered in current locale. User names, paths, commands, settings, task IDs, work text and raw diagnostics are never translated. CLI, popup, website and external docs have independent language contracts; [current state (Chinese)](current-state.md) records candidate inclusion.

### Custom protection

Custom in Protect plan edits seven core controls: product metrics, error reports, proactive feedback, quality surveys, `DO_NOT_TRACK`, feature flags and nonessential traffic. Each shows current value/disable semantics; Source and restoration exposes exact variable, settings path, fresh-start requirements and restoration boundary.

- **Keep current:** no add/delete/rewrite, including unselected fields. All-keep records verification without rewriting settings or permissions.
- **Disable:** uses string `"1"`; already-disabled values retain their bytes. A nonempty variable's `"false"` still disables, never enables.
- **Remove this environment override:** deletes the exact user-settings env field and may reopen traffic. Shell/project/organization sources remain influential; no feature-enabled guarantee.

Preview shows exact changes/expected Remote Control conditions. Keep Remote Control never overrides explicit custom choices; users resolve conflicts or accept impacts. Receipts retain choices, restoration checks later edits. Preset switching retains custom draft; host/environment switches never share it. Old runners without `supported_presets: custom` show update guidance/disable preview while preserving drafts; recheck after update. Policy v3 requires new previews for old pending plans; completed jobs/restoration remain.

Local/SSH share the form; remote uses fixed JSON/durable submit. Official updates, WebFetch safety prechecks and marketplace automatic installation are outside these seven controls; see [core scope](core.en.md).

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

Only Explicitly inspect authentication sources runs target Claude auth status. Official logout invokes that environment's `auth logout` and may contact servers; unverifiable shared auth/schema refuses. Without logout, Keychain/outside-root auth remains unknown; file completion is not full logout. Browser/Desktop/IDE/supervisor scopes remain separate.

Archives require passphrases ≥12 characters, temporarily in form/request, never drafts/localStorage/journal. Records → Work archive or result View work archive unlocks inventory and paged structured messages/tools/unknown records/raw text. Category import needs a separate plan/approval and refuses same-name overwrite. Default materials enter references; explicitly enabling reviewed personal instructions chooses client-discovered instruction paths. Sessions/memories enter `lintel-imports`, without hooks/MCP activation or continuation claims.

Launch/resume share source checks: root, explicit cwd/bounded ancestors, managed files/drop-ins and hook/MCP/helper/plugin declarations. Present/restricted sources are visible; checked-absent entries expand. Old runners lacking projection cannot approve new App launch. Auth/shared-profile metadata only; Keychain/account/Terminal shell/worktree main checkout/actual loading remain unverified. Changes require new previews; original attempts are query-only. Archive/import never auto-launch/login.

Launch errors retain the original ID for query. Only a core-read unattempted original plan permits rechecking launch target: retain draft/cwd, create new ID and withdraw old review. Existing intent/results remain query-only.

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

The game shares one implementation with the website. Clawd is one continuous square horizontal body with four short straight attached legs, without separate feet. Running stays level; leg pairs alternate while attached, without horizontal toes. Jumping stretches the body and landing briefly compresses it. In current source, tapping the track or Jump keeps the original jump height; holding then releasing jumps higher. Space/Up can charge too. Full charge never takes off automatically, and the road keeps moving while charging. Ordinary stars vary in height within one tap jump's reach (+25 each); every fourth star is larger and higher (+50), requiring a higher jump. P/button pauses, collisions restart, speed increases and brief prelanding input queues the next jump. Pause, focus/window loss, hidden/offscreen state, pointer cancellation/lost capture and disposal clear pending input and charge. Touch scrolling remains possible through pointer cancellation. Switching to the album or closing disposes listeners/animation; reopening starts one instance. Themes follow App. Reduced motion disables background/dust/leg/stretch/landing scale while retaining deliberate jumps. The App best-score key remains `lintel.clawd.runner.best`; website scores remain separately local. No network, models or external services are used.

Charge and high-star behavior belongs to this source change. The existing `e1a2df4` App candidate retains the earlier game until separately rebuilt. Pure logic and built-App Chromium/synthetic UI evidence do not replace native WebKit acceptance; production website state is separately recorded in [current state (Chinese)](current-state.md).

Home uses fixed SVG geometry; game shares it, album uses fixed grids independent of fonts. [Clawd reference](https://pbs.twimg.com/tweet_video_thumb/G2C8pCLaMAAy2P1.jpg) and [TUI screenshot](https://zenn.dev/tutupizizizi/articles/claude-code-clawd-mascot) inform original rendering/scenes without downloaded product assets. Clawd belongs to Anthropic; Lintel is independent.

`src/styles.css` owns layout/style, `Clawd.tsx` character/emotion, `ClawdFlightMenu.tsx` menu, `ClawdPlayroom.tsx` React album/game mounting/disposal, `clawd-landscapes.ts` four artworks. [`apps/site/clawd-game.mjs`](../apps/site/clawd-game.mjs)/CSS is the only runner, loaded directly by site/bundled by Vite; old React runner is removed. Running chest stays level with alternating ground-contact legs; jumps remain ballistic and real landing settles briefly. Pause/reduced motion use static silhouette without changing play/star heights/scores. Character/album styles retain matching prefixes.

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

`contracts/task-catalog.json` owns six goals/names/descriptions/actions/help anchors consumed directly by home/help/CLI. `ArchivePanel.tsx` is the only reader (index/read/draft panes); `LaunchPanel.tsx` handles cwd/review/copy/results; `AgentPanel.tsx` only finite metadata; `PlanReview.tsx` frozen destinations; `TaskOutcome.tsx` goal/coverage separate with raw receipt compatibility. `ComponentPanel.tsx` finite projection and `WorkCapacity.tsx` metadata admission bind exact targets. `taskDrafts.ts` isolates nonsecret host/environment drafts. `homeGreetings.ts` owns 56 greetings/date/minute/rare events; `main.tsx` chooses before React mount, special dates only first daily opening and localStorage retains only last shown date. `ui.tsx` `WorkspaceActions` owns the single dock outside scrolling body for protect/preserve/cleanup; help/settings do not scroll with environments.

[Visual language (Chinese)](visual-language.md) retains identity authority. Selen can refine optical spacing/typography/lines/shadow/motion; wired buttons/full states/window/keyboard semantics cannot regress into mockups. Chromium/WebKit/Terminal/client evidence is in [18 checks (Chinese)](specs/product-baseline-status.md).

## Telemetry and App updates

Egress and permissions retains IP/channel checks. [Telemetry catalog](telemetry.en.md) offers exact optional targets, separating settings readback/tests/traffic. App updates live in Settings and modules → Lintel version and updates, unconfigured by default in source with background checks off. `AppUpdates.tsx`/native `app_updates.rs` wire download verification/approved install/explicit restart; `activity.rs`/active channels block install/restart during operations. Browser/VPS runners retain independent reviewed installers. See [App updates](app-updates.en.md) and [current state (Chinese)](current-state.md).

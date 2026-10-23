# Browser extension and local bridge

[简体中文 (canonical)](browser.md) · English · [Documentation](index.en.md)

Status: 0.1.0 development candidate. Chrome/Edge MV3 packages and the independent Firefox event-page adapter can be built; formal browser installation, store publication and full three-browser acceptance are incomplete. This document corresponds to the [full target (Chinese)](SPEC.md#9-模块-c浏览器能力必须真实可操作) and [B01–B15 acceptance (Chinese)](ACCEPTANCE.md#c-浏览器实测), without claiming delivery of the complete target.

Pending host-installation discovery, popup isolation-release recovery and legacy receipt ownership checks below are current-source enhancements, not a newly published App or extension release. Exact source, candidate and published-version facts are separate in [current state (Chinese)](current-state.md).

Current source supports continuing an interrupted App bundled-host installation with the original approval: intent precedes effects, complete staging precedes conditional publication, and recovery verifies file identity and content. The same installer consumes registration-upgrade backups and publication journals; external replacements or edits are retained as conflicts. New publication explicitly refuses filesystems without birth-time evidence rather than weakening identity checks. Do not delete intents, registrations or originals to retry. This does not prove real browser installation or pairing. If the isolation-release marker persisted but owner removal failed, retry removes only the exactly matching old owner without repeating browser-data deletion or releasing a newer operation.

## Entry points and actual scope

The extension popup independently previews/confirms site clear, WebRTC, Chromium site permissions, site-blocking rules and Chromium site proxy. The App submits through the same native host; users still review and confirm in the target extension. Web pages have no message entry. The extension has no content script, debugger, remote code or default all-site host permission.

The default site is `https://claude.ai`; `https://console.anthropic.com` requires selection. Email, Google, Apple and other SSO sites are absent. This version compiles the site allowlist and accepts no arbitrary website-clear input.

| Capability | Chromium / Chrome / Edge adapter | Firefox adapter |
| --- | --- | --- |
| Cookies | Origins input expands to the full registrable domain, disclosed in preview | Hostnames; optional cookieStoreId |
| localStorage / IndexedDB | Exact origins | Exact hostnames, including all origins on that host; optional cookieStoreId |
| Service Workers | Native unregister before other storage deletion | Hostname unregister; cannot scope to cookieStoreId |
| CacheStorage | Native `browsingData` category | Separate site-specific removal unavailable; explicitly refused |
| HTTP cache | Can clear with origins | Site-level requests refused by default; separate whole-current-profile cache action needs new preview/confirmation |
| WebRTC | Profile-wide native BrowserSetting, ownership check/readback | Separate Firefox BrowserSetting call; actual-browser acceptance not run |
| Location/camera/microphone/notifications | Selected sites set to `block` with readback, no default blanket disable | No contentSettings API; explicit direction to browser permission panel |
| Local proxy | PAC for approved sites only, fixed `127.0.0.1` user port, no DIRECT fallback | Site proxy unavailable; never substitutes whole-profile proxy |
| DNR | Blocks all selected-site requests, separate optional permission | Separate manifest/native DNR; actual-browser acceptance not run |

WebRTC is not device-wide no-leak proof. Site proxy does not constrain DNS, WebRTC, QUIC, other programs or all browser background traffic. DNR affects all site features and does not precisely classify telemetry. No public IP/ASN/STUN/DNS diagnostic requests run.

Clear has two steps. First approval installs persistent DNR isolation including `main_frame`/`sub_frame`, requests `webNavigation` as needed, closes target top-level tabs and host tabs embedding target iframes, then unregisters selected Service Workers. Frame URLs are read only for matching this operation; page bodies/history are never saved. The first receipt is `awaiting-browser-restart` / `preparation-only`; other storage remains undeleted. Chromium's registrable-domain cookie expansion also expands isolation/tab scope. Container clear removes only the selected store's data/tabs, but DNR temporarily isolates profile-wide requests to target hosts, disclosed in preview. Store-specific Service Worker unregister is unsupported, so background rewrites remain unverified.

Isolation remains after clear. The receipt offers a separate action to release it after checking results; users then revisit manually. Automatic revisits never confuse new cookies with old residual data. Isolation belongs to preparation/deletion operations; an old receipt cannot release a newer task's isolation.

After fully quitting/restarting the browser, the extension records a new native `runtime.onStartup` generation. Preview continuation after restart and confirm the original scope again; only the new `finishClear` operation deletes storage. Worker restart does not count. Missing/changed isolation refuses continuation. Preparation is consumed once; disconnection never repeats deletion. A real negative case motivated this: an unregistered active Service Worker can finish a `waitUntil` event and recreate CacheStorage, so successful deletion APIs alone do not prove writers stopped. Lintel never quits, kills or restarts the user's browser itself.

Successful second-confirmation callbacks/Promises are `browser-acknowledged`. Firefox containers still report `container-frames-closed-service-workers-not-verified`, without claiming all background writers stopped. Container actions need optional `cookies` permission to verify store existence/identify tabs; unknown stores refuse before mutation. Without general storage enumeration, no zero-residual claim appears. If optional cookies permission is granted, only remaining counts return; cookie values never leave extension execution or enter journal. Formal packages do not request it by default; synthetic packages preauthorize it separately.

## App preparation and developer loading

The macOS App carries Chromium/Firefox companions; desktop users need neither Node/source nor host compilation. Open Browser → Connect a new profile and choose the browser:

1. Preview extension directory: readonly version, size, files and destination, then approve file preparation. Chrome/Edge share `~/Library/Application Support/Lintel/browser-extensions/chromium/`; Firefox uses sibling `firefox/`. Then reveal in Finder or copy directory/manifest path.
2. Load in the target profile. Chrome `chrome://extensions` or Edge `edge://extensions`: enable Developer mode, Load unpacked, select the prepared directory. Firefox `about:debugging#/runtime/this-firefox`: Load Temporary Add-on, select `manifest.json`. Copy the management page's exact extension ID into App step two, preview and separately approve the local connection.
3. Create pairing, copy the App's 12-character code into the target extension and submit. Compare both codes, then approve this profile in the App. Directory preparation, host registration, profile pairing and recent-poll online status are four separate facts.

After an interrupted host installation or upgrade, reopening the App discovers the original pending approval and offers “Continue original installation.” Review that original scope before continuing; a new preview or approval cannot overwrite its unresolved intent. Changed resources, external registration edits or a damaged intent produce a visible error and preserve the original state. Do not delete the intent or substitute a new plan to hide uncertainty. A completed intent whose receipt matches does not block a later upgrade. This host recovery path is separate from the extension-directory preparation flow below.

Preparation only copies bundled files, never browser profiles/preferences/load registrations/native manifests/allowlists. Stable paths survive App moves. Owned files can be reviewed/approved for update, followed by browser reload. Unowned directories, edited files and ownership conflicts refuse overwrite; the whole plan is rechecked before approval. After interrupted preparation, new preview lists the pending owned-directory switch and requires new approval; backups/staging are not completion. Changed App resource versions during a pending switch prevent recovery: inspect original resources/directory changes and retain state; never automatically continue with a new version. Identical prepared versions can be rechecked without rewrite.

This remains developer loading: Chrome/Edge need Developer mode; Firefox temporary add-ons disappear on exit and lasting installation needs signing. Store/signed distribution and non-developer first-install acceptance are incomplete. Prepared files do not prove loading. Independent Chromium has the real two-phase runtime evidence below; formal browsers/non-developer installation need separate acceptance.

### AdsPower and other Chromium browsers

AdsPower [profile documentation](https://help.adspower.com/docs/creating_browser_profiles) distinguishes Chrome-based SunBrowser and Firefox-based FlowerBrowser. Its [extension documentation](https://help.adspower.com/docs/extensions) describes Chrome Store/ZIP/managed installation. AdsPower extension management may upload/cache/sync data; assess actual settings. Lintel never uploads automatically. API compatibility does not establish Native Messaging. No verifiable AdsPower host-registration path was found in this round, and no existing AdsPower profile was operated.

The App installer has fixed paths only for Chrome, Edge and Firefox; selecting Chrome does not install into AdsPower. AdsPower Native Messaging/full restart/clear are unaccepted and it is not a supported platform. Its actual registration/call behavior needs verification in a new independent profile before an installer entry can be implemented.

## Independent builds and developer installation

Developers/independent operators can run from the repository root; App users use preparation above:

```sh
node extensions/browser/scripts/build.mjs
cargo build --release --manifest-path extensions/browser/native-host/Cargo.toml
node extensions/browser/scripts/package.mjs
```

Extension build/package needs no npm runtime dependencies; packaging uses system `zip`. Outputs:

- `extensions/browser/dist/chromium/`: Chrome/Edge Load unpacked directory.
- `extensions/browser/dist/firefox/`: independent Firefox MV3 manifest/module event page.
- `extensions/browser/artifacts/lintel-chromium-0.1.0-dev.zip` and corresponding Firefox ZIP: developer archives, neither signed nor store-published.
- `extensions/browser/native-host/target/release/lintel-browser-host`: independent host binary.

Chrome/Edge: enable Developer mode, load the Chromium directory and record the browser-reported ID. Different paths can yield different IDs. Firefox: about:debugging → This Firefox → Load Temporary Add-on → Firefox `manifest.json`. Exit removes temporary installation. Mozilla signing for persistent formal installation is incomplete. The ID is `lintel@lintel.local`; temporary loading is not non-developer delivery.

### Native host manifests and registration

**Desktop users:** Browser → Connect a new profile, choose Chrome/Edge/Firefox, enter exact extension ID and Preview local connection installation. The bundled host requires no executable path/build. Readonly preview shows version, size, digest, paths, registration and existing registration. Approval installs under current-user `~/Library/Application Support/Lintel/browser-host/<version>-<sha256>/`; the manifest points at its `lintel-browser-host` independent of App moves.

Desktop installation currently supports macOS. Matching components/registrations can be checked; installer-owned older versions can be compared/approved for upgrade, retaining old binaries. Other registrations, modified components or mismatched ownership refuse overwrite without automatic adoption. Recheck the full approved plan before writes; stale plans require new preview. It authorizes the exact extension ID, while **short-code pairing remains separate approval**; success does not mean online. Bundling does not establish extension signing/publication or two-phase acceptance.

`npm run desktop:build` builds non-fixture extensions and packages the independent host, currently only for native builds matching Rust host architecture. Cross-architecture/universal builds refuse to avoid wrong components. Development `npm run prepare:browser-extension` / `npm run prepare:browser-host` in `apps/desktop` only generates resources, never personal registration. Missing resources return specific `bundle_unavailable`. Linux standalone CLI retains manual installation below; App no longer accepts handwritten host paths.

**Independent CLI:** these manifest helpers output JSON only, without registration or personal-browser changes; replace the executable placeholder with a real reviewed absolute path:

```sh
lintel-browser-host manifest chrome EXTENSION_ID /absolute/path/to/lintel-browser-host
lintel-browser-host manifest firefox lintel@lintel.local /absolute/path/to/lintel-browser-host
```

With an executable host, preview fixed-path installation; without `--apply` there are no writes:

```sh
lintel-browser-host register chrome CHROME_EXTENSION_ID /absolute/path/to/lintel-browser-host
lintel-browser-host register edge EDGE_EXTENSION_ID /absolute/path/to/lintel-browser-host
lintel-browser-host register firefox lintel@lintel.local /absolute/path/to/lintel-browser-host
```

After review, repeat the chosen command with `--apply` to register manifest and exact ID allowlist. Identical content returns `already-registered`; differing existing manifests return `manifest_conflict` without overwrite. Missing/nonexecutable hosts or wrong IDs refuse. Installation does not pair. `--home /absolute/synthetic-home` specifies an independent test home; this round installed nothing into personal registration directories. macOS/Linux Chrome/Edge/Firefox paths have temporary-directory contract tests, separate from browser-install acceptance.

After review, explicitly install `app.lintel.browser.json` into the browser's current-user directory (`~` is the actual user's home):

| System | Chrome | Edge | Firefox |
| --- | --- | --- | --- |
| macOS | `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/` | `~/Library/Application Support/Microsoft Edge/NativeMessagingHosts/` | `~/Library/Application Support/Mozilla/NativeMessagingHosts/` |
| Linux | `~/.config/google-chrome/NativeMessagingHosts/` | `~/.config/microsoft-edge/NativeMessagingHosts/` | `~/.mozilla/native-messaging-hosts/` |

Smoke Chromium/Chrome for Testing reads `NativeMessagingHosts/` under its temporary user-data-dir. Other distributions need independent path verification, never inference for AdsPower. Automated smoke registers only under its new temporary user-data-dir with the host DB at the same temporary root, never personal directories. Profile and host registration have different scopes; pairing identity still needs independent confirmation.

Host authorizes exact IDs **only** during confirmed `register --apply` or desktop `install_bundled_host` installation/update. Generic `allow_extension` always returns `allow_extension_requires_installer`, preventing same-user processes from bypassing confirmed installation to authorize arbitrary extensions.

App installs/registers only after exact preview approval. Before uninstall, restore settings still owned by the extension and release clear isolation in the extension. Removing manifest/host makes the connection offline; browser extension removal drops DNR/content/privacy/proxy control. Deleted data is irrecoverable; bridge receipts offer no restore-everything fiction.

## Original-task continuation and unexecuted previews

App clear remains `awaiting-browser-restart` after first approval. Fully quit/restart, preview continuation in popup and approve again. Engine persists child and original final result together, then reports using the original App operation ID. Query original task in App for completion/reconciliation. `result.continuedBy` points to continuation; App finishClear uses the same path. A real `runtime.onStartup` is mandatory; worker restart does not replace it.

Previews expire after five minutes; reads/polls/execution persist `expired`. Popup cancellation marks only unexecuted previews `canceled`, without withdrawing started tasks. Terminal states return to host; new previews get new IDs, old IDs never reexecute. Host returns at most two requests per poll and rotates the whole queue by instance, so old previews cannot block later ones.

Native ACK updates latest-record metadata only under Engine serialization. Old preparation ACKs neither acknowledge changed final results nor clear `finishedBy`/`restoredBy`. DNR restoration compares sets sorted by rule ID; browser array order cannot create conflict.

If isolation rules were released but cleanup of the original owner record failed, reopening the popup still offers the release-recovery action. It reconciles only the matching owner, without repeating browsing-data deletion or removing a later operation's rules. Identity reset, commit and receipt delivery are serialized. Old journals retain their original instance and cannot execute or deliver through a newly paired identity. A legacy record without an instance is bound only after the host confirms the same ID, instance and action; otherwise it remains preserved with an explicit unknown-ownership limitation. One receipt-delivery failure does not invalidate a new pairing whose poll succeeded.

## Native bridge interface

Static CLI browser catalog/schema and host validator share lengths/identity constants: instance/operation/challenge/receipt IDs are 8–80 ASCII letters/digits/`-`/`_`. Installation accepts 32 `a`–`p` characters for Chrome/Edge, only `lintel@lintel.local` for Firefox. Manifest helpers accept `chromium`; fixed-path registration does not, so helper support implies no installation entry.

`extensions/browser/native-host` has its own Cargo workspace. Desktop can depend directly on `lintel-browser-host` without shell/subprocess:

```rust
let response = lintel_browser_host::control(serde_json::json!({"op":"instances"}));
```

Control returns `{ "ok": true, "data": ... }` or `{ "ok": false, "error": { "code": "...", "message": "..." } }`. `lintel-browser-host control` reads the same JSON on stdin and prints the same envelope.

| Input | data |
| --- | --- |
| `{"op":"bundled_extension_plan","browser":"chrome"}` | **Desktop only:** readonly `{schema,browser,package,version,sha256,bytes,files,extension_path,manifest_path,resource_path,platform,effect,existing_installation,existing_files,recovery,status,install_action}`. Chrome/Edge map to chromium, Firefox to firefox. status ready/already-installed/conflict; install_action install/upgrade/none/blocked. recovery null or previous approved preparation's `{phase,action,message,transaction,stage_path,previous_path,stage_files,stage_record,previous_installation,previous_files}`. |
| `{"op":"install_bundled_extension","approved_plan":{...}}` | **Desktop only:** approves/rechecks full plan; `{status,plan,loading:"required",pairing:"required"}`, status installed/updated/already-installed. Prepares fixed directory only; interruption cannot return success and needs new recovery preview/approval. |
| `{"op":"reveal_bundled_extension","browser":"chrome"}` | **Desktop only:** verifies complete owned fixed directory and opens Finder; `{status:"revealed",browser,package,extension_path}`. No path/URL input; no completed owned directory means refusal. |
| `{"op":"bundled_host_plan","browser":"chrome","extension_id":"EXTENSION_ID"}` | **Desktop only:** readonly `{schema,browser,extension_id,version,sha256,bytes,host_path,manifest_path,manifest,state_path,effect,platform,architecture,existing_manifest,existing_registration,existing_host,installed_host,status,install_action}`. status ready/already-registered/conflict; install_action install/upgrade/none/blocked. |
| `{"op":"bundled_host_pending","browser":"chrome"}` | **Desktop only:** original complete approval for an unfinished installation, or null when none is pending. No new extension ID is accepted; this query neither replaces nor approves a plan. |
| `{"op":"install_bundled_host","approved_plan":{...}}` | **Desktop only:** full frozen-plan approval/recheck; `{status,plan,pairing:"required"}`, status registered/updated/already-registered. Identical successful approval is query-only without repeat writes/authorization; blocked refuses. |
| `{"op":"installation_plan","browser":"chrome","extension_id":"EXTENSION_ID","host_path":"/absolute/path/to/lintel-browser-host"}` | `{browser,extension_id,manifest_path,manifest,state_path,effect}`; fixed current-user path preview. |
| `{"op":"install_native_host","browser":"chrome","extension_id":"EXTENSION_ID","host_path":"/absolute/path/to/lintel-browser-host"}` | `{status,plan,pairing:"required"}` after user approval; edge/firefox also accepted. These two manual-path operations are independent CLI local-control only; desktop returns unsupported_browser_operation, extension pipe exposes neither. |
| `{"op":"pair_create"}` | `{challenge, code, expires_at}`; 12 uppercase hex characters, five minutes. At most eight pending challenges; oldest evicted beyond that. |
| `{"op":"pair_pending"}` | `[{challenge, code, instance_id, label, browser, extension_id}]` |
| `{"op":"pair_approve","challenge":"..."}` | Paired instance; explicit desktop-user approval required. |
| `{"op":"instances"}` | `[{instance_id,label,browser,extension_id,paired,conflict,last_seen,online}]` |
| `{"op":"submit","instance_id":"...","operation_id":"UUID","action":{...}}` | `{instance_id,id,action,phase,created_at}` |
| `{"op":"query","instance_id":"...","operation_id":"UUID"}` | Same operation and received `receipt`, without reexecution. |
| `{"op":"allow_extension","extension_id":"..."}` | Always `allow_extension_requires_installer`; authorization is installation-only. |

`expires_at`/`created_at`/`last_seen` are Unix seconds; extension `createdAt`/`startedAt`/`completedAt` are epoch milliseconds. Durable `paired` differs from `online`, a native poll in the latest 20 seconds. Disconnected ports, revoked permission or duplicate identity must not display effective protection.

Example reversible WebRTC submission:

```json
{"op":"submit","instance_id":"INSTANCE_UUID","operation_id":"OPERATION_UUID","action":{"kind":"webrtc","setting":"disable_non_proxied_udp"}}
```

Initial phase is `awaiting-browser-confirmation`, then `running` after extension approval; usual terminal phases are `completed`, `uncertain`, `rejected`. Clear preparation is `awaiting-browser-restart`, not deletion success; a separate post-restart `finishClear` yields deletion results. Success example:

```json
{"id":"OPERATION_UUID","phase":"completed","result":{"verification":"effective-readback","configured":"disable_non_proxied_udp","effective":"disable_non_proxied_udp","controller":"controlled_by_this_extension","scope":"current-profile"},"completedAt":1790985600000}
```

Restore uses a new operation ID and `{"kind":"restore","receiptId":"OPERATION_UUID"}`, still previewed/confirmed. Changed effective value/controller yields `restore_conflict`. Previously controlled BrowserSettings use `clear` to remove this extension's override and reveal current underlying values, rather than permanently overriding with an old value. contentSettings lacks full controller queries; restore clears/rebuilds only extension-owned rules and reads effective values, preserving other managed settings. Readback differing from intended owned rules refuses; control-source visibility remains limited.

Other allowed actions:

```json
{"kind":"clear","origins":["https://claude.ai"],"types":["cookies","localStorage","indexedDB","serviceWorkers","cacheStorage"]}
{"kind":"finishClear","receiptId":"PREPARATION_OPERATION_UUID"}
{"kind":"clearProfileCache"}
{"kind":"sitePermission","origins":["https://claude.ai"],"setting":"location"}
{"kind":"proxy","origins":["https://claude.ai"],"port":8080}
{"kind":"blockSites","origins":["https://claude.ai"]}
{"kind":"pauseRules","minutes":10}
```

`sitePermission.setting`: `location/camera/microphone/notifications`; `webrtc.setting`: `default/default_public_interface_only/disable_non_proxied_udp`. Pause is 1–60 minutes; browser alarms restore owned persistent blocking rules without releasing clear isolation. If closed at expiry, next-start reconciliation restores; no exact offline-time guarantee.

Expiry restoration and new approved pauses share Engine's mutation queue. After reading the durable plan and verifying effective DNR rules, reconciliation consumes only the record it read; late old callbacks cannot erase a newer pause. Legacy records without operation IDs remain recoverable. DNR/storage failures retain records for later alarms/startup. Already restored rules consume the confirmed record without adding again; changed rule sets retain the plan/report conflict without overwrite.

### Native pipe and trust boundary

Browser initiates `connectNative("app.lintel.browser")`. Host requires exact manifest origin/ID, allowlist and paired local instance token. Frames use native-endian 32-bit length + UTF-8 JSON, max 64 KiB. No exec/read_file/delete/export, URL fetch or arbitrary-path operations exist.

Internal fields are `{op, request_id, instance_id, token}`; token is a local random 256-bit value stored only in extension `storage.local` and private user bridge DB, never sync.

- `pair_request` adds `{code,label}`, binding challenge to instance and returning `{paired:false,pending:true,challenge,code}` without self-approval. Host derives browser from argv-validated caller ID (`lintel@lintel.local` → firefox; 32-character Chromium ID → chromium), ignoring self-reported browser. Five consecutive wrong attempts invalidate all live pending codes; regenerate.
- `poll` returns `{paired:true,proposals:[...]}` bound to the current native-connection lease.
- `operation_query` adds `{operation_id}` and only queries an original operation under the current authenticated instance. It creates or submits nothing. Legacy journal ownership is checked against the ID and action; another instance's identity cannot be borrowed for reads or receipt delivery.
- `receipt` adds `{receipt:{id,phase,result?,error?,completedAt?}}`, reporting only, never execution.

Replies retain `request_id`. Bridge directories are 0700, state/locks 0600, with exclusive locks, temporary writes, fsync, rename and directory fsync. Defaults: macOS user Application Support `Lintel/browser-bridge`; Linux user `.local/state/lintel/browser-bridge`. `LINTEL_BROWSER_STATE` can set a synthetic temporary root as process configuration, never an extension-provided path. No network listener.

Existing `bridge.json` first passes bounded structural validation. Syntactically valid JSON with malformed field shapes returns `invalid_state` and retains the original bytes instead of overwriting the damaged file on read. This does not imply automatic repair of every semantic conflict.

Two concurrent native connections using identical credentials persist `duplicate_instance_conflict` and stop accepting operations on both. The extension can generate a new identity and pair again. Browsers offer no trusted ordinary-profile identity: **offline clones that never connect simultaneously cannot be proven distinct**. This portion of B02 is incomplete; connection-conflict detection is not complete arbitrary-clone detection.

### No-repeat boundary

Before native mutation, extension persists `running` in `storage.local`; completed results persist before reporting host. Restart turns `running` into `uncertain`; the same completed/uncertain ID never reruns. Lost ACKs resend receipts only, so a new user login cannot be cleared by transport retry.

Host records and extension journal complement each other. Operation IDs bind original actions; changing action refuses. On disconnection/host-only running records, GUI queries/waits instead of generating a new automatic retry ID. At 1000 host operations, new submissions refuse. No user archive/cleanup tool exists yet, so unattended long-term operation is incomplete.

## Verification and remaining scope

```sh
node --test extensions/browser/tests/*.test.mjs
cargo test --manifest-path extensions/browser/native-host/Cargo.toml
cd extensions/browser
npm ci
npx playwright install chromium
npm run test:browser
```

First build App frontend from root with `npm --prefix apps/desktop run build`; smoke needs a working Rust toolchain and builds debug host/runner. Existing Playwright can be reused via `PLAYWRIGHT_MODULE=/absolute/path/to/playwright/index.mjs npm run test:browser`. Browser/Playwright revisions should match. `PLAYWRIGHT_CHROMIUM_EXECUTABLE` accepts only executables under `ms-playwright/chromium-*`, never ordinary `/Applications` Chrome. Tests create temporary profiles/native DB and synthetic localhost HTTP on fixed port 18765; conflicts fail without modifying services. They use no personal Chrome profile/Cookie DB or real Claude sites.

`node scripts/build.mjs --fixture` generates conspicuously named packages accepting only `http://localhost:18765`, with separate preauthorized Cookie/DNR/loopback permissions. Never distribute fixtures to ordinary users.

Local 2026-10-05 evidence: 29 JS contract tests and 14 native-host Rust tests passed. macOS arm64 Chromium smoke retained 11 assertions and added the twelfth App clear → full browser exit/real onStartup → popup continuation → original App ID final receipt, plus preview-cancellation return and no old-ID execution. App synthetic invoke calls real core/host; HTTPS pages are locally fulfilled without real Claude. Native WebKit/personal profiles are excluded. See [candidate history (Chinese)](candidate-history.md) and [current state (Chinese)](current-state.md).

Test-only [persistent-install.mjs](../extensions/browser/tests/persistent-install.mjs) uses Chromium's native manager installer: temporarily damages only its own copied fixture manifest to obtain browser-generated load-failure recovery credentials, restores bytes exactly, then retries natively. This follows [Chromium reload/loadUnpacked](https://chromium.googlesource.com/chromium/src/+/main/chrome/browser/extensions/api/developer_private/developer_private_functions.cc), not a product automatic installer. Tests verify old process exit/new process identity, restart without `--load-extension`/`--disable-extensions-except`, unchanged extension identity and real production-listener `runtime.onStartup`, then separately approve finishClear. No direct profile-preference/generation edits or forged events. Generated local `browser-smoke.json` / `browser-smoke-failure.json` under `extensions/browser/artifacts/` stay outside Git.

The prior active-SW negative case proved unregister can still rewrite. Command-line temporary loading/CDP temporary installation lacked persistent startup generations and correctly refused `browser_restart_required`, motivating the current two-step/native harness. Old static-worker success cannot replace current checks. macOS headless fixtures need `bringToFront()` before planting/reading storage, matching revisions, and unchanged 30-second stage deadlines.

App pairing has repeatable page-to-real-host checks. The old page copied a 64-character internal challenge and failed `invalid_pairing_code`; now it copies the actual 12-character code, then reviews/approves the internal pending challenge. Run from root:

```sh
npm --prefix apps/desktop run build
python3 tests/verify.py --checks browser-smoke,browser-pairing-ui --json /tmp/lintel-browser-runtime.json
```

These need Playwright above. `browser-pairing-ui` runs real core/host with synthetic invoke/clipboard, checking copied values accepted by framed native requests, explicit approval, offline state, new codes and keyboard/Day/Night layout. It does not prove WebKit/OS clipboard/non-developer first install. CI selects both separately on macOS/Ubuntu, outside the default synthetic group; see [current state (Chinese)](current-state.md) and [verification (Chinese)](verification.md).

Remaining: formal Chrome/Edge/Firefox/AdsPower versions; Firefox container runtime/separate CacheStorage/site proxy; offline-clone identity; signed host/extension distribution; dedicated browser launch/process takeover prevention; App deep links; user journal archiving. Independent Chromium uses real runtime with synthetic data, without claude.ai/real authentication/relogin. Developer packages/partial passes do not deliver the full browser module.

## Official capability references

Reviewed 2026-10-03:

- [Chrome browsingData](https://developer.chrome.com/docs/extensions/reference/api/browsingData): origins, registrable-cookie domain and category Promises.
- [Firefox RemovalOptions](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/browsingData/RemovalOptions): differing hostname/cookieStoreId category support.
- [Firefox removeCache](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/browsingData/removeCache): whole-profile HTTP cache.
- [Firefox privacy.network](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/privacy/network): WebRTC values/BrowserSetting.
- [Chrome contentSettings](https://developer.chrome.com/docs/extensions/reference/api/contentSettings): site patterns, effective values, owned clear.
- [Chrome proxy](https://developer.chrome.com/docs/extensions/reference/api/proxy): PAC/profile scope, not all traffic.
- [ServiceWorkerRegistration.unregister](https://developer.mozilla.org/en-US/docs/Web/API/ServiceWorkerRegistration/unregister): unregister differs from running-event termination.
- [Chrome runtime.onStartup](https://developer.chrome.com/docs/extensions/reference/api/runtime#event-onStartup): distinguishes profile startup/worker restart.
- [Chrome webNavigation](https://developer.chrome.com/docs/extensions/reference/api/webNavigation): getAllFrames target-iframe matching.
- [Firefox DataTypeSet](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/browsingData/DataTypeSet): no independent CacheStorage category; whole-cache/serviceWorkers are no substitute.
- [Firefox background](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/manifest.json/background): Firefox scripts/event pages versus Chrome workers.
- [Firefox Native Messaging](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/Native_messaging), [Chrome Native Messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging): manifest installation/framing/caller identity.

## Finite unified-CLI adapter

macOS `lintel browser operations` / `describe browser.OP` / `schema browser.OP` statically describe control/actions without browser-state initialization. `browser operations --instance ID` inspects pairing/conflict/online; extension confirmation still decides permissions. `browser instances`, `pair create/pending/approve`, `submit/query/control` reuse the host with original-ID query-only recovery.

Linux runner explicitly lacks this component; use the independent host CLI on the profile's host. Manual host_path CLI installation and App bundled-resource approval remain separate entries, without assuming GUI installation. See [Agent CLI browser adapter](agents.en.md#browser-adapter) for installation/stdin/human waits, and [operator guide](operator-guide.en.md) for pairing/clear.

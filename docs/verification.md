# Repository verification

`tests/verify.py` is the single local entrypoint for the repository's existing
checks. It aggregates commands that already live in this repo; it does not
re-implement them or add new test logic.

This page describes how to run the checks and how to read their results. It does
not upgrade any check into acceptance evidence for the complete product target.
Real browser, real authentication, and real Linux/SSH runtime evidence stay
independent and are listed here explicitly (see [Independent evidence](#independent-evidence)).

## Quick start

```sh
python3 tests/verify.py --list          # show every check and its requirements
python3 tests/verify.py                 # run all default (synthetic) checks
python3 tests/verify.py --category rust # run one category
python3 tests/verify.py --checks js-engine-test,desktop-typecheck
python3 tests/verify.py --json /tmp/lintel-verify/evidence.json
python3 tests/verify.py --self-test     # test the entrypoint's own behavior
```

The script uses only the Python standard library. It exits non-zero if any
executed check fails. `deferred` and `skipped` checks never count as passes.

The table below lists the equivalent raw commands; [开发与文档](../README.md#开发与文档) points to this entrypoint.

## Checks

| id | category | what it runs |
| --- | --- | --- |
| `cargo-workspace-test` | rust | `cargo test --workspace` (crates/core, crates/operations, crates/remote, crates/egress, apps/runner) |
| `desktop-rust-test` | rust | `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` (standalone Tauri crate, excluded from the workspace) |
| `native-host-rust-test` | rust | `cargo test --manifest-path extensions/browser/native-host/Cargo.toml` (standalone crate) |
| `runner-build` | rust | `cargo build -p lintel-runner` |
| `js-engine-test` | js | `node --test extensions/browser/tests/engine.test.mjs` (no dependencies) |
| `site-game-test` | js | `node --test tests/site_game.test.mjs` (shared runner gait/jump/stars/collision/pause physics, no dependencies) |
| `home-greetings-test` | js | original time-of-day pools, date/time/rare priorities and local once-per-date persistence |
| `linux-vm-control-test` | python | `python3 -m unittest discover -s tests/fixtures/linux_vm -v` (launcher, barrier and finite probe tests; no VM) |
| `python-ssh-test` | python | `python3 -m unittest discover -s platform/ssh/tests` (fake SSH transport, synthetic state) |
| `desktop-typecheck` | desktop | `npm run typecheck` in `apps/desktop` |
| `desktop-build` | desktop | `npm run build` in `apps/desktop` (tsc + Vite) |
| `shared-remote-journey` | python | shared Rust finite controller public API and schemas, synthetic transport |
| `journey-product-baseline` | journey | `python3 tests/backend_baseline_journey.py`; readonly context/help, frozen target and bounded session/launch contracts |
| `journey-agent-cli` | journey | static catalog, named CLI, durable original-ID wait, portable archive/import and restore |
| `journey-agent-adapters` | journey | synthetic SSH registry, browser absent-profile handling and foreground network stream/stop |
| `journey-portable-work` | journey | independent state import, package failures and persistent error codes |
| `journey-cli` | journey | `python3 tests/cli_journey.py` (real CLI JSON boundary) |
| `journey-submission` | journey | `python3 tests/submission_journey.py` (detached durable ACK, replay dedup) |
| `journey-components` | journey | named CLI finite metadata/config sources, original-ID scope, no auth/body disclosure or journal reconciliation |
| `journey-large-work` | journey | `python3 tests/large_work_journey.py`; >8 MiB files / >32 MiB total archive, independent state import and preserve, full hashes, metadata blockers and isolated per-process RSS |
| `journey-work-capacity` | journey | metadata-only byte/file bounds, incomplete scans, strict schema and unchanged full archive admission |
| `journey-work-preservation` | journey | `python3 tests/work_preservation_journey.py` |
| `journey-launch` | journey | `python3 tests/launch_journey.py` (CLI/TUI real PTY launch plus custom policy approval and restoration, synthetic roots) |
| `journey-policy` | journey | `python3 tests/policy_journey.py` (versioned policy, custom keep/disable/remove, no-op, restoration and external edits) |

The two standalone Rust entrypoints beyond the root workspace (`apps/desktop/src-tauri`,
`extensions/browser/native-host`) are listed because the root `Cargo.toml`
excludes them; verifying only `cargo test --workspace` would silently skip them.

The standalone website has an independent `site-ui` check: `python3 tests/verify.py --checks site-game-test,site-ui` runs game physics and an isolated Chromium journey for responsive layout, theme persistence, illustrative plan/receipt tabs, game lifecycle, keyboard/pointer controls, reduced motion, unavailable storage and absence of external requests. It serves only `apps/site` on an ephemeral loopback port and uses the existing Playwright installation (or `PLAYWRIGHT_MODULE`). It does not open a personal browser profile or exercise the App/core/native bridge. See the [website guide](../apps/site/README.md) for local preview; there is no website build step. Independent `clawd-app-ui` (`node tests/clawd_app_ui_journey.mjs`) exercises the same runner through the built App pocket menu with an empty synthetic inventory: existing score retention, a single animation loop, disposal on tab/modal exit, focus return, landscape interaction and Day/Night/System. Build the App frontend first; this is Chromium UI evidence, not native WebKit or an installed bundle.

Toolchains (`cargo`, `node`, `npm`, `python3`) are resolved from the caller's
`PATH` and environment. The entrypoint does not install anything, does not probe
another user's home, and does not rewrite `CARGO_HOME`/`RUSTUP_HOME`; a host
where a tool cannot run for environment reasons simply shows the check failing
or skipped, which is that host's environment, not a change to the check.

The default desktop build already runs TypeScript checking; `desktop-typecheck` remains separately selectable.

`desktop-tauri-bundle` also prepares non-fixture Chromium / Firefox extension
files and the existing browser native-host executable before the frontend/native
build. It does not register a personal
browser. The ordinary frontend build excludes operator-supplied local fonts;
explicit `--mode local-candidate` is a local-only artifact, not the default
candidate build.

Bundled-host tests cover read-only previews, exact approval, fixed-user binary
and manifest installation, ownership-aware reviewed upgrades, stale/conflicting
registrations and query-only replay on synthetic homes. To additionally run the
actual prepared executable from its installed manifest and parse native frames:

```sh
npm --prefix apps/desktop run prepare:browser-host
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml built_host_runs_from_installed_manifest_and_returns_native_frames -- --ignored
```

This requires a host build on the current platform. It proves the bundled
executable/installer/native framing boundary, not a personal browser connection,
extension installation or real `runtime.onStartup` acceptance.
After a Tauri build, set `LINTEL_TEST_BROWSER_HOST_RESOURCES` to the resulting
App's `Contents/Resources/browser-host` directory to run the same ignored test
against the packaged artifact. This variable is read only by that synthetic test,
not by production IPC. The host packaging helper currently rejects cross-target
and universal App builds; use a matching native macOS architecture.

Bundled-extension tests cover read-only inventory previews, complete approval,
fixed-user preparation, owned updates, stale/unowned/symlink conflicts and
explicit recovery of interrupted directory replacement on synthetic homes.
To exercise the actual generated extension resources:

```sh
npm --prefix apps/desktop run prepare:browser-extension
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml actual_packaged_extensions_install_from_bundle_with_no_profile_mutation -- --ignored
```

After building the App, set the test-only
`LINTEL_TEST_BROWSER_EXTENSION_RESOURCES` to its
`Contents/Resources/browser-extensions` directory to verify the packaged files.
This tests the App-resource-to-user-directory boundary and absence of browser
profile/native-host mutation. It does not load an extension or establish
persistent browser installation, `runtime.onStartup` or the full two-stage clear.
The Finder action accepts a named browser only; the actual user interaction with
Finder and the browser's load picker remains independent runtime acceptance.

### Categories

`rust`, `js`, `python`, `desktop`, `journey`, `independent`.

## Synthetic-only guarantee

Default checks operate only on synthetic temporary roots that the checks create
themselves (`LINTEL_TEST_HOME` / `LINTEL_STATE_DIR` pointing at throwaway
directories). They do not read the operator's real Claude credentials,
personal browser profiles, SSH remotes, or network policy. The Python SSH suite
uses a fake SSH transport, and the JS suite uses an in-memory browser API stub.
Startup observations in unit tests and matching `LINTEL_TEST_HOME` CLI runs
redirect managed candidates under the synthetic home and stop ancestor walks at
the independent temporary fixture, including fixtures with sibling home/project
directories. Invalid synthetic scope is refused before observation.

Two loopback-only checks (`cargo-workspace-test` via `crates/egress/tests/proxy.rs`
and `desktop-rust-test` via the desktop network module) bind `127.0.0.1` sockets.
On a host that forbids localhost binding these tests surface as failures with
`Operation not permitted`; that is an environment restriction, not a product
result, and the entrypoint reports it honestly rather than masking it.

Browser smoke also accepts an explicit `PLAYWRIGHT_MODULE` module path, matching the harness.

`desktop-build` needs `apps/desktop/node_modules` (`npm ci`). If it is missing the
check is reported as skipped with a reason.

## Independent evidence

These checks are real-runtime evidence. They are never run by the default
selection, are never replaced by mocks or skips, and must be triggered
explicitly (`--all`, or by naming the check or its category). When requested but
a prerequisite is unavailable they report `skipped` with a reason, never a pass.

| id | what it needs |
| --- | --- |
| `cli-candidate` | macOS arm64 native CLI or Linux x86_64/aarch64 static musl CLI; Node/tar; macOS cc/shasum or Linux sha256sum. Mac cc builds only a synthetic malformed-probe fixture; the installed CLI does not need a compiler. Packages/extracts/runs the native CLI in synthetic homes, rejects wrong/dynamic/missing inputs and overwrites, and preserves original state/job across two selected version directories. Other-architecture inputs are visibly synthetic format fixtures; these do not prove their runtime. The optional full-real smoke requires all three correctly formatted binaries and both canonical runner files plus manifest; incomplete or wrong-platform local outputs cannot trigger it. Unsupported native hosts, including Intel macOS, skip before launching the journey. ELF program headers must match the Linux loader's exact 56-byte entry size and 64 KiB table limit. |
| `linux-ssh-runtime` | Real Linux x86_64, OpenSSH sshd/client, static musl runner and shared Rust controller; temporary loopback keys/config/HOME/state, inert Claude. No real VPS or account actions. |
| `browser-smoke` | Built desktop frontend, Rust and Playwright Chromium, with a real restart that emits `runtime.onStartup`. Two-phase browser clear cannot be accepted from extension-worker restarts or synthetic generations. |
| `browser-pairing-ui` | Built desktop frontend, Playwright Chromium and Rust. Renders the App, copies its actual short code and submits a framed request to the real native host; invoke and clipboard are synthetic. This does not prove native WebKit/OS clipboard or a non-developer installation. |
| `remote-task-ui` | Built desktop frontend and Playwright Chromium. ACK loss, App reload, local→A/B→A full receipt routing through refresh, Agent packet/query command and startup preview, late-response isolation and separately approved restoration; invoke/SSH/registry are synthetic. Set `LINTEL_REMOTE_TASK_UI_REPORT` for the report and optional `LINTEL_REMOTE_TASK_UI_ARTIFACTS` for external screenshots. |
| `baseline-ui` | Built frontend + real synthetic core; once-per-date greeting across StrictMode and App reload, task/target isolation, source-bound reader, explicit context, finite startup sources with sanitized declarations/auth unknown, old-runner approval refusal, clipboard failure, original request query followed by a fresh preview only for confirmed unattempted plans, and finite Agent metadata. Static CLI/clipboard/launch-error transport is modeled; not native WebKit or authenticated Claude. |
| `work-ui` | Built desktop frontend, Playwright Chromium and runner. Real core in independent synthetic homes; archive-only/preserve/portable import, metadata paging, exact project/session/file selection excluding the actual runner file limit + 1 sparse bytes, frozen approval/bytes, old-runner refusal, wrong password, keyboard and Day/Night; synthetic invoke, not native WebKit. |
| `components-ui` | Built frontend, runner and Playwright Chromium; real synthetic cores, component/cwd facts, malformed settings, exact original alias/ID, late-response isolation and capacity correction. Invoke/SSH are modeled; not native WebKit or production. Set `LINTEL_COMPONENTS_UI_REPORT` for external evidence. |
| `service-ui` | Built desktop frontend and Playwright Chromium. Service inspection, exact approval, lost ACK query, external-edit conflict and separate resume approval; service manager/invoke are synthetic. |
| `site-ui` | Static website, Playwright Chromium and an ephemeral loopback server; no native/core transport or external requests. |
| `clawd-app-ui` | Built desktop frontend and Playwright Chromium; shared game lifecycle through the App with an empty synthetic inventory. |
| `linux-vm-runtime` | Linux x86_64, static musl runner, QEMU, cloud-image-utils, OpenSSH client, gpgv and Ubuntu cloud-image public keyring. Creates a disposable Ubuntu guest with real systemd/PAM, synthetic services and users; host policy and production VPS remain outside the test. |
| `desktop-tauri-bundle` | macOS host and Xcode Command Line Tools; builds the native app bundle (`npm run desktop:build`). |

CLI candidate packaging has its own explicit selection:

```sh
# macOS arm64: the entrypoint selects target/debug/lintel.
cargo build --locked -p lintel-runner
python3 tests/verify.py --checks cli-candidate --json /tmp/lintel-cli-candidate.json
```

On Linux x86_64, first build `target/x86_64-unknown-linux-musl/release/lintel`
with the static musl command below. Linux aarch64/arm64 selects
`target/aarch64-unknown-linux-musl/release/lintel`; build the corresponding
static musl target on that host. The selection uses the native architecture.
Architecture-selection control tests do not establish aarch64 runtime acceptance.
CI selects this check on both platforms after the applicable build. Installation
verification uses the packaged checksum list with macOS `shasum` or Linux
`sha256sum`; Node is a packaging-host prerequisite, not a CLI runtime dependency.
The producer supplies the selected full source revision. Package checksums and
native version output do not authenticate source provenance or a signature.

Browser runtime and App pairing have a separate opt-in entrypoint (repository root):

```sh
npm --prefix apps/desktop ci
npm --prefix apps/desktop run build
npm --prefix extensions/browser ci
(cd extensions/browser && npx playwright install chromium)
python3 tests/verify.py --checks browser-smoke,browser-pairing-ui --json /tmp/lintel-browser-runtime.json
```

CI explicitly selects both checks on macOS and Ubuntu after the default checks;
Ubuntu also selects the Linux OpenSSH check below. Browser smoke uses a real
headless Chromium process, a disposable persistent profile and a synthetic
extension. Test pages use loopback fixtures or locally fulfilled synthetic HTTPS;
no real Claude service is visited. Its native installation
helper restores its disposable manifest byte for byte, closes the old browser
process, then relaunches without extension-loading flags. It requires a different
native `runtime.onStartup` generation before the second, separately approved
deletion. No profile preferences or startup markers are written by the harness.
After explicit isolation release, the harness waits for the completed UI update,
checks the durable release receipt and absence of its DNR rules before navigating.
Permission previews are visible before confirmation; clicking an asynchronous
button alone is not completion evidence.
The detailed `browser-smoke.json` records process IDs, loading flags, generation
and native-host receipts. These are real-runtime observations with synthetic
data, not claude.ai or logged-in Claude acceptance. Formal Chrome, Edge, Firefox
and AdsPower remain separate platform checks; see the [browser guide](browser.md).

The pairing journey starts its own loopback preview server on an ephemeral port.
Set `LINTEL_PAIRING_UI_REPORT` to an output file to retain its detailed report;
the parent directory must already exist. CI retains this alongside entrypoint
and startup evidence. Both browser checks accept `PLAYWRIGHT_MODULE` when using
an already installed matching Playwright runtime.

Linux OpenSSH runtime has a separate opt-in entrypoint:

```sh
rustup target add x86_64-unknown-linux-musl
cargo build --locked -p lintel-runner --release --target x86_64-unknown-linux-musl
python3 tests/verify.py --checks linux-ssh-runtime --json /tmp/lintel-linux-ssh.json
```

It exercises native preview/approval/upload, installation ACK loss and query-only
recovery, real durable submit with dropped ACK, original-job reconnect and
interactive SSH PTY launch. Claude is inert; no login or model calls occur. The
launcher requires exactly one executed Rust acceptance test, rather than treating
an empty test filter as a pass. Ubuntu CI explicitly selects this check after its
default synthetic checks. All server/config/key files are temporary; sshd runs
under the current user on loopback with PAM disabled. It does not edit accounts,
system sshd or services.

Actual VPS logout/cgroup, host reboot, aarch64 runtime and the complete macOS Terminal-to-VPS GUI journey remain
independent gaps; this Linux fixture does not prove those behaviors.

### Disposable systemd / PAM / reboot VM

The separate VM check boots a signed official Ubuntu 24.04 LTS cloud image
(release build `20260801`), with its own kernel boot ID, systemd PID 1, packaged
OpenSSH and real PAM sessions. Downloaded image/manifest checksums are verified
against Ubuntu's signed `SHA256SUMS` using the installed public cloud-image
keyring; the cache stays outside Git. This is a test prerequisite, not an App
dependency or VPS installation path.

```sh
# On an Ubuntu x86_64 test host; these are test-only prerequisites.
sudo apt-get install -y qemu-system-x86 qemu-utils cloud-image-utils openssh-client gpgv ubuntu-keyring
cargo build --locked -p lintel-runner --release --target x86_64-unknown-linux-musl
python3 tests/verify.py --checks linux-vm-runtime --json /tmp/lintel-vm-entry.json
```

Set `LINTEL_VM_REPORT` to choose the detailed Git-external report path (default
`/tmp/lintel-vm-runtime.json`). The launcher never changes host login policy or
starts host services. It binds the guest SSH forwarding to `127.0.0.1`, denies
guest external networking and destroys its own VM overlay, seed and keys after
QEMU stops, including failures.

The canonical service journey supplies real Restart=always / timer / manual
start, neighbor preservation, exact approval, replay, external-edit conflicts
and original inactive-state restoration. Its prepare/recover phases also verify
that the owned persistent hold remains effective across a real guest reboot.
Separate runner cases observe both guest `KillUserProcesses` policies, the
worker's real PAM session/cgroup and post-reboot original-job reconciliation.
A further strict `KillUserProcesses=yes` comparison checks already-root system
manager, the fixture user with pre-existing linger/user bus, and ineligible
setsid. The observer must see the original PAM session end before reconnecting
as the target user. Eligible workers MUST remain alive and complete the original
approved policy task after explicit barrier release; lost workers fail the
check. The same real reboot also interrupts one system-managed original job,
which must reconcile on repeated query without resubmission. Only this guest
fixture can enable/restore the synthetic user's linger; production code cannot.
An inert auth fixture adds a shared profile after preview; a system-managed
worker must preserve the submitting environment and reject before acceptance,
without calling logout, deleting its synthetic credentials or persisting the
environment value. Its read-only cleanup preview inspects all candidate system
services and uses a finite 180-second budget under emulation; ordinary requests
still use 60 seconds. Known shared auth scope is rejected before slow service
reads. No real Claude or network request is made.
The synthetic-only `LINTEL_TEST_ACCEPT_BARRIER` records a marker inside
`LINTEL_TEST_HOME` and pauses the worker after durable acceptance/ACK so the
interruption is reproducible. `LINTEL_TEST_WAIT_BARRIER` and
`LINTEL_TEST_WAIT_RELEASE` instead hold a live worker to separate a stopped
process from ordinary execution. Markers/releases must belong to the synthetic
home; neither barrier modifies normal submissions.

`evidence_complete` means those observations finished, including the limited setsid route and
required eligible-manager survival/completion. It does **not** promise `setsid` survives arbitrary cgroup
cleanup or establish a production VPS policy. Missing VM/systemd/PAM or a failed
boot is a failed run. Runtime versions, boot IDs, observed survival/termination,
original receipts and VM cleanup are retained in the detailed JSON.

Clean `8c9d85c` [CI37177180828](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37177180828)
passed macOS/Ubuntu defaults 12/12 each, browser/UI checks 3/3 each, Ubuntu
OpenSSH 1/1 and this VM check 1/1. All six entrypoint reports record the same
clean source HEAD. The detailed VM report is `evidence_complete`: Ubuntu
24.04.4, systemd 255, real OpenSSH PAM sessions, complete root system-manager
service suite, changed kernel boot ID, persistent hold recovery and original-job
query without reexecution. Both effective `KillUserProcesses=no` and `yes`
cases observed worker termination on logout; original receipts became
`needs_reconciliation`, with one submission each. This is executed observation
and reconciliation evidence, **not reliable logout-surviving execution**.
The VM and its overlay/seed/keys were cleaned up. This historical run used
setsid and is not evidence for the new manager path.

Earlier clean `7a05291` [CI37216354079](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37216354079)
passed macOS/Ubuntu defaults 13/13 each, independently selected browser/UI
checks 4/4 each, Ubuntu OpenSSH 1/1 and real Linux VM 1/1. All six entrypoint
reports identify the same clean HEAD. The VM report is `evidence_complete`:
under effective `KillUserProcesses=yes`, both eligible system/user-manager
workers stayed in their exact unit cgroups after the original PAM logout and
completed the original policy task after barrier release. Ineligible setsid
terminated and reconciled. Real reboot changed the boot ID; system-managed and
setsid accepted jobs remained the same original `needs_reconciliation` jobs
on repeated query, with one submission and no reexecution. The managed shared
auth fixture preserved caller context and rejected before acceptance, without
logout, credential deletion or persisting the environment value; its read-only
preview took 55.279 seconds. The complete service/neighbor/conflict/persistent
hold recovery suite also passed. This is isolated x86_64 runtime evidence,
not production VPS, aarch64 or real Claude/auth acceptance. The App reopen /
original-query / full-receipt / separate-recovery journey uses real rendered
Chromium with synthetic invoke/SSH/registry, not native WebKit or production
transport. Current candidate and remaining limits are recorded in
[current-state](current-state.md).

The built service UI can be checked separately with
`python3 tests/verify.py --checks service-ui`. Set `LINTEL_SERVICE_UI_REPORT` for
its JSON and `LINTEL_SERVICE_UI_ARTIFACTS` for optional Git-external screenshots.
`work-ui` uses real core processes in two independent synthetic homes to verify
archive-only, preservation and portable selective import through the built App.
It also verifies bounded metadata pagination, project/session/file choices,
explicit oversized-file exclusion, exact archive and preserve destinations,
unselected-file changes, and refusal when a runner ignores exact selection.
Set `LINTEL_WORK_UI_REPORT` for its JSON; fixture screenshots stay outside Git.
It uses real headless Chromium and core with a synthetic invoke transport, not
native WebKit or authenticated Claude. Independent browser, pairing, service, remote-task and work UI checks are explicitly selected in CI;
their actual current results are recorded in [current-state](current-state.md).

These are the same gaps recorded in [current-state](current-state.md#完整目标仍缺少)
and the [acceptance status](acceptance-status.json). Passing the default checks
does not close them.

## Large work resource evidence

`journey-large-work` remains synthetic and uses the canonical debug runner. Its
fixture has a 24 MiB non-UTF-8 session and a >12 MiB JSONL session, plus instruction
and memory files, exceeding the former per-file and aggregate limits. Archive,
independent-state inspection, head/middle/tail pages, import and preserve verify
complete digests. Sparse 256 MiB + 1 and 1 GiB + 1 selections exercise metadata
blockers without reading those oversized bodies; exact selection excludes them.
Wrong passphrase, truncated/tag/tail failures clean current-operation staging;
legacy v1 without optional bytes, original CRLF/non-UTF-8 offsets, and selected
identity changes are explicit cases.

Set `LINTEL_LARGE_WORK_REPORT` to an external JSON path. Each measured CLI runs in
its own Python wrapper, so `RUSAGE_CHILDREN.ru_maxrss` covers one runner process;
the report records raw units (bytes on macOS, KiB on Linux), elapsed time,
platform/build conditions, semantic success and the actual age header's log N.
This includes the device-selected default age KDF, not a reduced-cost test key.
The JSON codec uses bounded buffers and private disk staging; KDF memory, metadata,
page projections and disk/time costs remain separate. A measured peak on one
host/fixture is not a universal memory guarantee or runtime evidence at 1 GiB.

2026-10-07 local observation: macOS 26.5.2 arm64 debug runner, 36 MiB across
four original files, actual default log N 14. Archive peak was 24.5 MiB / 67.5s,
inspect 25.8 MiB / 35.4s, three page requests 24.9–26.3 MiB / 36.6–53.7s,
and preserve 25.0 MiB / 131.3s. Eight journey cases passed with no optional age
CLI dependency or legacy-fixture skip. Default 23/23 passed with
`RUST_TEST_THREADS=4`; final focused checks cover migration-probe protection,
the workspace, product-baseline CLI, built work-ui and strict schema rejection.
Final desktop build and baseline-ui also passed after the unattempted-plan
re-preview recovery change; uncertain requests must query the original ID first,
and no second launch is sent by re-preview. These
are dirty-source checks plus focused final evidence, not a packaged release or
another architecture's runtime result.

## Evidence JSON

Pass `--json PATH` to write a machine-readable record outside Git (default
`$TMPDIR/lintel-verify/evidence.json`). The document records, per run:

* the actual command, working directory, and fixture category per check;
* status (`passed` / `failed` / `skipped` / `deferred`), exit code, and duration;
* tail of stdout/stderr for failures;
* platform (system, release, machine), tool versions, and Python version;
* `git.head` plus a `dirty` flag and dirty-entry count — a dirty run is never a
  committed-HEAD proof;
* start/finish timestamps.

The file is never auto-committed, and no hashes are computed for their own sake.
Evidence should be stored outside the repository (for example under the system
temporary directory or a private operator location).

## Self-test

`python3 tests/verify.py --self-test` exercises the entrypoint itself: registry
uniqueness, default/explicit/`--all` selection semantics, unknown-id rejection,
git-state fields, that the default evidence path stays outside the repository,
and `passed`/`failed`/`skipped`/`deferred` reporting against throwaway commands.

## Earlier acceptance observations

These dated observations retain their original source and scope. Current source,
CI results and package identities live in [current-state](current-state.md).

Earlier custom-policy source acceptance: clean `1fb9ab6` [CI run 37125403657](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37125403657) passed 12/12 defaults on both macOS and Ubuntu. The separately selected Linux OpenSSH runtime check passed 1/1 with 0 ignored; it exercised the seven-field custom contract on a real x86_64 static-musl runner, including exact subset changes, frozen receipt, lost ACK recovery and interactive PTY launch. It uses temporary synthetic roots and inert Claude, and does not establish real Claude effects, production VPS state or aarch64 runtime.

Earlier browser source acceptance: clean `93c3f74` [CI run 37163091952](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37163091952) passed 12/12 defaults and 2/2 explicitly selected browser checks on both macOS arm64 and Ubuntu x86_64. Detailed Chromium 151.0.7922.34 reports retain all 11 smoke assertions, actual process exit/replacement, a new production `runtime.onStartup` generation, no relaunch extension-loading flags, retained identity and completed native receipt. The App copy-to-real-host pairing report passes with synthetic invoke/clipboard. Ubuntu OpenSSH runtime also passes 1/1 with 0 ignored. All five entrypoint reports record the same clean HEAD; packaged-App opt-ins and local Chromium 155 evidence remain separate. Formal browser distributions, AdsPower, real Claude/auth, native WebKit/OS clipboard and non-developer installation remain unverified.

`work-ui` is an independent selection: built frontend + real core processes with isolated synthetic homes, invoke fixture and headless Playwright Chromium. It verifies task help, archive-only, preservation and portable selective import; it does not prove native WebKit or production transfer.


Latest installable-candidate source acceptance: clean `25fdb43`
([push CI37267381234](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37267381234),
[PR CI37267385529](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37267385529))
passed macOS arm64/Ubuntu x86_64 defaults 18/18 each, native CLI candidate
packaging/upgrade 1/1 each, independent browser/UI 7/7 each, Ubuntu OpenSSH 1/1
and disposable Linux VM 1/1. All eight entrypoint reports record the same clean
source HEAD. Both native checks extract and execute the supplied executable,
inspect version/capabilities/schema, retain the original environment/job across
new version directories and reject overwrite. Ignored tests are not passes.

The local candidate identity is `0.1.0-candidate-25fdb4305785`. Actual Mac arm64 and
both Linux archives match the fresh current-runtime build inputs and carry both
canonical static Linux runners beside bin/lintel. SHA256SUMS covers every file
except itself, including candidate.json/README.txt; source revision remains
caller-declared, not authenticated provenance. The actual local Mac archive was extracted and its version/capabilities/schema
executed; the same release input passed the complete native/upgrade journey
with synthetic version identities. The actual extracted CLI also passed the
named CLI and finite-adapter journeys. Ubuntu CI
executes its own native static x86_64 build; it does not establish runtime of the
local cross-built Linux archive bytes. Other-architecture fixtures and host-metadata
selection checks are synthetic; aarch64 runtime remains unverified.

Focused regressions cover metadata/instruction corruption, missing Mac probe
identity, manifest snapshot preservation, both architectures' 32 MiB installer
limit, unsafe version paths, ELF static entry/load/memory/dependency constraints
including the exact program-header size/table limit,
and Mach-O command/entry/macOS platform constraints. Unsupported native hosts
including Intel macOS skip before launching. Static PIE with larger
zero-filled memory, macOS legacy platform/thread commands remain accepted.
Inherited TAR_OPTIONS cannot change members; the actual member list is checked
before publication. Reported tar/index failures roll back only unchanged files
owned by the current invocation; external replacements/edits survive. Abrupt
process death is not claimed to have automatic transaction recovery.

The local App (22.63 MiB) was rebuilt after integrating `e276f67` recovery,
browser and shared-game source. It includes both fresh cross-built Linux runners
plus canonical browser host/Chromium/Firefox resources. Actual App/ZIP runner
bytes, manifest and CRC match the inputs; two actual App resource synthetic
installation tests passed. Later changes are packaging/tests/docs only and leave
these runtime bytes unchanged. The site journey waits for the real responsive
media notification before asserting orientation; a delayed-notification regression
fails with the former immediate read and passes with the bounded wait. These are
local development candidates: no Applications activation, Developer ID signing,
notarization, formal release or production VPS installation.

Earlier recovery/browser source acceptance: clean `e276f67`
[CI37263003720](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37263003720)
passed macOS/Ubuntu defaults 18/18 each, independent browser/UI 7/7 each,
Ubuntu OpenSSH 1/1 and disposable Linux VM 1/1. This earlier stage predates
the explicit CLI-candidate CI selection; it does not replace the current
installable-candidate evidence above.

Earlier unified CLI/work-preservation source acceptance: clean `d0851c7`
[CI37249445342](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37249445342)
passed macOS arm64/Ubuntu x86_64 defaults 17/17 each, independently selected
browser/UI checks 5/5 each, Ubuntu OpenSSH 1/1 and disposable Linux VM 1/1.
All six entrypoint reports record the same clean source HEAD. Both detailed
Chromium reports retain all 11 assertions, actual process exit/replacement,
production `runtime.onStartup`, no relaunch extension-loading flags and completed
native receipts. The VM report is `evidence_complete`, including eligible
system/user-manager continuation after real PAM logout, exact unit cgroups,
original-job completion, real-reboot reconciliation and persistent service-hold
recovery. All data/accounts/Claude programs are synthetic; production VPS,
aarch64 runtime, real authentication and formal browser/native WebKit acceptance
remain independent.

That round's local arm64 App was built from source `d0851c7`; its two actual
App-resource tests passed. The release CLI independently installed from
`d0851c7` passed named/adapter journeys and portable 5/5. Those App and CLI builds correspond to `d0851c7`. The built App rejects empty reset/retire work selections while repair
login remains independent. Shared strict named/SSH validation accepts omitted or empty repair-login categories while reset/retire require a nonempty selection; operations 6/6 and the real CLI journey cover this distinction. The work fixture serializes requests from its window
and assertions, preserving each original home rather than racing core locks. Malformed wait, real-PTY launch and
target-capabilities IDs reject before state initialization. Explicit archive
output freezes the parent device/inode and rechecks it before acceptance and
publication; changed directories and legacy explicit-output plans without that
identity require a fresh preview. Existing directory permissions are preserved
and missing work parents are private. Preview and execution preflight every
actual import parent inside the approved root for effective UID ownership and
search/write access before any archived content is published; known barriers
reject without automatic chmod/chown. That `d0851c7` App included browser host and extensions, but no Linux runner
bundles. The later rebuilt candidate includes both architectures; see the
resource table in [current-state](current-state.md). CI runtime evidence alone
does not establish App bundling or a production install.

New-root creation allocates and journals the exact new_root/new_environment_id before mkdir, then registers the same ID through the canonical path. Journal failure prevents creation; registration failure and interrupted original-job queries retain the intent without replay. The App labels unfinished intents as pending verification and offers policy editing only for inventory entries. That round's core 80/80 covered registration-failure recovery, journal-failure no-write and successful identity checks.

The atomic_new publisher uses native no-replace rename on macOS/Linux, consuming the staged name and publishing a single-link destination in one operation. Unsupported filesystem/kernel capability returns atomic_publication_unsupported without hard-link/overwrite fallback. The publication-boundary regression recovers the package before any later cleanup/readback (it failed on the old two-link state); core 80/80 passed. The App labels unfinished archive/state-backup paths as pending verification and uses shared readback evidence for both viewing and task-archive selection; the work journey retains completed archive read/import and checks the incomplete intent paths without replay.

Remote receipt refresh uses original-task reconnect. Manual App ID lookup and CLI remote job use read-only request.job; the shared controller preserves an existing task’s frozen runner and allocates no task record for an unknown ID, leaving unsubmitted plans eligible for their first approved submission. The rendered regression rejected the former generic job request and passed after the fix, preserving original-ID query and late-response suppression. Browser smoke awaits the actual asynchronous preview/release completion and reads back removed isolation rules; all original startup/storage/neighbor/native assertions remain.

That round's shared remote tests: 41 passed, 2 independent ignored. The installed-runner upgrade check covers reconnect and both job lookup fields; unknown-ID lookup creates no task/dedup record, later execute submits once, and repeats query. The real installed-CLI regression also proves that a manual job lookup bypasses submission storage without opening SSH. A new explicit reconnect durably captures the current runner binding before its first query; three fixture queries retain that binding across an alias upgrade with no submission. Retained original-job lookups remain available after alias removal. The unchanged stdin-transport/no-task-directory assertion and shared current-HOME journey passed.

Shared plan-hash shape is exactly 64 lowercase hex characters for strict named execute and new remote submissions. Malformed approval rejects before named core state or remote task intent/SSH; existing task records remain query-only. Old controller/schema regressions failed, then operations 6/6, remote 41 passed/2 ignored and the actual CLI journey passed; schemas use the same owner. Raw protocol-1 core approval checks remain compatible.

## Current integration requirements

Browser smoke also renders the built App through a synthetic invoke adapter to
real core/native-host processes. Build the desktop frontend first. Its additional
App-clear journey uses a locally fulfilled synthetic HTTPS page, a real Chromium
process exit and production runtime.onStartup, popup continuation, original-ID
query, and unexecuted-preview cancellation. It never visits the Claude service.
The default `home-greetings-test` checks both greeting sets, hour boundaries, date→time→rare→ordinary priorities and local date persistence/failure. Core lifecycle regressions use only synthetic auth status and credential files, including A→B without local credential changes, post-preservation drift, file token updates/fallback and no replay of accepted jobs; they do not inspect the operator’s Keychain.
The default group includes 23 checks; the CI independent browser/UI group includes
browser-smoke, browser-pairing-ui, service-ui, work-ui, components-ui, baseline-ui, remote-task-ui, site-ui and
clawd-app-ui. Linux OpenSSH and VM remain separate checks within the same CI workflow.

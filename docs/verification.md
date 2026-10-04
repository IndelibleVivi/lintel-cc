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
| `cargo-workspace-test` | rust | `cargo test --workspace` (crates/core, crates/egress, apps/runner) |
| `desktop-rust-test` | rust | `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` (standalone Tauri crate, excluded from the workspace) |
| `native-host-rust-test` | rust | `cargo test --manifest-path extensions/browser/native-host/Cargo.toml` (standalone crate) |
| `runner-build` | rust | `cargo build -p lintel-runner` |
| `js-engine-test` | js | `node --test extensions/browser/tests/engine.test.mjs` (no dependencies) |
| `linux-vm-control-test` | python | `python3 -m unittest discover -s tests/fixtures/linux_vm -v` (launcher, barrier and finite probe tests; no VM) |
| `python-ssh-test` | python | `python3 -m unittest discover -s platform/ssh/tests` (fake SSH transport, synthetic state) |
| `desktop-typecheck` | desktop | `npm run typecheck` in `apps/desktop` |
| `desktop-build` | desktop | `npm run build` in `apps/desktop` (tsc + Vite) |
| `journey-cli` | journey | `python3 tests/cli_journey.py` (real CLI JSON boundary) |
| `journey-submission` | journey | `python3 tests/submission_journey.py` (detached durable ACK, replay dedup) |
| `journey-work-preservation` | journey | `python3 tests/work_preservation_journey.py` |
| `journey-launch` | journey | `python3 tests/launch_journey.py` (CLI/TUI real PTY launch plus custom policy approval and restoration, synthetic roots) |
| `journey-policy` | journey | `python3 tests/policy_journey.py` (versioned policy, custom keep/disable/remove, no-op, restoration and external edits) |

The two standalone Rust entrypoints beyond the root workspace (`apps/desktop/src-tauri`,
`extensions/browser/native-host`) are listed because the root `Cargo.toml`
excludes them; verifying only `cargo test --workspace` would silently skip them.

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
| `linux-ssh-runtime` | Real Linux x86_64, OpenSSH sshd/client, static musl runner and desktop build prerequisites; temporary loopback keys/config/HOME/state, inert Claude. No real VPS or account actions. |
| `browser-smoke` | A real Chromium restart that emits `runtime.onStartup`, plus the Playwright dependency. Two-phase browser clear cannot be accepted from extension-worker restarts or synthetic generations. |
| `browser-pairing-ui` | Built desktop frontend, Playwright Chromium and Rust. Renders the App, copies its actual short code and submits a framed request to the real native host; invoke and clipboard are synthetic. This does not prove native WebKit/OS clipboard or a non-developer installation. |
| `remote-task-ui` | Built desktop frontend and Playwright Chromium. ACK loss, App reload, original-task query, full receipt, late-response isolation and separately approved restoration; invoke/SSH/registry are synthetic. Set `LINTEL_REMOTE_TASK_UI_REPORT` for the report and optional `LINTEL_REMOTE_TASK_UI_ARTIFACTS` for external screenshots. |
| `service-ui` | Built desktop frontend and Playwright Chromium. Service inspection, exact approval, lost ACK query, external-edit conflict and separate resume approval; service manager/invoke are synthetic. |
| `linux-vm-runtime` | Linux x86_64, static musl runner, QEMU, cloud-image-utils, OpenSSH client, gpgv and Ubuntu cloud-image public keyring. Creates a disposable Ubuntu guest with real systemd/PAM, synthetic services and users; host policy and production VPS remain outside the test. |
| `desktop-tauri-bundle` | macOS host and Xcode Command Line Tools; builds the native app bundle (`npm run desktop:build`). |

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
extension that accepts local fixture origins only. Its native installation
helper restores its disposable manifest byte for byte, closes the old browser
process, then relaunches without extension-loading flags. It requires a different
native `runtime.onStartup` generation before the second, separately approved
deletion. No profile preferences or startup markers are written by the harness.
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
The synthetic-only `LINTEL_TEST_ACCEPT_BARRIER` records a marker inside
`LINTEL_TEST_HOME` and pauses the worker after durable acceptance/ACK so the
interruption is reproducible; it does not modify normal submissions.

`evidence_complete` means those observations finished, including an observed
logout limitation. It does **not** promise `setsid` survives arbitrary cgroup
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
The VM and its overlay/seed/keys were cleaned up; production VPS and
user-manager behavior remain unverified.

The built service UI can be checked separately with
`python3 tests/verify.py --checks service-ui`. Set `LINTEL_SERVICE_UI_REPORT` for
its JSON and `LINTEL_SERVICE_UI_ARTIFACTS` for optional Git-external screenshots.
It uses real headless Chromium with synthetic service state, not real systemd
or native WebKit. Both new independent checks are explicitly selected in CI;
their actual current results are recorded in [current-state](current-state.md).

These are the same gaps recorded in [current-state](current-state.md#完整目标仍缺少)
and the [acceptance status](acceptance-status.json). Passing the default checks
does not close them.

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

Earlier custom-policy source acceptance: clean `1fb9ab6` [CI run 37125403657](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37125403657) passed 12/12 defaults on both macOS and Ubuntu. The separately selected Linux OpenSSH runtime check passed 1/1 with 0 ignored; it exercised the seven-field custom contract on a real x86_64 static-musl runner, including exact subset changes, frozen receipt, lost ACK recovery and interactive PTY launch. It uses temporary synthetic roots and inert Claude, and does not establish real Claude effects, production VPS state or aarch64 runtime.

Current browser source acceptance: clean `93c3f74` [CI run 37163091952](https://github.com/IndelibleVivi/lintel-cc/actions/runs/37163091952) passed 12/12 defaults and 2/2 explicitly selected browser checks on both macOS arm64 and Ubuntu x86_64. Detailed Chromium 151.0.7922.34 reports retain all 11 smoke assertions, actual process exit/replacement, a new production `runtime.onStartup` generation, no relaunch extension-loading flags, retained identity and completed native receipt. The App copy-to-real-host pairing report passes with synthetic invoke/clipboard. Ubuntu OpenSSH runtime also passes 1/1 with 0 ignored. All five entrypoint reports record the same clean HEAD; packaged-App opt-ins and local Chromium 155 evidence remain separate. Formal browser distributions, AdsPower, real Claude/auth, native WebKit/OS clipboard and non-developer installation remain unverified.

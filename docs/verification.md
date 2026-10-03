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
| `python-ssh-test` | python | `python3 -m unittest discover -s platform/ssh/tests` (fake SSH transport, synthetic state) |
| `desktop-typecheck` | desktop | `npm run typecheck` in `apps/desktop` |
| `desktop-build` | desktop | `npm run build` in `apps/desktop` (tsc + Vite) |
| `journey-cli` | journey | `python3 tests/cli_journey.py` (real CLI JSON boundary) |
| `journey-submission` | journey | `python3 tests/submission_journey.py` (detached durable ACK, replay dedup) |
| `journey-work-preservation` | journey | `python3 tests/work_preservation_journey.py` |
| `journey-launch` | journey | `python3 tests/launch_journey.py` (CLI/TUI real PTY, inert user-local Claude) |
| `journey-policy` | journey | `python3 tests/policy_journey.py` (versioned policy identity and drift) |

The two standalone Rust entrypoints beyond the root workspace (`apps/desktop/src-tauri`,
`extensions/browser/native-host`) are listed because the root `Cargo.toml`
excludes them; verifying only `cargo test --workspace` would silently skip them.

Toolchains (`cargo`, `node`, `npm`, `python3`) are resolved from the caller's
`PATH` and environment. The entrypoint does not install anything, does not probe
another user's home, and does not rewrite `CARGO_HOME`/`RUSTUP_HOME`; a host
where a tool cannot run for environment reasons simply shows the check failing
or skipped, which is that host's environment, not a change to the check.

The default desktop build already runs TypeScript checking; `desktop-typecheck` remains separately selectable.

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
| `desktop-tauri-bundle` | macOS host and Xcode Command Line Tools; builds the native app bundle (`npm run desktop:build`). |

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

Actual VPS logout/cgroup, host reboot, aarch64 runtime and macOS Terminal GUI remain
independent gaps; this Linux fixture does not prove those behaviors.

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

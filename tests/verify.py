#!/usr/bin/env python3
"""Lintel repository verification entrypoint.

A small stdlib registry plus a sequential subprocess runner aggregating the
repository's existing checks (it does not re-implement them). See
docs/verification.md for the check inventory and evidence format.

Guarantees: default checks only touch synthetic temporary roots; toolchains come
from the caller's PATH/environment (no installs, no home probing, no CARGO_HOME/
RUSTUP_HOME rewriting); an un-runnable check is skipped with a reason, never a
pass; real browser/auth/Linux evidence is independent and never a mock/skip;
optional JSON evidence goes outside Git and records command, result, platform,
fixture category, git HEAD + dirty flag, and timestamps.

Run `--list` for checks; `--checks IDS` / `--category NAME` to select; `--all`
for independent checks; `--json [PATH]` for Git-external evidence. Exits
non-zero if any executed check fails.
"""
from __future__ import annotations

import argparse
import dataclasses
import datetime as _dt
import io
import json
import os
import platform
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
from contextlib import redirect_stdout
from pathlib import Path
from typing import Dict, List, Optional, Sequence

ROOT = Path(__file__).resolve().parents[1]

PASS, FAIL, SKIP, DEFER = "passed", "failed", "skipped", "deferred"
LABEL = {PASS: "PASS", FAIL: "FAIL", SKIP: "SKIP", DEFER: "DEFER"}

CARGO = shutil.which("cargo")
NODE = shutil.which("node")
NPM = shutil.which("npm")
PYTHON = shutil.which("python3") or sys.executable
GIT = shutil.which("git")


@dataclasses.dataclass(frozen=True)
class Check:
    """One named verification step backed by an existing repo command."""

    id: str
    description: str
    category: str
    command: Sequence[str]
    cwd: Path = ROOT
    fixture: str = "synthetic-temp-root"
    tools: Sequence[str] = ()             # tools that must resolve on PATH
    paths: Sequence[Path] = ()            # files/dirs that must exist
    loopback: bool = False                # needs loopback socket binding
    requires: str = ""                    # shown by --list
    build: Sequence[Sequence[str]] = ()   # built before the check
    independent: bool = False             # real-runtime evidence; not default
    reason: str = ""                      # shown while deferred

    @property
    def display_command(self) -> str:
        return " ".join(shlex.quote(part) for part in self.command)


def _check(
    id: str, description: str, category: str, *command: str,
    cwd: Path = ROOT, tools: Sequence[str] = (), paths: Sequence[Path] = (),
    loopback: bool = False, requires: str = "", build: Sequence[Sequence[str]] = (),
    independent: bool = False, reason: str = "",
) -> Check:
    return Check(id, description, category, list(command), cwd, "synthetic-temp-root",
                 tools, paths, loopback, requires, build, independent, reason)


_CARGO_BUILD_RUNNER = ([CARGO or "cargo", "build", "-p", "lintel-runner"],)


def _cli_package_binary(system: str, machine: str) -> Optional[Path]:
    if system == "Linux" and machine in ("x86_64", "aarch64", "arm64"):
        arch = "aarch64" if machine in ("aarch64", "arm64") else "x86_64"
        return ROOT / f"target/{arch}-unknown-linux-musl/release/lintel"
    if system == "Darwin" and machine == "arm64":
        return ROOT / "target/debug/lintel"
    return None


_CLI_PACKAGE_BINARY = _cli_package_binary(platform.system(), platform.machine())
if _CLI_PACKAGE_BINARY is not None and os.environ.get("LINTEL_CLI_CANDIDATE_BINARY"):
    _CLI_PACKAGE_BINARY = Path(os.environ["LINTEL_CLI_CANDIDATE_BINARY"]).resolve()
_CLI_PACKAGE_TOOLS = (("node", "tar", "cc", "shasum") if platform.system() == "Darwin"
                      else ("node", "tar", "sha256sum"))

CHECKS: List[Check] = [
    _check("cargo-workspace-test", "cargo test --workspace (core, operations, remote, egress, runner)", "rust",
           *(CARGO or "cargo", "test", "--workspace"), tools=("cargo",), loopback=True,
           requires="cargo; loopback sockets for crates/egress/tests/proxy.rs"),
    _check("desktop-rust-test", "cargo test in standalone Tauri crate apps/desktop/src-tauri",
           "rust", *(CARGO or "cargo", "test", "--manifest-path",
                     "apps/desktop/src-tauri/Cargo.toml"), tools=("cargo",), loopback=True,
           requires="cargo; loopback sockets for desktop network proxy tests"),
    _check("native-host-rust-test", "cargo test in extensions/browser/native-host", "rust",
           *(CARGO or "cargo", "test", "--manifest-path",
             "extensions/browser/native-host/Cargo.toml"), tools=("cargo",)),
    _check("runner-build", "cargo build -p lintel-runner (prerequisite for journeys)", "rust",
           *(CARGO or "cargo", "build", "-p", "lintel-runner"), tools=("cargo",)),
    _check("js-engine-test", "browser engine JS unit tests (node:test, no deps)", "js",
           *(NODE or "node", "--test", "extensions/browser/tests/engine.test.mjs"), tools=("node",)),
    _check("site-game-test", "shared Clawd silhouette, whole-body pose, jump, varied reachable stars, collision and pause physics (node:test, no deps)", "js",
           *(NODE or "node", "--test", "tests/site_game.test.mjs"), tools=("node",)),
    _check("home-greetings-test", "Clawd greeting pools, local date dedup and egg priorities", "js",
           *(NODE or "node", "--experimental-strip-types", "--test", "tests/home_greetings.test.mjs"), tools=("node",)),
    _check("site-ui", "static website responsive story, native reading, theme and Clawd journey", "independent",
           *(NODE or "node", "tests/site_ui_journey.mjs"), tools=("node",),
           paths=(Path(os.environ["PLAYWRIGHT_MODULE"]) if os.environ.get("PLAYWRIGHT_MODULE") else ROOT / "extensions/browser/node_modules/playwright",),
           loopback=True, independent=True, reason="isolated headless Chromium; website only, no native bridge"),
    _check("clawd-app-ui", "shared Clawd runner lifecycle inside the built desktop playroom", "independent",
           *(NODE or "node", "tests/clawd_app_ui_journey.mjs"), tools=("node",),
           paths=(Path(os.environ["PLAYWRIGHT_MODULE"]) if os.environ.get("PLAYWRIGHT_MODULE") else ROOT / "extensions/browser/node_modules/playwright", ROOT / "apps/desktop/dist/index.html"),
           loopback=True, independent=True, reason="built App in isolated Chromium with synthetic inventory; not native WebKit"),
    _check("python-ssh-test", "platform/ssh Python controller tests (fake SSH, synthetic)",
           "python", PYTHON, "-m", "unittest", "discover", "-s", "platform/ssh/tests", "-v"),
    _check("desktop-typecheck", "desktop TypeScript typecheck (tsc --noEmit)", "desktop",
           *(NPM or "npm", "run", "typecheck"), cwd=ROOT / "apps/desktop", tools=("npm",),
           paths=(ROOT / "apps/desktop/node_modules",), requires="apps/desktop/node_modules"),
    _check("desktop-build", "desktop frontend build (tsc + vite build)", "desktop",
           *(NPM or "npm", "run", "build"), cwd=ROOT / "apps/desktop", tools=("npm",),
           paths=(ROOT / "apps/desktop/node_modules",), requires="apps/desktop/node_modules"),
    # Independent / real-runtime evidence: never in the implicit default group.
    _check("cli-candidate", "portable CLI package/extract/native execution and retained-state upgrade", "independent",
           PYTHON, "tests/cli_package_journey.py", *((str(_CLI_PACKAGE_BINARY),) if _CLI_PACKAGE_BINARY else ()),
           tools=_CLI_PACKAGE_TOOLS, paths=(_CLI_PACKAGE_BINARY,) if _CLI_PACKAGE_BINARY else (),
           independent=True, requires="macOS arm64 native CLI or Linux x86_64/aarch64 static musl CLI; Node/tar; macOS cc/shasum or Linux sha256sum for test fixtures/checksums",
           reason="executes the native extracted package; other architectures use visibly synthetic format fixtures"),
    _check("linux-ssh-runtime", "real Linux OpenSSH native install/submit/query/TTY journey", "independent",
           PYTHON, "tests/remote_linux_ssh_journey.py", "target/x86_64-unknown-linux-musl/release/lintel",
           tools=("cargo", "ssh", "ssh-keygen"), paths=(ROOT / "target/x86_64-unknown-linux-musl/release/lintel",),
           loopback=True, requires="Linux x86_64; OpenSSH sshd; static musl runner; shared remote Rust crate",
           independent=True, reason="needs an actual Linux runtime and isolated loopback sshd"),
    _check("browser-smoke", "real Chromium two-phase clear smoke (Playwright, synthetic profile)",
           "independent", *(NODE or "node", "extensions/browser/tests/browser-smoke.mjs"),
           tools=("node", "cargo"), paths=(Path(os.environ["PLAYWRIGHT_MODULE"]) if os.environ.get("PLAYWRIGHT_MODULE") else ROOT / "extensions/browser/node_modules/playwright", ROOT / "apps/desktop/dist/index.html"),
           independent=True, reason="needs built App, Playwright and a real browser restart generation; synthetic invoke, not native WebKit"),
    _check("browser-pairing-ui", "built App copy/real native host pairing/approval journey (synthetic invoke and clipboard)",
           "independent", *(NODE or "node", "tests/browser_pairing_ui_journey.mjs"),
           tools=("node", "cargo"), paths=(
               Path(os.environ["PLAYWRIGHT_MODULE"]) if os.environ.get("PLAYWRIGHT_MODULE") else ROOT / "extensions/browser/node_modules/playwright",
               ROOT / "apps/desktop/node_modules", ROOT / "apps/desktop/dist/index.html"),
           loopback=True, requires="Playwright Chromium; built desktop frontend; Rust; synthetic invoke/clipboard, not native WebKit",
           independent=True, reason="needs Playwright and a built App; real host values and framed native pairing"),
    _check("service-ui", "built App service inspection/approval/conflict/resume (synthetic manager)",
           "independent", *(NODE or "node", "tests/service_ui_journey.mjs"),
           tools=("node",), paths=(
               Path(os.environ["PLAYWRIGHT_MODULE"]) if os.environ.get("PLAYWRIGHT_MODULE") else ROOT / "extensions/browser/node_modules/playwright",
               ROOT / "apps/desktop/dist/index.html"), loopback=True,
           independent=True, reason="headless Chromium UI evidence; service manager and invoke are synthetic"),
    _check("work-ui", "built App exact selection, archive/preserve and portable import (real synthetic core)", "independent", *(NODE or "node", "tests/work_ui_journey.mjs"), tools=("node", "cargo"), paths=(Path(os.environ["PLAYWRIGHT_MODULE"]) if os.environ.get("PLAYWRIGHT_MODULE") else ROOT / "extensions/browser/node_modules/playwright", ROOT / "apps/desktop/dist/index.html"), loopback=True, build=_CARGO_BUILD_RUNNER, independent=True, reason="isolated headless Chromium and invoke fixture; not native WebKit"),
    _check("components-ui", "built component/original task/metadata capacity journey (real synthetic core)", "independent", *(NODE or "node", "tests/components_ui_journey.mjs"), tools=("node", "cargo"), paths=(Path(os.environ["PLAYWRIGHT_MODULE"]) if os.environ.get("PLAYWRIGHT_MODULE") else ROOT / "extensions/browser/node_modules/playwright", ROOT / "apps/desktop/dist/index.html"), loopback=True, build=_CARGO_BUILD_RUNNER, independent=True, reason="Chromium + real synthetic cores; finite SSH/invoke modeled; not native WebKit/production"),
    _check("baseline-ui", "built product-baseline task/session/Agent/approval journey (real synthetic core)", "independent", *(NODE or "node", "tests/baseline_ui_journey.mjs"), tools=("node", "cargo"), paths=(Path(os.environ["PLAYWRIGHT_MODULE"]) if os.environ.get("PLAYWRIGHT_MODULE") else ROOT / "extensions/browser/node_modules/playwright", ROOT / "apps/desktop/dist/index.html"), loopback=True, build=_CARGO_BUILD_RUNNER, independent=True, reason="Chromium + real synthetic core; static CLI/clipboard/launch-error boundary modeled; not native WebKit/Claude"),
    _check("archive-wait-ui", "built App delayed package read, accessible feedback and cross-page queue lifecycle", "independent", *(NODE or "node", "tests/archive_wait_ui_journey.mjs"), tools=("node",), paths=(Path(os.environ["PLAYWRIGHT_MODULE"]) if os.environ.get("PLAYWRIGHT_MODULE") else ROOT / "extensions/browser/node_modules/playwright", ROOT / "apps/desktop/dist/index.html"), loopback=True, independent=True, reason="isolated Chromium + delayed synthetic invoke; not decryption performance or native WebKit"),
    _check("drift-ui", "built App drift result and acceptance bound to host/environment/request generation", "independent", *(NODE or "node", "tests/drift_ui_journey.mjs"), tools=("node",), paths=(Path(os.environ["PLAYWRIGHT_MODULE"]) if os.environ.get("PLAYWRIGHT_MODULE") else ROOT / "extensions/browser/node_modules/playwright", ROOT / "apps/desktop/dist/index.html"), loopback=True, independent=True, reason="isolated Chromium + controlled synthetic invoke; not native WebKit"),
    _check("work-scale", "release 256 MiB/member, 1 GiB package and 10,000-file runtime evidence", "independent", PYTHON, "tests/work_scale_journey.py", paths=(Path(os.environ.get("LINTEL_SCALE_RUNNER", str(ROOT / "target/release/lintel"))),), independent=True, requires="explicit release runner and temporary filesystem with >=16 GiB free; LINTEL_SCALE_RUNNER/LINTEL_SCALE_TMPDIR/LINTEL_SCALE_REPORT select local evidence paths", reason="slow independent capacity/resource acceptance; never implicit default or production data"),
    _check("remote-task-ui", "built App original SSH task/reopen/conflict/separate restore (synthetic transport)",
           "independent", *(NODE or "node", "tests/remote_task_ui_journey.mjs"),
           tools=("node",), paths=(
               Path(os.environ["PLAYWRIGHT_MODULE"]) if os.environ.get("PLAYWRIGHT_MODULE") else ROOT / "extensions/browser/node_modules/playwright",
               ROOT / "apps/desktop/dist/index.html"), loopback=True,
           independent=True, reason="headless Chromium recovery UI evidence; SSH/invoke/registry are synthetic"),
    _check("linux-vm-control-test", "disposable VM launcher and barrier control-flow tests (no VM)", "python",
           PYTHON, "-m", "unittest", "discover", "-s", "tests/fixtures/linux_vm", "-v"),
    _check("linux-vm-runtime", "disposable Ubuntu VM systemd/PAM/logout/cgroup/reboot journey", "independent",
           PYTHON, "tests/linux_vm_journey.py", "--runner", "target/x86_64-unknown-linux-musl/release/lintel",
           "--json", os.environ.get("LINTEL_VM_REPORT", str(Path(tempfile.gettempdir()) / "lintel-vm-runtime.json")),
           tools=("qemu-system-x86_64", "qemu-img", "cloud-localds", "ssh", "ssh-keygen", "gpgv"),
           paths=(ROOT / "target/x86_64-unknown-linux-musl/release/lintel",), loopback=True,
           independent=True, requires="Linux x86_64; QEMU; cloud-image-utils; ubuntu cloudimage public keyring; official image download",
           reason="real disposable VM; never the operator's VPS or login policy"),
    _check("desktop-tauri-bundle", "native macOS Tauri app bundle build (npm run desktop:build)",
           "independent", *(NPM or "npm", "run", "desktop:build"), cwd=ROOT / "apps/desktop",
           tools=("npm", "cargo"), requires="Xcode Command Line Tools and a macOS host",
           independent=True, reason="native bundle + real WebKit runtime; macOS-only"),
]

CHECKS.append(_check("shared-remote-journey", "shared finite SSH public API and target request contracts (synthetic transport)", "python", PYTHON, "tests/shared_remote_journey.py", tools=("cargo",), paths=(ROOT / "tests/shared_remote_journey.py",)))

# Synthetic end-to-end journeys sharing the runner-build prerequisite.
for _id, _desc, _file in (
    ("journey-agent-cli", "agent CLI contract, portable work, independent import and original-job wait", "agent_cli_journey.py"),
    ("journey-product-baseline", "readonly context/help, frozen targets, bounded session and immutable startup contracts", "backend_baseline_journey.py"),
    ("journey-agent-adapters", "finite CLI adapters and foreground network lifecycle", "agent_adapters_journey.py"),
    ("journey-portable-work", "portable archive in independent state and persisted failure codes", "portable_work_journey.py"),
    ("journey-cli", "CLI journey (register/preview/approve/apply/restore)", "cli_journey.py"),
    ("journey-submission", "detached submission journey (durable ACK, replay dedup)",
     "submission_journey.py"),
    ("journey-execution-admission", "generic execute/submit rejects launch, resume and unsupported plan kinds before acceptance", "execution_admission_journey.py"),
    ("journey-work-preservation", "work-preservation journey (large session, A->B->C, damaged settings)",
     "work_preservation_journey.py"),
    ("journey-components", "finite readonly component sources, original records and strict CLI scope", "components_journey.py"),
    ("journey-large-work", "streaming large work archive/carry/import/preserve, bounded pages and isolated RSS",
     "large_work_journey.py"),
    ("journey-work-capacity", "read-only work-capacity preflight (metadata-only admission, blockers, strict schema)",
     "work_capacity_journey.py"),
    ("journey-plan-capacity", "long-path plan admission, readable persisted state and exact-scope recovery", "plan_capacity_journey.py"),
    ("journey-launch", "interactive CLI/TUI launch and custom policy (real PTY, synthetic roots)", "launch_journey.py"),
    ("journey-resume-staging", "successful PTY resume exec removes full-package plaintext and retains only the approved copy", "resume_staging_journey.py"),
    ("journey-policy", "versioned policy journey (version/value/ownership compatibility)", "policy_journey.py"),
):
    CHECKS.append(_check(_id, _desc, "journey", PYTHON, f"tests/{_file}", tools=("cargo",),
                         paths=(ROOT / "tests" / _file,), build=_CARGO_BUILD_RUNNER))


def select(args: argparse.Namespace) -> List[Check]:
    chosen = list(CHECKS)
    if args.checks:
        wanted = [item.strip() for item in args.checks.split(",") if item.strip()]
        unknown = [item for item in wanted if item not in {check.id for check in CHECKS}]
        if unknown:
            raise SystemExit(f"unknown check id(s): {', '.join(unknown)}")
        chosen = [check for check in chosen if check.id in wanted]
    if args.category:
        chosen = [check for check in chosen if check.category == args.category]
    # Independent checks are excluded from the implicit default group. Naming
    # them explicitly, selecting their category, or --all is the opt-in.
    if not args.all and not (args.checks or args.category):
        chosen = [check for check in chosen if not check.independent and check.id != "desktop-typecheck"]
    return chosen


def git_state() -> Dict[str, object]:
    info: Dict[str, object] = {"head": None, "dirty": None, "dirty_entries": None}
    if not GIT:
        return info
    try:
        head = subprocess.run([GIT, "rev-parse", "HEAD"], cwd=ROOT,
                              capture_output=True, text=True, timeout=20)
        if head.returncode == 0:
            info["head"] = head.stdout.strip()
        status = subprocess.run([GIT, "status", "--porcelain"], cwd=ROOT,
                                capture_output=True, text=True, timeout=30)
        if status.returncode == 0:
            entries = [line for line in status.stdout.splitlines() if line.strip()]
            info["dirty_entries"] = len(entries)
            info["dirty"] = bool(entries)
    except (OSError, subprocess.SubprocessError):
        pass
    return info


def run(argv: Sequence[str], cwd: Path, timeout: float) -> Dict[str, object]:
    started = time.monotonic()
    try:
        result = subprocess.run(argv, cwd=str(cwd), text=True, capture_output=True,
                                timeout=timeout)
        return {"exit_code": result.returncode, "stdout": result.stdout,
                "stderr": result.stderr, "timed_out": False,
                "duration_seconds": round(time.monotonic() - started, 3)}
    except subprocess.TimeoutExpired as exc:
        return {"exit_code": None, "stdout": exc.stdout or "",
                "stderr": f"{exc.stderr or ''}\n[verify.py] timed out after {timeout}s",
                "timed_out": True, "duration_seconds": round(time.monotonic() - started, 3)}
    except OSError as exc:
        return {"exit_code": None, "stdout": "",
                "stderr": f"[verify.py] failed to launch: {exc}", "timed_out": False,
                "duration_seconds": round(time.monotonic() - started, 3)}


def tail(text: object, limit: int = 3000) -> str:
    value = str(text)
    return value if len(value) <= limit else "...[truncated]...\n" + value[-limit:]


def run_check(check: Check, args: argparse.Namespace) -> Dict[str, object]:
    record: Dict[str, object] = {
        "id": check.id, "description": check.description, "category": check.category,
        "command": check.display_command,
        "cwd": (str(check.cwd.relative_to(ROOT)) if check.cwd.is_relative_to(ROOT)
                else str(check.cwd)),
        "fixture_category": check.fixture, "loopback": check.loopback,
        "independent": check.independent,
    }
    if check.requires:
        record["requires"] = check.requires

    def finish(status: str, reason: str = "", duration: float = 0.0, **extra: object) -> Dict[str, object]:
        record.update(status=status, duration_seconds=duration)
        if reason:
            record["reason"] = reason
        record.update(extra)
        return record

    # Independent checks are omitted from the implicit default group. When the
    # caller opts in (--all or an explicit selection) they run for real, or skip
    # if their prerequisites are missing -- never a pass by mock.
    explicit = bool(args.all or getattr(args, "checks", None) or getattr(args, "category", None))
    if check.independent and not explicit:
        return finish(DEFER, check.reason)
    if check.id == "cli-candidate" and _cli_package_binary(platform.system(), platform.machine()) is None:
        return finish(SKIP, f"unsupported host {platform.system()}/{platform.machine()}; requires macOS arm64 or Linux x86_64/aarch64")
    if not check.cwd.is_dir():
        return finish(SKIP, f"working directory missing: {check.cwd}")
    missing_tool = next((tool for tool in check.tools if shutil.which(tool) is None), None)
    if missing_tool:
        return finish(SKIP, f"required tool '{missing_tool}' not found on PATH")
    missing_path = next((path for path in check.paths if not path.exists()), None)
    if missing_path:
        shown = missing_path.relative_to(ROOT) if missing_path.is_relative_to(ROOT) else missing_path
        return finish(SKIP, f"prerequisite missing: {shown}")

    started = time.monotonic()
    builds: List[Dict[str, object]] = []
    if args.build:
        for build_argv in check.build:
            build = run(build_argv, ROOT, args.timeout)
            builds.append({"command": " ".join(shlex.quote(p) for p in build_argv),
                           "exit_code": build["exit_code"],
                           "duration_seconds": build["duration_seconds"],
                           "stderr_tail": tail(build["stderr"])})
            if build["exit_code"] != 0:
                return finish(FAIL, "prerequisite build failed", round(time.monotonic() - started, 3),
                              builds=builds, stdout_tail="", stderr_tail=tail(build["stderr"]))

    result = run(check.command, check.cwd, args.timeout)
    status = PASS if result["exit_code"] == 0 else FAIL
    extra: Dict[str, object] = {"exit_code": result["exit_code"], "timed_out": result["timed_out"],
                                "stdout_tail": tail(result["stdout"]),
                                "stderr_tail": tail(result["stderr"])}
    if builds:
        extra["builds"] = builds
    return finish(status, "", result["duration_seconds"], **extra)


def print_list(checks: Sequence[Check]) -> None:
    width = max((len(check.id) for check in checks), default=20)
    for check in checks:
        marker = " [loopback]" if check.loopback else ""
        print(f"{check.id:<{width}}  {check.category:<11}{marker}  {check.description}")
        if check.requires:
            print(f"{'':<{width}}  requires: {check.requires}")


def print_results(records: Sequence[Dict[str, object]]) -> None:
    print()
    print("=" * 78)
    for record in records:
        status = str(record["status"])
        print(f"[{LABEL.get(status, status.upper())}] {record['id']} ({record['duration_seconds']}s)")
        if status in (SKIP, DEFER):
            print(f"        {record.get('reason', '')}")
        elif status == FAIL:
            for line in str(record.get("stderr_tail", "")).strip().splitlines()[-6:]:
                print(f"        | {line}")
        elif status == PASS:
            lines = str(record.get("stdout_tail", "")).strip().splitlines()
            if lines:
                print(f"        {lines[-1]}")
    print("=" * 78)
    counts: Dict[str, int] = {}
    for record in records:
        counts[str(record["status"])] = counts.get(str(record["status"]), 0) + 1
    print("summary: " + ", ".join(f"{LABEL.get(k, k)}={v}" for k, v in sorted(counts.items())))


def default_evidence_path() -> Path:
    return Path(os.environ.get("TMPDIR") or tempfile.gettempdir()) / "lintel-verify" / "evidence.json"


def build_evidence(records: Sequence[Dict[str, object]], args: argparse.Namespace,
                   started: _dt.datetime, finished: _dt.datetime) -> Dict[str, object]:
    def version(tool: Optional[str], *extra: str) -> Optional[str]:
        if not tool:
            return None
        try:
            out = subprocess.run([tool, *extra], capture_output=True, text=True, timeout=30)
        except (OSError, subprocess.SubprocessError):
            return None
        lines = (out.stdout or out.stderr).strip().splitlines()
        return lines[0].strip() if lines else None

    return {
        "schema": "lintel.verify/v1",
        "generated_by": "tests/verify.py",
        "product": "Lintel",
        "root": str(ROOT),
        "platform": {"system": platform.system(), "release": platform.release(),
                     "machine": platform.machine(), "python": platform.python_version()},
        "tool_versions": {"cargo": version(CARGO, "--version"),
                          "rustc": version(shutil.which("rustc"), "--version"),
                          "node": version(NODE, "--version"), "npm": version(NPM, "--version"),
                          "python3": version(PYTHON, "--version")},
        "git": git_state(),
        "selection": {"all": args.all, "checks": getattr(args, "checks", None),
                      "category": getattr(args, "category", None)},
        "started_at": started.isoformat(), "finished_at": finished.isoformat(),
        "duration_seconds": round((finished - started).total_seconds(), 3),
        "results": list(records),
    }


def run_self_test() -> int:
    """Exercise this entrypoint's own behavior with throwaway commands only."""
    failures: List[str] = []

    def expect(condition: bool, message: str) -> None:
        print(("ok   - " if condition else "FAIL - ") + message)
        if not condition:
            failures.append(message)

    def ns(**kw: object) -> argparse.Namespace:
        base = {"checks": None, "category": None, "all": False}
        base.update(kw)
        return argparse.Namespace(**base)
    ids = [check.id for check in CHECKS]
    expect(len(ids) == len(set(ids)), "check ids are unique")
    expect(all(check.command for check in CHECKS), "every check has a command")
    expect("journey-policy" in ids, "policy journey is registered in the default group")
    default_ids = {check.id for check in select(ns())}
    expect({"journey-cli", "journey-policy"} <= default_ids, "default group includes journey checks")
    expect("browser-smoke" not in default_ids, "default group excludes independent checks")
    every = {check.id for check in select(ns(all=True))}
    expect({"browser-smoke", "desktop-tauri-bundle"} <= every, "--all includes independent checks")
    expect([c.id for c in select(ns(checks="browser-smoke"))] == ["browser-smoke"],
           "explicit --checks can name an independent check")
    expect(select(ns(category="independent")), "--category independent selects independent checks")
    try:
        select(ns(checks="does-not-exist"))
        expect(False, "unknown --checks id is rejected")
    except SystemExit:
        expect(True, "unknown --checks id is rejected")

    expect({"head", "dirty", "dirty_entries"} <= set(git_state()),
           "git_state exposes head/dirty/dirty_entries")
    expect(not default_evidence_path().resolve().is_relative_to(ROOT.resolve()),
           "default evidence path is outside the repository")

    with tempfile.TemporaryDirectory(prefix="lintel-verify-selftest-") as temp:
        args = argparse.Namespace(all=False, build=False, timeout=30.0)
        ok = lambda **kw: Check("s", "self-test", "selftest", [PYTHON, "-c", "print(1)"], Path(temp), **kw)
        bad = Check("s", "self-test", "selftest", [PYTHON, "-c", "import sys; sys.exit(3)"], Path(temp))
        expect(run_check(ok(), args)["status"] == PASS, "runner reports PASS on exit 0")
        expect(run_check(bad, args)["status"] == FAIL, "runner reports FAIL on non-zero exit")
        expect(run_check(ok(tools=("no-such-tool-xyz",)), args)["status"] == SKIP,
               "runner reports SKIP on missing tool")
        deferred = Check("defer", "self-test", "selftest", [], Path(temp), independent=True, reason="x")
        expect(run_check(deferred, args)["status"] == DEFER, "runner defers independent checks by default")
        opted_in = argparse.Namespace(all=True, build=False, timeout=30.0)
        expect(run_check(ok(independent=True, reason="x", paths=(Path(temp) / "absent",)),
                         opted_in)["status"] == SKIP,
               "opted-in independent check skips (not passes) when prerequisites are missing")

        from unittest.mock import patch
        for machine, arch in (("x86_64", "x86_64"), ("aarch64", "aarch64"), ("arm64", "aarch64")):
            expect(_cli_package_binary("Linux", machine) == ROOT / f"target/{arch}-unknown-linux-musl/release/lintel",
                   f"candidate selects native Linux {machine} binary")
        expect(_cli_package_binary("Darwin", "arm64") == ROOT / "target/debug/lintel",
               "candidate retains native Mac input")
        for system, machine in (("Darwin", "x86_64"), ("Linux", "riscv64"), ("Windows", "AMD64")):
            expect(_cli_package_binary(system, machine) is None,
                   f"candidate has no native input on unsupported {system}/{machine}")
            with patch.object(platform, "system", return_value=system), \
                    patch.object(platform, "machine", return_value=machine), \
                    patch(__name__ + ".run", side_effect=AssertionError("unsupported journey launched")):
                candidate_check = next(c for c in CHECKS if c.id == "cli-candidate")
                result = run_check(candidate_check, opted_in)
                expect(result["status"] == SKIP and "unsupported host" in str(result.get("reason")),
                       f"candidate skips unsupported {system}/{machine} before launching")
        candidate = next(c for c in CHECKS if c.id == "cli-candidate")
        prebuilt = Path(temp) / "native-cli"
        prebuilt.touch()
        candidate = dataclasses.replace(candidate, paths=(prebuilt,))
        missing_tools = ("cc", "shasum") if platform.system() == "Darwin" else ("sha256sum",)
        for missing in missing_tools:
            with patch.object(shutil, "which", side_effect=lambda name, missing=missing: None if name == missing else "/synthetic/tool"), \
                 patch(__name__ + ".run", return_value={"exit_code": 3, "duration_seconds": 0,
                       "timed_out": False, "stdout": "", "stderr": "unexpected journey launch"}) as launched:
                result = run_check(candidate, opted_in)
                expect(result["status"] == SKIP and missing in result.get("reason", "")
                       and not launched.called,
                       f"candidate skips missing {missing} before launching a prebuilt-native journey")

        now = _dt.datetime.now(_dt.timezone.utc)
        doc = build_evidence([run_check(ok(), args)], args, now, now)
        expect(doc["schema"] == "lintel.verify/v1", "evidence schema id present")
        expect(bool(doc["started_at"] and doc["finished_at"]), "evidence records timestamps")
        expect(bool(doc["platform"]["system"]), "evidence records platform")
        expect({"head", "dirty"} <= set(doc["git"]), "evidence records git head and dirty flag")
        expect(doc["results"][0]["fixture_category"] == "synthetic-temp-root",
               "evidence records fixture category")

    buffer = io.StringIO()
    with redirect_stdout(buffer):
        print_list(CHECKS)
    expect(buffer.getvalue().count("\n") >= len(CHECKS), "--list renders at least one line per check")

    print()
    if failures:
        print(f"self-test: {len(failures)} failure(s)", file=sys.stderr)
        return 1
    print("self-test: all checks passed")
    return 0


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="Lintel repository verification entrypoint")
    parser.add_argument("--list", action="store_true", help="list checks and exit")
    parser.add_argument("--self-test", dest="self_test", action="store_true",
                        help="test this entrypoint's own behavior")
    parser.add_argument("--checks", metavar="IDS", help="comma-separated check ids")
    parser.add_argument("--category", metavar="NAME", help="run only this category")
    parser.add_argument("--all", action="store_true", help="include independent checks")
    parser.add_argument("--json", metavar="PATH", nargs="?", const=str(default_evidence_path()),
                        default=None, help=f"write JSON evidence (default: {default_evidence_path()})")
    parser.add_argument("--no-build", dest="build", action="store_false",
                        help="skip prerequisite builds")
    parser.add_argument("--timeout", type=float, default=1800.0, help="per-check timeout seconds")
    args = parser.parse_args(argv)

    if args.self_test:
        return run_self_test()
    if args.list:
        print_list(CHECKS)
        return 0

    try:
        chosen = select(args)
    except SystemExit as exc:
        print(str(exc), file=sys.stderr)
        return 2
    if not chosen:
        print("no checks selected", file=sys.stderr)
        return 2

    print(f"Lintel verification: {len(chosen)} check(s) on {platform.system()}/{platform.machine()}")
    state = git_state()
    print(f"git HEAD: {state.get('head')} ({'dirty' if state.get('dirty') else 'clean'})")
    print(f"toolchains: cargo={bool(CARGO)} node={bool(NODE)} npm={bool(NPM)} python3={PYTHON}")

    started = _dt.datetime.now(_dt.timezone.utc)
    records = []
    built = set()
    for check in chosen:
        print(f"\n>>> {check.id}: {check.display_command}")
        check = dataclasses.replace(check, build=tuple(b for b in check.build if tuple(b) not in built))
        record = run_check(check, args)
        if record["status"] == PASS:
            built.add(tuple(check.command))
            built.update(tuple(b) for b in check.build)
        records.append(record)
        print(f"    -> {LABEL.get(str(record['status']), record['status'])}")
    finished = _dt.datetime.now(_dt.timezone.utc)
    print_results(records)

    if args.json:
        path = Path(args.json).resolve()
        if path.is_relative_to(ROOT.resolve()):
            print("evidence path must be outside the repository", file=sys.stderr)
            return 2
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(build_evidence(records, args, started, finished), indent=2) + "\n")
        print(f"\nevidence written: {path}")

    failed = sum(1 for record in records if record["status"] == FAIL)
    if failed:
        print(f"\n{failed} check(s) failed", file=sys.stderr)
        return 1
    deferred = sum(1 for record in records if record["status"] == DEFER)
    if deferred:
        print(f"\nnote: {deferred} independent check(s) not run; see docs/verification.md", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())

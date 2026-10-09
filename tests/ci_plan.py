#!/usr/bin/env python3
"""Finite CI planning for Lintel.

`tests/verify.py` stays the single provider-neutral verification entrypoint with
its existing implicit default group. This module adds an explicit, testable CI
plan **on top of** that entrypoint:

* `classify_path` maps a changed repository path to one finite *area*.
* `PROFILES` maps a finite profile name to a finite set of jobs, each job owning
  a finite, explicit list of `verify.py` check ids and the platforms it runs on.
* `plan_for_event` turns a GitHub event plus the changed paths into one plan,
  failing closed to `full` for a missing base, an unknown path, or a change to
  the CI controller itself.

It never runs a check, installs anything, or touches the network, credentials or
host state. `verify.py` owns check execution; this module owns only *selection*.

The workflow calls this through its CLI and passes the resulting per-job check
lists to `python3 tests/verify.py --checks ...`. `--self-check` (used by
`tests/ci_plan_test.py`) proves the plan covers the entrypoint's whole registry
without silently dropping a check.

See docs/verification.md for the CI contract.
"""
from __future__ import annotations

import argparse
import dataclasses
import fnmatch
import importlib.util
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Dict, FrozenSet, Iterable, List, Optional, Sequence, Tuple

ROOT = Path(__file__).resolve().parents[1]

# ---------------------------------------------------------------------------
# Areas
# ---------------------------------------------------------------------------
# A finite, ordered set of changed-path areas. `unknown` is a real area: any
# path that matches no rule classifies as `unknown` and fails the plan closed to
# `full`. `ci` is the CI controller itself; a change there also fails closed.

AREA_CI = "ci"
AREA_DOCS = "docs"
AREA_SITE = "site"
AREA_APP_UI = "app-ui"
AREA_EXTENSIONS = "extensions"
AREA_CORE = "core"
AREA_UNKNOWN = "unknown"

AREAS: Tuple[str, ...] = (
    AREA_CI, AREA_DOCS, AREA_SITE, AREA_APP_UI, AREA_EXTENSIONS, AREA_CORE, AREA_UNKNOWN,
)

# Exact repository-relative paths owned by the CI controller. A change here is
# what the plan keys on, so it must always re-run the full matrix to avoid a
# stale or self-referential selection.
_CI_EXACT = frozenset({
    "tests/verify.py",
    "tests/ci_plan.py",
    "tests/ci_plan_test.py",
    "tests/requirements-ci.txt",
})

# Repository-owned product directories, matched before the generic markdown
# rule: a Markdown file *inside* a product directory (an App/native/extension/
# website resource, guide or fixture) belongs to that directory's area, not to
# `docs`. Order matters because fnmatch's `*` also matches `/`.
_OWNER_DIR_RULES: Tuple[Tuple[str, str], ...] = (
    # Native App crate is core/desktop-rust evidence, not pure frontend UI.
    (AREA_CORE, "apps/desktop/src-tauri/**"),
    (AREA_APP_UI, "apps/desktop/**"),
    (AREA_SITE, "apps/site/**"),
    (AREA_SITE, "assets/preview/**"),
    (AREA_SITE, "scripts/prepare-site.mjs"),
    (AREA_SITE, "scripts/prepare-preview-art.mjs"),
    (AREA_EXTENSIONS, "extensions/**"),
    # Broad product/native surface: any of these can invalidate core, shared
    # protocol, runner or heavy-work evidence, so they stay on the conservative
    # `core` profile.
    (AREA_CORE, "crates/**"),
    (AREA_CORE, "apps/runner/**"),
    (AREA_CORE, "contracts/**"),
    (AREA_CORE, "platform/**"),
    (AREA_CORE, "scripts/**"),
    (AREA_SITE, "tests/site_ui_journey.mjs"),
    (AREA_SITE, "tests/site_game.test.mjs"),
    (AREA_APP_UI, "tests/*_ui_journey.mjs"),
    (AREA_APP_UI, "tests/home_greetings.test.mjs"),
    (AREA_CORE, "tests/**"),
)

# (area, glob) rules, evaluated in order; the first match wins. Owner
# directories come first so their internal Markdown/content is not misread as
# `docs`; only then do the CI-controller, explicit docs and top-level config
# rules apply.
_PATH_RULES: Tuple[Tuple[str, str], ...] = (
    # CI controller: any `.github/**` change (including a workflow, template or
    # issue-template Markdown file) fails the plan closed.
    (AREA_CI, ".github/workflows/**"),
    (AREA_CI, ".github/**"),
    *_OWNER_DIR_RULES,
    # Explicit docs identities and the docs tree.
    (AREA_DOCS, "AGENTS.md"),
    (AREA_DOCS, "README.md"),
    (AREA_DOCS, "README.en.md"),
    (AREA_DOCS, "docs/**"),
    (AREA_DOCS, "LICENSE"),
    (AREA_DOCS, "LICENSE.*"),
    # Top-level-only Markdown: a `*.md` at the repository root. Owner-dir rules
    # above already claimed product-internal Markdown, so this cannot reach into
    # `apps/`, `crates/`, `extensions/`, `tests/`, `scripts/` or `docs/`.
    (AREA_DOCS, "*.md"),
    # Top-level build/config files (root only, no `/` in the path).
    (AREA_CORE, "Cargo.toml"),
    (AREA_CORE, "Cargo.lock"),
    (AREA_CORE, "rust-toolchain*"),
    (AREA_CORE, "package.json"),
    (AREA_CORE, "package-lock.json"),
    (AREA_CORE, "*.toml"),
    (AREA_CORE, "*.json"),
)


def classify_path(path: str) -> str:
    """Return the finite area for one repository-relative changed path.

    Deleted and renamed paths are classified by this same rule; the caller is
    responsible for passing every side of a rename.
    """
    normalized = path
    while normalized.startswith("./"):
        normalized = normalized[2:]
    if normalized.startswith("/"):
        normalized = normalized[1:]
    if not normalized:
        return AREA_UNKNOWN
    if normalized in _CI_EXACT:
        return AREA_CI
    for area, pattern in _PATH_RULES:
        if fnmatch.fnmatchcase(normalized, pattern):
            return area
    return AREA_UNKNOWN


def classify_paths(paths: Iterable[str]) -> FrozenSet[str]:
    """Return the set of areas touched by an iterable of changed paths."""
    areas = set()
    shared = {"apps/site/clawd-game.mjs", "apps/site/clawd-game.css",
              "tests/site_game.test.mjs"}
    for path in paths:
        areas.add(classify_path(path))
        if path.removeprefix("./") in shared:
            areas.update((AREA_SITE, AREA_APP_UI))
    return frozenset(areas)


# ---------------------------------------------------------------------------
# Check groups (mirror tests/verify.py ids; validated by --self-check)
# ---------------------------------------------------------------------------
# These are *selection* names handed to `verify.py --checks`. An empty tuple
# means the job does not run for this profile.

# Sentinels owned by the CI plan job itself (not verify.py checks).
SELFTEST_CHECKS: Tuple[str, ...] = ("__verify_self_test__", "__ci_plan_test__")

FRONTEND_CHECKS: Tuple[str, ...] = (
    "js-engine-test", "site-game-test", "home-greetings-test", "app-release-test",
    "python-ssh-test", "linux-vm-control-test", "desktop-typecheck", "desktop-build",
)
NATIVE_CHECKS: Tuple[str, ...] = (
    "cargo-workspace-test", "desktop-rust-test", "native-host-rust-test", "runner-build",
    "shared-remote-journey", "journey-network", "journey-agent-cli",
    "journey-product-baseline", "journey-agent-adapters", "journey-components",
    "journey-cli", "journey-submission", "journey-execution-admission",
    "journey-launch", "journey-policy",
)
HEAVY_CHECKS: Tuple[str, ...] = (
    "journey-portable-work", "journey-work-preservation", "journey-large-work",
    "journey-work-capacity", "journey-plan-capacity", "journey-resume-staging",
)
# Built App + browser UI checks that require a built frontend.
APP_UI_CHECKS: Tuple[str, ...] = (
    "browser-smoke", "browser-pairing-ui", "service-ui", "remote-task-ui", "work-ui",
    "components-ui", "baseline-ui", "archive-wait-ui", "drift-ui", "network-ui",
    "telemetry-ui", "app-update-ui", "clawd-app-ui",
)
SITE_UI_CHECKS: Tuple[str, ...] = ("site-ui",)
RUNNER_CHECKS: Tuple[str, ...] = ("linux-ssh-runtime", "cli-candidate")
VM_CHECKS: Tuple[str, ...] = ("linux-vm-runtime",)

# Independent/real-runtime checks that intentionally never run in routine CI.
# Keep the reason next to each so a future reader does not "fix" the omission.
EXCLUDED_FROM_CI: Dict[str, str] = {
    "work-scale": "release-capacity runtime evidence (four 256 MiB members, >=16 GiB temp free); explicitly local/manual, not routine CI",
    "desktop-tauri-bundle": "native macOS Tauri bundle + real WebKit build; needs macOS/Xcode and stays out of the synthetic Linux-first matrix",
}

LINUX = "ubuntu-24.04"
MACOS = "macos-latest"
BOTH: Tuple[str, ...] = (MACOS, LINUX)
LINUX_ONLY: Tuple[str, ...] = (LINUX,)


@dataclasses.dataclass(frozen=True)
class Job:
    id: str
    checks: Tuple[str, ...]
    platforms: Tuple[str, ...]

    def runs(self) -> bool:
        return bool(self.checks)


@dataclasses.dataclass(frozen=True)
class Profile:
    name: str
    reason: str
    jobs: Tuple[Job, ...]

    def job(self, job_id: str) -> Job:
        for candidate in self.jobs:
            if candidate.id == job_id:
                return candidate
        raise KeyError(job_id)

    def platforms_of(self) -> Tuple[str, ...]:
        seen: List[str] = []
        for job in self.jobs:
            for platform in job.platforms:
                if platform not in seen:
                    seen.append(platform)
        return tuple(seen)


def _job(job_id: str, checks: Sequence[str], platforms: Sequence[str]) -> Job:
    return Job(job_id, tuple(checks), tuple(platforms))


# Finite profiles. `full` is the fail-closed default and runs everything the
# routine CI matrix is allowed to run (excluding EXCLUDED_FROM_CI) on both
# platforms plus the Linux VM. The light profiles are deliberately narrow; when
# in doubt `classify_path`/`PROFILE_BY_AREA` escalate toward `core`/`full`.
PROFILES: Dict[str, Profile] = {
    "full": Profile(
        "full",
        "fail-closed default: both platforms, heavy work and Linux VM",
        (
            _job("control", SELFTEST_CHECKS, LINUX_ONLY),
            _job("frontend", FRONTEND_CHECKS, BOTH),
            _job("native", NATIVE_CHECKS, BOTH),
            _job("heavy", HEAVY_CHECKS, BOTH),
            _job("browser", APP_UI_CHECKS + SITE_UI_CHECKS, BOTH),
            _job("runner", RUNNER_CHECKS, BOTH),
            _job("vm", VM_CHECKS, LINUX_ONLY),
        ),
    ),
    "docs": Profile(
        "docs",
        "documentation/identity-markdown only: controller self-tests",
        (
            _job("control", SELFTEST_CHECKS, LINUX_ONLY),
            _job("frontend", (), LINUX_ONLY),
            _job("native", (), LINUX_ONLY),
            _job("heavy", (), LINUX_ONLY),
            _job("browser", (), LINUX_ONLY),
            _job("runner", (), LINUX_ONLY),
            _job("vm", (), LINUX_ONLY),
        ),
    ),
    "site": Profile(
        "site",
        "standalone website only: shared game physics and website Chromium journey (Linux first)",
        (
            _job("control", SELFTEST_CHECKS, LINUX_ONLY),
            _job("frontend", ("site-game-test", "app-release-test"), LINUX_ONLY),
            _job("native", (), LINUX_ONLY),
            _job("heavy", (), LINUX_ONLY),
            _job("browser", SITE_UI_CHECKS, LINUX_ONLY),
            _job("runner", (), LINUX_ONLY),
            _job("vm", (), LINUX_ONLY),
        ),
    ),
    "app-ui": Profile(
        "app-ui",
        "desktop frontend / built-App UI only: frontend build plus applicable built-App/browser checks (Linux first)",
        (
            _job("control", SELFTEST_CHECKS, LINUX_ONLY),
            _job("frontend", ("desktop-typecheck", "desktop-build", "home-greetings-test"),
                 LINUX_ONLY),
            _job("native", (), LINUX_ONLY),
            _job("heavy", (), LINUX_ONLY),
            _job("browser", APP_UI_CHECKS, LINUX_ONLY),
            _job("runner", (), LINUX_ONLY),
            _job("vm", (), LINUX_ONLY),
        ),
    ),
    "browser-ext": Profile(
        "browser-ext",
        "browser extension/native host source: native-host tests plus browser/UI checks",
        (
            _job("control", SELFTEST_CHECKS, LINUX_ONLY),
            _job("frontend", ("js-engine-test",), LINUX_ONLY),
            _job("native", ("native-host-rust-test", "cargo-workspace-test"), LINUX_ONLY),
            _job("heavy", (), LINUX_ONLY),
            _job("browser", APP_UI_CHECKS + SITE_UI_CHECKS, LINUX_ONLY),
            _job("runner", (), LINUX_ONLY),
            _job("vm", (), LINUX_ONLY),
        ),
    ),
    "core": Profile(
        "core",
        "native/core/shared-protocol/runner or broad repository change: full synthetic matrix, heavy work and Linux VM",
        (
            _job("control", SELFTEST_CHECKS, LINUX_ONLY),
            _job("frontend", FRONTEND_CHECKS, BOTH),
            _job("native", NATIVE_CHECKS, BOTH),
            _job("heavy", HEAVY_CHECKS, BOTH),
            _job("browser", APP_UI_CHECKS + SITE_UI_CHECKS, BOTH),
            _job("runner", RUNNER_CHECKS, BOTH),
            _job("vm", VM_CHECKS, LINUX_ONLY),
        ),
    ),
}

# Area -> profile. `ci` and `unknown` deliberately map to `full` (fail closed).
PROFILE_BY_AREA: Dict[str, str] = {
    AREA_CI: "full",
    AREA_UNKNOWN: "full",
    AREA_DOCS: "docs",
    AREA_SITE: "site",
    AREA_APP_UI: "app-ui",
    AREA_EXTENSIONS: "browser-ext",
    AREA_CORE: "core",
}

# Profiles are combined by union when a change spans multiple areas. A profile
# dominates a lesser one when it already covers the others; ordering is only
# used for the human-readable reason and for `full` domination.
_PROFILE_ORDER: Tuple[str, ...] = ("full", "core", "browser-ext", "app-ui", "site", "docs")

# Every job id a profile may contain (the workflow mirrors these).
JOB_IDS: Tuple[str, ...] = ("control", "frontend", "native", "heavy", "browser", "runner", "vm")


def profile_for_areas(areas: Iterable[str]) -> Profile:
    """Combine the profiles selected by a set of touched areas.

    Union of per-job check lists and per-job platforms. `full` and any `ci`/
    `unknown` area short-circuit to the whole `full` profile.
    """
    area_set = frozenset(areas)
    if not area_set:
        return PROFILES["full"]
    selected = {PROFILE_BY_AREA.get(area, "full") for area in area_set}
    if "full" in selected:
        return PROFILES["full"]
    ordered = [name for name in _PROFILE_ORDER if name in selected]
    if len(ordered) == 1:
        return PROFILES[ordered[0]]
    # `core` is a superset of the light profiles (both platforms, full
    # synthetic matrix, heavy work and the VM), so it dominates them.
    if "core" in selected:
        return PROFILES["core"]
    # Union: merge every job's checks (dedup, stable) and platforms.
    combined: List[Job] = []
    for job_id in JOB_IDS:
        checks: List[str] = []
        platforms: List[str] = []
        for name in ordered:
            job = PROFILES[name].job(job_id)
            for check in job.checks:
                if check not in checks:
                    checks.append(check)
            for platform in job.platforms:
                if platform not in platforms:
                    platforms.append(platform)
        combined.append(_job(job_id, checks, platforms))
    return Profile(
        "+".join(ordered),
        "combined areas: " + ", ".join(sorted(area_set)),
        tuple(combined),
    )


# ---------------------------------------------------------------------------
# Events and git
# ---------------------------------------------------------------------------

def plan_for_event(event: str, changed_paths: Sequence[str], *,
                   full_reason: str = "") -> Tuple[Profile, str]:
    """Return (profile, reason) for an event and its changed paths.

    `full_reason` (when set) forces the `full` profile with that reason instead
    of consulting paths. Callers set it for a missing/zero base, a scheduled or
    manually dispatched run, and any other conservative fallback.
    """
    if full_reason:
        return PROFILES["full"], full_reason
    if event in ("schedule", "workflow_dispatch"):
        return PROFILES["full"], f"{event} runs the complete cross-platform acceptance"
    if not changed_paths:
        return PROFILES["full"], "no changed paths resolved; fail closed to full"
    areas = classify_paths(changed_paths)
    profile = profile_for_areas(areas)
    return profile, f"areas={','.join(sorted(areas))} -> profile={profile.name}"


_FULL_SHA_RE = re.compile(r"^[0-9a-f]{40}$")
_ZERO_SHA = "0" * 40

def _valid_sha(value: Optional[str]) -> bool:
    return bool(value) and bool(_FULL_SHA_RE.match(value)) and value != _ZERO_SHA

def git_merge_base(base: Optional[str], head: Optional[str], *,
                   runner=subprocess.run, cwd: Path = ROOT) -> Optional[str]:
    """Return `git merge-base base head`, or None when it cannot be resolved.

    A PR diff must branch off the merge base (the fork point), not the raw base
    ref, so a PR is not blamed for unrelated changes that landed on the base
    branch after the fork. None makes the caller fail closed to full.
    """
    if not _valid_sha(base) or not _valid_sha(head):
        return None
    try:
        result = runner(["git", "merge-base", base, head],
                        cwd=str(cwd), capture_output=True, timeout=60)
    except (OSError, subprocess.SubprocessError):
        return None
    if getattr(result, "returncode", 1) != 0:
        return None
    stdout = result.stdout
    if isinstance(stdout, bytes):
        stdout = stdout.decode("utf-8", "replace")
    merge_base = stdout.strip()
    return merge_base if _valid_sha(merge_base) else None

def git_changed_paths(base: Optional[str], head: Optional[str], *,
                      runner=subprocess.run, cwd: Path = ROOT) -> Optional[List[str]]:
    """Repository-relative changed paths between `base` and `head`.

    Returns None (never an empty list) when the base is missing, all-zero or not
    resolvable, so the caller can fail closed. Includes both sides of a rename
    and deleted paths.
    """
    if not _valid_sha(base) or not _valid_sha(head):
        return None
    try:
        result = runner(
            ["git", "diff", "--name-status", "--find-renames", "-z", f"{base}..{head}"],
            cwd=str(cwd), capture_output=True, timeout=60,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if getattr(result, "returncode", 1) != 0:
        return None
    # `-z` separates records by NUL; a rename record is b"R<score>\0old\0new\0".
    raw = result.stdout
    if isinstance(raw, str):  # tolerate a text-mode runner (e.g. a test double)
        raw = raw.encode("utf-8", "surrogateescape")
    fields = [part.decode("utf-8", "surrogateescape")
              for part in raw.split(b"\0") if part != b""]
    paths: List[str] = []
    index = 0
    while index < len(fields):
        status = fields[index]
        index += 1
        if status and status[0] in ("R", "C"):
            if index + 1 >= len(fields):
                break
            paths.extend([fields[index], fields[index + 1]])
            index += 2
        else:
            if index >= len(fields):
                break
            paths.append(fields[index])
            index += 1
    return paths


@dataclasses.dataclass(frozen=True)
class Plan:
    event: str
    head: Optional[str]
    diff_base: Optional[str]
    profile: str
    reason: str
    changed_paths: Tuple[str, ...]
    areas: Tuple[str, ...]
    jobs: Tuple[Job, ...]
    change_head: Optional[str] = None

    def job(self, job_id: str) -> Job:
        for job in self.jobs:
            if job.id == job_id:
                return job
        return Job(job_id, (), ())

    @property
    def platforms(self) -> Tuple[str, ...]:
        seen: List[str] = []
        for job in self.jobs:
            for platform in job.platforms:
                if platform not in seen:
                    seen.append(platform)
        order = {MACOS: 0, LINUX: 1}
        return tuple(sorted(seen, key=lambda name: order.get(name, 9)))

    def to_github_outputs(self) -> Dict[str, str]:
        # Only fixed scalar/list outputs are exported. `changed_paths` is
        # deliberately absent: a filename could contain a newline and corrupt
        # GITHUB_OUTPUT, and the full plan travels as the JSON artifact instead.
        outputs = {
            "event": self.event,
            "profile": self.profile,
            "reason": self.reason,
            "platforms": json.dumps(list(self.platforms)),
            "areas": json.dumps(list(self.areas)),
        }
        for job in self.jobs:
            key = job.id.replace("-", "_")
            outputs[key + "_checks"] = ",".join(job.checks)
            outputs[key + "_enabled"] = "true" if job.runs() else "false"
        return outputs

    def to_dict(self) -> Dict[str, object]:
        return {
            "event": self.event,
            "head": self.head,
            "change_head": self.change_head,
            "diff_base": self.diff_base,
            "profile": self.profile,
            "reason": self.reason,
            "platforms": list(self.platforms),
            "areas": list(self.areas),
            "changed_paths": list(self.changed_paths),
            "jobs": [{"id": job.id, "checks": list(job.checks),
                      "platforms": list(job.platforms)} for job in self.jobs],
        }

def _plan_from_dict(document: Dict[str, object]) -> Plan:
    """Rebuild a Plan from its `to_dict()` JSON (used by --aggregate)."""
    jobs = tuple(
        Job(str(job["id"]), tuple(str(c) for c in job["checks"]),
            tuple(str(pl) for pl in job["platforms"]))
        for job in document["jobs"]  # type: ignore[union-attr]
    )
    return Plan(
        event=str(document.get("event", "")),
        head=(str(document["head"]) if document.get("head") else None),
        diff_base=(str(document["diff_base"]) if document.get("diff_base") else None),
        profile=str(document.get("profile", "")),
        reason=str(document.get("reason", "")),
        changed_paths=tuple(str(p) for p in document.get("changed_paths", [])),
        areas=tuple(str(a) for a in document.get("areas", [])),
        jobs=jobs,
        change_head=(str(document["change_head"]) if document.get("change_head") else None),
    )

def build_plan(event: str, *, base: Optional[str], head: Optional[str],
               changed_paths: Optional[Sequence[str]] = None,
               full_reason: str = "",
               merge_base_runner=None, cwd: Path = ROOT,
               checkout_head: Optional[str] = None) -> Plan:
    """Assemble a Plan, resolving base/head through git when paths are not given.

    Push diffs use the pushed `before`..`head`. Pull requests diff from the
    merge base (fork point) of `base`/`head` to `head`, so a PR is not blamed
    for commits that landed on the base branch after the fork. Any
    unresolvable base/head still fails closed to `full`.
    """
    def merge(b: Optional[str], h: Optional[str]) -> Optional[str]:
        if merge_base_runner is not None:
            return merge_base_runner(b, h)
        return git_merge_base(b, h, cwd=cwd)

    resolved: Optional[List[str]] = list(changed_paths) if changed_paths is not None else None
    reason = full_reason
    diff_base: Optional[str] = None
    if resolved is None and not full_reason:
        if event == "pull_request":
            diff_base = merge(base, head)
            if diff_base is None:
                reason = "pull request merge base unavailable; fail closed to full"
                resolved = []
            else:
                resolved = git_changed_paths(diff_base, head, cwd=cwd)
                if resolved is None:
                    reason = "pull request diff unavailable; fail closed to full"
                    resolved = []
        else:
            diff_base = base if _valid_sha(base) else None
            resolved = git_changed_paths(base, head, cwd=cwd)
            if resolved is None:
                reason = "base or head unavailable; fail closed to full"
                resolved = []
    if resolved is None:
        resolved = []
    profile, profile_reason = plan_for_event(event, resolved, full_reason=reason)
    final_reason = reason or profile_reason
    areas = tuple(sorted(classify_paths(resolved))) if resolved else ()
    if checkout_head is not None and not _valid_sha(checkout_head):
        raise ValueError("checkout head must be a full nonzero SHA")
    return Plan(event, checkout_head or head, diff_base, profile.name, final_reason,
                tuple(resolved), areas, profile.jobs, head)


# ---------------------------------------------------------------------------
# Self-checks: prove the plan covers the entrypoint registry
# ---------------------------------------------------------------------------

def _load_verify_module():
    spec = importlib.util.spec_from_file_location(
        "lintel_verify_registry", ROOT / "tests/verify.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules["lintel_verify_registry"] = module
    spec.loader.exec_module(module)
    return module


def self_check() -> List[str]:
    """Return a list of problems (empty when the plan is complete)."""
    problems: List[str] = []
    verify = _load_verify_module()
    registry = {check.id: check for check in verify.CHECKS}

    # Every CI job check id (except the plan-owned sentinels) must exist in the
    # verify.py registry.
    for profile in PROFILES.values():
        for job in profile.jobs:
            for check in job.checks:
                if check in SELFTEST_CHECKS:
                    continue
                if check not in registry:
                    problems.append(f"{profile.name}/{job.id}: unknown check id {check!r}")

    # No profile may select an independent check that is excluded from CI.
    for profile in PROFILES.values():
        for job in profile.jobs:
            for check in job.checks:
                if check in EXCLUDED_FROM_CI:
                    problems.append(
                        f"{profile.name}/{job.id}: selects CI-excluded check {check!r}")

    # Every non-independent, non-`desktop-typecheck` default check must appear in
    # the `full` profile so a new check cannot be silently dropped from CI.
    full_checks = {c for job in PROFILES["full"].jobs for c in job.checks}
    default_checks = {check.id for check in verify.select(
        argparse.Namespace(checks=None, category=None, all=False))}
    missing_default = sorted(default_checks - full_checks)
    if missing_default:
        problems.append("full profile omits default checks: " + ", ".join(missing_default))

    # Every independent check must be either in the full profile or explicitly
    # excluded with a reason.
    independent = {check.id for check in verify.CHECKS if check.independent}
    unaccounted = sorted(independent - full_checks - set(EXCLUDED_FROM_CI))
    if unaccounted:
        problems.append("independent checks neither run nor excluded: " + ", ".join(unaccounted))

    # Areas must classify consistently and every area must map to a profile.
    for area in AREAS:
        if area not in PROFILE_BY_AREA:
            problems.append(f"area {area!r} has no profile mapping")
        elif PROFILE_BY_AREA[area] not in PROFILES:
            problems.append(f"area {area!r} maps to unknown profile {PROFILE_BY_AREA[area]!r}")

    # Job sets must be identical across profiles (the workflow mirrors the ids).
    for profile in PROFILES.values():
        ids = tuple(job.id for job in profile.jobs)
        if ids != JOB_IDS:
            problems.append(f"profile {profile.name!r} jobs {ids} != {JOB_IDS}")

    # Every platform key a profile schedules must map to a known runner label.
    for profile in PROFILES.values():
        for job in profile.jobs:
            for platform in job.platforms:
                try:
                    platform_for_runner(platform)
                except ValueError:
                    problems.append(
                        f"{profile.name}/{job.id}: unknown platform key {platform!r}")

    # `full` must include the Linux VM and both platforms.
    full = PROFILES["full"]
    if not full.job("vm").checks:
        problems.append("full profile must run the Linux VM")
    if set(full.platforms_of()) != set(BOTH):
        problems.append("full profile must run both macOS and Linux")

    # The canonical-evidence model must cover every check a profile schedules on
    # every platform it schedules it for, so no selected check can escape the
    # aggregate gate. `control` is exempt (it runs the self-tests directly).
    for profile in PROFILES.values():
        for job in profile.jobs:
            if job.id == "control":
                continue
            for platform in job.platforms:
                covered = {c for _, checks in canonical_evidence(job.id, platform, job.checks)
                           for c in checks}
                expected = set(checks_for_platform(job.checks, platform))
                if not job.checks:
                    continue
                missing = sorted(expected - covered)
                if missing:
                    problems.append(
                        f"{profile.name}/{job.id}/{platform}: canonical evidence does not cover "
                        + ", ".join(missing))
                if not canonical_evidence(job.id, platform, job.checks):
                    problems.append(
                        f"{profile.name}/{job.id}/{platform}: no canonical evidence model")

    return problems


# ---------------------------------------------------------------------------
# Canonical CI evidence and the production aggregate gate
# ---------------------------------------------------------------------------
# The workflow uploads one evidence artifact per (job, platform). This section
# defines, for each scheduled job/platform, the exact *canonical* `verify.py`
# evidence file and the checks it must prove. Helper sidecar reports (per-UI
# reports, browser-smoke.json, the VM runtime sidecar) are deliberately NOT read
# here: only the canonical verifier record can prove a check passed.

PLATFORM_SYSTEM: Dict[str, str] = {MACOS: "Darwin", LINUX: "Linux"}

# Checks that only run on one platform. Everything else runs on every platform a
# job is scheduled for.
PLATFORM_ONLY_CHECKS: Dict[str, Tuple[str, ...]] = {
    "linux-ssh-runtime": (LINUX,),
    "linux-vm-runtime": (LINUX,),
}

# Jobs whose evidence is one canonical file named `<job>-<platform>.json`.
_MATRIX_EVIDENCE_JOBS: Tuple[str, ...] = ("frontend", "native", "heavy", "browser")

# Jobs that must be `success` (the always-scheduled plan/control gate). Every
# other enabled job must also be `success`; a disabled job must be `skipped`.
ALWAYS_ON_JOBS: Tuple[str, ...] = ("plan", "control")


def platform_for_runner(runner_name: str) -> str:
    """Map an Actions `runs-on` label to our canonical platform key."""
    if runner_name == MACOS:
        return MACOS
    if runner_name == LINUX:
        return LINUX
    raise ValueError(f"unknown runner label {runner_name!r}")


def checks_for_platform(check_ids: Sequence[str], platform: str) -> Tuple[str, ...]:
    """Return the subset of `check_ids` that actually runs on `platform`."""
    result: List[str] = []
    for check_id in check_ids:
        allowed = PLATFORM_ONLY_CHECKS.get(check_id)
        if allowed is not None and platform not in allowed:
            continue
        result.append(check_id)
    return tuple(result)


def canonical_evidence(job_id: str, platform: str,
                       checks: Sequence[str]) -> List[Tuple[str, Tuple[str, ...]]]:
    """Return [(filename, checks)] the aggregate requires for one job/platform.

    Filenames are repository-relative to the merged evidence download directory
    (`evidence/<artifact>/<file>`), so the gate reads only the canonical file
    produced for that job/platform.
    """
    if job_id in _MATRIX_EVIDENCE_JOBS:
        surface = checks_for_platform(checks, platform)
        if not surface:
            return []
        return [(f"{job_id}-{platform}.json", surface)]
    if job_id == "runner":
        surface = checks_for_platform(checks, platform)
        entries: List[Tuple[str, Tuple[str, ...]]] = []
        if "cli-candidate" in surface:
            entries.append((f"cli-candidate-{platform}.json", ("cli-candidate",)))
        if "linux-ssh-runtime" in surface:
            entries.append(("linux-ssh-runtime.json", ("linux-ssh-runtime",)))
        return entries
    if job_id == "vm":
        surface = checks_for_platform(checks, platform)
        if not surface:
            return []
        return [("linux-vm-entry.json", surface)]
    return []


def fetch_evidence(docs: Dict[str, object], filename: str) -> Optional[Dict[str, object]]:
    """Find an evidence document by its canonical basename across artifacts.

    The download keeps one directory per artifact, so the same basename can
    appear at most once per job/platform; a duplicate basename is ambiguous and
    the caller treats a missing/duplicate lookup as a problem.
    """
    matches = [doc for name, doc in docs.items() if Path(name).name == filename]
    if len(matches) != 1:
        return None
    return matches[0]  # type: ignore[return-value]


def _evidence_problems(document: Dict[str, object], filename: str,
                       job_id: str, platform: str, checks: Sequence[str],
                       plan_head: Optional[str]) -> List[str]:
    problems: List[str] = []
    if str(document.get("schema")) != "lintel.verify/v1":
        problems.append(f"{job_id}/{platform}: {filename} is not a lintel.verify/v1 document")
        return problems
    recorded_head = str(document.get("git", {}).get("head") or "")
    if plan_head and recorded_head != plan_head:
        problems.append(
            f"{job_id}/{platform}: {filename} was produced for git {recorded_head!r}, "
            f"not this run's {plan_head!r}")
    platform_info = document.get("platform", {})
    expected_system = PLATFORM_SYSTEM.get(platform, platform)
    recorded_system = str(platform_info.get("system"))
    if recorded_system != expected_system:
        problems.append(
            f"{job_id}/{platform}: {filename} platform.system is {recorded_system!r}, "
            f"expected {expected_system!r}")
    results = {str(r.get("id")): str(r.get("status")) for r in document.get("results", [])}
    for check_id in checks:
        if check_id not in results:
            problems.append(f"{job_id}/{platform}: {filename} has no record for {check_id}")
        elif results[check_id] != "passed":
            problems.append(
                f"{job_id}/{platform}: {check_id} status is {results[check_id]!r}, not 'passed'")
    return problems


def evaluate_jobs(plan: Plan, results: Dict[str, object]) -> List[str]:
    """Return problems from the job-result table (`needs` JSON).

    `plan`/`control` must be `success`; every other enabled job must be
    `success`; every disabled job must be exactly `skipped`. Anything else
    (failure/cancelled/missing/unexpected-skip) is a problem, so a required job
    cannot be masked by a skipped sibling.
    """
    problems: List[str] = []
    for job_id in ("plan", *JOB_IDS):
        entry = results.get(job_id) if isinstance(results, dict) else None
        result = entry.get("result") if isinstance(entry, dict) else None
        if job_id in ALWAYS_ON_JOBS:
            if result != "success":
                problems.append(f"required job {job_id!r} result is {result!r}, not 'success'")
            continue
        job = plan.job(job_id)
        if job.runs():
            if result != "success":
                problems.append(
                    f"selected job {job_id!r} result is {result!r}, not 'success'")
        else:
            if result != "skipped":
                problems.append(
                    f"unscheduled job {job_id!r} result is {result!r}, expected 'skipped'")
    return problems


def evaluate_evidence(plan: Plan, docs: Dict[str, object]) -> List[str]:
    """Return problems proving every selected check/platform produced evidence."""
    problems: List[str] = []
    for job in plan.jobs:
        if job.id == "control":
            continue  # runs the planner/verifier self-tests, not verify.py checks
        for platform in job.platforms:
            for filename, checks in canonical_evidence(job.id, platform, job.checks):
                document = fetch_evidence(docs, filename)
                if document is None:
                    problems.append(
                        f"{job.id}/{platform}: missing canonical evidence {filename}")
                    continue
                problems.extend(_evidence_problems(
                    document, filename, job.id, platform, checks, plan.head))
    return problems


def aggregate(plan: Plan, results: Dict[str, object],
              docs: Dict[str, object]) -> List[str]:
    """The production aggregate gate: job results plus canonical evidence.

    A scheduled job that failed/cancelled, a required job (plan/control) that
    did not succeed, a selected check that skipped, missing/wrong-platform/stale
    evidence, and a bad or unexpected JSON document all fail. A *planned* skip
    (a job the plan did not schedule) passes.
    """
    return evaluate_jobs(plan, results) + evaluate_evidence(plan, docs)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def _collect_evidence(root: Path) -> Dict[str, object]:
    """Read every evidence JSON under `root`, keyed by its path relative to root."""
    docs: Dict[str, object] = {}
    if not root.is_dir():
        return docs
    for path in sorted(root.rglob("*.json")):
        try:
            docs[str(path.relative_to(root))] = json.loads(path.read_text())
        except (OSError, ValueError) as error:
            docs[str(path.relative_to(root))] = {"_unreadable": str(error)}
    return docs


def _main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="Lintel finite CI planner")
    parser.add_argument("--event", default="push",
                        help="github event name (push, pull_request, schedule, workflow_dispatch)")
    parser.add_argument("--base", default=None, help="base SHA (push before / PR base)")
    parser.add_argument("--head", default=None, help="changed branch head SHA")
    parser.add_argument("--checkout-head", default=None,
                        help="tested checkout SHA (PR merge commit); defaults to --head")
    parser.add_argument("--changed-file", action="append", default=None,
                        dest="changed_files", help="explicit changed path (repeatable); bypasses git")
    parser.add_argument("--full-reason", default="",
                        help="force the full profile with this reason")
    parser.add_argument("--json", default=None, help="write the plan as JSON to this path")
    parser.add_argument("--github-output", default=None,
                        help="append job check lists as GitHub step outputs to this file")
    parser.add_argument("--summary-file", default=None,
                        help="append a human-readable summary to this file")
    parser.add_argument("--self-check", action="store_true",
                        help="verify the plan covers the entrypoint registry and exit")
    parser.add_argument("--aggregate", action="store_true",
                        help="production gate: evaluate job results plus canonical evidence")
    parser.add_argument("--plan", default=None, help="plan JSON path for --aggregate")
    parser.add_argument("--results", default=None, help="needs/results JSON path for --aggregate")
    parser.add_argument("--evidence-dir", default=None, help="merged evidence root for --aggregate")
    args = parser.parse_args(argv)

    if args.aggregate:
        if not args.plan or not args.results:
            print("--aggregate requires --plan and --results", file=sys.stderr)
            return 2
        try:
            plan = _plan_from_dict(json.loads(Path(args.plan).read_text()))
        except (OSError, ValueError, KeyError, TypeError) as error:
            print(f"--aggregate: unreadable plan ({error})", file=sys.stderr)
            return 1
        try:
            results = json.loads(Path(args.results).read_text())
        except (OSError, ValueError) as error:
            print(f"--aggregate: unreadable results ({error})", file=sys.stderr)
            return 1
        docs = _collect_evidence(Path(args.evidence_dir)) if args.evidence_dir else {}
        problems = aggregate(plan, results, docs)
        for problem in problems:
            print("FAIL - " + problem, file=sys.stderr)
        if problems:
            print(f"aggregate: {len(problems)} problem(s)", file=sys.stderr)
            return 1
        print("aggregate: every selected job and check passed; planned skips allowed")
        return 0

    if args.self_check:
        problems = self_check()
        for problem in problems:
            print("FAIL - " + problem, file=sys.stderr)
        if problems:
            print(f"ci_plan self-check: {len(problems)} problem(s)", file=sys.stderr)
            return 1
        print("ci_plan self-check: plan covers the verify.py registry")
        return 0

    plan = build_plan(args.event, base=args.base, head=args.head,
                      changed_paths=args.changed_files, full_reason=args.full_reason,
                      checkout_head=args.checkout_head)

    summary = (
        f"CI plan: event={plan.event} profile={plan.profile} "
        f"platforms={','.join(plan.platforms)}\n"
        f"reason: {plan.reason}\n"
    )
    for job in plan.jobs:
        summary += f"  {job.id}: {', '.join(job.checks) if job.checks else '(none)'}\n"
    print(summary, end="")

    if args.json:
        Path(args.json).write_text(json.dumps(plan.to_dict(), indent=2) + "\n")
    if args.github_output:
        with open(args.github_output, "a", encoding="utf-8") as handle:
            for key, value in plan.to_github_outputs().items():
                handle.write(f"{key}={value}\n")
    if args.summary_file:
        with open(args.summary_file, "a", encoding="utf-8") as handle:
            handle.write(summary + "\n")
    return 0

if __name__ == "__main__":
    sys.exit(_main())

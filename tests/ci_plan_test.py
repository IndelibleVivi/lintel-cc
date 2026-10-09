#!/usr/bin/env python3
"""Tests for the CI planner (tests/ci_plan.py).

These are selection/gate tests: they never run a real product check, touch the
network, or read credentials. They exercise the *production* planner and gate
code (not a re-implementation) across the CI contract boundaries: push vs. PR merge-base, deleted/renamed paths, unknown/CI fail-closed,
owner-directory Markdown, profile coverage and unknown checks, pure-UI without
heavy/VM, core with heavy/VM, manual/weekly full, missing-base fallback, the
production aggregate gate (job results + canonical per-job/platform evidence),
and the strict selected-skip rule.

`tests/verify.py --self-test` runs this module (and `ci_plan.py --self-check`)
as a subprocess so the canonical entrypoint covers CI selection; the workflow
requires the YAML workflow tests in its `control` job.
"""
from __future__ import annotations

import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]


def _load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


ci_plan = _load("lintel_ci_plan", ROOT / "tests/ci_plan.py")


class FakeCompleted:
    def __init__(self, returncode: int = 0, stdout: bytes = b""):
        self.returncode = returncode
        self.stdout = stdout
        self.stderr = b""


class ClassificationTests(unittest.TestCase):
    def test_docs_paths(self):
        for path in ("docs/verification.md", "README.md", "README.en.md", "AGENTS.md",
                     "LICENSE", "notes.md", "CHANGELOG.md"):
            self.assertEqual(ci_plan.classify_path(path), ci_plan.AREA_DOCS, path)

    def test_site_paths(self):
        for path in ("apps/site/index.html", "apps/site/site-world.mjs",
                     "assets/preview/hero.svg", "scripts/prepare-site.mjs",
                     "scripts/prepare-preview-art.mjs"):
            self.assertEqual(ci_plan.classify_path(path), ci_plan.AREA_SITE, path)

    def test_app_ui_vs_core_native(self):
        self.assertEqual(ci_plan.classify_path("apps/desktop/src/App.tsx"), ci_plan.AREA_APP_UI)
        self.assertEqual(ci_plan.classify_path("apps/desktop/package-lock.json"), ci_plan.AREA_APP_UI)
        # The native Tauri crate is core/desktop-rust evidence, not pure UI.
        self.assertEqual(ci_plan.classify_path("apps/desktop/src-tauri/src/main.rs"), ci_plan.AREA_CORE)

    def test_owner_directory_markdown_is_not_docs(self):
        # A Markdown file inside a product directory belongs to that directory's
        # area, not to `docs` (fnmatch's `*` also matches `/`).
        self.assertEqual(ci_plan.classify_path("apps/desktop/src-tauri/resources/guide.md"),
                         ci_plan.AREA_CORE)
        self.assertEqual(ci_plan.classify_path("extensions/browser/README.md"),
                         ci_plan.AREA_EXTENSIONS)
        self.assertEqual(ci_plan.classify_path("apps/site/docs/notes.md"), ci_plan.AREA_SITE)
        self.assertEqual(ci_plan.classify_path("crates/core/src/notes.md"), ci_plan.AREA_CORE)
        self.assertEqual(ci_plan.classify_path("tests/fixtures/readme.md"), ci_plan.AREA_CORE)
        self.assertEqual(ci_plan.classify_path("scripts/README.md"), ci_plan.AREA_CORE)
        # The real docs tree and root-only Markdown stay docs.
        self.assertEqual(ci_plan.classify_path("docs/verification.md"), ci_plan.AREA_DOCS)
        self.assertEqual(ci_plan.classify_path("GUIDE.md"), ci_plan.AREA_DOCS)

    def test_core_and_extension_paths(self):
        for path in ("crates/core/src/lib.rs", "apps/runner/src/main.rs",
                     "contracts/task-catalog.json", "platform/ssh/client.py",
                     "scripts/prepare-remote-runners.mjs", "Cargo.lock", "Cargo.toml"):
            self.assertEqual(ci_plan.classify_path(path), ci_plan.AREA_CORE, path)
        self.assertEqual(ci_plan.classify_path("extensions/browser/engine.js"), ci_plan.AREA_EXTENSIONS)

    def test_ci_controller_paths(self):
        for path in (".github/workflows/verify.yml", ".github/ISSUE_TEMPLATE/x.md",
                     "tests/verify.py", "tests/ci_plan.py", "tests/ci_plan_test.py"):
            self.assertEqual(ci_plan.classify_path(path), ci_plan.AREA_CI, path)

    def test_unknown_path(self):
        self.assertEqual(ci_plan.classify_path("some/new/thing.bin"), ci_plan.AREA_UNKNOWN)
        self.assertEqual(ci_plan.classify_path(""), ci_plan.AREA_UNKNOWN)


class ProfileTests(unittest.TestCase):
    def test_docs_profile_has_only_control(self):
        profile, reason = ci_plan.plan_for_event("push", ["docs/x.md"])
        self.assertEqual(profile.name, "docs")
        self.assertEqual(profile.job("control").checks, ci_plan.SELFTEST_CHECKS)
        for job_id in ("frontend", "native", "heavy", "browser", "runner", "vm"):
            self.assertFalse(profile.job(job_id).runs(), job_id)

    def test_site_profile(self):
        profile, _ = ci_plan.plan_for_event("push", ["apps/site/site-world.mjs"])
        self.assertEqual(profile.name, "site")
        self.assertIn("site-ui", profile.job("browser").checks)
        self.assertIn("site-game-test", profile.job("frontend").checks)
        self.assertFalse(profile.job("heavy").runs())
        self.assertFalse(profile.job("vm").runs())

    def test_app_ui_profile_no_heavy_no_vm(self):
        profile, _ = ci_plan.plan_for_event("push", ["apps/desktop/src/App.tsx"])
        self.assertEqual(profile.name, "app-ui")
        self.assertIn("desktop-build", profile.job("frontend").checks)
        self.assertIn("desktop-typecheck", profile.job("frontend").checks)
        for check in ("browser-smoke", "browser-pairing-ui", "work-ui", "clawd-app-ui"):
            self.assertIn(check, profile.job("browser").checks)
        self.assertFalse(profile.job("heavy").runs())
        self.assertFalse(profile.job("vm").runs())
        self.assertFalse(profile.job("native").runs())

    def test_core_profile_has_heavy_and_vm(self):
        profile, _ = ci_plan.plan_for_event("push", ["crates/core/src/lib.rs"])
        self.assertEqual(profile.name, "core")
        self.assertTrue(profile.job("heavy").runs())
        self.assertTrue(profile.job("vm").runs())
        self.assertIn("journey-large-work", profile.job("heavy").checks)
        self.assertIn("linux-vm-runtime", profile.job("vm").checks)
        self.assertEqual(set(profile.platforms_of()), set(ci_plan.BOTH))

    def test_extension_profile(self):
        profile, _ = ci_plan.plan_for_event("push", ["extensions/browser/native-host/src/lib.rs"])
        self.assertEqual(profile.name, "browser-ext")
        self.assertIn("native-host-rust-test", profile.job("native").checks)
        self.assertIn("browser-smoke", profile.job("browser").checks)

    def test_ci_and_unknown_fail_closed(self):
        for path in (".github/workflows/verify.yml", "tests/ci_plan.py",
                     "some/new/thing.bin"):
            profile, reason = ci_plan.plan_for_event("push", [path])
            self.assertEqual(profile.name, "full", path)
            self.assertTrue(profile.job("heavy").runs(), path)
            self.assertTrue(profile.job("vm").runs(), path)

    def test_mixed_areas_union(self):
        profile, _ = ci_plan.plan_for_event("push", ["docs/x.md", "apps/site/index.html"])
        self.assertEqual(set(profile.name.split("+")), {"site", "docs"})
        self.assertIn("site-game-test", profile.job("frontend").checks)
        self.assertFalse(profile.job("heavy").runs())

    def test_mixed_with_core_dominates(self):
        profile, _ = ci_plan.plan_for_event("push", ["docs/x.md", "crates/core/src/lib.rs"])
        self.assertEqual(profile.name, "core")
        self.assertTrue(profile.job("heavy").runs())

    def test_mixed_with_ci_forces_full(self):
        profile, _ = ci_plan.plan_for_event("push", ["docs/x.md", "tests/verify.py"])
        self.assertEqual(profile.name, "full")


class EventTests(unittest.TestCase):
    def test_schedule_and_dispatch_are_full(self):
        for event in ("schedule", "workflow_dispatch"):
            profile, reason = ci_plan.plan_for_event(event, [])
            self.assertEqual(profile.name, "full")
            self.assertIn(event, reason)

    def test_empty_paths_fail_closed(self):
        profile, reason = ci_plan.plan_for_event("push", [])
        self.assertEqual(profile.name, "full")
        self.assertIn("fail closed", reason)


class GitResolutionTests(unittest.TestCase):
    def test_missing_base_returns_none(self):
        for base, head in ((None, "a" * 40), ("", "a" * 40), ("0" * 40, "a" * 40),
                           ("short", "a" * 40)):
            self.assertIsNone(ci_plan.git_changed_paths(base, head), (base, head))

    def test_rename_and_delete_include_every_side(self):
        payload = b"R100\0old/path.py\0new/path.py\0D\0gone/file.md\0M\0kept/file.rs\0"
        captured = {}

        def fake_runner(argv, **kwargs):
            captured["argv"] = argv
            return FakeCompleted(0, payload)

        paths = ci_plan.git_changed_paths("a" * 40, "b" * 40, runner=fake_runner)
        self.assertEqual(paths, ["old/path.py", "new/path.py", "gone/file.md", "kept/file.rs"])
        self.assertIn("--name-status", captured["argv"])
        self.assertIn("a" * 40 + ".." + "b" * 40, captured["argv"])

    def test_git_failure_returns_none(self):
        self.assertIsNone(ci_plan.git_changed_paths(
            "a" * 40, "b" * 40, runner=lambda *a, **k: FakeCompleted(128, b"")))

    def test_merge_base_requires_valid_shas(self):
        for base, head in ((None, "a" * 40), ("0" * 40, "a" * 40), ("x", "y")):
            self.assertIsNone(ci_plan.git_merge_base(base, head), (base, head))

    def test_merge_base_parses_output(self):
        mb = "c" * 40
        got = ci_plan.git_merge_base("a" * 40, "b" * 40,
                                     runner=lambda *a, **k: FakeCompleted(0, (mb + "\n").encode()))
        self.assertEqual(got, mb)

    def test_merge_base_failure_returns_none(self):
        self.assertIsNone(ci_plan.git_merge_base("a" * 40, "b" * 40,
                                                 runner=lambda *a, **k: FakeCompleted(1, b"")))

    def test_build_plan_missing_base_falls_back_to_full(self):
        plan = ci_plan.build_plan("push", base=None, head=None)
        self.assertEqual(plan.profile, "full")
        self.assertIn("unavailable", plan.reason)

    def test_build_plan_push_uses_paths(self):
        plan = ci_plan.build_plan("push", base=None, head=None,
                                  changed_paths=["docs/a.md"])
        self.assertEqual(plan.profile, "docs")

    def test_pr_uses_merge_base(self):
        with mock.patch.object(ci_plan, "git_changed_paths",
                               return_value=["apps/desktop/src/App.tsx"]) as diff, \
             mock.patch.object(ci_plan, "git_merge_base", return_value="c" * 40) as mb:
            plan = ci_plan.build_plan("pull_request", base="a" * 40, head="b" * 40)
        mb.assert_called_once()
        # The diff base must be the merge base, not the raw PR base.
        diff.assert_called_once_with("c" * 40, "b" * 40, cwd=ci_plan.ROOT)
        self.assertEqual(plan.profile, "app-ui")
        self.assertEqual(plan.diff_base, "c" * 40)
        self.assertEqual(plan.head, "b" * 40)

    def test_pr_without_merge_base_fails_closed(self):
        with mock.patch.object(ci_plan, "git_merge_base", return_value=None):
            plan = ci_plan.build_plan("pull_request", base="a" * 40, head="b" * 40)
        self.assertEqual(plan.profile, "full")
        self.assertIn("merge base", plan.reason)


class RealGitRepoTests(unittest.TestCase):
    """Real temporary git repositories (no mocked git output)."""

    def setUp(self):
        self.repo = Path(tempfile.mkdtemp(prefix="lintel-ci-git-"))
        self._git("init", "-q", "-b", "main")
        self._git("config", "user.email", "synthetic@example.invalid")
        self._git("config", "user.name", "Synthetic")
        (self.repo / "docs").mkdir()
        (self.repo / "docs" / "guide.md").write_text("base\n")
        (self.repo / "crates").mkdir()
        (self.repo / "crates" / "lib.rs").write_text("fn main() {}\n")
        self._git("add", "-A")
        self._git("commit", "-q", "-m", "base")
        self.base = self._git("rev-parse", "HEAD").strip()

    def tearDown(self):
        shutil.rmtree(self.repo, ignore_errors=True)

    def _git(self, *args):
        result = subprocess.run(["git", *args], cwd=self.repo, capture_output=True, text=True)
        if result.returncode != 0:
            raise AssertionError(f"git {' '.join(args)} failed: {result.stderr}")
        return result.stdout

    def _commit(self, message):
        self._git("add", "-A")
        self._git("commit", "-q", "-m", message)
        return self._git("rev-parse", "HEAD").strip()

    def _paths(self, base, head):
        return ci_plan.git_changed_paths(base, head, cwd=self.repo)

    def test_pr_merge_base_excludes_base_only_commits(self):
        # Feature branch off base, touching a real App UI path.
        self._git("checkout", "-q", "-b", "feature")
        (self.repo / "apps" / "desktop" / "src").mkdir(parents=True)
        (self.repo / "apps" / "desktop" / "src" / "App.tsx").write_text("ui\n")
        feature_head = self._commit("feature ui")
        # Base branch advances with an unrelated native change.
        self._git("checkout", "-q", "main")
        (self.repo / "crates" / "lib.rs").write_text("fn main() { let _ = 1; }\n")
        base_tip = self._commit("unrelated native change on main")
        # PR base = current main tip; head = feature branch.
        merge_base = ci_plan.git_merge_base(base_tip, feature_head, cwd=self.repo)
        self.assertEqual(merge_base, self.base)
        paths = self._paths(merge_base, feature_head)
        self.assertIn("apps/desktop/src/App.tsx", paths)
        self.assertNotIn("crates/lib.rs", paths)  # base-only change must not leak
        # build_plan resolves the merge base itself; pass no explicit paths.
        plan = ci_plan.build_plan("pull_request", base=base_tip, head=feature_head,
                                  cwd=self.repo)
        self.assertEqual(plan.profile, "app-ui")
        self.assertEqual(plan.diff_base, self.base)
        # The naive `base_tip..feature_head` two-dot diff would additionally show
        # the base-only crates change; the merge base must exclude it.
        naive = set(self._paths(base_tip, feature_head) or [])
        self.assertIn("crates/lib.rs", naive)

    def test_rename_reports_both_sides(self):
        self._git("mv", "docs/guide.md", "docs/renamed.md")
        head = self._commit("rename doc")
        paths = self._paths(self.base, head)
        self.assertIn("docs/guide.md", paths)
        self.assertIn("docs/renamed.md", paths)

    def test_delete_is_reported(self):
        self._git("rm", "docs/guide.md")
        head = self._commit("delete doc")
        paths = self._paths(self.base, head)
        self.assertIn("docs/guide.md", paths)
        # A deleted path still classifies (and here stays docs). Deleting a core
        # file must still trip the core profile.
        self._git("rm", "crates/lib.rs")
        head2 = self._commit("delete core lib")
        paths2 = self._paths(head, head2)
        self.assertIn("crates/lib.rs", paths2)
        plan = ci_plan.build_plan("push", base=head, head=head2, changed_paths=paths2)
        self.assertEqual(plan.profile, "core")


class GithubOutputTests(unittest.TestCase):
    def test_outputs_expose_job_checks_and_enabled_flags(self):
        plan = ci_plan.build_plan("push", base=None, head=None,
                                  changed_paths=["apps/site/index.html"])
        outputs = plan.to_github_outputs()
        self.assertEqual(outputs["profile"], "site")
        self.assertEqual(outputs["browser_checks"], "site-ui")
        self.assertEqual(outputs["browser_enabled"], "true")
        self.assertEqual(outputs["heavy_enabled"], "false")
        self.assertEqual(outputs["heavy_checks"], "")
        self.assertEqual(outputs["platforms"], '["ubuntu-24.04"]')
        # `changed_paths` must not be exported (a newline in a filename would
        # corrupt GITHUB_OUTPUT); the plan JSON artifact carries it instead.
        self.assertNotIn("changed_paths", outputs)
        self.assertNotIn("changed_paths", plan.to_dict()["jobs"][0])

    def test_full_platforms_are_both(self):
        plan = ci_plan.build_plan("schedule", base=None, head=None)
        self.assertEqual(plan.platforms, (ci_plan.MACOS, ci_plan.LINUX))


class CanonicalEvidenceModelTests(unittest.TestCase):
    def test_platform_scoped_checks(self):
        self.assertEqual(ci_plan.checks_for_platform(ci_plan.RUNNER_CHECKS, ci_plan.LINUX),
                         ("linux-ssh-runtime", "cli-candidate"))
        self.assertEqual(ci_plan.checks_for_platform(ci_plan.RUNNER_CHECKS, ci_plan.MACOS),
                         ("cli-candidate",))
        self.assertEqual(ci_plan.checks_for_platform(ci_plan.VM_CHECKS, ci_plan.MACOS), ())

    def test_canonical_files_per_job(self):
        self.assertEqual(
            ci_plan.canonical_evidence("frontend", ci_plan.LINUX, ci_plan.FRONTEND_CHECKS),
            [(f"frontend-{ci_plan.LINUX}.json", ci_plan.FRONTEND_CHECKS)])
        runner_macos = ci_plan.canonical_evidence("runner", ci_plan.MACOS, ci_plan.RUNNER_CHECKS)
        self.assertEqual(runner_macos, [(f"cli-candidate-{ci_plan.MACOS}.json", ("cli-candidate",))])
        runner_linux = ci_plan.canonical_evidence("runner", ci_plan.LINUX, ci_plan.RUNNER_CHECKS)
        self.assertIn(("linux-ssh-runtime.json", ("linux-ssh-runtime",)), runner_linux)
        self.assertIn((f"cli-candidate-{ci_plan.LINUX}.json", ("cli-candidate",)), runner_linux)
        self.assertEqual(ci_plan.canonical_evidence("vm", ci_plan.LINUX, ci_plan.VM_CHECKS),
                         [("linux-vm-entry.json", ("linux-vm-runtime",))])

    def test_control_has_no_verify_evidence(self):
        self.assertEqual(ci_plan.canonical_evidence("control", ci_plan.LINUX, ci_plan.SELFTEST_CHECKS), [])


class ProductionAggregateGateTests(unittest.TestCase):
    """Exercise the production aggregate gate, not a re-implementation."""

    HEAD = "f" * 40

    def _plan(self, paths):
        return ci_plan.build_plan("push", base=None, head=self.HEAD, changed_paths=paths)

    def _jobs(self, plan):
        return {job.id: job for job in plan.jobs}

    @staticmethod
    def _doc(checks, system, head):
        return {"schema": "lintel.verify/v1", "git": {"head": head},
                "platform": {"system": system},
                "results": [{"id": c, "status": "passed"} for c in checks]}

    def _all_skips(self):
        # plan/control are always scheduled and must succeed; every other job is
        # a planned (allowed) skip.
        results = {"plan": {"result": "success"}, "control": {"result": "success"}}
        for job in ci_plan.JOB_IDS:
            if job == "control":
                continue
            results[job] = {"result": "skipped"}
        return results

    def test_pr_evidence_binds_the_tested_merge_checkout(self):
        branch_head, merge_head = "a" * 40, "b" * 40
        plan = ci_plan.build_plan(
            "pull_request", base="c" * 40, head=branch_head,
            checkout_head=merge_head, changed_paths=["apps/desktop/src/styles.css"])
        self.assertEqual(plan.head, merge_head)
        self.assertEqual(plan.change_head, branch_head)
        restored = ci_plan._plan_from_dict(plan.to_dict())
        self.assertEqual(restored.head, merge_head)
        results = {"plan": {"result": "success"}}
        docs = {}
        for job in plan.jobs:
            results[job.id] = {"result": "success" if job.runs() else "skipped"}
            if job.id == "control":
                continue
            for platform in job.platforms:
                for filename, checks in ci_plan.canonical_evidence(job.id, platform, job.checks):
                    docs[filename] = self._doc(checks, ci_plan.PLATFORM_SYSTEM[platform], merge_head)
        self.assertEqual(ci_plan.aggregate(restored, results, docs), [])
        for document in docs.values():
            document["git"]["head"] = branch_head
        self.assertTrue(ci_plan.aggregate(restored, results, docs))

    def test_docs_only_aggregate_passes_with_no_evidence(self):
        # THE regression: empty expected set AND no evidence files must pass.
        plan = self._plan(["docs/x.md"])
        problems = ci_plan.aggregate(plan, self._all_skips(), {})
        self.assertEqual(problems, [])

    def test_docs_only_via_cli_passes(self):
        # End-to-end through the CLI the workflow actually calls.
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            plan_path = tmp / "plan.json"
            plan_path.write_text(json.dumps(self._plan(["docs/x.md"]).to_dict()))
            results_path = tmp / "results.json"
            results_path.write_text(json.dumps(self._all_skips()))
            result = subprocess.run(
                [sys.executable, "tests/ci_plan.py", "--aggregate", "--plan", str(plan_path),
                 "--results", str(results_path), "--evidence-dir", str(tmp / "absent")],
                cwd=ROOT, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_plan_job_must_succeed(self):
        plan = self._plan(["docs/x.md"])
        results = self._all_skips()
        results["plan"] = {"result": "failure"}
        self.assertTrue(any("plan" in p for p in ci_plan.aggregate(plan, results, {})))

    def test_control_must_succeed(self):
        plan = self._plan(["docs/x.md"])
        results = self._all_skips()
        results["control"] = {"result": "skipped"}
        self.assertTrue(any("control" in p for p in ci_plan.aggregate(plan, results, {})))

    def test_selected_job_unexpectedly_skipped_fails(self):
        # A job the plan scheduled but which was skipped must fail the gate.
        plan = self._plan(["apps/site/index.html"])
        results = self._all_skips()
        problems = ci_plan.aggregate(plan, results, {})
        self.assertTrue(any("frontend" in p for p in problems), problems)

    def test_selected_job_missing_from_results_fails(self):
        plan = self._plan(["apps/site/index.html"])
        problems = ci_plan.aggregate(plan, {"plan": {"result": "success"}}, {})
        self.assertTrue(problems)

    def test_unscheduled_job_success_is_a_problem(self):
        # A disabled job reporting success is unexpected (its `if` was false).
        plan = self._plan(["docs/x.md"])
        results = self._all_skips()
        results["vm"] = {"result": "success"}
        self.assertTrue(any("vm" in p for p in ci_plan.aggregate(plan, results, {})))

    def test_selected_check_skip_fails(self):
        plan = self._plan(["apps/site/index.html"])
        jobs = self._jobs(plan)
        results = self._all_skips()
        results["frontend"] = {"result": "success"}
        results["browser"] = {"result": "success"}
        docs = {
            f"evidence-frontend-{ci_plan.LINUX}/frontend-{ci_plan.LINUX}.json":
                self._doc(jobs["frontend"].checks, "Linux", self.HEAD),
            f"evidence-browser-{ci_plan.LINUX}/browser-{ci_plan.LINUX}.json":
                {"schema": "lintel.verify/v1", "git": {"head": self.HEAD},
                 "platform": {"system": "Linux"},
                 "results": [{"id": "site-ui", "status": "skipped"}]},
        }
        problems = ci_plan.aggregate(plan, results, docs)
        self.assertTrue(any("site-ui" in p and "skipped" in p for p in problems), problems)

    def test_missing_canonical_evidence_fails(self):
        plan = self._plan(["apps/site/index.html"])
        results = self._all_skips()
        results["frontend"] = {"result": "success"}
        results["browser"] = {"result": "success"}
        problems = ci_plan.aggregate(plan, results, {})  # no evidence at all
        self.assertTrue(any("missing canonical evidence" in p for p in problems), problems)

    def test_stale_head_and_wrong_platform_fail(self):
        plan = self._plan(["apps/site/index.html"])
        jobs = self._jobs(plan)
        results = self._all_skips()
        results["frontend"] = {"result": "success"}
        results["browser"] = {"result": "success"}
        good = {
            f"evidence-frontend-{ci_plan.LINUX}/frontend-{ci_plan.LINUX}.json":
                self._doc(jobs["frontend"].checks, "Linux", self.HEAD),
            f"evidence-browser-{ci_plan.LINUX}/browser-{ci_plan.LINUX}.json":
                self._doc(jobs["browser"].checks, "Linux", self.HEAD),
        }
        self.assertEqual(ci_plan.aggregate(plan, results, good), [])
        stale = dict(good)
        stale[f"evidence-frontend-{ci_plan.LINUX}/frontend-{ci_plan.LINUX}.json"] = \
            self._doc(jobs["frontend"].checks, "Linux", "0" * 40)
        self.assertTrue(any("not this run" in p for p in ci_plan.aggregate(plan, results, stale)))
        wrong = dict(good)
        wrong[f"evidence-browser-{ci_plan.LINUX}/browser-{ci_plan.LINUX}.json"] = \
            self._doc(jobs["browser"].checks, "Darwin", self.HEAD)
        self.assertTrue(any("platform.system" in p for p in ci_plan.aggregate(plan, results, wrong)))

    def test_bad_schema_fails(self):
        plan = self._plan(["apps/site/index.html"])
        jobs = self._jobs(plan)
        results = self._all_skips()
        results["frontend"] = {"result": "success"}
        results["browser"] = {"result": "success"}
        docs = {
            f"evidence-frontend-{ci_plan.LINUX}/frontend-{ci_plan.LINUX}.json":
                self._doc(jobs["frontend"].checks, "Linux", self.HEAD),
            f"evidence-browser-{ci_plan.LINUX}/browser-{ci_plan.LINUX}.json":
                {"schema": "something-else", "results": []},
        }
        self.assertTrue(any("lintel.verify/v1" in p for p in ci_plan.aggregate(plan, results, docs)))

    def test_duplicate_basename_is_ambiguous(self):
        # Two artifacts providing the same canonical basename can let one
        # platform impersonate another; the gate must reject the ambiguity.
        plan = self._plan(["apps/site/index.html"])
        results = self._all_skips()
        results["frontend"] = {"result": "success"}
        results["browser"] = {"result": "success"}
        jobs = self._jobs(plan)
        docs = {
            f"evidence-frontend-{ci_plan.LINUX}/frontend-{ci_plan.LINUX}.json":
                self._doc(jobs["frontend"].checks, "Linux", self.HEAD),
            f"evidence-browser-{ci_plan.LINUX}/browser-{ci_plan.LINUX}.json":
                self._doc(jobs["browser"].checks, "Linux", self.HEAD),
            f"evidence-other/browser-{ci_plan.LINUX}.json":
                self._doc(jobs["browser"].checks, "Linux", self.HEAD),
        }
        problems = ci_plan.aggregate(plan, results, docs)
        self.assertTrue(any("missing canonical evidence" in p for p in problems), problems)

    def test_full_plan_requires_per_platform_runner_evidence(self):
        # A full run must prove Linux-only checks from Linux evidence and macOS
        # only needs cli-candidate: losing Linux evidence must fail.
        plan = self._plan([".github/workflows/verify.yml"])  # -> full
        self.assertEqual(plan.profile, "full")
        jobs = self._jobs(plan)
        results = {"plan": {"result": "success"}, "control": {"result": "success"}}
        for job in ("frontend", "native", "heavy", "browser", "runner", "vm"):
            results[job] = {"result": "success"}
        docs = {}
        for platform, system in ((ci_plan.MACOS, "Darwin"), (ci_plan.LINUX, "Linux")):
            for job_id in ("frontend", "native", "heavy", "browser"):
                docs[f"evidence-{job_id}-{platform}/{job_id}-{platform}.json"] = \
                    self._doc(jobs[job_id].checks, system, self.HEAD)
            docs[f"evidence-runner-{platform}/cli-candidate-{platform}.json"] = \
                self._doc(["cli-candidate"], system, self.HEAD)
        docs["evidence-vm/linux-vm-entry.json"] = \
            self._doc(["linux-vm-runtime"], "Linux", self.HEAD)
        # Missing linux-ssh-runtime evidence.
        problems = ci_plan.aggregate(plan, results, docs)
        self.assertTrue(any("linux-ssh-runtime" in p for p in problems), problems)
        docs["evidence-runner-linux/linux-ssh-runtime.json"] = \
            self._doc(["linux-ssh-runtime"], "Linux", self.HEAD)
        self.assertEqual(ci_plan.aggregate(plan, results, docs), [])


class CoverageTests(unittest.TestCase):
    def test_self_check_passes(self):
        self.assertEqual(ci_plan.self_check(), [])

    def test_self_check_detects_unknown_check(self):
        bad = ci_plan.dataclasses.replace(
            ci_plan.PROFILES["full"],
            jobs=tuple(
                ci_plan.dataclasses.replace(job, checks=job.checks + ("no-such-check",))
                if job.id == "frontend" else job
                for job in ci_plan.PROFILES["full"].jobs))
        with mock.patch.dict(ci_plan.PROFILES, {"full": bad}):
            problems = ci_plan.self_check()
        self.assertTrue(any("no-such-check" in p for p in problems), problems)

    def test_self_check_detects_missing_default_coverage(self):
        dropped = "journey-policy"
        bad = ci_plan.dataclasses.replace(
            ci_plan.PROFILES["full"],
            jobs=tuple(
                ci_plan.dataclasses.replace(
                    job, checks=tuple(c for c in job.checks if c != dropped))
                if job.id == "native" else job
                for job in ci_plan.PROFILES["full"].jobs))
        with mock.patch.dict(ci_plan.PROFILES, {"full": bad}):
            problems = ci_plan.self_check()
        self.assertTrue(any(dropped in p for p in problems), problems)

    def test_self_check_detects_excluded_check_selected(self):
        bad = ci_plan.dataclasses.replace(
            ci_plan.PROFILES["full"],
            jobs=tuple(
                ci_plan.dataclasses.replace(job, checks=job.checks + ("work-scale",))
                if job.id == "heavy" else job
                for job in ci_plan.PROFILES["full"].jobs))
        with mock.patch.dict(ci_plan.PROFILES, {"full": bad}):
            problems = ci_plan.self_check()
        self.assertTrue(any("work-scale" in p for p in problems), problems)

    def test_excluded_from_ci_has_reasons(self):
        self.assertTrue(ci_plan.EXCLUDED_FROM_CI)
        for check, reason in ci_plan.EXCLUDED_FROM_CI.items():
            self.assertTrue(reason.strip(), check)


class WorkflowContractTests(unittest.TestCase):
    """Validate the real workflow file against the minimal CI contract.

    Parsed with PyYAML when available (the repository's canonical venv provides
    it); otherwise this class is skipped rather than silently passing. It never
    runs any workflow step.
    """

    def setUp(self):
        try:
            import yaml  # noqa: F401
        except ImportError:
            self.skipTest("PyYAML not available; workflow contract test skipped")
        path = ROOT / ".github/workflows/verify.yml"
        if not path.is_file():
            self.skipTest("verify.yml not present")
        import yaml
        self.raw = path.read_text()
        self.doc = yaml.safe_load(self.raw)

    def _triggers(self):
        return self.doc.get(True) or self.doc.get("on")

    def test_triggers_and_permissions(self):
        triggers = self._triggers()
        for name in ("push", "pull_request", "schedule", "workflow_dispatch"):
            self.assertIn(name, triggers)
        self.assertEqual(self.doc["permissions"], {"contents": "read"})

    def test_concurrency_is_run_scoped_for_acceptance(self):
        concurrency = self.doc["concurrency"]
        self.assertIn("github.run_id", concurrency["group"])
        self.assertIn("cancel-in-progress", concurrency)
        # Never cancel scheduled/manual acceptance runs.
        self.assertIn("github.event_name == 'push'", concurrency["cancel-in-progress"])

    def test_evidence_dir_is_outside_the_repo(self):
        self.assertNotIn("EVIDENCE_DIR", self.doc.get("env", {}))
        for jobid in ("frontend", "native", "heavy", "browser", "runner", "vm"):
            init = next(step for step in self.doc["jobs"][jobid]["steps"]
                        if step.get("name") == "Initialize evidence directory")
            self.assertIn('EVIDENCE_DIR=$RUNNER_TEMP/lintel-verify', init["run"])
            self.assertIn('"$GITHUB_ENV"', init["run"])
        self.assertNotIn(".ci-evidence", self.raw)
        # A generated-evidence path must not be hidden in .gitignore.
        gitignore = (ROOT / ".gitignore")
        if gitignore.is_file():
            self.assertNotIn("ci-evidence", gitignore.read_text())

    def test_jobs_match_planner_job_ids(self):
        self.assertEqual(set(self.doc["jobs"]),
                         set(ci_plan.JOB_IDS) | {"plan", "aggregate"})
        for job_id in ci_plan.JOB_IDS:
            job = self.doc["jobs"][job_id]
            self.assertEqual(job.get("needs"), "plan", job_id)

    def test_aggregate_uses_production_gate(self):
        steps = self.doc["jobs"]["aggregate"]["steps"]
        runs = "\n".join(s.get("run", "") for s in steps)
        self.assertIn("tests/ci_plan.py --aggregate", runs)
        self.assertIn("--plan", runs)
        self.assertIn("--results", runs)
        self.assertIn("--evidence-dir", runs)
        self.assertEqual(self.doc["jobs"]["aggregate"]["if"], "always()")
        needs = self.doc["jobs"]["aggregate"]["needs"]
        self.assertIn("control", needs)

    def test_job_checks_are_planner_selections(self):
        # Each verify.py invocation in a matrix job must use the plan output CSV
        # and strict mode, and be gated on its own enabled flag.
        for job_id in ("frontend", "native", "heavy", "browser"):
            job = self.doc["jobs"][job_id]
            runs = "\n".join(s.get("run", "") for s in job.get("steps", []))
            self.assertIn(f"needs.plan.outputs.{job_id}_checks", runs)
            self.assertIn("--require-passed", runs)
            self.assertEqual(job["if"], f"needs.plan.outputs.{job_id}_enabled == 'true'")

    def test_runner_and_vm_use_explicit_checks_and_canonical_filenames(self):
        runner = self.doc["jobs"]["runner"]
        runs = "\n".join(s.get("run", "") for s in runner["steps"])
        # runner splits linux-ssh-runtime (Linux) and cli-candidate (both) so each
        # has its own canonical evidence file, matching the gate model.
        self.assertIn("--checks linux-ssh-runtime", runs)
        self.assertIn("--checks cli-candidate", runs)
        self.assertIn("--require-passed", runs)
        self.assertIn("cli-candidate-${{ matrix.os }}.json", runs)
        self.assertIn("linux-ssh-runtime.json", runs)
        self.assertEqual(runner["if"], "needs.plan.outputs.runner_enabled == 'true'")
        vm = self.doc["jobs"]["vm"]
        vm_runs = "\n".join(s.get("run", "") for s in vm["steps"])
        self.assertIn("--checks linux-vm-runtime", vm_runs)
        self.assertIn("linux-vm-entry.json", vm_runs)
        self.assertEqual(vm["if"], "needs.plan.outputs.vm_enabled == 'true'")

    def test_evidence_filenames_match_canonical_model(self):
        # The workflow's `<job>-${{ matrix.os }}.json` filename satisfies the
        # planner's canonical model for each scheduled matrix job/platform.
        checks_by_job = {
            "frontend": ci_plan.FRONTEND_CHECKS,
            "native": ci_plan.NATIVE_CHECKS,
            "heavy": ci_plan.HEAVY_CHECKS,
            "browser": ci_plan.APP_UI_CHECKS + ci_plan.SITE_UI_CHECKS,
        }
        for platform in (ci_plan.LINUX, ci_plan.MACOS):
            for job_id, checks in checks_by_job.items():
                self.assertEqual(
                    ci_plan.canonical_evidence(job_id, platform, checks),
                    [(f"{job_id}-{platform}.json", ci_plan.checks_for_platform(checks, platform))])

    def test_native_job_installs_tauri_linux_packages(self):
        job = self.doc["jobs"]["native"]
        runs = "\n".join(s.get("run", "") for s in job.get("steps", []))
        for pkg in ("libwebkit2gtk-4.1-dev", "libgtk-3-dev", "libayatana-appindicator3-dev",
                    "librsvg2-dev", "libsoup-3.0-dev", "libjavascriptcoregtk-4.1-dev"):
            self.assertIn(pkg, runs, pkg)

    def test_cargo_cache_covers_standalone_targets(self):
        for job_id in ("native", "browser"):
            job = self.doc["jobs"][job_id]
            cache = next(s for s in job["steps"] if s.get("name") == "Cargo cache")
            self.assertIn("extensions/browser/native-host/target", cache["with"]["path"])
        native_cache = next(s for s in self.doc["jobs"]["native"]["steps"]
                            if s.get("name") == "Cargo cache")
        self.assertIn("apps/desktop/src-tauri/target", native_cache["with"]["path"])

    def test_macos_cli_build_uses_package_name(self):
        job = self.doc["jobs"]["runner"]
        build = next(s for s in job["steps"] if s.get("name") == "Build native macOS CLI binary")
        self.assertIn("-p lintel-runner", build["run"])
        self.assertNotIn("-p lintel ", build["run"] + " ")

    def test_vm_is_independent_and_not_gated_on_ui(self):
        vm = self.doc["jobs"]["vm"]
        self.assertEqual(vm.get("needs"), "plan")
        self.assertNotIn("browser", str(vm.get("needs")))
        # VM only runs on a Linux host.
        self.assertEqual(vm["runs-on"], "ubuntu-24.04")

    def test_no_third_party_paths_filter(self):
        self.assertNotIn("paths-filter", self.raw)
        self.assertNotIn("dorny", self.raw)


class CIIntegrationTests(unittest.TestCase):
    def test_shared_playroom_changes_cover_both_consumers(self):
        for path in ("apps/site/clawd-game.mjs", "apps/site/clawd-game.css", "tests/site_game.test.mjs"):
            with self.subTest(path=path):
                plan = ci_plan.build_plan("push", base=None, head="f" * 40, changed_paths=[path])
                checks = plan.job("browser").checks
                self.assertIn("site-ui", checks)
                self.assertIn("clawd-app-ui", checks)
                self.assertFalse(plan.job("heavy").runs())
                self.assertFalse(plan.job("vm").runs())

    def test_app_ui_test_changes_keep_the_ui_profile(self):
        for path in ("tests/network_ui_journey.mjs", "tests/telemetry_ui_journey.mjs", "tests/clawd_app_ui_journey.mjs"):
            with self.subTest(path=path):
                plan = ci_plan.build_plan("push", base=None, head="f" * 40, changed_paths=[path])
                self.assertEqual(plan.profile, "app-ui")
                self.assertTrue(plan.job("browser").runs())
                self.assertFalse(plan.job("heavy").runs())
                self.assertFalse(plan.job("vm").runs())

    def test_changed_filename_whitespace_is_not_erased(self):
        self.assertEqual(ci_plan.classify_path(" apps/desktop/src/styles.css"), ci_plan.AREA_UNKNOWN)

    def test_strict_workflow_tests_fail_when_yaml_is_unavailable(self):
        result = subprocess.run(
            [sys.executable, "-S", "tests/ci_plan_test.py", "--require-workflow"],
            cwd=ROOT, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("refusing a skipped gate", result.stderr)

    def test_workflow_controller_requires_its_yaml_tests(self):
        try:
            import yaml
        except ImportError:
            self.skipTest("workflow contract tests require tests/requirements-ci.txt")
        workflow = yaml.safe_load((ROOT / ".github/workflows/verify.yml").read_text())
        control_runs = "\n".join(step.get("run", "") for step in workflow["jobs"]["control"]["steps"])
        self.assertIn("-r tests/requirements-ci.txt", control_runs)
        self.assertIn("verify.py --self-test --require-passed", control_runs)
        self.assertNotIn("EVIDENCE_DIR", workflow.get("env", {}))
        for job in workflow["jobs"].values():
            for value in job.get("env", {}).values():
                self.assertNotIn("runner.temp", str(value))
        plan_step = next(step for step in workflow["jobs"]["plan"]["steps"] if step.get("id") == "compute")
        self.assertEqual(plan_step["env"]["CHECKOUT_HEAD"], "${{ github.sha }}")
        self.assertIn('--checkout-head "$CHECKOUT_HEAD"', plan_step["run"])


if __name__ == "__main__":
    if "--require-workflow" in sys.argv:
        sys.argv.remove("--require-workflow")
        try:
            import yaml
        except ImportError:
            sys.exit("CI workflow tests require tests/requirements-ci.txt; refusing a skipped gate")
    unittest.main()

#!/usr/bin/env python3
"""Host control-flow regressions for the after-ACK barrier modes, the fixture
prepare allowlist, the finite /proc classification and the session-scoped logind
evidence. These never boot QEMU and never produce Linux runtime evidence: the
real PAM/logout observation is the separate Ubuntu CI route. Temporary
directories are removed in tearDown."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[3]
VM = ROOT / "tests/linux_vm_journey.py"
PROBE = ROOT / "tests/fixtures/linux_vm/runtime_probe.py"

def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

journey = load("linux_vm_journey", VM)
probe = load("runtime_probe", PROBE)

def observation(state, alive, classification, plan_id="p1",
                cgroup="0::/user.slice/user-1001.slice/session-27.scope\n"):
    return {"marker": {"pid": 4242, "plan_id": plan_id, "cgroup": cgroup},
            "present": state is not None, "state": state, "identity": {},
            "exe": "/opt/lintel-vm/lintel", "runner_identity_matches": alive,
            "current_cgroup": cgroup, "alive": alive, "zombie": classification == "zombie",
            "classification": classification, "current_boot_id": "boot"}

def probe_responder(seen, kill=False):
    """Deterministic guest.probe responses keyed by the probe operation, matching
    the argument names logout_case actually uses."""
    responses = {
        "configure-logind": {"kill_user_processes": "b true" if kill else "b false"},
        "read-json": {"id": "27", "present": True, "properties": {"Remote": "yes", "Scope": "session-27.scope"}},
        "session": {"present": True, "properties": {"State": "inactive"}},
        "process": seen,
        "session-evidence": {"scope_unit": "session-27.scope"},
        "continue": {"continued": True},
        "release": {"released": True},
    }
    def respond(operation, *args, **kwargs):
        return responses[operation]
    return respond

class TempBase(unittest.TestCase):
    def setUp(self):
        self.base = Path(tempfile.mkdtemp(prefix="lintel-probe-"))
        self.proc = self.base / "proc"
        self.boot = self.base / "boot_id"
        self.boot.write_text("boot-1\n")
        probe.BASE = self.base

    def tearDown(self):
        shutil.rmtree(self.base, ignore_errors=True)


class BarrierProcessTests(unittest.TestCase):
    def guest(self, observations):
        guest = Mock()
        guest.shell.side_effect = [
            Mock(returncode=0, stdout=json.dumps(value).encode()) for value in observations
        ]
        return guest

    def case(self):
        return {"barrier": "/x/b.json", "plan": {"id": "p1"}}

    def test_running_mode_requires_live_worker(self):
        seen = journey.barrier_process(self.guest([observation("S", True, "running")]),
                                       self.case(), "running", timeout=1)
        self.assertEqual(seen["classification"], "running")
        self.assertTrue(seen["alive"])

    def test_stopped_mode_returns_surviving_frozen_worker(self):
        # A frozen worker that survived logout is a valid result, not an error.
        seen = journey.barrier_process(self.guest([observation("T", True, "stopped")]),
                                       self.case(), "stopped", timeout=1)
        self.assertTrue(seen["alive"])
        self.assertEqual(seen["classification"], "stopped")

    def test_terminated_worker_returns_instead_of_looping(self):
        seen = journey.barrier_process(self.guest([observation(None, False, "missing")]),
                                       self.case(), "running", timeout=1)
        self.assertFalse(seen["alive"])
        self.assertEqual(seen["classification"], "missing")

    def test_zombie_in_stopped_mode_is_observed_termination(self):
        seen = journey.barrier_process(self.guest([observation("Z", False, "zombie")]),
                                       self.case(), "stopped", timeout=1)
        self.assertFalse(seen["alive"])
        self.assertEqual(seen["classification"], "zombie")

    def test_marker_for_another_job_is_rejected(self):
        with self.assertRaisesRegex(RuntimeError, "another job"):
            journey.barrier_process(self.guest([observation("S", True, "running", plan_id="other")]),
                                    self.case(), "running", timeout=1)


class ResumeSurvivorTests(unittest.TestCase):
    def case(self):
        return {"barrier": "/x/b.json", "release": "/x/r.json", "plan": {"id": "p1"}}

    def test_stopped_survivor_is_continued(self):
        guest = Mock()
        journey.resume_survivor(guest, self.case(), "stopped")
        guest.probe.assert_called_once_with("continue", "/x/b.json")

    def test_running_survivor_is_released(self):
        guest = Mock()
        journey.resume_survivor(guest, self.case(), "running")
        guest.probe.assert_called_once_with("release", "/x/r.json")


class PrepareCaseTests(TempBase):
    """Exercise the REAL prepare() allowlist and its filesystem effect, not the
    transport forwarding above it."""
    def case(self, name, uid=1001, username="lintel-fixture"):
        return probe.prepare(name, uid=uid, username=username,
                             home=Path("/home/lintel-fixture"), base=self.base / name)

    def test_all_four_logout_cases_are_allowed(self):
        for name in ("logout-retain-running", "logout-retain-stopped",
                     "logout-kill-running", "logout-kill-stopped"):
            prepared = self.case(name)
            root = Path(prepared["root"])
            self.assertTrue(root.is_dir(), name)
            self.assertEqual(root.joinpath("settings.json").read_text(),
                             json.dumps({"env": {"SYNTHETIC_VM_NEIGHBOR": "keep"}}))
            self.assertEqual(prepared["barrier"], str(self.base / name / "barrier.json"))
            self.assertEqual(prepared["release"], str(self.base / name / "release.json"))

    def test_reboot_case_is_allowed(self):
        prepared = self.case("reboot-interrupt")
        self.assertTrue(Path(prepared["root"]).is_dir())

    def test_unknown_case_is_rejected(self):
        with self.assertRaisesRegex(RuntimeError, "synthetic target user"):
            self.case("logout-maybe")

    def test_wrong_user_is_rejected(self):
        with self.assertRaisesRegex(RuntimeError, "synthetic target user"):
            self.case("logout-kill-running", uid=0, username="root")

    def test_repeated_prepare_rejects_existing_case_dir(self):
        self.case("logout-kill-running")
        with self.assertRaises(FileExistsError):
            self.case("logout-kill-running")


class SessionEvidenceTests(TempBase):
    """Assert the REAL probe commands and filtering, not forwarding."""
    def test_evidence_is_scoped_to_the_original_session(self):
        responses = {
            ("journalctl", "-b", "-n", "40", "--no-pager", "-u", "session-27.scope"): "scope ended\n",
            ("journalctl", "-b", "-n", "40", "--no-pager", "_COMM=systemd-logind", "--grep", r"(?i)\bsession\s+27\b"): "Removed session 27\n",
            ("systemctl", "show", "session-27.scope", "--no-pager", "-p", "ActiveState", "-p", "SubState", "-p", "LoadState"): "ActiveState=inactive\n",
        }
        calls = []

        def fake_command(args, check=True):
            calls.append(tuple(args))
            self.assertIn(tuple(args), responses, f"unexpected probe command: {args}")
            return Mock(returncode=0, stdout=responses[tuple(args)])

        with patch.object(probe, "command", fake_command):
            evidence = probe.session_evidence("27")
        self.assertEqual(set(calls), set(responses))
        for args in calls:
            self.assertNotIn("--user", args)
            self.assertNotIn("lintel*", args)
        self.assertEqual(evidence["session_id"], "27")
        self.assertEqual(evidence["scope_unit"], "session-27.scope")
        self.assertEqual(evidence["scope_properties"]["ActiveState"], "inactive")
        self.assertIn("Removed session 27", evidence["logind_session_lines"][0])


class ProbeClassificationTests(TempBase):
    def layout(self, pid, state="S", starttime=99, exe=None, cmdline=b""):
        proc = self.proc / str(pid)
        proc.mkdir(parents=True, exist_ok=True)
        fields = ["0"] * 22
        fields[0] = state
        fields[1], fields[2], fields[3], fields[19] = "1", "2", "3", str(starttime)
        (proc / "stat").write_text(f"{pid} (lintel) " + " ".join(fields) + "\n")
        (proc / "cgroup").write_text("0::/user.slice/user-1001.slice/session-27.scope\n")
        if exe:
            (proc / "exe").symlink_to(exe)
        (proc / "cmdline").write_bytes(cmdline)
        return proc

    def marker(self, pid):
        path = self.base / "barrier.json"
        path.write_text(json.dumps({"pid": pid, "plan_id": "p1", "starttime": 99, "boot_id": self.boot.read_text().strip()}))
        return path

    def test_missing_pid_is_missing(self):
        seen = probe.process(self.marker(4242), self.proc, self.boot)
        self.assertFalse(seen["present"])
        self.assertEqual(seen["classification"], "missing")

    def test_zombie_classified_from_stat_without_exe(self):
        # A zombie has no exe link; it must be reported as zombie, NOT missing.
        self.layout(4242, state="Z")
        seen = probe.process(self.marker(4242), self.proc, self.boot)
        self.assertTrue(seen["present"])
        self.assertTrue(seen["zombie"])
        self.assertFalse(seen["alive"])
        self.assertEqual(seen["classification"], "zombie")
        self.assertEqual(seen["identity"]["starttime"], 99)

    def test_matching_worker_is_running(self):
        self.layout(4242, state="S", exe=probe.RUNNER, cmdline=b"lintel\0__worker\0")
        seen = probe.process(self.marker(4242), self.proc, self.boot)
        self.assertTrue(seen["alive"])
        self.assertEqual(seen["classification"], "running")
        self.assertEqual(seen["identity"]["starttime"], 99)
        json.dumps(seen)  # Real probe must emit JSON; never raw cmdline bytes.

    def test_reused_pid_cannot_be_continued(self):
        self.layout(4242, state="T", starttime=100, exe=probe.RUNNER, cmdline=b"lintel\0__worker\0")
        seen = probe.process(self.marker(4242), self.proc, self.boot)
        self.assertFalse(seen["alive"])
        self.assertEqual(seen["classification"], "identity_mismatch")

    def test_matching_stopped_worker_is_stopped(self):
        self.layout(4242, state="T", exe=probe.RUNNER, cmdline=b"lintel\0__worker\0")
        seen = probe.process(self.marker(4242), self.proc, self.boot)
        self.assertTrue(seen["alive"])
        self.assertEqual(seen["classification"], "stopped")

    def test_wrong_exe_is_identity_mismatch(self):
        self.layout(4242, state="S", exe="/bin/sleep", cmdline=b"sleep\0")
        seen = probe.process(self.marker(4242), self.proc, self.boot)
        self.assertFalse(seen["runner_identity_matches"])
        self.assertEqual(seen["classification"], "identity_mismatch")


class LogoutSurvivorTests(unittest.TestCase):
    """Exercise logout_case's surviving-worker branch for BOTH modes with a fully
    scripted guest and no real sleeps: a stopped survivor is SIGCONTed and a
    running survivor released before the original job finishes."""
    def case(self):
        return {"home": "/x", "state": "/x", "barrier": "/x/b.json", "release": "/x/r.json",
                "session": "/x/s.json", "plan": {"id": "p1", "hash": "h1"}}

    def test_surviving_worker_finishes_original_job(self):
        for mode, action, arg in (("stopped", "continue", "/x/b.json"),
                                  ("running", "release", "/x/r.json")):
            with self.subTest(mode=mode):
                seen = observation("T" if mode == "stopped" else "S", True,
                                   "stopped" if mode == "stopped" else "running")
                guest = Mock()
                # barrier_process observes via guest.shell; the logout watch loop
                # samples the worker via guest.probe.
                guest.shell.return_value = Mock(returncode=0, stdout=json.dumps(seen).encode())
                guest.probe.side_effect = probe_responder(seen)
                guest.request.side_effect = [
                    {"status": "accepted"},                                              # execute ACK
                    {"id": "p1", "plan_id": "p1", "status": "accepted"},                 # first query
                    {"id": "p1", "plan_id": "p1", "status": "partially_completed"},      # terminal
                    {"id": "p1", "plan_id": "p1", "status": "partially_completed"},      # repeated
                ]
                clock = iter(range(0, 100000))
                with patch.object(journey, "prepare_policy", return_value=self.case()), \
                     patch.object(journey.time, "monotonic", lambda: next(clock)), \
                     patch.object(journey.time, "sleep"):
                    result = journey.logout_case(guest, False, mode)
                guest.probe.assert_any_call(action, arg)
                self.assertEqual(result["survival"], "observed_survived")
                self.assertEqual(result["mode"], mode)
                self.assertEqual(result["receipt_after_query_or_continue"]["status"], "partially_completed")

    def test_terminated_worker_reconciles_without_resume(self):
        seen = observation(None, False, "missing")
        guest = Mock()
        guest.shell.return_value = Mock(returncode=0, stdout=json.dumps(seen).encode())
        guest.probe.side_effect = probe_responder(seen, kill=True)
        guest.request.side_effect = [
            {"status": "accepted"},
            {"id": "p1", "plan_id": "p1", "status": "needs_reconciliation"},
            {"id": "p1", "plan_id": "p1", "status": "needs_reconciliation"},
        ]
        clock = iter(range(0, 100000))
        with patch.object(journey, "prepare_policy", return_value=self.case()), \
             patch.object(journey.time, "monotonic", lambda: next(clock)), \
             patch.object(journey.time, "sleep"):
            result = journey.logout_case(guest, True, "stopped")
        self.assertEqual(result["survival"], "observed_terminated")
        self.assertEqual(result["classification"], "missing")
        self.assertNotIn("continue", [a.args[0] for a in guest.probe.call_args_list])
        self.assertNotIn("release", [a.args[0] for a in guest.probe.call_args_list])


if __name__ == "__main__":
    unittest.main()

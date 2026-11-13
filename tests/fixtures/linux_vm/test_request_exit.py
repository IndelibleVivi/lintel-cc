"""VM controller exit/error handling; no SSH, guest or runtime evidence."""
import importlib.util
import itertools
import json
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).resolve().parents[2] / "linux_vm_journey.py"
spec = importlib.util.spec_from_file_location("lintel_vm_request_exit", SOURCE)
journey = importlib.util.module_from_spec(spec)
spec.loader.exec_module(journey)


class RequestExitTests(unittest.TestCase):
    def request(self, response, exit_code=1, expected_error="shared_auth_scope"):
        guest = journey.Guest(Path("/synthetic-unused"), 0, {})
        case = {"home": "/synthetic-home", "state": "/synthetic-state", "session": "/synthetic-session.json"}
        def shell(command, **options):
            if options.get("check", True) and exit_code:
                raise RuntimeError("ssh failed (1): expected rejection")
            return subprocess.CompletedProcess(command, exit_code, json.dumps(response).encode(), b"")
        with patch.object(guest, "shell", side_effect=shell):
            return guest.request(case, {"command": "execute"}, submit=True, expected_error=expected_error)

    def test_expected_submit_rejection_is_read_after_nonzero_exit(self):
        response = {"ok": False, "error": {"code": "shared_auth_scope"}}
        self.assertEqual(self.request(response), response)

    def test_expected_rejection_requires_nonzero_exit(self):
        with self.assertRaisesRegex(RuntimeError, "nonzero"):
            self.request({"ok": False, "error": {"code": "shared_auth_scope"}}, exit_code=0)

    def test_different_error_cannot_pass_expected_rejection(self):
        with self.assertRaisesRegex(RuntimeError, "Expected shared_auth_scope"):
            self.request({"ok": False, "error": {"code": "different_error"}})

    def test_ordinary_ssh_failure_still_raises(self):
        with self.assertRaisesRegex(RuntimeError, "ssh failed"):
            self.request({"ok": False, "error": {"code": "shared_auth_scope"}}, expected_error=None)


class ManagedAuthSubmissionTests(unittest.TestCase):
    PLAN_ID = "344e11f2-f95b-40d0-bc69-4034aa1fcd13"

    def setUp(self):
        self.case = {"home": "/tmp/synthetic-auth", "state": "/tmp/synthetic-auth/state",
                     "session": "/tmp/synthetic-auth/session.json", "as_root": True,
                     "plan": {"id": self.PLAN_ID, "hash": "synthetic-approval"}}
        self.evidence = {}
        self.calls = []

    def receipt(self, status, *, plan_id=None, execution=None):
        plan_id = plan_id or self.PLAN_ID
        return {"ok": True, "data": {"id": plan_id, "plan_id": plan_id, "status": status,
                "execution": execution or {"mode": "system_manager", "manager": "system",
                    "unit": f"lintel-{plan_id}.service", "reboot_survival": False,
                    "continuation": "synthetic", "limitation": None}}}

    def uncertain(self, plan_id=None):
        return {"ok": False, "error": {"code": "submission_uncertain", "message": "query original only"},
                "plan_id": plan_id or self.PLAN_ID}

    def missing(self):
        return {"ok": False, "error": {"code": "job_not_found", "message": "not durably accepted yet"}}

    def run_sequence(self, responses, *, timeout=180):
        guest = journey.Guest(Path("/synthetic-unused"), 0, {})
        responses = iter(responses)
        clock = itertools.count()

        def shell(command, **options):
            request = json.loads(options["data"])
            self.calls.append(request)
            self.assertEqual(request["command"], "execute" if len(self.calls) == 1 else "job")
            self.assertEqual(request["plan_id"], self.PLAN_ID)
            if len(self.calls) == 1:
                self.assertEqual(request["approval"], "synthetic-approval")
                self.assertIn("ANTHROPIC_PROFILE=SYNTHETIC_PRIVATE_CONTEXT", command)
            else:
                self.assertNotIn("approval", request)
                self.assertLessEqual(options["timeout"], 60)
            response = next(responses, self.missing())
            exit_code = 0 if response.get("ok") is True else 1
            if options.get("check", True) and exit_code:
                raise RuntimeError("Unexpected unchecked error envelope")
            return subprocess.CompletedProcess(command, exit_code, json.dumps(response).encode(), b"")

        with patch.object(guest, "shell", side_effect=shell), \
             patch.object(journey.time, "monotonic", side_effect=lambda: next(clock)), \
             patch.object(journey.time, "sleep"):
            return journey.submit_managed_auth_once(guest, self.case, self.evidence,
                        extra_env={"ANTHROPIC_PROFILE": "SYNTHETIC_PRIVATE_CONTEXT"}, timeout=timeout)

    def assert_single_submission(self):
        self.assertEqual(sum(request["command"] == "execute" for request in self.calls), 1)
        record = self.evidence["submission"]
        self.assertEqual(record["submit_calls"], 1)
        self.assertEqual(record["query_calls"], len(self.calls) - 1)
        self.assertTrue(record["query_only_after_submit"])
        self.assertFalse(record["reexecuted"])
        self.assertNotIn("SYNTHETIC_PRIVATE_CONTEXT", json.dumps(self.evidence))

    def test_immediate_ack_queries_same_original_task_until_terminal(self):
        receipt = self.run_sequence([self.receipt("accepted"), self.receipt("executing"),
                                     self.receipt("partially_completed")])
        self.assertEqual(receipt["status"], "partially_completed")
        self.assertEqual(self.evidence["submission"]["initial_outcome"], "accepted")
        self.assert_single_submission()

    def test_uncertain_ack_waits_for_durable_journal_without_resubmitting(self):
        receipt = self.run_sequence([self.uncertain(), self.missing(), self.receipt("accepted"),
                                     self.receipt("verifying"), self.receipt("partially_completed")])
        self.assertEqual(receipt["id"], self.PLAN_ID)
        record = self.evidence["submission"]
        self.assertEqual(record["initial_outcome"], "submission_uncertain")
        self.assertEqual(record["initial_error"], self.uncertain())
        self.assertTrue(record["journal_observed"])
        self.assert_single_submission()

    def test_uncertainty_for_wrong_plan_never_starts_queries(self):
        with self.assertRaisesRegex(RuntimeError, "not bound to the original auth plan"):
            self.run_sequence([self.uncertain("another-plan")])
        self.assertEqual(len(self.calls), 1)
        self.assert_single_submission()

    def test_query_receipt_for_wrong_plan_fails_without_resubmitting(self):
        with self.assertRaisesRegex(RuntimeError, "another original task"):
            self.run_sequence([self.uncertain(), self.receipt("completed", plan_id="another-plan")])
        self.assertEqual(len(self.calls), 2)
        self.assert_single_submission()

    def test_other_submit_error_is_preserved_and_not_polled(self):
        with self.assertRaisesRegex(RuntimeError, "stale_auth_plan"):
            self.run_sequence([{"ok": False, "error": {"code": "stale_auth_plan", "message": "new preview required"}}])
        self.assertEqual(len(self.calls), 1)
        self.assert_single_submission()

    def test_other_query_error_is_preserved_and_not_retried(self):
        with self.assertRaisesRegex(RuntimeError, "invalid_journal"):
            self.run_sequence([self.uncertain(), {"ok": False, "error": {"code": "invalid_journal"}}])
        self.assertEqual(len(self.calls), 2)
        self.assert_single_submission()

    def test_missing_journal_after_acceptance_is_not_a_transient(self):
        with self.assertRaisesRegex(RuntimeError, "journal disappeared"):
            self.run_sequence([self.receipt("accepted"), self.missing()])
        self.assertEqual(len(self.calls), 2)
        self.assert_single_submission()

    def test_no_journal_before_deadline_remains_uncertain_and_never_reexecutes(self):
        with self.assertRaisesRegex(RuntimeError, "within 5s; query only"):
            self.run_sequence([self.uncertain()], timeout=5)
        self.assertEqual(self.evidence["submission"]["initial_outcome"], "submission_uncertain")
        self.assertNotIn("journal_observed", self.evidence["submission"])
        self.assertEqual(self.evidence["submission"]["last_query_error"], self.missing())
        self.assert_single_submission()

    def test_execution_context_must_remain_the_original_selected_context(self):
        changed = dict(self.receipt("accepted")["data"]["execution"], continuation="changed")
        with self.assertRaisesRegex(RuntimeError, "context changed"):
            self.run_sequence([self.receipt("accepted"), self.receipt("completed", execution=changed)])
        self.assert_single_submission()


if __name__ == "__main__":
    unittest.main()

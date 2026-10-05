"""VM controller exit/error handling; no SSH, guest or runtime evidence."""
import importlib.util
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


if __name__ == "__main__":
    unittest.main()

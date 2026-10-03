from concurrent.futures import ThreadPoolExecutor
from contextlib import redirect_stderr, redirect_stdout
import io
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

MODULE = Path(__file__).parents[1] / "lintel_ssh.py"
spec = importlib.util.spec_from_file_location("lintel_ssh", MODULE)
ssh = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ssh)

FAKE_SSH = '''#!/usr/bin/env python3
import json, os, pathlib, sys, time
root = pathlib.Path(os.environ["LINTEL_FAKE_ROOT"])
request = json.load(sys.stdin)
with (root / "calls.jsonl").open("a") as output:
    output.write(json.dumps({"args": sys.argv[1:], "request": request}) + "\\n")
mode = os.environ.get("LINTEL_FAKE_MODE", "normal")
if mode == "runner_error":
    print(json.dumps({"ok": False, "error": {"code": "synthetic_rejection", "message": "synthetic rejection"}}))
    sys.exit(1)
if mode == "timeout":
    sys.stderr.write("SYNTHETIC_PRIVATE_STDERR"); sys.stderr.flush(); time.sleep(10)
if mode == "oversized":
    sys.stdout.write("x" * (3 * 1024 * 1024)); sys.stdout.flush(); sys.exit(0)
if request["command"] == "execute":
    if mode == "lose_ack_echo_passphrase":
        sys.stderr.write(request.get("archive_passphrase", "")); sys.stderr.flush()
    receipt = {"id": request["plan_id"], "plan_id": request["plan_id"], "status": "completed"}
    (root / "remote-job.json").write_text(json.dumps(receipt))
    if mode in {"lose_ack", "lose_ack_echo_passphrase"}:
        sys.exit(255)
    response = {"ok": True, "data": receipt}
elif request["command"] == "job":
    path = root / "remote-job.json"
    response = {"ok": True, "data": json.loads(path.read_text())} if path.exists() else {"ok": False, "error": {"code": "not_found", "message": "absent"}}
else:
    response = {"ok": True, "data": {"synthetic": True}}
print(json.dumps(response))
'''


class TransportTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="lintel-ssh-synthetic-")
        self.root = Path(self.temporary.name)
        self.fake = self.root / "fake-ssh"
        self.fake.write_text(FAKE_SSH)
        self.fake.chmod(0o700)
        self.environment = patch.dict(os.environ, {"LINTEL_FAKE_ROOT": str(self.root), "LINTEL_FAKE_MODE": "normal"})
        self.environment.start()
        self.controller = ssh.Controller(self.root / "state", ssh.Transport(str(self.fake), 2))

    def tearDown(self):
        self.environment.stop()
        self.temporary.cleanup()

    def calls(self):
        return [json.loads(line) for line in (self.root / "calls.jsonl").read_text().splitlines()]

    def test_alias_scan_is_static_and_does_not_follow_include_or_match(self):
        config = self.root / "config"
        sentinel = self.root / "must-not-exist"
        included = self.root / "included"
        included.write_text("Host hidden-alias\n")
        config.write_text(f'Host alpha beta *.example !excluded\nMatch exec "touch {sentinel}"\nInclude {included}\nHost=gamma\n')
        aliases = ssh.list_aliases(config)
        self.assertEqual(aliases["aliases"], ["alpha", "beta", "gamma"])
        self.assertIn("Include", aliases["ignored"])
        self.assertIn("Match", aliases["ignored"])
        self.assertFalse(sentinel.exists())
        self.assertFalse((self.root / "calls.jsonl").exists())

    def test_strict_host_check_fixed_command_and_stdin_data_no_interpolation(self):
        sentinel = self.root / "must-not-exist"
        malicious = f"$(touch {sentinel}); ' `echo bad`"
        result = self.controller.request("synthetic-host", {"command": "inspect", "environment_id": malicious})
        self.assertTrue(result["ok"])
        call = self.calls()[0]
        self.assertEqual(call["args"][-3:], ["synthetic-host", "lintel", "request"])
        self.assertIn("-oStrictHostKeyChecking=yes", call["args"])
        self.assertIn("-oUpdateHostKeys=no", call["args"])
        self.assertIn("-oPermitLocalCommand=no", call["args"])
        self.assertNotIn(malicious, call["args"])
        self.assertEqual(call["request"]["environment_id"], malicious)
        self.assertFalse(sentinel.exists())
        for alias in ["-oProxyCommand=bad", "host;touch bad", "user@host", "../../escape"]:
            with self.assertRaises(ssh.ControllerError):
                self.controller.request(alias, {"command": "jobs"})
        self.assertEqual(len(self.calls()), 1)

    def test_custom_policy_choices_are_finite_and_kept_on_stdin(self):
        request = {"command": "plan_policy", "environment_id": "synthetic-env", "preset": "custom",
                   "keep_remote_control": True, "trusted_devices": "not_required",
                   "custom_settings": {"DISABLE_TELEMETRY": "keep", "DISABLE_GROWTHBOOK": "remove",
                                       "DISABLE_ERROR_REPORTING": "disable"}}
        for choices in [None, [], {"API_KEY": "remove"}, {"DISABLE_TELEMETRY": "0"},
                        {"DISABLE_TELEMETRY": {"action": "disable"}}]:
            with self.assertRaises(ssh.ControllerError):
                self.controller.request("synthetic-host", dict(request, custom_settings=choices))
        for change in [{"preset": "reduce"}, {"release_settings": ["DISABLE_GROWTHBOOK"]}]:
            with self.assertRaises(ssh.ControllerError):
                self.controller.request("synthetic-host", request | change)
        self.assertFalse((self.root / "calls.jsonl").exists())
        self.assertTrue(self.controller.request("synthetic-host", request)["ok"])
        self.assertEqual(self.calls()[0]["request"], request)
        self.assertNotIn("custom_settings", " ".join(self.calls()[0]["args"]))

    def test_lost_ack_reconnect_and_repeated_execute_query_one_original_job(self):
        with patch.dict(os.environ, {"LINTEL_FAKE_MODE": "lose_ack"}):
            with self.assertRaises(ssh.ControllerError) as error:
                self.controller.execute("synthetic-host", "plan-one", "SYNTHETIC_APPROVAL_SECRET")
        self.assertEqual(error.exception.code, "transport_unknown")
        # Recreate controller to prove on-disk intent survives the GUI/process restart.
        controller = ssh.Controller(self.root / "state", ssh.Transport(str(self.fake), 2))
        self.assertTrue(controller.reconnect("synthetic-host", "plan-one")["ok"])
        self.assertTrue(controller.execute("synthetic-host", "plan-one", "SYNTHETIC_APPROVAL_SECRET")["ok"])
        self.assertEqual([c["request"]["command"] for c in self.calls()], ["execute", "job", "job"])
        self.assertEqual([c["args"][-1] for c in self.calls()], ["submit", "request", "request"])
        self.assertEqual([c["request"].get("job_id") for c in self.calls()][1:], ["plan-one", "plan-one"])
        state = (self.root / "state/synthetic-host/plan-one.json").read_text()
        self.assertNotIn("SYNTHETIC_APPROVAL_SECRET", state)
        self.assertEqual(json.loads(state)["status"], "completed")

    def test_archive_passphrase_stdin_only_and_lost_ack_queries_without_secret(self):
        secret = "SYNTHETIC_ARCHIVE_PASSPHRASE_ONLY"
        plan_id = "00000000-0000-4000-8000-000000000002"
        arguments = [str(MODULE), "--state-dir", str(self.root / "state"), "execute", "synthetic-host",
                     "--plan-id", plan_id, "--approval", "synthetic-hash", "--ask-archive-passphrase"]
        output, errors = io.StringIO(), io.StringIO()
        with patch.object(ssh.sys, "argv", arguments), patch.object(ssh, "Controller", return_value=self.controller), \
                patch.object(ssh.getpass, "getpass", return_value=secret) as prompt, \
                patch.dict(os.environ, {"LINTEL_FAKE_MODE": "lose_ack_echo_passphrase"}), \
                redirect_stdout(output), redirect_stderr(errors):
            self.assertEqual(ssh.main(), 1)
        prompt.assert_called_once()
        self.assertEqual(json.loads(output.getvalue())["error"]["code"], "transport_unknown")
        call = self.calls()[0]
        self.assertEqual(call["request"]["archive_passphrase"], secret)
        self.assertNotIn(secret, json.dumps(call["args"]))
        self.assertNotIn(secret, json.dumps(arguments))
        self.assertNotIn(secret, output.getvalue() + errors.getvalue())
        for path in (self.root / "state").rglob("*"):
            if path.is_file():
                self.assertNotIn(secret, path.read_text())
        controller = ssh.Controller(self.root / "state", ssh.Transport(str(self.fake), 2))
        self.assertTrue(controller.reconnect("synthetic-host", plan_id)["ok"])
        self.assertTrue(controller.execute("synthetic-host", plan_id, "synthetic-hash")["ok"])
        calls = self.calls()
        self.assertEqual([call["request"]["command"] for call in calls], ["execute", "job", "job"])
        for call in calls[1:]:
            self.assertNotIn("archive_passphrase", call["request"])
        self.assertNotIn(secret, (self.root / "state" / "synthetic-host" / (plan_id + ".json")).read_text())

    def test_short_archive_passphrase_does_not_submit_or_create_intent(self):
        with self.assertRaises(ssh.ControllerError) as error:
            self.controller.execute("synthetic-host", "plan-short", "synthetic-hash", archive_passphrase="short")
        self.assertEqual(error.exception.code, "invalid_archive_passphrase")
        self.assertFalse((self.root / "calls.jsonl").exists())
        self.assertFalse((self.root / "state/synthetic-host/plan-short.json").exists())

    def test_getpass_cannot_fall_back_to_echoing_input(self):
        arguments = [str(MODULE), "execute", "synthetic-host", "--plan-id", "plan-no-tty",
                     "--approval", "synthetic-hash", "--ask-archive-passphrase"]
        output, errors = io.StringIO(), io.StringIO()
        with patch.object(ssh.sys, "argv", arguments), patch.object(ssh, "Controller", return_value=self.controller), \
                patch.object(ssh.getpass, "getpass", side_effect=ssh.getpass.GetPassWarning("cannot disable echo")), \
                redirect_stdout(output), redirect_stderr(errors):
            self.assertEqual(ssh.main(), 1)
        self.assertEqual(json.loads(output.getvalue())["error"]["code"], "archive_passphrase_input_unavailable")
        self.assertFalse((self.root / "calls.jsonl").exists())
        self.assertEqual(errors.getvalue(), "")

    def test_concurrent_submitters_share_one_durable_intent(self):
        other = ssh.Controller(self.root / "state", ssh.Transport(str(self.fake), 2))
        with ThreadPoolExecutor(max_workers=2) as pool:
            one = pool.submit(self.controller.execute, "synthetic-host", "plan-concurrent", "synthetic-hash")
            two = pool.submit(other.execute, "synthetic-host", "plan-concurrent", "synthetic-hash")
            self.assertTrue(one.result()["ok"])
            self.assertTrue(two.result()["ok"])
        self.assertEqual([c["request"]["command"] for c in self.calls()], ["execute", "job"])

    def test_absent_job_after_local_intent_does_not_replay(self):
        path = self.root / "state/synthetic-host/plan-absent.json"
        path.parent.mkdir(parents=True)
        ssh.Controller._save(path, {"plan_id": "plan-absent", "lookup_id": "plan-absent", "status": "submission_unknown"})
        response = self.controller.execute("synthetic-host", "plan-absent", "synthetic-approval")
        self.assertEqual(response["error"]["code"], "reconciliation_required")
        self.assertEqual([c["request"]["command"] for c in self.calls()], ["job"])

    def test_timeout_and_response_limits_fail_without_logging_stderr(self):
        transport = ssh.Transport(str(self.fake), 0.1)
        for mode in ["timeout", "oversized"]:
            with patch.dict(os.environ, {"LINTEL_FAKE_MODE": mode}):
                with self.assertRaises(ssh.ControllerError) as error:
                    transport.call("synthetic-host", {"command": "jobs"})
            self.assertEqual(error.exception.code, "transport_unknown")
            self.assertNotIn("SYNTHETIC_PRIVATE_STDERR", str(error.exception))

    def test_runner_error_envelope_survives_nonzero_runner_exit(self):
        with patch.dict(os.environ, {"LINTEL_FAKE_MODE": "runner_error"}):
            response = self.controller.request("synthetic-host", {"command": "jobs"})
        self.assertEqual(response["error"]["code"], "synthetic_rejection")

    def test_request_schema_cannot_bypass_durable_execute_or_run_shell(self):
        for payload in [
            {"command": "execute", "plan_id": "p", "approval": "a"},
            {"command": "shell", "script": "id"},
            {"command": []},
            {"command": "discover", "extra": "bad"},
            {"command": "plan_policy", "environment_id": "fixture", "preset": "reduce", "keep_remote_control": "false"},
        ]:
            with self.assertRaises(ssh.ControllerError):
                self.controller.request("synthetic-host", payload)
        self.assertFalse((self.root / "calls.jsonl").exists())


if __name__ == "__main__":
    unittest.main()

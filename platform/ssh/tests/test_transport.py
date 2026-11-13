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

    def test_authentication_inventory_verify_and_selection_use_fixed_command_and_stdin(self):
        environment = "00000000-0000-4000-8000-000000000001"
        job = "00000000-0000-4000-8000-000000000002"
        source = "auth-0123456789abcdef01234567"
        sentinel = self.root / "must-not-exist"
        # A target-host path remains literal JSON data even with shell syntax.
        project = f"{self.root}/project; $(touch {sentinel})"
        requests = [
            {"command": "auth_inventory", "environment_id": environment},
            {"command": "auth_inventory", "environment_id": environment, "project_cwd": project},
            {"command": "auth_verify", "environment_id": environment},
            {"command": "auth_verify", "environment_id": environment, "job_id": job, "observe_runtime": True},
            {"command": "plan_cleanup", "environment_id": environment, "recipe": "repair_login",
             "writers_confirmed_stopped": True, "official_logout": False, "auth_source_ids": [source],
             "project_cwd": project, "verify_auth_status": False},
        ]
        for request in requests:
            self.assertTrue(self.controller.request("synthetic-host", request)["ok"])
        calls = self.calls()
        self.assertEqual([call["request"] for call in calls], requests)
        for call in calls:
            self.assertEqual(call["args"][-3:], ["synthetic-host", "lintel", "request"])
            self.assertIn("-oStrictHostKeyChecking=yes", call["args"])
            for value in (environment, job, source, project):
                self.assertNotIn(value, call["args"])
        self.assertFalse(sentinel.exists())

    def test_authentication_source_ids_are_unique_bounded_and_never_paths_or_values(self):
        environment = "00000000-0000-4000-8000-000000000001"
        source = "auth-0123456789abcdef01234567"
        request = {"command": "plan_cleanup", "environment_id": environment, "recipe": "repair_login",
                   "writers_confirmed_stopped": True, "official_logout": False, "auth_source_ids": [source]}
        for sources in ([], [f"auth-{index:024x}" for index in range(128)]):
            valid = request | {"auth_source_ids": sources}
            self.assertEqual(ssh.validate_request(valid), valid)
        for sources in ("all", None, [source, source], [f"auth-{index:024x}" for index in range(129)],
                        ["/synthetic/.zshrc"], ["SYNTHETIC_OAUTH_VALUE_MUST_NOT_BE_AN_ID"],
                        ["auth-ABCDEF0123456789abcdef01"], ["auth-" + "0" * 23], [42], [{}]):
            with self.subTest(selection=sources), self.assertRaises(ssh.ControllerError) as error:
                self.controller.request("synthetic-host", request | {"auth_source_ids": sources})
            self.assertEqual(error.exception.code, "invalid_request")
            self.assertNotIn("SYNTHETIC_OAUTH_VALUE_MUST_NOT_BE_AN_ID", str(error.exception))
        self.assertFalse((self.root / "calls.jsonl").exists())

    def test_authentication_fields_reject_wrong_types_unknown_values_and_residue_mixing(self):
        environment = "00000000-0000-4000-8000-000000000001"
        inventory = {"command": "auth_inventory", "environment_id": environment}
        verify = {"command": "auth_verify", "environment_id": environment}
        cleanup = {"command": "plan_cleanup", "environment_id": environment, "recipe": "repair_login",
                   "writers_confirmed_stopped": True, "official_logout": False, "auth_source_ids": []}
        invalid = [
            inventory | {"environment_id": "not-a-uuid"}, inventory | {"environment_id": False},
            inventory | {"project_cwd": "relative"}, inventory | {"project_cwd": "/synthetic/../other"},
            inventory | {"project_cwd": "/synthetic/new\nline"}, inventory | {"project_cwd": False},
            inventory | {"token": "SYNTHETIC_SECRET"}, inventory | {"source_path": "/synthetic/.credentials.json"},
            verify | {"job_id": "not-a-uuid"}, verify | {"observe_runtime": "true"},
            verify | {"observe_runtime": 1}, verify | {"project_cwd": "/synthetic/project"},
            cleanup | {"environment_id": "not-a-uuid"}, cleanup | {"verify_auth_status": "false"},
            cleanup | {"verify_auth_status": 0}, cleanup | {"auth_source_paths": ["/synthetic/.zshrc"]},
            cleanup | {"token": "SYNTHETIC_SECRET"}, cleanup | {"helper_command": "never-execute"},
        ]
        residue = {"command": "plan_cleanup", "environment_id": environment, "recipe": "clear_residue",
                   "writers_confirmed_stopped": True, "official_logout": False,
                   "residue_paths": ["/synthetic/debug/old.log"]}
        for fields in ({"auth_source_ids": []}, {"verify_auth_status": False}, {"project_cwd": "/synthetic/project"}):
            invalid.append(residue | fields)
        for request in invalid:
            with self.subTest(request=request), self.assertRaises(ssh.ControllerError) as error:
                self.controller.request("synthetic-host", request)
            self.assertEqual(error.exception.code, "invalid_request")
            self.assertNotIn("SYNTHETIC_SECRET", str(error.exception))
        # Existing callers without the additive authentication contract retain
        # their old syntax; the target runner still validates actual identity.
        legacy = residue | {"environment_id": "synthetic-legacy-environment"}
        self.assertEqual(ssh.validate_request(legacy), legacy)
        self.assertFalse((self.root / "calls.jsonl").exists())

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

    def test_client_control_finite_readers_and_baselines(self):
        requests = [
            {'command': 'snapshot_accept', 'environment_id': 'synthetic', 'old_snapshot_id': 'old', 'new_snapshot_id': 'new', 'selected_changes': ['exact-change']},
            {'command': 'residue_archive_inspect', 'job_id': 'original', 'archive_passphrase': 'synthetic-passphrase'},
            {'command': 'plan_residue_restore', 'environment_id': 'synthetic', 'archive_path': '/synthetic/residue.age', 'archive_passphrase': 'synthetic-passphrase', 'selected_paths': ['/synthetic/debug/old.log']},
            {'command': 'residue_inventory', 'environment_id': 'synthetic', 'offset': 1, 'expected_digest': 'a' * 64, 'limit': 2, 'extra_roots': ['/synthetic/extra'], 'since': 1, 'until': 100},
            {'command': 'drift', 'environment_id': 'synthetic', 'lightweight': True},
        ]
        for request in requests:
            self.assertEqual(ssh.validate_request(request), request)
        for bad in [requests[0] | {'selected_changes': []}, requests[0] | {'selected_changes': 'all'},
                    requests[1] | {'archive_path': '/synthetic/ambiguous.age'}, requests[2] | {'selected_paths': []},
                    requests[3] | {'expected_digest': 'invalid'}, requests[3] | {'limit': 201},
                    requests[3] | {'extra_roots': ['relative']}, requests[4] | {'lightweight': 1}]:
            with self.assertRaises(ssh.ControllerError):
                ssh.validate_request(bad)

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

    def test_service_requests_are_finite_and_stay_on_stdin(self):
        request = {"command": "plan_service_quiesce", "environment_id": "synthetic-env",
                   "manager": "user", "unit": "claude-work.service"}
        for changed in ({"manager": "sudo"}, {"unit": "*.service"},
                        {"unit": "target.service; id"}, {"unit": "../target.service"},
                        {"script": "id"}):
            with self.assertRaises(ssh.ControllerError):
                self.controller.request("synthetic-host", dict(request, **changed))
        self.assertFalse((self.root / "calls.jsonl").exists())
        self.controller.request("synthetic-host", request)
        self.controller.request("synthetic-host", {"command": "plan_service_resume", "job_id": "synthetic-job"})
        calls = self.calls()
        self.assertEqual(calls[0]["request"], request)
        self.assertEqual(calls[1]["request"]["command"], "plan_service_resume")
        self.assertNotIn("claude-work.service", " ".join(calls[0]["args"]))


if __name__ == "__main__":
    unittest.main()

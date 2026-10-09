#!/usr/bin/env python3
"""Synthetic named job-submit admission through the runner/core durable ACK.

Build cargo build -p lintel-runner first. All plans, settings, archives and the
inert client live under a disposable home; no real launch or credentials.
"""
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / "target/debug/lintel"
PASS = "synthetic-execution-admission-passphrase"


def run():
    with tempfile.TemporaryDirectory(prefix="lintel-execution-admission-") as temp:
        base = Path(temp).resolve()
        home, state = base / "home", base / "state"
        root, project = home / "synthetic-config", home / "project"
        root.mkdir(parents=True)
        project.mkdir()
        settings = root / "settings.json"
        original = b'{"env":{"KEEP_SYNTHETIC":"exact bytes"}}\n'
        settings.write_bytes(original)
        (root / "CLAUDE.md").write_text("Synthetic unsupported transcript")
        marker = base / "client-executed"
        native = home / ".local/bin/claude"
        native.parent.mkdir(parents=True)
        native.write_text("#!/bin/sh\ntouch " + shlex.quote(str(marker)) + "\n")
        native.chmod(0o700)
        env = dict(os.environ, HOME=str(home), LINTEL_TEST_HOME=str(home),
                   LINTEL_STATE_DIR=str(state), PATH="/usr/bin:/bin")

        def invoke(args, payload=None, good=True):
            process = subprocess.run([str(BINARY), *args],
                                     input=json.dumps(payload) if payload is not None else None,
                                     text=True, capture_output=True, env=env, timeout=30)
            try:
                response = json.loads(process.stdout)
            except json.JSONDecodeError as error:
                raise AssertionError((args, process.stdout, process.stderr)) from error
            assert response["ok"] is good, (args, response)
            return response["data"] if good else response["error"]

        def request(command, **fields):
            return invoke(["request"], dict(command=command, **fields))

        environment = request("register", name="Synthetic admission", root=str(root))
        assert environment["executable"] == str(native)
        archive = request("plan_archive", environment_id=environment["id"], categories=["instructions"])
        archived = request("execute", plan_id=archive["id"], approval=archive["hash"], archive_passphrase=PASS)
        launch = request("plan_launch", environment_id=environment["id"], project_cwd=str(project), mode="interactive")
        resume = request("plan_resume", environment_id=environment["id"], project_cwd=str(project),
                         job_id=archived["id"], archive_passphrase=PASS, path="CLAUDE.md")
        assert resume["resume"]["supported"] is False
        before = settings.stat()
        jobs_before = set((state / "jobs").iterdir())
        for plan in (launch, resume):
            rejected = invoke(["job", "submit", "--plan", plan["id"], "--approval", plan["hash"]],
                              {"archive_passphrase": PASS}, good=False)
            assert rejected["code"] == "invalid_plan_kind", rejected
            assert set((state / "jobs").iterdir()) == jobs_before, "rejection acquired durable ACK"
            assert not (state / "launches" / (plan["id"] + ".json")).exists()
            assert settings.read_bytes() == original
            after = settings.stat()
            assert (after.st_dev, after.st_ino, after.st_mode, after.st_mtime_ns) == (
                before.st_dev, before.st_ino, before.st_mode, before.st_mtime_ns)
            assert not marker.exists(), "wrong executor ran inert client"
            if plan["kind"] == "resume":
                assert not Path(plan["resume"]["private_copy_path"]).exists()
        allowed = request("plan_policy", environment_id=environment["id"], preset="reduce", keep_remote_control=False)
        accepted = invoke(["job", "submit", "--plan", allowed["id"], "--approval", allowed["hash"]])
        assert accepted["id"] == allowed["id"]
        completed = invoke(["job", "wait", allowed["id"], "--timeout", "20s"])
        assert completed["status"] == "completed", completed
        assert completed["steps"][0]["id"] == "settings"
        assert not marker.exists()
        print("PASS: real launch/unsupported-resume plans rejected by named submit before durable ACK; settings bytes/identity unchanged; inert client untouched; allowed policy completed")


if __name__ == "__main__":
    run()

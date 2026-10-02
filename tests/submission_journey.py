#!/usr/bin/env python3
"""Real detached runner test; all state and work files are synthetic."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

binary = Path(__file__).resolve().parents[1] / "target/debug/lintel"
with tempfile.TemporaryDirectory(prefix="lintel-submit-") as tmp:
    home = Path(tmp).resolve() / "home"
    root = home / "cc"
    root.mkdir(parents=True)
    state = Path(tmp).resolve() / "state"
    env = dict(os.environ, LINTEL_TEST_HOME=str(home), LINTEL_STATE_DIR=str(state))
    def request(payload):
        p = subprocess.run([str(binary), "request"], input=json.dumps(payload), text=True,
                           capture_output=True, env=env, timeout=25)
        v = json.loads(p.stdout)
        assert v["ok"], v
        return v["data"]
    e = request(dict(command="register", name="Synthetic detached", root=str(root)))
    (root / "CLAUDE.md").write_text("Only synthetic work is archived.")
    p = request(dict(command="plan_reset", environment_id=e["id"], recipe="rebuild", categories=["instructions"]))
    payload = dict(command="execute", plan_id=p["id"], approval=p["hash"], archive_passphrase="synthetic transient passphrase")
    submitted = subprocess.run([str(binary), "submit"], input=json.dumps(payload), text=True,
                               capture_output=True, env=env, timeout=45, start_new_session=True)
    ack = json.loads(submitted.stdout)
    assert ack["ok"] and ack["data"]["status"] == "accepted", ack
    assert (state / "jobs" / f'{p["id"]}.json').exists(), "ACK preceded durable journal"
    deadline = time.monotonic() + 45
    while True:
        job = request(dict(command="job", plan_id=p["id"]))
        if job["status"] not in ["accepted", "executing", "verifying"]:
            break
        assert time.monotonic() < deadline, job
        time.sleep(.15)
    assert job["status"] == "partially_completed", job
    assert Path(job["new_root"]).joinpath("CLAUDE.md").read_text() == "Only synthetic work is archived."
    again = subprocess.run([str(binary), "submit"], input=json.dumps(payload), text=True,
                           capture_output=True, env=env, timeout=45)
    replay = json.loads(again.stdout)
    assert replay["data"]["new_environment_id"] == job["new_environment_id"]
    for path in state.rglob("*.json"):
        assert payload["archive_passphrase"] not in path.read_text(), f"Persisted secret in {path}"
    cap = request(dict(command="discover"))["capabilities"]
    assert any(x["name"] == "detached_submission" for x in cap)
    print("PASS: detached durable ACK, parent exits before completion, query original task, replay dedup, no stored passphrase")

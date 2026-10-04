#!/usr/bin/env python3
"""Real detached runner test; all state and work files are synthetic.

Covers the durable ACK, parent exit before completion, query of the original
task, replay dedup and no stored passphrase, plus (below) the synthetic-only
RUNNING wait barrier used by the Linux VM logout acceptance: the marker is
written inside the synthetic home, the worker stays live until an explicit
release file appears, and the original approved plan only then completes."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

binary = Path(__file__).resolve().parents[1] / "target/debug/lintel"

LINUX = Path("/proc/self/stat").exists()

def marker_facts(path):
    """Read the after-ACK marker. On Linux also return the live /proc state letter
    so the held worker is proven live; the worker is not this process's child, so
    absence of a signal is never inferred from a missing pid. On macOS (no /proc)
    the state is None and only the marker and job status are used."""
    marker = json.loads(path.read_text())
    state = None
    if LINUX:
        state = (Path("/proc") / str(marker["pid"]) / "stat").read_text().rsplit(")", 1)[1].split()[0]
    return marker, state

def wait_until(predicate, timeout, message):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return True
        time.sleep(0.05)
    raise AssertionError(message)

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

    # --- RUNNING wait barrier: live worker held between ACK and plan body ---
    hold_root = home / "hold-cc"
    hold_root.mkdir()
    (hold_root / "CLAUDE.md").write_text("Synthetic held work is archived.")
    he = request(dict(command="register", name="Synthetic held", root=str(hold_root)))
    hp = request(dict(command="plan_reset", environment_id=he["id"], recipe="rebuild", categories=["instructions"]))
    hpayload = dict(command="execute", plan_id=hp["id"], approval=hp["hash"],
                    archive_passphrase="synthetic transient passphrase")
    marker = home / "hold-barrier.json"
    release = home / "hold-release.json"
    henv = dict(env, LINTEL_TEST_WAIT_BARRIER=str(marker), LINTEL_TEST_WAIT_RELEASE=str(release))
    submitted = subprocess.run([str(binary), "submit"], input=json.dumps(hpayload), text=True,
                               capture_output=True, env=henv, timeout=45, start_new_session=True)
    hack = json.loads(submitted.stdout)
    assert hack["ok"] and hack["data"]["status"] == "accepted", hack
    # The ACK is durable and the marker is written inside the synthetic home.
    assert (state / "jobs" / f'{hp["id"]}.json').exists(), "Wait barrier ACK preceded durable journal"
    wait_until(marker.is_file, 10, "RUNNING wait barrier never wrote its marker")
    hmarker, hstate = marker_facts(marker)
    assert hmarker["plan_id"] == hp["id"], "Barrier marker belongs to another job"
    assert all(k in hmarker for k in ("pid", "ppid", "pgrp", "session", "cgroup", "boot_id")), hmarker
    # Bounded wait: on Linux the worker must still be live (not a zombie/stopped/
    # stolen pid); on macOS the held job staying "accepted" is the bound.
    if LINUX:
        wait_until(lambda: marker_facts(marker)[1] in ("R", "S"), 10, "Held worker is not live")
    held = request(dict(command="job", plan_id=hp["id"]))
    assert held["status"] == "accepted", f"Held worker lost its operation ownership: {held}"
    # An explicit release under the same synthetic home lets the ORIGINAL plan run.
    assert not release.exists(), "Release file must not pre-exist"
    release.write_text("released\n")
    deadline = time.monotonic() + 45
    while True:
        done = request(dict(command="job", plan_id=hp["id"]))
        if done["status"] not in ["accepted", "executing", "verifying"]:
            break
        assert time.monotonic() < deadline, done
        time.sleep(.15)
    assert done["status"] == "partially_completed", done
    assert Path(done["new_root"]).joinpath("CLAUDE.md").read_text() == "Synthetic held work is archived."
    assert done["plan_id"] == hp["id"], "Released job reported a different plan"

    # Negative: a marker/release outside the synthetic home must NOT arm a barrier.
    with tempfile.TemporaryDirectory(prefix="lintel-outside-") as outside_tmp:
        outside = Path(outside_tmp)
        op = request(dict(command="plan_reset", environment_id=he["id"], recipe="rebuild", categories=["instructions"]))
        oenv = dict(env, LINTEL_TEST_WAIT_BARRIER=str(outside / "barrier.json"),
                    LINTEL_TEST_WAIT_RELEASE=str(outside / "release.json"))
        subprocess.run([str(binary), "submit"], input=json.dumps(dict(op, approval=op["hash"],
                       archive_passphrase="synthetic transient passphrase")), text=True,
                       capture_output=True, env=oenv, timeout=45, start_new_session=True)
        assert not (outside / "barrier.json").exists() and not (outside / "release.json").exists(), \
            "Barrier armed outside the synthetic home"
    print("PASS: durable ACK, parent exit, query original task, replay dedup, no stored passphrase; "
          "RUNNING wait barrier bounded/live/releasable and synthetic-home-only")

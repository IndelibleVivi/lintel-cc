#!/usr/bin/env python3
"""Real PTY resume exec closes the full decoded package staging.

Synthetic-only: a temporary home/root/state, an inert `claude` fixture that
records argv and exits, and an archive that carries the selected transcript plus
an unselected sensitive synthetic member. A successful `exec` replaces the
process, so the ordinary RAII drop never runs; the launch path must have closed
the decoded staging before exec. No real Claude, credentials, VPS or network.
"""
import errno
import json
import os
from pathlib import Path
import pty
import select
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / "target/debug/lintel"
PASS = "synthetic-resume-staging-passphrase"
SELECTED = b'{"type":"user","sessionId":"s","cwd":"/p","uuid":"u","message":{"content":[{"type":"text","text":"selected transcript"}]}}\n'
UNSELECTED_SECRET = b"synthetic unselected sensitive memory body"


def run():
    with tempfile.TemporaryDirectory(prefix="lintel-resume-staging-") as tmp:
        base = Path(tmp).resolve()
        home = base / "home"
        home.mkdir()
        state = base / "state"
        root = home / "root"
        root.mkdir()
        env = dict(os.environ, HOME=str(home), LINTEL_TEST_HOME=str(home),
                   LINTEL_STATE_DIR=str(state), PATH="/usr/bin:/bin")

        def request(payload):
            proc = subprocess.run([str(BINARY), "request"], input=json.dumps(payload),
                                  text=True, capture_output=True, env=env, timeout=120)
            result = json.loads(proc.stdout)
            assert result["ok"], (payload, result, proc.stderr)
            assert PASS not in proc.stdout + proc.stderr
            return result["data"]

        def cli(*args, payload=None):
            proc = subprocess.run([str(BINARY), *args],
                                  input="" if payload is None else json.dumps(payload),
                                  text=True, capture_output=True, env=env, timeout=120)
            result = json.loads(proc.stdout)
            assert result["ok"], (args, result, proc.stderr)
            assert PASS not in proc.stdout + proc.stderr
            return result["data"]

        # A real TTY exec fixture: record cwd/config/argc/argv and exit 0.
        versioned = home / "claude/versions/2.1.285"
        versioned.parent.mkdir(parents=True)
        capture = base / "client-observed"
        versioned.write_text(
            "#!/bin/sh\n"
            "printf '%s\\n' \"$PWD\" \"$CLAUDE_CONFIG_DIR\" \"$#\" \"$*\" > " + str(capture) + "\n"
            "exit 0\n"
        )
        versioned.chmod(0o700)
        link = home / ".local/bin/claude"
        link.parent.mkdir(parents=True)
        os.symlink(versioned, link)

        identity = request({"command": "register", "name": "synthetic resume staging", "root": str(root)})["id"]
        request({"command": "discover"})
        project = base / "project"
        project.mkdir()
        # Selected transcript plus an unselected sensitive member in the archive.
        (root / "projects/p").mkdir(parents=True)
        (root / "projects/p/s.jsonl").write_bytes(SELECTED)
        (root / "projects/p/other.jsonl").write_bytes(UNSELECTED_SECRET)

        archive = request({"command": "plan_archive", "environment_id": identity, "categories": ["sessions"]})
        receipt = cli("job", "submit", "--plan", archive["id"], "--approval", archive["hash"],
                      payload={"archive_passphrase": PASS})
        archived = cli("job", "wait", archive["id"], "--timeout", "30s")
        cipher = Path(archived["archive_path"])
        original_cipher = cipher.read_bytes()
        plan = request({"command": "plan_resume", "environment_id": identity,
                            "project_cwd": str(project), "job_id": receipt["id"],
                            "archive_passphrase": PASS, "path": "projects/p/s.jsonl"})
        assert plan["resume"]["supported"] is True, plan
        copy = Path(plan["resume"]["private_copy_path"])
        assert not copy.exists(), "preview created the private copy"
        # The frozen archive path is decrypted into a private staging dir under
        # the state root; none may survive the successful exec below.
        def staging_dirs():
            return [p for p in state.glob(".lintel-work-stage-*")]
        assert staging_dirs() == [], "staging existed before the resume"

        master, slave = pty.openpty()
        proc = None
        try:
            proc = subprocess.Popen([str(BINARY), "launch", "resume", plan["id"], plan["hash"]],
                                    env=env, stdin=slave, stdout=slave, stderr=slave,
                                    start_new_session=True)
            os.close(slave)
            out = bytearray()
            wrote_secret = False
            deadline = time.monotonic() + 30
            while True:
                assert time.monotonic() < deadline, "resume PTY timed out: " + out.decode(errors="replace")
                if select.select([master], [], [], 0.2)[0]:
                    try:
                        block = os.read(master, 65536)
                    except OSError as error:
                        if error.errno == errno.EIO:
                            break
                        raise
                    if not block:
                        break
                    out.extend(block)
                    if not wrote_secret and "归档口令" in out.decode(errors="replace"):
                        os.write(master, (PASS + "\n").encode())
                        wrote_secret = True
                elif proc.poll() is not None:
                    break
            assert proc.wait(timeout=5) == 0, out.decode(errors="replace")
        finally:
            os.close(master)
            if proc is not None and proc.poll() is None:
                proc.terminate()
                proc.wait(timeout=5)

        # The successful exec ran the frozen client with the private copy only.
        observed = capture.read_text().splitlines()
        assert observed[2] == "3", observed
        assert "--resume" in observed[3] and "--fork-session" in observed[3], observed
        assert str(copy) in observed[3] and str(root) not in observed[3], observed
        # The approved private copy is intact and byte-exact.
        assert copy.exists(), "private copy missing after successful exec"
        assert copy.read_bytes() == SELECTED, "private copy differs from the selected transcript"
        assert oct(copy.stat().st_mode & 0o777) == "0o600"
        # The full decoded package staging (which held the unselected sensitive
        # member) is gone: the launch path closed it before the process replaced
        # itself, so no plaintext from the package is left behind.
        assert staging_dirs() == [], f"decoded staging survived exec: {staging_dirs()}"
        # No decoded plaintext anywhere under the state root leaks the secret.
        for path in state.rglob("*"):
            if path.is_file():
                assert UNSELECTED_SECRET not in path.read_bytes(), f"secret leaked in {path}"
        # The original archive is untouched and the source bytes are unchanged.
        assert (root / "projects/p/s.jsonl").read_bytes() == SELECTED
        assert (root / "projects/p/other.jsonl").read_bytes() == UNSELECTED_SECRET
        assert cipher.read_bytes() == original_cipher, "resume changed the original encrypted package"
        # A repeat is query-only and runs no second exec.
        before = capture.read_bytes()
        replay = cli("launch", "query", plan["id"])
        assert replay["observed"] == "record" and replay["replayed_query"] is True, replay
        assert capture.read_bytes() == before, "repeat resume re-ran the client"
        print("PASS: real PTY resume exec closes full-package staging; approved private copy intact; "
              "unselected sensitive member leaves no plaintext; original archive unchanged; query-only repeat")


if __name__ == "__main__":
    run()

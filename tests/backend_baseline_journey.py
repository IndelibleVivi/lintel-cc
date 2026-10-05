#!/usr/bin/env python3
"""Backend baseline journey (SPEC A/C/D): frozen targets, bounded reading,
finite launch/resume and readonly static CLI. All fixtures are synthetic
temporary roots; no real Claude, credentials, VPS or network is used."""
import json
import os
from pathlib import Path
import pty
import select
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / "target/debug/lintel"
PASS = "synthetic-baseline-passphrase"


def run():
    with tempfile.TemporaryDirectory(prefix="lintel-backend-baseline-") as tmp:
        base = Path(tmp).resolve()
        home = base / "home"
        home.mkdir()
        state = base / "state"
        env = dict(os.environ, HOME=str(home), LINTEL_TEST_HOME=str(home),
                   LINTEL_STATE_DIR=str(state), PATH="/usr/bin:/bin")

        def cli(*args, payload=None, good=True, context=env):
            proc = subprocess.run([str(BINARY), *args],
                                  input="" if payload is None else json.dumps(payload),
                                  text=True, capture_output=True, env=context, timeout=60)
            if proc.returncode not in (0, 1) or not proc.stdout.strip():
                raise AssertionError((args, proc.returncode, proc.stdout, proc.stderr))
            result = json.loads(proc.stdout)
            assert result["ok"] is good, (args, result)
            assert (proc.returncode == 0) is good, (args, result, proc.returncode)
            assert PASS not in proc.stdout + proc.stderr
            return result["data"] if good else result

        # --- readonly static commands never create state (A12/A14) -----------
        for args in [("context", "--json"), ("tasks", "--json"), ("version", "--json"),
                     ("capabilities", "--json"), ("describe", "plan_launch"),
                     ("schema", "session_read")]:
            cli(*args)
            assert not state.exists(), f"static command created state: {args}"

        ctx = cli("context", "--json")
        assert ctx["state"]["exists"] is False and ctx["initialized"] is False, ctx
        assert ctx["state"]["source"] == "LINTEL_STATE_DIR", ctx
        assert not state.exists(), "context created state"

        # Named group help is static and covers every group.
        for group in ["env", "policy", "work", "job", "restore", "remote", "launch", "session"]:
            data = cli("help", group)
            assert data["static"] is True and data["group"] == group, data
        assert not state.exists(), "group help created state"
        for words in [
            ("env", "register", "--help"), ("work", "archive", "plan", "--help"),
            ("work", "session", "read", "-h"), ("job", "submit", "--help"),
            ("restore", "plan", "help"), ("launch", "resume", "--help"),
            ("launch", "request", "help"), ("remote", "launch", "resume", "--help"),
            ("plan", "show", "--help"), ("browser", "pair", "approve", "--help"),
            ("network", "serve", "--help"),
        ]:
            data = cli(*words)
            assert data["static"] is True and data["group"] == words[0], (words, data)
            assert not state.exists(), f"nested help created state: {words}"


        tasks = cli("tasks", "--json")["tasks"]
        ids = [t["id"] for t in tasks]
        assert ids == ["reduce_egress", "preserve_work", "repair_cleanup_retire",
                       "browser_profile", "ssh_remote", "recover_results"], ids
        anchors = {t["id"]: t["help_anchor"] for t in tasks}
        assert anchors["recover_results"] == "recovery" and anchors["browser_profile"] == "browser", anchors

        # --- frozen new target + bounded reading (A05/A07/A08) ---------------
        root = home / "synthetic-claude"
        root.mkdir()
        (root / "projects/p").mkdir(parents=True)
        (root / "CLAUDE.md").write_text("instruction\n")
        lines = []
        expected = []
        for index in range(6000):
            text = f"记录 {index} data {index}"
            expected.append(text)
            lines.append(json.dumps({"type": "user", "timestamp": f"2026-01-01T00:00:{index % 60:02d}Z",
                                     "message": {"content": text}}, ensure_ascii=False))
        (root / "projects/p/s.jsonl").write_text("\n".join(lines) + "\n", encoding="utf-8")
        identity = cli("env", "register", "--name", "Synthetic", "--root", str(root))["id"]

        preserve = cli("call", "plan_preserve", payload={"environment_id": identity, "categories": ["instructions", "sessions"]})
        planned = preserve["planned_target"]["new_root"]
        assert preserve["plan_revision"] == "lintel.plan/2"
        assert not Path(planned).exists(), "preview created the planned root"
        assert all(f["destination"].startswith(planned) for f in preserve["planned_target"]["files"])
        # Offline collision: occupy the planned root then request execution.
        Path(planned).mkdir(parents=True)
        (Path(planned) / "occupier").write_text("x")
        refused = cli("job", "submit", "--plan", preserve["id"], "--approval", preserve["hash"],
                      payload={"archive_passphrase": PASS}, good=False)
        assert refused["error"]["code"] == "stale_plan", refused
        assert (Path(planned) / "occupier").exists()
        import shutil
        shutil.rmtree(planned)

        receipt = cli("job", "submit", "--plan", preserve["id"], "--approval", preserve["hash"],
                      payload={"archive_passphrase": PASS})
        done = cli("job", "wait", preserve["id"], "--timeout", "30s")
        assert done["status"] == "completed", done
        assert done["new_root"] == planned, (done["new_root"], planned)
        assert done["task_result"]["outcome"] == "completed"
        # planned_target coverage reflects the executed create step.
        cov = {c["scope"]: c["state"] for c in done["task_result"]["coverage"]}
        assert cov.get("planned_target") == "done", done["task_result"]

        # Read the entire long corpus across bounded pages and compare in order.
        manifest = cli("work", "archive", "inspect", "--job", done["id"], payload={"archive_passphrase": PASS})
        member = next(f for f in manifest["files"] if f["path"].endswith("s.jsonl"))
        offset, collected, pages = 0, [], 0
        while True:
            page = cli("work", "session", "read", "--job", done["id"],
                       payload={"archive_passphrase": PASS, "path": member["path"],
                                "offset": offset, "expected_digest": member["digest"]})
            pages += 1
            assert pages < 500, "paging did not terminate"
            assert page["source"]["package_digest"] == done["archive_digest"], page["source"]
            for record in page["records"]:
                if record.get("kind") == "user" and "text" in record:
                    collected.append(record["text"])
            if page["done"]:
                break
            offset = page["next_offset"]
            assert offset is not None and offset > 0
        assert collected == expected, f"paged reading lost records ({len(collected)} vs {len(expected)})"

        # A wrong bound digest is refused and never maps to another source.
        bad = cli("work", "session", "read", "--job", done["id"],
                  payload={"archive_passphrase": PASS, "path": member["path"],
                           "expected_digest": "a" * 64}, good=False)
        assert bad["error"]["code"] == "stale_archive", bad

        # --- finite launch request: one immutable ID, no replay (A04) --------
        # A versioned native install so static metadata reports a declared version,
        # reachable through the ~/.local/bin fallback.
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
        if link.exists() or link.is_symlink():
            link.unlink()
        os.symlink(versioned, link)
        cli("discover")  # records the resolved native executable
        project = base / "project"
        project.mkdir()
        launch = cli("call", "plan_launch", payload={"environment_id": identity,
                                                     "project_cwd": str(project), "mode": "interactive"})
        assert launch["launch_request"]["id"] == launch["id"], launch
        assert launch["launch_request"]["project_cwd"] == str(project), launch

        # launch_query is readonly and finds the frozen plan before any attempt.
        planned = cli("launch", "query", launch["id"])
        assert planned["status"] == "planned" and planned["observed"] == "plan", planned
        assert planned["root"] == launch["launch_request"]["config_root"], planned

        # A real TTY reaches the fixed client exec. The inert executable exits 0
        # immediately, so this exercises the actual terminal handoff without a
        # personal client. On a non-TTY the server must refuse specifically.
        def pty_call(args, payload, expected=0, secret=None, wait_for=None):
            import errno
            import time
            master, slave = pty.openpty()
            proc = None
            try:
                proc = subprocess.Popen([str(BINARY), *args], env=env, stdin=slave,
                                        stdout=slave, stderr=slave, start_new_session=True)
                os.close(slave)
                if payload is not None:
                    os.write(master, (json.dumps(payload) + "\n").encode())
                out = bytearray()
                deadline = time.monotonic() + 20
                while True:
                    assert time.monotonic() < deadline, "pty_call timed out: " + out.decode(errors="replace")
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
                        # Only write the passphrase AFTER the no-echo prompt is
                        # observed, so it can never leak into echo output.
                        if secret is not None and wait_for is not None and wait_for in out.decode(errors="replace"):
                            os.write(master, (secret + "\n").encode())
                            secret = None
                    elif proc.poll() is not None:
                        break
                assert proc.wait(timeout=5) == expected, out.decode(errors="replace")
                decoded = out.decode(errors="replace")
                assert PASS not in decoded, "passphrase echoed by TTY"
                return decoded
            finally:
                os.close(master)
                if proc is not None and proc.poll() is None:
                    proc.terminate()
                    proc.wait(timeout=5)

        # Non-TTY core JSON must be refused, never silently executed.
        no_tty = cli("call", "launch_request", payload={"request_id": launch["id"], "approval": launch["hash"]}, good=False)
        assert no_tty["error"]["code"] == "terminal_required", no_tty
        assert not (state / "launches" / f"{launch['id']}.json").exists()
        # The dedicated TTY entry (lintel launch request <id> <hash>) is the
        # parser branch that actually runs the client.
        pty_call(["launch", "request", launch["id"], launch["hash"]], None)
        observed = capture.read_text().splitlines()
        assert observed[0] == str(project), observed
        assert observed[1] == launch["launch_request"]["config_root"], observed
        assert observed[2] == "0", observed  # no prompt/argv for a plain launch
        recorded = state / "launches" / f"{launch['id']}.json"
        assert recorded.exists(), "no durable launch intent recorded"
        record = json.loads(recorded.read_text())
        # Direct exec replaces the process on success, so the durable record stays
        # at the pending intent (never a false completion); a rejected Terminal
        # open records launch_failed.
        assert record["status"] in ("launch_intent", "launch_requested", "launch_failed"), record
        assert record["environment_id"] == identity, record
        assert record["project_cwd"] == str(project), record
        # Repeat over the TTY is a pure query and never opens a second session.
        pty_call(["launch", "request", launch["id"], launch["hash"]], None)
        assert json.loads(recorded.read_text())["status"] == record["status"]
        replay = cli("launch", "query", launch["id"])
        assert replay["observed"] == "record" and replay["replayed_query"] is True, replay
        # The JSON/pipe entry can never select the interactive exec (fix #1); it
        # refuses specifically and records nothing.
        other = cli("call", "plan_launch", payload={"environment_id": identity,
                                                    "project_cwd": str(project), "mode": "interactive"})
        refused_json = cli("call", "launch_request", payload={"request_id": other["id"], "approval": other["hash"]}, good=False)
        assert refused_json["error"]["code"] == "terminal_required", refused_json
        assert not (state / "launches" / f"{other['id']}.json").exists()
        # A wrong approval over the real TTY is refused before any intent.
        pty_call(["launch", "request", other["id"], "a" * 64], None, expected=1)
        assert not (state / "launches" / f"{other['id']}.json").exists()

        # --- native resume: refusal vs supported synthetic adapter (A11) -----
        # The transcript carries the fields the declared resume path relies on.
        body = json.dumps({"type": "user", "sessionId": "s", "cwd": str(project), "uuid": "u",
                           "message": {"content": [{"type": "text", "text": "hi"}]}}) + "\n"
        (root / "projects/r.jsonl").write_text(body)
        archive = cli("call", "plan_archive", payload={"environment_id": identity, "categories": ["sessions"]})
        archive_receipt = cli("job", "submit", "--plan", archive["id"], "--approval", archive["hash"],
                              payload={"archive_passphrase": PASS})
        cli("job", "wait", archive["id"], "--timeout", "30s")
        unknown = cli("call", "plan_resume", payload={"environment_id": identity, "project_cwd": str(project),
                                                      "job_id": archive_receipt["id"], "archive_passphrase": PASS,
                                                      "path": "projects/r.jsonl"})
        assert unknown["resume"]["supported"] is True, unknown
        assert unknown["resume"]["auth_unverified"] is True and unknown["resume"]["archive_unmodified"] is True
        assert unknown["resume"]["write_scope"]["config_root"] == unknown["resume"]["config_root"], unknown
        copy = unknown["resume"]["private_copy_path"]
        assert not Path(copy).exists(), "preview created the private copy"
        assert "code.claude.com" in json.dumps(unknown["resume"]["client_support"]), unknown
        assert json.loads(json.dumps(unknown["resume"]["client_support"]))["declared_versions"], unknown
        # The dedicated TTY resume entry writes the 0600 copy and runs --resume.
        pty_call(["launch", "resume", unknown["id"], unknown["hash"]], None,
                 secret=PASS, wait_for="归档口令")
        observed_resume = capture.read_text().splitlines()
        # $* is the whole argv: the finite resume adapter passes the absolute
        # private copy and --fork-session, never a rewritten ID or original path.
        assert observed_resume[2] == "3", observed_resume
        argv = observed_resume[3]
        assert "--resume" in argv and "--fork-session" in argv, argv
        assert copy in argv and str(root) not in argv, argv
        assert observed_resume[1] == unknown["resume"]["config_root"], observed_resume
        assert Path(copy).exists(), "private running copy missing"
        assert Path(copy).read_bytes() == body.encode(), "private copy bytes differ from source"
        assert oct(Path(copy).stat().st_mode & 0o777) == "0o600"
        # A repeat is query-only.
        replay_r = cli("launch", "query", unknown["id"])
        assert replay_r["observed"] == "record" and replay_r["replayed_query"] is True, replay_r
        before = capture.read_bytes()
        repeat_tty = pty_call(["launch", "resume", unknown["id"], "wrong"], None)
        assert "归档口令" not in repeat_tty and unknown["id"] in repeat_tty
        assert capture.read_bytes() == before, "repeat resumed the client again"


        # --- secret hygiene: no passphrase persisted ------------------------
        for path in state.rglob("*.json"):
            assert PASS not in path.read_text(), f"secret persisted in {path}"

        print("PASS: readonly static CLI; frozen new target + offline collision; "
              "structured result/coverage; bounded full-corpus paging with digest binding; "
              "finite one-ID launch with no-replay; native resume supported/refusal + private copy; "
              "no persisted passphrase")


if __name__ == "__main__":
    run()

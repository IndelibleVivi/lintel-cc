#!/usr/bin/env python3
"""Portable work-preservation journey across independent Lintel installs.

Follows the new core contract on top of the existing work/archive path:
  - plan_archive is archive-only: it never creates a root and never touches
    settings or credentials;
  - a preserve plan completes and leaves the original root/login intact, while
    the legacy plan_reset(recipe=rebuild) receipt stays partially_completed;
  - the encrypted package can be carried to a *separate* home/state (no access
    to the original job or inventory) and be inspected, read and imported via an
    explicit absolute archive_path, refusing overwrite/existing names;
  - an accepted-then-failed receipt persists a structured error across a fresh
    process;
  - plan_show reads the frozen plan back without any secret.

Never points at the operator's home. Build `cargo build -p lintel-runner`
first, then run: python3 tests/portable_work_journey.py [path-to-lintel]
"""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 and not sys.argv[1].startswith('-') else ROOT / 'target/debug/lintel'
PASSPHRASE = "synthetic-portable-passphrase-only"


class Install:
    """One independent Lintel install: its own synthetic home and state."""

    def __init__(self, base: Path, name: str) -> None:
        self.base = base
        self.home = base / name / "home"
        self.state = base / name / "state"
        self.home.mkdir(parents=True)
        self.env = {
            "HOME": str(self.home),
            "LINTEL_TEST_HOME": str(self.home),
            "LINTEL_STATE_DIR": str(self.state),
            "PATH": "/usr/bin:/bin",
        }
        self.approved: dict[str, str] = {}

    def call(self, command: str, **fields: Any) -> dict[str, Any]:
        if command == "execute":
            assert self.approved.get(fields["plan_id"]) == fields["approval"], "execute used an unapproved hash"
        payload = {"command": command, **fields}
        result = subprocess.run(
            [str(BINARY), "request"], input=json.dumps(payload), text=True,
            capture_output=True, cwd=self.base, env=self.env, timeout=45, check=False,
        )
        try:
            response = json.loads(result.stdout)
        except json.JSONDecodeError as error:
            raise AssertionError(
                f"runner did not return JSON: exit={result.returncode}; {error}; {result.stderr[:300]}"
            ) from error
        assert isinstance(response, dict), response
        assert "ok" in response, response
        return response

    def data(self, command: str, **fields: Any) -> Any:
        response = self.call(command, **fields)
        assert response["ok"], json.dumps(response.get("error"), ensure_ascii=False)
        return response["data"]

    def named(self, *args: str, **fields: Any) -> Any:
        result = subprocess.run(
            [str(BINARY), *args], input=json.dumps(fields), text=True,
            capture_output=True, cwd=self.base, env=self.env, timeout=45, check=False,
        )
        response = json.loads(result.stdout)
        assert response["ok"], response
        return response["data"]

    def register(self, root: Path, name: str = "synthetic") -> str:
        environment = self.data("register", name=name, root=str(root))
        return str(environment["id"])

    def approve(self, plan: dict[str, Any]) -> None:
        self.approved[plan["id"]] = plan["hash"]

    def execute(self, plan: dict[str, Any]) -> dict[str, Any]:
        self.approve(plan)
        return self.data("execute", plan_id=plan["id"], approval=plan["hash"], archive_passphrase=PASSPHRASE)


class PortableWorkJourney(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="lintel-portable-")
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name).resolve()
        self.source = Install(self.base, "source")
        self.root = self.source.home / "claude-root"
        self.root.mkdir(parents=True)
        self.write("CLAUDE.md", b"Synthetic instruction only.\n")
        self.write("projects/example/memory/MEMORY.md", b"Synthetic memory only.\n")
        self.write("projects/example/session.jsonl", b'{"synthetic":true}\n')
        self.write(".credentials.json", b"SYNTHETIC_CREDENTIAL_DO_NOT_COPY")
        self.write("settings.json", b'{"env":{"SYNTHETIC_KEEP":"1"}}')

    def write(self, relative: str, content: bytes) -> None:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)

    def test_archive_only_creates_no_root_and_carries_package(self) -> None:
        environment_id = self.source.register(self.root)
        before = len(self.source.data("discover")["environments"])
        out = self.base / "carried.age"
        plan = self.source.data(
            "plan_archive", environment_id=environment_id,
            categories=["instructions", "memory", "sessions"], output_path=str(out),
        )
        self.assertEqual(plan["kind"], "archive")
        self.assertEqual(plan["outcome"], "archive_only")
        receipt = self.source.execute(plan)
        self.assertEqual(receipt["status"], "completed")
        self.assertNotIn("new_root", receipt)
        self.assertFalse((self.root / ".credentials.json").read_bytes().decode() != "SYNTHETIC_CREDENTIAL_DO_NOT_COPY")
        self.assertEqual((self.root / "settings.json").read_bytes(), b'{"env":{"SYNTHETIC_KEEP":"1"}}')
        self.assertEqual(len(self.source.data("discover")["environments"]), before, "archive-only created a root")
        encrypted = out.read_bytes()
        self.assertTrue(encrypted.startswith(b"age-encryption.org/"))
        self.assertNotIn(b"Synthetic instruction", encrypted)
        # A frozen output path is never overwritten.
        again = self.source.call(
            "plan_archive", environment_id=environment_id,
            categories=["instructions"], output_path=str(out),
        )
        self.assertFalse(again["ok"])
        self.assertEqual(again["error"]["code"], "output_exists")
        # plan_show reads the frozen plan without any secret.
        shown = self.source.data("plan_show", plan_id=plan["id"])
        self.assertEqual(shown["hash"], plan["hash"])
        self.assertNotIn(PASSPHRASE, json.dumps(shown))
        self.assertNotIn("snapshot", shown)

    def test_package_imports_into_independent_install(self) -> None:
        self.write("lintel-imports/projects/example/session.jsonl", b"Earlier preserved session.\n")
        self.write("lintel-imports/projects/example/memory/MEMORY.md", b"Earlier preserved memory.\n")
        self.write("lintel-imports/projects/foo.jsonl", b"Earlier ancestor file.\n")
        self.write("projects/foo.jsonl/session.jsonl", b"Active nested session.\n")
        environment_id = self.source.register(self.root)
        out = self.base / "carried.age"
        plan = self.source.data(
            "plan_archive", environment_id=environment_id,
            categories=["instructions", "memory", "sessions"], output_path=str(out),
        )
        self.source.execute(plan)

        target = Install(self.base, "target")
        dest = target.home / "other-claude"
        dest.mkdir(parents=True)
        imports = dest / "lintel-imports"
        projects = imports / "projects"
        projects.mkdir(parents=True)
        existing_modes = [(dest, 0o755), (imports, 0o775), (projects, 0o750)]
        for directory, mode in existing_modes:
            directory.chmod(mode)
        dest_id = target.register(dest)
        path = str(out)
        manifest = target.data("archive_inspect", archive_path=path, archive_passphrase=PASSPHRASE)
        self.assertEqual(manifest["generator"], "Lintel")
        self.assertEqual(len(manifest["files"]), 7)
        read = target.data("archive_read", archive_path=path, archive_passphrase=PASSPHRASE, path="CLAUDE.md")
        self.assertEqual(read["text"], "Synthetic instruction only.\n")
        import_plan = target.named(
            "work", "import", "plan", "--environment", dest_id, "--archive-path", path,
            "--categories", "instructions,memory,sessions", archive_passphrase=PASSPHRASE,
        )
        public_manifest = import_plan["import_manifest"]
        self.assertEqual(public_manifest["package"], {
            "format": "lintel.work/1", "generator": "Lintel",
            "sha256": hashlib.sha256(out.read_bytes()).hexdigest(),
        })
        shown = target.named("plan", "show", import_plan["id"])
        self.assertEqual(shown, import_plan)
        expected_mapping = {
            "projects/example/session.jsonl": "lintel-imports/projects/example/lintel-1-session.jsonl",
            "projects/example/memory/MEMORY.md": "lintel-imports/projects/example/memory/lintel-1-MEMORY.md",
            "lintel-imports/projects/foo.jsonl": "lintel-imports/projects/lintel-1-foo.jsonl",
            "projects/foo.jsonl/session.jsonl": "lintel-imports/projects/foo.jsonl/session.jsonl",
        }
        for entry in public_manifest["files"]:
            self.assertEqual(set(entry), {"source", "destination", "category", "size", "sha256"})
            self.assertFalse(Path(entry["source"]).is_absolute())
            self.assertFalse(Path(entry["destination"]).is_absolute())
            if entry["source"] in expected_mapping:
                self.assertEqual(entry["destination"], expected_mapping[entry["source"]])
        encoded = json.dumps(shown)
        # The approved plan now names the frozen target root and each file's
        # absolute final destination (SPEC §6.1): the user must see where content
        # lands before approving. Secrets and file bodies stay absent, and the
        # source archive path is not part of this public manifest.
        for private in [PASSPHRASE, "Synthetic instruction only.", str(out)]:
            self.assertNotIn(private, encoded)
        self.assertIn(str(dest), encoded)
        receipt = target.execute(import_plan)
        self.assertEqual(receipt["status"], "completed")
        for directory, mode in existing_modes:
            self.assertEqual(directory.stat().st_mode & 0o777, mode)
        for directory in [projects / "example", projects / "example/memory", projects / "foo.jsonl"]:
            self.assertEqual(directory.stat().st_mode & 0o777, 0o700)
        self.assertEqual((dest / "CLAUDE.md").read_bytes(), b"Synthetic instruction only.\n")
        self.assertEqual((dest / "lintel-imports/projects/example/session.jsonl").read_bytes(), b"Earlier preserved session.\n")
        self.assertEqual((dest / "lintel-imports/projects/example/lintel-1-session.jsonl").read_bytes(), b'{"synthetic":true}\n')
        self.assertEqual((dest / "lintel-imports/projects/example/memory/MEMORY.md").read_bytes(), b"Earlier preserved memory.\n")
        self.assertEqual((dest / "lintel-imports/projects/example/memory/lintel-1-MEMORY.md").read_bytes(), b"Synthetic memory only.\n")
        self.assertEqual((dest / "lintel-imports/projects/lintel-1-foo.jsonl").read_bytes(), b"Earlier ancestor file.\n")
        self.assertEqual((dest / "lintel-imports/projects/foo.jsonl/session.jsonl").read_bytes(), b"Active nested session.\n")
        # Wrong passphrase and corrupt packages are refused.
        wrong = target.call("archive_inspect", archive_path=path, archive_passphrase="definitely-wrong-synthetic")
        self.assertEqual(wrong["error"]["code"], "archive_locked")
        damaged = self.base / "damaged.age"
        damaged.write_bytes(b"not an age archive")
        broken = target.call("archive_inspect", archive_path=str(damaged), archive_passphrase=PASSPHRASE)
        self.assertEqual(broken["error"]["code"], "invalid_archive")
        # Same-name import is refused; the already-imported file is untouched.
        conflict = target.call(
            "plan_import", environment_id=dest_id, archive_path=path,
            categories=["instructions"], archive_passphrase=PASSPHRASE,
        )
        self.assertEqual(conflict["error"]["code"], "import_conflict")

    def test_preserve_completes_and_legacy_reset_is_partial(self) -> None:
        self.write("lintel-imports/projects/example/session.jsonl", b"Earlier preserved session.\n")
        self.write("lintel-imports/projects/example/memory/MEMORY.md", b"Earlier preserved memory.\n")
        self.write("lintel-imports/projects/foo.jsonl", b"Earlier ancestor file.\n")
        self.write("projects/foo.jsonl/session.jsonl", b"Active nested session.\n")
        environment_id = self.source.register(self.root)
        preserve = self.source.data(
            "plan_preserve", environment_id=environment_id,
            categories=["instructions", "memory", "sessions"], name="writing",
        )
        self.assertEqual(preserve["kind"], "preserve")
        receipt = self.source.execute(preserve)
        self.assertEqual(receipt["status"], "completed")
        self.assertEqual(receipt["outcome"], "preserved")
        new_root = Path(receipt["new_root"])
        self.assertEqual((new_root / "CLAUDE.md").read_bytes(), b"Synthetic instruction only.\n")
        self.assertEqual((new_root / "lintel-imports/projects/example/session.jsonl").read_bytes(), b"Earlier preserved session.\n")
        self.assertEqual((new_root / "lintel-imports/projects/example/lintel-1-session.jsonl").read_bytes(), b'{"synthetic":true}\n')
        self.assertEqual((new_root / "lintel-imports/projects/example/memory/MEMORY.md").read_bytes(), b"Earlier preserved memory.\n")
        self.assertEqual((new_root / "lintel-imports/projects/example/memory/lintel-1-MEMORY.md").read_bytes(), b"Synthetic memory only.\n")
        self.assertEqual((new_root / "lintel-imports/projects/lintel-1-foo.jsonl").read_bytes(), b"Earlier ancestor file.\n")
        self.assertEqual((new_root / "lintel-imports/projects/foo.jsonl/session.jsonl").read_bytes(), b"Active nested session.\n")
        # The retained old environment is a desired outcome, not an outstanding
        # task: the step is reported as `preserved` with explicit coverage.
        retained = next(step for step in receipt["steps"] if step["id"] == "status")
        self.assertEqual(retained["status"], "preserved")
        self.assertEqual(receipt["coverage"]["old_login"], "retained")
        self.assertEqual(receipt["coverage"]["service_binding"], "unchanged")
        self.assertTrue(receipt["next_steps"])
        # Original root/login/settings untouched.
        self.assertTrue((self.root / ".credentials.json").exists())
        self.assertEqual((self.root / "settings.json").read_bytes(), b'{"env":{"SYNTHETIC_KEEP":"1"}}')
        legacy = self.source.data(
            "plan_reset", environment_id=environment_id, recipe="rebuild", categories=["instructions"],
        )
        legacy_receipt = self.source.execute(legacy)
        self.assertEqual(legacy_receipt["status"], "partially_completed")

    def test_reset_client_keeps_preservation_before_destruction(self) -> None:
        # A fake CLI whose logout fails. preview and receipt order must both keep
        # every preservation step ahead of logout and the local file removals.
        script = self.source.home / ".local/bin/claude"
        script.parent.mkdir(parents=True)
        script.write_text(
            "#!/bin/sh\ncase \"$2\" in\n"
            " status) printf '{\"configDirectory\":\"%s\",\"authMethod\":\"claude.ai\"}' \"$CLAUDE_CONFIG_DIR\"; exit 0;;\n"
            " logout) exit 3;;\nesac\nexit 9\n"
        )
        script.chmod(0o700)
        environment_id = self.source.register(self.root)
        # Discovery and the execution recheck must resolve the same inert CLI.
        # Native-install fallback stays confined to this synthetic HOME.
        planned = self.source.call(
            "plan_cleanup", environment_id=environment_id, recipe="reset_client",
            writers_confirmed_stopped=True, official_logout=True, categories=["instructions"],
        )
        self.assertTrue(planned["ok"], json.dumps(planned.get("error"), ensure_ascii=False))
        plan = planned["data"]
        order = [action["id"] for action in plan["actions"]]
        index = {name: order.index(name) for name in order}
        self.assertEqual(order[0], "quiescence")
        self.assertLess(index["archive"], index["rebuild"])
        self.assertLess(index["rebuild"], index["logout"])
        self.assertLess(index["logout"], index["credentials"])
        self.source.approve(plan)
        receipt = self.source.data(
            "execute", plan_id=plan["id"], approval=plan["hash"], archive_passphrase=PASSPHRASE,
        )
        # Logout failed AFTER the fresh root and migration were produced.
        self.assertEqual(receipt["status"], "needs_reconciliation")
        self.assertEqual(receipt["error"]["code"], "logout_failed")
        self.assertTrue(receipt["archive_path"])
        self.assertTrue(receipt["state_archive_path"])
        self.assertTrue(receipt["new_root"])
        self.assertEqual(
            (Path(receipt["new_root"]) / "CLAUDE.md").read_bytes(), b"Synthetic instruction only.\n"
        )
        step_ids = [step["id"] for step in receipt["steps"]]
        self.assertLess(step_ids.index("archive"), step_ids.index("create"))
        self.assertLess(step_ids.index("create"), step_ids.index("logout"))
        # No destructive removal happened.
        self.assertNotIn("credentials", step_ids)
        self.assertNotIn("client_state", step_ids)
        self.assertTrue((self.root / ".credentials.json").exists())

    def test_cleanup_freezes_executable_before_any_auth_recheck(self) -> None:
        scripts = []
        markers = []
        for name in ("first", "second"):
            directory = self.base / name
            directory.mkdir()
            script = directory / "claude"
            marker = directory / "calls"
            script.write_text(
                '#!/bin/sh\nprintf "%s\\n" "$2" >> "' + str(marker) + '"\n'
                "printf '{\"configDirectory\":\"%s\",\"authMethod\":\"claude.ai\"}' \"$CLAUDE_CONFIG_DIR\"\n"
            )
            script.chmod(0o700)
            scripts.append(directory)
            markers.append(marker)
        self.source.env["PATH"] = str(scripts[0]) + ":/usr/bin:/bin"
        environment_id = self.source.register(self.root)
        for discover_after_change in (False, True):
            self.source.env["PATH"] = str(scripts[0]) + ":/usr/bin:/bin"
            self.source.data("discover")
            plan = self.source.data(
                "plan_cleanup", environment_id=environment_id, recipe="repair_login",
                writers_confirmed_stopped=True, official_logout=True,
            )
            calls_before = markers[0].read_bytes()
            self.source.env["PATH"] = str(scripts[1]) + ":/usr/bin:/bin"
            if discover_after_change:
                self.source.data("discover")
            self.source.approve(plan)
            rejected = self.source.call("execute", plan_id=plan["id"], approval=plan["hash"])
            self.assertEqual(rejected["error"]["code"], "executable_changed")
            self.assertEqual(markers[0].read_bytes(), calls_before)
            self.assertFalse(markers[1].exists(), "replacement executable was probed under an old approval")
            self.assertFalse((self.source.state / "jobs" / (plan["id"] + ".json")).exists())
            self.assertTrue((self.root / ".credentials.json").exists())

    def test_cleanup_rechecks_new_writer_after_preservation_with_or_without_logout(self) -> None:
        script = self.source.home / ".local/bin/claude"
        script.parent.mkdir(parents=True)
        logout_marker = self.base / "unexpected-logout"
        script.write_text(
            '#!/bin/sh\nif test "$2" = logout; then touch "' + str(logout_marker) + '"; exit 3; fi\n'
            "printf '{\"configDirectory\":\"%s\",\"authMethod\":\"claude.ai\"}' \"$CLAUDE_CONFIG_DIR\"\n"
        )
        script.chmod(0o700)
        # This inert sleeper has the process name that the production ps check
        # recognizes; its only binding is our disposable root.
        writer_path = self.base / "writer" / "claude"
        writer_path.parent.mkdir()
        source = writer_path.with_suffix(".c")
        source.write_text("#include <unistd.h>\nint main(void) { sleep(40); return 0; }\n")
        # The build toolchain already needs cc. Compile our inert fixture rather
        # than copying a signed system binary (which macOS may refuse to run).
        compiled = subprocess.run(
            ["cc", str(source), "-o", str(writer_path)], cwd=self.base,
            env=self.source.env, capture_output=True, text=True, check=False, timeout=30,
        )
        self.assertEqual(compiled.returncode, 0, compiled.stderr)
        environment_id = self.source.register(self.root)
        for official_logout in (False, True):
            with self.subTest(official_logout=official_logout):
                plan = self.source.data(
                    "plan_cleanup", environment_id=environment_id, recipe="reset_client",
                    writers_confirmed_stopped=True, official_logout=official_logout,
                    categories=["instructions"],
                )
                self.source.approve(plan)
                process = subprocess.Popen(
                    [str(BINARY), "request"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE, text=True, cwd=self.base, env=self.source.env,
                )
                process.stdin.write(json.dumps({
                    "command": "execute", "plan_id": plan["id"], "approval": plan["hash"],
                    "archive_passphrase": PASSPHRASE,
                }))
                process.stdin.close()
                process.stdin = None
                writer = None
                try:
                    journal = self.source.state / "jobs" / (plan["id"] + ".json")
                    deadline = time.monotonic() + 30
                    while time.monotonic() < deadline:
                        if journal.exists() and json.loads(journal.read_text()).get("state_archive_path"):
                            break
                        self.assertIsNone(process.poll(), "cleanup ended before the preservation checkpoint")
                        time.sleep(0.01)
                    else:
                        self.fail("state archive checkpoint did not appear")
                    writer = subprocess.Popen(
                        [str(writer_path), "40"], cwd=self.base,
                        env={**self.source.env, "CLAUDE_CONFIG_DIR": str(self.root)},
                        stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                    )
                    output, errors = process.communicate(timeout=45)
                    self.assertIsNone(writer.poll(), "synthetic writer must remain live through recheck")
                    envelope = json.loads(output)
                    self.assertTrue(envelope["ok"], (envelope, errors))
                    receipt = envelope["data"]
                    self.assertEqual(receipt["status"], "needs_reconciliation")
                    self.assertEqual(receipt["error"]["code"], "writers_active")
                    self.assertEqual(receipt["error"]["step_id"], "quiescence")
                    self.assertTrue(Path(receipt["archive_path"]).is_file())
                    self.assertTrue(Path(receipt["state_archive_path"]).is_file())
                    self.assertEqual((Path(receipt["new_root"]) / "CLAUDE.md").read_bytes(), b"Synthetic instruction only.\n")
                    self.assertTrue((self.root / ".credentials.json").exists())
                    self.assertFalse(logout_marker.exists())
                    steps = [step["id"] for step in receipt["steps"]]
                    self.assertIn("migrate", steps)
                    self.assertNotIn("credentials", steps)
                    self.assertNotIn("logout", steps)
                finally:
                    if writer is not None:
                        writer.terminate()
                        writer.wait(timeout=5)
                    if process.poll() is None:
                        process.kill()
                        process.wait(timeout=5)

    def test_accepted_after_failure_code_persists_across_processes(self) -> None:
        environment_id = self.source.register(self.root)
        plan = self.source.data("plan_archive", environment_id=environment_id, categories=["instructions"])
        # Occupy the exact private archive destination with a directory so the
        # failure lands after the durable accept.
        dest = self.source.state / "archives" / f'{plan["id"]}.age'
        dest.mkdir(parents=True)
        (dest / "x").write_text("block")
        self.source.approve(plan)
        receipt = self.source.data(
            "execute", plan_id=plan["id"], approval=plan["hash"], archive_passphrase=PASSPHRASE,
        )
        self.assertEqual(receipt["status"], "needs_reconciliation")
        code = receipt["error"]["code"]
        self.assertTrue(code)
        # A brand-new process over the same state reports the same structured error.
        queried = self.source.data("job", job_id=plan["id"])
        self.assertEqual(queried["status"], "needs_reconciliation")
        self.assertEqual(queried["error"]["code"], code)
        self.assertEqual(queried["error"]["phase"], "executing")
        self.assertEqual(queried["id"], receipt["id"])

if __name__ == "__main__":
    print("Portable work-preservation journey; synthetic temporary roots only.", file=sys.stderr)
    unittest.main(argv=[sys.argv[0], *[a for a in sys.argv[1:] if a.startswith('-')]])

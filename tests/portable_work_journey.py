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
        dest_id = target.register(dest)
        path = str(out)
        manifest = target.data("archive_inspect", archive_path=path, archive_passphrase=PASSPHRASE)
        self.assertEqual(manifest["generator"], "Lintel")
        self.assertEqual(len(manifest["files"]), 7)
        read = target.data("archive_read", archive_path=path, archive_passphrase=PASSPHRASE, path="CLAUDE.md")
        self.assertEqual(read["text"], "Synthetic instruction only.\n")
        import_plan = target.data(
            "plan_import", environment_id=dest_id, archive_path=path,
            categories=["instructions", "memory", "sessions"], archive_passphrase=PASSPHRASE,
        )
        receipt = target.execute(import_plan)
        self.assertEqual(receipt["status"], "completed")
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

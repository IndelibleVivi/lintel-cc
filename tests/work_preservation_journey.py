#!/usr/bin/env python3
"""Black-box regression journey for independent work preservation.

Origin: external code review dated 2026-10-03 (baseline e4a4dab), findings
F01/F02/F03. These assertions describe the repaired behavior:
  - environment inspection stays available when work files exceed the archive
    admission limits (inspection reads metadata only; destructive plans still
    run the full manifest checks);
  - a second rebuild archives work that a previous rebuild had preserved into
    lintel-imports (no silent omission, no double-wrapped paths);
  - a damaged settings.json blocks settings-writing plans but not independent
    work-preservation plans, and the original bytes stay untouched.

Never points at the operator's home. Build `cargo build -p lintel-runner`
first, then run: python3 tests/work_preservation_journey.py [path-to-lintel]
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
PASSPHRASE = "synthetic-lintel-regression-only"
ALLOWED = {"register", "inspect", "plan_reset", "plan_preserve", "execute", "archive_inspect"}


class WorkPreservationJourney(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="lintel-audit-")
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name).resolve()
        self.home = self.base / "home"
        self.state = self.base / "state"
        self.root = self.home / "synthetic-claude-root"
        self.root.mkdir(parents=True)
        self.environment = {
            "HOME": str(self.home),
            "LINTEL_TEST_HOME": str(self.home),
            "LINTEL_STATE_DIR": str(self.state),
            "PATH": "/usr/bin:/bin",
        }
        self.approved_plans: dict[str, str] = {}

    def write(self, relative: str, content: bytes) -> None:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)

    def envelope(self, command: str, **fields: Any) -> dict[str, Any]:
        self.assertIn(command, ALLOWED)
        if command == "register":
            self.assertTrue(Path(fields["root"]).resolve().is_relative_to(self.base))
        if command == "execute":
            self.assertEqual(self.approved_plans.get(fields["plan_id"]), fields["approval"])
        payload = {"command": command, **fields}
        result = subprocess.run(
            [str(BINARY), "request"],
            input=json.dumps(payload), text=True, capture_output=True,
            cwd=self.base, env=self.environment, timeout=45, check=False,
        )
        try:
            response = json.loads(result.stdout)
        except json.JSONDecodeError as error:
            self.fail(f"Runner did not return JSON: exit={result.returncode}; {error}")
        self.assertIsInstance(response, dict)
        self.assertIn("ok", response)
        if response["ok"]:
            self.assertEqual(result.returncode, 0)
        return response

    def data(self, command: str, **fields: Any) -> Any:
        response = self.envelope(command, **fields)
        self.assertTrue(response["ok"], json.dumps(response.get("error"), ensure_ascii=False))
        return response["data"]

    def register(self) -> str:
        environment = self.data("register", name="synthetic audit fixture", root=str(self.root))
        self.assertEqual(Path(environment["root"]), self.root)
        return str(environment["id"])

    def rebuild(self, environment_id: str) -> dict[str, Any]:
        plan = self.data(
            "plan_reset", environment_id=environment_id, recipe="rebuild",
            categories=["instructions", "memory", "sessions"],
        )
        self.assertEqual(plan["environment_id"], environment_id)
        self.approved_plans[plan["id"]] = plan["hash"]
        receipt = self.data(
            "execute", plan_id=plan["id"], approval=plan["hash"],
            archive_passphrase=PASSPHRASE,
        )
        self.assertIn(receipt["status"], ("completed", "partially_completed"))
        self.assertTrue(Path(receipt["new_root"]).resolve().is_relative_to(self.base))
        return receipt

    def preserve(self, environment_id: str, instructions_active: bool) -> dict[str, Any]:
        plan = self.data(
            "plan_preserve", environment_id=environment_id,
            categories=["instructions", "memory", "sessions"],
            activate={"instructions": instructions_active},
        )
        self.assertEqual(plan["environment_id"], environment_id)
        self.approved_plans[plan["id"]] = plan["hash"]
        receipt = self.data(
            "execute", plan_id=plan["id"], approval=plan["hash"],
            archive_passphrase=PASSPHRASE,
        )
        self.assertIn(receipt["status"], ("completed", "partially_completed"))
        self.assertTrue(Path(receipt["new_root"]).resolve().is_relative_to(self.base))
        return receipt

    def test_large_session_does_not_disable_independent_settings_inspection(self) -> None:
        self.write("settings.json", b'{"env":{"DISABLE_ERROR_REPORTING":"1"}}')
        self.write("projects/demo/session.jsonl", b'{"payload":"' + b"x" * (9 * 1024 * 1024) + b'"}\n')
        environment_id = self.register()
        response = self.envelope("inspect", environment_id=environment_id)
        self.assertTrue(
            response["ok"],
            "A large session must not make independent environment/settings inspection fail: "
            + json.dumps(response.get("error"), ensure_ascii=False),
        )
        self.assertTrue(response["data"]["settings"])
        self.assertEqual(response["data"]["environment"]["id"], environment_id)
        # Asset completeness is reported separately; this does not relax the
        # archive's own admission rules.

    def test_two_rebuilds_preserve_previously_imported_work_in_next_archive(self) -> None:
        self.write("settings.json", b"{}")
        instruction = b"Synthetic instructions only.\n"
        memory = b"Synthetic memory only.\n"
        session = b'{"type":"synthetic","text":"preserve me"}\n'
        self.write("CLAUDE.md", instruction)
        self.write("projects/demo/memory/MEMORY.md", memory)
        self.write("projects/demo/session.jsonl", session)
        original = self.register()
        first = self.rebuild(original)
        second = self.rebuild(first["new_environment_id"])
        archive = self.data(
            "archive_inspect", job_id=second["id"], archive_passphrase=PASSPHRASE,
        )
        digests = {item["digest"] for item in archive["files"]}
        expected = {hashlib.sha256(content).hexdigest() for content in (instruction, memory, session)}
        self.assertTrue(
            expected.issubset(digests),
            "A→B→C must retain work already imported into B; an on-disk copy left behind in B "
            "does not establish completeness of C's archive.",
        )
        self.assertFalse(
            (Path(second["new_root"]) / "lintel-imports" / "lintel-imports").exists(),
            "imported paths must not be wrapped a second time",
        )
        self.assertEqual((self.root / "projects/demo/memory/MEMORY.md").read_bytes(), memory)
        self.assertEqual((self.root / "projects/demo/session.jsonl").read_bytes(), session)

    def test_reference_instructions_survive_repeated_preservation(self) -> None:
        # When instructions are kept as reference (activate.instructions false),
        # the first preserve writes CLAUDE.md into lintel-imports. A later
        # preserve must still archive that reference file, and repeated imports
        # must never wrap it a second time or silently drop it.
        self.write("settings.json", b"{}")
        instruction = b"Synthetic reference instructions only.\n"
        memory = b"Synthetic memory only.\n"
        session = b'{"type":"synthetic","text":"reference run"}\n'
        self.write("CLAUDE.md", instruction)
        self.write("projects/demo/memory/MEMORY.md", memory)
        self.write("projects/demo/session.jsonl", session)
        original = self.register()
        first = self.preserve(original, instructions_active=False)
        first_root = Path(first["new_root"])
        self.assertEqual(
            (first_root / "lintel-imports" / "CLAUDE.md").read_bytes(), instruction,
        )
        self.assertFalse((first_root / "CLAUDE.md").exists())
        second = self.preserve(first["new_environment_id"], instructions_active=False)
        second_root = Path(second["new_root"])
        self.assertEqual(
            (second_root / "lintel-imports" / "CLAUDE.md").read_bytes(), instruction,
            "reference instructions must survive a second preserve without data loss",
        )
        self.assertFalse((second_root / "CLAUDE.md").exists())
        self.assertFalse(
            (second_root / "lintel-imports" / "lintel-imports").exists(),
            "reference instruction paths must not be wrapped a second time",
        )
        archive = self.data(
            "archive_inspect", job_id=second["id"], archive_passphrase=PASSPHRASE,
        )
        digests = {item["digest"] for item in archive["files"]}
        expected = {hashlib.sha256(content).hexdigest() for content in (instruction, memory, session)}
        self.assertTrue(
            expected.issubset(digests),
            "A→B→C with reference instructions must retain every selected source in C's archive.",
        )

    def test_active_and_reference_instructions_coexist_without_data_loss(self) -> None:
        # An active root CLAUDE.md and a previously imported reference CLAUDE.md
        # coexist. Preserving with references inactive must retain both under
        # distinct generated names instead of overwriting either.
        self.write("settings.json", b"{}")
        active = b"Synthetic active instructions only.\n"
        reference = b"Synthetic reference instructions only.\n"
        self.write("CLAUDE.md", active)
        self.write("lintel-imports/CLAUDE.md", reference)
        original = self.register()
        receipt = self.preserve(original, instructions_active=False)
        root = Path(receipt["new_root"])
        names = sorted(
            entry.name
            for entry in (root / "lintel-imports").iterdir()
            if entry.is_file()
        )
        self.assertEqual(names, ["CLAUDE.md", "lintel-1-CLAUDE.md"])
        # Sorted manifest order puts the active root file first, so it takes the
        # unsuffixed reference slot and the prior reference is disambiguated.
        self.assertEqual((root / "lintel-imports" / "CLAUDE.md").read_bytes(), active)
        self.assertEqual(
            (root / "lintel-imports" / "lintel-1-CLAUDE.md").read_bytes(), reference,
        )
        archive = self.data(
            "archive_inspect", job_id=receipt["id"], archive_passphrase=PASSPHRASE,
        )
        digests = {item["digest"] for item in archive["files"]}
        expected = {hashlib.sha256(content).hexdigest() for content in (active, reference)}
        self.assertTrue(expected.issubset(digests))
        # The generated collision name must survive the next actual preservation
        # and archive, together with another newly active instruction in B.
        fresh = b"Synthetic newly active B instruction.\n"
        (root / "CLAUDE.md").write_bytes(fresh)
        again = self.preserve(receipt["new_environment_id"], instructions_active=False)
        target = Path(again["new_root"])
        self.assertFalse((target / "CLAUDE.md").exists())
        self.assertFalse((target / "lintel-imports/lintel-imports").exists())
        archive = self.data("archive_inspect", job_id=again["id"], archive_passphrase=PASSPHRASE)
        instructions = [entry for entry in archive["files"] if entry["category"] == "instructions"]
        self.assertEqual(len(instructions), 3)
        self.assertEqual({entry["digest"] for entry in instructions},
                         {hashlib.sha256(content).hexdigest() for content in (active, reference, fresh)})
        self.assertEqual({entry.read_bytes() for entry in (target / "lintel-imports").iterdir()},
                         {active, reference, fresh})

    def test_malformed_settings_does_not_prevent_independent_work_preservation_plan(self) -> None:
        damaged = b'{"env":{"DISABLE_TELEMETRY":"1",}}'
        self.write("settings.json", damaged)
        self.write("CLAUDE.md", b"Preservable even when settings are broken.\n")
        environment_id = self.register()
        response = self.envelope(
            "plan_reset", environment_id=environment_id, recipe="rebuild",
            categories=["instructions"],
        )
        self.assertEqual((self.root / "settings.json").read_bytes(), damaged)
        self.assertTrue(
            response["ok"],
            "A work-preservation plan that does not edit settings must remain available: "
            + json.dumps(response.get("error"), ensure_ascii=False),
        )
        self.assertTrue(response["data"]["archive_passphrase_required"])


if __name__ == "__main__":
    print("Work-preservation regression journey; synthetic temporary roots only.", file=sys.stderr)
    unittest.main(argv=[sys.argv[0], *[a for a in sys.argv[1:] if a.startswith('-')]])

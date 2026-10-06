#!/usr/bin/env python3
"""Synthetic journey for the read-only work-capacity preflight.

Covers the delivered capacity-preflight first order: `lintel work preflight`
and the canonical `work_preflight` core operation.

It asserts the named CLI argv boundary and the operations strict schema (not a
helper), that preflight is metadata-only (a large sparse file is reported by
length without being read), that per-category and selected totals are correct,
that oversized files / over-limit totals / over-limit file counts / symbolic
links / unreadable roots are reported as blockers or explicit incompleteness
rather than an empty directory, that a malformed strict request is refused, and
that a normal archive still works while an oversized archive is still rejected
with the original bytes unchanged.

Metadata-only synthetic temporary roots; never the operator's home, credentials,
account or network. Build `cargo build -p lintel-runner` first, then run:
  python3 tests/work_capacity_journey.py [path-to-lintel]
"""
from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 and not sys.argv[1].startswith('-') else ROOT / 'target/debug/lintel'
PASSPHRASE = "synthetic-capacity-preflight-passphrase"
FILE_LIMIT = 8 * 1024 * 1024
TOTAL_LIMIT = 32 * 1024 * 1024
ENTRY_LIMIT = 50000


class WorkCapacityJourney(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="lintel-capacity-")
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

    # -- fixtures -------------------------------------------------------------
    def write(self, relative: str, content: bytes) -> Path:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        return path

    def sparse(self, relative: str, size: int) -> Path:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "wb") as handle:
            handle.truncate(size)
        return path

    # -- CLI boundary ---------------------------------------------------------
    def invoke(self, argv: list[str]) -> tuple[int, Any, str]:
        proc = subprocess.run(
            [str(BINARY), *argv], cwd=self.base, env=self.environment,
            capture_output=True, text=True, timeout=45,
        )
        try:
            response = json.loads(proc.stdout)
        except json.JSONDecodeError:
            self.fail(f"runner did not return JSON: argv={argv}; stdout={proc.stdout[:400]!r}; stderr={proc.stderr[:400]!r}")
        return proc.returncode, response, proc.stderr

    def named(self, argv: list[str], good: bool = True) -> Any:
        code, response, stderr = self.invoke(argv)
        self.assertEqual(response["ok"], good, json.dumps(response, ensure_ascii=False))
        if good:
            self.assertEqual(code, 0, stderr)
        return response["data"] if good else response["error"]

    def request(self, payload: dict[str, Any], good: bool = True) -> Any:
        proc = subprocess.run(
            [str(BINARY), "request"], input=json.dumps(payload), cwd=self.base,
            env=self.environment, capture_output=True, text=True, timeout=45,
        )
        response = json.loads(proc.stdout)
        self.assertEqual(response["ok"], good, response)
        return response["data"] if good else response["error"]

    def register(self, name: str = "synthetic capacity fixture") -> str:
        environment = self.request({"command": "register", "name": name, "root": str(self.root)})
        return str(environment["id"])

    def preflight(self, environment_id: str, categories: list[str]) -> dict[str, Any]:
        return self.named([
            "work", "preflight",
            "--environment", environment_id,
            "--categories", ",".join(categories),
        ])

    # -- tests ----------------------------------------------------------------
    def test_named_preflight_strict_schema_and_totals(self) -> None:
        self.write("settings.json", b"{}")
        self.write("CLAUDE.md", b"Instruction only.\n")
        self.write("projects/demo/memory/MEMORY.md", b"Memory only.\n")
        self.write("projects/demo/session.jsonl", b'{"synthetic":true}\n')
        environment_id = self.register()

        report = self.preflight(environment_id, ["instructions", "memory", "sessions"])
        self.assertEqual(report["environment_id"], environment_id)
        self.assertEqual(Path(report["root"]).resolve(), self.root.resolve())
        self.assertEqual(report["complete"], True)
        self.assertEqual(report["eligible"], True)
        self.assertEqual(report["limits"], {
            "file_bytes": FILE_LIMIT, "total_bytes": TOTAL_LIMIT,
            "files": 10000, "entries": ENTRY_LIMIT,
        })
        self.assertRegex(report["checked_at"], r"^\d{4}-\d{2}-\d{2}T")
        categories = {entry["category"]: entry for entry in report["categories"]}
        self.assertEqual(categories["instructions"]["count"], 1)
        self.assertEqual(categories["memory"]["count"], 1)
        self.assertEqual(categories["sessions"]["count"], 1)
        self.assertEqual(report["totals"]["files"], 3)
        expected = len(b"Instruction only.\n") + len(b"Memory only.\n") + len(b'{"synthetic":true}\n')
        self.assertEqual(report["totals"]["bytes"], expected)
        self.assertEqual(report["blockers"], [])
        self.assertEqual(report["blockers_truncated"], False)

        subset = self.preflight(environment_id, ["sessions"])
        self.assertEqual(subset["totals"]["files"], 1)
        self.assertEqual(subset["totals"]["bytes"], len(b'{"synthetic":true}\n'))
        self.assertEqual(
            {c["category"]: c["count"] for c in subset["categories"]},
            {"instructions": 1, "memory": 1, "sessions": 1},
        )

    def test_static_schema_and_describe_are_readonly(self) -> None:
        schema = self.named(["schema", "work_preflight"])
        self.assertEqual(schema["additionalProperties"], False)
        self.assertEqual(set(schema["required"]), {"command", "environment_id", "categories"})
        self.assertEqual(schema["properties"]["categories"]["minItems"], 1)
        describe = self.named(["describe", "work_preflight"])
        self.assertEqual(describe["requires_plan"], False)
        self.assertEqual(describe["effects"]["target"], "read_selected_work_metadata_only")
        self.assertEqual(describe["secret_fields"], [])

    def test_oversized_sparse_file_is_reported_without_reading_body(self) -> None:
        self.write("settings.json", b"{}")
        self.write("CLAUDE.md", b"Ok instruction.\n")
        big = self.sparse("projects/demo/session.jsonl", 9 * 1024 * 1024)
        environment_id = self.register()
        report = self.preflight(environment_id, ["instructions", "sessions"])
        self.assertEqual(report["complete"], True)
        self.assertEqual(report["eligible"], False)
        oversized = next(
            (b for b in report["blockers"]
             if b["code"] == "file_too_large" and b["path"] == "projects/demo/session.jsonl"),
            None,
        )
        self.assertIsNotNone(oversized, report["blockers"])
        # The message must not imply per-file selection exists this round.
        self.assertIn("类别", oversized["message"])
        self.assertNotIn("按有限文件集合单独处理", oversized["message"])
        self.assertEqual(big.stat().st_size, 9 * 1024 * 1024)
        self.assertEqual(report["totals"]["bytes"], len(b"Ok instruction.\n") + 9 * 1024 * 1024)

    def test_oversized_total_is_a_blocker(self) -> None:
        self.write("settings.json", b"{}")
        for index in range(5):
            self.sparse(f"projects/demo/session-{index}.jsonl", 7 * 1024 * 1024)
        environment_id = self.register()
        report = self.preflight(environment_id, ["sessions"])
        self.assertEqual(report["complete"], True)
        self.assertEqual(report["eligible"], False)
        self.assertTrue(any(b["code"] == "total_bytes_exceeded" for b in report["blockers"]), report["blockers"])

    def test_file_count_over_limit_is_a_blocker(self) -> None:
        self.write("settings.json", b"{}")
        # 10001 tiny session files exceed the 10000-file limit while staying far
        # under the byte limits; the count must block, not silently pass.
        directory = self.root / "projects/demo/many"
        directory.mkdir(parents=True)
        for index in range(10001):
            (directory / f"s{index}.jsonl").write_bytes(b"{}\n")
        environment_id = self.register()
        report = self.preflight(environment_id, ["sessions"])
        self.assertEqual(report["complete"], True)
        self.assertEqual(report["eligible"], False)
        self.assertEqual(report["totals"]["files"], 10001)
        self.assertTrue(any(b["code"] == "file_count_exceeded" for b in report["blockers"]), report["blockers"])

    def test_symbolic_link_is_not_followed_and_marks_incomplete(self) -> None:
        self.write("settings.json", b"{}")
        self.write("projects/demo/real.jsonl", b'{"synthetic":true}\n')
        os.symlink("/etc/hosts", self.root / "projects/demo/link.jsonl")
        environment_id = self.register()
        report = self.preflight(environment_id, ["sessions"])
        self.assertEqual(report["complete"], False)
        self.assertEqual(report["eligible"], False)
        incomplete = next((b for b in report["blockers"] if b["code"] == "scan_incomplete"), None)
        self.assertIsNotNone(incomplete, report["blockers"])
        # The incomplete blocker retains a bounded meaningful relative path for
        # the first unobserved entry, not just a reason.
        self.assertEqual(incomplete["path"], "projects/demo/link.jsonl")
        self.assertEqual(report["totals"]["files"], 1)

    def test_fifo_is_not_opened_or_admitted(self) -> None:
        self.write("settings.json", b"{}")
        self.write("projects/demo/real.jsonl", b'{"synthetic":true}\n')
        os.mkfifo(self.root / "projects/demo/session.jsonl", 0o600)
        environment_id = self.register()
        # The preflight must return promptly (never open/block on the FIFO) and
        # must not admit the FIFO as a countable session file.
        report = self.preflight(environment_id, ["sessions"])
        self.assertEqual(report["complete"], False)
        self.assertEqual(report["eligible"], False)
        nonregular = next((b for b in report["blockers"] if b["code"] == "nonregular_entry"), None)
        self.assertIsNotNone(nonregular, report["blockers"])
        self.assertEqual(nonregular["path"], "projects/demo/session.jsonl")
        self.assertEqual(report["totals"]["files"], 1)
        self.assertEqual(report["totals"]["bytes"], len(b'{"synthetic":true}\n'))
        # The incomplete scan keeps the FIFO as its first unobserved path.
        incomplete = next((b for b in report["blockers"] if b["code"] == "scan_incomplete"), None)
        self.assertIsNotNone(incomplete, report["blockers"])
        self.assertEqual(incomplete["path"], "projects/demo/session.jsonl")

    def test_fifo_in_unselected_category_marks_scope_incomplete(self) -> None:
        self.write("settings.json", b"{}")
        self.write("CLAUDE.md", b"Reference instruction.\n")
        (self.root / "projects/demo").mkdir(parents=True)
        os.mkfifo(self.root / "projects/demo/session.jsonl", 0o600)
        environment_id = self.register()
        # Selecting only instructions keeps the FIFO (sessions) out of this
        # selection's blockers; the scan is still incomplete for coverage.
        report = self.preflight(environment_id, ["instructions"])
        self.assertFalse(any(b["code"] == "nonregular_entry" for b in report["blockers"]), report["blockers"])
        self.assertEqual(report["complete"], False)
        self.assertEqual(report["eligible"], False)
        self.assertEqual(report["totals"]["files"], 1)

    def test_unreadable_root_is_incomplete_not_empty(self) -> None:
        self.write("settings.json", b"{}")
        self.write("projects/demo/session.jsonl", b'{"synthetic":true}\n')
        environment_id = self.register()
        shutil.rmtree(self.root)
        report = self.preflight(environment_id, ["sessions"])
        self.assertEqual(report["complete"], False)
        self.assertEqual(report["eligible"], False)
        self.assertEqual(report["totals"]["files"], 0)
        incomplete = next((b for b in report["blockers"] if b["code"] == "scan_incomplete"), None)
        self.assertIsNotNone(incomplete, report["blockers"])
        # Root-level failure uses `.` as the bounded relative path.
        self.assertEqual(incomplete["path"], ".")

    def test_malformed_strict_requests_are_refused(self) -> None:
        environment_id = self.register()
        cases = [
            ["work", "preflight", "--environment", environment_id],
            ["work", "preflight", "--categories", "memory"],
            ["work", "preflight", "--environment", "synthetic", "--categories", "memory"],
            ["work", "preflight", "--environment", environment_id, "--categories", "bogus"],
            ["work", "preflight", "--environment", environment_id, "--categories", ""],
            ["work", "preflight", "--environment", environment_id, "--categories", "memory", "--approval", "x"],
            ["work", "preflight", "--environment", environment_id, "--categories", "memory", "--unknown", "1"],
        ]
        for argv in cases:
            error = self.named(argv, good=False)
            self.assertEqual(error["code"], "invalid_request", argv)
        # The raw protocol path shares the same core: an empty selection is
        # refused there too, never read as a trivially passing scan.
        error = self.request({"command": "work_preflight", "environment_id": environment_id, "categories": []}, good=False)
        self.assertIn(error["code"], ("invalid_request", "invalid_categories"))

    def test_preflight_never_writes_state_or_plans(self) -> None:
        self.write("settings.json", b"{}")
        self.write("CLAUDE.md", b"Untouched instruction.\n")
        environment_id = self.register()
        before = (self.root / "CLAUDE.md").read_bytes()
        plans_dir = self.state / "plans"
        plans_before = sorted(p.name for p in plans_dir.iterdir()) if plans_dir.exists() else []
        jobs_dir = self.state / "jobs"
        jobs_before = sorted(p.name for p in jobs_dir.iterdir()) if jobs_dir.exists() else []
        self.preflight(environment_id, ["instructions", "memory", "sessions"])
        self.assertEqual((self.root / "CLAUDE.md").read_bytes(), before)
        plans_after = sorted(p.name for p in plans_dir.iterdir()) if plans_dir.exists() else []
        jobs_after = sorted(p.name for p in jobs_dir.iterdir()) if jobs_dir.exists() else []
        self.assertEqual(plans_after, plans_before, "preflight must not persist a plan")
        self.assertEqual(jobs_after, jobs_before, "preflight must not persist a receipt")

    def test_normal_archive_works_and_oversized_archive_rejected(self) -> None:
        self.write("settings.json", b"{}")
        instruction = b"Synthetic instruction for capacity journey.\n"
        session = b'{"type":"synthetic","text":"capacity"}\n'
        self.write("CLAUDE.md", instruction)
        self.write("projects/demo/session.jsonl", session)
        environment_id = self.register()
        plan = self.request({
            "command": "plan_archive", "environment_id": environment_id,
            "categories": ["instructions", "sessions"],
        })
        receipt = self.request({
            "command": "execute", "plan_id": plan["id"], "approval": plan["hash"],
            "archive_passphrase": PASSPHRASE,
        })
        self.assertIn(receipt["status"], ("completed", "partially_completed"))
        self.assertEqual((self.root / "CLAUDE.md").read_bytes(), instruction)
        self.assertEqual((self.root / "projects/demo/session.jsonl").read_bytes(), session)

        self.sparse("projects/demo/big.jsonl", 9 * 1024 * 1024)
        error = self.request({
            "command": "plan_archive", "environment_id": environment_id,
            "categories": ["sessions"],
        }, good=False)
        self.assertEqual(error["code"], "file_limit", error)
        self.assertEqual((self.root / "CLAUDE.md").read_bytes(), instruction)


if __name__ == "__main__":
    unittest.main(verbosity=2)

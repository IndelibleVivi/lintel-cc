#!/usr/bin/env python3
"""Long-path frozen-plan admission; real synthetic files, no account state."""
from __future__ import annotations
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
RUNNER = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / "target/debug/lintel"


class PlanCapacity(unittest.TestCase):
    def test_long_manifest_cannot_publish_an_unreadable_plan(self):
        with tempfile.TemporaryDirectory(prefix="lintel-plan-budget-") as temporary:
            base = Path(temporary).resolve(); home = base / "home"; root = home / "root"; state = base / "state"
            root.mkdir(parents=True)
            deep = root / "projects" / "synthetic"
            for number in range(6): deep /= str(number) + "x" * 139
            deep.mkdir(parents=True)
            for number in range(10000): (deep / f"{number:05}.jsonl").touch()
            actor = {"HOME": str(home), "LINTEL_TEST_HOME": str(home), "LINTEL_STATE_DIR": str(state), "PATH": "/usr/bin:/bin"}
            def call(command, **fields):
                result = subprocess.run([str(RUNNER), "request"], input=json.dumps({"command": command, **fields}),
                                        text=True, capture_output=True, env=actor, timeout=180)
                return json.loads(result.stdout)
            environment = call("register", root=str(root), name="synthetic long manifest")["data"]["id"]
            preflight = call("work_preflight", environment_id=environment, categories=["sessions"])
            self.assertTrue(preflight["data"]["eligible"], "metadata capacity is a separate guarantee")
            # Archive-only has one small enough manifest; it must stay usable.
            archive = call("plan_archive", environment_id=environment, categories=["sessions"])
            self.assertTrue(archive["ok"], archive)
            self.assertTrue(call("plan_show", plan_id=archive["data"]["id"])["ok"])
            saved = set((state / "plans").glob("*.json"))
            preserve = call("plan_preserve", environment_id=environment, categories=["sessions"])
            self.assertFalse(preserve["ok"], "old behavior publishes a >16 MiB plan that cannot subsequently be read")
            self.assertIn(preserve["error"]["code"], ["state_limit", "plan_capacity", "file_limit"])
            self.assertEqual(set((state / "plans").glob("*.json")), saved, "no unreadable plan left on disk")
            self.assertEqual(list((state / "jobs").glob("*.json")), [], "refused preview never accepts a job")
            self.assertTrue(all(file.stat().st_size <= 16 * 1024 * 1024 for file in saved))
            # The same originals remain preservable with an exact small scope.
            selected = (deep / "00000.jsonl").relative_to(root).as_posix()
            limited = call("plan_preserve", environment_id=environment, categories=["sessions"], selected_paths=[selected])
            self.assertTrue(limited["ok"], limited)
            self.assertTrue(call("plan_show", plan_id=limited["data"]["id"])["ok"])
            self.assertEqual(len(list(deep.glob("*.jsonl"))), 10000)


if __name__ == "__main__": unittest.main(argv=[sys.argv[0]])

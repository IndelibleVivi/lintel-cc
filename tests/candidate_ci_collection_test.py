"""Finite counterexamples for authenticated CI metadata admission, no network."""
import copy
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("candidate_ci_collection", ROOT / "scripts/collect-candidate-ci.py")
collector = importlib.util.module_from_spec(spec)
spec.loader.exec_module(collector)
SHA = "a" * 40


class CandidateCiCollectionTests(unittest.TestCase):
    def test_only_unambiguous_exact_head_push_is_selected(self):
        run = {"id": 7, "head_sha": SHA, "event": "push", "path": ".github/workflows/verify.yml"}
        for mutation in [{"head_sha": "b" * 40}, {"event": "pull_request"}, {"path": ".github/workflows/other.yml"}]:
            self.assertIsNone(collector.select_run([{**run, **mutation}], SHA))
        self.assertEqual(collector.select_run([run], SHA), run)
        with self.assertRaisesRegex(ValueError, "ambiguous"):
            collector.select_run([run, {**run, "id": 8}], SHA)

    def test_group_success_requires_every_exact_native_matrix_sibling(self):
        groups = collector.expected_groups(SHA)
        jobs = [{"name": name, "head_sha": SHA, "run_id": 7, "status": "completed", "conclusion": "success"}
                for name in ["aggregate", *(name for names in groups.values() for name in names)]]
        self.assertEqual(set(collector.group_results(jobs, SHA, 7)), set(groups))
        for mutation in ["missing", "duplicate", "other_head", "other_run", "skipped", "cancelled", "failure"]:
            altered = copy.deepcopy(jobs)
            if mutation == "missing": altered.pop()
            elif mutation == "duplicate": altered.append(altered[-1])
            elif mutation == "other_head": altered[-1]["head_sha"] = "b" * 40
            elif mutation == "other_run": altered[-1]["run_id"] = 8
            else: altered[-1]["conclusion"] = mutation
            with self.assertRaises(ValueError, msg=mutation):
                collector.group_results(altered, SHA, 7)


if __name__ == "__main__":
    unittest.main()

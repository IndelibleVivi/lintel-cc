#!/usr/bin/env python3
"""Final-package consumer checks; only finite synthetic archives, no execution."""
import copy
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("final_cli_acceptance", ROOT / "scripts/accept-cli-candidate.py")
acceptance = importlib.util.module_from_spec(spec)
spec.loader.exec_module(acceptance)


class CliCandidateAcceptanceTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="lintel-final-package-test-")
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name).resolve()
        self.archive = self.base / "candidate.tar.gz"
        self.payload = {"bin/lintel": b"synthetic, never executed", "project-licenses/LICENSE": b"synthetic terms"}
        runners = []
        for triple in list(acceptance.gate.TARGETS.values())[1:]:
            content = b"synthetic " + triple.encode()
            self.payload["bin/remote-runners/" + triple + "/lintel"] = content
            runners.append({"target": triple, "version": "0.2.0-preview.1", "protocol": 1,
                            "bytes": len(content), "sha256": hashlib.sha256(content).hexdigest()})
        self.payload["bin/remote-runners/manifest.json"] = json.dumps({"runners": runners}).encode()
        self.manifest = {"schema": "lintel.cli-candidate/1", "target": "linux-x86_64", "platform": "linux",
                         "version": "0.2.0-preview.1", "source_revision": "a" * 40, "protocol": 1, "catalog_version": 1,
                         "notices_included": True,
                         "files": [{"path": name, "bytes": len(content), "sha256": hashlib.sha256(content).hexdigest()}
                                   for name, content in self.payload.items()]}

    def write(self):
        payload = {**self.payload, "candidate.json": json.dumps(self.manifest).encode(), "README.txt": b"Synthetic only"}
        payload["SHA256SUMS"] = "".join(f"{hashlib.sha256(content).hexdigest()}  {name}\n" for name, content in sorted(payload.items())).encode()
        with tarfile.open(self.archive, "w:gz") as archive:
            for name, content in payload.items():
                item = tarfile.TarInfo(name)
                item.size = len(content)
                item.mode = 0o755 if name.endswith("/lintel") else 0o644
                archive.addfile(item, io.BytesIO(content))
        return acceptance.gate.digest(self.archive)

    def test_exact_archive_is_extracted_with_executable_mode_and_matching_inventory(self):
        expected = self.write()
        output = self.base / "new-install"
        output.mkdir(mode=0o700)
        manifest = acceptance.inspect_package(self.archive, expected, "linux-x86_64", output)
        self.assertEqual(manifest, self.manifest)
        self.assertEqual((output / "bin/lintel").stat().st_mode & 0o777, 0o755)
        self.assertEqual((output / "bin/lintel").read_bytes(), self.payload["bin/lintel"])

    def test_expected_final_digest_rejects_repacked_archive_before_extraction(self):
        expected = self.write()
        self.manifest["version"] = "wrong"
        self.write()
        output = self.base / "new-install"
        output.mkdir()
        with self.assertRaisesRegex(ValueError, "changed before extraction"):
            acceptance.inspect_package(self.archive, expected, "linux-x86_64", output)
        self.assertEqual(list(output.iterdir()), [])

    def test_self_consistent_envelope_cannot_hide_wrong_target_or_file_declaration(self):
        original = copy.deepcopy(self.manifest)
        for mutation in ["target", "duplicate", "unlisted", "hash"]:
            self.manifest = copy.deepcopy(original)
            if mutation == "target": self.manifest["target"] = "linux-aarch64"
            elif mutation == "duplicate": self.manifest["files"].append(self.manifest["files"][0])
            elif mutation == "unlisted": self.manifest["files"].pop()
            else: self.manifest["files"][0]["sha256"] = "0" * 64
            expected = self.write()
            with self.assertRaises(ValueError):
                acceptance.inspect_package(self.archive, expected, "linux-x86_64")

    def test_runner_manifest_does_not_accept_duplicate_targets_or_changed_bytes(self):
        name = "bin/remote-runners/manifest.json"
        original = json.loads(self.payload[name])
        for mutation in ["duplicate", "hash", "version"]:
            manifest = copy.deepcopy(original)
            if mutation == "duplicate": manifest["runners"][1] = manifest["runners"][0]
            elif mutation == "hash": manifest["runners"][0]["sha256"] = "0" * 64
            else: manifest["runners"][0]["version"] = "0.1.0"
            self.payload[name] = json.dumps(manifest).encode()
            entry = next(item for item in self.manifest["files"] if item["path"] == name)
            entry.update(bytes=len(self.payload[name]), sha256=hashlib.sha256(self.payload[name]).hexdigest())
            expected = self.write()
            with self.assertRaisesRegex(ValueError, "runner"):
                acceptance.inspect_package(self.archive, expected, "linux-x86_64")

    def test_acceptance_refuses_cross_architecture_and_never_inherits_authentication(self):
        with mock.patch.object(acceptance, "native_target", return_value="linux-aarch64"):
            with self.assertRaisesRegex(ValueError, "native operating system"):
                acceptance.accept(self.base / "missing-receipt", "a" * 40, "linux-x86_64")
        with mock.patch.dict("os.environ", {"ANTHROPIC_API_KEY": "synthetic-must-not-inherit", "CLAUDE_CONFIG_DIR": "/not-fixture"}):
            env = acceptance.actor(self.base, "isolated-home")
        self.assertNotIn("ANTHROPIC_API_KEY", env)
        self.assertNotIn("CLAUDE_CONFIG_DIR", env)
        self.assertEqual(env["HOME"], env["LINTEL_TEST_HOME"])
        self.assertEqual(env["PATH"], "/usr/bin:/bin:/usr/sbin:/sbin")


if __name__ == "__main__":
    unittest.main()

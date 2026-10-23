#!/usr/bin/env python3
"""Finite synthetic gate tests; no native builds, accounts or network."""
import copy
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("lintel_candidate", ROOT / "scripts/prepare-candidate.py")
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)
SHA = "a" * 40


def info(data):
    return {"sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data)}


class CandidateGateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="lintel-candidate-test-")
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        ci = gate.ci_module()
        plan = ci.build_plan("workflow_dispatch", base=None, head=SHA, checkout_head=SHA)
        self.plan = plan.to_dict()
        self.results = {"plan": {"result": "success"}}
        self.docs = {}
        for job in plan.jobs:
            self.results[job.id] = {"result": "success" if job.checks else "skipped"}
            for platform in job.platforms:
                for filename, checks in ci.canonical_evidence(job.id, platform, job.checks):
                    self.docs[filename] = {"schema": "lintel.verify/v1", "git": {"head": SHA, "dirty": False},
                        "platform": {"system": ci.PLATFORM_SYSTEM[platform]},
                        "results": [{"id": check, "status": "passed"} for check in checks]}
        names = {
            "runners": ["manifest.json", "x86_64-unknown-linux-musl/lintel", "aarch64-unknown-linux-musl/lintel"],
            "cli_runners": ["manifest.json", "x86_64-unknown-linux-musl/lintel", "aarch64-unknown-linux-musl/lintel"],
            "host": ["manifest.json", "lintel-browser-host"],
            "extensions": ["manifest.json", "chromium/manifest.json", "firefox/manifest.json"],
            "notices": ["inventory.json", "README.md", "frontend.md", "rust.md", "rust-standard-library.html", "rust-native-runtime.md", "licenses/MIT"],
            "terms": gate.TERMS,
        }
        self.components = {name: {"source_revision": SHA, "files": {file: info((name + "/" + file).encode()) for file in files}} for name, files in names.items()}
        runner_json = json.dumps({"runners":[{"target":triple,"version":"0.1.0"} for triple in list(gate.TARGETS.values())[1:]]}).encode()
        for component in ["runners", "cli_runners"]:
            self.components[component]["version"] = "0.1.0" if component == "cli_runners" else "0.1.1"
            self.components[component]["files"]["manifest.json"] = info(runner_json if component == "cli_runners" else runner_json.replace(b"0.1.0", b"0.1.1"))
        self.payload = {"bin/lintel": b"synthetic current binary", "candidate.json": b"{}", "README.txt": b"synthetic", "SHA256SUMS": b"synthetic sums"}
        for component, prefix in {"cli_runners": "bin/remote-runners", "notices": "third-party-notices", "terms": "project-licenses"}.items():
            for name in names[component]:
                self.payload[f"{prefix}/{name}"] = (component + "/" + name).encode()
        self.payload["bin/remote-runners/manifest.json"] = runner_json
        self.payload["candidate.json"] = json.dumps({"schema":"lintel.cli-candidate/1", "source_revision":SHA, "version":"0.1.0", "target":"macos-arm64", "notices_included":True}).encode()
        self.archive = self.base / "cli.tar.gz"
        self.write_archive()
        self.receipt = {"schema": gate.SCHEMA, "source_revision": SHA, "cli_version": "0.1.0", "app_version":"0.1.1", "origin_authenticated": False,
            "build_kind": "fixed-canonical-fresh-clone", "tools": {"rustc": "synthetic"}, "commands": [{"argv": ["cargo", "build", "--locked"], "cwd": "."}],
            "inputs": {"Cargo.lock": info(b"synthetic lock")}, "components": self.components,
            "artifacts": [{"kind": "cli", "target": "macos-arm64", "path": "cli.tar.gz", "artifact": gate.digest(self.archive), "files": {name: info(data) for name, data in self.payload.items()}}], "limitations": ["synthetic local test"]}

        native_archive, native_manifest, native_binary = self.archive, self.payload["candidate.json"], self.payload["bin/lintel"]
        for target in ["linux-x86_64", "linux-aarch64"]:
            self.archive = self.base / (target + ".tar.gz")
            manifest = json.loads(native_manifest)
            manifest["target"] = target
            self.payload["candidate.json"] = json.dumps(manifest).encode()
            self.payload["bin/lintel"] = ("cli_runners/" + gate.TARGETS[target] + "/lintel").encode()
            self.write_archive()
            artifact = copy.deepcopy(self.receipt["artifacts"][0])
            artifact.update(target=target, path=self.archive.name, artifact=gate.digest(self.archive), files={name:info(data) for name,data in self.payload.items()})
            self.receipt["artifacts"].append(artifact)
        self.archive, self.payload["candidate.json"], self.payload["bin/lintel"] = native_archive, native_manifest, native_binary

    def write_archive(self, extra=None):
        with tarfile.open(self.archive, "w:gz") as stream:
            for name, data in self.payload.items():
                member = tarfile.TarInfo(name)
                member.size = len(data)
                stream.addfile(member, io.BytesIO(data))
            if extra:
                stream.addfile(extra, io.BytesIO(b"x") if extra.size else None)

    def check(self):
        return gate.verify(self.receipt, self.base, self.plan, self.results, self.docs)

    def refresh_outer_hash(self):
        self.receipt["artifacts"][0]["artifact"] = gate.digest(self.archive)

    def test_valid_full_ci_and_actual_archive_remain_unauthenticated(self):
        result = self.check()
        self.assertTrue(result["consistency_passed"])
        for key in ["build_origin_authenticated", "ci_origin_authenticated", "release_authorized"]:
            self.assertFalse(result[key])

    def test_wrong_head_reduced_plan_and_missing_or_skipped_check_rejected(self):
        original_plan = copy.deepcopy(self.plan)
        self.plan["head"] = "b" * 40
        with self.assertRaises(ValueError): self.check()
        self.plan = copy.deepcopy(original_plan)
        self.plan["jobs"][1]["checks"] = []
        with self.assertRaises(ValueError): self.check()
        self.plan = original_plan
        for alteration in ["missing", "skipped", "wrong_head", "dirty"]:
            original = copy.deepcopy(self.docs)
            document = next(v for v in self.docs.values() if v["results"])
            if alteration == "missing": document["results"].pop()
            elif alteration == "skipped": document["results"][0]["status"] = "skipped"
            elif alteration == "wrong_head": document["git"]["head"] = "b" * 40
            else: document["git"]["dirty"] = True
            with self.assertRaises(ValueError): self.check()
            self.docs = original
        self.results["native"] = {"result": "skipped"}
        with self.assertRaises(ValueError): self.check()

    def test_captured_package_manifest_rejects_duplicate_keys(self):
        for name in ["candidate.json", "bin/remote-runners/manifest.json"]:
            original = self.payload[name]
            self.payload[name] = b'{"version":"old","version":"new"}'
            self.write_archive()
            self.refresh_outer_hash()
            self.receipt["artifacts"][0]["files"][name] = info(self.payload[name])
            with self.assertRaisesRegex(ValueError, "duplicate JSON key"): self.check()
            self.payload[name] = original

    def test_old_binary_and_recomputed_checksum_do_not_match_build_bytes(self):
        self.payload["bin/lintel"] = b"synthetic stale binary same version"
        self.payload["SHA256SUMS"] = hashlib.sha256(self.payload["bin/lintel"]).hexdigest().encode()
        self.write_archive()
        self.refresh_outer_hash()
        with self.assertRaisesRegex(ValueError, "extracted CLI bytes"): self.check()

    def test_missing_notices_or_license_rejected_even_with_self_consistent_file_manifest(self):
        for name in ["third-party-notices/rust.md", "project-licenses/LICENSE"]:
            data = self.payload.pop(name)
            self.write_archive()
            self.refresh_outer_hash()
            files = self.receipt["artifacts"][0]["files"]
            old = files.pop(name)
            with self.assertRaises(ValueError): self.check()
            files[name] = old
            self.payload[name] = data

    def test_runner_bytes_must_match_component_build_receipt(self):
        name = "bin/remote-runners/x86_64-unknown-linux-musl/lintel"
        self.payload[name] = b"stale runner"
        self.write_archive()
        self.refresh_outer_hash()
        self.receipt["artifacts"][0]["files"][name] = info(self.payload[name])
        with self.assertRaisesRegex(ValueError, "component bytes"): self.check()

    def test_duplicate_path_traversal_unknown_and_links_are_rejected(self):
        for name, kind in [("bin/lintel", tarfile.REGTYPE), ("../escape", tarfile.REGTYPE), ("link", tarfile.SYMTYPE), ("hard", tarfile.LNKTYPE)]:
            extra = tarfile.TarInfo(name)
            extra.type = kind
            extra.linkname = "bin/lintel" if kind != tarfile.REGTYPE else ""
            self.write_archive(extra)
            self.refresh_outer_hash()
            with self.assertRaises(ValueError): self.check()
        self.payload["unknown"] = b"unreviewed"
        self.write_archive()
        self.refresh_outer_hash()
        with self.assertRaises(ValueError): self.check()

    def test_app_and_cli_runner_version_projections_cannot_be_crossed(self):
        self.components["cli_runners"]["version"] = "0.1.1"
        with self.assertRaisesRegex(ValueError, "projection identity"): self.check()
        self.components["cli_runners"]["version"] = "0.1.0"
        self.payload["bin/remote-runners/manifest.json"] = self.payload["bin/remote-runners/manifest.json"].replace(b"0.1.0", b"0.1.1")
        self.write_archive()
        self.refresh_outer_hash()
        name = "bin/remote-runners/manifest.json"
        self.receipt["artifacts"][0]["files"][name] = info(self.payload[name])
        self.components["cli_runners"]["files"]["manifest.json"] = info(self.payload[name])
        with self.assertRaisesRegex(ValueError, "CLI runner projection version"): self.check()

    def test_unknown_component_source_and_missing_required_component_are_rejected(self):
        self.components["host"]["source_revision"] = "unknown"
        with self.assertRaises(ValueError): self.check()
        self.components["host"]["source_revision"] = SHA
        self.components["runners"]["files"].pop("aarch64-unknown-linux-musl/lintel")
        with self.assertRaises(ValueError): self.check()

    def test_fixed_build_flow_owns_empty_target_and_freezes_component_bytes(self):
        output = self.base / "fresh-build"
        sysroot = self.base / "synthetic-sysroot"
        for name in ["COPYRIGHT-library.html", "licenses/MIT.txt", "licenses/Apache-2.0.txt"]:
            file = sysroot / "share/doc/rust" / name
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_text("synthetic toolchain notice")
        calls = []
        scripts = ["apps/desktop/scripts/prepare-remote-runners.mjs", "apps/desktop/scripts/prepare-browser-host.mjs", "apps/desktop/scripts/prepare-browser-extension.mjs", "scripts/prepare-rust-notices.mjs", "scripts/package-cli.mjs", "apps/desktop/scripts/build-macos-app.mjs"]
        tracked = scripts + ["Cargo.toml", "apps/desktop/src-tauri/Cargo.toml", "extensions/browser/native-host/Cargo.toml", "LICENSE", "LICENSE-DOCUMENTATION", "LICENSING.md", "apps/desktop/package.json", "Cargo.lock", "apps/runner/Cargo.toml"]
        def export(repo, revision, out):
            source = out / "source"
            (source / "apps/desktop").mkdir(parents=True)
            for name in tracked:
                path = source / name
                path.parent.mkdir(parents=True, exist_ok=True)
                if name in scripts:
                    path.write_bytes((ROOT / name).read_bytes())
                    continue
                path.write_text(json.dumps({"version":"0.1.1"}) if name.endswith("package.json") else '[package]\nversion="0.1.0"\n' if name.endswith("Cargo.toml") else "synthetic locked source")
            return source
        def write(directory, name, data=b"synthetic component"):
            path = directory / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        def tool(command, cwd, env=None, capture=False):
            command = [str(item) for item in command]
            calls.append(command)
            source = output / "source"
            resources = source / "apps/desktop/src-tauri"
            if command[:3] == ["git", "rev-parse", "HEAD"]: return SHA
            if command[:2] == ["git", "status"]: return ""
            if command[:2] == ["git", "ls-files"]: return "\0".join(tracked) + "\0"
            if capture:
                if command == ["zig", "version"]: return "0.16.0"
                if command == ["rustc", "--print", "sysroot"]: return str(sysroot)
                if command == ["rustup", "target", "list", "--installed"]: return "\n".join(gate.TARGETS.values())
                return "synthetic tool version"
            self.assertEqual(env["CARGO_TARGET_DIR"], str(source / "target"))
            self.assertNotIn("RUSTFLAGS", env)
            for key, folder in [("ZIG_GLOBAL_CACHE_DIR", "zig-cache-global"),
                                ("ZIG_LOCAL_CACHE_DIR", "zig-cache-local"),
                                ("XDG_CACHE_HOME", "cache"),
                                ("NPM_CONFIG_CACHE", "npm-cache")]:
                self.assertEqual(env[key], str(output / folder))
            if command[0] == "node":
                script = Path(command[1])
                resolved = (cwd / script).resolve() if not script.is_absolute() else script.resolve()
                self.assertTrue(resolved.is_file(), f"node script path does not resolve from actual cwd: {command} cwd={cwd}")
                self.assertIn(resolved.relative_to(source.resolve()).as_posix(), scripts)
            elif command[0] == "cargo":
                self.assertEqual(cwd, source)
                if "--manifest-path" in command:
                    self.assertTrue((cwd / command[command.index("--manifest-path") + 1]).is_file())
            elif command[0] == "npm":
                self.assertEqual(cwd, source / "apps/desktop")
                self.assertTrue((cwd / "package.json").is_file())
            if command[:2] in (["cargo", "build"], ["cargo", "zigbuild"]):
                triples = [command[index+1] for index, value in enumerate(command) if value == "--target"]
                self.assertEqual(triples, [gate.TARGETS["macos-arm64"]] if command[1] == "build" else [gate.TARGETS["linux-x86_64"], gate.TARGETS["linux-aarch64"]])
                for triple in triples:
                    target = source / "target" / triple / "release/lintel"
                    self.assertFalse(target.exists(), "fresh outputs cannot consume old binary")
                    write(source / "target", triple + "/release/lintel", triple.encode())
            elif command[:2] == ["cargo", "fetch"]: pass
            elif command[0] == "npm": pass
            elif command[:2] == ["node", "apps/desktop/scripts/prepare-remote-runners.mjs"]:
                for triple in list(gate.TARGETS.values())[1:]:
                    write(resources / "runner-bundles", triple + "/lintel", (source / "target" / triple / "release/lintel").read_bytes())
                write(resources / "runner-bundles", "manifest.json", json.dumps({"runners":[{"target":triple,"version":"0.1.1"} for triple in list(gate.TARGETS.values())[1:]]}).encode())
            elif command[:2] == ["node", "apps/desktop/scripts/prepare-browser-host.mjs"]:
                for name in ["manifest.json", "lintel-browser-host"]: write(resources / "browser-host-bundle", name)
            elif command[:2] == ["node", "apps/desktop/scripts/prepare-browser-extension.mjs"]:
                for name in ["manifest.json", "chromium/manifest.json", "firefox/manifest.json"]: write(resources / "browser-extension-bundle", name)
            elif command[:2] == ["node", "scripts/prepare-rust-notices.mjs"]:
                for name in self.components["notices"]["files"]: write(resources / "third-party-notices", name)
            elif command[:2] == ["node", str(source / "apps/desktop/scripts/build-macos-app.mjs")]:
                app = output / "app/Lintel.app"
                write(app, "Contents/MacOS/lintel-desktop", b"synthetic freshly built App")
                write(app, "Contents/Info.plist", gate.plistlib.dumps({"CFBundleShortVersionString":"0.1.1"}))
                for folder, prefix in [("runner-bundles", "remote-runners"), ("browser-host-bundle", "browser-host"), ("browser-extension-bundle", "browser-extensions"), ("third-party-notices", "third-party-notices")]:
                    for name in gate.inventory(resources / folder): write(app / "Contents/Resources" / prefix, name, (resources / folder / name).read_bytes())
                for name in gate.TERMS: write(app / "Contents/Resources/project-licenses", name, (source / name).read_bytes())
                write(output / "app", "Lintel.dmg", b"synthetic DMG")
            elif command[:2] == ["node", "scripts/package-cli.mjs"]:
                packages = Path(command[command.index("--out")+1])
                packages.mkdir()
                for target, triple in gate.TARGETS.items():
                    data = {"bin/lintel": (source / "target" / triple / "release/lintel").read_bytes()}
                    for directory, prefix in [(output / "cli-runners", "bin/remote-runners"), (resources / "third-party-notices", "third-party-notices")]:
                        for name in gate.inventory(directory): data[prefix + "/" + name] = (directory / name).read_bytes()
                    data.update({"project-licenses/"+name: (source / name).read_bytes() for name in gate.TERMS})
                    data.update({"candidate.json": json.dumps({"schema":"lintel.cli-candidate/1", "source_revision":SHA,"version":"0.1.0","target":target,"notices_included":True}).encode(), "README.txt":b"synthetic", "SHA256SUMS":b"synthetic"})
                    with tarfile.open(packages / ("synthetic-"+target+".tar.gz"), "w:gz") as tar:
                        for name, body in data.items():
                            member = tarfile.TarInfo(name); member.size = len(body); tar.addfile(member, io.BytesIO(body))
            else: self.fail("unexpected build command " + repr(command))
        with mock.patch.object(gate, "source_export", export), mock.patch.object(gate, "run", tool), mock.patch.object(gate.shutil, "which", return_value="synthetic-existing-tool"), mock.patch.object(gate.sys, "platform", "darwin"), mock.patch.object(gate.platform, "machine", return_value="arm64"), mock.patch.object(gate, "dmg_inventory", side_effect=lambda path, source, app_version, expected_artifact: gate.inventory(output / "app/Lintel.app")):
            receipt = gate.build(ROOT, SHA, output, app=True)
        self.assertEqual(len([c for c in calls if c[:2] == ["cargo","build"]]), 1)
        self.assertEqual(len([c for c in calls if c[:2] == ["cargo","zigbuild"] and "--version" not in c]), 1)
        self.assertEqual(receipt["tools"]["zig"], "0.16.0")
        self.assertIn("cargo_zigbuild", receipt["tools"])
        self.assertEqual(len([c for c in calls if c == ["cargo-zigbuild", "--version"]]), 2)
        self.assertEqual(len([c for c in calls if c == ["zig", "version"]]), 2)
        self.assertEqual(receipt["cli_version"], "0.1.0")
        self.assertEqual(receipt["app_version"], "0.1.1")
        self.assertEqual(receipt, gate.json_read(output / "build-receipt.json"))
        with mock.patch.object(gate, "dmg_inventory", side_effect=lambda path, source, app_version, expected_artifact: gate.inventory(output / "app/Lintel.app")):
            self.assertTrue(gate.verify(receipt, output, self.plan, self.results, self.docs)["consistency_passed"])
        with mock.patch.object(gate, "run", tool), mock.patch.object(gate.sys, "platform", "darwin"), mock.patch.object(gate.platform, "machine", return_value="arm64"):
            with self.assertRaises(ValueError): gate.build(ROOT, SHA, output)

    def test_dmg_subgate_uses_readonly_mount_and_detaches_even_when_verifier_fails(self):
        mount_root = self.base / "managed-mount"
        mount_root.mkdir()
        calls = []
        (self.base / "final.dmg").write_bytes(b"synthetic DMG")
        expected = gate.digest(self.base / "final.dmg")
        def native(command, **kwargs):
            calls.append(command)
            if command[1] == "attach":
                mount = Path(command[command.index("-mountpoint")+1])
                (mount / "Lintel.app").mkdir()
            return type("Result", (), {"returncode":0})()
        with mock.patch.object(gate.sys, "platform", "darwin"), mock.patch.object(gate.tempfile, "mkdtemp", return_value=str(mount_root)), mock.patch.object(gate.subprocess, "run", side_effect=native), mock.patch.object(gate, "run", side_effect=ValueError("synthetic invalid resource seal")):
            with self.assertRaisesRegex(ValueError, "invalid resource seal"):
                gate.dmg_inventory(self.base / "final.dmg", ROOT, "0.1.1", expected)
        self.assertIn("-readonly", calls[0])
        self.assertEqual(calls[-1][:2], ["hdiutil", "detach"])
        self.assertFalse(mount_root.exists())

    def test_missing_tool_wrong_zig_targets_and_rust_docs_fail_before_output_or_clone(self):
        for failure in ["missing", "zig_version", "targets", "rust_docs"]:
            output = self.base / ("never-created-" + failure)
            def tool(command, cwd, env=None, capture=False):
                if command[:3] == ["git", "rev-parse", "HEAD"]: return SHA
                if command[:2] == ["git", "status"]: return ""
                if command == ["zig", "version"]: return "0.15.2" if failure == "zig_version" else "0.16.0"
                if command == ["rustup", "target", "list", "--installed"]: return gate.TARGETS["macos-arm64"] if failure == "targets" else "\n".join(gate.TARGETS.values())
                if command == ["rustc", "--print", "sysroot"]: return str(self.base / "missing-docs-sysroot")
                return "synthetic version"
            def which(name, path=None):
                return None if failure == "missing" and name == "cargo-zigbuild" else "synthetic-existing-tool"
            with mock.patch.object(gate, "run", tool), mock.patch.object(gate.shutil, "which", side_effect=which), mock.patch.object(gate, "source_export") as clone, mock.patch.object(gate.sys, "platform", "darwin"), mock.patch.object(gate.platform, "machine", return_value="arm64"):
                with self.assertRaisesRegex(ValueError, {"missing":"preinstalled tool", "zig_version":"Zig 0.16.0", "targets":"Rust targets missing", "rust_docs":"rust-docs notice texts missing"}[failure]): gate.build(ROOT, SHA, output)
                clone.assert_not_called()
                self.assertFalse(output.exists())

    def test_local_clone_keeps_real_git_metadata_without_fetch_or_shared_objects(self):
        output = self.base / "clone-output"
        output.mkdir()
        with mock.patch.object(gate, "run", return_value="") as run, mock.patch.object(gate, "clean_revision") as clean:
            source = gate.source_export(ROOT, SHA, output)
        clone, checkout = [call.args[0] for call in run.call_args_list]
        self.assertEqual(clone[:5], ["git", "clone", "--no-local", "--no-hardlinks", "--no-checkout"])
        self.assertEqual(clone[-2], ROOT.resolve())
        self.assertIn("core.hooksPath=/dev/null", checkout)
        self.assertEqual(checkout[-2:], ["--detach", SHA])
        clean.assert_called_once_with(source, SHA)

    def test_digest_rejects_replaced_mutated_and_symlinked_file(self):
        path = self.base / "binary"
        path.write_bytes(b"original bytes")
        original_digest = gate.stream_digest
        def replaced(stream):
            result = original_digest(stream)
            replacement = self.base / "replacement"
            replacement.write_bytes(b"original bytes")
            replacement.replace(path)
            return result
        with mock.patch.object(gate, "stream_digest", side_effect=replaced):
            with self.assertRaisesRegex(ValueError, "changed during read"): gate.digest(path)
        def mutated(stream):
            result = original_digest(stream)
            path.write_bytes(b"modified bytes")
            return result
        with mock.patch.object(gate, "stream_digest", side_effect=mutated):
            with self.assertRaisesRegex(ValueError, "changed during read"): gate.digest(path)
        link = self.base / "binary-link"
        link.symlink_to(path)
        with self.assertRaises(OSError): gate.digest(link)

    def test_absent_output_no_replace_and_duplicate_json(self):
        target = self.base / "result.json"
        gate.publish(target, {"first": True})
        with self.assertRaises(FileExistsError): gate.publish(target, {"second": True})
        self.assertEqual(gate.json_read(target), {"first": True})
        target.write_text('{"same":1,"same":2}')
        with self.assertRaises(ValueError): gate.json_read(target)


if __name__ == "__main__":
    unittest.main()

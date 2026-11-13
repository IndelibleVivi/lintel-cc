#!/usr/bin/env python3
"""Execute one exact final native CLI archive in a disposable synthetic home.

Consumes the canonical build receipt, never substitutes/repackages binaries.
An optional digest-pinned previous public archive creates the original state
and ciphertext. No personal settings, network, service or browser mutations.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("candidate_gate", ROOT / "scripts/prepare-candidate.py")
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)
require = gate.require


def native_target():
    return {("Darwin", "arm64"): "macos-arm64", ("Linux", "x86_64"): "linux-x86_64",
            ("Linux", "aarch64"): "linux-aarch64", ("Linux", "arm64"): "linux-aarch64"}.get((platform.system(), platform.machine()))


def inspect_package(archive, expected, target, destination=None):
    metadata = {}
    files = gate.tar_inventory(archive, destination=destination, expected_artifact=expected, metadata=metadata)
    manifest = metadata.get("candidate.json", {})
    require(manifest.get("schema") == "lintel.cli-candidate/1" and manifest.get("target") == target,
            "package target/schema differs from native acceptance")
    require(manifest.get("platform") == ("macos" if target == "macos-arm64" else "linux"), "package platform mismatch")
    require(manifest.get("protocol") == 1 and manifest.get("catalog_version") == 1, "unsupported package protocol/catalog")
    require(isinstance(manifest.get("version"), str), "missing package version")
    require(isinstance(manifest.get("files"), list) and manifest["files"], "missing declared package files")
    declared = {}
    for entry in manifest["files"]:
        require(isinstance(entry, dict) and set(entry) == {"path", "bytes", "sha256"}, "invalid package file metadata")
        name = gate.safe_name(entry["path"])
        require(name not in declared, "duplicate declared package file")
        declared[name] = {key: entry[key] for key in ("bytes", "sha256")}
    require(declared == {name: value for name, value in files.items() if name not in {"candidate.json", "SHA256SUMS", "README.txt"}},
            "package declaration differs from installed files")
    require("bin/lintel" in declared, "package has no CLI")
    runners = metadata.get("bin/remote-runners/manifest.json", {}).get("runners", [])
    require(len(runners) == 2 and {item.get("target") for item in runners} == set(list(gate.TARGETS.values())[1:]), "incomplete runner manifest")
    for item in runners:
        require(item.get("protocol") == 1 and item.get("version") == manifest["version"], "runner identity mismatch")
        require(files.get("bin/remote-runners/" + item["target"] + "/lintel") == {key: item.get(key) for key in ("bytes", "sha256")},
                "runner declaration differs from installed bytes")
    return manifest


def actor(base, name):
    home = base / name
    home.mkdir(mode=0o700)
    return {"PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "HOME": str(home), "CFFIXED_USER_HOME": str(home),
            "LINTEL_TEST_HOME": str(home), "LINTEL_STATE_DIR": str(base / (name + "-state")),
            "LINTEL_BROWSER_STATE": str(base / (name + "-browser")), "TMPDIR": str(base)}


def call(executable, env, *args, request=None):
    result = subprocess.run([str(executable), *args], input=json.dumps(request) + "\n" if request is not None else None,
                            text=True, capture_output=True, timeout=180, env=env, cwd=env["HOME"])
    require(len(result.stdout) <= 4 * 1024**2 and len(result.stderr) <= 64 * 1024, "CLI response exceeded acceptance budget")
    require(result.returncode == 0, f"native CLI {args[0]} exited {result.returncode}")
    response = gate.unique_json(result.stdout)
    require(response.get("ok") is True, f"native CLI {args[0]} refused its synthetic request")
    return response["data"]


def request(executable, env, command, **fields):
    return call(executable, env, "request", request={"command": command, **fields})


def accept(receipt_path, revision, target, previous=None, previous_sha=None):
    require(native_target() == target, "selected package needs its native operating system and architecture")
    receipt = gate.json_read(receipt_path)
    require(receipt.get("schema") == gate.SCHEMA and receipt.get("source_revision") == revision, "build receipt revision mismatch")
    chosen = [item for item in receipt.get("artifacts", []) if item.get("kind") == "cli" and item.get("target") == target]
    require(len(chosen) == 1, "exactly one final native CLI artifact required")
    artifact = chosen[0]
    archive = receipt_path.parent / gate.safe_name(artifact["path"])
    require(archive.resolve().is_relative_to(receipt_path.parent.resolve()), "artifact escapes receipt root")
    current = inspect_package(archive, artifact["artifact"], target)
    require(current.get("source_revision") == revision and current["version"] == receipt["cli_version"], "archive differs from candidate build")
    require(current.get("notices_included") is True, "final candidate requires distribution notices")
    require(bool(previous) == bool(previous_sha), "previous archive and independent SHA256 are required together")
    previous_manifest, previous_digest = None, None
    if previous:
        require(re.fullmatch(r"[a-f0-9]{64}", previous_sha) is not None, "invalid previous archive SHA256")
        previous_digest = gate.digest(previous)
        require(previous_digest["sha256"] == previous_sha, "previous public archive digest mismatch")
        previous_manifest = inspect_package(previous, previous_digest, target)
        require(previous_manifest.get("source_revision") != revision, "upgrade needs a distinct previous source revision")
    checks = []
    with tempfile.TemporaryDirectory(prefix="lintel-native-candidate-") as tmp:
        base = Path(tmp).resolve()
        installed = base / "installed"
        installed.mkdir(mode=0o700)
        inspect_package(archive, artifact["artifact"], target, installed)
        executable = installed / "bin/lintel"
        env = actor(base, "home")
        version = call(executable, env, "version", "--json")
        require(version.get("product") == "Lintel" and version.get("version") == current["version"] and version.get("protocol") == 1
                and version.get("catalog_version") == 1 and version.get("platform") == current["platform"], "installed executable identity mismatch")
        require(not Path(env["LINTEL_STATE_DIR"]).exists(), "version probe unexpectedly initialized state")
        operations = {item["id"] for item in call(executable, env, "capabilities", "--json")["operations"]}
        require({"discover", "plan_archive", "archive_inspect", "plan_import"} <= operations, "installed CLI capability contract incomplete")
        require(call(executable, env, "schema", "plan_archive")["type"] == "object", "installed schema unavailable")
        checks.append("exact_final_archive_native_identity_capabilities_schema")

        old_executable = executable
        previous_installed_digest = None
        if previous:
            old = base / "previous-installation"
            old.mkdir(mode=0o700)
            inspect_package(previous, previous_digest, target, old)
            old_executable = old / "bin/lintel"
            previous_installed_digest = gate.digest(old_executable)
            require(call(old_executable, env, "version", "--json")["version"] == previous_manifest["version"], "previous executable identity mismatch")
        root = Path(env["HOME"]) / "claude-root"
        root.mkdir()
        source_files = {"CLAUDE.md": b"Synthetic candidate instruction.\n", "projects/trial/session.jsonl": b'{"type":"user","message":{"content":"Synthetic candidate session."}}\r\n'}
        for name, content in source_files.items():
            file = root / name
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_bytes(content)
        registered = request(old_executable, env, "register", name="Synthetic candidate", root=str(root))
        plan = request(old_executable, env, "plan_policy", environment_id=registered["id"], preset="reduce", keep_remote_control=False)
        submitted = call(old_executable, env, "job", "submit", "--plan", plan["id"], "--approval", plan["hash"])
        original = call(old_executable, env, "job", "wait", submitted["id"], "--timeout", "30s")
        require(original["status"] == "completed", "previous policy job did not complete")
        passphrase = "synthetic-final-archive-acceptance-only"
        archive_plan = request(old_executable, env, "plan_archive", environment_id=registered["id"], categories=["instructions", "sessions"])
        archived = request(old_executable, env, "execute", plan_id=archive_plan["id"], approval=archive_plan["hash"], archive_passphrase=passphrase)
        require(archived["status"] == "completed", "previous work archive did not complete")
        ciphertext = Path(archived["archive_path"])
        require(ciphertext.resolve().is_relative_to(Path(env["LINTEL_STATE_DIR"]).resolve()), "synthetic archive escaped its state")
        archive_files = {entry.name: gate.digest(entry) for entry in ciphertext.parent.iterdir() if entry.is_file()}

        shown = request(executable, env, "job", job_id=original["id"])
        require(shown["id"] == original["id"] and shown["plan_id"] == plan["id"] and shown["status"] == "completed", "original task identity was not retained")
        retained = request(executable, env, "job", job_id=archived["id"])
        require(retained["archive_path"] == archived["archive_path"] and retained["status"] == "completed", "original archive receipt changed")
        # Repeating an accepted original request is query-only, including after
        # a binary upgrade; it may never create a second task or encrypted copy.
        replay = request(executable, env, "execute", plan_id=archive_plan["id"], approval=archive_plan["hash"], archive_passphrase=passphrase)
        require(replay["id"] == archived["id"] and replay["archive_path"] == archived["archive_path"], "original accepted task was replayed")
        require({entry.name: gate.digest(entry) for entry in ciphertext.parent.iterdir() if entry.is_file()} == archive_files,
                "repeated accepted request changed ciphertext or created another archive")
        checks.append("prior_state_receipts_original_ids_query_only")

        independent = actor(base, "independent-home")
        source = {"archive_path": archived["archive_path"], "archive_passphrase": passphrase}
        inspected = request(executable, independent, "archive_inspect", **source)
        require({item["path"] for item in inspected["files"]} == set(source_files), "old ciphertext selection changed")
        for name, content in source_files.items():
            read = request(executable, independent, "archive_read", path=name, **source)
            require(read["text"].encode() == content and read["truncated"] is False, "old ciphertext bytes changed")
        destination = Path(independent["HOME"]) / "import"
        destination.mkdir()
        environment = request(executable, independent, "register", name="Synthetic independent", root=str(destination))
        import_plan = request(executable, independent, "plan_import", environment_id=environment["id"], categories=["instructions", "sessions"], activate={"instructions": False}, **source)
        imported = request(executable, independent, "execute", plan_id=import_plan["id"], approval=import_plan["hash"], archive_passphrase=passphrase)
        require(imported["status"] == "completed", "current candidate import did not complete")
        for name, content in source_files.items():
            require((destination / "lintel-imports" / name).read_bytes() == content and (root / name).read_bytes() == content, "source/import bytes differ")
        checks.append("prior_ciphertext_independent_state_exact_read_and_import")
        # The old executable and its package remain untouched. Rollback means
        # selecting those preserved bytes, never deleting state to restart.
        if previous:
            require(gate.digest(old_executable) == previous_installed_digest, "previous installation changed")
            checks.append("distinct_public_previous_binary_retained")
    return {"schema": "lintel.final-cli-acceptance/1", "source_revision": revision, "target": target,
            "host": {"system": platform.system(), "machine": platform.machine(), "release": platform.release()},
            "version": current["version"], "artifact": artifact["artifact"], "checks": checks, "passed": True,
            "upgrade_tested": previous is not None,
            "previous": None if not previous else {"version": previous_manifest["version"], "source_revision": previous_manifest.get("source_revision"), "artifact": previous_digest},
            "scope": "native extracted final package; synthetic HOME/state; no source-tree runtime dependencies",
            "limits": ["does not accept production SSH, personal authentication, browser profiles or OS distribution trust",
                       "old binary retained for manual rollback; backward writes by old version are not exercised"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", required=True, type=Path)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--target", required=True, choices=gate.TARGETS)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--previous-archive", type=Path)
    parser.add_argument("--previous-sha256")
    args = parser.parse_args()
    require(not args.out.exists() and not args.out.is_symlink(), "acceptance output must be absent")
    result = accept(args.receipt.absolute(), args.revision, args.target, args.previous_archive, args.previous_sha256)
    gate.publish(args.out, result)
    print(json.dumps(result))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError) as error:
        raise SystemExit(f"native final-package acceptance rejected: {error}")

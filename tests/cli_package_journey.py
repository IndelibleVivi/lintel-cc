#!/usr/bin/env python3
"""Portable CLI candidate packaging, install and versioned-upgrade acceptance.

The single required argument is the *native* Lintel CLI for this host
(`target/debug/lintel` on macOS arm64, `target/x86_64-unknown-linux-musl/release/lintel`
on Linux x86_64). The journey ALWAYS packages that native input, extracts it,
executes it (`version`/`capabilities`/`schema`) and performs a retained-state
upgrade. Other architectures and the canonical runner-bundles are visibly
synthetic fixtures built in a private temporary root, so acceptance does not
depend on any other repository target output. A missing, unreadable or
wrong-architecture native input, or an unsupported host, is a hard failure.

An optional full-real-input smoke runs only when every canonical input already
exists locally; it never replaces the native acceptance. Nothing is a silent
skip. Never points at the operator's home. Run:
python3 tests/cli_package_journey.py <native-lintel>
"""
from __future__ import annotations

import errno
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import struct
import subprocess
import sys
import tarfile
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
NODE = shutil.which("node") or "node"
SCRIPT = ROOT / "scripts/package-cli.mjs"
REVISION = "a239123903b620171877498a8af1da349c701060"
SECOND_REVISION = "b0001234567890abcdef1234567890abcdef1234"


def elf(machine: int, interp: bool = False) -> bytes:
    header = bytearray(64)
    header[0:4] = b"\x7fELF"
    header[4] = 2
    header[5] = 1
    struct.pack_into("<H", header, 18, machine)
    phoff, phentsize, phnum = 64, 56, 1
    struct.pack_into("<Q", header, 32, phoff)
    struct.pack_into("<H", header, 54, phentsize)
    struct.pack_into("<H", header, 56, phnum)
    program = bytearray(56)
    struct.pack_into("<I", program, 0, 3 if interp else 1)
    return bytes(header) + bytes(program) + b"\x90" * 256


def macho_arm64() -> bytes:
    # Visibly synthetic Mach-O header, used only off macOS to exercise format
    # validation. Never executed and never claimed as a real build.
    return struct.pack("<II", 0xFEEDFACF, 0x0100000C) + b"\x00" * 128


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def host_target() -> str | None:
    system, machine = platform.system(), platform.machine()
    if system == "Darwin" and machine == "arm64":
        return "macos-arm64"
    if system == "Linux" and machine == "x86_64":
        return "linux-x86_64"
    if system == "Linux" and machine in ("aarch64", "arm64"):
        return "linux-aarch64"
    return None


class PackageJourney:
    def __init__(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="lintel-package-")
        self.base = Path(self.temp.name).resolve()
        self.host = host_target()
        self.native = self._native_input()
        # Synthetic fixtures for the two non-native targets and the canonical
        # runner-bundles (which must contain both Linux architectures).
        self.fixtures = self.base / "fixtures"
        self.fixtures.mkdir()
        self.macos_cli = self.fixtures / "lintel-macos-arm64"
        self.macos_cli.write_bytes(macho_arm64())
        self.linux_x86 = self.fixtures / "lintel-x86_64"
        self.linux_x86.write_bytes(elf(62))
        self.linux_arm = self.fixtures / "lintel-aarch64"
        self.linux_arm.write_bytes(elf(183))
        self.syn_runners = self.fixtures / "runner-bundles"

    @staticmethod
    def _native_input() -> Path:
        if len(sys.argv) != 2:
            raise SystemExit("usage: cli_package_journey.py <native-lintel>")
        path = Path(sys.argv[1]).resolve()
        if not path.is_file():
            raise SystemExit(f"native CLI input missing: {path}")
        if not os.access(path, os.X_OK):
            raise SystemExit(f"native CLI input is not executable: {path}")
        return path

    def runner_bundles(self, version: str, native_payload: bytes | None = None) -> Path:
        # The canonical runner-bundles must contain x86_64 and aarch64 static
        # ELF. On a Linux x86_64 host the x86_64 runner is the real native bytes
        # (so the native CLI input matches its canonical runner); every other
        # slot is the synthetic fixture.
        x86 = native_payload if (native_payload and self.host == "linux-x86_64") else self.linux_x86.read_bytes()
        arm = native_payload if (native_payload and self.host == "linux-aarch64") else self.linux_arm.read_bytes()
        payloads = {"x86_64-unknown-linux-musl": x86, "aarch64-unknown-linux-musl": arm}
        runners = []
        for triple, payload in payloads.items():
            target = self.syn_runners / triple
            target.mkdir(parents=True, exist_ok=True)
            (target / "lintel").write_bytes(payload)
            runners.append({
                "target": triple,
                "version": version,
                "protocol": 1,
                "bytes": len(payload),
                "sha256": sha256_bytes(payload),
            })
        (self.syn_runners / "manifest.json").write_text(json.dumps({"runners": runners}, indent=2) + "\n")
        return self.syn_runners

    def native_inputs(self, version: str, native_payload: bytes) -> dict[str, object]:
        # Assemble the complete 3-input set for the packager. The native slot
        # gets the real native bytes (executed at runtime); the other two slots
        # are visibly synthetic fixtures. The runner-bundles set is completed so
        # every Linux CLI input matches its canonical runner byte-for-byte.
        runners = self.runner_bundles(version, native_payload if self.host != "macos-arm64" else None)
        inputs = {"macos": self.macos_cli, "x86": self.linux_x86, "arm": self.linux_arm, "runners": runners}
        if self.host == "macos-arm64":
            inputs["macos"] = self.native
        elif self.host == "linux-x86_64":
            inputs["x86"] = self.native
        elif self.host == "linux-aarch64":
            inputs["arm"] = self.native
        return inputs

    def package(self, out: Path, inputs: dict, *, version: str | None = None,
                revision: str = REVISION, x86: Path | None = None) -> subprocess.CompletedProcess[str]:
        argv = [NODE, str(SCRIPT),
                "--macos-arm64", str(inputs["macos"]),
                "--linux-x86_64", str(x86 if x86 is not None else inputs["x86"]),
                "--linux-aarch64", str(inputs["arm"]),
                "--linux-runners", str(inputs["runners"]),
                "--revision", revision,
                "--out", str(out)]
        if version is not None:
            argv += ["--version", version]
        return subprocess.run(argv, text=True, capture_output=True, cwd=ROOT, timeout=180, check=False)

    @staticmethod
    def extract(archive: Path, destination: Path) -> None:
        destination.mkdir(parents=True, exist_ok=True)
        subprocess.run(["tar", "-xzf", str(archive), "-C", str(destination)],
                       check=True, capture_output=True)

    @staticmethod
    def verify_files(root: Path) -> dict:
        manifest = json.loads((root / "candidate.json").read_text())
        assert manifest["signed"] is False and manifest["release"] is False, manifest
        for entry in manifest["files"]:
            payload = (root / entry["path"]).read_bytes()
            assert len(payload) == entry["bytes"], (entry, "size")
            assert sha256_bytes(payload) == entry["sha256"], (entry, "digest")
        return manifest

    @staticmethod
    def check_sums(root: Path, valid: bool = True) -> str:
        tool = shutil.which("shasum")
        args = [tool, "-a", "256", "-c", "SHA256SUMS"] if tool else ["sha256sum", "-c", "SHA256SUMS"]
        proc = subprocess.run(args, cwd=str(root), text=True, capture_output=True, check=False)
        assert (proc.returncode == 0) is valid, (args, proc.stdout, proc.stderr)
        if valid:
            assert "OK" in proc.stdout and "FAILED" not in proc.stdout, proc.stdout
        else:
            assert "FAILED" in proc.stdout, proc.stdout
        return Path(args[0]).name

    @staticmethod
    def run_native(exe: Path, *args: str, env: dict | None = None) -> dict:
        proc = subprocess.run([str(exe), *args], text=True, capture_output=True,
                              timeout=60, check=False, env=env)
        assert proc.returncode == 0, (args, proc.returncode, proc.stderr)
        return json.loads(proc.stdout)

    def cleanup(self) -> None:
        self.temp.cleanup()


def documented_unpack(archive: Path, parent: Path, identity: str) -> subprocess.CompletedProcess[str]:
    # Exactly the documented, non-destructive flow: create only the parent, then
    # require `mkdir "$dest"` to succeed before extracting.
    dest = parent / identity
    script = f'mkdir -p "{parent}"\ndest="{dest}"\nmkdir "$dest" && tar -xzf "{archive}" -C "$dest"'
    return subprocess.run(["bash", "-c", script], text=True, capture_output=True, check=False)


def manifest_snapshot(journey: PackageJourney, inputs: dict, version: str) -> None:
    # The synthetic FIFO blocks the first runner read *after* the manifest was
    # parsed. Change the manifest before releasing bytes, without timing races
    # or a production test hook. The archive must retain the validated snapshot.
    runners = journey.base / "snapshot-runners"
    shutil.copytree(inputs["runners"], runners)
    manifest_path = runners / "manifest.json"
    original = manifest_path.read_bytes()
    fifo = runners / "x86_64-unknown-linux-musl/lintel"
    payload = fifo.read_bytes()
    fifo.unlink()
    os.mkfifo(fifo)
    out = journey.base / "snapshot-out"
    argv = [NODE, str(SCRIPT), "--macos-arm64", str(inputs["macos"]),
            "--linux-x86_64", str(inputs["x86"]), "--linux-aarch64", str(inputs["arm"]),
            "--linux-runners", str(runners), "--revision", REVISION,
            "--version", version, "--out", str(out)]
    proc = subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        deadline = time.monotonic() + 30
        while True:
            try:
                fd = os.open(fifo, os.O_WRONLY | os.O_NONBLOCK)
                break
            except OSError as error:
                if error.errno != errno.ENXIO:  # Reader has not opened the FIFO yet.
                    raise
                assert proc.poll() is None and time.monotonic() < deadline, "packager never reached runner read"
                time.sleep(0.01)
        os.set_blocking(fd, True)
        with os.fdopen(fd, "wb") as writer:
            changed = json.loads(original)
            changed["runners"][0]["sha256"] = "0" * 64
            manifest_path.write_text(json.dumps(changed) + "\n")
            writer.write(payload)
        stdout, stderr = proc.communicate(timeout=180)
        assert proc.returncode == 0, stderr
        summary = json.loads(stdout)
        for record in summary["archives"]:
            with tarfile.open(out / record["archive"]) as archive:
                packed = archive.extractfile("bin/remote-runners/manifest.json").read()
                assert packed == original, "archive mixed regenerated manifest with validated runner bytes"
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()


def main() -> int:
    journey = PackageJourney()
    base = journey.base
    if journey.host is None:
        raise SystemExit(f"unsupported host {platform.system()}/{platform.machine()}; this check runs on macOS arm64 or Linux x86_64")
    # The supplied native input must match this host's packaged target.
    with open(journey.native, "rb") as handle:
        native_bytes = handle.read()
    if journey.host == "macos-arm64":
        if native_bytes[:4] != b"\xcf\xfa\xed\xfe" or int.from_bytes(native_bytes[4:8], "little") != 0x0100000C:
            raise SystemExit("native CLI input is not a macOS arm64 Mach-O")
    else:
        if native_bytes[:4] != b"\x7fELF":
            raise SystemExit("native CLI input is not an ELF")
        machine = {62: "linux-x86_64", 183: "linux-aarch64"}.get(int.from_bytes(native_bytes[18:20], "little"))
        if machine != journey.host:
            raise SystemExit(f"native CLI input architecture {machine} does not match host {journey.host}")
    # Query the native input for its real static identity (used as the manifest
    # version; it is executable on this host by definition).
    native_version = PackageJourney.run_native(journey.native, "version", "--json")["data"]
    version = native_version["version"]
    assert native_version["protocol"] == 1 and native_version["catalog_version"] == 1, native_version
    inputs = journey.native_inputs(version, native_bytes)
    try:
        # --- native candidate: always packaged, extracted and executed --------
        out = base / "native-out"
        result = journey.package(out, inputs, version=version)
        assert result.returncode == 0, result.stderr
        summary = json.loads(next(out.glob("candidates-*.json")).read_text())
        assert summary["version"] == version and summary["protocol"] == 1, summary
        assert summary["catalog_version"] == 1 and summary["source_revision"] == REVISION, summary
        assert set(a["target"] for a in summary["archives"]) == {
            "macos-arm64", "linux-x86_64", "linux-aarch64"}, summary
        native_root = base / "native-extract" / journey.host
        native_manifest = None
        for record in summary["archives"]:
            extracted = base / "native-extract" / record["target"]
            journey.extract(out / record["archive"], extracted)
            manifest = journey.verify_files(extracted)
            assert manifest["candidate_identity"] == summary["candidate_identity"], manifest
            assert (extracted / "bin/remote-runners/manifest.json").is_file(), record
            assert (extracted / "bin/remote-runners/x86_64-unknown-linux-musl/lintel").is_file(), record
            assert (extracted / "bin/remote-runners/aarch64-unknown-linux-musl/lintel").is_file(), record
            if record["target"] == journey.host:
                native_manifest = manifest
        assert native_manifest is not None, "native target package missing"
        assert native_manifest["target"] == journey.host, native_manifest
        sums_tool = journey.check_sums(native_root)
        native_cli = native_root / "bin/lintel"
        assert os.access(native_cli, os.X_OK), native_cli
        assert native_cli.read_bytes() == native_bytes, "packaged native CLI bytes differ from the supplied input"
        for name in ("candidate.json", "README.txt"):
            metadata = native_root / name
            original = metadata.read_bytes()
            try:
                metadata.write_bytes(original + b"Synthetic corruption.\n")
                journey.check_sums(native_root, valid=False)
            finally:
                metadata.write_bytes(original)
        # Execute the extracted native package (not the source tree).
        home = base / "home"
        home.mkdir()
        env = dict(os.environ, HOME=str(home), LINTEL_TEST_HOME=str(home),
                   LINTEL_STATE_DIR=str(base / "state"), PATH="/usr/bin:/bin")
        version_out = journey.run_native(native_cli, "version", "--json", env=env)["data"]
        assert version_out["product"] == "Lintel" and version_out["version"] == version, version_out
        assert version_out["protocol"] == 1 and version_out["catalog_version"] == 1, version_out
        assert version_out["platform"] == native_manifest["platform"], version_out
        catalog = journey.run_native(native_cli, "capabilities", "--json", env=env)["data"]
        operations = {op["id"] for op in catalog["operations"]}
        assert "discover" in operations and "plan_archive" in operations, operations
        schema = journey.run_native(native_cli, "schema", "plan_archive", env=env)
        assert schema["ok"] is True and schema["data"]["type"] == "object", schema

        # --- retained-state upgrade to a second identity ----------------------
        claude_root = home / "claude-root"
        claude_root.mkdir()
        (claude_root / "CLAUDE.md").write_text("Synthetic instruction only.\n")
        registered = json.loads(subprocess.run(
            [str(native_cli), "env", "register", "--name", "synthetic", "--root", str(claude_root)],
            text=True, capture_output=True, env=env, timeout=60, check=True).stdout)["data"]
        plan = json.loads(subprocess.run(
            [str(native_cli), "policy", "plan", "--environment", registered["id"],
             "--preset", "reduce", "--no-keep-remote-control"],
            text=True, capture_output=True, env=env, timeout=60, check=True).stdout)["data"]
        receipt = json.loads(subprocess.run(
            [str(native_cli), "job", "submit", "--plan", plan["id"], "--approval", plan["hash"]],
            text=True, capture_output=True, env=env, timeout=60, check=True).stdout)["data"]
        out_new = base / "native-out-new"
        result_new = journey.package(out_new, inputs, version=version, revision=SECOND_REVISION)
        assert result_new.returncode == 0, result_new.stderr
        new_summary = json.loads(next(out_new.glob("candidates-*.json")).read_text())
        assert new_summary["candidate_identity"] != summary["candidate_identity"]
        new_root = base / "native-extract-new" / journey.host
        journey.extract(out_new / f"lintel-cli-{new_summary['candidate_identity']}-{journey.host}.tar.gz", new_root)
        journey.verify_files(new_root)
        journey.check_sums(new_root)
        new_cli = new_root / "bin/lintel"
        environments = json.loads(subprocess.run(
            [str(new_cli), "env", "list"], text=True, capture_output=True, env=env,
            timeout=60, check=True).stdout)["data"]
        assert any(item["id"] == registered["id"] for item in environments["environments"]), environments
        shown = json.loads(subprocess.run(
            [str(new_cli), "job", "show", receipt["id"]], text=True, capture_output=True,
            env=env, timeout=60, check=True).stdout)["data"]
        assert shown["id"] == receipt["id"] and shown["plan_id"] == plan["id"], shown

        # --- documented non-destructive extraction flow -----------------------
        pack_parent = base / "installed"
        first = documented_unpack(out / f"lintel-cli-{summary['candidate_identity']}-{journey.host}.tar.gz",
                                  pack_parent, summary["candidate_identity"])
        assert first.returncode == 0, first.stderr
        installed = pack_parent / summary["candidate_identity"]
        (installed / "SENTINEL").write_text("keep\n")
        before = sorted(p.relative_to(installed).as_posix() for p in installed.rglob("*") if p.is_file())
        second = documented_unpack(out / f"lintel-cli-{summary['candidate_identity']}-{journey.host}.tar.gz",
                                   pack_parent, summary["candidate_identity"])
        assert second.returncode != 0, "documented flow overwrote an existing version directory"
        after = sorted(p.relative_to(installed).as_posix() for p in installed.rglob("*") if p.is_file())
        assert before == after, (before, after)
        assert (installed / "SENTINEL").read_text() == "keep\n"

        # --- rejection evidence (host-independent fixtures) -------------------
        rejected = base / "rejected"
        wrong_arch = base / "wrong-arch"
        wrong_arch.write_bytes(elf(40))
        dynamic = base / "dynamic"
        dynamic.write_bytes(elf(62, interp=True))
        stale = base / "stale-x86"
        stale.write_bytes(elf(62) + b"stale")
        cases = {
            "wrong_architecture": journey.package(rejected, inputs, version=version, x86=wrong_arch),
            "dynamic_loader": journey.package(rejected, inputs, version=version, x86=dynamic),
            "missing_input": journey.package(rejected, inputs, version=version, x86=base / "absent"),
            "input_runner_mismatch": journey.package(rejected, inputs, version=version, x86=stale),
            "invalid_revision": journey.package(rejected, inputs, version=version, revision="a239123"),
        }
        for code, outcome in cases.items():
            assert outcome.returncode != 0 and code in outcome.stderr, (code, outcome.stderr)
        assert not rejected.exists(), "a rejected run still wrote output"

        # --- no-replace: archive, dangling symlink, dangling index -------------
        again = journey.package(out, inputs, version=version)
        assert again.returncode != 0 and "output_exists" in again.stderr, again.stderr
        link_out = base / "link-out"
        link_out.mkdir()
        dangling = link_out / f"lintel-cli-{summary['candidate_identity']}-macos-arm64.tar.gz"
        os.symlink("/nonexistent/lintel", dangling)
        link_run = journey.package(link_out, inputs, version=version)
        assert link_run.returncode != 0 and "output_exists" in link_run.stderr, link_run.stderr
        assert dangling.is_symlink() and os.readlink(dangling) == "/nonexistent/lintel"
        index_out = base / "index-out"
        index_out.mkdir()
        dangling_index = index_out / f"candidates-{summary['candidate_identity']}.json"
        os.symlink("/nonexistent/index", dangling_index)
        index_run = journey.package(index_out, inputs, version=version)
        assert index_run.returncode != 0 and "output_exists" in index_run.stderr, index_run.stderr
        assert not list(index_out.glob("*.tar.gz")), "index refusal still wrote an archive"
        assert dangling_index.is_symlink() and os.readlink(dangling_index) == "/nonexistent/index"

        manifest_snapshot(journey, inputs, version)

        if journey.host == "macos-arm64":
            # A native Mach-O may exit successfully without returning identity
            # data. It must use declared identity, never claim a verified probe.
            source = base / "missing-identity.c"
            source.write_text('#include <stdio.h>\nint main(void) { puts("{\\"ok\\":true}"); return 0; }\n')
            stub = base / "missing-identity"
            subprocess.run(["cc", str(source), "-o", str(stub)], check=True, capture_output=True)
            probe_out = base / "missing-identity-out"
            malformed = journey.package(probe_out, dict(inputs, macos=stub), version=version)
            assert malformed.returncode == 0, malformed.stderr
            probe_summary = json.loads(next(probe_out.glob("candidates-*.json")).read_text())
            mac_archive = next(a for a in probe_summary["archives"] if a["target"] == "macos-arm64")
            probe_root = base / "missing-identity-extract"
            journey.extract(probe_out / mac_archive["archive"], probe_root)
            probe_manifest = json.loads((probe_root / "candidate.json").read_text())
            assert probe_manifest["identity_source"] == "declared_static", probe_manifest
            assert probe_manifest["identity_verified_executed"] is False, probe_manifest

        # --- optional full-real-input smoke (never replaces native acceptance) -
        real_note = "skipped (canonical inputs absent)"
        real_paths = {
            "macos": ROOT / "target/release/lintel",
            "x86": ROOT / "target/x86_64-unknown-linux-musl/release/lintel",
            "arm": ROOT / "target/aarch64-unknown-linux-musl/release/lintel",
        }
        if all(p.exists() for p in real_paths.values()):
            real_out = base / "real-out"
            # The real smoke uses only canonical inputs so every Linux CLI input
            # is byte-identical to its canonical runner.
            real_inputs = {
                "macos": real_paths["macos"],
                "x86": real_paths["x86"],
                "arm": real_paths["arm"],
                "runners": ROOT / "apps/desktop/src-tauri/runner-bundles",
            }
            real_version = None if journey.host == "macos-arm64" else version
            real = journey.package(real_out, real_inputs, version=real_version)
            assert real.returncode == 0, real.stderr
            real_summary = json.loads(next(real_out.glob("candidates-*.json")).read_text())
            real_note = f"ran ({real_summary['candidate_identity']})"

        print("cli_package_journey: OK")
        print(f"  host={journey.host} native_input={journey.native} version={version}")
        print(f"  executed={native_cli} sums_tool={sums_tool}")
        print(f"  upgrade={new_summary['candidate_identity']} retained environment={registered['id']} job={receipt['id']}")
        print(f"  synthetic_fixtures={journey.syn_runners}")
        print(f"  real_input_smoke={real_note}")
        return 0
    finally:
        journey.cleanup()


if __name__ == "__main__":
    raise SystemExit(main())

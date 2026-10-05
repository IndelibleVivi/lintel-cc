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
exists locally in the expected platform format; it never replaces the native
acceptance. Nothing is a silent skip. Never points at the operator's home. Run:
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


def elf(machine: int, interp: bool = False, *, file_type: int = 2,
        entry: int = 0x401000, needed: bool = False, dynamic: bool = False,
        memory_size: int = 256) -> bytes:
    header = bytearray(64)
    header[0:4] = b"\x7fELF"
    header[4] = 2
    header[5] = 1
    header[6] = 1
    struct.pack_into("<H", header, 16, file_type)
    struct.pack_into("<H", header, 18, machine)
    struct.pack_into("<I", header, 20, 1)
    struct.pack_into("<Q", header, 24, entry)
    struct.pack_into("<H", header, 52, 64)
    phoff, phentsize, phnum = 64, 56, 2 if needed or dynamic else 1
    struct.pack_into("<Q", header, 32, phoff)
    struct.pack_into("<H", header, 54, phentsize)
    struct.pack_into("<H", header, 56, phnum)
    program = bytearray(56)
    payload_offset = phoff + phentsize * phnum
    struct.pack_into("<IIQQQQQQ", program, 0, 3 if interp else 1, 5,
                     payload_offset, 0x401000, 0x401000, 256, memory_size, 1)
    payload = bytearray(b"\x90" * 256)
    if needed or dynamic:
        dynamic_program = struct.pack("<IIQQQQQQ", 2, 4, payload_offset, 0x401000, 0x401000, 32, 32, 8)
        payload[:32] = struct.pack("<QQQQ", 1 if needed else 0, 1, 0, 0)
        program += dynamic_program
    return bytes(header) + bytes(program) + bytes(payload)


def macho_arm64(file_type: int = 2, *, entry: bool = True,
                entryoff: int | None = None, executable: bool = True,
                legacy: bool = False) -> bytes:
    # Synthetic format fixture, never counted as native runtime acceptance.
    command_size = 72 + ((288 if legacy else 24) if entry else 0) + (0 if legacy else 32)
    payload_offset = 32 + command_size
    file_size = payload_offset + 256
    segment = struct.pack("<II16sQQQQIIII", 0x19, 72, b"__TEXT", 0x100000000,
                          0x4000, 0, file_size, 5, 5 if executable else 1, 0, 0)
    main = struct.pack("<IIQQ", 0x80000028, 24,
                       payload_offset if entryoff is None else entryoff, 0) if entry else b""
    if entry and legacy:
        state = bytearray(272)
        struct.pack_into("<Q", state, 256, 0x100000000 + payload_offset)
        main = struct.pack("<IIII", 5, 288, 6, 68) + state
    dyld = struct.pack("<III", 0xe, 32, 12) + b"/usr/lib/dyld\0" + b"\0" * 6
    header = struct.pack("<IIIIIIII", 0xFEEDFACF, 0x0100000C, 0, file_type,
                         1 + int(entry) + int(not legacy), command_size, 0, 0)
    return header + segment + main + (b"" if legacy else dyld) + b"\x20\x00\x80\x52\xc0\x03\x5f\xd6" + b"\0" * 248


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


def canonical_smoke_inputs(root: Path) -> dict[str, Path] | None:
    inputs = {
        "macos": root / "target/release/lintel",
        "x86": root / "target/x86_64-unknown-linux-musl/release/lintel",
        "arm": root / "target/aarch64-unknown-linux-musl/release/lintel",
        "runners": root / "apps/desktop/src-tauri/runner-bundles",
    }
    binaries = [(inputs["macos"], None), (inputs["x86"], 62), (inputs["arm"], 183),
                (inputs["runners"] / "x86_64-unknown-linux-musl/lintel", 62),
                (inputs["runners"] / "aarch64-unknown-linux-musl/lintel", 183)]
    # Availability only: the packager still owns full format/manifest validation.
    # target/release/lintel is a Linux host build on Linux, not a Mac input.
    try:
        if not (inputs["runners"] / "manifest.json").is_file():
            return None
        for binary, machine in binaries:
            with binary.open("rb") as handle:
                header = handle.read(64)
            if machine is None:
                if len(header) < 32 or struct.unpack_from("<I", header)[0] != 0xFEEDFACF \
                        or struct.unpack_from("<I", header, 4)[0] != 0x0100000C \
                        or struct.unpack_from("<I", header, 12)[0] != 2:
                    return None
            elif len(header) < 64 or header[:6] != b"\x7fELF\x02\x01" \
                    or struct.unpack_from("<H", header, 18)[0] != machine \
                    or struct.unpack_from("<H", header, 16)[0] not in (2, 3):
                return None
    except OSError:
        return None
    return inputs


def smoke_selection(base: Path) -> None:
    root = base / "smoke-selection"
    runners = root / "apps/desktop/src-tauri/runner-bundles"
    payloads = {
        root / "target/release/lintel": macho_arm64(),
        root / "target/x86_64-unknown-linux-musl/release/lintel": elf(62),
        root / "target/aarch64-unknown-linux-musl/release/lintel": elf(183),
        runners / "x86_64-unknown-linux-musl/lintel": elf(62),
        runners / "aarch64-unknown-linux-musl/lintel": elf(183),
        runners / "manifest.json": b'{"runners":[]}',
    }
    for path, payload in payloads.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(payload)
    assert canonical_smoke_inputs(root) is not None
    for path, original in payloads.items():
        path.unlink()
        assert canonical_smoke_inputs(root) is None, path
        path.write_bytes(original)
        if path.name == "lintel":
            path.write_bytes(elf(62) if path == root / "target/release/lintel" else macho_arm64())
            assert canonical_smoke_inputs(root) is None, path
            path.write_bytes(original)
    assert canonical_smoke_inputs(root) is not None


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
                revision: str = REVISION, x86: Path | None = None,
                env: dict | None = None) -> subprocess.CompletedProcess[str]:
        argv = [NODE, str(SCRIPT),
                "--macos-arm64", str(inputs["macos"]),
                "--linux-x86_64", str(x86 if x86 is not None else inputs["x86"]),
                "--linux-aarch64", str(inputs["arm"]),
                "--linux-runners", str(inputs["runners"]),
                "--revision", revision,
                "--out", str(out)]
        if version is not None:
            argv += ["--version", version]
        return subprocess.run(argv, text=True, capture_output=True, cwd=ROOT, timeout=180, check=False, env=env)

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
        args = (["shasum", "-a", "256", "-c", "SHA256SUMS"] if platform.system() == "Darwin"
                else ["sha256sum", "-c", "SHA256SUMS"])
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


def executable_inputs(journey: PackageJourney, inputs: dict, version: str) -> None:
    failures = []
    header_only = struct.pack("<IIIIIIII", 0xFEEDFACF, 0x0100000C, 0, 2, 0, 0, 0, 0)
    broken_command = bytearray(macho_arm64())
    struct.pack_into("<I", broken_command, 36, 4)
    for name, payload, code in (("truncated-macho", macho_arm64()[:20], "wrong_format"),
                                ("macho-object", macho_arm64(1), "not_executable"),
                                ("macho-dylib", macho_arm64(6), "not_executable"),
                                ("macho-header-only", header_only, "not_executable"),
                                ("macho-no-entry", macho_arm64(entry=False), "not_executable"),
                                ("macho-unmapped-entry", macho_arm64(entryoff=99999), "not_executable"),
                                ("macho-nonexec", macho_arm64(executable=False), "not_executable"),
                                ("macho-broken-command", broken_command, "wrong_format")):
        binary = journey.base / name
        binary.write_bytes(payload)
        output = journey.base / (name + "-out")
        result = journey.package(output, dict(inputs, macos=binary), version=version)
        if result.returncode == 0 or code not in result.stderr or output.exists():
            failures.append((name, result.returncode, result.stderr, output.exists()))
    for key, triple, machine in (("x86", "x86_64-unknown-linux-musl", 62),
                                ("arm", "aarch64-unknown-linux-musl", 183)):
        for kind, payload, code in (("object", elf(machine, file_type=1), "not_executable"),
                                    ("shared", elf(machine, file_type=3, entry=0), "not_executable"),
                                    ("needed", elf(machine, file_type=3, needed=True), "dynamic_dependency"),
                                    ("small-mapping", elf(machine, memory_size=255), "wrong_format"),
                                    ("empty-mapping", elf(machine, memory_size=0), "wrong_format")):
            name = f"{key}-{kind}"
            runners = journey.base / (name + "-runners")
            shutil.copytree(inputs["runners"], runners)
            binary = runners / triple / "lintel"
            binary.write_bytes(payload)
            manifest_path = runners / "manifest.json"
            manifest = json.loads(manifest_path.read_text())
            meta = next(r for r in manifest["runners"] if r["target"] == triple)
            meta.update(bytes=len(payload), sha256=sha256_bytes(payload))
            manifest_path.write_text(json.dumps(manifest))
            supplied = dict(inputs, runners=runners)
            supplied[key] = binary
            output = journey.base / (name + "-out")
            result = journey.package(output, supplied, version=version)
            if result.returncode == 0 or code not in result.stderr or output.exists():
                failures.append((name, result.returncode, result.stderr, output.exists()))
    if journey.host == "macos-arm64":
        source = journey.base / "unsafe-version.c"
        payload = json.dumps({"ok": True, "data": {"product": "Lintel", "version": "1/unsafe",
                              "protocol": 1, "catalog_version": 1}})
        source.write_text('#include <stdio.h>\nint main(void) { puts(' + json.dumps(payload) + '); return 0; }\n')
        binary = journey.base / "unsafe-version"
        subprocess.run(["cc", str(source), "-o", str(binary)], check=True, capture_output=True)
        runners = journey.base / "unsafe-version-runners"
        shutil.copytree(inputs["runners"], runners)
        manifest_path = runners / "manifest.json"
        manifest = json.loads(manifest_path.read_text())
        for meta in manifest["runners"]:
            meta["version"] = "1/unsafe"
        manifest_path.write_text(json.dumps(manifest))
        output = journey.base / "unsafe-version-out"
        result = journey.package(output, dict(inputs, macos=binary, runners=runners))
        if result.returncode == 0 or "invalid_argument" not in result.stderr or output.exists():
            failures.append(("unsafe-version", result.returncode, result.stderr, output.exists()))
    assert not failures, failures
    # Static PIE can have a dynamic table for self-relocations without any
    # external dependency; do not reject ET_DYN/PT_DYNAMIC categorically.
    runners = journey.base / "static-pie-runners"
    shutil.copytree(inputs["runners"], runners)
    manifest_path = runners / "manifest.json"
    manifest = json.loads(manifest_path.read_text())
    supplied = dict(inputs, runners=runners)
    for key, triple, machine in (("x86", "x86_64-unknown-linux-musl", 62),
                                ("arm", "aarch64-unknown-linux-musl", 183)):
        payload = elf(machine, file_type=3, dynamic=True, memory_size=512)
        binary = runners / triple / "lintel"
        binary.write_bytes(payload)
        supplied[key] = binary
        meta = next(r for r in manifest["runners"] if r["target"] == triple)
        meta.update(bytes=len(payload), sha256=sha256_bytes(payload))
    manifest_path.write_text(json.dumps(manifest))
    result = journey.package(journey.base / "static-pie-out", supplied, version=version)
    assert result.returncode == 0, result.stderr
    legacy = journey.base / "arm64-thread-macho"
    legacy.write_bytes(macho_arm64(legacy=True))
    result = journey.package(journey.base / "thread-macho-out", dict(inputs, macos=legacy), version=version)
    assert result.returncode == 0, result.stderr


def publication_failure(journey: PackageJourney, inputs: dict, version: str) -> None:
    identity = f"{version}-candidate-{REVISION[:12]}"
    real_tar = shutil.which("tar")
    failures = []
    for mode in ("tar-later", "index-race", "replaced-archive", "modified-archive"):
        output = journey.base / (mode + "-out")
        output.mkdir()
        keep = output / "KEEP"
        keep.write_text("unrelated output\n")
        index = output / f"candidates-{identity}.json"
        first = output / f"lintel-cli-{identity}-macos-arm64.tar.gz"
        tools = journey.base / (mode + "-tools")
        tools.mkdir()
        count = tools / "count"
        wrapper = tools / "tar"
        wrapper.write_text(f'''#!{sys.executable}
import pathlib,subprocess,sys
count=pathlib.Path({str(count)!r})
n=int(count.read_text())+1 if count.exists() else 1
count.write_text(str(n))
mode={mode!r}
if mode=='tar-later' and n==2: sys.exit(42)
if mode!='tar-later' and n==3:
    pathlib.Path({str(index)!r}).write_text('external index')
    if mode=='replaced-archive':
        first=pathlib.Path({str(first)!r})
        first.rename(first.parent/'moved-original')
        first.write_text('external replacement')
    if mode=='modified-archive':
        pathlib.Path({str(first)!r}).write_text('external replacement')
sys.exit(subprocess.call([{real_tar!r},*sys.argv[1:]]))
''')
        wrapper.chmod(0o755)
        env = dict(os.environ, PATH=str(tools) + os.pathsep + os.environ["PATH"])
        result = journey.package(output, inputs, version=version, env=env)
        archives = sorted(p.name for p in output.glob("*.tar.gz"))
        expected = [first.name] if mode in ("replaced-archive", "modified-archive") else []
        if result.returncode == 0 or archives != expected:
            failures.append((mode, result.returncode, archives, result.stderr))
            continue
        assert keep.read_text() == "unrelated output\n"
        if mode != "tar-later":
            assert index.read_text() == "external index"
            if mode in ("replaced-archive", "modified-archive"):
                assert first.read_text() == "external replacement"
                continue
            index.unlink()  # Own synthetic collision fixture, never product cleanup.
        retry = journey.package(output, inputs, version=version)
        assert retry.returncode == 0 and index.is_file(), retry.stderr
        assert keep.read_text() == "unrelated output\n"
    assert not failures, failures


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
        executable_inputs(journey, inputs, version)
        publication_failure(journey, inputs, version)

        for key, triple in (("x86", "x86_64-unknown-linux-musl"),
                            ("arm", "aarch64-unknown-linux-musl")):
            oversized_runners = base / f"oversized-runners-{key}"
            shutil.copytree(inputs["runners"], oversized_runners)
            oversized_cli = oversized_runners / triple / "lintel"
            with oversized_cli.open("ab") as handle:
                handle.truncate(32 * 1024 * 1024 + 1)
            oversized_bytes = oversized_cli.read_bytes()
            manifest_path = oversized_runners / "manifest.json"
            manifest = json.loads(manifest_path.read_text())
            meta = next(r for r in manifest["runners"] if r["target"] == triple)
            meta.update(bytes=len(oversized_bytes), sha256=sha256_bytes(oversized_bytes))
            manifest_path.write_text(json.dumps(manifest))
            oversized_inputs = dict(inputs, runners=oversized_runners)
            oversized_inputs[key] = oversized_cli
            oversized_out = base / f"oversized-out-{key}"
            outcome = journey.package(oversized_out, oversized_inputs, version=version)
            assert outcome.returncode != 0 and "runner_too_large" in outcome.stderr, (key, outcome.stderr)
            assert not oversized_out.exists(), "oversized runner still published output"

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
        smoke_selection(base)
        real_note = "skipped (canonical inputs absent or wrong platform format)"
        real_inputs = canonical_smoke_inputs(ROOT)
        if real_inputs is not None:
            real_out = base / "real-out"
            # The real smoke uses only canonical inputs so every Linux CLI input
            # is byte-identical to its canonical runner.
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

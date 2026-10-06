#!/usr/bin/env python3
"""End-to-end large-work preservation journey (bounded-memory streaming).

Exercises the same `lintel.work/1` (age passphrase) format end to end with
files that exceed the historical 8 MiB / 32 MiB in-memory limits, and measures
the real peak RSS of the core commands in an isolated child process.

Flow:
  - install A: archive-only a selection of >8 MiB single files with a total
    >32 MiB (a large binary, a JSONL carrying opaque thinking/signature and
    unknown fields, plus small reference files);
  - carry the encrypted package to an independent install B (its own HOME and
    state, no access to A's job/inventory) and use the explicit `archive_path`:
    inspect, paged `session_read` at the head/middle/tail, `plan_import`
    (+execute) into B's own root, and `plan_preserve` into a new root;
  - assert every migrated/preserved file matches the original SHA-256 and that
    A's originals are byte-identical afterwards (reference stays reference);
  - assert the 256 MiB/file and 1 GiB/total metadata preflight blockers fire on
    real sparse sources and that an exact selection excluding them passes;
  - assert an exact selection refuses a changed / same-bytes (inode-replaced)
    selected file before accept and never expands for unselected changes;
  - assert a wrong passphrase, a truncated ciphertext, and a tampered tail are
    refused with no private staging left behind;
  - accept a static synthetic legacy `lintel.work/1` package that omits the
    optional `bytes` member, without an external age executable or skip.

Never points at the operator's home; synthetic temporary roots only. Build
`cargo build -p lintel-runner` first, then run:
  python3 tests/large_work_journey.py [path-to-lintel]
The optional memory report is written to $LINTEL_LARGE_WORK_REPORT or a
temporary path and summarised as one small JSON object on stdout.
"""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import tempfile
import unittest
from typing import Any, Callable, Optional

ROOT = Path(__file__).resolve().parents[1]
# `argv[1]` may be an explicit binary path; every other non-flag argument is a
# unittest selector (module/Class/Class.test).
_ARGV = sys.argv[1:]
BINARY = ROOT / 'target/debug/lintel'
if _ARGV and not _ARGV[0].startswith('-') and Path(_ARGV[0]).is_file():
    BINARY = Path(_ARGV[0]).resolve()
    _SELECTORS = _ARGV[1:]
else:
    _SELECTORS = [a for a in _ARGV if not a.startswith('-')]
PASSPHRASE = "synthetic-large-work-passphrase-only"
RUN_REPORT: dict[str, Any] = {
    "schema": "lintel.large-work-evidence/1", "platform": platform.platform(),
    "machine": platform.machine(), "binary_build": "caller-supplied (canonical check uses debug runner)",
    "kdf": {"writer": "age 0.12.1 device-selected default", "reader_max_log_n": 20}, "tests": {},
}

# Synthetic age-encrypted historical v1 JSON; files omit the optional bytes field.
LEGACY_V1_CIPHER = 'YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IHNjcnlwdCBRdXJnSVlmYkovU05vVjFBdXJTL1B3IDE0CmtrV0ZSRXhKMXhxRk5QdjhaMyt3TXlwUEpFTGhyOURrQi8rR3NyTUQ5d0UKLS0tIDNpM1FvMi9VektQWWxuUGpxNHgwT0x4MXczQXFIK25RVkVhNTE1eU1lOWcKLbpSArmg+ZyyfG4eqkMU83/61rbQ6sK4K1LJhC+EgLmnzUMJBCWlwyDeSP8+cPR9rHxmVTnD6XRoaj1iaHOGlA5EJZG+s20ty3u6y71DCFU5A70KFURrk6BdYv2TDFo30GvVUUrdOTPiXh9NryJuiC2rFNdxB7yk2R6KKqKlwKkyfsHBCaXKpf9rsg3v8R5CmdUEvlx52l6RUyHywkMkJS/XaEknTcWeDYs859tZk304BG7cDHWbuKccMgzRU7z1/uQnXW2dZFshBTTCPjcVgUYCFjmAl2uCt/wmKH86hSU6IqrSpytT9rZRILqE0RQpf226+sMpNOXmSNXINRet9To4jrAJXzBXs2c3d8Z1d7zZUDvwAZh36wKHZ+zgG0rwC81fxticAQg2QQ9FM6hjl+oRiSzh+A4+cdWVkT2sL8nS4md3NvD6Fw=='

MIB = 1024 * 1024
# Production finite capacity (must match crates/core/src/work.rs).
FILE_LIMIT = 256 * MIB
TOTAL_LIMIT = 1024 * MIB
# Sizes chosen so each is >8 MiB and the total is >32 MiB, but small enough to
# run quickly.
BIG_BINARY = 24 * MIB
BIG_JSONL = 12 * MIB


def sha256_file(path: Path) -> str:
    """Stream a file's SHA-256 without loading it whole."""
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_binary(path: Path, size: int) -> str:
    """Write `size` deterministic bytes in chunks; return the SHA-256."""
    path.parent.mkdir(parents=True, exist_ok=True)
    digest = hashlib.sha256()
    block = bytes(range(256)) * (1024 // 256)  # 1 KiB pattern
    remaining = size
    with path.open("wb") as handle:
        while remaining > 0:
            chunk = block[: min(len(block), remaining)]
            handle.write(chunk)
            digest.update(chunk)
            remaining -= len(chunk)
    return digest.hexdigest()


def write_jsonl(path: Path, size: int) -> tuple[str, int]:
    """Write a synthetic transcript of >= `size` bytes with opaque thinking,
    signature and unknown fields; return (sha256, records)."""
    path.parent.mkdir(parents=True, exist_ok=True)
    digest = hashlib.sha256()
    records = 0
    written = 0
    with path.open("wb") as handle:
        while written < size:
            lines = [
                json.dumps({
                    "type": "user",
                    "sessionId": "synthetic-session",
                    "cwd": "/synthetic",
                    "uuid": f"u{records}",
                    "message": {"content": [{"type": "text", "text": "synthetic user text"}]},
                }).encode(),
                json.dumps({
                    "type": "assistant",
                    "sessionId": "synthetic-session",
                    "message": {"content": [
                        {"type": "thinking", "thinking": "synthetic chain", "signature": "synthetic-signature"},
                        {"type": "text", "text": "synthetic answer"},
                    ]},
                    "unknownField": {"nested": [1, 2, 3]},
                }).encode(),
                json.dumps({"type": "totally-unknown", "payload": {"k": records}}).encode(),
            ]
            for line in lines:
                handle.write(line + b"\n")
                digest.update(line + b"\n")
                written += len(line) + 1
                records += 1
            if written >= size:
                break
    return digest.hexdigest(), records


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

    def call(self, command: str, **fields: Any) -> dict[str, Any]:
        payload = {"command": command, **fields}
        result = subprocess.run(
            [str(BINARY), "request"], input=json.dumps(payload), text=True,
            capture_output=True, cwd=self.base, env=self.env, timeout=300, check=False,
        )
        try:
            return json.loads(result.stdout)
        except json.JSONDecodeError as error:
            raise AssertionError(
                f"runner did not return JSON: exit={result.returncode}; {error}; {result.stderr[:300]}"
            ) from error

    def data(self, command: str, **fields: Any) -> Any:
        response = self.call(command, **fields)
        assert response["ok"], json.dumps(response.get("error"), ensure_ascii=False)
        return response["data"]

    def named(self, *args: str, input_fields: Optional[dict[str, Any]] = None) -> Any:
        result = subprocess.run(
            [str(BINARY), *args],
            input=json.dumps(input_fields) if input_fields is not None else "",
            text=True, capture_output=True, cwd=self.base, env=self.env, timeout=300,
        )
        return json.loads(result.stdout)

    def register(self, root: Path, name: str = "synthetic large") -> str:
        return str(self.data("register", name=name, root=str(root))["id"])

    def execute(self, plan: dict[str, Any]) -> dict[str, Any]:
        return self.data(
            "execute", plan_id=plan["id"], approval=plan["hash"], archive_passphrase=PASSPHRASE,
        )


def measure(argv: list[str], env: dict[str, str], cwd: Path, stdin_text: str = "", *,
            timeout: int = 600, on_data: Optional[Callable[[Any], None]] = None,
            disk_root: Optional[Path] = None, require_completed: bool = True) -> dict[str, Any]:
    """Run one command in an isolated child and report its peak RSS + elapsed.

    `resource.getrusage(RUSAGE_CHILDREN)` is measured inside a dedicated
    wrapper process so the value covers exactly this command (macOS reports
    bytes, Linux kilobytes; the raw value and platform are both recorded).
    """
    wrapper = '''
import json,os,resource,subprocess,sys,threading,time
argv=json.loads(sys.argv[1]); cwd=sys.argv[2]; env=json.loads(sys.argv[3]); disk=sys.argv[4]
payload=sys.stdin.read(); stop=threading.Event(); peaks={'logical_bytes':0,'allocated_bytes':0}
def sample():
    logical=allocated=0
    for parent,dirs,files in os.walk(disk):
        for name in files:
            try:
                stat=os.lstat(os.path.join(parent,name)); logical+=stat.st_size; allocated+=stat.st_blocks*512
            except FileNotFoundError: pass
    peaks['logical_bytes']=max(peaks['logical_bytes'],logical)
    peaks['allocated_bytes']=max(peaks['allocated_bytes'],allocated)
def watch():
    while not stop.is_set():
        sample(); stop.wait(1)
if disk:
    sample(); watcher=threading.Thread(target=watch,daemon=True); watcher.start()
start=time.monotonic()
proc=subprocess.run(argv,cwd=cwd,env=env,input=payload,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
elapsed=time.monotonic()-start
if disk: stop.set(); watcher.join(); sample()
usage=resource.getrusage(resource.RUSAGE_CHILDREN)
print(json.dumps({'response':json.loads(proc.stdout),'returncode':proc.returncode,'elapsed':elapsed,
 'ru_maxrss':usage.ru_maxrss,'maxrss_units':('bytes' if sys.platform=='darwin' else 'kilobytes'),
 'platform':sys.platform,'peak_fixture_disk':peaks if disk else None}))
'''
    result = subprocess.run(
        [sys.executable, "-c", wrapper, json.dumps(argv), str(cwd), json.dumps(env), str(disk_root) if disk_root else ""],
        input=stdin_text, capture_output=True, text=True, timeout=timeout,
    )
    if result.returncode:
        raise AssertionError(result.stderr)
    measured = json.loads(result.stdout)
    response = measured.pop("response")
    if not response.get("ok"):
        raise AssertionError(response)
    data = response["data"]
    if require_completed and "status" in data and data["status"] != "completed":
        raise AssertionError(data)
    if on_data:
        on_data(data)
    measured["response_ok"] = True
    return measured


class LargeWorkJourney(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="lintel-large-")
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name).resolve()
        self.source = Install(self.base, "source")
        self.root = self.source.home / "claude-root"
        self.root.mkdir(parents=True)
        self.report: dict[str, Any] = {"measurements": {}, "notes": []}

    def tearDown(self) -> None:
        RUN_REPORT["tests"][self._testMethodName] = self.report
        report_path = Path(os.environ.get("LINTEL_LARGE_WORK_REPORT", str(self.base.parent / "lintel-large-work-report.json")))
        report_path.write_text(json.dumps(RUN_REPORT, ensure_ascii=False, indent=2))
        print(json.dumps({"large_work_report": str(report_path), "test": self._testMethodName,
                          "measurements": self.report["measurements"]}))

    # -- fixtures -------------------------------------------------------------
    def build_source(self) -> dict[str, Any]:
        digests = {}
        # A large binary-content session file. Its extension is `.jsonl` so it is
        # admitted as work content, but its bytes are deliberately not UTF-8: the
        # preservation path must treat a transcript as opaque bytes and never
        # truncate or re-encode it.
        digests["projects/demo/big.jsonl"] = write_binary(self.root / "projects/demo/big.jsonl", BIG_BINARY)
        jsonl_digest, records = write_jsonl(self.root / "projects/demo/session.jsonl", BIG_JSONL)
        digests["projects/demo/session.jsonl"] = jsonl_digest
        (self.root / "CLAUDE.md").write_bytes(b"Synthetic instruction only.\n")
        digests["CLAUDE.md"] = hashlib.sha256(b"Synthetic instruction only.\n").hexdigest()
        (self.root / "projects/demo/memory").mkdir(parents=True, exist_ok=True)
        (self.root / "projects/demo/memory/MEMORY.md").write_bytes(b"# synthetic memory\n")
        digests["projects/demo/memory/MEMORY.md"] = hashlib.sha256(b"# synthetic memory\n").hexdigest()
        return {"digests": digests, "records": records}

    # -- tests ----------------------------------------------------------------
    def test_large_archive_carry_and_import_and_preserve(self) -> None:
        fixture = self.build_source()
        digests = fixture["digests"]
        total = sum((self.root / p).stat().st_size for p in digests)
        self.assertGreater((self.root / "projects/demo/big.jsonl").stat().st_size, 8 * MIB)
        self.assertGreater(total, 32 * MIB, "selection total must exceed the old 32 MiB limit")
        self.report["fixture"] = {
            "files": {p: (self.root / p).stat().st_size for p in digests},
            "total": total,
            "jsonl_records": fixture["records"],
        }
        environment_id = self.source.register(self.root)
        preflight = self.source.data(
            "work_preflight", environment_id=environment_id,
            categories=["instructions", "memory", "sessions"],
        )
        self.assertTrue(preflight["eligible"], preflight["blockers"])
        self.assertEqual(preflight["totals"]["bytes"], total)
        self.assertEqual(preflight["limits"], {
            "file_bytes": FILE_LIMIT, "total_bytes": TOTAL_LIMIT, "files": 10000, "entries": 50000,
        })

        out = self.base / "carried.age"
        plan = self.source.data(
            "plan_archive", environment_id=environment_id,
            categories=["instructions", "memory", "sessions"], output_path=str(out),
        )
        self.assertEqual(plan["outcome"], "archive_only")
        self.assertNotIn("source_identity", json.dumps(plan))
        # Measure the real archive-only execute (default age scrypt KDF).
        archive_argv = json.dumps({
            "command": "execute", "plan_id": plan["id"], "approval": plan["hash"],
            "archive_passphrase": PASSPHRASE,
        })
        measurement = measure([str(BINARY), "request"], self.source.env, self.source.base, archive_argv)
        self.assertEqual(measurement["returncode"], 0, measurement)
        self.report["measurements"]["archive_execute"] = measurement
        receipt = self.source.data(
            "execute", plan_id=plan["id"], approval=plan["hash"], archive_passphrase=PASSPHRASE,
        )
        self.assertEqual(receipt["status"], "completed", receipt)
        self.assertNotIn("new_root", receipt)
        for path, digest in digests.items():
            self.assertEqual(sha256_file(self.root / path), digest, path)
        with out.open("rb") as cipher:
            header = cipher.read(1024)
        self.assertTrue(header.startswith(b"age-encryption.org/"))
        stanza = re.search(rb"-> scrypt [A-Za-z0-9+/]+ (\d+)\n", header)
        self.assertIsNotNone(stanza)
        self.report["scrypt_log_n"] = int(stanza.group(1))
        self.report["cipher_bytes"] = out.stat().st_size

        # --- independent install B, explicit archive_path ---
        target = Install(self.base, "target")
        dest = target.home / "other-root"
        dest.mkdir(parents=True)
        dest_id = target.register(dest)
        path = str(out)
        inspect_payload = json.dumps({"archive_passphrase": PASSPHRASE})
        measurement = measure(
            [str(BINARY), "work", "archive", "inspect", "--archive-path", path],
            target.env, target.base, inspect_payload,
        )
        self.assertEqual(measurement["returncode"], 0, measurement)
        self.report["measurements"]["inspect"] = measurement
        manifest = target.data("archive_inspect", archive_path=path, archive_passphrase=PASSPHRASE)
        self.assertEqual(manifest["generator"], "Lintel")
        self.assertEqual(manifest["schema"], "lintel.work/1")
        inspected = {f["path"]: f["digest"] for f in manifest["files"]}
        self.assertEqual(inspected, digests)

        # Paged session read at head/middle/tail of the large JSONL.
        sessions = "projects/demo/session.jsonl"
        session_size = (self.root / sessions).stat().st_size
        for offset in [0, session_size // 2, session_size - 1024]:
            read_payload = json.dumps({"archive_passphrase": PASSPHRASE})
            measurement = measure(
                [str(BINARY), "work", "session", "read", "--archive-path", path,
                 "--path", sessions, "--offset", str(offset)],
                target.env, target.base, read_payload,
            )
            self.assertEqual(measurement["returncode"], 0, measurement)
            self.report["measurements"].setdefault("session_read", []).append(measurement)
            page = target.data(
                "session_read", archive_path=path, archive_passphrase=PASSPHRASE,
                path=sessions, offset=offset,
            )
            self.assertEqual(page["digest"], digests[sessions])
            self.assertLessEqual(page["page_bytes"], 256 * 1024)
            self.assertEqual(page["offset"], offset)
            if offset == 0:
                kinds = {r.get("kind") for r in page["records"]}
                self.assertIn("user", kinds)
                self.assertNotIn("secret", json.dumps(page))
            if offset == session_size - 1024:
                self.assertTrue(page["done"], "tail page must reach the end")

        # Import into B's own root, then verify every file byte-for-byte.
        import_plan = target.named(
            "work", "import", "plan", "--environment", dest_id, "--archive-path", path,
            "--categories", "instructions,memory,sessions",
            input_fields={"archive_passphrase": PASSPHRASE},
        )["data"]
        imported = target.data(
            "execute", plan_id=import_plan["id"], approval=import_plan["hash"],
            archive_passphrase=PASSPHRASE,
        )
        self.assertEqual(imported["status"], "completed", imported)
        self.assertEqual((dest / "CLAUDE.md").read_bytes(), b"Synthetic instruction only.\n")
        for relative, digest in [
            ("lintel-imports/projects/demo/big.jsonl", digests["projects/demo/big.jsonl"]),
            ("lintel-imports/projects/demo/session.jsonl", digests["projects/demo/session.jsonl"]),
            ("lintel-imports/projects/demo/memory/MEMORY.md", digests["projects/demo/memory/MEMORY.md"]),
        ]:
            self.assertTrue((dest / relative).is_file(), relative)
            self.assertEqual(sha256_file(dest / relative), digest, relative)
        # Legacy raw/named import defaults activate instructions; the App uses
        # an explicit reference default. Sessions/memory remain reference files.
        self.assertFalse((dest / "lintel-imports/projects/demo/big.jsonl").is_symlink())

        # Preserve install A's own large root into a fresh new root.
        preserve = self.source.data(
            "plan_preserve", environment_id=environment_id,
            categories=["instructions", "memory", "sessions"],
            activate={"instructions": False},
        )
        preserve_argv = json.dumps({
            "command": "execute", "plan_id": preserve["id"], "approval": preserve["hash"],
            "archive_passphrase": PASSPHRASE,
        })
        # Preserve archives A's root and creates a new one; measure it directly.
        measurement = measure([str(BINARY), "request"], self.source.env, self.source.base, preserve_argv)
        self.assertEqual(measurement["returncode"], 0, measurement)
        self.report["measurements"]["preserve_execute"] = measurement
        preserved = self.source.data(
            "execute", plan_id=preserve["id"], approval=preserve["hash"],
            archive_passphrase=PASSPHRASE,
        )
        self.assertEqual(preserved["outcome"], "preserved", preserved)
        new_root = Path(preserved["new_root"])
        # Instructions inactive: CLAUDE.md is kept as reference, not activated.
        self.assertFalse((new_root / "CLAUDE.md").exists())
        for relative, digest in [
            ("lintel-imports/CLAUDE.md", digests["CLAUDE.md"]),
            ("lintel-imports/projects/demo/big.jsonl", digests["projects/demo/big.jsonl"]),
            ("lintel-imports/projects/demo/session.jsonl", digests["projects/demo/session.jsonl"]),
            ("lintel-imports/projects/demo/memory/MEMORY.md", digests["projects/demo/memory/MEMORY.md"]),
        ]:
            self.assertTrue((new_root / relative).is_file(), relative)
            self.assertEqual(sha256_file(new_root / relative), digest, relative)
        # A's originals are still byte-identical after preserve.
        for path, digest in digests.items():
            self.assertEqual(sha256_file(self.root / path), digest, path)

    def test_metadata_preflight_blocks_real_oversize_and_exact_excludes(self) -> None:
        # Sparse sources trigger the metadata-only limit checks without reading.
        big = self.root / "projects/demo/big.jsonl"
        big.parent.mkdir(parents=True, exist_ok=True)
        with big.open("wb") as handle:
            handle.truncate(FILE_LIMIT + 1)
        (self.root / "CLAUDE.md").write_bytes(b"kept instructions\n")
        self.root.joinpath("projects/demo/memory").mkdir(parents=True, exist_ok=True)
        self.root.joinpath("projects/demo/memory/MEMORY.md").write_bytes(b"# m\n")
        environment_id = self.source.register(self.root)
        report = self.source.data(
            "work_preflight", environment_id=environment_id, categories=["instructions", "sessions"],
        )
        self.assertFalse(report["eligible"], report)
        self.assertTrue(
            any(b["code"] == "file_too_large" and b["path"] == "projects/demo/big.jsonl"
                for b in report["blockers"]),
            report["blockers"],
        )
        # An exact selection excluding the oversized file is eligible.
        exact = self.source.data(
            "work_preflight", environment_id=environment_id, categories=["instructions"],
            selected_paths=["CLAUDE.md"],
        )
        self.assertTrue(exact["eligible"], exact)

    def test_total_limit_blocks_and_exact_excludes(self) -> None:
        # Four sparse 256 MiB files plus one byte exceed 1 GiB.
        for index in range(4):
            path = self.root / f"projects/demo/bulk-{index}.jsonl"
            path.parent.mkdir(parents=True, exist_ok=True)
            with path.open("wb") as handle:
                handle.truncate(FILE_LIMIT)
        self.root.joinpath("CLAUDE.md").write_bytes(b"k")
        environment_id = self.source.register(self.root)
        report = self.source.data(
            "work_preflight", environment_id=environment_id, categories=["instructions", "sessions"],
        )
        self.assertFalse(report["eligible"], report)
        self.assertTrue(any(b["code"] == "total_bytes_exceeded" for b in report["blockers"]), report["blockers"])
        exact = self.source.data(
            "work_preflight", environment_id=environment_id, categories=["instructions"],
            selected_paths=["CLAUDE.md"],
        )
        self.assertTrue(exact["eligible"], exact)

    def test_exact_selection_rejects_mutation_and_ignores_unselected(self) -> None:
        selected = self.root / "projects/demo/keep.jsonl"
        selected.parent.mkdir(parents=True, exist_ok=True)
        selected.write_bytes(b"frozen bytes\n")
        (self.root / "CLAUDE.md").write_bytes(b"kept\n")
        environment_id = self.source.register(self.root)
        for mutation in ["content", "same-bytes-replacement", "missing"]:
            with self.subTest(mutation=mutation):
                selected.write_bytes(b"frozen bytes\n")
                plan = self.source.data(
                    "plan_archive", environment_id=environment_id, categories=["sessions"],
                    selected_paths=["projects/demo/keep.jsonl"],
                )
                if mutation == "content":
                    selected.write_bytes(b"changed bytes\n")
                elif mutation == "missing":
                    selected.unlink()
                else:
                    backup = self.base / (plan["id"] + ".original")
                    selected.rename(backup)
                    selected.write_bytes(b"frozen bytes\n")
                result = self.source.call(
                    "execute", plan_id=plan["id"], approval=plan["hash"], archive_passphrase=PASSPHRASE,
                )
                self.assertFalse(result["ok"], result)
                self.assertIn(result["error"]["code"], ["stale_plan", "selected_missing", "selected_changed"])
                self.assertFalse((self.source.state / "jobs" / (plan["id"] + ".json")).exists())

    def test_unselected_changes_do_not_expand_exact_archive(self) -> None:
        selected = self.root / "CLAUDE.md"
        selected.write_bytes(b"exact original\n")
        environment_id = self.source.register(self.root)
        plan = self.source.data("plan_archive", environment_id=environment_id,
                                categories=["instructions"], selected_paths=["CLAUDE.md"])
        neighbor = self.root / "projects/demo/neighbor.jsonl"
        neighbor.parent.mkdir(parents=True)
        with neighbor.open("wb") as handle:
            handle.truncate(FILE_LIMIT + 1)
        receipt = self.source.execute(plan)
        self.assertEqual(receipt["status"], "completed", receipt)
        manifest = self.source.data("archive_inspect", job_id=receipt["id"], archive_passphrase=PASSPHRASE)
        self.assertEqual([f["path"] for f in manifest["files"]], ["CLAUDE.md"])
        self.assertEqual(selected.read_bytes(), b"exact original\n")

    def test_wrong_pass_truncation_and_tail_tamper_are_refused(self) -> None:
        (self.root / "CLAUDE.md").write_bytes(b"instruction\n")
        environment_id = self.source.register(self.root)
        out = self.base / "carried.age"
        plan = self.source.data(
            "plan_archive", environment_id=environment_id, categories=["instructions"],
            output_path=str(out),
        )
        self.source.execute(plan)
        full = out.read_bytes()
        staging = self.source.state
        # Wrong passphrase.
        wrong = self.source.call("archive_inspect", archive_path=str(out), archive_passphrase="wrong passphrase here")
        self.assertEqual(wrong["error"]["code"], "archive_locked")
        # Missing final age tag.
        truncated = self.base / "truncated.age"
        truncated.write_bytes(full[:-20])
        bad = self.source.call("archive_inspect", archive_path=str(truncated), archive_passphrase=PASSPHRASE)
        self.assertEqual(bad["error"]["code"], "invalid_archive")
        # Appended byte after the age stream.
        tampered = self.base / "tampered.age"
        tampered.write_bytes(full + b"\x00")
        bad = self.source.call("archive_inspect", archive_path=str(tampered), archive_passphrase=PASSPHRASE)
        self.assertIn(bad["error"]["code"], ["invalid_archive", "archive_integrity"])
        # No private staging directories left behind by any of the reads.
        leftover = [p for p in staging.glob(".lintel-work-stage-*")] if staging.exists() else []
        self.assertEqual(leftover, [], leftover)

    def test_session_offsets_use_original_crlf_and_invalid_utf8_bytes(self) -> None:
        lines = [b'{"type":"user","message":{"content":"first"}}\r\n',
                 b'unknown \xff record\r\n',
                 b'{"type":"user","message":{"content":"third"}}\r\n']
        source = self.root / "projects/demo/offsets.jsonl"
        source.parent.mkdir(parents=True)
        source.write_bytes(b"".join(lines))
        environment_id = self.source.register(self.root)
        plan = self.source.data("plan_archive", environment_id=environment_id, categories=["sessions"])
        receipt = self.source.execute(plan)
        self.assertEqual(receipt["status"], "completed", receipt)
        page = self.source.data("session_read", job_id=receipt["id"], archive_passphrase=PASSPHRASE,
                                path="projects/demo/offsets.jsonl")
        self.assertEqual([r["offset"] for r in page["records"]], [0, len(lines[0]), len(lines[0])+len(lines[1])])
        self.assertEqual(page["page_bytes"], sum(map(len, lines)))
        self.assertEqual(page["digest"], hashlib.sha256(b"".join(lines)).hexdigest())

    def test_legacy_package_without_bytes_is_accepted(self) -> None:
        import base64
        legacy = self.base / "legacy.age"
        legacy.write_bytes(base64.b64decode(LEGACY_V1_CIPHER))
        inspected = self.source.data("archive_inspect", archive_path=str(legacy), archive_passphrase=PASSPHRASE)
        self.assertEqual(inspected["files"][0]["bytes"], len(b"legacy instruction\n"))
        self.assertEqual(inspected["files"][0]["digest"], hashlib.sha256(b"legacy instruction\n").hexdigest())
        self.report["notes"].append({"legacy_fixture": "static age + lintel.work/1 without bytes"})


if __name__ == "__main__":
    print("Large-work preservation journey; synthetic temporary roots only.", file=sys.stderr)
    unittest.main(argv=[sys.argv[0], *_SELECTORS, *[a for a in _ARGV if a.startswith('-')]])

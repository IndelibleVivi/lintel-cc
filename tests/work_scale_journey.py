#!/usr/bin/env python3
"""Independent release-capacity evidence; synthetic temporary roots only.

Run through tests/verify.py --checks work-scale with LINTEL_SCALE_RUNNER set
to an explicit release binary and LINTEL_SCALE_TMPDIR on a filesystem with
at least 16 GiB free. It does not compile, install or modify personal state.
"""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import tempfile

from large_work_journey import Install, PASSPHRASE, measure, sha256_file

ROOT = Path(__file__).resolve().parents[1]
RUNNER = Path(os.environ.get("LINTEL_SCALE_RUNNER", str(ROOT / "target/release/lintel"))).resolve()
GIB = 1024 ** 3
MIB = 1024 ** 2
report = {"schema": "lintel.work-scale-evidence/1", "platform": platform.platform(),
          "machine": platform.machine(), "runner": str(RUNNER),
          "build": "caller-selected release candidate; source identity checked separately",
          "checks": [], "measurements": {}, "passed": False}
report_path = Path(os.environ.get("LINTEL_SCALE_REPORT", str(Path(tempfile.gettempdir()) / "lintel-work-scale.json")))


class Actor(Install):
    def call(self, command, **fields):
        result = subprocess.run([str(RUNNER), "request"], input=json.dumps({"command": command, **fields}),
                                text=True, capture_output=True, cwd=self.base, env=self.env, timeout=3600)
        return json.loads(result.stdout)


def measured(actor, name, command, check, **fields):
    report["measurements"][name] = measure(
        [str(RUNNER), "request"], actor.env, actor.base,
        json.dumps({"command": command, **fields}), timeout=3600, on_data=check, disk_root=actor.base,
        require_completed=not command.startswith("plan_"),
    )
    persist()
    print(json.dumps({"step": name, **report["measurements"][name]}), flush=True)


def persist():
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2))


def no_staging(*actors):
    for actor in actors:
        assert not list(actor.state.glob(".lintel-work-stage-*")), "completed operation left full-package plaintext staging"


try:
    assert RUNNER.is_file(), "missing explicit release runner"
    temporary_parent = Path(os.environ.get("LINTEL_SCALE_TMPDIR", tempfile.gettempdir()))
    assert shutil.disk_usage(temporary_parent).free >= 16 * GIB, "scale evidence needs at least 16 GiB free"
    with tempfile.TemporaryDirectory(prefix="lintel-work-scale-", dir=temporary_parent) as temporary:
        base = Path(temporary).resolve(); report["temporary_scope"] = str(base)
        source = Actor(base, "source"); target = Actor(base, "target")
        root = source.home / "config"; root.mkdir()
        expected = {}; block = bytes(range(256)) * 4096
        # Exactly the supported 1 GiB plaintext total and 256 MiB per member.
        for index in range(4):
            relative = f"projects/synthetic/member-{index}.jsonl"
            file = root / relative; file.parent.mkdir(parents=True, exist_ok=True)
            digest = hashlib.sha256()
            with file.open("wb") as output:
                for _ in range(256): output.write(block); digest.update(block)
            expected[relative] = digest.hexdigest()
        environment = source.register(root)
        preflight = source.data("work_preflight", environment_id=environment, categories=["sessions"])
        assert preflight["eligible"] and preflight["totals"]["bytes"] == GIB
        assert preflight["limits"]["file_bytes"] == 256 * MIB and preflight["limits"]["total_bytes"] == GIB
        archive = base / "complete-1gib.age"
        plan = source.data("plan_archive", environment_id=environment, categories=["sessions"], output_path=str(archive))
        def archived(data):
            assert data["status"] == "completed" and data["archive_path"] == str(archive), data
        measured(source, "archive_1gib", "execute", archived, plan_id=plan["id"], approval=plan["hash"], archive_passphrase=PASSPHRASE)
        report["plaintext_bytes"] = GIB; report["member_bytes"] = 256 * MIB
        report["cipher_bytes"] = archive.stat().st_size
        with archive.open("rb") as handle:
            stanza = re.search(rb"-> scrypt [A-Za-z0-9+/]+ (\d+)\n", handle.read(1024))
        assert stanza; report["scrypt_log_n"] = int(stanza.group(1))
        no_staging(source)
        def inspected(data):
            assert {file["path"]: file["digest"] for file in data["files"]} == expected
            assert sum(file["bytes"] for file in data["files"]) == GIB
        measured(target, "independent_inspect_1gib", "archive_inspect", inspected, archive_path=str(archive), archive_passphrase=PASSPHRASE)
        member = next(iter(expected))
        # The deterministic last 17 bytes contain no newline. A .jsonl page
        # otherwise stops at its last complete line, leaving the final partial
        # line for the next offset even when its byte window reached EOF.
        tail_offset = 256 * MIB - 17
        for label, offset in [("head", 0), ("middle", 128 * MIB), ("tail", tail_offset)]:
            def page_checked(data, at=offset):
                assert data["digest"] == expected[member] and data["offset"] == at
                assert 0 < data["page_bytes"] <= 256 * 1024
                if at == tail_offset:
                    assert data["done"] and data["page_bytes"] == 17 and data["next_offset"] is None
            measured(target, f"page_1gib_{label}", "session_read", page_checked, archive_path=str(archive), archive_passphrase=PASSPHRASE, path=member, offset=offset, expected_digest=expected[member])
            no_staging(target)
        destination = target.home / "imported"; destination.mkdir()
        destination_id = target.register(destination)
        frozen = []
        measured(target, "plan_import_1gib", "plan_import", lambda data: frozen.append(data), environment_id=destination_id,
                 archive_path=str(archive), archive_passphrase=PASSPHRASE, categories=["sessions"])
        measured(target, "import_1gib", "execute", lambda data: None,
                 plan_id=frozen[0]["id"], approval=frozen[0]["hash"], archive_passphrase=PASSPHRASE)
        for relative, digest in expected.items():
            assert sha256_file(root / relative) == digest
            assert sha256_file(destination / "lintel-imports" / relative) == digest
        no_staging(source, target)
        report["checks"].append("exact 256 MiB/member and 1 GiB aggregate: archive/readback, independent inspect, bounded head/middle/tail pages, import and original/copy digest agreement")
        # Real file-count limit with short, portable names; no giant transcript.
        many = source.home / "many"; many.mkdir(); directory = many / "projects/synthetic"; directory.mkdir(parents=True)
        for index in range(10000): (directory / f"{index:05}.jsonl").write_bytes(b"synthetic\n")
        many_id = source.register(many)
        assert source.data("work_preflight", environment_id=many_id, categories=["sessions"])["eligible"]
        many_archive = base / "10000-files.age"
        many_plan = source.data("plan_archive", environment_id=many_id, categories=["sessions"], output_path=str(many_archive))
        measured(source, "archive_10000_files", "execute", lambda data: None, plan_id=many_plan["id"], approval=many_plan["hash"], archive_passphrase=PASSPHRASE)
        many_inspected = []
        measured(target, "inspect_10000_files", "archive_inspect", lambda data: many_inspected.append(data), archive_path=str(many_archive), archive_passphrase=PASSPHRASE)
        assert len(many_inspected[0]["files"]) == 10000
        count_dest = target.home / "many-import"; count_dest.mkdir(); count_id = target.register(count_dest)
        count_plan = target.data("plan_import", environment_id=count_id, archive_path=str(many_archive), archive_passphrase=PASSPHRASE, categories=["sessions"])
        measured(target, "import_10000_files", "execute", lambda data: None, plan_id=count_plan["id"], approval=count_plan["hash"], archive_passphrase=PASSPHRASE)
        assert len(list((count_dest / "lintel-imports/projects/synthetic").glob("*.jsonl"))) == 10000
        for file in (count_dest / "lintel-imports/projects/synthetic").glob("*.jsonl"): assert file.read_bytes() == b"synthetic\n"
        no_staging(source, target)
        report["checks"].append("10,000 real short-path originals: preflight, frozen preview, archive/inspect/import and every copy byte checked")
        report["passed"] = True; persist()
finally:
    persist(); print(json.dumps({"report": str(report_path), "passed": report["passed"], "checks": report["checks"]}), flush=True)

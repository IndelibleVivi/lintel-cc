#!/usr/bin/env python3
"""Prove the macOS Preview gate rejects linker-only/tampered bundles and
accepts a complete bundle-level ad-hoc signature. Uses only a synthetic App."""
from __future__ import annotations

import json
import plistlib
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
GATE = ROOT / "apps/desktop/scripts/verify-macos-bundle.mjs"


def run(*args: str, check: bool = False) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(args, text=True, capture_output=True, timeout=120)
    if check and result.returncode != 0:
        raise RuntimeError(f"{args[0]} failed: {result.stderr}")
    return result


def fixture(root: Path) -> Path:
    app = root / "Lintel.app"
    macos = app / "Contents/MacOS"
    resources = app / "Contents/Resources"
    macos.mkdir(parents=True)
    resources.mkdir(parents=True)
    source = macos / "lintel-desktop.c"
    source.write_text("int main(void){return 0;}\n", encoding="utf-8")
    run("/usr/bin/clang", "-arch", "arm64", "-Wl,-adhoc_codesign",
        "-o", str(macos / "lintel-desktop"), str(source), check=True)
    linker = run("/usr/bin/codesign", "-dvvv", str(macos / "lintel-desktop"))
    if "linker-signed" not in linker.stderr:
        raise RuntimeError("could not produce a linker-signed synthetic executable")
    info = {
        "CFBundleIdentifier": "app.lintel.desktop",
        "CFBundleExecutable": "lintel-desktop",
        "CFBundleShortVersionString": "0.1.0",
        "CFBundleVersion": "0.1.0",
    }
    (app / "Contents/Info.plist").write_bytes(plistlib.dumps(info))
    (resources / "synthetic-resource.txt").write_text("synthetic\n", encoding="utf-8")
    return app


def main() -> None:
    if sys.platform != "darwin":
        print("SKIP: macOS bundle signature gate is Darwin-only")
        return
    with tempfile.TemporaryDirectory(prefix="lintel-app-signature-") as temp:
        app = fixture(Path(temp))
        rejected = run("node", str(GATE), str(app))
        assert rejected.returncode != 0, "linker-only bundle was accepted"
        assert "linker signature" in rejected.stderr, rejected.stderr

        run("/usr/bin/codesign", "--force", "--deep", "--sign", "-", str(app), check=True)
        accepted = run("node", str(GATE), str(app))
        assert accepted.returncode == 0, accepted.stderr
        result = json.loads(accepted.stdout)
        assert result["signature"] == "ad-hoc", result

        resource = app / "Contents/Resources/synthetic-resource.txt"
        resource.write_text("tampered\n", encoding="utf-8")
        tampered = run("node", str(GATE), str(app))
        assert tampered.returncode != 0, "resource tamper was accepted"
        assert "codesign strict verification failed" in tampered.stderr, tampered.stderr
        resource.write_text("synthetic\n", encoding="utf-8")

        run("/usr/bin/codesign", "--force", "--deep", "--sign", "-", str(app), check=True)
        info = app / "Contents/Info.plist"
        data = plistlib.loads(info.read_bytes())
        data["CFBundleShortVersionString"] = "0.1.1"
        info.write_bytes(plistlib.dumps(data))
        plist_tamper = run("node", str(GATE), str(app))
        assert plist_tamper.returncode != 0, "Info.plist tamper was accepted"

        info.write_bytes(plistlib.dumps({
            "CFBundleIdentifier": "app.lintel.desktop",
            "CFBundleExecutable": "lintel-desktop",
            "CFBundleShortVersionString": "0.1.0",
            "CFBundleVersion": "0.1.0",
        }))
        run("/usr/bin/codesign", "--force", "--deep", "--sign", "-", str(app), check=True)
        outside = Path(temp) / "outside.txt"
        outside.write_text("synthetic\n", encoding="utf-8")
        link = app / "Contents/Resources/outside-link"
        link.symlink_to(outside)
        escape = run("node", str(GATE), str(app))
        assert escape.returncode != 0, "escaping App symlink was accepted"
        assert "symlink escapes" in escape.stderr or "sealed resource" in escape.stderr, escape.stderr
        link.unlink()
        link.symlink_to(Path("synthetic-resource.txt"))
        run("/usr/bin/codesign", "--force", "--deep", "--sign", "-", str(app), check=True)
        contained = run("node", str(GATE), str(app))
        assert contained.returncode == 0, contained.stderr
        cache_alias = Path(temp) / "cache-alias"
        cache_alias.symlink_to(Path(temp), target_is_directory=True)
        through_alias = run("node", str(GATE), str(cache_alias / "Lintel.app"))
        assert through_alias.returncode == 0, through_alias.stderr
        print("PASS: linker-only rejected; complete ad-hoc bundle accepted; resource/plist tamper and escaping symlink rejected")


if __name__ == "__main__":
    main()

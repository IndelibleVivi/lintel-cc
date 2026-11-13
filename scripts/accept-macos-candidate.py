#!/usr/bin/env python3
"""Read the exact DMG, copy into a temporary install, record native/trust facts.

Never changes Applications, quarantine, Gatekeeper, Keychain or signing keys.
Process liveness is explicitly separate from visible-window/task acceptance.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import platform
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("candidate_gate", ROOT / "scripts/prepare-candidate.py")
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


def observation(argv, timeout=120):
    result = subprocess.run([str(item) for item in argv], text=True, capture_output=True, timeout=timeout)
    return {"exit_code": result.returncode, "stdout": result.stdout[:8192], "stderr": result.stderr[:8192]}


def accept(receipt_path, revision):
    gate.require(platform.system() == "Darwin" and platform.machine() == "arm64", "native macOS arm64 required")
    receipt = gate.json_read(receipt_path)
    gate.require(receipt.get("schema") == gate.SCHEMA and receipt.get("source_revision") == revision, "candidate revision mismatch")
    items = [item for item in receipt.get("artifacts", []) if item.get("kind") == "app" and item.get("target") == "macos-arm64"]
    gate.require(len(items) == 1, "exactly one final DMG required")
    item = items[0]
    dmg = receipt_path.parent / gate.safe_name(item["path"])
    gate.require(dmg.resolve().is_relative_to(receipt_path.parent.resolve()), "DMG escapes candidate root")
    gate.require(gate.digest(dmg) == item["artifact"], "DMG bytes differ from build receipt")
    with tempfile.TemporaryDirectory(prefix="lintel-native-app-") as tmp:
        base = Path(tmp).resolve()
        app = base / "Lintel.app"
        actual = gate.dmg_inventory(dmg, ROOT, receipt["app_version"], item["artifact"], install_copy=app)
        gate.require(actual == item["files"], "installed DMG App differs from build receipt")
        signature = observation(["node", ROOT / "apps/desktop/scripts/verify-macos-bundle.mjs", app])
        gate.require(signature["exit_code"] == 0, "installed copy strict signature rejected")
        facts = json.loads(signature["stdout"])
        facts.pop("bundle", None)
        gatekeeper = observation(["/usr/sbin/spctl", "--assess", "--type", "execute", "--verbose=4", app])
        stapler = observation(["/usr/bin/xcrun", "stapler", "validate", app])
        quarantine = observation(["/usr/bin/xattr", "-p", "com.apple.quarantine", app])
        home = base / "home"
        home.mkdir(mode=0o700)
        env = {"PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "HOME": str(home), "CFFIXED_USER_HOME": str(home),
               "LINTEL_TEST_HOME": str(home), "LINTEL_STATE_DIR": str(base / "state"),
               "LINTEL_BROWSER_STATE": str(base / "browser-state"), "TMPDIR": str(base)}
        log = base / "process.log"
        with log.open("wb") as output:
            process = subprocess.Popen([str(app / "Contents/MacOS/lintel-desktop")], cwd=base, env=env, stdout=output, stderr=output)
            try:
                deadline = time.monotonic() + 5
                while process.poll() is None and time.monotonic() < deadline:
                    time.sleep(0.1)
                native = {"alive_after_five_seconds": process.poll() is None, "early_exit_code": process.poll(),
                          "launch": "direct native executable in synthetic HOME", "visible_window": "not_observed"}
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=10)
        # Only synthetic diagnostics, bounded and with temporary path removed.
        with log.open("rb") as stream:
            native["diagnostic"] = stream.read(8192).decode("utf-8", "replace").replace(str(base), "<synthetic-install>")
        for result in [gatekeeper, stapler, quarantine]:
            for key in ["stdout", "stderr"]:
                result[key] = result[key].replace(str(base), "<synthetic-install>")
        return {"schema": "lintel.native-app-acceptance/1", "source_revision": revision,
                "version": receipt["app_version"], "artifact": item["artifact"],
                "plist_versions": {"CFBundleShortVersionString": receipt["app_version"], "CFBundleVersion": receipt["app_version"]},
                "host": {"system": platform.system(), "machine": platform.machine(), "release": platform.release()},
                "package_and_installed_copy_passed": True, "signature": facts,
                "gatekeeper": gatekeeper, "notarization_ticket": stapler, "quarantine": quarantine,
                "native_process": native, "ordinary_download_install_accepted": False,
                "limits": ["temporary copied installation, no Applications replacement",
                           "direct launch does not accept LaunchServices, WebKit interaction or first-download trust",
                           "Gatekeeper rejection is retained evidence; no quarantine removal or security exception was applied"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", required=True, type=Path)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    gate.require(not args.out.exists() and not args.out.is_symlink(), "output must be absent")
    result = accept(args.receipt.absolute(), args.revision)
    gate.publish(args.out, result)
    print(json.dumps(result))
    # A dead native executable is a failed candidate, even if codesign passed.
    return 0 if result["native_process"]["alive_after_five_seconds"] else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError) as error:
        raise SystemExit(f"native App candidate acceptance rejected: {error}")

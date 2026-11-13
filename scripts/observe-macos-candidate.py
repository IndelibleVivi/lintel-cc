#!/usr/bin/env python3
"""Observe a copied exact candidate App without changing privacy or its bundle.

Separates candidate source/artifact identity from this probe's source revision.
Only the spawned synthetic App PID is inspected; no desktop-wide screenshots.
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
import time

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("candidate_gate", ROOT / "scripts/prepare-candidate.py")
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


def normal_windows(observation):
    result = []
    for item in observation.get("on_screen_windows", []):
        bounds = item.get("bounds", {})
        if item.get("layer") == 0 and bounds.get("Width", 0) > 1 and bounds.get("Height", 0) > 1 and item.get("alpha", 1) > 0:
            result.append(item)
    return sorted(result, key=lambda item: item["bounds"]["Width"] * item["bounds"]["Height"], reverse=True)


def observe(receipt_path, revision, probe_revision, out):
    gate.require(platform.system() == "Darwin" and platform.machine() == "arm64", "native macOS arm64 required")
    gate.require(re.fullmatch(r"[a-f0-9]{40}", revision) and re.fullmatch(r"[a-f0-9]{40}", probe_revision), "exact candidate and probe SHAs required")
    receipt = gate.json_read(receipt_path)
    gate.require(receipt.get("schema") == gate.SCHEMA and receipt.get("source_revision") == revision, "candidate revision mismatch")
    items = [item for item in receipt.get("artifacts", []) if item.get("kind") == "app" and item.get("target") == "macos-arm64"]
    gate.require(len(items) == 1, "exactly one final DMG required")
    item = items[0]
    dmg = receipt_path.parent / gate.safe_name(item["path"])
    gate.require(dmg.resolve().is_relative_to(receipt_path.parent.resolve()), "DMG escapes candidate root")
    gate.require(not out.exists() and not out.is_symlink(), "observation output directory must be absent")
    out.mkdir()
    with tempfile.TemporaryDirectory(prefix="lintel-window-observation-") as temporary:
        base = Path(temporary).resolve()
        app = base / "Lintel.app"
        actual = gate.dmg_inventory(dmg, ROOT, receipt["app_version"], item["artifact"], install_copy=app)
        gate.require(actual == item["files"], "installed DMG App differs from build receipt")
        probe = base / "window-observer"
        compiled = subprocess.run(["/usr/bin/xcrun", "swiftc", ROOT / "scripts/macos-window-observer.swift", "-o", probe],
                                  capture_output=True, text=True, timeout=120)
        gate.require(compiled.returncode == 0, "Swift window observer compilation failed: " + compiled.stderr[-8192:])
        home = base / "home"
        home.mkdir(mode=0o700)
        env = {"PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "HOME": str(home), "CFFIXED_USER_HOME": str(home),
               "LINTEL_TEST_HOME": str(home), "LINTEL_STATE_DIR": str(base / "state"),
               "LINTEL_BROWSER_STATE": str(base / "browser-state"), "TMPDIR": str(base)}
        log = base / "app.log"
        observations, screenshot = [], {"status": "no_normal_window_observed"}
        with log.open("wb") as output:
            process = subprocess.Popen([app / "Contents/MacOS/lintel-desktop"], cwd=base, env=env, stdout=output, stderr=output)
            try:
                started = time.monotonic()
                while process.poll() is None and time.monotonic() - started < 25:
                    sample = subprocess.run([probe, str(process.pid)], capture_output=True, text=True, timeout=12)
                    gate.require(sample.returncode == 0 and len(sample.stdout) <= 128 * 1024, "native window observer failed or exceeded output budget")
                    observation = gate.unique_json(sample.stdout)
                    gate.require(observation.get("pid") == process.pid, "window observer PID mismatch")
                    observation["elapsed_seconds"] = round(time.monotonic() - started, 3)
                    observations.append(observation)
                    windows = normal_windows(observation)
                    if windows:
                        # Allow the initial WebKit content to render after the
                        # native shell becomes visible, without sending UI input.
                        time.sleep(3)
                        # Refresh ownership after rendering; never capture an
                        # old window ID after the process/window disappeared.
                        if process.poll() is not None:
                            screenshot = {"status": "app_exited_before_capture"}
                            break
                        refreshed = subprocess.run([probe, str(process.pid)], capture_output=True, text=True, timeout=12)
                        gate.require(refreshed.returncode == 0 and len(refreshed.stdout) <= 128 * 1024, "native window refresh failed")
                        observation = gate.unique_json(refreshed.stdout)
                        gate.require(observation.get("pid") == process.pid, "window refresh PID mismatch")
                        observation["elapsed_seconds"] = round(time.monotonic() - started, 3)
                        observations.append(observation)
                        windows = normal_windows(observation)
                        if not windows:
                            screenshot = {"status": "app_window_disappeared_before_capture"}
                            break
                        screenshot = {"status": "screen_capture_not_preapproved", "window_id": windows[0]["window_id"]}
                        if observation.get("screen_capture_preflight") is True and process.poll() is None:
                            path = out / "lintel-window.png"
                            result = subprocess.run(["/usr/sbin/screencapture", "-x", "-l", str(windows[0]["window_id"]), str(path)],
                                                    capture_output=True, text=True, timeout=20)
                            screenshot.update(exit_code=result.returncode, diagnostic=result.stderr[:2048].replace(str(base), "<synthetic-install>"))
                            if result.returncode == 0 and path.is_file() and path.stat().st_size > 0:
                                screenshot.update(status="captured_selected_lintel_window", file=path.name, artifact=gate.digest(path))
                            else:
                                screenshot["status"] = "capture_failed"
                        break
                    time.sleep(1)
                process_state = {"alive_at_observation_end": process.poll() is None, "early_exit_code": process.poll()}
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=10)
        with log.open("rb") as stream:
            diagnostic = stream.read(8192).decode("utf-8", "replace").replace(str(base), "<synthetic-install>")
        result = {"schema": "lintel.native-window-observation/1", "candidate_source_revision": revision,
                  "probe_source_revision": probe_revision, "version": receipt["app_version"], "artifact": item["artifact"],
                  "host": {"system": platform.system(), "machine": platform.machine(), "release": platform.release()},
                  "installed_copy_inventory_passed": True, "process": process_state, "observations": observations,
                  "normal_window_observed": any(normal_windows(sample) for sample in observations), "screenshot": screenshot,
                  "diagnostic": diagnostic, "normal_control_operation_observed": False,
                  "limits": ["read-only native window/AX observation in synthetic HOME; no UI action or task completion",
                             "screenshot attempted only with existing screen capture approval, and only for spawned App window",
                             "no permission request, privacy override, quarantine change or candidate bundle modification",
                             "visible window is separate from Gatekeeper first-download acceptance"]}
        gate.publish(out / "observation.json", result)
        return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", required=True, type=Path)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--probe-revision", required=True)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    result = observe(args.receipt.absolute(), args.revision, args.probe_revision, args.out.absolute())
    print(json.dumps(result))
    return 0 if result["normal_window_observed"] else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError) as error:
        raise SystemExit(f"native window observation failed: {error}")

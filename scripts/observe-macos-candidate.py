#!/usr/bin/env python3
"""Observe a copied exact candidate App without changing privacy or its bundle.

Separates candidate source/artifact identity from this probe's source revision.
Only the spawned synthetic App PID is observed and receives finite native AX
control actions; no desktop-wide screenshots or product task execution.
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


def interact(process, helper, observer, out):
    """Finite normal-control actions, separate from the read-only observer."""
    out.mkdir()
    actions, checks = [], []
    class OutcomeTimeout(ValueError):
        def __init__(self, state):
            super().__init__("native AX action did not produce its expected visible state")
            self.state = state
    def invoke(command, *arguments):
        gate.require(process.poll() is None, "App exited during native interaction")
        result = subprocess.run([helper, str(process.pid), command, *arguments], capture_output=True, text=True, timeout=15)
        gate.require(result.returncode == 0 and len(result.stdout) <= 512 * 1024, "bounded native AX helper failed")
        value = gate.unique_json(result.stdout)
        gate.require(value.get("pid") == process.pid, "AX helper PID differs")
        return value
    def matches(state, role, title):
        return [item for item in state.get("elements", []) if item.get("role") == role and item.get("title") == title]
    def selected(state, title):
        found = matches(state, "AXCheckBox", title)
        return len(found) == 1 and found[0].get("value") in [True, 1, "1", "true"]
    def press(role, title):
        result = invoke("press", role, title)
        actions.append(result)
        if not result.get("performed"):
            if "error_code" in result:
                raise ValueError(f"native AX action failed: {role} / {title}, error {result['error_code']}")
            raise LookupError(f"AX action unavailable: {role} / {title}")
    def wait_for(condition):
        deadline = time.monotonic() + 6
        while True:
            state = invoke("snapshot")
            if condition(state): return state
            if time.monotonic() >= deadline: raise OutcomeTimeout(state)
            time.sleep(0.25)
    def retain(name, state):
        gate.publish(out / (name + ".json"), state)
        # Fresh read-only ownership and permission observation before each
        # selected-window screenshot; never use a screen/global capture.
        sample = subprocess.run([observer, str(process.pid)], capture_output=True, text=True, timeout=12)
        gate.require(sample.returncode == 0 and len(sample.stdout) <= 128 * 1024, "window ownership observation failed")
        observed = gate.unique_json(sample.stdout)
        gate.require(observed.get("pid") == process.pid, "capture PID differs")
        windows = normal_windows(observed)
        capture = {"status": "unavailable"}
        if windows and observed.get("screen_capture_preflight") is True and process.poll() is None:
            time.sleep(0.5)
            path = out / (name + ".png")
            result = subprocess.run(["/usr/sbin/screencapture", "-x", "-l", str(windows[0]["window_id"]), path], capture_output=True, timeout=20)
            if result.returncode == 0 and path.is_file() and path.stat().st_size:
                capture = {"status": "captured", "file": path.name, "artifact": gate.digest(path)}
            else: capture = {"status": "capture_failed", "exit_code": result.returncode}
        return capture
    def check(name, operation):
        try:
            state = operation()
            checks.append({"name": name, "status": "passed", "capture": retain(name, state)})
        except LookupError as error:
            record = {"name": name, "status": "unsupported", "reason": str(error)}
            try: record["capture"] = retain(name + "-unsupported", invoke("snapshot"))
            except (ValueError, OSError, subprocess.SubprocessError) as diagnostic: record["evidence_error"] = str(diagnostic)
            checks.append(record)
        except (ValueError, OSError, subprocess.SubprocessError) as error:
            record = {"name": name, "status": "failed", "reason": str(error)}
            try: record["capture"] = retain(name + "-failed", invoke("snapshot"))
            except (ValueError, OSError, subprocess.SubprocessError) as diagnostic: record["evidence_error"] = str(diagnostic)
            checks.append(record)
    original = invoke("snapshot")
    gate.publish(out / "initial-state.json", original)
    if not original.get("trusted"):
        report = {"status": "unsupported", "reason": "AX client not already trusted", "checks": [], "actions": []}
        gate.publish(out / "report.json", report)
        return report
    theme = next((name for name in ["Light Day", "Dark Night", "Follow system"] if selected(original, name)), None)
    language = matches(original, "AXPopUpButton", "Interface language")
    language_value = language[0].get("value") if len(language) == 1 else None
    def theme_change(title):
        press("AXCheckBox", title)
        return wait_for(lambda state: selected(state, title))
    if theme:
        alternate_theme = "Light Day" if theme == "Dark Night" else "Dark Night"
        check("theme-change", lambda: theme_change(alternate_theme))
        check("theme-restored", lambda: theme_change(theme))
    else: checks.append({"name": "theme", "status": "unsupported", "reason": "original selected theme could not be read"})
    def language_change(control_title, option, condition):
        press("AXPopUpButton", control_title)
        try:
            wait_for(lambda state: bool(matches(state, "AXMenuItem", option)))
        except OutcomeTimeout as error:
            if not any(item.get("role") == "AXMenuItem" for item in error.state.get("elements", [])):
                raise LookupError("native popup menu is not exposed within this App's AX windows scope") from error
            raise  # A visible accessible menu with the wrong items is a mismatch.
        press("AXMenuItem", option)
        return wait_for(condition)
    def navigate(title, heading):
        press("AXButton", title)
        return wait_for(lambda state: bool(matches(state, "AXHeading", heading)))
    check("history", lambda: navigate("Records & recovery", "Every step, on record"))
    def work_needs_target():
        press("AXButton", "Preserve work")
        return wait_for(lambda state: bool(matches(state, "AXHeading", "Choose a specific environment first")) and bool(matches(state, "AXButton", "Add existing environment")))
    check("work-needs-target", work_needs_target)
    def home():
        press("AXButton", "Environments")
        return wait_for(lambda state: bool(matches(state, "AXButton", "Start a task")))
    check("home-restored", home)
    def task_dialog():
        press("AXButton", "Start a task")
        return wait_for(lambda state: bool(matches(state, "AXHeading", "Start a task")) and bool(matches(state, "AXButton", "Close panel")))
    check("task-chooser", task_dialog)
    def close_dialog():
        press("AXButton", "Close panel")
        return wait_for(lambda state: not matches(state, "AXButton", "Close panel") and bool(matches(state, "AXButton", "Start a task")))
    check("task-chooser-closed", close_dialog)
    # Run language last: an OS popup outside the permitted AX tree must not
    # prevent the independent normal navigation checks above.
    if language_value in ["Follow system", "English"]:
        check("chinese", lambda: language_change("Interface language", "简体中文", lambda state: bool(matches(state, "AXButton", "记录与恢复")) and bool(matches(state, "AXButton", "工作保全"))))
        current = invoke("snapshot")
        if matches(current, "AXPopUpButton", "界面语言"):
            option = "跟随系统" if language_value == "Follow system" else "English"
            check("language-restored", lambda: language_change("界面语言", option, lambda state: len(matches(state, "AXPopUpButton", "Interface language")) == 1 and matches(state, "AXPopUpButton", "Interface language")[0].get("value") == language_value))
        else:
            # A popup unsupported by AX may still be open. Cancellation is a
            # finite supported AX action, never a global Escape key.
            cancelled = invoke("cancel", "AXPopUpButton", "Interface language")
            actions.append(cancelled)
            current = invoke("snapshot")
            unchanged = matches(current, "AXPopUpButton", "Interface language")
            checks.append({"name": "language-restored", "status": "passed" if len(unchanged) == 1 and unchanged[0].get("value") == language_value else "failed", "reason": "verified original value after unsupported/unchanged selection"})
    else: checks.append({"name": "language", "status": "unsupported", "reason": "original English/system language value could not be read", "observed_value": language_value})
    statuses = {item["status"] for item in checks}
    report = {"status": "failed" if "failed" in statuses else "partial" if "unsupported" in statuses else "passed",
              "original_theme": theme, "original_language_value": language_value, "actions": actions, "checks": checks,
              "scope": "native AX normal language/theme/navigation/dialog controls on one synthetic App; actual resulting states required",
              "limits": ["no archive, approval, cleanup, SSH or task execution was performed", "no global input, WebKit injection or permission request"]}
    gate.publish(out / "report.json", report)
    return report


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
        action_helper = base / "ax-action"
        compiled_actions = subprocess.run(["/usr/bin/xcrun", "swiftc", ROOT / "scripts/macos-ax-action.swift", "-o", action_helper], capture_output=True, text=True, timeout=120)
        home = base / "home"
        home.mkdir(mode=0o700)
        env = {"PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "HOME": str(home), "CFFIXED_USER_HOME": str(home),
               "LINTEL_TEST_HOME": str(home), "LINTEL_STATE_DIR": str(base / "state"),
               "LINTEL_BROWSER_STATE": str(base / "browser-state"), "TMPDIR": str(base)}
        log = base / "app.log"
        observations, screenshot = [], {"status": "no_normal_window_observed"}
        interaction = {"status": "unsupported", "reason": "no normal window", "checks": []}
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
                        if compiled_actions.returncode == 0:
                            try:
                                interaction = interact(process, action_helper, probe, out / "interactions")
                            except (ValueError, OSError, subprocess.SubprocessError) as error:
                                interaction = {"status": "failed", "reason": str(error), "checks": []}
                        else:
                            interaction = {"status": "failed", "reason": "native AX action helper compilation failed", "diagnostic": compiled_actions.stderr[-8192:], "checks": []}
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
                  "diagnostic": diagnostic, "interaction": interaction,
                  "normal_control_operation_observed": any(check.get("status") == "passed" and check.get("name") in {"theme-change", "chinese", "history", "work-needs-target", "task-chooser"} for check in interaction.get("checks", [])),
                  "limits": ["read-only window observation and separate finite native control experiment in synthetic HOME; no task completion",
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
    return 0 if result["normal_window_observed"] and result["interaction"]["status"] != "failed" else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError) as error:
        raise SystemExit(f"native window observation failed: {error}")

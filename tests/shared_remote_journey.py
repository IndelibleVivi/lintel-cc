#!/usr/bin/env python3
"""Shared SSH schema/fake-transport checks and public API HOME on synthetic roots.

Runs only the shared crate's finite regression tests. No OpenSSH connection or
operator settings/profile/service is used. Rust transport fixtures inject fake
SSH privately; the public API checks only aliases and the local host registry.
"""
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def run():
    with tempfile.TemporaryDirectory(prefix="lintel-shared-remote-") as temporary:
        base = Path(temporary).resolve()
        home = base / "home"
        (home / ".ssh").mkdir(parents=True)
        (home / ".ssh/config").write_text("Host synthetic-current-home\n  HostName synthetic.invalid\n")
        other = base / "other-home"
        (other / ".ssh").mkdir(parents=True)
        (other / ".ssh/config").write_text("Host synthetic-other-home\n")
        # Keep dependency/toolchain caches at their installed locations while the
        # controller sees only this explicit synthetic current-user HOME.
        env = dict(os.environ)
        installed_home = Path.home()
        env.setdefault("CARGO_HOME", str(installed_home / ".cargo"))
        env.setdefault("RUSTUP_HOME", str(installed_home / ".rustup"))
        env.update(HOME=str(home), LINTEL_TEST_HOME=str(other),
                   LINTEL_REMOTE_SYNTHETIC_FIXTURE=str(base))
        env.pop("LINTEL_STATE_DIR", None)
        command = ["cargo", "test", "--manifest-path", "crates/remote/Cargo.toml", "--lib"]
        subprocess.run(command + ["shared_contract_", "--", "--include-ignored", "--nocapture"],
                       cwd=ROOT, env=env, check=True)
        # The supported state override is a separate process contract, so verify
        # it in a fresh runtime without mutating the test process environment.
        env["LINTEL_STATE_DIR"] = str(base / "explicit-state")
        subprocess.run(command + ["tests::shared_contract_system_context_uses_current_home_and_explicit_resources",
                                  "--", "--exact", "--ignored", "--nocapture"],
                       cwd=ROOT, env=env, check=True)
        print("PASS: shared finite schema, plan/job/archive stdin transport, current HOME and explicit state/resources")


if __name__ == "__main__":
    run()

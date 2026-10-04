#!/usr/bin/env python3
"""Finite probes for the disposable VM, never an installed Lintel component."""
import argparse
import json
import os
from pathlib import Path
import pwd
import signal
import subprocess

TARGET = "lintel-fixture"
BASE = Path("/home") / TARGET / "vm-journey"
RUNNER = Path("/opt/lintel-vm/lintel")


def command(args, check=True):
    result = subprocess.run(args, text=True, capture_output=True)
    if check and result.returncode:
        raise RuntimeError(f"{args[0]} failed ({result.returncode}): {result.stderr}")
    return result


def properties(text):
    return dict(line.split("=", 1) for line in text.splitlines() if "=" in line)


def synthetic(path):
    path = Path(path)
    if not path.resolve().is_relative_to(BASE):
        raise RuntimeError("Probe files must belong to the disposable VM fixture")
    return path


def session(session_id=None):
    session_id = session_id or os.environ.get("XDG_SESSION_ID")
    if not session_id:
        raise RuntimeError("SSH did not establish an XDG/PAM login session")
    result = command(["loginctl", "show-session", session_id, "--no-pager", "--all"], check=False)
    return {"id": session_id, "present": result.returncode == 0,
            "properties": properties(result.stdout),
            "process_cgroup": Path("/proc/self/cgroup").read_text(),
            "boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip()}


def process(marker):
    marker = json.loads(synthetic(marker).read_text())
    pid = int(marker["pid"])
    proc = Path("/proc") / str(pid)
    state, cgroup, matches = None, None, False
    try:
        # comm may contain spaces; the state follows its final closing paren.
        state = (proc / "stat").read_text().rsplit(")", 1)[1].split()[0]
        cgroup = (proc / "cgroup").read_text()
        matches = Path(os.readlink(proc / "exe")) == RUNNER and b"__worker" in (proc / "cmdline").read_bytes().split(b"\0")
    except (FileNotFoundError, ProcessLookupError):
        pass
    return {"marker": marker, "alive": matches and state != "Z", "state": state,
            "runner_identity_matches": matches, "current_cgroup": cgroup,
            "current_boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip()}


def facts():
    if os.geteuid() != 0:
        raise RuntimeError("VM runtime facts require its synthetic observer's sudo")
    effective = command(["/usr/sbin/sshd", "-T"]).stdout
    pam = {str(path): path.read_text() for path in (
        Path("/etc/pam.d/sshd"), Path("/etc/pam.d/common-session"))}
    packages = command(["dpkg-query", "-W", "-f=${Package}\t${Version}\n", "systemd", "openssh-server", "libpam-systemd", "cloud-init", "python3"]).stdout
    return {"os_release": Path("/etc/os-release").read_text(),
            "kernel": command(["uname", "-srmo"]).stdout.strip(),
            "boot_id": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
            "pid1": Path("/proc/1/comm").read_text().strip(),
            "sshd_use_pam": "usepam yes" in effective.splitlines(),
            "pam_systemd_configured": any("pam_systemd.so" in text for text in pam.values()),
            "pam_sshd_includes_common_session": "@include common-session" in pam["/etc/pam.d/sshd"],
            "logind_active": command(["systemctl", "is-active", "systemd-logind"]).stdout.strip(),
            "kill_user_processes": command(["busctl", "get-property", "org.freedesktop.login1", "/org/freedesktop/login1", "org.freedesktop.login1.Manager", "KillUserProcesses"]).stdout.strip(),
            "fixture_linger": properties(command(["loginctl", "show-user", TARGET, "-p", "Linger"], check=False).stdout).get("Linger"),
            "package_versions": properties(packages.replace("\t", "="))}


def configure_logind(kill):
    if os.geteuid() != 0 or not Path("/etc/lintel-vm-fixture").is_file():
        raise RuntimeError("Logind policy changes are allowed only inside the disposable fixture VM")
    path = Path("/etc/systemd/logind.conf.d/90-lintel-vm-fixture.conf")
    path.parent.mkdir(exist_ok=True)
    path.write_text(f"[Login]\nKillUserProcesses={kill}\nKillOnlyUsers={TARGET}\nKillExcludeUsers=\n")
    command(["systemctl", "restart", "systemd-logind"])
    return facts()


def prepare(case):
    if pwd.getpwuid(os.getuid()).pw_name != TARGET or case not in ("logout-retain", "logout-kill", "reboot-interrupt"):
        raise RuntimeError("Case preparation requires the VM's synthetic target user")
    base = BASE / case
    base.mkdir(parents=True, exist_ok=False)
    base.chmod(0o700)
    root = base / "root"
    root.mkdir()
    (root / "settings.json").write_text(json.dumps({"env": {"SYNTHETIC_VM_NEIGHBOR": "keep"}}))
    return {"home": str(Path.home()), "base": str(base), "root": str(root),
            "state": str(base / "state"), "barrier": str(base / "barrier.json"),
            "session": str(base / "session.json")}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("facts", "session", "configure-logind", "prepare", "process", "continue", "read-json"))
    parser.add_argument("value", nargs="?")
    parser.add_argument("--json", type=Path)
    args = parser.parse_args()
    if args.operation == "facts":
        result = facts()
    elif args.operation == "session":
        result = session(args.value)
        if args.json:
            synthetic(args.json).write_text(json.dumps(result))
            return
    elif args.operation == "configure-logind":
        if args.value not in ("yes", "no"):
            raise RuntimeError("Expected one explicit fixture logout policy")
        result = configure_logind(args.value)
    elif args.operation == "prepare":
        result = prepare(args.value)
    elif args.operation == "process":
        result = process(args.value)
    elif args.operation == "continue":
        result = process(args.value)
        if not result["alive"] or result["state"] not in ("T", "t"):
            raise RuntimeError("The recorded synthetic runner is no longer stopped")
        os.kill(int(result["marker"]["pid"]), signal.SIGCONT)
        result["continued"] = True
    else:
        result = json.loads(synthetic(args.value).read_text())
    print(json.dumps(result))


if __name__ == "__main__":
    main()

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
    if not path.resolve().is_relative_to(BASE.resolve()):
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


def process(marker, proc_root=Path("/proc"), boot_id_path=Path("/proc/sys/kernel/random/boot_id")):
    """Finite /proc identity facts. Never collapses absence into "killed": the
    classification names what was actually observed so a caller cannot mistake an
    unreadable pid for a signal-confirmed death.

    /proc/<pid>/stat is read first and decides presence. A zombie has no readable
    /proc/<pid>/exe or cmdline by kernel design, so its missing link must NOT be
    reported as "missing"; a zombie is classified from its stat state alone."""
    marker = json.loads(synthetic(marker).read_text())
    pid = int(marker["pid"])
    proc = Path(proc_root) / str(pid)
    try:
        # comm may contain spaces; state/ppid/pgrp/session/... starttime follow the
        # final ")".
        rest = (proc / "stat").read_text().rsplit(")", 1)[1].split()
        state = rest[0]
        identity = {"ppid": int(rest[1]), "pgrp": int(rest[2]), "session": int(rest[3]),
                    "starttime": int(rest[19])}
    except (FileNotFoundError, ProcessLookupError):
        return {"marker": marker, "present": False, "state": None, "identity": {},
                "exe": None,  "runner_identity_matches": False,
                "current_cgroup": None, "alive": False, "zombie": False,
                "classification": "missing",
                "current_boot_id": boot_id_path.read_text().strip()}
    if state == "Z":
        # A zombie keeps its stat identity but has no exe link and cannot be a
        # running worker; record the absence as a fact, not a disappearance.
        cgroup = read_optional(proc / "cgroup")
        return {"marker": marker, "present": True, "state": state, "identity": identity,
                "exe": None,  "runner_identity_matches": False,
                "current_cgroup": cgroup, "alive": False, "zombie": True,
                "classification": "zombie",
                "current_boot_id": boot_id_path.read_text().strip()}
    cgroup = read_optional(proc / "cgroup")
    exe = read_optional_link(proc / "exe")
    cmdline = read_optional_bytes(proc / "cmdline")
    matches = (exe == str(RUNNER) and cmdline is not None and b"__worker" in cmdline.split(b"\0")
               and identity["starttime"] == marker["starttime"]
               and boot_id_path.read_text().strip() == marker["boot_id"])
    if matches and state in ("T", "t"):
        classification = "stopped"
    elif matches:
        classification = "running"
    else:
        classification = "identity_mismatch"
    return {"marker": marker, "present": True, "state": state, "identity": identity,
            "exe": exe,  "runner_identity_matches": matches,
            "current_cgroup": cgroup, "alive": matches and state != "Z", "zombie": False,
            "classification": classification,
            "current_boot_id": boot_id_path.read_text().strip()}

def read_optional(path):
    try:
        return path.read_text()
    except (FileNotFoundError, ProcessLookupError, PermissionError, IsADirectoryError):
        return None

def read_optional_link(path):
    try:
        return os.readlink(path)
    except OSError:
        return None

def read_optional_bytes(path):
    try:
        return path.read_bytes()
    except OSError:
        return None

def release(path):
    """Explicitly unblock a held (running) synthetic worker. The path must resolve
    inside the disposable fixture base, so a probe can never touch another path."""
    release_path = synthetic(path)
    if release_path.exists():
        return {"released": False, "already_present": True, "release": str(release_path)}
    release_path.write_text("released\n")
    return {"released": True, "already_present": False, "release": str(release_path)}

def session_evidence(session_id):
    """Bounded evidence for the EXACT original PAM session, never the observer's.
    Two finite sources, both keyed to the original session id:
      * the session's own scope unit journal (what the scope did when it ended);
      * logind messages that name this session id (what logind decided and when).
    No observer-user unit listing and no unrelated-session log is read."""
    scope = f"session-{session_id}.scope"
    unit = command(["journalctl", "-b", "-n", "40", "--no-pager", "-u", scope], check=False)
    logind = command(["journalctl", "-b", "-n", "40", "--no-pager", "_COMM=systemd-logind",
                      "--grep", rf"(?i)\bsession\s+{session_id}\b"], check=False)
    scope_result = command(["systemctl", "show", scope, "--no-pager",
                            "-p", "ActiveState", "-p", "SubState", "-p", "LoadState"], check=False)
    return {"session_id": str(session_id), "scope_unit": scope,
            "scope_properties": properties(scope_result.stdout),
            "scope_journal": unit.stdout.splitlines()[-40:],
            "logind_session_lines": logind.stdout.splitlines()[-40:],
            "journal_status": {"scope": unit.returncode, "logind": logind.returncode}}


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


# The finite set of synthetic cases the launcher may prepare. Each logout policy
# is observed with a frozen worker and with a live (held) worker, so all four
# logout combinations are explicit.
FIXTURE_CASES = (
    "logout-retain-running", "logout-retain-stopped",
    "logout-kill-running", "logout-kill-stopped",
    "reboot-interrupt",
)

def prepare(case, uid=None, username=None, home=None, base=None):
    if uid is None:
        uid = os.getuid()
    if username is None:
        username = pwd.getpwuid(uid).pw_name
    if username != TARGET or case not in FIXTURE_CASES:
        raise RuntimeError("Case preparation requires the VM's synthetic target user")
    if home is None:
        home = Path.home()
    if base is None:
        base = BASE / case
    base.mkdir(parents=True, exist_ok=False)
    base.chmod(0o700)
    root = base / "root"
    root.mkdir()
    (root / "settings.json").write_text(json.dumps({"env": {"SYNTHETIC_VM_NEIGHBOR": "keep"}}))
    return {"home": str(home), "base": str(base), "root": str(root),
            "state": str(base / "state"), "barrier": str(base / "barrier.json"),
            "release": str(base / "release.json"), "session": str(base / "session.json")}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("facts", "session", "session-evidence", "configure-logind",
                                              "prepare", "process", "continue", "release", "read-json"))
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
    elif args.operation == "release":
        result = release(args.value)
    elif args.operation == "session-evidence":
        result = session_evidence(args.value)
    else:
        result = json.loads(synthetic(args.value).read_text())
    print(json.dumps(result))


if __name__ == "__main__":
    main()

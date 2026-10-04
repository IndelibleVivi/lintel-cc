#!/usr/bin/env python3
"""Disposable Ubuntu VM acceptance for systemd, PAM logout and guest reboot.

Ubuntu host prerequisites (installed by CI, not by this script):
  qemu-system-x86 qemu-utils cloud-image-utils openssh-client gpgv ubuntu-keyring
Build the x86_64 musl runner, then select this independent acceptance explicitly:
  python3 tests/linux_vm_journey.py --runner target/x86_64-unknown-linux-musl/release/lintel --json /tmp/lintel-vm-report.json

Only the official signed Ubuntu OS image is cached. Every VM overlay, cloud-init
seed, SSH key and guest fixture is disposable. QEMU binds SSH to host loopback;
the guest cannot access the Internet. This never changes host services or login policy.
"""
import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import selectors
import shlex
import shutil
import signal
import socket
import struct
import subprocess
import sys
import tempfile
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
PROBE_SOURCE = ROOT / "tests/fixtures/linux_vm/runtime_probe.py"
SERVICE_SOURCE = ROOT / "tests/service_systemd_journey.py"
IMAGE_NAME = "ubuntu-24.04-server-cloudimg-amd64.img"
MANIFEST_NAME = "ubuntu-24.04-server-cloudimg-amd64.manifest"
ADMIN, TARGET = "lintel-admin", "lintel-fixture"
RUNNER = "/opt/lintel-vm/lintel"
PROBE = "/opt/lintel-vm/runtime_probe.py"
SERVICE = "/opt/lintel-vm/service_systemd_journey.py"
SERVICE_SUITE_REPORT = "/var/tmp/lintel-vm-service-suite.json"
SERVICE_PREPARE_REPORT = "/var/tmp/lintel-vm-service-prepare.json"
SERVICE_RECOVER_REPORT = "/var/tmp/lintel-vm-service-recover.json"


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def run(command, *, data=None, timeout=60, check=True):
    result = subprocess.run(command, input=data, capture_output=True, timeout=timeout)
    if check and result.returncode:
        raise RuntimeError(f"{Path(command[0]).name} failed ({result.returncode}): {result.stderr.decode(errors='replace')}{result.stdout.decode(errors='replace')}")
    return result


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download(url, path, expected=None):
    if path.is_file() and (expected is None or sha256(path) == expected):
        return
    partial = path.with_name(path.name + ".part")
    offset = partial.stat().st_size if partial.exists() else 0
    request = urllib.request.Request(url, headers={"Range": f"bytes={offset}-"} if offset else {})
    print(f"Download official Ubuntu artifact: {path.name}", flush=True)
    with urllib.request.urlopen(request, timeout=60) as source:
        mode = "ab" if offset and source.status == 206 else "wb"
        with partial.open(mode) as target:
            shutil.copyfileobj(source, target, length=1024 * 1024)
    if expected is not None:
        require(sha256(partial) == expected, f"Official artifact checksum mismatch: {path.name}; the existing verified image was not used")
    partial.replace(path)


def outside_repo(path, description):
    path = path.expanduser().resolve()
    require(not path.is_relative_to(ROOT), f"{description} must be outside the Git tree")
    return path


def image(args, tools):
    require(re.fullmatch(r"\d{8}", args.image_build), "Image build must be one explicit Ubuntu release serial (YYYYMMDD)")
    cache = outside_repo(args.cache_dir, "Ubuntu image cache") / args.image_build
    cache.mkdir(parents=True, exist_ok=True)
    url = f"https://cloud-images.ubuntu.com/releases/noble/release-{args.image_build}"
    for name in ("SHA256SUMS", "SHA256SUMS.gpg"):
        download(url + "/" + name, cache / name)
    # System public keyring only: no import, personal GPG state or trust edits.
    verifier_home = cache / "gpgv-home"
    verifier_home.mkdir(mode=0o700, exist_ok=True)
    verified = run([tools["gpgv"], "--homedir", str(verifier_home), "--status-fd", "1", "--keyring", str(args.keyring), str(cache / "SHA256SUMS.gpg"), str(cache / "SHA256SUMS")])
    fingerprints = re.findall(r"\[GNUPG:\] VALIDSIG ([0-9A-F]+)", verified.stdout.decode())
    require(fingerprints, "Ubuntu checksum signature did not expose a valid signing fingerprint")
    checksums = {}
    for line in (cache / "SHA256SUMS").read_text().splitlines():
        digest, name = line.split(maxsplit=1)
        checksums[name.lstrip(" *")] = digest
    for name in (IMAGE_NAME, MANIFEST_NAME):
        require(name in checksums, f"Signed Ubuntu checksum manifest does not contain {name}")
        download(url + "/" + name, cache / name, checksums[name])
    packages = dict(line.split(maxsplit=1) for line in (cache / MANIFEST_NAME).read_text().splitlines() if line.strip())
    return cache / IMAGE_NAME, {
        "distribution": "Ubuntu 24.04 LTS (noble)", "release_build": args.image_build,
        "source_url": url + "/" + IMAGE_NAME, "checksums_url": url + "/SHA256SUMS",
        "signature_url": url + "/SHA256SUMS.gpg", "signature_verified": True,
        "signing_fingerprints": fingerprints, "image_sha256": checksums[IMAGE_NAME],
        "package_manifest_sha256": checksums[MANIFEST_NAME],
        "manifest_packages": {name: packages.get(name) for name in ("systemd", "openssh-server", "libpam-systemd", "cloud-init", "python3")},
    }


def runner_identity(path):
    with path.open("rb") as source:
        header = source.read(64)
        require(len(header) == 64 and header[:6] == b"\x7fELF\x02\x01" and struct.unpack_from("<H", header, 18)[0] == 62,
                "Runner must be the real Linux x86_64 ELF64 artifact")
        offset = struct.unpack_from("<Q", header, 32)[0]
        size, count = struct.unpack_from("<HH", header, 54)
        source.seek(offset)
        table = source.read(size * count)
        require(size >= 4 and len(table) == size * count, "Runner has an invalid ELF program header table")
        require(all(struct.unpack_from("<I", table, size * i)[0] != 3 for i in range(count)), "Runner needs a dynamic loader; supply the static musl target")
    return sha256(path)


class Guest:
    def __init__(self, directory, port, tools):
        self.directory, self.port, self.tools = directory, port, tools

    def argv(self, user, command):
        return [self.tools["ssh"], "-F", "/dev/null", "-T", "-i", str(self.directory / "client-key"),
                "-oBatchMode=yes", "-oIdentitiesOnly=yes", "-oStrictHostKeyChecking=yes", "-oUpdateHostKeys=no",
                "-oUserKnownHostsFile=" + str(self.directory / "known_hosts"), "-oGlobalKnownHostsFile=/dev/null",
                "-oPermitLocalCommand=no", "-oProxyCommand=none", "-oRemoteCommand=none", "-oClearAllForwardings=yes",
                "-oRequestTTY=no", "-oConnectTimeout=5", "-oConnectionAttempts=1", "-oServerAliveInterval=10", "-oServerAliveCountMax=2",
                "-p", str(self.port), f"{user}@127.0.0.1", command]

    def shell(self, command, *, user=ADMIN, data=None, timeout=60, check=True):
        return run(self.argv(user, command), data=data, timeout=timeout, check=check)

    def probe(self, operation, value=None, *, user=ADMIN):
        argv = ["python3", PROBE, operation] + ([] if value is None else [str(value)])
        if user == ADMIN:
            argv = ["sudo", "-n"] + argv
        return json.loads(self.shell(shlex.join(argv), user=user).stdout)

    def request(self, case, payload, *, submit=False, barrier=None, user=None, extra_env=None):
        user = user or (ADMIN if case.get("as_root") else TARGET)
        prefix = ["sudo", "-n"] if case.get("as_root") else []
        env = {"HOME": case["home"], "LINTEL_TEST_HOME": case["home"], "LINTEL_STATE_DIR": case["state"]}
        if barrier == "stopped":
            env["LINTEL_TEST_ACCEPT_BARRIER"] = case["barrier"]
        elif barrier == "running":
            env["LINTEL_TEST_WAIT_BARRIER"] = case["barrier"]
            env["LINTEL_TEST_WAIT_RELEASE"] = case["release"]
        env.update(extra_env or {})
        command = shlex.join(prefix + ["env"] + [f"{key}={value}" for key, value in env.items()] + [RUNNER, "submit" if submit else "request"])
        if submit:
            session_command = shlex.join(prefix + ["python3", PROBE, "session"]) + ' "$XDG_SESSION_ID" ' + shlex.join(["--json", case["session"]])
            command = session_command + " && exec " + command
        result = self.shell(command, user=user, data=json.dumps(payload).encode(), timeout=60)
        response = json.loads(result.stdout)
        require(response.get("ok") is True, f"Canonical runner rejected {payload['command']}: {response}")
        return response["data"]

    def kept_submission(self, case, payload, *, barrier="stopped"):
        env = [f"HOME={case['home']}", f"LINTEL_TEST_HOME={case['home']}", f"LINTEL_STATE_DIR={case['state']}"]
        if barrier == "stopped":
            env.append(f"LINTEL_TEST_ACCEPT_BARRIER={case['barrier']}")
        else:
            env += [f"LINTEL_TEST_WAIT_BARRIER={case['barrier']}", f"LINTEL_TEST_WAIT_RELEASE={case['release']}"]
        command = shlex.join(["python3", PROBE, "session", "--json", case["session"]]) + " && " + shlex.join(["env"] + env + [RUNNER, "submit"]) + "; sleep 600"
        child = subprocess.Popen(self.argv(TARGET, command), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            child.stdin.write(json.dumps(payload).encode())
            child.stdin.close()
            started, output = time.monotonic(), b""
            with selectors.DefaultSelector() as ready:
                ready.register(child.stdout, selectors.EVENT_READ)
                while b"\n" not in output:
                    require(time.monotonic() - started < 60, "Kept PAM session did not receive durable ACK")
                    for key, _ in ready.select(timeout=1):
                        chunk = os.read(key.fileobj.fileno(), 65536)
                        if not chunk:
                            diagnostic = child.stderr.read().decode(errors="replace") if child.poll() is not None else "SSH stdout closed"
                            raise RuntimeError("Kept PAM submission ended before ACK: " + diagnostic)
                        output += chunk
            ack = json.loads(output.split(b"\n", 1)[0])
            require(ack.get("ok") is True, f"Kept durable submission failed: {ack}")
            return child, ack["data"]
        except BaseException:
            child.terminate()
            child.wait(timeout=10)
            raise

    def upload(self, path, destination, *, executable=False):
        temporary = "/home/lintel-admin/upload-" + Path(destination).name
        self.shell("umask 077; cat > " + shlex.quote(temporary), data=path.read_bytes(), timeout=120)
        self.shell(shlex.join(["sudo", "-n", "install", "-m", "755" if executable else "644", temporary, destination]) + " && rm " + shlex.quote(temporary))


def seed(directory, port, tools):
    for name in ("client-key", "host-key"):
        run([tools["ssh-keygen"], "-q", "-t", "ed25519", "-N", "", "-C", "lintel-disposable-vm", "-f", str(directory / name)])
    public = (directory / "client-key.pub").read_text().strip()
    users = [{"name": ADMIN, "shell": "/bin/bash", "lock_passwd": True, "sudo": "ALL=(ALL) NOPASSWD:ALL", "ssh_authorized_keys": [public]},
             {"name": TARGET, "shell": "/bin/bash", "lock_passwd": True, "ssh_authorized_keys": [public]}]
    config = {"users": users, "disable_root": True, "ssh_pwauth": False, "ssh_deletekeys": True,
              "ssh_keys": {"ed25519_private": (directory / "host-key").read_text(), "ed25519_public": (directory / "host-key.pub").read_text()},
              "ssh_publish_hostkeys": {"enabled": False}, "ssh_quiet_keygen": True,
              "package_update": False, "package_upgrade": False,
              "write_files": [{"path": "/etc/ssh/sshd_config.d/10-lintel-vm.conf", "content": "UsePAM yes\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nAllowUsers lintel-admin lintel-fixture\nAllowTcpForwarding no\nAllowAgentForwarding no\nX11Forwarding no\n", "permissions": "0644"},
                              {"path": "/etc/lintel-vm-fixture", "content": "Disposable Lintel VM only\n", "permissions": "0600"}]}
    # JSON is valid YAML and avoids requiring PyYAML in the host harness.
    (directory / "user-data").write_text("#cloud-config\n" + json.dumps(config))
    (directory / "meta-data").write_text(json.dumps({"instance-id": "lintel-vm-" + directory.name, "local-hostname": "lintel-disposable"}))
    (directory / "network-config").write_text(json.dumps({"version": 2, "ethernets": {"fixture": {"match": {"macaddress": "52:54:00:4c:49:4e"}, "set-name": "eth0", "dhcp4": True}}}))
    run([tools["cloud-localds"], "--network-config", str(directory / "network-config"), str(directory / "seed.img"), str(directory / "user-data"), str(directory / "meta-data")])
    (directory / "known_hosts").write_text(f"[127.0.0.1]:{port} " + (directory / "host-key.pub").read_text())


@contextmanager
def vm(base_image, args, tools, report):
    temporary = tempfile.TemporaryDirectory(prefix="lintel-vm-")
    try:
        directory = Path(temporary.name).resolve()
        with socket.socket() as available:
            available.bind(("127.0.0.1", 0))
            port = available.getsockname()[1]
        seed(directory, port, tools)
        overlay = directory / "guest.qcow2"
        run([tools["qemu-img"], "create", "-f", "qcow2", "-F", "qcow2", "-b", str(base_image), str(overlay), "12G"])
        accel = args.accel
        if accel == "auto":
            accel = "kvm" if os.access("/dev/kvm", os.R_OK | os.W_OK) else "tcg"
        command = [tools["qemu-system-x86_64"], "-name", "lintel-disposable", "-machine", "q35",
                   "-accel", "kvm" if accel == "kvm" else "tcg,thread=multi", "-cpu", "host" if accel == "kvm" else "max",
                   "-smp", "2", "-m", "2048", "-display", "none", "-monitor", "none",
                   "-serial", "file:" + str(directory / "serial.log"),
                   "-drive", "file=" + str(overlay) + ",if=virtio,format=qcow2",
                   "-drive", "file=" + str(directory / "seed.img") + ",if=virtio,format=raw,readonly=on",
                   "-netdev", f"user,id=fixture,restrict=on,hostfwd=tcp:127.0.0.1:{port}-:22",
                   "-device", "virtio-net-pci,netdev=fixture,mac=52:54:00:4c:49:4e"]
        report["vm"] = {"accelerator": accel, "ssh_bound_to": "127.0.0.1", "guest_outbound_network": "restricted", "overlay_disposable": True}
        with (directory / "qemu.log").open("wb") as log:
            process = subprocess.Popen(command, stdout=log, stderr=log)
            guest = Guest(directory, port, tools)
            try:
                wait_boot(guest, process, args.boot_timeout)
                yield guest, process
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=10)
                report["vm"]["qemu_terminated"] = process.poll() is not None
                report["vm"]["serial_tail"] = (directory / "serial.log").read_text(errors="replace")[-24000:] if (directory / "serial.log").exists() else ""
                report["vm"]["qemu_log"] = (directory / "qemu.log").read_text(errors="replace")[-8000:]
    finally:
        temporary.cleanup()
        if "vm" in report:
            report["vm"]["overlay_seed_and_keys_removed"] = True


def wait_boot(guest, process, timeout, previous_boot=None):
    deadline, last = time.monotonic() + timeout, "not contacted"
    while (remaining := deadline - time.monotonic()) > 0:
        require(process.poll() is None, "The owned QEMU process exited before the guest was ready")
        try:
            response = guest.shell("cat /proc/sys/kernel/random/boot_id", timeout=min(10, remaining), check=False)
            if response.returncode == 0:
                boot = response.stdout.decode().strip()
                if boot and boot != previous_boot:
                    remaining = deadline - time.monotonic()
                    if remaining <= 0:
                        last = "Boot ID was observed after the overall readiness deadline"
                        break
                    done = guest.shell("cloud-init status --wait", timeout=remaining, check=False)
                    require(done.returncode == 0, "Cloud-init failed: " + done.stdout.decode() + done.stderr.decode())
                    return boot
            last = response.stderr.decode(errors="replace")
        except subprocess.TimeoutExpired as error:
            # Socket activation/cloud-init under TCG can outlast one SSH probe.
            # Keep the same guest and QEMU, but never reset their boot deadline.
            last = str(error)
            if error.stderr:
                last += ": " + error.stderr.decode(errors="replace")
        time.sleep(min(1, max(0, deadline - time.monotonic())))
    raise RuntimeError("Guest did not complete its real boot/SSH boundary: " + last)


def prepare_policy(guest, name):
    as_root = name in ("supervised-system", "supervised-system-reboot")
    case = guest.probe("prepare", name, user=ADMIN if as_root else TARGET)
    case["as_root"] = as_root
    environment = guest.request(case, {"command": "register", "name": "Synthetic VM " + name, "root": case["root"]})
    plan = guest.request(case, {"command": "plan_policy", "environment_id": environment["id"], "preset": "reduce", "keep_remote_control": False})
    case["plan"] = plan
    return case


def barrier_process(guest, case, mode, timeout=15, user=ADMIN):
    """Observe the worker after its durable ACK. In "stopped" mode a frozen worker
    is the point; in "running" mode the worker must still be live (running or
    sleeping, never stopped) so a later logout compares a live process. A worker
    that is already gone is returned as observed termination, never looped on."""
    deadline = time.monotonic() + timeout
    observed = None
    while time.monotonic() < deadline:
        prefix = ["sudo", "-n"] if user == ADMIN else []
        response = guest.shell(shlex.join(prefix + ["python3", PROBE, "process", case["barrier"]]), user=user, check=False)
        if response.returncode == 0:
            observed = json.loads(response.stdout)
            require(observed["marker"]["plan_id"] == case["plan"]["id"], "Barrier belongs to another job")
            if mode == "stopped" and observed["state"] in ("T", "t"):
                return observed
            if mode == "running" and observed["alive"] and observed["state"] in ("R", "S"):
                return observed
            if not observed["alive"]:
                return observed
        time.sleep(0.2)
    raise RuntimeError(f"Synthetic after-accept barrier produced no {mode} worker within {timeout}s: {observed}")


def terminal_receipt(guest, case, timeout=30):
    receipt = guest.request(case, {"command": "job", "plan_id": case["plan"]["id"]})
    deadline = time.monotonic() + timeout
    while receipt["status"] in ("accepted", "executing", "verifying") and time.monotonic() < deadline:
        time.sleep(0.2)
        receipt = guest.request(case, {"command": "job", "plan_id": case["plan"]["id"]})
    return receipt

def resume_survivor(guest, case, mode):
    """Resume a worker that survived logout. A stopped worker is resumed with
    SIGCONT; a running (held) worker is explicitly released. Both are valid
    outcomes and both keep the original approved job, which the caller then
    waits on for a terminal observation."""
    if mode == "stopped":
        guest.probe("continue", case["barrier"])
    else:
        guest.probe("release", case["release"])

def logout_case(guest, kill, mode):
    """One logout observation. mode "stopped" freezes the worker (SIGSTOP); mode
    "running" keeps it live behind an explicit release. The two modes run under
    the SAME effective KillUserProcesses policy so a live worker and a frozen one
    are compared without the barrier itself being the variable."""
    policy = guest.probe("configure-logind", "yes" if kill else "no")
    require(policy["kill_user_processes"] == ("b true" if kill else "b false"), "Effective logind policy does not match the VM fixture")
    case = prepare_policy(guest, f"logout-{'kill' if kill else 'retain'}-{mode}")
    ack = guest.request(case, {"command": "execute", "plan_id": case["plan"]["id"], "approval": case["plan"]["hash"]}, submit=True, barrier=mode)
    require(ack["status"] == "accepted", "Submit did not return durable accepted ACK")
    observed = barrier_process(guest, case, mode)
    pam = guest.probe("read-json", case["session"])
    require(pam["present"] and pam["properties"].get("Remote") == "yes", "Submission did not traverse a real remote PAM session")
    scope = pam["properties"].get("Scope")
    require(scope and scope in observed["marker"]["cgroup"], "setsid worker left its originating PAM session cgroup unexpectedly")
    # Wait for logind's asynchronous session cleanup; do not open another target
    # session until its original logout effect has actually been observed. The
    # observation loop watches the worker too, so a live worker is not declared
    # surviving merely because the session record cleared first.
    deadline = time.monotonic() + 20
    after = guest.probe("session", pam["id"])
    while time.monotonic() < deadline and (after["properties"].get("State") == "active" or observed["alive"]):
        time.sleep(0.3)
        after = guest.probe("session", pam["id"])
        observed = guest.probe("process", case["barrier"])
    require(not after["present"] or after["properties"].get("State") != "active",
            "Original PAM session stayed active; logout observation did not complete")
    # The original session is gone now; a fresh observer session may read logind
    # evidence without perturbing the observation above.
    evidence = guest.probe("session-evidence", pam["id"])
    receipt_once = guest.request(case, {"command": "job", "plan_id": case["plan"]["id"]})
    receipt = receipt_once
    if observed["alive"]:
        # A surviving worker keeps its operation ownership. A stopped worker is
        # resumed with SIGCONT; a running (held) worker is explicitly released.
        # Only then may the original approved synthetic plan run to completion.
        require(receipt["status"] == "accepted", "Surviving worker lost its operation ownership before release")
        resume_survivor(guest, case, mode)
        receipt = terminal_receipt(guest, case)
        require(receipt["status"] in ("completed", "partially_completed", "needs_reconciliation"),
                f"Continued original job has no terminal observation: {receipt}")
    else:
        require(observed["classification"] in ("missing", "zombie", "identity_mismatch"),
                f"Non-alive worker was classified as {observed['classification']}, not an observed termination")
        require(receipt["status"] == "needs_reconciliation", "Dead worker must not remain presented as live or completed")
    repeated = guest.request(case, {"command": "job", "plan_id": case["plan"]["id"]})
    require(repeated["id"] == receipt["id"] and repeated["status"] == receipt["status"],
            "Repeated query changed/replayed the original job")
    require(receipt["plan_id"] == case["plan"]["id"], "Query returned a different job")
    return {"kill_user_processes": kill, "mode": mode, "effective_policy": policy, "pam_session": pam,
            "original_session_after_logout": after, "worker_after_logout": observed,
            "session_evidence": evidence,
            "survival": "observed_survived" if observed["alive"] else "observed_terminated",
            "classification": observed["classification"], "survival_guaranteed": False,
            "receipt_after_query_or_continue": receipt, "receipt_first_query": receipt_once,
            "repeated_query": repeated, "submitted_once": True, "reexecuted": False}


def supervised_case(guest, kind, kill):
    """Observe the original PAM logout before opening another target-user login.
    Eligible units MUST remain live and complete the original approved plan;
    losing one fails acceptance rather than relabeling reconciliation as success.
    Linger changes apply only to this disposable VM's synthetic user.
    """
    policy = guest.probe("configure-logind", "yes" if kill else "no")
    require(policy["kill_user_processes"] == ("b true" if kill else "b false"), "Effective logind policy does not match the VM fixture")
    linger_before = guest.probe("linger", "read")
    if kind == "user":
        guest.probe("linger", "enable")
    elif kind == "setsid" and linger_before["linger"] == "yes":
        guest.probe("linger", "disable")
    try:
        case = prepare_policy(guest, f"supervised-{kind}")
        expect = {"system": "system_manager", "user": "user_manager", "setsid": "setsid"}[kind]
        ack = guest.request(case, {"command": "execute", "plan_id": case["plan"]["id"], "approval": case["plan"]["hash"]},
                            submit=True, barrier="running")
        require(ack["status"] == "accepted", "Supervised submit did not return durable accepted ACK")
        execution = ack["execution"]
        require(execution["mode"] == expect, f"Receipt execution mode {execution} != {expect}")
        require(execution["reboot_survival"] is False, "Receipt must not claim reboot survival")
        before = barrier_process(guest, case, "running")
        unit = execution["unit"]
        unit_before = None
        if kind in ("system", "user"):
            require(unit and unit in before["marker"]["cgroup"], "Worker is outside the selected unit cgroup")
            unit_before = guest.probe("unit-state", f"{kind}:{unit}")
            require(unit_before["properties"].get("ActiveState") == "active", f"Selected unit not active: {unit_before}")
            require("session-" not in before["marker"]["cgroup"], "Manager-launched worker stayed inside the login session scope")
        else:
            require(unit is None and execution["limitation"], "setsid must record an explicit limitation without a unit")
        pam = guest.probe("read-json", case["session"])
        require(pam["present"] and pam["properties"].get("Remote") == "yes", "Submission did not traverse a real remote PAM session")
        # Only the separate observer reads /proc and the original session here.
        # No target-user SSH reconnect is allowed until the observation ends.
        deadline = time.monotonic() + 20
        after = guest.probe("session", pam["id"])
        observed = guest.probe("process", case["barrier"])
        while time.monotonic() < deadline:
            time.sleep(0.3)
            after = guest.probe("session", pam["id"])
            observed = guest.probe("process", case["barrier"])
            if not observed["alive"] and (not after["present"] or after["properties"].get("State") != "active"):
                break
        require(not after["present"] or after["properties"].get("State") != "active", "Original PAM session stayed active")
        evidence = guest.probe("session-evidence", pam["id"])
        if kind in ("system", "user"):
            require(observed["alive"] and observed["classification"] == "running",
                    f"Eligible {kind} manager lost its original worker after logout: {observed}")
        receipt = guest.request(case, {"command": "job", "plan_id": case["plan"]["id"]})
        require(receipt["execution"] == execution, "Persisted execution context differs from ACK")
        first_query = receipt
        if observed["alive"]:
            require(receipt["status"] == "accepted", "Surviving worker lost original ownership")
            guest.probe("release", case["release"])
            receipt = terminal_receipt(guest, case)
            require(receipt["status"] == "completed", f"Released original policy task did not complete: {receipt}")
        else:
            require(receipt["status"] == "needs_reconciliation", "Lost worker must reconcile")
        again = guest.request(case, {"command": "job", "plan_id": case["plan"]["id"]})
        require(again["id"] == receipt["id"] and again["status"] == receipt["status"], "Repeated query changed/replayed original task")
        require(again["execution"] == execution, "Final query lost persisted execution facts")
        return {"kind": kind, "kill_user_processes": kill, "effective_policy": policy,
                "expected_mode": expect, "linger_before": linger_before["linger"],
                "receipt_execution": execution, "worker_before": before, "worker_after_logout": observed,
                "unit_before": unit_before, "pam_session": pam, "original_session_after_logout": after,
                "session_evidence": evidence, "survival": "observed_survived" if observed["alive"] else "observed_terminated",
                "classification": observed["classification"], "receipt_first_query": first_query,
                "receipt_after": receipt, "repeated_query": again, "submitted_once": True, "reexecuted": False}
    finally:
        guest.probe("linger", "enable" if linger_before["linger"] == "yes" else "disable")


def services(guest, evidence, phase=None):
    output = {None: SERVICE_SUITE_REPORT, "prepare": SERVICE_PREPARE_REPORT, "recover": SERVICE_RECOVER_REPORT}[phase]
    args = ["sudo", "-n", "python3", SERVICE, "--runner", RUNNER, "--json", output]
    if phase is not None:
        args += ["--phase", phase]
    if phase == "recover":
        args += ["--previous-report", SERVICE_PREPARE_REPORT]
    response = guest.shell(shlex.join(args), timeout=240, check=False)
    print(response.stdout.decode(errors="replace"), end="", flush=True)
    saved = guest.shell(shlex.join(["sudo", "-n", "cat", output]), check=False)
    if response.returncode:
        evidence["services_failure"] = {"phase": phase or "full", "exit_code": response.returncode,
                                        "stderr": response.stderr.decode(errors="replace")[-8192:]}
        if saved.returncode == 0:
            evidence["services_failure"]["report"] = json.loads(saved.stdout)
        raise RuntimeError("Real service lifecycle failed: " + evidence["services_failure"]["stderr"])
    require(saved.returncode == 0, "Service lifecycle did not save its evidence report")
    return json.loads(saved.stdout)


def journey(guest, process, args, report):
    guest.shell("sudo -n install -d -m 755 /opt/lintel-vm")
    guest.upload(args.runner, RUNNER, executable=True)
    guest.upload(PROBE_SOURCE, PROBE)
    guest.upload(SERVICE_SOURCE, SERVICE)
    uploaded = guest.shell(shlex.join(["sha256sum", RUNNER])).stdout.decode().split()[0]
    require(uploaded == report["runner"]["sha256"], "Activated VM runner differs from the selected artifact")
    facts = guest.probe("facts")
    require(facts["pid1"] == "systemd" and facts["logind_active"] == "active", "VM did not boot real systemd/logind")
    require(facts["sshd_use_pam"] and facts["pam_systemd_configured"] and facts["pam_sshd_includes_common_session"], "VM SSH does not use real PAM/systemd sessions")
    report["runtime_before"] = facts
    report["services_full_suite"] = services(guest, report)
    report["services_before_reboot"] = services(guest, report, "prepare")
    # Compare a frozen worker and a live (held) worker under both effective
    # logind policies, so termination cannot be blamed on the barrier alone.
    report["logout"] = [logout_case(guest, kill, mode)
                        for kill in (False, True) for mode in ("running", "stopped")]
    # Supervised selection: root system manager, non-root user manager after the
    # fixture enables Linger on ONLY the synthetic user, and the ineligible setsid
    # route. KillUserProcesses=yes is the strict policy that matters for survival.
    report["supervised"] = [supervised_case(guest, kind, True)
                            for kind in ("system", "user", "setsid")]
    guest.probe("configure-logind", "no")
    case = prepare_policy(guest, "reboot-interrupt")
    child, ack = guest.kept_submission(case, {"command": "execute", "plan_id": case["plan"]["id"], "approval": case["plan"]["hash"]})
    # A supervised (system-manager) accepted job is also interrupted by the same
    # reboot, so one reboot yields both the setsid and the supervised post-boot
    # reconciliation evidence.
    supervised_case_reboot = prepare_policy(guest, "supervised-system-reboot")
    supervised_ack = guest.request(supervised_case_reboot,
                                   {"command": "execute", "plan_id": supervised_case_reboot["plan"]["id"],
                                    "approval": supervised_case_reboot["plan"]["hash"]},
                                   submit=True, barrier="stopped", user=ADMIN)
    try:
        require(ack["status"] == "accepted", "Reboot scenario did not receive durable ACK")
        require(supervised_ack["status"] == "accepted", "Supervised reboot scenario did not receive durable ACK")
        supervised_receipt = guest.request(supervised_case_reboot,
                                           {"command": "job", "plan_id": supervised_case_reboot["plan"]["id"]})
        require(supervised_receipt["execution"]["mode"] == "system_manager",
                f"Supervised reboot job was not system-managed: {supervised_receipt['execution']}")
        stopped = barrier_process(guest, case, "stopped")
        supervised_stopped = barrier_process(guest, supervised_case_reboot, "stopped", user=ADMIN)
        require(supervised_receipt["execution"]["unit"] in supervised_stopped["marker"]["cgroup"],
                "Supervised reboot worker is not in its selected unit cgroup")
        require(stopped["alive"] and stopped["state"] in ("T", "t"), "Reboot scenario requires a real stopped worker before boot changes")
        old_boot = facts["boot_id"]
        require(stopped["marker"]["boot_id"] == old_boot, "Worker boot identity differs before reboot")
        print("Reboot the disposable guest with an accepted job still paused", flush=True)
        guest.shell("sudo -n systemctl reboot --no-block", check=False)
        new_boot = wait_boot(guest, process, args.boot_timeout, previous_boot=old_boot)
        require(new_boot != old_boot, "Guest reboot did not change the kernel boot ID")
        interrupted = guest.request(case, {"command": "job", "plan_id": case["plan"]["id"]})
        require(interrupted["status"] == "needs_reconciliation" and interrupted["plan_id"] == case["plan"]["id"], "Interrupted original job did not reconcile after guest reboot")
        again = guest.request(case, {"command": "job", "plan_id": case["plan"]["id"]})
        require(again["id"] == interrupted["id"] and again["status"] == "needs_reconciliation", "Repeated post-boot query changed/replayed the original job")
        supervised_after = guest.request(supervised_case_reboot,
                                         {"command": "job", "plan_id": supervised_case_reboot["plan"]["id"]})
        require(supervised_after["status"] == "needs_reconciliation"
                and supervised_after["plan_id"] == supervised_case_reboot["plan"]["id"],
                "Interrupted supervised job did not reconcile after guest reboot")
        supervised_again = guest.request(supervised_case_reboot,
                                         {"command": "job", "plan_id": supervised_case_reboot["plan"]["id"]})
        require(supervised_again["id"] == supervised_after["id"]
                and supervised_again["status"] == "needs_reconciliation",
                "Repeated post-boot supervised query changed/replayed the original job")
        report["reboot"] = {"before_boot_id": old_boot, "after_boot_id": new_boot, "worker_before": stopped,
                            "original_job_after": interrupted, "repeated_query": again, "submitted_once": True, "reexecuted": False}
        report["supervised_reboot"] = {"worker_before": supervised_stopped, "original_job_after": supervised_after,
                                       "repeated_query": supervised_again, "submitted_once": True, "reexecuted": False}
        report["runtime_after"] = guest.probe("facts")
        report["services_after_reboot"] = services(guest, report, "recover")
    finally:
        if child.poll() is None:
            child.terminate()
        try:
            child.wait(timeout=10)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait(timeout=10)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runner", type=Path, required=True)
    parser.add_argument("--json", type=Path, required=True, help="Evidence report outside the Git tree, written on success and failure")
    parser.add_argument("--cache-dir", type=Path, default=Path(os.environ.get("XDG_CACHE_HOME", str(Path.home() / ".cache"))) / "lintel-vm-images")
    parser.add_argument("--image-build", default="20260801", help="Pinned official Ubuntu noble release serial")
    parser.add_argument("--keyring", type=Path, default=Path("/usr/share/keyrings/ubuntu-cloudimage-keyring.gpg"))
    parser.add_argument("--accel", choices=("auto", "kvm", "tcg"), default="auto")
    parser.add_argument("--boot-timeout", type=int, default=600)
    args = parser.parse_args()
    report = {"schema": "lintel-linux-vm-acceptance-v1", "status": "failed", "claims": {
        "host_policy_unchanged": True, "disposable_guest": True, "runner_logout_survival_guaranteed": False,
        "production_or_vps_acceptance": False}}
    code = 1
    report_path = outside_repo(args.json, "VM evidence report")
    def interrupted(signum, frame):
        raise RuntimeError("VM acceptance interrupted by " + signal.Signals(signum).name)
    # A CI timeout sends SIGTERM; raise through the context manager so its own
    # QEMU and temporary guest assets still receive the same finally cleanup.
    signal.signal(signal.SIGTERM, interrupted)
    try:
        require(platform.system() == "Linux" and platform.machine() in ("x86_64", "amd64"), "This launcher requires a real Linux x86_64 host; no VM evidence was produced on this platform")
        tools = {name: shutil.which(name) for name in ("qemu-system-x86_64", "qemu-img", "cloud-localds", "ssh", "ssh-keygen", "gpgv")}
        require(all(tools.values()), "Missing VM prerequisites: " + ", ".join(name for name, path in tools.items() if not path))
        require(args.keyring.is_file(), "Ubuntu cloud-image public keyring is unavailable; install ubuntu-keyring")
        require(SERVICE_SOURCE.is_file(), "Canonical service_systemd_journey.py is unavailable; VM service lifecycle acceptance cannot run")
        args.runner = args.runner.resolve()
        report["runner"] = {"sha256": runner_identity(args.runner), "target": "x86_64-unknown-linux-musl"}
        base_image, report["image"] = image(args, tools)
        with vm(base_image, args, tools, report) as (guest, process):
            journey(guest, process, args, report)
        report["status"] = "evidence_complete"
        report["limitations"] = [
            "Observed survival or termination applies only to the recorded VM/PAM/logind policy; setsid is not a cgroup escape or survival guarantee.",
            "A terminated worker is reported with its observed /proc classification (missing/zombie/stopped/identity_mismatch); absence is never relabeled as a signal-confirmed death.",
            "Live-worker and frozen-worker logout cases only remove the synthetic barrier as a confound; they do not attribute the terminating signal.",
            "VM evidence does not prove any production VPS logout, guest boot, user-manager or account policy."]
        code = 0
        print("PASS: real Linux VM lifecycle; eligible supervisor logout continuation and reboot reconciliation verified", flush=True)
    except (RuntimeError, OSError, ValueError, subprocess.SubprocessError, KeyboardInterrupt) as error:
        report["error"] = {"type": type(error).__name__, "message": str(error)}
        print("FAIL: " + str(error), file=sys.stderr, flush=True)
    finally:
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(json.dumps(report, indent=2) + "\n")
    return code


if __name__ == "__main__":
    sys.exit(main())

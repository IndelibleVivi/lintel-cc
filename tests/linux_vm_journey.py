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

    def request(self, case, payload, *, submit=False, barrier=False):
        env = {"HOME": case["home"], "LINTEL_TEST_HOME": case["home"], "LINTEL_STATE_DIR": case["state"]}
        if barrier:
            env["LINTEL_TEST_ACCEPT_BARRIER"] = case["barrier"]
        command = shlex.join(["env"] + [f"{key}={value}" for key, value in env.items()] + [RUNNER, "submit" if submit else "request"])
        if submit:
            command = shlex.join(["python3", PROBE, "session", "--json", case["session"]]) + " && exec " + command
        result = self.shell(command, user=TARGET, data=json.dumps(payload).encode(), timeout=60)
        response = json.loads(result.stdout)
        require(response.get("ok") is True, f"Canonical runner rejected {payload['command']}: {response}")
        return response["data"]

    def kept_submission(self, case, payload):
        env = [f"HOME={case['home']}", f"LINTEL_TEST_HOME={case['home']}", f"LINTEL_STATE_DIR={case['state']}", f"LINTEL_TEST_ACCEPT_BARRIER={case['barrier']}"]
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
    while time.monotonic() < deadline:
        require(process.poll() is None, "The owned QEMU process exited before the guest was ready")
        response = guest.shell("cat /proc/sys/kernel/random/boot_id", timeout=10, check=False)
        if response.returncode == 0:
            boot = response.stdout.decode().strip()
            if boot and boot != previous_boot:
                done = guest.shell("cloud-init status --wait", timeout=timeout, check=False)
                require(done.returncode == 0, "Cloud-init failed: " + done.stdout.decode() + done.stderr.decode())
                return boot
        last = response.stderr.decode(errors="replace")
        time.sleep(1)
    raise RuntimeError("Guest did not complete its real boot/SSH boundary: " + last)


def prepare_policy(guest, name):
    case = guest.probe("prepare", name, user=TARGET)
    environment = guest.request(case, {"command": "register", "name": "Synthetic VM " + name, "root": case["root"]})
    plan = guest.request(case, {"command": "plan_policy", "environment_id": environment["id"], "preset": "reduce", "keep_remote_control": False})
    case["plan"] = plan
    return case


def barrier_process(guest, case, timeout=15):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        response = guest.shell(shlex.join(["sudo", "-n", "python3", PROBE, "process", case["barrier"]]), check=False)
        if response.returncode == 0:
            observed = json.loads(response.stdout)
            if observed["state"] in ("T", "t") or not observed["alive"]:
                require(observed["marker"]["plan_id"] == case["plan"]["id"], "Barrier belongs to another job")
                return observed
        time.sleep(0.2)
    raise RuntimeError("Synthetic after-accept barrier did not produce an observable stopped worker")


def logout_case(guest, kill):
    policy = guest.probe("configure-logind", "yes" if kill else "no")
    require(policy["kill_user_processes"] == ("b true" if kill else "b false"), "Effective logind policy does not match the VM fixture")
    case = prepare_policy(guest, "logout-kill" if kill else "logout-retain")
    ack = guest.request(case, {"command": "execute", "plan_id": case["plan"]["id"], "approval": case["plan"]["hash"]}, submit=True, barrier=True)
    require(ack["status"] == "accepted", "Submit did not return durable accepted ACK")
    observed = barrier_process(guest, case)
    pam = guest.probe("read-json", case["session"])
    require(pam["present"] and pam["properties"].get("Remote") == "yes", "Submission did not traverse a real remote PAM session")
    scope = pam["properties"].get("Scope")
    require(scope and scope in observed["marker"]["cgroup"], "setsid worker left its originating PAM session cgroup unexpectedly")
    # Wait for logind's asynchronous session cleanup; do not open another target
    # session until its original logout effect has actually been observed.
    deadline = time.monotonic() + 20
    after = guest.probe("session", pam["id"])
    while time.monotonic() < deadline and (after["properties"].get("State") == "active" or (kill and observed["alive"])):
        time.sleep(0.3)
        after = guest.probe("session", pam["id"])
        observed = guest.probe("process", case["barrier"])
    require(not after["present"] or after["properties"].get("State") != "active",
            "Original PAM session stayed active; logout observation did not complete")
    receipt = guest.request(case, {"command": "job", "plan_id": case["plan"]["id"]})
    if observed["alive"]:
        require(receipt["status"] == "accepted", "Live stopped worker lost its operation ownership")
        guest.probe("continue", case["barrier"])
        deadline = time.monotonic() + 30
        while receipt["status"] in ("accepted", "executing", "verifying") and time.monotonic() < deadline:
            time.sleep(0.2)
            receipt = guest.request(case, {"command": "job", "plan_id": case["plan"]["id"]})
        require(receipt["status"] in ("completed", "needs_reconciliation"), f"Continued original job has no terminal observation: {receipt}")
    else:
        require(receipt["status"] == "needs_reconciliation", "Dead worker must not remain presented as live or completed")
    require(receipt["plan_id"] == case["plan"]["id"], "Query returned a different job")
    return {"kill_user_processes": kill, "effective_policy": policy, "pam_session": pam,
            "original_session_after_logout": after, "worker_after_logout": observed,
            "survival": "observed_survived" if observed["alive"] else "observed_terminated",
            "survival_guaranteed": False, "receipt_after_query_or_continue": receipt,
            "submitted_once": True, "reexecuted": False}


def services(guest, phase=None):
    output = {None: SERVICE_SUITE_REPORT, "prepare": SERVICE_PREPARE_REPORT, "recover": SERVICE_RECOVER_REPORT}[phase]
    args = ["sudo", "-n", "python3", SERVICE, "--runner", RUNNER, "--json", output]
    if phase is not None:
        args += ["--phase", phase]
    if phase == "recover":
        args += ["--previous-report", SERVICE_PREPARE_REPORT]
    response = guest.shell(shlex.join(args), timeout=240)
    print(response.stdout.decode(errors="replace"), end="", flush=True)
    return json.loads(guest.shell(shlex.join(["sudo", "-n", "cat", output])).stdout)


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
    report["services_full_suite"] = services(guest)
    report["services_before_reboot"] = services(guest, "prepare")
    report["logout"] = [logout_case(guest, False), logout_case(guest, True)]
    guest.probe("configure-logind", "no")
    case = prepare_policy(guest, "reboot-interrupt")
    child, ack = guest.kept_submission(case, {"command": "execute", "plan_id": case["plan"]["id"], "approval": case["plan"]["hash"]})
    try:
        require(ack["status"] == "accepted", "Reboot scenario did not receive durable ACK")
        stopped = barrier_process(guest, case)
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
        report["reboot"] = {"before_boot_id": old_boot, "after_boot_id": new_boot, "worker_before": stopped,
                            "original_job_after": interrupted, "repeated_query": again, "submitted_once": True, "reexecuted": False}
        report["runtime_after"] = guest.probe("facts")
        report["services_after_reboot"] = services(guest, "recover")
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
        report["limitations"] = ["Observed survival or termination applies only to the recorded VM/PAM/logind policy; setsid is not a cgroup escape or survival guarantee.",
                                 "VM evidence does not prove any production VPS logout, guest boot, user-manager or account policy."]
        code = 0
        print("PASS: disposable real Linux VM lifecycle evidence complete; logout survival remains an observed limitation", flush=True)
    except (RuntimeError, OSError, ValueError, subprocess.SubprocessError, KeyboardInterrupt) as error:
        report["error"] = {"type": type(error).__name__, "message": str(error)}
        print("FAIL: " + str(error), file=sys.stderr, flush=True)
    finally:
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(json.dumps(report, indent=2) + "\n")
    return code


if __name__ == "__main__":
    sys.exit(main())

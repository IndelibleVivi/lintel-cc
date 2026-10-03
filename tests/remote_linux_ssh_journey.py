#!/usr/bin/env python3
"""Opt-in Linux OpenSSH acceptance for the native desktop remote controller.

Build the real x86_64 musl runner first, then run:
  python3 tests/remote_linux_ssh_journey.py target/x86_64-unknown-linux-musl/release/lintel

Requires Linux x86_64, OpenSSH client/server and the desktop Cargo build prerequisites.
The server is a foreground loopback process under the current user. All keys,
known_hosts, config, HOME, Lintel state and Claude fixtures live in one temporary
directory; no accounts, system sshd configuration or user shell are changed.
"""
import argparse
import os
from pathlib import Path
import platform
import pwd
import shlex
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
ALIAS = "lintel-acceptance"


def checked(command, **kwargs):
    result = subprocess.run(command, text=True, capture_output=True, **kwargs)
    if result.returncode:
        raise RuntimeError(f"{Path(command[0]).name} failed ({result.returncode}): {result.stderr}{result.stdout}")
    return result


def write_script(path, text):
    path.write_text("#!/bin/sh\n" + text)
    path.chmod(0o700)


def static_runner(path):
    """Check the same artifact boundary as the desktop bundle preparer."""
    data = path.read_bytes()
    if len(data) < 64 or data[:6] != b"\x7fELF\x02\x01" or struct.unpack_from("<H", data, 18)[0] != 62:
        raise RuntimeError("Acceptance runner must be a Linux x86_64 ELF64 executable")
    offset = struct.unpack_from("<Q", data, 32)[0]
    size, count = struct.unpack_from("<HH", data, 54)
    if size < 4 or offset + size * count > len(data):
        raise RuntimeError("Acceptance runner has an invalid ELF program header table")
    if any(struct.unpack_from("<I", data, offset + size * i)[0] == 3 for i in range(count)):
        raise RuntimeError("Acceptance runner needs a dynamic loader; build the musl target")


def fixture(base, ssh, keygen, sshd):
    home = base / "home"
    (home / "synthetic-claude").mkdir(parents=True)
    (base / "tools").mkdir()
    versions = home / "claude/versions"
    versions.mkdir(parents=True)
    # Static version metadata is the filename. Running this fixture is allowed
    # only for the final launch; zero arguments means no model prompt or request.
    fake_claude = versions / "2.1.283"
    write_script(fake_claude, f"""set -eu
[ "$#" -eq 0 ] || exit 90
in_tty=false; out_tty=false
[ ! -t 0 ] || in_tty=true
[ ! -t 1 ] || out_tty=true
printf '{{"root":"%s","config_root":"%s","home":"%s","stdin_tty":%s,"stdout_tty":%s,"argc":%s}}\\n' "$PWD" "$CLAUDE_CONFIG_DIR" "$HOME" "$in_tty" "$out_tty" "$#" > {shlex.quote(str(base / 'launch.json'))}
""")
    # OpenSSH's noninteractive PATH deliberately omits .local/bin. This tests
    # the runner's native-install fallback in its own synthetic HOME.
    (home / ".local/bin").mkdir(parents=True)
    (home / ".local/bin/claude").symlink_to(fake_claude)
    checked([keygen, "-q", "-t", "ed25519", "-N", "", "-f", str(base / "host-key")])
    checked([keygen, "-q", "-t", "ed25519", "-N", "", "-f", str(base / "client-key")])
    (base / "authorized_keys").write_text((base / "client-key.pub").read_text())
    (base / "known_hosts").write_text(ALIAS + " " + (base / "host-key.pub").read_text())
    # ForceCommand is fixture-only: it preserves the actual SSH and runner
    # boundaries, sets a synthetic environment, and then runs the original
    # command. ACK loss occurs after the real runner has emitted its response.
    write_script(base / "remote-shell", f"""set -eu
export HOME={shlex.quote(str(home))}
export LINTEL_TEST_HOME="$HOME"
export LINTEL_STATE_DIR={shlex.quote(str(base / 'runner-state'))}
export PATH={shlex.quote(str(base / 'tools'))}:/usr/bin:/bin:/usr/sbin:/sbin
export LC_ALL=C
cd "$HOME"
printf '%s\\n' "$SSH_ORIGINAL_COMMAND" >> {shlex.quote(str(base / 'commands'))}
case "$SSH_ORIGINAL_COMMAND" in
  *uploaded*)
    if [ -f {shlex.quote(str(base / 'lose-install-ack'))} ]; then
      rm {shlex.quote(str(base / 'lose-install-ack'))}
      /bin/sh -c "$SSH_ORIGINAL_COMMAND" > {shlex.quote(str(base / 'lost-install-ack.json'))}
      exit "$?"
    fi;;
  *' submit')
    if [ -f {shlex.quote(str(base / 'lose-submit-ack'))} ]; then
      rm {shlex.quote(str(base / 'lose-submit-ack'))}
      /bin/sh -c "$SSH_ORIGINAL_COMMAND" > {shlex.quote(str(base / 'lost-submit-ack.json'))}
      exit "$?"
    fi;;
esac
exec /bin/sh -c "$SSH_ORIGINAL_COMMAND"
""")
    with socket.socket() as reserved:
        reserved.bind(("127.0.0.1", 0))
        port = reserved.getsockname()[1]
    user = pwd.getpwuid(os.getuid()).pw_name
    server_config = base / "sshd_config"
    server_config.write_text(f"""Port {port}
ListenAddress 127.0.0.1
HostKey {base / 'host-key'}
PidFile {base / 'sshd.pid'}
AuthorizedKeysFile {base / 'authorized_keys'}
StrictModes no
PubkeyAuthentication yes
PasswordAuthentication no
KbdInteractiveAuthentication no
UsePAM no
PermitRootLogin prohibit-password
AllowUsers {user}
AllowAgentForwarding no
AllowTcpForwarding no
X11Forwarding no
PermitTunnel no
PermitTTY yes
PrintMotd no
PrintLastLog no
LogLevel ERROR
ForceCommand {shlex.quote(str(base / 'remote-shell'))}
""")
    config = base / "ssh_config"
    config.write_text(f"""Host {ALIAS}
  HostName 127.0.0.1
  Port {port}
  User {user}
  IdentityFile {base / 'client-key'}
  IdentitiesOnly yes
  UserKnownHostsFile {base / 'known_hosts'}
  GlobalKnownHostsFile /dev/null
  HostKeyAlias {ALIAS}
  BatchMode yes
  StrictHostKeyChecking yes
  UpdateHostKeys no
  ConnectionAttempts 1
""")
    write_script(base / "ssh", f"exec {shlex.quote(ssh)} -F {shlex.quote(str(config))} \"$@\"\n")
    # Native launch uses a test-injected opener. It runs the generated Terminal
    # script synchronously; its SSH command still allocates a real remote PTY.
    write_script(base / "terminal-opener", f"""set -eu
printf '%s\\n' "$@" > {shlex.quote(str(base / 'terminal-args'))}
for arg do script="$arg"; done
exec /bin/sh "$script"
""")
    checked([sshd, "-t", "-f", str(server_config)])
    return config, server_config


def run(runner):
    if platform.system() != "Linux" or platform.machine() not in ("x86_64", "amd64"):
        raise RuntimeError("This acceptance requires real Linux x86_64; macOS builds are not Linux runtime evidence")
    static_runner(runner)
    ssh = shutil.which("ssh")
    keygen = shutil.which("ssh-keygen")
    sshd = shutil.which("sshd") or ("/usr/sbin/sshd" if Path("/usr/sbin/sshd").is_file() else None)
    if not all((ssh, keygen, sshd)):
        raise RuntimeError("OpenSSH client, ssh-keygen and sshd are required")
    with tempfile.TemporaryDirectory(prefix="lintel-linux-ssh-") as temporary:
        base = Path(temporary).resolve()
        config, server_config = fixture(base, ssh, keygen, sshd)
        with (base / "sshd.log").open("w+") as log:
            server = subprocess.Popen([sshd, "-D", "-e", "-f", str(server_config)], stdout=log, stderr=log)
            try:
                deadline = time.monotonic() + 10
                while True:
                    if server.poll() is not None:
                        raise RuntimeError("Temporary sshd exited: " + (base / "sshd.log").read_text())
                    probe = subprocess.run([ssh, "-F", str(config), "-T", ALIAS, "true"], capture_output=True, text=True, timeout=3)
                    if probe.returncode == 0:
                        break
                    if time.monotonic() >= deadline:
                        raise RuntimeError("Temporary SSH fixture unavailable: " + probe.stderr + (base / "sshd.log").read_text())
                    time.sleep(0.1)
                env = dict(os.environ, LINTEL_SSH_ACCEPTANCE="1", LINTEL_ACCEPTANCE_FIXTURE=str(base), LINTEL_ACCEPTANCE_RUNNER=str(runner))
                test = subprocess.Popen([
                    "cargo", "test", "--locked", "--manifest-path", str(ROOT / "apps/desktop/src-tauri/Cargo.toml"),
                    "remote::acceptance::linux_openssh_runtime_journey", "--", "--ignored", "--exact", "--nocapture", "--test-threads=1",
                ], cwd=ROOT, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
                summary = ""
                for line in test.stdout:
                    print(line, end="", flush=True)
                    if line.startswith("test result:"):
                        summary = line
                code = test.wait()
                if code or "1 passed; 0 failed; 0 ignored;" not in summary:
                    raise RuntimeError(f"Native OpenSSH acceptance did not pass exactly one selected test (exit {code}); sshd log:\n" + (base / "sshd.log").read_text())
                print("PASS: real loopback Linux OpenSSH, native install/submit/query/TTY launch on synthetic roots")
            finally:
                server.terminate()
                try:
                    server.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    server.kill()
                    server.wait()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("runner", nargs="?", type=Path, default=ROOT / "target/x86_64-unknown-linux-musl/release/lintel")
    args = parser.parse_args()
    try:
        run(args.runner.resolve())
    except (RuntimeError, OSError, subprocess.SubprocessError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)

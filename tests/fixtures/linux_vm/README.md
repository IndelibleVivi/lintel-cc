# Disposable Linux VM fixtures

`runtime_probe.py` is uploaded only by `tests/linux_vm_journey.py` into its own
Ubuntu guest. It records real systemd, sshd/PAM, logind, session cgroup and kernel
boot identity; it never substitutes command fixtures for those runtime facts.

The launcher first runs the complete canonical `tests/service_systemd_journey.py`
suite, then reuses its prepare and recover phases around a real guest reboot.
Service changes use that journey's approved core plans. These probes only change
the disposable guest's logout policy for its synthetic `lintel-fixture` user and observe/continue the exact
runner process recorded by the synthetic after-accept barrier.

Run the independent acceptance explicitly on an Ubuntu x86_64 host:

```sh
python3 tests/linux_vm_journey.py \
  --runner target/x86_64-unknown-linux-musl/release/lintel \
  --json /tmp/lintel-vm-report.json
```

The script lists its host prerequisites with `--help`. Reports and the official
OS image cache must be outside the Git tree. Image integrity uses Ubuntu's
signed `SHA256SUMS`, the installed public cloud-image keyring and the exact image
and package manifest checksums. Overlay, seed, keys and all synthetic guest
data are deleted after the owned QEMU process stops, including failure and
ordinary CI cancellation paths.

`status: evidence_complete` means the named runtime observations finished. The
report separately records whether a `setsid` worker survived each recorded
logout policy after the originating PAM session leaves its active state. It
never promises survival on a VPS; absence of VM/PAM/runtime
prerequisites is a failed run, not a skipped green result.

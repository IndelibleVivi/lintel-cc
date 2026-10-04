# Disposable Linux VM fixtures

`runtime_probe.py` is uploaded only by `tests/linux_vm_journey.py` into its own
Ubuntu guest. It records real systemd, sshd/PAM, logind, session cgroup and kernel
boot identity; it never substitutes command fixtures for those runtime facts.

The launcher first runs the complete canonical `tests/service_systemd_journey.py`
suite, then reuses its prepare and recover phases around a real guest reboot.
Service changes use that journey's approved core plans. These probes only change
the disposable guest's logout policy for its synthetic `lintel-fixture` user and observe/continue the exact
runner process recorded by the synthetic after-accept barrier.

Logout is observed twice per effective `KillUserProcesses` policy: once with a
worker frozen by `LINTEL_TEST_ACCEPT_BARRIER` (SIGSTOP) and once with a worker
kept live by `LINTEL_TEST_WAIT_BARRIER`/`LINTEL_TEST_WAIT_RELEASE`, which blocks
between the durable ACK and the original approved plan body until the harness
writes an explicitly released marker under the same synthetic home. Running the
two modes under both policies separates "the synthetic barrier was frozen" from
"the host ended the worker". The probe records finite `/proc` identity
(pid/ppid/pgrp/session/starttime/state/cgroup) and a classification
(`running`/`stopped`/`zombie`/`missing`/`identity_mismatch`); a vanished pid is
reported as `missing` and is never presented as signal-confirmed death. A zombie
has no `/proc/<pid>/exe` by kernel design, so it is classified from its `stat`
state and never collapsed into `missing`. `session-evidence` is bounded to the
exact original session: its `session-<id>.scope` unit journal and logind lines
naming that session id, never the observer's user unit list. A surviving worker
is resumed (SIGCONT for a stopped worker, explicit release for a running one) and
must then finish the same original approved plan; a terminated worker must
reconcile to `needs_reconciliation`.

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
ordinary CI cancellation paths. When the service suite fails, its saved fixture
wire report is copied into the host VM report before guest cleanup; failure stays
a failed acceptance.

`status: evidence_complete` means the named runtime observations finished. The
report separately records whether a `setsid` worker survived each recorded
logout policy after the originating PAM session leaves its active state. It
never promises survival on a VPS; absence of VM/PAM/runtime
prerequisites is a failed run, not a skipped green result.

The host readiness-loop regression can run without QEMU:

```sh
python3 tests/fixtures/linux_vm/test_wait_boot.py -v
```

It checks that a transient SSH timeout keeps the same guest and the original
overall deadline. CI runs it with the verification control self-tests; it does
not produce Linux runtime evidence.

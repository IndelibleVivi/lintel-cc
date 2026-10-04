#!/usr/bin/env python3
"""Host control-flow regressions; these never produce Linux runtime evidence."""
import importlib.util
from pathlib import Path
import subprocess
import unittest
from unittest.mock import Mock, call, patch


SOURCE = Path(__file__).resolve().parents[2] / "linux_vm_journey.py"
spec = importlib.util.spec_from_file_location("linux_vm_journey", SOURCE)
journey = importlib.util.module_from_spec(spec)
spec.loader.exec_module(journey)


class Clock:
    def __init__(self):
        self.now = 0

    def monotonic(self):
        return self.now

    def sleep(self, seconds):
        self.now += seconds


class WaitBootTests(unittest.TestCase):
    def test_one_ssh_timeout_retries_same_guest_and_process(self):
        clock = Clock()
        guest, process = Mock(), Mock()
        process.poll.return_value = None

        def first_timeout(command, *, timeout, check):
            clock.now += timeout
            raise subprocess.TimeoutExpired(command, timeout, stderr=b"SSH socket still activating")

        attempts = 0

        def shell(command, *, timeout, check):
            nonlocal attempts
            attempts += 1
            if attempts == 1:
                return first_timeout(command, timeout=timeout, check=check)
            if command == "cat /proc/sys/kernel/random/boot_id":
                return subprocess.CompletedProcess(command, 0, b"new-boot\n", b"")
            return subprocess.CompletedProcess(command, 0, b"status: done\n", b"")

        guest.shell.side_effect = shell
        with patch.object(journey.time, "monotonic", clock.monotonic), patch.object(journey.time, "sleep", clock.sleep):
            self.assertEqual(journey.wait_boot(guest, process, 60), "new-boot")
        self.assertEqual(guest.shell.call_args_list, [
            call("cat /proc/sys/kernel/random/boot_id", timeout=10, check=False),
            call("cat /proc/sys/kernel/random/boot_id", timeout=10, check=False),
            call("cloud-init status --wait", timeout=49, check=False),
        ])
        self.assertEqual(process.poll.call_count, 2)
        process.terminate.assert_not_called()
        process.kill.assert_not_called()

    def test_overall_deadline_fails_with_last_timeout_diagnostic(self):
        clock = Clock()
        guest, process = Mock(), Mock()
        process.poll.return_value = None

        def timeout(command, *, timeout, check):
            clock.now += timeout
            raise subprocess.TimeoutExpired(command, timeout, stderr=b"latest SSH readiness diagnostic")

        guest.shell.side_effect = timeout
        with patch.object(journey.time, "monotonic", clock.monotonic), patch.object(journey.time, "sleep", clock.sleep):
            with self.assertRaisesRegex(RuntimeError, "Guest did not complete.*latest SSH readiness diagnostic"):
                journey.wait_boot(guest, process, 12)
        self.assertEqual(clock.now, 12)
        self.assertEqual(guest.shell.call_args_list, [
            call("cat /proc/sys/kernel/random/boot_id", timeout=10, check=False),
            call("cat /proc/sys/kernel/random/boot_id", timeout=1, check=False),
        ])
        process.terminate.assert_not_called()
        process.kill.assert_not_called()


if __name__ == "__main__":
    unittest.main()

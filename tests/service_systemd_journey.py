#!/usr/bin/env python3
"""Real systemd acceptance on an explicitly selected disposable Linux test system.

Only creates unique inert fixtures under /var/tmp and exact named system units.
Run as root; no sudo, Claude, accounts, private homes, or network calls in this script.
prepare retains fixtures for a separately authorized reboot; recover reads their report.
"""
import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import time
import uuid


class Journey:
    def __init__(self, runner, report=None):
        self.runner = str(Path(runner).resolve())
        self.report = report
        if report:
            self.base = Path(report['fixture_root'])
            self.identifier = report['fixture_id']
            self.home = Path(report['home'])
            self.state = Path(report['state_dir'])
            self.root = Path(report['root'])
            self.target = report['unit']
            self.neighbor = report['neighbor_unit']
            self.timer = report['timer_unit']
            self.alias = report['alias_unit']
            self.check_ownership()
        else:
            self.identifier = str(uuid.uuid4())
            self.base = Path('/var/tmp') / ('lintel-synthetic-service-' + self.identifier)
            self.home = self.base / 'home'
            self.state = self.base / 'state'
            self.root = self.home / 'synthetic-target'
            prefix = 'lintel-synthetic-' + self.identifier
            self.target = prefix + '-target.service'
            self.neighbor = prefix + '-neighbor.service'
            self.timer = prefix + '-trigger.timer'
            self.alias = prefix + '-alias.service'
        self.env = dict(os.environ, LINTEL_TEST_HOME=str(self.home), LINTEL_STATE_DIR=str(self.state))
        self.env.pop('LINTEL_TEST_ACCEPT_BARRIER', None)
        self.header = '# LINTEL SYNTHETIC SERVICE JOURNEY ' + self.identifier
        self.files = [Path('/etc/systemd/system') / name for name in (self.target, self.neighbor, self.timer)]

    def check_ownership(self):
        assert self.base.parent == Path('/var/tmp')
        assert self.base.name == 'lintel-synthetic-service-' + str(uuid.UUID(self.identifier))
        assert self.base.is_dir() and not self.base.is_symlink()
        assert (self.base / '.lintel-synthetic-owner').read_text() == self.identifier
        assert self.home == self.base / 'home' and self.state == self.base / 'state'
        assert self.root == self.home / 'synthetic-target'
        prefix = 'lintel-synthetic-' + self.identifier
        assert self.target == prefix + '-target.service'
        assert self.neighbor == prefix + '-neighbor.service'
        assert self.timer == prefix + '-trigger.timer'
        assert self.alias == prefix + '-alias.service'

    def systemctl(self, *args, good=True):
        result = subprocess.run(['/usr/bin/systemctl', '--system', '--no-pager', '--no-ask-password', *args],
                                text=True, capture_output=True, timeout=30)
        if good:
            assert result.returncode == 0, (args, result.stdout, result.stderr)
        return result

    def request(self, command, good=True, **fields):
        proc = subprocess.run([self.runner, 'request'], input=json.dumps(dict(command=command, **fields)),
                              text=True, capture_output=True, env=self.env, timeout=90)
        try:
            result = json.loads(proc.stdout)
        except Exception as error:
            raise AssertionError((command, proc.stdout, proc.stderr)) from error
        assert result['ok'] is good, (command, result)
        return result['data'] if good else result['error']

    def service(self, command):
        return self.request(command, environment_id=self.report['environment_id'], manager='system', unit=self.target)

    def execute(self, plan):
        receipt = self.request('execute', plan_id=plan['id'], approval=plan['hash'])
        assert receipt['status'] == 'completed', receipt
        return receipt

    def properties(self, unit):
        out = self.systemctl('show', '--all', '--property=ActiveState,MainPID,NRestarts,InvocationID,ControlGroup', unit).stdout
        return dict(line.split('=', 1) for line in out.splitlines())

    def pulse(self, root):
        path = root / 'synthetic-pulse.jsonl'
        return len(path.read_text().splitlines()) if path.exists() else 0

    def wait_pulse(self, root, previous=0):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            if self.pulse(root) > previous:
                return
            time.sleep(.1)
        raise AssertionError(('inert service did not write pulse', str(root)))

    def setup(self):
        self.base.mkdir(mode=0o700)
        (self.base / '.lintel-synthetic-owner').write_text(self.identifier)
        self.root.mkdir(parents=True)
        other = self.home / 'synthetic-neighbor'
        other.mkdir()
        (self.root / 'settings.json').write_text('{}\n')
        (self.root / '.credentials.json').write_text('SYNTHETIC_NOT_A_REAL_CREDENTIAL\n')
        writer = self.base / 'inert-writer.py'
        writer.write_text('''import json, os, pathlib, time
root = pathlib.Path(os.environ['CLAUDE_CONFIG_DIR'])
while True:
    with (root / 'synthetic-pulse.jsonl').open('a') as output:
        output.write(json.dumps({'synthetic': True, 'pid': os.getpid()}) + '\\n')
        output.flush()
    time.sleep(.1)
''')
        for unit, root in [(self.target, self.root), (self.neighbor, other)]:
            path = Path('/etc/systemd/system') / unit
            assert not path.exists(), path
            path.write_text(self.header + '\n[Unit]\nDescription=Lintel disposable inert writer\n'
                            '[Service]\nType=simple\nEnvironment=CLAUDE_CONFIG_DIR=' + str(root) + '\n'
                            'ExecStart=/usr/bin/python3 ' + str(writer) + '\nRestart=always\nRestartSec=100ms\n'
                            'KillMode=control-group\n[Install]\nWantedBy=multi-user.target\n')
        timer_path = Path('/etc/systemd/system') / self.timer
        assert not timer_path.exists()
        timer_path.write_text(self.header + '\n[Unit]\nDescription=Lintel disposable precise trigger\n'
                              '[Timer]\nOnActiveSec=200ms\nOnUnitInactiveSec=200ms\nAccuracySec=10ms\n'
                              'Unit=' + self.target + '\n[Install]\nWantedBy=timers.target\n')
        alias_path = Path('/etc/systemd/system') / self.alias
        assert not alias_path.exists()
        alias_path.symlink_to(self.target)
        self.systemctl('daemon-reload')
        self.systemctl('enable', self.target, self.neighbor, self.timer)
        self.systemctl('start', self.target, self.neighbor, self.timer)
        self.wait_pulse(self.root)
        self.wait_pulse(other)
        registered = self.request('register', name='Synthetic service target', root=str(self.root))
        self.report = dict(fixture_id=self.identifier, fixture_root=str(self.base), state_dir=str(self.state),
                           home=str(self.home), root=str(self.root), environment_id=registered['id'], manager='system',
                           unit=self.target, neighbor_unit=self.neighbor, timer_unit=self.timer, alias_unit=self.alias,
                           boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),
                           platform=dict(kernel=platform.release(), architecture=platform.machine(),
                                         systemd=self.systemctl('--version').stdout.splitlines()[0]), checks=[])
        return self.report

    def prepare(self):
        self.setup()
        observed = self.service('service_inspect')
        assert observed['bound'] and observed['active_state'] == 'active'
        assert observed['restart'] == 'always'
        assert self.timer in observed['triggered_by']
        wrong_root = self.home / 'different-root'
        wrong_root.mkdir()
        other = self.request('register', name='Synthetic wrong binding', root=str(wrong_root))
        unbound = self.request('plan_service_quiesce', good=False, environment_id=other['id'], manager='system', unit=self.target)
        assert unbound['code'] == 'service_root_unbound', unbound
        alias = self.request('plan_service_quiesce', good=False, environment_id=self.report['environment_id'], manager='system', unit=self.alias)
        assert alias['code'] == 'service_unit_unsupported', alias
        cleanup = self.request('plan_cleanup', good=False, environment_id=self.report['environment_id'], recipe='repair_login',
                               writers_confirmed_stopped=True, official_logout=False, categories=[])
        assert cleanup['code'] == 'service_quiescence_required', cleanup
        neighbor_before = self.properties(self.neighbor)
        plan = self.service('plan_service_quiesce')
        assert not Path(plan['service']['hold']['path']).exists()
        mismatch = self.request('execute', good=False, plan_id=plan['id'], approval='not-approved')
        assert mismatch['code'] == 'approval_mismatch'
        assert self.properties(self.target)['MainPID'] == str(observed['main_pid'])
        self.report['quiesce_job_id'] = plan['id']
        self.report['quiesce_plan'] = plan
        receipt = self.execute(plan)
        current = self.service('service_inspect')
        assert current['quiesced'] and current['active_state'] == 'inactive' and current['main_pid'] == 0, current
        count = self.pulse(self.root)
        neighbor_count = self.pulse(self.home / 'synthetic-neighbor')
        # Restart=always and a running timer must not restore the target writer.
        self.systemctl('start', self.target)  # native start succeeds with a skipped condition, never writes.
        time.sleep(1)
        assert self.pulse(self.root) == count, 'target rewrote synthetic state while held'
        assert self.pulse(self.home / 'synthetic-neighbor') > neighbor_count, 'neighbor stopped'
        assert self.properties(self.neighbor)['InvocationID'] == neighbor_before['InvocationID'], 'neighbor was restarted'
        assert self.properties(self.target)['ActiveState'] == 'inactive'
        assert self.request('execute', plan_id=plan['id'], approval=plan['hash'])['id'] == receipt['id']
        assert self.request('job', job_id=receipt['id'])['id'] == receipt['id']
        assert self.pulse(self.root) == count, 'replay mutated the original job'
        assert (self.root / '.credentials.json').read_text() == 'SYNTHETIC_NOT_A_REAL_CREDENTIAL\n'
        assert (self.root / 'settings.json').read_text() == '{}\n'
        # A real external unit edit must survive a rejected restoration.
        unit_path = Path('/etc/systemd/system') / self.target
        original = unit_path.read_text()
        external = original.replace('Description=Lintel disposable inert writer', 'Description=External synthetic edit')
        unit_path.write_text(external)
        self.systemctl('daemon-reload')
        conflict = self.request('plan_service_resume', good=False, job_id=receipt['id'])
        assert conflict['code'] == 'service_restore_conflict', conflict
        assert unit_path.read_text() == external and Path(plan['service']['hold']['path']).exists()
        unit_path.write_text(original)  # Only restores this script's own synthetic fixture.
        self.systemctl('daemon-reload')
        current = self.service('service_inspect')
        assert current['quiesced'], current
        self.report.update(prebootinspect=current, hold=current['hold'], target_pulse_before_reboot=count,
                           quiesce_receipt=receipt, phase='prepared')
        self.report['checks'] += ['ordinary_local_unit', 'exact_root_binding', 'alias_rejected', 'wrong_approval_no_mutation',
                                  'cleanup_checkbox_requires_live_hold', 'target_cgroup_stopped', 'restart_and_timer_blocked',
                                  'neighbor_keeps_running_without_restart', 'same_job_query_and_replay', 'unit_edit_conflict_retained',
                                  'credentials_and_settings_untouched']
        return self.report

    def recover(self):
        self.check_ownership()
        jid = self.report['quiesce_job_id']
        receipt = self.request('job', job_id=jid)
        assert receipt['id'] == jid
        observed = self.service('service_inspect')
        assert observed['quiesced'] and observed['active_state'] == 'inactive' and observed['main_pid'] == 0, observed
        assert observed['quiesce_job_id'] == jid
        assert self.pulse(self.root) == self.report['target_pulse_before_reboot'], 'target restarted before approved recovery'
        self.systemctl('start', self.target)
        assert self.properties(self.target)['ActiveState'] == 'inactive'
        self.wait_pulse(self.home / 'synthetic-neighbor')
        before = self.properties(self.neighbor)
        plan = self.request('plan_service_resume', job_id=jid)
        assert plan['service']['before']['active_state'] == 'active'
        assert Path(plan['service']['hold']['path']).exists(), 'restore preview mutated state'
        restored = self.execute(plan)
        self.wait_pulse(self.root, self.report['target_pulse_before_reboot'])
        assert self.properties(self.target)['ActiveState'] == 'active'
        assert self.properties(self.neighbor)['InvocationID'] == before['InvocationID']
        assert not Path(plan['service']['hold']['path']).exists()
        invocation = self.properties(self.target)['InvocationID']
        assert self.request('execute', plan_id=plan['id'], approval=plan['hash'])['id'] == restored['id']
        assert self.properties(self.target)['InvocationID'] == invocation
        # Original inactive service must remain inactive after separate approved restore.
        self.systemctl('stop', self.timer)
        self.systemctl('stop', self.target)
        paused = self.service('plan_service_quiesce')
        held = self.execute(paused)
        resume = self.request('plan_service_resume', job_id=held['id'])
        assert resume['service']['before']['active_state'] == 'inactive'
        self.execute(resume)
        assert self.properties(self.target)['ActiveState'] == 'inactive'
        assert (self.root / '.credentials.json').read_text() == 'SYNTHETIC_NOT_A_REAL_CREDENTIAL\n'
        new_boot = Path('/proc/sys/kernel/random/boot_id').read_text().strip()
        self.report.update(postbootinspect=observed, resume_receipt=restored, reboot_observed=new_boot != self.report['boot_id'],
                           recovered_boot_id=new_boot, phase='recovered', status='passed')
        self.report['checks'] += ['persistent_hold_loaded_on_recovery', 'original_job_recovered_without_reexecute',
                                  'independent_resume_approval', 'resume_replay_no_restart', 'original_inactive_stays_inactive']
        if self.report['reboot_observed']:
            self.report['checks'].append('persistent_hold_survived_real_reboot')
        return self.report

    def cleanup(self):
        if not self.base.exists():
            return
        self.check_ownership()
        for unit in (self.timer, self.target, self.neighbor):
            self.systemctl('stop', unit, good=False)
            self.systemctl('disable', unit, good=False)
        alias = Path('/etc/systemd/system') / self.alias
        if alias.is_symlink() and os.readlink(alias) == self.target:
            alias.unlink()
        for path in self.files:
            if path.exists():
                assert path.read_text().startswith(self.header + '\n'), ('fixture ownership changed', str(path))
                path.unlink()
        dropins = Path('/etc/systemd/system') / (self.target + '.d')
        if dropins.exists():
            for path in dropins.glob('90-lintel-*.conf'):
                assert path.read_text().startswith('# Lintel owned hold; restore through original job ')
                assert str(self.state) in path.read_text(), ('foreign hold retained', str(path))
                path.unlink()
            if not list(dropins.iterdir()):
                dropins.rmdir()
        self.systemctl('daemon-reload')
        shutil.rmtree(self.base)  # Owned disposable root checked above, never a personal home.


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runner', required=True)
    parser.add_argument('--json', type=Path)
    parser.add_argument('--phase', choices=['prepare', 'recover'])
    parser.add_argument('--previous-report', type=Path)
    args = parser.parse_args()
    if platform.system() != 'Linux' or os.geteuid() != 0:
        parser.error('requires root on an explicitly selected disposable Linux system with real systemd/cgroup v2')
    assert Path(args.runner).is_absolute() and Path(args.runner).is_file()
    assert Path('/run/systemd/system').is_dir() and Path('/sys/fs/cgroup/cgroup.controllers').exists()
    if args.phase and not args.json:
        parser.error('--phase requires --json so original job/fixture evidence survives interruption')
    if args.phase == 'recover' and not args.previous_report:
        parser.error('recover requires --previous-report')
    previous = json.loads(args.previous_report.read_text()) if args.previous_report else None
    journey = Journey(args.runner, previous)
    try:
        report = journey.recover() if args.phase == 'recover' else journey.prepare()
        if args.json:
            args.json.write_text(json.dumps(report, indent=2) + '\n')
        if not args.phase:
            report = journey.recover()
        if args.phase != 'prepare':
            journey.cleanup()
            report['fixtures_cleaned'] = True
        if args.json:
            args.json.write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps(report))
    except Exception:
        if args.phase != 'prepare':
            journey.cleanup()
        raise


if __name__ == '__main__':
    main()

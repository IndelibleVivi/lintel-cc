#!/usr/bin/env python3
"""Versioned policy through the real CLI; synthetic homes and inert executables only."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / 'target/debug/lintel'


def run():
    with tempfile.TemporaryDirectory(prefix='lintel-policy-') as temp:
        base = Path(temp).resolve()
        home = base / 'home'
        root = home / 'synthetic-claude'
        root.mkdir(parents=True)
        bindir = base / 'bin'
        bindir.mkdir()
        versions = base / 'claude/versions'
        versions.mkdir(parents=True)
        marker = base / 'target-was-executed'
        for version in ('2.1.282', '2.1.283'):
            exe = versions / version
            exe.write_text('#!/bin/sh\ntouch "' + str(marker) + '"\nexit 99\n')
            exe.chmod(0o700)
        alias = bindir / 'claude'
        alias.symlink_to(versions / '2.1.282')
        env = dict(os.environ, HOME=str(home), LINTEL_TEST_HOME=str(home),
                   LINTEL_STATE_DIR=str(base / 'state'), PATH=str(bindir))
        settings = root / 'settings.json'

        def request(command, good=True, **fields):
            proc = subprocess.run([str(BINARY), 'request'], input=json.dumps(dict(command=command, **fields)),
                                  text=True, capture_output=True, env=env, timeout=20)
            response = json.loads(proc.stdout)
            assert response['ok'] is good, (command, response)
            return response['data'] if good else response['error']

        eid = request('register', name='Synthetic versioned product', root=str(root))['id']
        assert request('discover')['environments'][0]['product_version'] == '2.1.282'
        plan = request('plan_policy', environment_id=eid, preset='reduce', keep_remote_control=False)
        receipt = request('execute', plan_id=plan['id'], approval=plan['hash'])
        assert receipt['status'] == 'completed'
        keep = request('plan_policy', environment_id=eid, preset='reduce', keep_remote_control=True)
        assert keep['policy']['remote_control']['status'] == 'blocked'
        assert not keep['changes'], 'keep must not silently erase existing ownership'
        release = request('plan_policy', environment_id=eid, preset='reduce', keep_remote_control=True,
                          release_settings=['DISABLE_TELEMETRY'])
        assert release['changes'][0]['before'] == '1' and release['changes'][0]['after'] is None
        assert release['policy']['remote_control']['status'] == 'configuration_compatible'
        # An external writer after preview must be preserved, including a nonempty false value.
        doc = json.loads(settings.read_text())
        doc['env']['DISABLE_TELEMETRY'] = 'false'
        doc['env']['UNRELATED'] = 'keep'
        settings.write_text(json.dumps(doc))
        assert request('execute', good=False, plan_id=release['id'], approval=release['hash'])['code'] == 'stale_plan'
        assert json.loads(settings.read_text()) == doc
        release = request('plan_policy', environment_id=eid, preset='reduce', keep_remote_control=True,
                          release_settings=['DISABLE_TELEMETRY'])
        receipt = request('execute', plan_id=release['id'], approval=release['hash'])
        assert receipt['status'] == 'completed' and receipt['policy'] == release['policy']
        assert 'DISABLE_TELEMETRY' not in json.loads(settings.read_text())['env']
        restore = request('plan_restore', job_id=receipt['id'])
        assert request('execute', plan_id=restore['id'], approval=restore['hash'])['status'] == 'completed'
        assert json.loads(settings.read_text()) == doc
        alias.unlink()  # only our disposable synthetic executable alias
        alias.symlink_to(versions / '2.1.283')
        for condition, expected in [('unknown', 'conditional'), ('required', 'blocked'),
                                    ('not_required', 'configuration_compatible')]:
            inspected = request('inspect', environment_id=eid, trusted_devices=condition)
            assert inspected['environment']['product_version'] == '2.1.283'
            assert inspected['policy']['remote_control']['status'] == expected
        # Boolean feedback/survey controls differ from nonempty telemetry/error controls.
        settings.write_text(json.dumps({'env': {key: 'false' for key in (
            'DISABLE_TELEMETRY', 'DISABLE_ERROR_REPORTING',
            'DISABLE_FEEDBACK_COMMAND', 'CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY')}}))
        states = {s['key']: s['status'] for s in request('inspect', environment_id=eid)['settings']}
        assert states == {'DISABLE_TELEMETRY': 'configured', 'DISABLE_ERROR_REPORTING': 'configured',
                          'DISABLE_FEEDBACK_COMMAND': 'unchanged', 'CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY': 'unchanged'}
        pending = request('plan_policy', environment_id=eid, preset='reduce', keep_remote_control=True,
                          trusted_devices='not_required')
        assert pending['policy']['remote_control']['status'] == 'configuration_compatible'
        assert {c['key'] for c in pending['changes']} == {'DISABLE_FEEDBACK_COMMAND', 'CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY'}
        alias.unlink()
        alias.symlink_to(versions / '2.1.282')
        assert request('execute', good=False, plan_id=pending['id'], approval=pending['hash'])['code'] == 'stale_product'
        alias.unlink()
        unknown = request('inspect', environment_id=eid)
        assert unknown['environment']['product_version'] is None
        assert unknown['policy']['remote_control']['status'] == 'conditional'
        assert not marker.exists(), 'version discovery must never run the target executable'
        print('PASS: static version identity, old/new/unknown matrix, explicit release, external edits, restore, stale product, distinct value semantics')


if __name__ == '__main__':
    run()

#!/usr/bin/env python3
"""Synthetic end-to-end checks through the shipped CLI JSON boundary.

Never points at the operator's home. Build `cargo build -p lintel-runner`
(or the runner package documented in README) first, then run this script.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / 'target/debug/lintel'


def run():
    with tempfile.TemporaryDirectory(prefix='lintel-journey-') as temp:
        home = Path(temp).resolve() / 'home'
        home.mkdir()
        state = Path(temp).resolve() / 'state'
        target = home / 'synthetic-claude'
        target.mkdir()
        settings = target / 'settings.json'
        original = {'env': {'UNRELATED_SYNTHETIC_FLAG': 'keep', 'OTEL_SERVICE_NAME': 'synthetic-observer'},
                    'permissions': {'allow': ['Read']}}
        settings.write_text(json.dumps(original))
        env = dict(os.environ, LINTEL_TEST_HOME=str(home), LINTEL_STATE_DIR=str(state))
        def request(command, good=True, **fields):
            proc = subprocess.run([str(BINARY), 'request'], input=json.dumps(dict(command=command, **fields)),
                                  text=True, capture_output=True, env=env, timeout=20)
            try:
                response = json.loads(proc.stdout)
            except Exception:
                raise AssertionError(f'{command}: invalid envelope: {proc.stdout[:500]} {proc.stderr[:500]}')
            assert response['ok'] is good, (command, response)
            return response['data'] if good else response['error']
        registered = request('register', name='Synthetic primary', root=str(target))
        identity = registered['id']
        observation = request('inspect', environment_id=identity)
        assert observation['environment']['id'] == identity
        plan = request('plan_policy', environment_id=identity, preset='reduce', keep_remote_control=False)
        assert json.loads(settings.read_text()) == original, 'Planning mutated target'
        request('execute', good=False, plan_id=plan['id'], approval='not-approved')
        assert json.loads(settings.read_text()) == original, 'Wrong approval mutated target'
        receipt = request('execute', plan_id=plan['id'], approval=plan['hash'])
        assert receipt['status'] == 'completed', receipt
        written = json.loads(settings.read_text())
        for key in ['DISABLE_TELEMETRY','DISABLE_ERROR_REPORTING','DISABLE_FEEDBACK_COMMAND','CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY']:
            assert written['env'][key] == '1', key
        assert written['permissions'] == original['permissions']
        for key,value in original['env'].items(): assert written['env'][key] == value
        assert request('job', plan_id=plan['id'])['id'] == receipt['id'], 'Lost ACK lookup failed'
        assert request('execute', plan_id=plan['id'], approval=plan['hash'])['id'] == receipt['id'], 'Replay created new job'
        written['env']['UNRELATED_LATER_EDIT'] = 'keep-later'
        settings.write_text(json.dumps(written))
        restore = request('plan_restore', job_id=receipt['id'])
        restored = request('execute', plan_id=restore['id'], approval=restore['hash'])
        assert restored['status'] == 'completed', restored
        final = json.loads(settings.read_text())
        assert final['env']['UNRELATED_LATER_EDIT'] == 'keep-later'
        assert 'DISABLE_TELEMETRY' not in final['env']
        stale = request('plan_policy', environment_id=identity, preset='reduce', keep_remote_control=False)
        final['env']['EXTERNAL_WRITER'] = 'changed-after-preview'
        settings.write_text(json.dumps(final))
        rejection = request('execute', good=False, plan_id=stale['id'], approval=stale['hash'])
        assert json.loads(settings.read_text()) == final, ('Stale plan changed target', rejection)
        support = request('export_support')
        encoded = json.dumps(support)
        assert str(home) not in encoded, 'Support report contains private root'
        print('PASS: real CLI register/inspect/preview/approval/apply/readback/replay/query/restore/staleness/redaction on synthetic files')

if __name__ == '__main__':
    run()

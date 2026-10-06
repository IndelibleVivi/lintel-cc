#!/usr/bin/env python3
"""Finite component inspection through real named CLI; synthetic HOME/state only."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import uuid

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / 'target/debug/lintel'
with tempfile.TemporaryDirectory(prefix='lintel-components-') as directory:
    base = Path(directory).resolve()
    home, state = base / 'home', base / 'state'
    root, neighbor, project = home / 'config A', home / 'config B', home / 'project'
    for path in (root, neighbor, project / '.claude'):
        path.mkdir(parents=True)
    actor = {**os.environ, 'HOME': str(home), 'LINTEL_TEST_HOME': str(home), 'LINTEL_STATE_DIR': str(state), 'PATH': str(home / 'bin')}
    def call(argv, payload=None):
        result = subprocess.run([str(BINARY), *argv], input=json.dumps(payload) if payload is not None else '', text=True, capture_output=True, env=actor, timeout=30)
        envelope = json.loads(result.stdout)
        assert result.returncode == (0 if envelope['ok'] else 1), (result.returncode, result.stderr, envelope)
        return envelope
    def data(argv, payload=None):
        result = call(argv, payload)
        assert result['ok'], result
        return result['data']
    invalid = call(['env', 'components', 'bad-id'])
    assert not invalid['ok'] and not state.exists(), invalid
    schema = data(['schema', 'inspect_components'])
    assert schema['additionalProperties'] is False
    environment = data(['env', 'register', '--name', 'Synthetic A', '--root', str(root)])
    other = data(['env', 'register', '--name', 'Synthetic B', '--root', str(neighbor)])
    executable = home / '.local/bin/claude'
    executable.parent.mkdir(parents=True)
    marker = home / 'unexpected-execution'
    executable.write_text(f"#!/bin/sh\ntouch '{marker}'\n")
    executable.chmod(0o700)
    (root / 'settings.json').write_text('{"hooks":{"PreToolUse":"SYNTHETIC_PRIVATE_COMMAND"},"env":{"TOKEN":"SYNTHETIC_SECRET_VALUE"}}')
    (root / '.credentials.json').write_text('SYNTHETIC_PRIVATE_CREDENTIAL')
    (project / '.claude/settings.json').write_text('malformed')
    original = str(uuid.uuid4())
    receipt = {'id': original, 'environment_id': environment['id'], 'status': 'executing', 'title': 'Synthetic original task', 'created_at': '2026-10-06', 'warnings': [], 'restorable': False, 'steps': [], 'task_result': {'coverage': [{'scope': 'configuration', 'state': 'done', 'detail': 'Historical synthetic fact'}]}, 'service': {'manager': 'user', 'unit': 'synthetic.service'}}
    journal = state / 'jobs' / f'{original}.json'
    journal.write_text(json.dumps(receipt))
    neighbor_receipt = {**receipt, 'id': str(uuid.uuid4()), 'environment_id': other['id']}
    (state / 'jobs' / (neighbor_receipt['id'] + '.json')).write_text(json.dumps(neighbor_receipt))
    report = data(['env', 'components', environment['id'], '--project-cwd', str(project)])
    assert report['environment_id'] == environment['id'] and report['root'] == str(root)
    assert report['project_cwd'] == str(project)
    assert [r['id'] for r in report['records']] == [original]
    components = {r['id']: r for r in report['items']}
    assert components['configuration']['state'] == 'unknown'
    assert components['cli']['facts']['selected_executable'] == str(executable)
    assert components['authentication']['state'] == 'unknown'
    assert components['desktop_ide']['state'] == 'unsupported'
    assert components['browser']['state'] == 'separate_module'
    if os.uname().sysname != 'Linux':
        assert components['services']['facts']['services'][0]['state'] == 'unknown'
    assert json.loads(journal.read_text()) == receipt
    assert not marker.exists()
    public = json.dumps(report)
    assert all(value not in public for value in ('SYNTHETIC_PRIVATE_COMMAND', 'SYNTHETIC_SECRET_VALUE', 'SYNTHETIC_PRIVATE_CREDENTIAL'))
    assert not call(['call', 'inspect_components'], {'environment_id': environment['id'], 'unexpected': True})['ok']
    assert not call(['env', 'components', environment['id'], '--project-cwd', 'relative'])['ok']
    assert list((state / 'plans').iterdir()) == []
print('PASS: finite metadata/config projection, original IDs, host-local root, no account execution/body leakage/reconciliation')

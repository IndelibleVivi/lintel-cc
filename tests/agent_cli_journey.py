#!/usr/bin/env python3
"""Discoverable CLI contract and portable work, only in disposable homes."""
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / 'target/debug/lintel'
PASSWORD = 'synthetic-work-only-passphrase'


def run():
    with tempfile.TemporaryDirectory(prefix='lintel-agent-') as tmp:
        base = Path(tmp).resolve()
        home = base / 'home'
        home.mkdir()
        state = base / 'state'
        browser = base / 'browser-state'
        env = dict(os.environ, HOME=str(home), LINTEL_TEST_HOME=str(home),
                   LINTEL_STATE_DIR=str(state), LINTEL_BROWSER_STATE=str(browser))

        def command(*args, payload=None, good=True, context=env):
            proc = subprocess.run([str(BINARY), *args], input='' if payload is None else json.dumps(payload),
                                  text=True, capture_output=True, env=context, timeout=55)
            result = json.loads(proc.stdout)
            assert result['ok'] is good, (args, result, proc.stderr)
            assert (proc.returncode == 0) is good, (args, result, proc.returncode)
            assert PASSWORD not in proc.stdout + proc.stderr
            return result['data'] if good else result

        for args in [('version', '--json'), ('capabilities', '--json'), ('describe', 'plan_policy'),
                     ('schema', 'plan_archive'), ('remote','operations'), ('describe','remote.install_runner')]:
            command(*args)
            assert not state.exists() and not browser.exists(), 'Static discovery initialized private state'
        catalog = command('capabilities')
        operations = {operation['id']: operation for operation in catalog['operations']}
        assert operations['discover']['effects']['lintel_state'] == 'update_inventory'
        assert operations['archive_read']['secret_fields'] == ['archive_passphrase']
        # Generic JSON automation must stop before the core/GUI launch path.
        hidden = command('call', 'launch', payload={'environment_id': '00000000-0000-4000-8000-000000000000'}, good=False)
        assert hidden['error']['code'] == 'interactive_launch_required', hidden
        assert not state.exists(), 'Hidden-pipe launch initialized core state'
        remote_hidden = command('remote', 'control', payload={'op': 'launch', 'alias': 'missing-synthetic',
                                'environment_id': '00000000-0000-4000-8000-000000000000'}, good=False)
        assert remote_hidden['error']['code'] == 'interactive_launch_required', remote_hidden
        remote_hidden = command('remote', 'launch', 'missing-synthetic',
                                '00000000-0000-4000-8000-000000000000', good=False)
        assert remote_hidden['error']['code'] == 'interactive_launch_required', remote_hidden
        empty_selection = command('work', 'archive', 'plan', '--environment',
                                  '00000000-0000-4000-8000-000000000000', '--categories', '', good=False)
        assert empty_selection['error']['code'] == 'invalid_request', empty_selection
        assert not state.exists(), 'Rejected launch/selection initialized core state'
        for operation in ['service_inspect', 'plan_service_quiesce']:
            unit_schema = command('schema', operation)['properties']['unit']
            for unit in ['claude.service', 'claude@synthetic.service', 'claude@.service',
                         '/tmp/claude.service', '*.service', 'claude.service\n', '猫.service']:
                schema_accepts = (len(unit) <= unit_schema['maxLength']
                                  and re.search(unit_schema['pattern'], unit) is not None
                                  and re.search(unit_schema['not']['pattern'], unit) is None)
                assert schema_accepts == (unit in ['claude.service', 'claude@synthetic.service']), unit
                if not schema_accepts:
                    rejected = command('call', operation, payload={'environment_id': '00000000-0000-4000-8000-000000000000',
                                       'manager': 'user', 'unit': unit}, good=False)
                    assert rejected['error']['code'] == 'invalid_request', rejected
                    assert not state.exists(), 'Invalid service unit initialized core state'
        command('describe', 'does-not-exist', good=False)

        for payload in ['{', '{"command":"discover"}', '{"command":"execute"}']:
            proc = subprocess.run([str(BINARY), 'submit'], input=payload, text=True,
                                  capture_output=True, env=env, timeout=55)
            assert proc.returncode != 0
            assert json.loads(proc.stdout)['ok'] is False

        root = home / 'synthetic-claude'
        root.mkdir()
        (root / 'projects/synthetic/memory').mkdir(parents=True)
        (root / 'CLAUDE.md').write_text('# Synthetic instructions\n')
        (root / 'projects/synthetic/memory/notes.md').write_text('Synthetic note\n')
        (root / 'projects/session.jsonl').write_text('{"synthetic":true}\n')
        original = '{"env":{"UNRELATED_SYNTHETIC":"keep"}}\n'
        (root / 'settings.json').write_text(original)
        (root / '.credentials.json').write_text('{"synthetic":"must-remain"}\n')
        identity = command('env', 'register', '--name', 'Synthetic source', '--root', str(root))['id']
        a = command('discover')
        b = command('request', payload={'command': 'discover'})
        assert {c['name'] for c in a['capabilities']} == {c['name'] for c in b['capabilities']}
        assert 'detached_submission' in {c['name'] for c in a['capabilities']}

        output = base / 'portable.age'
        archive = command('work', 'archive', 'plan', '--environment', identity,
                          '--categories', 'instructions,memory', '--output-path', str(output))
        assert command('plan', 'show', archive['id'])['hash'] == archive['hash']
        assert not output.exists(), 'Preview wrote the archive'
        before = command('env', 'list')['environments']
        accepted = command('job', 'submit', '--plan', archive['id'], '--approval', archive['hash'],
                           payload={'archive_passphrase': PASSWORD})
        assert accepted['id'] == archive['id']
        result = command('job', 'wait', archive['id'], '--timeout', '30s')
        assert result['status'] == 'completed', result
        assert command('env', 'list')['environments'] == before, 'Archive-only created an environment'
        assert (root / 'settings.json').read_text() == original
        assert (root / '.credentials.json').read_text() == '{"synthetic":"must-remain"}\n'
        assert output.is_file()
        assert PASSWORD.encode() not in output.read_bytes()

        other_home = base / 'other-home'
        other_home.mkdir()
        other = dict(env, HOME=str(other_home), LINTEL_TEST_HOME=str(other_home), LINTEL_STATE_DIR=str(base / 'other-state'))
        portable = base / 'transferred.age'
        shutil.copyfile(output, portable)
        manifest = command('work', 'archive', 'inspect', '--archive-path', str(portable),
                           payload={'archive_passphrase': PASSWORD}, context=other)
        assert {f['category'] for f in manifest['files']} == {'instructions', 'memory'}
        target = command('env', 'create', '--name', 'Synthetic destination', context=other)
        plan = command('work', 'import', 'plan', '--environment', target['id'], '--archive-path', str(portable),
                       '--categories', 'instructions,memory', payload={'archive_passphrase': PASSWORD}, context=other)
        command('job', 'submit', '--plan', plan['id'], '--approval', plan['hash'],
                payload={'archive_passphrase': PASSWORD}, context=other)
        imported = command('job', 'wait', plan['id'], '--timeout', '30s', context=other)
        assert imported['status'] == 'completed', imported
        dest = Path(target['root'])
        assert (dest / 'CLAUDE.md').read_bytes() == (root / 'CLAUDE.md').read_bytes()
        assert (dest / 'lintel-imports/projects/synthetic/memory/notes.md').read_bytes() == (root / 'projects/synthetic/memory/notes.md').read_bytes()
        assert not list(dest.rglob('session.jsonl')) and not (dest / '.credentials.json').exists()
        command('work', 'import', 'plan', '--environment', target['id'], '--archive-path', str(portable),
                '--categories', 'instructions', payload={'archive_passphrase': PASSWORD}, context=other, good=False)

        preserve = command('work', 'preserve', 'plan', '--environment', identity,
                           '--categories', 'instructions,memory', '--name', 'Synthetic reduced start')
        command('job', 'submit', '--plan', preserve['id'], '--approval', preserve['hash'],
                payload={'archive_passphrase': PASSWORD})
        preserved = command('job', 'wait', preserve['id'], '--timeout', '30s')
        assert preserved['status'] == 'completed', preserved
        new_id = preserved['new_environment_id']
        policy = command('policy', 'plan', '--environment', new_id, '--preset', 'reduce', '--no-keep-remote-control')
        marker = home / 'accepted-marker'
        release = home / 'release-marker'
        held = dict(env, LINTEL_TEST_WAIT_BARRIER=str(marker), LINTEL_TEST_WAIT_RELEASE=str(release))
        try:
            command('job', 'submit', '--plan', policy['id'], '--approval', policy['hash'], context=held)
            timeout = command('job', 'wait', policy['id'], '--timeout', '50ms', good=False)
            assert timeout['error']['code'] == 'wait_timeout' and timeout['plan_id'] == policy['id']
            assert len([j for j in command('jobs')['jobs'] if j['id'] == policy['id']]) == 1
        finally:
            release.touch()
        written = command('job', 'wait', policy['id'], '--timeout', '30s')
        assert written['status'] == 'completed'
        restore = command('restore', 'plan', '--job', policy['id'])
        command('job', 'submit', '--plan', restore['id'], '--approval', restore['hash'])
        assert command('job', 'wait', restore['id'], '--timeout', '30s')['status'] == 'completed'
        assert (root / 'settings.json').read_text() == original, 'Source was modified'
        for file in state.rglob('*.json'):
            assert PASSWORD not in file.read_text(), 'Secret entered persistent state'
        print('PASS: static catalog, discover parity, submit exit, portable archive, independent import, preservation, original-ID wait and restoration')


if __name__ == '__main__':
    run()

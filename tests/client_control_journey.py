#!/usr/bin/env python3
"""Client control through fresh CLI processes; synthetic homes and bytes only."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / 'target/debug/lintel'


def run():
    with tempfile.TemporaryDirectory(prefix='lintel-client-control-') as temporary:
        base = Path(temporary).resolve()
        home = base / 'home'
        home.mkdir()
        state = base / 'state'
        bindir = base / 'bin'
        bindir.mkdir()
        marker = base / 'client-was-executed'
        version = base / 'claude' / 'versions' / '2.1.285'
        version.parent.mkdir(parents=True)
        version.write_text(f'#!/bin/sh\ntouch "{marker}"\nexit 99\n')
        version.chmod(0o700)
        (bindir / 'claude').symlink_to(version)
        env = dict(os.environ, HOME=str(home), LINTEL_TEST_HOME=str(home),
                   LINTEL_STATE_DIR=str(state), PATH=str(bindir))

        def invoke(argv, fields=None, good=True):
            result = subprocess.run([str(BINARY), *argv], env=env,
                                    input=json.dumps(fields) if fields is not None else '',
                                    text=True, capture_output=True, timeout=45)
            assert result.stdout.strip(), (argv, result.returncode, result.stderr)
            response = json.loads(result.stdout)
            assert response['ok'] is good, (argv, response, result.stderr)
            return response['data'] if good else response['error']

        def request(command, good=True, **fields):
            return invoke(['request'], dict(command=command, **fields), good)

        roots = [home / 'environment-a', home / 'environment-b']
        ids = []
        original = {}
        for index, root in enumerate(roots):
            (root / 'debug').mkdir(parents=True)
            (root / 'debug' / 'synthetic.log').write_bytes(b'synthetic-private-log-body')
            (root / 'settings.json').write_text(json.dumps({'env': {
                'OTEL_LOG_USER_PROMPTS': str(index),
                'OTEL_EXPORTER_OTLP_HEADERS': 'synthetic-private-header-value',
                'OTEL_EXPORTER_OTLP_ENDPOINT': 'https://synthetic:private@collector.example.invalid?token=private',
                'UNRELATED': 'preserve-this-value'}, 'hooks': {'synthetic': 'preserve'}}))
            (root / '.claude.json').write_text(json.dumps({
                'userID': 'synthetic-private-shared-identity',
                'oauthAccount': {'email': 'synthetic-private@example.invalid'},
                'unrelated': 'preserve-this-state'}))
            original.update({str(path): path.read_bytes() for path in root.rglob('*') if path.is_file()})
            ids.append(request('register', name=f'Synthetic {index}', root=str(root))['id'])

        def public(value):
            text = json.dumps(value)
            for secret in ('synthetic-private-header-value', 'synthetic-private-shared-identity',
                           'synthetic-private@example.invalid', 'synthetic-private-log-body',
                           'https://synthetic:private@', 'token=private', 'comparison_scope', 'digests'):
                assert secret not in text, (secret, value)

        pair = request('env_compare', environment_id=ids[0], other_environment_id=ids[1])
        assert pair['comparison']['identity']['userID'] == 'same'
        assert pair['comparison']['settings']['OTEL_LOG_USER_PROMPTS']['same'] is False
        assert pair['comparison']['settings']['OTEL_EXPORTER_OTLP_HEADERS']['same'] is None
        assert not (state / 'identity-salt').exists(), 'pair comparison created persistent comparison material'
        public(pair)
        # Named CLI and the static schema feed use the same finite contract.
        named_pair = invoke(['env', 'compare', ids[0], '--other-environment-id', ids[1]])
        assert named_pair['comparison']['identity']['userID'] == 'same'
        for operation in ('residue_inventory', 'identity_inspect', 'snapshot_save',
                          'snapshot_list', 'snapshot_compare', 'env_compare'):
            described = invoke(['describe', operation])
            assert 'finite_ssh_request' in described['transports']
            assert invoke(['schema', operation])['additionalProperties'] is False
        public(named_pair)

        for eid in ids:
            observed = request('residue_inventory', environment_id=eid)
            assert any(row['category'] == 'debug_logs' and row['file_count'] == 1
                       for row in observed['locations'])
            public(observed)
            presence = request('identity_inspect', environment_id=eid)
            assert presence['fields'][0]['present'] is True
            public(presence)
            public(invoke(['env', 'residue', eid]))
            public(invoke(['env', 'identity', eid]))

        first = request('snapshot_save', environment_id=ids[0])['snapshot']['snapshot_id']
        salt = (state / 'identity-salt').read_bytes()
        second = invoke(['snapshot', 'save', '--environment', ids[0]])['snapshot']['snapshot_id']
        assert (state / 'identity-salt').read_bytes() == salt
        comparison = invoke(['snapshot', 'compare', '--environment', ids[0],
                             '--old-snapshot-id', first, '--new-snapshot-id', second])
        assert any(row['area'] == 'identity' and row['state'] == 'same' for row in comparison['changes'])
        public(comparison)
        # All observations across fresh processes leave both source roots intact.
        for path, expected in original.items():
            assert Path(path).read_bytes() == expected, path

        settings = json.loads((roots[0] / 'settings.json').read_text())
        settings['env']['OTEL_LOG_USER_PROMPTS'] = '1'
        (roots[0] / 'settings.json').write_text(json.dumps(settings))
        third = request('snapshot_save', environment_id=ids[0])['snapshot']['snapshot_id']
        comparison = request('snapshot_compare', environment_id=ids[0],
                             old_snapshot_id=second, new_snapshot_id=third)
        assert comparison['status'] == 'changed'
        assert any(row['area'] == 'settings' and row['key'] == 'OTEL_LOG_USER_PROMPTS'
                   and row['state'] == 'changed' for row in comparison['changes'])
        public(comparison)
        assert request('snapshot_compare', good=False, environment_id=ids[1],
                       old_snapshot_id=first, new_snapshot_id=third)['code'] == 'snapshot_not_found'

        (roots[0] / 'settings.json').write_text('{synthetic-malformed}')
        partial = request('snapshot_save', environment_id=ids[0])
        assert partial['snapshot']['coverage']['settings'] is False
        assert partial['identity']['state'] == 'observed'
        public(partial)
        # A bad neighboring record does not erase the readable observations.
        (state / 'snapshots' / ids[0] / 'synthetic-damaged.json').write_text('{broken}')
        listed = request('snapshot_list', environment_id=ids[0])
        assert listed['listed'] == 4 and len(listed['errors']) == 1
        public(listed)
        public(invoke(['snapshot', 'list', '--environment', ids[0]]))
        assert not marker.exists(), 'static observations ran the client executable'
        assert not list((state / 'jobs').glob('*.json')), 'observations created mutation jobs'
        # A selected semantic baseline remains observation-only. No old snapshot
        # supplies restoration bytes; the next steps use original task backups.
        settings['synthetic_note'] = 'before'
        (roots[0] / 'settings.json').write_text(json.dumps(settings))
        fourth = request('snapshot_save', environment_id=ids[0])['snapshot']['snapshot_id']
        delta = request('snapshot_compare', environment_id=ids[0], old_snapshot_id=second, new_snapshot_id=fourth)
        changed = next(row for row in delta['changes'] if row.get('key') == 'OTEL_LOG_USER_PROMPTS' and row['state'] == 'changed')
        accepted = invoke(['snapshot', 'accept', '--environment', ids[0], '--old-snapshot-id', second,
                          '--new-snapshot-id', fourth, '--selected-changes', json.dumps([changed['id']])])
        assert accepted['status'] == 'accepted'
        assert request('accept_drift', good=False, environment_id=ids[0])['code'] == 'semantic_baseline_selection_required'
        public(accepted)
        assert not list((state / 'jobs').glob('*.json'))

        password = 'synthetic-client-control-passphrase'

        def execute(plan):
            result = request('execute', plan_id=plan['id'], approval=plan['hash'], archive_passphrase=password)
            assert result['status'] == 'completed', result
            return result

        config_path = str(roots[0] / 'settings.json')
        private = invoke(['config', 'read'], {'environment_id': ids[0], 'config_path': config_path})
        document = json.loads(private['document'])
        document['synthetic_note'] = 'after'
        edit = invoke(['config', 'edit', 'plan'], {'environment_id': ids[0], 'config_path': config_path,
                      'expected_digest': private['digest'], 'document': json.dumps(document)})
        public(edit)
        edit_job = execute(edit)
        assert json.loads(Path(config_path).read_text())['synthetic_note'] == 'after'
        # A later independent edit must survive selected-field restoration.
        document = json.loads(Path(config_path).read_text())
        document['later_unrelated'] = 'keep-later'
        Path(config_path).write_text(json.dumps(document))
        restored = invoke(['config', 'restore', 'plan'], {'environment_id': ids[0], 'job_id': edit_job['id'],
                          'selected_keys': ['/synthetic_note']})
        execute(restored)
        restored_document = json.loads(Path(config_path).read_text())
        assert restored_document['synthetic_note'] == 'before'
        assert restored_document['later_unrelated'] == 'keep-later'

        selected_log = roots[0] / 'debug/synthetic.log'
        kept = roots[0] / 'debug/keep.log'
        kept.write_bytes(b'keep-this-unselected-file')
        original_settings = json.loads(Path(config_path).read_text())
        combined = invoke(['data', 'workflow', 'plan'], {
            'environment_id': ids[0], 'writers_confirmed_stopped': True,
            'policy': {'preset': 'custom', 'keep_remote_control': True,
                       'custom_settings': {'OTEL_LOG_USER_PROMPTS': 'disable'}},
            'reset_identity': True, 'residue_paths': [str(selected_log)], 'archive_before_clear': True})
        public(combined)
        completed = execute(combined)
        assert not selected_log.exists()
        assert kept.read_bytes() == b'keep-this-unselected-file'
        identity = json.loads((roots[0] / '.claude.json').read_text())
        assert 'userID' not in identity and identity['oauthAccount']
        configured = json.loads(Path(config_path).read_text())
        assert configured['env']['OTEL_LOG_USER_PROMPTS'] == '0'
        assert configured['later_unrelated'] == original_settings['later_unrelated']
        for key in ('config_archive_path', 'identity_archive_path', 'residue_archive_path'):
            assert Path(completed[key]).is_file(), (key, completed)
        manifest = invoke(['data', 'archive', 'inspect'], {'job_id': completed['id'], 'archive_passphrase': password})
        assert manifest['schema'] == 'lintel.residue/1'
        assert [row['path'] for row in manifest['files']] == [str(selected_log)]
        public(manifest)
        restore = invoke(['data', 'restore', 'plan'], {'environment_id': ids[0], 'job_id': completed['id'],
                         'archive_passphrase': password, 'selected_paths': [str(selected_log)]})
        execute(restore)
        assert selected_log.read_bytes() == b'synthetic-private-log-body'
        selected_log.write_bytes(b'new-file-after-original-job')
        repeated = request('execute', plan_id=combined['id'], approval=combined['hash'])
        assert repeated['id'] == completed['id']
        assert selected_log.read_bytes() == b'new-file-after-original-job'
        assert request('plan_residue_restore', good=False, environment_id=ids[1], job_id=completed['id'],
                       archive_passphrase=password, selected_paths=[str(selected_log)])['code'] == 'environment_mismatch'
        assert not marker.exists(), 'client control ran Claude'
        for stored in (state / 'jobs').glob('*.json'):
            assert password not in stored.read_text()
        print('PASS: CLI observations, selected baselines, encrypted config edit/field restore, one composite approval, independent residue restore and no replay')


if __name__ == '__main__':
    run()

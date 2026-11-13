#!/usr/bin/env python3
"""Fresh-process named/raw CLI authentication journey; synthetic homes only.

The fake Claude only implements inert auth status. No real CLI, helper,
Keychain, profile contents, network login or credential is used.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / 'target/debug/lintel'
SECRETS = ('SYNTHETIC_SELECTED_OAUTH', 'SYNTHETIC_API_KEY',
           'SYNTHETIC_INHERITED_TOKEN', 'SYNTHETIC_MCP_KEEP',
           'SYNTHETIC_PLUGIN_KEEP', 'SYNTHETIC_HELPER_NEVER_EXECUTE',
           'SYNTHETIC_PROFILE_SELECTOR', 'SYNTHETIC_RUNTIME_IDENTITY')


def main():
    with tempfile.TemporaryDirectory(prefix='lintel-auth-sources-') as tmp:
        base = Path(tmp).resolve()
        home, state = base / 'home', base / 'state'
        default, root, neighbor = home / '.claude', home / 'custom', home / 'neighbor'
        for directory in (default, root, neighbor, home / '.local/bin', home / '.config/anthropic'):
            directory.mkdir(parents=True, exist_ok=True)
        exe = home / '.local/bin/claude'
        exe.write_text('''#!/bin/sh
[ "$1 $2" = 'auth status' ] || exit 99
printf 'called\\n' >> "$HOME/status-calls"
if [ -n "$CLAUDE_CODE_OAUTH_TOKEN" ]; then
  printf '{"configDirectory":"%s","authMethod":"oauth_token","loggedIn":true,"email":"SYNTHETIC_RUNTIME_IDENTITY","token":"SYNTHETIC_INHERITED_TOKEN"}' "$CLAUDE_CONFIG_DIR"
  exit 0
fi
printf '{"configDirectory":"%s","authMethod":"none","loggedIn":false}' "$CLAUDE_CONFIG_DIR"
exit 1
''')
        exe.chmod(0o700)
        env = {key: value for key, value in os.environ.items()
               if not key.startswith(('CLAUDE_', 'ANTHROPIC_', 'AWS_', 'GOOGLE_', 'AZURE_'))
               and key not in ('ZDOTDIR', 'XDG_CONFIG_HOME', 'BASH_ENV', 'ENV')}
        env.update(HOME=str(home), LINTEL_TEST_HOME=str(home), LINTEL_STATE_DIR=str(state),
                   PATH=str(home / '.local/bin'), CLAUDE_CONFIG_DIR=str(default),
                   CLAUDE_CODE_OAUTH_TOKEN='SYNTHETIC_INHERITED_TOKEN',
                   ANTHROPIC_PROFILE='SYNTHETIC_PROFILE_SELECTOR')

        def no_values(value):
            rendered = json.dumps(value)
            for secret in SECRETS:
                assert secret not in rendered, (secret, value)

        def invoke(args, payload=None, good=True):
            result = subprocess.run([str(BINARY), *args], env=env,
                                    input=json.dumps(payload) if payload is not None else '',
                                    capture_output=True, text=True, timeout=60)
            assert result.stdout.strip(), (args, result.returncode, result.stderr)
            response = json.loads(result.stdout)
            no_values(response)
            assert response['ok'] is good, (args, response, result.stderr)
            return response['data'] if good else response['error']

        def request(command, good=True, **fields):
            return invoke(['request'], dict(command=command, **fields), good)

        def seed(directory):
            value = {'claudeAiOauth': {'accessToken': SECRETS[0]},
                     'mcpOAuth': {'server': {'accessToken': SECRETS[3]}},
                     'pluginSecrets': {'key': SECRETS[4]}}
            (directory / '.credentials.json').write_text(json.dumps(value))
            return value

        original = seed(root)
        seed(default)
        seed(neighbor)
        neighbor_before = (neighbor / '.credentials.json').read_bytes()
        default_before = (default / '.credentials.json').read_bytes()
        (home / '.claude.json').write_text(json.dumps({'primaryApiKey': SECRETS[1], 'keep': True}))
        (root / '.claude.json').write_text(json.dumps({'primaryApiKey': SECRETS[1], 'mcpServers': {'keep': {}}}))
        settings = {'env': {'CLAUDE_CODE_OAUTH_TOKEN': SECRETS[0], 'KEEP': 'unchanged'},
                    'apiKeyHelper': SECRETS[5], 'hooks': {'keep': 'never execute'}}
        (root / 'settings.json').write_text(json.dumps(settings))
        (root / 'CLAUDE.md').write_text('Preserve these synthetic instructions.\n')
        (root / 'projects').mkdir()
        (root / 'projects/session.jsonl').write_text('Synthetic session remains.\n')
        shell = home / '.zshrc'
        shell.write_bytes(f"# synthetic\r\nexport ANTHROPIC_API_KEY='{SECRETS[1]}'\r\nexport PATH=\"/usr/bin:$PATH\"\r\n".encode())
        # A dynamic declaration is visible but never automatically executed or edited.
        dynamic = home / '.bashrc'
        dynamic.write_text('export CLAUDE_CODE_OAUTH_TOKEN="$(never-run-helper)"\n')
        eid = invoke(['env', 'register', '--name', 'Synthetic auth', '--root', str(root)])['id']
        default_id = request('register', name='Synthetic default', root=str(default))['id']

        for operation in ('auth_inventory', 'auth_verify'):
            assert invoke(['schema', operation])['additionalProperties'] is False
            assert 'finite_ssh_request' in invoke(['describe', operation])['transports']
        inventory = invoke(['auth', 'inventory', '--environment', eid])
        raw_inventory = request('auth_inventory', environment_id=eid)
        assert inventory['sources'] == raw_inventory['sources']
        assert inventory['context']['inherited_config_dir'] == 'different'
        assert inventory['limits']['coverage_complete'] is False
        assert not (home / 'status-calls').exists(), 'inventory ran the CLI/helper'
        sources = inventory['sources']
        assert any(s['kind'] == 'process_env' and s['name'] == 'CLAUDE_CODE_OAUTH_TOKEN'
                   and s['evidence'] == 'observed_context' and not s['removable'] for s in sources)
        assert any(s.get('path') == str(dynamic) and not s['removable'] for s in sources)
        assert any(s['kind'] == 'keychain' and s['presence'] == 'unknown' for s in sources)
        assert all(s['actual_use'] == 'unverified' for s in sources)
        selected = [s for s in sources if s['removable']]
        assert len(selected) == 5, selected
        args = ['auth', 'cleanup', 'plan', '--environment', eid, '--recipe', 'repair_login',
                '--writers-confirmed-stopped', '--verify-auth-status']
        for source in selected:
            args += ['--auth-source', source['id']]
        plan = invoke(args)
        assert plan['authentication']['selected_sources'] == selected
        assert plan['authentication']['runtime_verification_requested'] is True
        assert request('plan_show', plan_id=plan['id']) == plan
        assert (root / '.credentials.json').exists(), 'preview mutated credentials'
        assert not (home / 'status-calls').exists(), 'preview ran auth status'

        ack = invoke(['job', 'submit', '--plan', plan['id'], '--approval', plan['hash']])
        assert ack['id'] == plan['id']
        job = invoke(['job', 'wait', plan['id'], '--timeout', '30s'])
        assert job['local_cleanup'] == 'completed', job
        verification = job['auth_verification']
        assert all(s['state'] == 'removed' for s in verification['local_sources'])
        assert verification['runtime']['observation']['logged_in'] is True
        assert verification['runtime']['observation']['auth_method'] == 'oauth_token'
        assert verification['other_contexts'] == verification['server_revocation'] == 'unverified'
        assert not verification['retained_sensitive_material']
        assert (home / 'status-calls').read_text() == 'called\n'
        expected = dict(original)
        expected.pop('claudeAiOauth')
        assert json.loads((root / '.credentials.json').read_text()) == expected
        assert json.loads((root / '.claude.json').read_text()) == {'mcpServers': {'keep': {}}}
        settings['env'].pop('CLAUDE_CODE_OAUTH_TOKEN')
        settings.pop('apiKeyHelper')
        assert json.loads((root / 'settings.json').read_text()) == settings
        assert shell.read_bytes() == b'# synthetic\r\nexport PATH="/usr/bin:$PATH"\r\n'
        assert dynamic.read_text() == 'export CLAUDE_CODE_OAUTH_TOKEN="$(never-run-helper)"\n'
        assert (neighbor / '.credentials.json').read_bytes() == neighbor_before
        assert (default / '.credentials.json').read_bytes() == default_before
        assert (root / 'CLAUDE.md').read_text() == 'Preserve these synthetic instructions.\n'
        assert (root / 'projects/session.jsonl').read_text() == 'Synthetic session remains.\n'

        # Independent verification does not rewrite the historical receipt or rerun status by default.
        receipt_path = state / 'jobs' / f'{plan["id"]}.json'
        receipt_before = receipt_path.read_bytes()
        fresh = invoke(['auth', 'verify', '--environment', eid, '--job', plan['id']])
        assert fresh['runtime']['state'] == 'not_requested'
        assert receipt_path.read_bytes() == receipt_before
        assert (home / 'status-calls').read_text() == 'called\n'
        fresh = invoke(['auth', 'verify', '--environment', eid, '--job', plan['id'], '--observe-runtime'])
        assert fresh['runtime']['observation']['logged_in'] is True
        assert receipt_path.read_bytes() == receipt_before

        # A new login survives replay of the same original task.
        seed(root)
        current = (root / '.credentials.json').read_bytes()
        repeated = invoke(['job', 'submit', '--plan', plan['id'], '--approval', plan['hash']])
        assert repeated['id'] == plan['id']
        assert (root / '.credentials.json').read_bytes() == current
        assert invoke(['auth', 'verify', '--environment', eid, '--job', plan['id']])['local_sources'][0]['state'] == 'still_present'

        # An edited file rejects an unaccepted plan, then the default root uses its own global file.
        default_plan = invoke(['auth', 'cleanup', 'plan', '--environment', default_id,
                               '--recipe', 'repair_login', '--writers-confirmed-stopped'])
        assert default_plan['authentication']['local_source_count'] == 2
        (default / '.credentials.json').write_text(json.dumps({'claudeAiOauth': {'accessToken': 'SYNTHETIC_NEW_LOGIN'}}))
        failure = request('execute', good=False, plan_id=default_plan['id'], approval=default_plan['hash'])
        assert failure['code'] == 'stale_auth_source', failure
        assert not (state / 'jobs' / f'{default_plan["id"]}.json').exists()
        fresh_plan = invoke(['auth', 'cleanup', 'plan', '--environment', default_id,
                             '--recipe', 'repair_login', '--writers-confirmed-stopped'])
        assert request('execute', plan_id=fresh_plan['id'], approval=fresh_plan['hash'])['local_cleanup'] == 'completed'
        assert json.loads((home / '.claude.json').read_text()) == {'keep': True}
        assert (root / '.credentials.json').read_bytes() == current

        for bad in ('all', '/arbitrary/path', 'auth-NOT-AN-ID'):
            assert invoke(['auth', 'cleanup', 'plan', '--environment', eid, '--recipe', 'repair_login',
                           '--writers-confirmed-stopped', '--auth-source', bad], good=False)['code'] == 'invalid_request'
        assert request('plan_cleanup', good=False, environment_id=eid, recipe='repair_login',
                       writers_confirmed_stopped=True, official_logout=True)['code'] == 'official_logout_scope_unsafe'
        assert request('auth_verify', good=False, environment_id=default_id,
                       job_id=plan['id'])['code'] == 'auth_verification_scope'
        for file in state.rglob('*'):
            if file.is_file():
                content = file.read_bytes()
                for secret in (*SECRETS, 'SYNTHETIC_NEW_LOGIN'):
                    assert secret.encode() not in content, (secret, file)
        print('PASS authentication source CLI journey: exact sources, one approval, retained runtime auth, immutable history and no credential copies')


if __name__ == '__main__':
    main()

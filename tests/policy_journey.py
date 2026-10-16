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
# The v5 catalog: the seven privacy switches, then the OTel telemetry, content,
# exporter and identity-attribute family.
SWITCHES = {
    'DISABLE_TELEMETRY', 'DISABLE_ERROR_REPORTING', 'DISABLE_FEEDBACK_COMMAND',
    'CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY', 'DO_NOT_TRACK', 'DISABLE_GROWTHBOOK',
    'CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC',
}
OTEL_KEYS = {
    'CLAUDE_CODE_ENABLE_TELEMETRY', 'OTEL_LOG_USER_PROMPTS', 'OTEL_LOG_ASSISTANT_RESPONSES',
    'OTEL_LOG_TOOL_CONTENT', 'OTEL_LOG_TOOL_DETAILS', 'OTEL_LOG_RAW_API_BODIES',
    'OTEL_LOG_MANAGED_SETTINGS', 'CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH', 'OTEL_LOGS_EXPORTER',
    'OTEL_TRACES_EXPORTER', 'OTEL_METRICS_EXPORTER', 'OTEL_EXPORTER_OTLP_ENDPOINT',
    'OTEL_EXPORTER_OTLP_HEADERS', 'OTEL_METRICS_INCLUDE_ACCOUNT_UUID',
    'OTEL_METRICS_INCLUDE_SESSION_ID', 'OTEL_METRICS_INCLUDE_VERSION',
    'OTEL_METRICS_INCLUDE_ENTRYPOINT', 'OTEL_METRICS_INCLUDE_REPOSITORY',
    'CLAUDE_CODE_ENABLE_FEEDBACK_SURVEY_FOR_OTEL', 'CLAUDE_CODE_DISABLE_WEB_FETCH',
}
FIELDS = SWITCHES | OTEL_KEYS
# Names of the mask: whose values are credentials Lintel never records.
MASKED_KEY = 'OTEL_EXPORTER_OTLP_HEADERS'
# Absent fields whose documented default is "on": an absent key means the default
# is in effect, which the observation reports as `configured`.
DEFAULT_ON = {'OTEL_METRICS_INCLUDE_ACCOUNT_UUID', 'OTEL_METRICS_INCLUDE_SESSION_ID'}


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
        assert set(states) == FIELDS
        # Nonempty semantics is on for any nonempty spelling; standard booleans are
        # not. An absent field follows its documented default, so the two default-on
        # metrics fields read as configured while the other v5 families do not.
        assert {key for key, status in states.items() if status == 'configured'} == {
            'DISABLE_TELEMETRY', 'DISABLE_ERROR_REPORTING',
            'OTEL_METRICS_INCLUDE_ACCOUNT_UUID', 'OTEL_METRICS_INCLUDE_SESSION_ID'}
        assert all(states[key] == 'unchanged' for key in FIELDS - DEFAULT_ON
                   - {'DISABLE_TELEMETRY', 'DISABLE_ERROR_REPORTING'})
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
        alias.symlink_to(versions / '2.1.282')

        # A custom action remains explicit even when the user declares Remote Control intent.
        settings.write_text(json.dumps({'env': {'DISABLE_TELEMETRY': ''}}))
        explicit = request('plan_policy', environment_id=eid, preset='custom', keep_remote_control=True,
                           custom_settings={'DISABLE_TELEMETRY': 'disable'})
        assert [(c['key'], c['after']) for c in explicit['changes']] == [('DISABLE_TELEMETRY', '1')]
        assert explicit['policy']['remote_control']['status'] == 'blocked'

        original = {'env': {'DISABLE_TELEMETRY': 'false', 'DISABLE_ERROR_REPORTING': '0',
                           'DISABLE_FEEDBACK_COMMAND': 'false', 'CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY': 'uncertain-original',
                           'DO_NOT_TRACK': 'false', 'DISABLE_GROWTHBOOK': 'TrUe', 'UNRELATED': 'keep-original'},
                    'permissions': {'allow': ['Read']}, 'hooks': {'synthetic': 'keep'}}
        settings.write_text(json.dumps(original, indent=1) + '\n')
        settings.chmod(0o640)
        original_bytes = settings.read_bytes()
        observation = request('inspect', environment_id=eid)
        assert {s['key'] for s in observation['settings']} == FIELDS
        assert all(s['source'] == str(settings) and s['effect_timing'] == 'next_launch'
                   and s['runtime_verified'] is False for s in observation['settings'])
        assert observation['policy']['supported_presets'] == ['preserve', 'reduce', 'custom']
        assert observation['policy']['rule_version'] == 'claude-privacy-v5-2026-10-16'
        # A subset keep, an already-disabled nonempty value and an absent remove produce a true noop.
        noop = request('plan_policy', environment_id=eid, preset='custom', custom_settings={
            'DISABLE_ERROR_REPORTING': 'keep', 'DISABLE_TELEMETRY': 'disable',
            'CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC': 'remove'})
        assert not noop['changes'] and noop['policy']['preset'] == 'custom'
        noop_receipt = request('execute', plan_id=noop['id'], approval=noop['hash'])
        assert noop_receipt['status'] == 'completed' and noop_receipt['restorable'] is False
        assert settings.read_bytes() == original_bytes and settings.stat().st_mode & 0o777 == 0o640
        assert request('drift', environment_id=eid)['status'] == 'unchanged'
        # Reject malformed selections and ambiguous use with another preset before any mutation.
        invalid = [
            {'preset': 'custom', 'custom_settings': None},
            {'preset': 'custom', 'custom_settings': []},
            {'preset': 'custom', 'custom_settings': {'UNKNOWN': 'disable'}},
            {'preset': 'custom', 'custom_settings': {'DISABLE_TELEMETRY': {'action': 'disable'}}},
            # Unsupported actions and unpaired set values remain invalid.
            {'preset': 'custom', 'custom_settings': {'DISABLE_TELEMETRY': 'toggle'}},
            {'preset': 'custom', 'custom_settings': {'DISABLE_TELEMETRY': 'set'}},
            # The OTel family admits all five, but set/enable stay typed per field.
            {'preset': 'custom', 'custom_settings': {'OTEL_LOGS_EXPORTER': 'set'}},
            {'preset': 'custom', 'custom_settings': {'OTEL_LOG_USER_PROMPTS': 'set'},
             'custom_values': {'OTEL_LOG_USER_PROMPTS': '1'}},
            {'preset': 'custom', 'custom_settings': {'OTEL_LOGS_EXPORTER': 'set'},
             'custom_values': {'OTEL_LOGS_EXPORTER': 'prometheus'}},
            {'preset': 'custom', 'custom_settings': {'OTEL_LOGS_EXPORTER': 'set'},
             'custom_values': {'OTEL_LOGS_EXPORTER': ''}},
            {'preset': 'custom', 'custom_values': {'OTEL_LOGS_EXPORTER': 'none'}},
            {'preset': 'custom', 'custom_settings': {'OTEL_LOGS_EXPORTER': 'set'}, 'custom_values': {}},
            {'preset': 'reduce', 'custom_values': {'OTEL_LOGS_EXPORTER': 'none'}},
            # The credential-bearing field is never writable and never valued.
            {'preset': 'custom', 'custom_settings': {'OTEL_EXPORTER_OTLP_HEADERS': 'set'},
             'custom_values': {'OTEL_EXPORTER_OTLP_HEADERS': 'x'}},
            {'preset': 'custom', 'custom_values': {'OTEL_EXPORTER_OTLP_HEADERS': 'x'}},
            {'preset': 'custom', 'release_settings': ['DISABLE_TELEMETRY'], 'keep_remote_control': True},
            {'preset': 'preserve', 'custom_settings': {}},
            {'preset': 'reduce', 'custom_settings': {'DISABLE_TELEMETRY': 'keep'}},
        ]
        for fields in invalid:
            choices = fields.get('custom_settings')
            values = fields.get('custom_values')
            masked_write = (isinstance(choices, dict) and choices.get('OTEL_EXPORTER_OTLP_HEADERS') in ('set', 'enable')
                            or isinstance(values, dict) and 'OTEL_EXPORTER_OTLP_HEADERS' in values)
            expected_code = 'masked_setting_not_writable' if masked_write else 'invalid_request'
            actual = request('plan_policy', good=False, environment_id=eid, **fields)['code']
            assert actual == expected_code, (fields, actual, expected_code)
        assert settings.read_bytes() == original_bytes
        # A v5 `set` writes the explicit value and stays restorable field by field.
        otel = request('plan_policy', environment_id=eid, preset='custom', keep_remote_control=False,
                       custom_settings={'OTEL_LOGS_EXPORTER': 'set'}, custom_values={'OTEL_LOGS_EXPORTER': 'none'})
        assert [(c['key'], c['before'], c['after']) for c in otel['changes']] == [('OTEL_LOGS_EXPORTER', None, 'none')]
        otel_receipt = request('execute', plan_id=otel['id'], approval=otel['hash'])
        assert otel_receipt['status'] == 'completed'
        assert json.loads(settings.read_text())['env']['OTEL_LOGS_EXPORTER'] == 'none'
        otel_restore = request('plan_restore', job_id=otel_receipt['id'])
        assert request('execute', plan_id=otel_restore['id'], approval=otel_restore['hash'])['status'] == 'completed'
        assert 'OTEL_LOGS_EXPORTER' not in json.loads(settings.read_text())['env']
        assert json.loads(settings.read_text()) == original, 'A set/restore round trip changed unrelated settings'
        assert settings.stat().st_mode & 0o777 == 0o640
        original_bytes = settings.read_bytes()
        choices = {'DISABLE_TELEMETRY': 'disable', 'DISABLE_ERROR_REPORTING': 'disable',
                   'DISABLE_FEEDBACK_COMMAND': 'disable', 'DO_NOT_TRACK': 'remove',
                   'DISABLE_GROWTHBOOK': 'remove'}
        custom = request('plan_policy', environment_id=eid, preset='custom', custom_settings=choices,
                         keep_remote_control=True)
        assert custom['title'] == '自定义保护'
        assert custom['policy']['custom_settings'] == choices and custom['policy']['keep_remote_control'] is True
        assert {c['key'] for c in custom['changes']} == {'DISABLE_FEEDBACK_COMMAND', 'DO_NOT_TRACK', 'DISABLE_GROWTHBOOK'}
        assert settings.read_bytes() == original_bytes, 'Custom preview mutated settings'
        request('execute', good=False, plan_id=custom['id'], approval='wrong-approval')
        assert settings.read_bytes() == original_bytes, 'Wrong custom approval mutated settings'
        receipt = request('execute', plan_id=custom['id'], approval=custom['hash'])
        assert receipt['status'] == 'completed' and receipt['policy'] == custom['policy']
        applied = json.loads(settings.read_text())
        expected = json.loads(json.dumps(original))
        expected['env']['DISABLE_FEEDBACK_COMMAND'] = '1'
        del expected['env']['DO_NOT_TRACK']
        del expected['env']['DISABLE_GROWTHBOOK']
        assert applied == expected and settings.stat().st_mode & 0o777 == 0o640
        applied['env']['UNRELATED_LATER'] = 'keep-later'
        settings.write_text(json.dumps(applied))
        restore = request('plan_restore', job_id=receipt['id'])
        assert request('execute', plan_id=restore['id'], approval=restore['hash'])['status'] == 'completed'
        original['env']['UNRELATED_LATER'] = 'keep-later'
        assert json.loads(settings.read_text()) == original, 'Custom remove/disable did not restore exact original values'
        stale = request('plan_policy', environment_id=eid, preset='custom',
                        custom_settings={'DISABLE_TELEMETRY': 'remove'})
        changed = json.loads(settings.read_text())
        changed['env']['EXTERNAL_WRITER'] = 'after-preview'
        settings.write_text(json.dumps(changed))
        assert request('execute', good=False, plan_id=stale['id'], approval=stale['hash'])['code'] == 'stale_plan'
        assert json.loads(settings.read_text()) == changed
        removed = request('plan_policy', environment_id=eid, preset='custom', custom_settings={'DO_NOT_TRACK': 'remove'})
        receipt = request('execute', plan_id=removed['id'], approval=removed['hash'])
        changed = json.loads(settings.read_text())
        changed['env']['DO_NOT_TRACK'] = 'external-owner'
        settings.write_text(json.dumps(changed))
        assert request('plan_restore', good=False, job_id=receipt['id'])['code'] == 'restore_conflict'
        assert json.loads(settings.read_text()) == changed
        alias.unlink()
        unknown = request('inspect', environment_id=eid)
        assert unknown['environment']['product_version'] is None
        assert unknown['policy']['remote_control']['status'] == 'blocked', 'Unknown version must not hide the retained GrowthBook blocker'
        assert not marker.exists(), 'version discovery must never run the target executable'
        print('PASS: static version identity, old/new/unknown matrix, 27-field v5 inspect, explicit release, custom actions/values/validation/noop/receipt/restore, external edits, stale product, distinct value semantics')


if __name__ == '__main__':
    run()

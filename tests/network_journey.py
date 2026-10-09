#!/usr/bin/env python3
"""Finite CLI network admission in fresh synthetic homes; no host/public probes."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / 'target/debug/lintel'


def run():
    with tempfile.TemporaryDirectory(prefix='lintel-network-cli-') as directory:
        base = Path(directory)
        home = base / 'home'
        home.mkdir()
        env = dict(os.environ, HOME=str(home), LINTEL_TEST_HOME=str(home), LINTEL_STATE_DIR=str(base / 'state'))

        def call(args, payload=None, good=False):
            proc = subprocess.run([str(BINARY), *args], input=json.dumps(payload) if payload else '', text=True, capture_output=True, env=env, timeout=10)
            envelope = json.loads(proc.stdout)
            assert envelope['ok'] is good and (proc.returncode == 0) is good, (args, envelope, proc.stderr)
            return envelope['data'] if good else envelope['error']

        operations = {item['id']: item for item in call(['capabilities', '--json'], good=True)['operations']}
        for command in ['network_inspect', 'network_probe', 'plan_network_ipv6', 'plan_network_restore']:
            assert command in operations, command
            described = call(['describe', command], good=True)
            assert described['request_schema']['additionalProperties'] is False
        assert not (base / 'state').exists(), 'static schema initialized state'
        for args in [
            ['network', 'probe', '--ipv4-url', 'http://echo.invalid'],
            ['network', 'probe', '--ipv4-url', 'https://127.1/'],
            ['network', 'probe', '--ipv4-url', 'https://0x7f000001/'],
            ['network', 'probe', '--ipv6-url', 'https://user:secret@echo.invalid'],
            ['network', 'probe', '--proxy-url', 'http://remote.invalid:8080'],
            ['network', 'probe', '--timeout-seconds', '16'],
            ['network', 'probe', '--extra', 'unsupported'],
            ['network', 'ipv6', 'plan', '--service-id', 'synthetic-set:service', '--mode', 'manual'],
            ['network', 'ipv6', 'plan', '--service-id', 'synthetic-set:service', '--mode', 'off', '--probe', '{"ipv4_url":"https://echo.invalid","ipv6_url":"https://echo.invalid","extra":true}'],
            ['network', 'restore', 'plan', '--job', 'not-a-uuid'],
        ]:
            call(args)
            assert not (base / 'state').exists(), ('invalid request initialized state', args)
        # Defaults and named/nested adapters are valid, but explicit synthetic
        # homes always fail closed before observation, DNS or an OS write.
        for args in [
            ['network', 'inspect'], ['network', 'probe'],
            ['network', 'probe', '--proxy-url', 'http://127.0.0.1:55123'],
            ['network', 'ipv6', 'plan', '--service-id', 'synthetic-set:service', '--mode', 'off'],
            ['network', 'ipv6', 'plan', '--service-id', 'synthetic-set:service', '--mode', 'link_local', '--probe', '{"ipv4_url":"https://echo4.invalid","ipv6_url":"https://echo6.invalid","timeout_seconds":2}'],
        ]:
            error = call(args)
            assert error['code'] == 'network_synthetic_home_unsupported', (args, error)
        jobs = base / 'state/jobs'
        assert not jobs.exists() or not list(jobs.iterdir()), 'host task was accepted in synthetic home'
        print('PASS: static network schema, finite CLI targets/defaults and synthetic-home host boundary')


if __name__ == '__main__':
    run()

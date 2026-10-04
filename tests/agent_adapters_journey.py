#!/usr/bin/env python3
"""Finite CLI adapter owners in disposable state; no real SSH or browser."""
import json
import os
from pathlib import Path
import selectors
import signal
import socket
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / 'target/debug/lintel'


def run():
    with tempfile.TemporaryDirectory(prefix='lintel-adapters-') as tmp:
        base = Path(tmp).resolve()
        home = base / 'home'
        home.mkdir(mode=0o700)
        (home / '.ssh').mkdir(mode=0o700)
        config = home / '.ssh/config'
        config.write_text('Host synthetic-alias\n  HostName example.invalid\n  User synthetic\n')
        config.chmod(0o600)
        env = dict(os.environ, HOME=str(home), LINTEL_TEST_HOME=str(home),
                   LINTEL_STATE_DIR=str(base / 'state'), LINTEL_BROWSER_STATE=str(base / 'browser'))

        def call(*args, payload=None, good=True):
            proc = subprocess.run([str(BINARY), *args], input=json.dumps(payload) if payload else '',
                                  text=True, capture_output=True, env=env, timeout=10)
            data = json.loads(proc.stdout)
            assert data['ok'] is good and (proc.returncode == 0) is good, (args, data, proc.stderr)
            return data.get('data') if good else data['error']

        aliases = call('remote', 'aliases')['aliases']
        assert 'synthetic-alias' in aliases
        call('remote', 'control', payload={'op': 'add_host', 'alias': 'synthetic-alias'})
        assert call('remote', 'hosts')['hosts'] == [{'alias': 'synthetic-alias'}]
        call('remote', 'control', payload={'op': 'remove_host', 'alias': 'synthetic-alias'})
        assert call('remote', 'hosts')['hosts'] == []
        error = call('remote', 'control', payload={'op': 'request', 'alias': 'not-registered',
                                                  'request': {'command': 'execute'}}, good=False)
        assert error['code'] == 'host_not_registered'
        assert call('remote', 'submit', 'synthetic-alias', payload={'command': 'discover'}, good=False)['code'] == 'invalid_submission'
        # No connect/SSH operation is issued in this test.
        if sys.platform == 'darwin':
            catalog = call('browser', 'operations')
            assert not (base / 'browser').exists(), 'Static browser catalog opened state'
            assert any(op['id'] == 'browser.submit' for op in catalog['operations'])
            assert call('browser', 'instances') == []
            assert call('browser', 'operations', '--instance', 'missing-profile', good=False)['code'] == 'browser_instance_missing'
            assert call('browser', 'submit', payload={'instance_id': 'missing-profile', 'operation_id': 'synthetic-operation',
                         'action': {'kind': 'proxy', 'port': 8080}}, good=False)['code'] == 'pairing_required_or_conflicted'
        else:
            assert call('browser', 'operations', good=False)['code'] == 'browser_component_unavailable'
        config = base / 'channel.json'
        config.write_text(json.dumps({'environment_id': 'synthetic-owner', 'bind': '127.0.0.1:0',
                                     'default_action': 'deny', 'allowed': [], 'blocked': []}))
        child = subprocess.Popen([str(BINARY), 'network', 'serve', '--config', str(config)],
                                 stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env)
        try:
            ready = selectors.DefaultSelector()
            ready.register(child.stdout, selectors.EVENT_READ)
            assert ready.select(timeout=10), 'Foreground channel did not report listening'
            first = json.loads(child.stdout.readline())
            assert first['event'] == 'listening' and first['owner'] == 'foreground_process'
            assert first['pid'] == child.pid and first['coverage'] == 'proxy_connections_only'
            assert first['direct_connections_enforced'] is False
            assert first['active_config']['environment_id'] == 'synthetic-owner'
            host, port = first['address'].rsplit(':', 1)
            with socket.create_connection((host, int(port)), timeout=5) as client:
                client.sendall(b'CONNECT example.invalid:443 HTTP/1.1\r\nHost: example.invalid:443\r\n\r\n')
                assert b'403' in client.recv(1024), 'Deny channel forwarded a connection'
            child.send_signal(signal.SIGINT)
            output, errors = child.communicate(timeout=10)
            assert child.returncode == 0, errors
            records = [json.loads(line) for line in output.splitlines()]
            assert records and records[-1]['ok'] is True and records[-1]['data']['event'] == 'stopped'
            with socket.socket() as probe:
                probe.settimeout(1)
                assert probe.connect_ex((host, int(port))) != 0, 'Foreground owner retained its listener after stop'
        finally:
            if child.poll() is None:
                child.kill()
                child.communicate()
        print('PASS: CLI synthetic SSH registry, browser static/absent-profile behavior, foreground NDJSON network owner/rules/stop')


if __name__ == '__main__':
    run()

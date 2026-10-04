#!/usr/bin/env python3
"""Portable synthetic service boundary checks; never counts as Linux systemd evidence."""
import argparse
import json
import os
from pathlib import Path
import platform
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', nargs='?', default=str(ROOT / 'target/debug/lintel'))
    args = parser.parse_args()
    binary = str(Path(args.binary).resolve())
    with tempfile.TemporaryDirectory(prefix='lintel-service-boundary-') as temp:
        home = Path(temp).resolve() / 'home'
        home.mkdir()
        root = home / 'synthetic-root'
        root.mkdir()
        env = dict(os.environ, LINTEL_TEST_HOME=str(home), LINTEL_STATE_DIR=str(Path(temp).resolve() / 'state'))
        def request(command, **fields):
            proc = subprocess.run([binary, 'request'], input=json.dumps(dict(command=command, **fields)),
                                  text=True, capture_output=True, env=env, timeout=30)
            return json.loads(proc.stdout)
        registered = request('register', name='Synthetic service', root=str(root))['data']
        if platform.system() != 'Linux':
            for command in ('service_inspect', 'plan_service_quiesce'):
                r = request(command, environment_id=registered['id'], manager='user', unit='synthetic.service')
                assert r['error']['code'] == 'service_platform_unsupported', r
            assert not list((Path(temp) / 'state/jobs').glob('*.json'))
    print('PASS: synthetic service CLI boundary; systemd runtime is a separate explicit Linux selection')


if __name__ == '__main__':
    main()

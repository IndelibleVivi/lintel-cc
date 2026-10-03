#!/usr/bin/env python3
"""Exercise real CLI/TUI launch and custom policy in a synthetic PTY/home."""
import errno
import json
import os
from pathlib import Path
import pty
import select
import shlex
import subprocess
import tempfile
import time

binary = Path(__file__).resolve().parents[1] / 'target/debug/lintel'
with tempfile.TemporaryDirectory(prefix='lintel-launch-') as tmp:
    fixture = Path(tmp).resolve()
    home = fixture / 'home'
    root = home / "config root's space"
    root.mkdir(parents=True)
    log = fixture / 'launch-observed'
    native = home / '.local/bin/claude'
    native.parent.mkdir(parents=True)
    native.write_text('#!/bin/sh\n'
                      'test -t 0 && test -t 1 || exit 42\n'
                      'printf "%s\\n" "$CLAUDE_CONFIG_DIR" "$PWD" "$#" > '
                      + shlex.quote(str(log)) + '\n')
    native.chmod(0o700)
    env = dict(os.environ, HOME=str(home), LINTEL_TEST_HOME=str(home),
               LINTEL_STATE_DIR=str(fixture / 'state'), PATH='/usr/bin:/bin')
    def request(payload):
        result = subprocess.run([str(binary), 'request'], input=json.dumps(payload),
                                text=True, capture_output=True, env=env, timeout=20)
        response = json.loads(result.stdout)
        assert response['ok'], response
        return response['data']
    environment = request(dict(command='register', name='Synthetic native launch', root=str(root)))
    assert environment['executable'] == str(native)
    denied = subprocess.run([str(binary), 'launch', environment['id']], env=env,
                            capture_output=True, text=True, timeout=20)
    assert denied.returncode and 'terminal_required' in denied.stderr
    assert not log.exists(), 'hidden pipe invoked Claude'
    extra = subprocess.run([str(binary), 'launch', environment['id'], '--prompt'], env=env,
                           capture_output=True, text=True, timeout=20)
    assert extra.returncode and 'Usage:' in extra.stderr
    def terminal(args, input_text=b''):
        master, slave = pty.openpty()
        process = subprocess.Popen([str(binary), *args], env=env, stdin=slave,
                                   stdout=slave, stderr=slave, start_new_session=True)
        os.close(slave)
        output = bytearray()
        try:
            if input_text:
                os.write(master, input_text)
            deadline = time.monotonic() + 20
            while True:
                assert time.monotonic() < deadline, output.decode(errors='replace')
                if select.select([master], [], [], .1)[0]:
                    try:
                        block = os.read(master, 65536)
                    except OSError as error:
                        if error.errno == errno.EIO:
                            break
                        raise
                    if not block:
                        break
                    output.extend(block)
                elif process.poll() is not None:
                    break
            assert process.wait(timeout=2) == 0, output.decode(errors='replace')
        finally:
            os.close(master)
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)
        return output.decode(errors='replace')
    terminal(['launch', environment['id']])
    assert log.read_text().splitlines() == [str(root), str(root), '0']
    log.unlink()  # Only the inert fixture result is removed between its two invocations.
    output = terminal(['tui'], f"o\n{environment['id']}\nq\n".encode())
    assert '打开 Claude' in output
    assert log.read_text().splitlines() == [str(root), str(root), '0']
    settings = root / 'settings.json'
    original = {'env': {'DISABLE_TELEMETRY': 'false', 'DISABLE_ERROR_REPORTING': 'false',
                        'DISABLE_FEEDBACK_COMMAND': 'false', 'DISABLE_GROWTHBOOK': 'true',
                        'UNRELATED': 'keep'}, 'permissions': {'allow': ['Read']}}
    settings.write_text(json.dumps(original))
    # Seven canonical core controls in display order. Invalid action must re-prompt,
    # never silently preserve or apply a different choice. Approval remains separate.
    output = terminal(['tui'], (f"4\n{environment['id']}\ncustom\nn\n\n"
                               "keep\nkeep\ninvalid\ndisable\nkeep\nkeep\nremove\nkeep\napply\nq\n").encode())
    assert '自定义保护' in output and '请输入 keep / disable / remove' in output
    written = json.loads(settings.read_text())
    assert written['env']['DISABLE_FEEDBACK_COMMAND'] == '1'
    assert 'DISABLE_GROWTHBOOK' not in written['env']
    for key in ('DISABLE_TELEMETRY', 'DISABLE_ERROR_REPORTING', 'UNRELATED'):
        assert written['env'][key] == original['env'][key]
    assert written['permissions'] == original['permissions']
    receipt = request(dict(command='jobs'))['jobs'][0]
    assert receipt['policy']['preset'] == 'custom' and receipt['status'] == 'completed'
    restore = request(dict(command='plan_restore', job_id=receipt['id']))
    assert request(dict(command='execute', plan_id=restore['id'], approval=restore['hash']))['status'] == 'completed'
    assert json.loads(settings.read_text()) == original
    print('PASS: native install discovery; CLI/TUI PTY launch; exact root/cwd; zero prompts; hidden-pipe/extra-argument rejection; TUI custom choices/invalid input/approval/receipt/exact restoration')

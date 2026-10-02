import { defineConfig, type Plugin } from 'vite';
import react from '@vitejs/plugin-react';
import { spawn } from 'node:child_process';
import path from 'node:path';
import { tmpdir } from 'node:os';

function fixtureBridge(): Plugin {
  const fixture = process.env.LINTEL_FIXTURE_ROOT;
  const runner = path.resolve('../../target/debug/lintel');
  if (!fixture || !path.resolve(fixture).startsWith(path.join(tmpdir(), 'lintel-ui-fixture-'))) {
    throw new Error('Fixture mode must be started through npm run dev:synthetic.');
  }
  const allowed = new Set(['discover', 'register', 'create_environment', 'inspect', 'plan_policy', 'plan_reset', 'plan_restore', 'execute', 'jobs', 'job', 'drift', 'accept_drift', 'launch', 'export_support']);
  return {
    name: 'lintel-explicit-synthetic-bridge',
    configureServer(server) {
      server.middlewares.use('/__lintel_fixture/request', (req, res) => {
        const origin = req.headers.origin;
        if (req.method !== 'POST' || origin !== 'http://127.0.0.1:1420' || req.headers.host !== '127.0.0.1:1420' || req.headers['content-type'] !== 'application/json' || !['127.0.0.1', '::ffff:127.0.0.1'].includes(req.socket.remoteAddress ?? '')) {
          res.statusCode = 403; res.end('Local fixture access only'); return;
        }
        let body = '';
        req.on('data', chunk => { body += chunk; if (body.length > 65536) req.destroy(); });
        req.on('end', () => {
          let payload: Record<string, unknown>;
          try { payload = JSON.parse(body); } catch { res.statusCode = 400; res.end('Invalid JSON'); return; }
          if (!allowed.has(String(payload.command))) { res.statusCode = 400; res.end('Unsupported fixture command'); return; }
          // Registration stays inside this synthetic home even though the UI
          // accepts arbitrary paths in the real native application.
          const home = path.join(fixture, 'home');
          if (payload.command === 'register' && (typeof payload.root !== 'string' || !path.resolve(payload.root).startsWith(home + path.sep))) {
            res.setHeader('Content-Type', 'application/json'); res.end(JSON.stringify({ ok: false, error: { code: 'FIXTURE_SCOPE', message: '合成预览只能登记此临时测试目录中的环境。真实路径请使用桌面应用。' } })); return;
          }
          const child = spawn(runner, ['request'], { env: { ...process.env, LINTEL_TEST_HOME: home, LINTEL_STATE_DIR: path.join(fixture, 'state') }, stdio: ['pipe', 'pipe', 'pipe'] });
          let output = '';
          let errors = '';
          child.stdout.on('data', value => output += value);
          child.stderr.on('data', value => errors += value);
          child.on('error', () => { if (!res.writableEnded) { res.statusCode = 503; res.end('Build the synthetic runner first'); } });
          child.on('close', () => {
            if (res.writableEnded) return;
            try { JSON.parse(output); res.setHeader('Content-Type', 'application/json'); res.end(output); }
            catch { console.error('Fixture runner returned no envelope', errors); res.statusCode = 502; res.end('Fixture runner unavailable'); }
          });
          child.stdin.end(JSON.stringify(payload) + '\n');
        });
      });
    },
  };
}
export default defineConfig(({ mode }) => ({ plugins: [react(), ...(mode === 'fixture' ? [fixtureBridge()] : [])], server: { host: '127.0.0.1', port: 1420, strictPort: true }, clearScreen: false }));

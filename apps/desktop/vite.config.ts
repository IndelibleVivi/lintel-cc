import { defineConfig, type Plugin } from 'vite';
import react from '@vitejs/plugin-react';
import { spawn, execFileSync } from 'node:child_process';
import path from 'node:path';
import { tmpdir } from 'node:os';
import { existsSync, readdirSync, realpathSync } from 'node:fs';

function buildInfo() {
  try {
    const revision = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
    const dirty = !!execFileSync('git', ['status', '--porcelain', '--untracked-files=normal'], { encoding: 'utf8' }).trim();
    return { version: '0.1.0', docs_revision: dirty ? 'unknown' : revision, source_dirty: dirty };
  } catch { return { version: '0.1.0', docs_revision: 'unknown', source_dirty: true }; }
}
const fixturePort = Number(process.env.LINTEL_FIXTURE_PORT ?? '1420');
function fixtureBridge(): Plugin {
  const fixture = process.env.LINTEL_FIXTURE_ROOT;
  const runner = path.resolve('../../target/debug/lintel');
  if (!fixture || !path.resolve(fixture).startsWith(path.join(tmpdir(), 'lintel-ui-fixture-'))) {
    throw new Error('Fixture mode must be started through npm run dev:synthetic.');
  }
  const allowed = new Set(['discover', 'register', 'create_environment', 'inspect', 'inspect_components', 'work_preflight', 'plan_policy', 'plan_reset', 'plan_archive', 'plan_preserve', 'plan_show', 'plan_restore', 'execute', 'jobs', 'job', 'drift', 'accept_drift', 'launch', 'export_support', 'archive_inspect', 'archive_read', 'plan_import', 'cleanup_inspect', 'plan_cleanup', 'reactivate_environment', 'context', 'session_read', 'plan_launch', 'plan_resume', 'launch_request']);
  return {
    name: 'lintel-explicit-synthetic-bridge',
    configureServer(server) {
      server.middlewares.use('/__lintel_fixture/request', (req, res) => {
        const origin = req.headers.origin;
        if (req.method !== 'POST' || origin !== `http://127.0.0.1:${fixturePort}` || req.headers.host !== `127.0.0.1:${fixturePort}` || req.headers['content-type'] !== 'application/json' || !['127.0.0.1', '::ffff:127.0.0.1'].includes(req.socket.remoteAddress ?? '')) {
          res.statusCode = 403; res.end('Local fixture access only'); return;
        }
        let body = '';
        req.on('data', chunk => { body += chunk; if (body.length > 65536) req.destroy(); });
        req.on('end', () => {
          let payload: Record<string, unknown>;
          try { payload = JSON.parse(body); } catch { res.statusCode = 400; res.end('Invalid JSON'); return; }
          if (!allowed.has(String(payload.command))) { res.statusCode = 400; res.end('Unsupported fixture command'); return; }
          if (['launch','launch_request','resume_request'].includes(String(payload.command))) {
            res.setHeader('Content-Type','application/json');res.end(JSON.stringify({ok:false,error:{code:'FIXTURE_NATIVE_LAUNCH',message:'合成开发预览只核对启动计划；实际 Terminal／PTY 使用独立的 inert 客户端旅程验收。此页面不会启动真实 Claude。'}}));return;
          }
          // Registration stays inside this synthetic home even though the UI
          // accepts arbitrary paths in the real native application.
          const home = path.join(fixture, 'home');
          if (payload.command === 'register' && (typeof payload.root !== 'string' || !path.resolve(payload.root).startsWith(home + path.sep))) {
            res.setHeader('Content-Type', 'application/json'); res.end(JSON.stringify({ ok: false, error: { code: 'FIXTURE_SCOPE', message: '合成预览只能登记此临时测试目录中的环境。真实路径请使用桌面应用。' } })); return;
          }
          for (const key of ['output_path', 'archive_path', 'project_cwd']) {
            if (payload[key] !== undefined && (typeof payload[key] !== 'string' || ![home, realpathSync(home)].some(base => path.resolve(String(payload[key])).startsWith(base + path.sep)))) {
              res.setHeader('Content-Type', 'application/json'); res.end(JSON.stringify({ ok: false, error: { code: 'FIXTURE_SCOPE', message: '测试空间只允许读取或另存此合成 home 内的工作包。' } })); return;
            }
          }
          const child = spawn(runner, ['request'], { env: { ...process.env, HOME: home, LINTEL_TEST_HOME: home, LINTEL_STATE_DIR: path.join(fixture, 'state') }, stdio: ['pipe', 'pipe', 'pipe'] });
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
const localFontDirectory = path.resolve('public/local-fonts');
const localFontUrls = existsSync(localFontDirectory) ? readdirSync(localFontDirectory).filter(file => /^Anthropic(Sans|Serif|Mono)-(Roman|Italic)\.woff2$/.test(file)).map(file => `/local-fonts/${file}`) : [];
export default defineConfig(({ command, mode }) => {
  // public/ currently contains only optional operator-supplied fonts. Ordinary
  // candidate builds must not carry those local assets into someone else's App.
  const localAssets = command === 'serve' || mode === 'local-candidate';
  return { define: { __LINTEL_BUILD__: JSON.stringify(buildInfo()), __LINTEL_LOCAL_FONTS__: JSON.stringify(localAssets ? localFontUrls : []) }, publicDir: localAssets ? 'public' : false, plugins: [react(), ...(mode === 'fixture' ? [fixtureBridge()] : [])], server: { host: '127.0.0.1', port: fixturePort, strictPort: true }, clearScreen: false };
});

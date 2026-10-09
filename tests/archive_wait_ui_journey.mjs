// Built React App; delayed invoke responses are visibly synthetic and do not
// measure decryption performance or prove native WebKit behavior.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { stripVTControlCharacters } from 'node:util';

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const desktop = path.join(repo, 'apps/desktop');
const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE || path.join(repo, 'extensions/browser/node_modules/playwright/index.mjs')).href);
const fixture = await mkdtemp(path.join(os.tmpdir(), 'lintel-archive-wait-ui-'));
const environment = { id: 'synthetic-environment', name: '合成阅读环境', root: '/synthetic/config', executable: null, status: 'active' };
const context = { user: { home: '/synthetic/home', uid: 501, euid: 501 }, state: { path: '/synthetic/state', source: 'synthetic', exists: true } };
const manifest = { schema: 'lintel.work/1', files: [{ path: 'projects/demo/session.jsonl', category: 'sessions', digest: 'a'.repeat(64), bytes: 524288 }], notes: '合成工作包，用于等待状态验收。' };
const calls = []; let pendingRead, pendingUnlock, unlockFailure = false, legacyMissingRecords = false;
function gate(set) { return new Promise(resolve => set(resolve)); }
const preview = spawn(process.execPath, [path.join(desktop, 'node_modules/vite/bin/vite.js'), 'preview', '--host', '127.0.0.1', '--port', '0', '--strictPort'], { cwd: desktop, stdio: ['ignore', 'pipe', 'pipe'] });
const report = { fixture, runtime: 'built App + delayed synthetic invoke; not real package performance or native WebKit', checks: [], passed: false };
let browser;
try {
  const url = await new Promise((resolve, reject) => {
    let output = ''; const timer = setTimeout(() => reject(new Error('preview timeout: ' + output)), 15000);
    preview.once('error', reject);
    for (const stream of [preview.stdout, preview.stderr]) stream.on('data', data => {
      output += stripVTControlCharacters(data.toString()); const match = output.match(/http:\/\/127\.0\.0\.1:\d+\//);
      if (match) { clearTimeout(timer); resolve(match[0]); }
    });
  });
  report.url = url; browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1120, height: 760 } }); page.setDefaultTimeout(8000);
  const errors = []; page.on('pageerror', error => errors.push(error.message));
  await page.exposeFunction('syntheticInvoke', async (command, args) => {
    if (command === 'inspect_cli') return { ok: true, data: { executable: '/synthetic/bin/lintel', version: { version: '0.1.0', protocol: 1, platform: 'macos', architecture: 'aarch64' }, context, candidate: { status: 'unknown' }, checked_at: 1 } };
    assert.equal(command, 'request'); const payload = args.payload; calls.push(payload.command);
    let data;
    switch (payload.command) {
      case 'discover': data = { environments: [environment], capabilities: [] }; break;
      case 'jobs': data = { jobs: [] }; break;
      case 'inspect': data = { settings: [], assets: [], warnings: [] }; break;
      case 'context': data = context; break;
      case 'archive_inspect': {
        await gate(resolve => { pendingUnlock = resolve; });
        if (unlockFailure) return { ok: false, error: { code: 'archive_locked', message: '合成错误口令' } };
        data = manifest; break;
      }
      case 'session_read': {
        await gate(resolve => { pendingRead = resolve; });
        const at = payload.offset ?? 0;
        data = { path: payload.path, digest: manifest.files[0].digest, source: { package_digest: 'b'.repeat(64) }, content_kind: legacyMissingRecords ? 'messages' : 'text', raw_text: at ? 'SYNTHETIC_NEXT_PAGE' : 'SYNTHETIC_FIRST_PAGE', offset: at, total_bytes: 524288, page_bytes: 262144, next_offset: at ? null : 262144, done: !!at };
        break;
      }
      default: throw new Error('unexpected synthetic command: ' + payload.command);
    }
    return { ok: true, data };
  });
  await page.addInitScript(() => { window.isTauri = true; window.__TAURI_INTERNALS__ = { invoke: (command, args) => window.syntheticInvoke(command, args) }; });
  await page.goto(url); await page.locator('.home-title').waitFor();
  const navigate = name => page.locator('.sidebar nav').getByRole('button', { name, exact: true }).click();
  const activity = page.getByRole('status', { name: '工作包读取状态' });
  async function unlock() {
    await page.getByLabel('加密包完整路径').fill('/synthetic/carried.age');
    await page.getByPlaceholder('当前交互临时使用').fill('SYNTHETIC_SECRET_NEVER_IN_STATUS');
    await page.getByRole('button', { name: '解锁并查看', exact: true }).click();
    await activity.getByText('本机 · 正在核验并解锁工作包', { exact: true }).waitFor();
    pendingUnlock(); await page.locator('.archive-files button').waitFor();
  }
  await navigate('会话与资料'); await unlock();
  await page.locator('.archive-files button').click();
  await page.getByRole('status').getByText('正在核验工作包并读取这一页', { exact: true }).waitFor();
  assert.equal(await page.locator('.session-reader').getAttribute('aria-busy'), 'true');
  assert.equal(await page.getByRole('heading', { name: '选择一份原件', exact: true }).count(), 0);
  await page.screenshot({ path: path.join(fixture, 'first-read-1120-day.png') });
  pendingRead(); await page.getByText('SYNTHETIC_FIRST_PAGE', { exact: true }).waitFor();
  assert.equal(await page.locator('.session-reader').getAttribute('aria-busy'), 'false');
  assert.equal(await page.getByRole('button', { name: '结构化阅读', exact: true }).isDisabled(), true, 'a real text-shaped response does not provide records');
  await page.getByRole('button', { name: '原始文本', exact: true }).click();
  assert.equal(await page.getByText('SYNTHETIC_FIRST_PAGE', { exact: true }).isVisible(), true);
  // An older messages response may omit records too. Its structured view must
  // show the finite limitation and permit returning to the original bytes.
  legacyMissingRecords = true;
  await page.locator('.archive-files button').click();
  await page.locator('.reader-pending').waitFor(); pendingRead();
  await page.getByText('识别到消息记录', { exact: true }).waitFor();
  await page.getByRole('button', { name: '结构化阅读', exact: true }).click();
  await page.getByText('本页没有识别出的正文；使用原始文本查看格式限制。', { exact: true }).waitFor();
  await page.getByRole('button', { name: '原始文本', exact: true }).click();
  await page.getByText('SYNTHETIC_FIRST_PAGE', { exact: true }).waitFor(); legacyMissingRecords = false;
  report.checks.push('text pages omit records and expose only raw view; legacy messages without records show a bounded limitation and return to raw text without a render error');
  await page.getByRole('button', { name: '下一页', exact: true }).focus(); await page.keyboard.press('Enter');
  await page.locator('.reader-pending').getByText(/下方仍显示上一页/).waitFor();
  assert.equal(await page.getByText('SYNTHETIC_FIRST_PAGE', { exact: true }).isVisible(), true);
  assert.equal(await page.getByRole('button', { name: '下一页', exact: true }).isDisabled(), true);
  assert.ok(!(await activity.innerText()).includes('SYNTHETIC_SECRET_NEVER_IN_STATUS'));
  await page.setViewportSize({ width: 900, height: 640 }); await page.evaluate(() => document.documentElement.dataset.theme = 'dark');
  assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
  await page.locator('.reader-pending').scrollIntoViewIfNeeded(); await page.screenshot({ path: path.join(fixture, 'next-read-900-night.png') });
  report.checks.push('first read and keyboard next-page activation: explicit pending/status/aria-busy, previous-page label, no phantom empty prompt, 1120 Day and 900 Night without horizontal overflow');
  await navigate('终端与 Agent'); await activity.getByText(/已提交的读取仍会继续/).waitFor();
  await page.getByPlaceholder('/path/to/lintel-cli/bin/lintel').fill('/synthetic/bin/lintel');
  await page.getByRole('button', { name: '核对 CLI 与上下文', exact: true }).click();
  await activity.getByText(/还有 1 个请求等待/).waitFor();
  assert.equal(calls.includes('context'), false, 'context stays queued behind the original read');
  await page.screenshot({ path: path.join(fixture, 'agent-wait-900-night.png') });
  pendingRead(); await page.getByLabel('Agent 操作交接包').waitFor(); await activity.waitFor({ state: 'hidden' });
  assert.equal(calls.filter(command => command === 'context').length, 1);
  assert.equal(await page.getByText('SYNTHETIC_NEXT_PAGE', { exact: true }).count(), 0, 'late reader results do not restore private content on another page');
  await navigate('会话与资料');
  assert.equal(await page.getByPlaceholder('当前交互临时使用').inputValue(), '');
  assert.equal(await page.locator('.archive-files button').count(), 0);
  report.checks.push('leaving the page retains honest queue feedback, waits without bypassing core serialization, releases queued Agent context, clears private reader state, and ignores late pages');
  // A failed queued operation must release both the status and the queue.
  await page.getByLabel('加密包完整路径').fill('/synthetic/wrong.age');
  await page.getByPlaceholder('当前交互临时使用').fill('wrong synthetic secret'); unlockFailure = true;
  await page.getByRole('button', { name: '解锁并查看', exact: true }).click(); await activity.waitFor(); pendingUnlock();
  await page.getByRole('alert').getByText(/合成错误口令/).waitFor(); await activity.waitFor({ state: 'hidden' });
  await navigate('终端与 Agent'); await page.getByRole('button', { name: '核对 CLI 与上下文', exact: true }).click();
  await page.getByLabel('Agent 操作交接包').waitFor();
  report.checks.push('failed authentication releases the queue and its status; following context request succeeds');
  assert.deepEqual(errors, []); report.passed = true;
} finally {
  await browser?.close(); preview.kill(); await writeFile(path.join(fixture, 'report.json'), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report));
}

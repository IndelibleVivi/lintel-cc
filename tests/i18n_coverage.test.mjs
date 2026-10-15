// Static coverage: every translation key referenced in the App source must exist
// in BOTH catalogs. This is the guard against an unmigrated string silently
// falling back to the raw key.
//
// Three reference forms are scanned:
//   1. literal `t('a.b.c')` / `t("a.b.c")` calls;
//   2. template-literal dynamic keys built from a finite, enumerable family
//      (`album.${i}.*`, greeting keys) — enumerated below;
//   3. status-code maps in ui.tsx (validated by key membership here too).
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { zhCN } from '../apps/desktop/src/i18n.catalog.zh-CN.ts';
import { en } from '../apps/desktop/src/i18n.catalog.en.ts';
import { PRESENTATION_KEYS, present } from '../apps/desktop/src/i18n-presentation.ts';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const srcDir = path.join(root, 'apps/desktop/src');
const files = readdirSync(srcDir).filter(name => /\.(tsx|ts)$/.test(name) && !name.startsWith('i18n.catalog'));

// Both quote styles; `t("a.b.c")` and `t('a.b.c')`. Also `t(`a.b.${...}`)`
// template keys are matched with their static prefix and checked against the
// finite families below (dynamic suffixes are enumerated explicitly).
const singleKey = /\bt\(\s*'([a-zA-Z0-9_.]+)'/g;
const doubleKey = /\bt\(\s*"([a-zA-Z0-9_.]+)"/g;
// Conditional/key-map literals inside `t(...)`: a single-quoted dotted key used
// as the argument, e.g. `t(x ? 'a.b' : 'c.d')`. Any dotted literal that looks
// like a translation key is captured by scanning all dotted single-quoted
// strings and keeping the ones that start with a known key root.
const dottedLiteral = /'([a-z][a-zA-Z0-9_]*\.(?:[a-zA-Z0-9_]+\.)*[a-zA-Z0-9_]+)'/g;
// Roots whose keys are catalogs, not incidental dotted strings (CSS classes,
// file extensions, etc). Only literals under these roots are validated.
const KEY_ROOTS = new Set([
  'app','nav','label','home','overview','tab','state','launch','browser','policy','work','cleanup',
  'sessions','agent','history','settings','help','support','playroom','album','clawd','status','format',
  'time','resources','category','planReview','service','launchPanel','startup','failure','outcome','coverage',
  'launchRecords','workFiles','capacity','component','customPolicy','net','netPanel','netPlan','netReceipt',
  'ipnet','telemetry','updates','remote','api','package','agentPanel','bpanel','bhost','bext','greeting',
]);

test('every literal t() key (single and double quoted) in apps/desktop/src exists in both catalogs', () => {
  const missing = [];
  for (const file of files) {
    const text = readFileSync(path.join(srcDir, file), 'utf8');
    for (const re of [singleKey, doubleKey]) for (const match of text.matchAll(new RegExp(re.source, 'g'))) {
      const key = match[1];
      if (!(key in zhCN)) missing.push(`${file}: zh-CN missing ${key}`);
      if (!(key in en)) missing.push(`${file}: en missing ${key}`);
    }
  }
  assert.deepEqual(missing, [], missing.join('\n'));
});

test('dotted string literals under known key roots all resolve', () => {
  // Catches computed/conditional keys such as `t(cond ? 'a.b' : 'c.d')` that the
  // direct-call regex cannot see, while ignoring non-key dotted strings.
  const missing = [];
  for (const file of files) {
    const text = readFileSync(path.join(srcDir, file), 'utf8');
    for (const match of text.matchAll(new RegExp(dottedLiteral.source, 'g'))) {
      const key = match[1];
      const root = key.split('.')[0];
      if (!KEY_ROOTS.has(root)) continue;
      if (!(key in zhCN)) missing.push(`${file}: zh-CN missing ${key}`);
      if (!(key in en)) missing.push(`${file}: en missing ${key}`);
    }
  }
  assert.deepEqual(missing, [], missing.join('\n'));
});

test('dynamic key families resolve for every index they can take', () => {
  // The landscape album builds `album.<index>.<field>` for four plates.
  for (let i = 0; i < 4; i++) {
    for (const field of ['title', 'subtitle', 'description', 'secret']) {
      const key = `album.${i}.${field}`;
      assert.ok(key in zhCN, `zh-CN missing ${key}`);
      assert.ok(key in en, `en missing ${key}`);
    }
  }
  // Greeting identities: every period line, rare line, date and time egg.
  for (const period of ['morning', 'afternoon', 'evening', 'late']) {
    for (let i = 0; i < 14; i++) {
      const key = `greeting.line.${period}.${i}`;
      assert.ok(key in zhCN, `zh-CN missing ${key}`);
      assert.ok(key in en, `en missing ${key}`);
    }
  }
  for (let i = 0; i < 6; i++) assert.ok(`greeting.rare.${i}` in zhCN && `greeting.rare.${i}` in en);
});

test('the UI status vocabulary maps to real keys', () => {
  const ui = readFileSync(path.join(srcDir, 'ui.tsx'), 'utf8');
  const statusKeys = [...ui.matchAll(/'status\.[a-zA-Z0-9]+'/g)].map(m => m[0].slice(1, -1));
  assert.ok(statusKeys.length > 0, 'status vocabulary must have catalog keys');
  for (const key of new Set(statusKeys)) {
    assert.ok(key in zhCN, `zh-CN missing ${key}`);
    assert.ok(key in en, `en missing ${key}`);
  }
});

test('both catalogs expose the same keys (parity)', () => {
  assert.deepEqual(Object.keys(en).sort(), Object.keys(zhCN).sort());
});

test('presentation projection map is internally consistent and complete in both catalogs', () => {
  const sources = Object.keys(PRESENTATION_KEYS);
  assert.ok(sources.length > 0, 'core presentation projection must be populated');
  for (const source of sources) {
    const key = PRESENTATION_KEYS[source];
    assert.ok(key in zhCN, `zh-CN missing ${key}`);
    assert.ok(key in en, `en missing ${key}`);
    // The zh catalog holds the ORIGINAL source text for verified projections,
    // so the projection is exactly "verified source text -> catalog key".
    assert.equal(zhCN[key], source, `zh-CN ${key} must equal the verified source text`);
    // The English value is a real translation (differs unless genuinely shared).
    assert.equal(typeof en[key], 'string');
    assert.notEqual(en[key].trim(), '');
  }
  // Spot-check a representative explanation and preserve projections.
  assert.equal(present('所有原始工作内容与项目文件', 'en'), 'all original work content and project files');
  assert.equal(present('配置已写入；新启动及网络效果尚未验证。', 'en'), 'Configuration written; a new launch and network effect are not yet verified.');
  assert.equal(present('服务和路径说明（未知）', 'en'), '服务和路径说明（未知）', 'unknown stays raw');
});


test('every projected backend phrase is present in a canonical source, not invented in the App', () => {
  function sourceText(directory) {
    return readdirSync(directory,{withFileTypes:true}).flatMap(entry=>{
      const file=path.join(directory,entry.name);
      if(entry.isDirectory()&&!['target','node_modules'].includes(entry.name))return sourceText(file);
      return entry.isFile()&&/\.(rs|json)$/.test(entry.name)?[readFileSync(file,'utf8')]:[];
    });
  }
  const canonical=['crates','apps/desktop/src-tauri/src','contracts'].flatMap(relative=>sourceText(path.join(root,relative))).join('\n');
  for(const phrase of Object.keys(PRESENTATION_KEYS))assert.ok(canonical.includes(phrase),`projection has no canonical source: ${phrase}`);
});

// Guard the specific regression this suite was extended for: core ships the
// policy rule labels as canonical Chinese, so every place that renders a
// backend display label must project it rather than print the raw value.
// SettingsRows is the "Current configuration" list on Protection plan (and the
// same rows are reused by DriftCheck), so it must go through present().
test('backend display labels render through the presentation projection, not raw', () => {
  const text = file => readFileSync(path.join(srcDir, file), 'utf8');
  // SettingsRows is authored as a single-line component; scope the check to
  // that declaration line so neighbouring functions cannot satisfy it.
  const settingsRows = text('App.tsx').split('\n').find(line => line.startsWith('function SettingsRows'));
  assert.ok(settingsRows, 'SettingsRows must exist in App.tsx');
  assert.ok(/present\(setting\.label,\s*locale\)/.test(settingsRows),
    'SettingsRows must project the backend setting label through present(setting.label, locale)');
  // A bare {setting.label} render is exactly the bug: the theme labels came
  // through untranslated in an English UI.
  assert.ok(!/\{setting\.label\}/.test(settingsRows),
    'SettingsRows must not render the raw backend setting.label');
});

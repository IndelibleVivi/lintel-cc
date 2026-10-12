import test from 'node:test';
import assert from 'node:assert/strict';
import {
  LocalizedError, LOCALES, fallbackLocale, resolveLocale, sanitizePreferredLanguages, normalizeTag, catalogForTag,
  normalizeLanguageSetting, translate, formatDateTime, formatTime, formatNumber, MAX_PREFERRED_LANGUAGES,
} from '../apps/desktop/src/i18n.ts';
import { zhCN } from '../apps/desktop/src/i18n.catalog.zh-CN.ts';
import { en } from '../apps/desktop/src/i18n.catalog.en.ts';
import { homeGreeting, GREETING_DATE_KEY } from '../apps/desktop/src/homeGreetings.ts';
import { present, PRESENTATION_KEYS } from '../apps/desktop/src/i18n-presentation.ts';

const catalogs = { 'zh-CN': zhCN, en };
const placeholders = value => [...value.matchAll(/\{(\w+)\}/g)].map(m => m[1]).sort();

test('the two catalogs are a complete, flat, matching key set with matching placeholders', () => {
  const zhKeys = Object.keys(zhCN).sort();
  const enKeys = Object.keys(en).sort();
  assert.deepEqual(enKeys, zhKeys, 'en and zh-CN key sets must be identical');
  assert.ok(zhKeys.length > 0, 'the catalogs contain App wording');
  for (const key of zhKeys) {
    assert.equal(typeof zhCN[key], 'string', `zh-CN ${key} must be a string`);
    assert.equal(typeof en[key], 'string', `en ${key} must be a string`);
    assert.notEqual(zhCN[key].trim(), '', `zh-CN ${key} must not be empty`);
    assert.notEqual(en[key].trim(), '', `en ${key} must not be empty`);
    assert.deepEqual(placeholders(en[key]), placeholders(zhCN[key]), `placeholder mismatch for ${key}`);
    // No nested dotted traversal: a key must be exact, never resolved as a path.
    assert.ok(!key.includes(' '), `key ${key} must be a dotted identifier`);
  }
});

test('every statically referenced key exists in both catalogs (no silent key fallthrough)', () => {
  // Finite allowlist of source-controlled keys the App renders. This list is
  // maintained next to the migration; a new literal site must be added here.
  const used = [
    'language.system','language.select','language.hint',
    'nav.environments','nav.policy','nav.work','nav.rebuild','nav.history','nav.sessions','nav.agent',
  ];
  for (const key of used) {
    assert.ok(key in zhCN, `zh-CN missing used key ${key}`);
    assert.ok(key in en, `en missing used key ${key}`);
  }
});

test('translate uses a flat lookup, interpolates, and falls back finitely', () => {
  assert.equal(translate(catalogs, 'zh-CN', 'nav.work'), '工作保全');
  assert.equal(translate(catalogs, 'en', 'nav.work'), 'Preserve work');
  assert.equal(translate(catalogs, 'en', 'app.pendingReview', { count: 3 }), '3 task(s) need review');
  assert.equal(translate(catalogs, 'zh-CN', 'app.pendingReview', { count: 3 }), '3 份任务需要核对');
  // A missing key resolves to the visible key, never an empty string.
  assert.equal(translate(catalogs, 'en', 'does.not.exist'), 'does.not.exist');
  for (const key of ['constructor','toString','__proto__']) assert.equal(translate(catalogs, 'en', key), key);
  // A missing placeholder value is left visible, not blanked.
  assert.equal(translate(catalogs, 'en', 'app.pendingReview'), '{count} task(s) need review');
});

test('resolution: zh variants, en variants, unsupported/invalid and empty preferences', () => {
  for (const [pref, expected] of [[['zh'],'zh-CN'],[['zh-CN'],'zh-CN'],[['zh-Hans'],'zh-CN'],[['zh-Hant-TW'],'zh-CN'],
    [['en'],'en'],[['en-US'],'en'],[['en-GB'],'en']]) {
    assert.equal(resolveLocale('system', pref).locale, expected, `${pref} -> ${expected}`);
  }
  // Unsupported languages fall back to English (the unified fallback).
  assert.equal(fallbackLocale, 'en');
  assert.equal(resolveLocale('system', ['fr-FR','de-DE']).locale, 'en');
  assert.equal(resolveLocale('system', []).locale, 'en');
  assert.equal(resolveLocale('system', null).locale, 'en');
  assert.equal(resolveLocale('system', ['','   ','???','a'.repeat(200),123,null]).locale, 'en');
  // First matching supported preference wins.
  assert.equal(resolveLocale('system', ['fr','zh-Hans','en-US']).locale, 'zh-CN');
  assert.equal(resolveLocale('system', ['de','en-US','zh-CN']).locale, 'en');
  // A manual override ignores the preference list entirely.
  assert.deepEqual(resolveLocale('zh-CN', ['en-US']).locale, 'zh-CN');
  assert.deepEqual(resolveLocale('en', ['zh-Hans']).locale, 'en');
  assert.equal(resolveLocale('en', ['zh-Hans']).setting, 'en');
});

test('preference sanitizing: plausible tags, dedupe, finite cap, invalid rejected', () => {
  assert.deepEqual(sanitizePreferredLanguages(['en-US','en-US','zh-Hans']), ['en-US','zh-Hans']);
  assert.deepEqual(sanitizePreferredLanguages('not-an-array'), []);
  assert.deepEqual(sanitizePreferredLanguages([1, 2, {}]), []);
  assert.equal(sanitizePreferredLanguages(Array.from({length: 20}, (_, i) => `lang${'abcdefghijklmnopqrstuvwxyz'[i]}x`)).length, MAX_PREFERRED_LANGUAGES);
  assert.equal(normalizeTag('  en-GB  '), 'en-GB');
  assert.equal(normalizeTag(''), null);
  assert.equal(normalizeTag('x'), null);
  assert.equal(normalizeTag('aaaaaaaaa'), null);
  assert.equal(catalogForTag('zh-Hans-CN'), 'zh-CN');
  assert.equal(catalogForTag('EN'), 'en');
  assert.equal(catalogForTag('fr'), null);
});

test('normalizeLanguageSetting accepts only finite known values', () => {
  for (const good of ['system','zh-CN','en']) assert.equal(normalizeLanguageSetting(good), good);
  for (const bad of ['', 'zh', 'EN', null, undefined, 5, {}]) assert.equal(normalizeLanguageSetting(bad), 'system');
});

test('locale-aware formatting differs by locale and stays stable', () => {
  const iso = '2026-10-13T09:00:00Z';
  assert.equal(formatDateTime(iso, 'en'), formatDateTime(iso, 'en'));
  assert.notEqual(formatDateTime(iso, 'en'), formatDateTime(iso, 'zh-CN'));
  assert.match(formatDateTime(iso, 'en'), /Oct/);
  assert.match(formatDateTime(iso, 'zh-CN'), /10月/);
  assert.notEqual(formatTime(new Date(iso), 'en'), formatTime(new Date(iso), 'zh-CN'));
  assert.equal(formatNumber(12345, 'en'), '12,345');
  // An unparsable value is returned unchanged rather than inventing a date.
  assert.equal(formatDateTime('not-a-date', 'en'), 'not-a-date');
});

test('greeting identity is stable across a language switch with one date egg', () => {
  const storage = () => { const m = new Map(); return { getItem: k => m.get(k) ?? null, setItem: (k, v) => m.set(k, v), map: m }; };
  const store = storage();
  const chosen = homeGreeting(new Date(2026, 9, 13, 0, 0, 0), () => 0, store);
  assert.equal(chosen.text, '一周年。门还开着。');
  assert.deepEqual([...store.map], [[GREETING_DATE_KEY, '2026-10-13']]);
  // Same identity, translated: the en catalog holds the matching projection.
  assert.equal(chosen.key, 'greeting.date.10-13');
  assert.ok(chosen.key in zhCN && chosen.key in en);
  assert.equal(zhCN[chosen.key], chosen.text);
  assert.equal(en[chosen.key], 'One year. The door’s still open.');
  // A second opening the same day does not re-consume the date egg.
  const next = homeGreeting(new Date(2026, 9, 13, 9, 0, 0), () => 0.5, store);
  assert.notEqual(next.code, 'date:10-13');
  assert.ok(next.key in zhCN && next.key in en, 'ordinary greeting has a catalog identity too');
});

test('greeting pools are derived from the catalog (single source)', () => {
  // The chooser must not maintain its own copy of the Chinese greeting text.
  const morningKeys = Object.keys(zhCN).filter(key => key.startsWith('greeting.line.morning.'));
  assert.equal(morningKeys.length, 14, 'fourteen morning lines (8 zh + 6 mixed)');
  assert.equal(Object.keys(zhCN).filter(key => key.startsWith('greeting.line.')).length, 56);
  const rareKeys = Object.keys(zhCN).filter(key => key.startsWith('greeting.rare.'));
  assert.equal(rareKeys.length, 6);
});

test('presentation projection maps verified source-controlled text and passes unknowns raw', () => {
  assert.equal(present('减少外发', 'en'), 'Reduce outbound');
  assert.equal(present('减少外发', 'zh-CN'), '减少外发');
  assert.equal(present('保留内容，准备新环境', 'en'), 'Keep content, prepare a new environment');
  assert.equal(present('精确字段写入与读回', 'en'), 'Exact field write and readback');
  // Unknown / user data passes through unchanged, in both locales.
  assert.equal(present('/Users/me/secret plan', 'en'), '/Users/me/secret plan');
  assert.equal(present('A user-typed title', 'zh-CN'), 'A user-typed title');
  assert.equal(present('a'.repeat(64), 'en'), 'a'.repeat(64));
  assert.equal(present('', 'en'), '');
  assert.equal(present(null, 'en'), '');
  assert.equal(present(undefined, 'zh-CN'), '');
});

test('every presentation projection key resolves in both catalogs', () => {
  for (const [source, key] of Object.entries(PRESENTATION_KEYS)) {
    assert.ok(key in zhCN, `zh-CN missing ${key} (for ${source})`);
    assert.ok(key in en, `en missing ${key} (for ${source})`);
    // The zh catalog holds the original source text for verified projections.
    assert.equal(zhCN[key], source, `zh-CN ${key} should equal the source text`);
  }
});

test('presentation covers the real current core display fields and preserves raw identity', () => {
  // Real current source strings from canonical crates (verified by exact match).
  assert.equal(present('精确字段预览、读回与恢复；运行时效果另行验证', 'en'),
    'Exact field preview, readback and restore; runtime effect verified separately');
  assert.equal(present('所有原始工作内容与项目文件', 'en'), 'all original work content and project files');
  assert.equal(present('配置已写入；新启动及网络效果尚未验证。', 'en'),
    'Configuration written; a new launch and network effect are not yet verified.');
  assert.equal(present('相邻 service 与 SSH', 'en'), 'neighboring services and SSH');
  // Raw identity/diagnostic values are never projected, in either locale.
  const raw = ['constructor', 'toString', '__proto__', '/Users/me/config', 'a'.repeat(64), 'plan-raw-1234', 'rm -rf /tmp/x', 'SENTINEL_WORK_BODY', '{"env":{"X":"y"}}'];
  for (const value of raw) for (const locale of ['en','zh-CN']) assert.equal(present(value, locale), value, `${locale} keeps ${value} raw`);
});


test('UI-owned errors retain finite wording identity and exact interpolation data', () => {
  const args={reason:'UNKNOWN raw reason',unobserved:' · /synthetic/unobserved'};
  const error=new LocalizedError('workFiles.incomplete',args);
  assert.equal(error.key,'workFiles.incomplete');
  assert.deepEqual(error.args,args);
  assert.notEqual(translate(catalogs,'en',error.key,error.args),translate(catalogs,'zh-CN',error.key,error.args));
  for(const locale of ['en','zh-CN']){
    const rendered=translate(catalogs,locale,error.key,error.args);
    assert.ok(rendered.includes(args.reason));assert.ok(rendered.includes(args.unobserved));
  }
});

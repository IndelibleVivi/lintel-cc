// Finite source/generation checks are also imported by the canonical site-ui journey.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { SITE_COPY, ENGLISH_GUIDES } from '../apps/site/site-copy.mjs';
import { checkSiteLanguages, projectSiteHtml } from '../scripts/prepare-site-languages.mjs';
import { siteGuideHref, siteLanguageFromPath } from '../apps/site/site-language.mjs';
const source = await readFile(new URL('../apps/site/index.html',import.meta.url),'utf8');
const english = await readFile(new URL('../apps/site/en/index.html',import.meta.url),'utf8');

test('paired copy generates exactly the tracked no-JS English page', async () => {
  assert.deepEqual((await checkSiteLanguages()).languages,['zh-CN','en']);
  assert.equal(english,projectSiteHtml(source));
  for (const value of Object.values(SITE_COPY)) {
    assert.equal(typeof value.zh,'string'); assert.ok(value.zh.length);
    assert.equal(typeof value.en,'string'); assert.ok(value.en.length);
  }
  for (const [html,lang,url] of [[source,'zh-CN','https://lintel.page/'],[english,'en','https://lintel.page/en/']]) {
    assert.match(html,new RegExp(`<html lang="${lang}"`));
    assert.ok(html.includes(`rel="canonical" href="${url}"`));
    assert.ok(html.includes(`property="og:url" content="${url}"`));
    for (const language of ['zh-CN','en','x-default']) assert.ok(html.includes(`hreflang="${language}"`));
    assert.equal([...html.matchAll(/<article data-capability=/g)].length,6);
    assert.equal([...html.matchAll(/<svg class="capability-icon"[^>]*aria-hidden="true"/g)].length,6);
    assert.ok(!html.includes('class="capability-visual"'), 'retired large scenes do not return');
    assert.ok(html.includes('preload="none"')); assert.ok(!html.includes('autoplay'));
    assert.ok(!/(?:href|src|poster)="\.\//.test(html),'shared asset paths resolve at both locale URLs');
  }
});

test('source changes reject missing keys, stale pairs and uncovered Chinese copy', () => {
  assert.throws(()=>projectSiteHtml(source.replace('data-site-copy="c004"','data-site-copy="missing"')),/Missing or stale/);
  assert.throws(()=>projectSiteHtml(source.replace('跳到正文</span>','跳到新的正文</span>')),/Missing or stale/);
  assert.throws(()=>projectSiteHtml(source.replace('</main>','<p>新的未配对文案</p></main>')),/Unbound Chinese/);
  assert.throws(()=>projectSiteHtml(source,'fr'),/Unsupported website language/);
});

test('identity drawings and synthetic originals stay byte-identical across locales', () => {
  const blocks = html => [...html.matchAll(/<!-- lintel-(?:mark|wordmark):start -->([\s\S]*?)<!-- lintel-(?:mark|wordmark):end -->/g)].map(match=>match[1]);
  assert.deepEqual(blocks(english),blocks(source));
  const originals = html => [...html.matchAll(/<pre>([\s\S]*?)<\/pre>/g)].map(match=>match[1]);
  assert.deepEqual(originals(english),originals(source));
  assert.match(english,/Current capabilities and limits \(Chinese\)/);
  assert.match(english,/No formal download/);
});

test('language and guide routing are finite and preserve stable section anchors', async () => {
  assert.equal(siteLanguageFromPath('/'),'zh'); assert.equal(siteLanguageFromPath('/en/'),'en');
  assert.equal(siteLanguageFromPath('/english'),'zh');
  const base='https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/';
  for (const guide of ENGLISH_GUIDES) {
    assert.equal(siteGuideHref(`${base}${guide}.md#work`,'en'),`${base}${guide}.en.md#work`);
    assert.equal(siteGuideHref(`${base}${guide}.en.md#work`,'zh'),`${base}${guide}.md#work`);
    await readFile(new URL(`../docs/${guide}.en.md`,import.meta.url),'utf8');
  }
  assert.equal(siteGuideHref(`${base}current-state.md`,'en'),`${base}current-state.md`);
  assert.equal(siteGuideHref('https://example.com/operator-guide.md','en'),'https://example.com/operator-guide.md');
  const operator = await readFile(new URL('../docs/operator-guide.en.md',import.meta.url),'utf8');
  for (const anchor of ['protect','work','cleanup','recovery']) assert.ok(operator.includes(`<a id="${anchor}"></a>`),`English operator guide retains #${anchor}`);
});

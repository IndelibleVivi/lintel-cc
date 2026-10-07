// Disposable browser context; website only. No Claude, native bridge or personal profile.
//
// New stable page contract (coordinator-owned markup):
//   #top       hero: native .brand-wordmark-svg lockup, the terracotta tail overlapping the
//              final l; day/night raster plates assets/lintel-landscape{,-night}.png; the
//              decorative .cursor blinks slowly in the header/footer and stays still under
//              prefers-reduced-motion.
//   #journey   start of the product body copy.
//   #keep #review #continue   three Chinese product-narrative sections (no motion gating).
//   #sample-pages   native <details>/<summary> reading example with explicit synthetic
//                   CLAUDE.md / MEMORY.md / session.jsonl snippets. No reader editing/writing,
//                   clipboard, crypto or package-generation control.
//   #try       scope/limits, the unverified-download boundary, CN/EN quickstart and six docs links.
//   #clawd-game   the single shared clawd-game.mjs/.css runner, opt-in.
// The superseded six-chapter fake-window / checkbox / draft demo is gone and must not return.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE || path.join(repo, 'extensions/browser/node_modules/playwright/index.mjs')).href);
// Finite named asset map only: the server never serves arbitrary files.
const assets = new Map([['/', 'index.html'], ...[
  'index.html','styles.css','site.mjs',
  'clawd-game.mjs','clawd-game.css',
  'assets/favicon.svg',
  'assets/lintel-landscape.png','assets/lintel-landscape-night.png',
  'assets/lintel-keep.png','assets/lintel-crossing.png',
].map(file => [`/${file}`, file])]);
const types = { '.html':'text/html; charset=utf-8', '.css':'text/css', '.mjs':'text/javascript', '.svg':'image/svg+xml', '.png':'image/png' };
const server = createServer(async (request, response) => {
  const file = assets.get(new URL(request.url, 'http://localhost').pathname);
  if (!file) { response.writeHead(404).end(); return; }
  try { const data = await readFile(path.join(repo, 'apps/site', file)); response.writeHead(200, {'Content-Type':types[path.extname(file)]}).end(data); }
  catch { response.writeHead(500).end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const base = `http://127.0.0.1:${server.address().port}`;
let browser;
const checks = [], errors = [], external = [];
const check = name => checks.push(name);
const qa = process.env.LINTEL_SITE_QA_DIR;
if (qa) await mkdir(qa, {recursive:true});
async function capture(page, name) {
  if (qa) { await page.evaluate(() => scrollTo({top:0,behavior:'instant'})); await page.screenshot({path:path.join(qa, name)}); }
}
async function captureSection(page, selector, name) {
  if (qa) await page.locator(selector).screenshot({path:path.join(qa, name), animations:'disabled'});
}
async function captureFull(page, name) {
  if (qa) { await settleScenery(page); await page.screenshot({path:path.join(qa, name), fullPage:true, animations:'disabled'}); }
}
// Scroll the whole page once so lazy story art decodes and the scenery reveal observer fires,
// then wait for every .story-art image to be complete. Keeps a full-page capture free of the
// transparent-blank state that un-scrolled, loading="lazy", opacity:0 scenery would otherwise show.
async function settleScenery(page) {
  await page.evaluate(async () => {
    const step = Math.max(240, Math.floor(innerHeight * 0.8));
    for (let y = 0; y <= document.documentElement.scrollHeight; y += step) {
      scrollTo({top:y, behavior:'instant'});
      await new Promise(r => requestAnimationFrame(() => setTimeout(r, 60)));
    }
    scrollTo({top:0, behavior:'instant'});
  });
  await page.waitForFunction(() => [...document.querySelectorAll('.story-art')]
    .every(img => img.complete && img.naturalWidth > 0), null, {timeout:10000});
}
// Assert a specific landscape <img> has actually decoded after it scrolled into view.
async function assertImageLoaded(page, selector) {
  const img = page.locator(selector);
  assert.equal(await img.count(), 1, `${selector} present`);
  await img.first().scrollIntoViewIfNeeded();
  await page.waitForFunction(sel => { const node = document.querySelector(sel); return node && node.complete && node.naturalWidth > 0; }, selector, {timeout:10000});
  const ok = await img.first().evaluate(node => node.complete && node.naturalWidth > 0);
  assert.equal(ok, true, `${selector} decodes after scrolling into view`);
}
try {
  // --- Without JavaScript every product section, reading example and doc link stays visible. ---
  browser = await chromium.launch({headless:true});
  {
    const noJs = await browser.newContext({javaScriptEnabled:false, viewport:{width:1280,height:900}});
    const page = await noJs.newPage();
    await page.goto(base, {waitUntil:'domcontentloaded'});
    for (const id of ['#top','#journey','#keep','#review','#continue','#sample-pages','#try']) {
      assert.equal(await page.locator(id).count() > 0, true, `no-JS: ${id} present`);
    }
    assert.equal(await page.locator('.brand-wordmark-svg').count(), 3, 'no-JS: three native wordmarks');
    const text = await page.locator('#main').innerText();
    assert.match(text, /Claude Code|Claude/, 'no-JS: product description visible');
    // Reading-example snippets live inside a collapsed <details>; assert on textContent so the
    // closed (but server-rendered) content is still required to be present without JavaScript.
    const sampleSource = await page.locator('#sample-pages').evaluate(node => node.textContent);
    assert.match(sampleSource, /CLAUDE\.md/); assert.match(sampleSource, /MEMORY\.md/); assert.match(sampleSource, /session\.jsonl/);
    assert.equal(await page.locator('#main a[href*="quickstart.md"]').count() > 0, true);
    assert.equal(await page.locator('#main a[href*="quickstart.en.md"]').count() > 0, true);
    assert.equal(await page.locator('#main a[href*="current-state.md"]').count() > 0, true);
    assert.equal(await page.locator('#main a[href*="architecture.md"]').count() > 0, true);
    // The reading example is itself the native details control (id="sample-pages"), present and
    // toggleable without JavaScript.
    assert.equal(await page.locator('details#sample-pages').count(), 1, 'no-JS: native details#sample-pages present');
    assert.equal(await page.locator('details#sample-pages > summary').count(), 1, 'no-JS: details has a summary');
    await page.locator('details#sample-pages > summary').click();
    assert.equal(await page.locator('details#sample-pages').evaluate(node => node.open), true, 'native reading opens without JavaScript');
    assert.equal(await page.locator('#sample-pages pre').first().isVisible(), true, 'source text is actually visible without JavaScript');
    check('no-JS: product story and docs visible; native reading opens without JavaScript');
    await noJs.close();
  }

  const context = await browser.newContext({viewport:{width:1440,height:900},colorScheme:'light'});
  const page = await context.newPage();
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => { if (!request.url().startsWith(base)) external.push(request.url()); });
  page.on('response', response => { if (response.status() >= 400) errors.push(`${response.status()} ${response.url()}`); });
  await page.goto(base, {waitUntil:'networkidle'});
  assert.match(await page.title(), /Lintel/);
  assert.equal(await page.locator('h1').count(), 1, 'a single page headline');

  // --- Hero: native identity, tail geometry, raster plate actually loads. ---
  assert.equal(await page.locator('.brand-wordmark-svg').count(), 3, 'header, hero and footer wordmarks');
  for (const logo of await page.locator('.brand-wordmark-svg').all()) {
    const relation = await logo.evaluate(svg => {
      const b = svg.querySelector('path').getBBox(), c = svg.querySelector('rect').getBBox();
      const glyph = svg.querySelector('path');
      const foot = glyph.getPointAtLength(glyph.getTotalLength());
      return {overlap:b.x+b.width-c.x, extension:c.x+c.width-b.x-b.width, height:b.height, y:c.y-b.y, bottom:c.y+c.height-b.y, baselineGap:Math.abs(c.y+c.height-foot.y)};
    });
    assert.ok(relation.overlap > 0, 'orange tail overlaps the final l');
    assert.ok(relation.extension > 0 && relation.extension < relation.height * .25, 'only a short orange extension past the foot');
    assert.ok(relation.y > relation.height * .8 && relation.bottom <= relation.height, 'tail sits at the glyph foot');
    assert.ok(relation.baselineGap < .02, 'orange tail and final l share the same bottom edge');
  }
  assert.match(await page.locator('.hero-art').evaluate(node=>getComputedStyle(node).backgroundImage), /lintel-landscape\.png/);
  const heroLoaded = await page.evaluate(async () => {
    const urls = [...getComputedStyle(document.querySelector('.hero-art')).backgroundImage.matchAll(/url\(["']?(.*?)["']?\)/g)].map(m => m[1]);
    const results = await Promise.all(urls.map(url => new Promise(resolve => { const img = new Image(); img.onload = () => resolve(img.naturalWidth > 0); img.onerror = () => resolve(false); img.src = url; })));
    return results.length > 0 && results.every(Boolean);
  });
  assert.equal(heroLoaded, true, 'hero raster plate decodes');
  // Superseded composition is gone.
  assert.equal(await page.locator('.hero-identity, .hero-figure, .lintel-drawing').count(), 0, 'superseded hero composition retired');
  // No legacy six-chapter fake-window / checkbox / draft demo paths survive anywhere.
  for (const legacy of ['.demo-shell','[data-select-list]','[data-selection-summary]','[data-selection-frozen]','[data-package-files]','[data-draft]','#demo-panel','.feature-index']) {
    assert.equal(await page.locator(legacy).count(), 0, `legacy demo path removed: ${legacy}`);
  }
  check('hero identity tail geometry, finite raster plate loads, no legacy demo composition');

  // --- The two body landscape plates decode after their section scrolls into view (not just the
  //     hero background). They are loading="lazy", so assert after scrolling, not on first paint. ---
  await assertImageLoaded(page, '#keep img.story-art');
  await assertImageLoaded(page, '#review img.story-art');
  assert.match(await page.locator('#keep img.story-art').getAttribute('src'), /lintel-keep\.png/);
  assert.match(await page.locator('#review img.story-art').getAttribute('src'), /lintel-crossing\.png/);
  check('both story landscape images decode (lintel-keep in #keep, lintel-crossing in #review)');

  // --- #journey starts the body copy; three Chinese sections follow without motion gating. ---
  assert.equal(await page.locator('#journey').count(), 1);
  for (const id of ['keep','review','continue','try']) {
    assert.equal(await page.locator(`#${id}`).count(), 1, `#${id} present`);
  }
  // #journey is the first product narrative section inside main, before keep/review/continue/try.
  const journeyFirst = await page.evaluate(() => {
    const journey = document.getElementById('journey');
    if (!journey) return false;
    return ['keep','review','continue','try'].every(id => {
      const node = document.getElementById(id);
      return node && (journey.compareDocumentPosition(node) & Node.DOCUMENT_POSITION_FOLLOWING);
    });
  });
  assert.equal(journeyFirst, true, '#journey precedes the story sections');
  const keepText = await page.locator('#keep').innerText();
  assert.match(keepText, /[\u4e00-\u9fff]/, '#keep carries Chinese product copy');
  check('#journey begins the body; #keep/#review/#continue/#try sections exist with Chinese copy');

  // --- #sample-pages is itself a native details reading example: mouse + keyboard toggle. ---
  const sampleDetails = page.locator('details#sample-pages');
  assert.equal(await sampleDetails.count(), 1, 'native details#sample-pages reading example');
  const first = sampleDetails;
  const summary = first.locator('summary');
  assert.equal(await summary.count(), 1, 'summary present for toggling');
  assert.equal(await first.evaluate(node => node.open), false, 'collapsed by default');
  await summary.click();
  assert.equal(await first.evaluate(node => node.open), true, 'mouse opens the example');
  await summary.click();
  assert.equal(await first.evaluate(node => node.open), false, 'mouse closes the example');
  await summary.focus();
  await page.keyboard.press('Enter');
  assert.equal(await first.evaluate(node => node.open), true, 'keyboard Enter toggles the example');
  await page.keyboard.press('Space');
  assert.equal(await first.evaluate(node => node.open), false, 'keyboard Space toggles the example');
  const sampleText = await page.locator('#sample-pages').evaluate(node => node.textContent);
  for (const name of ['CLAUDE.md','MEMORY.md','session.jsonl']) assert.match(sampleText, new RegExp(name.replace('.','\\.')), `reading example shows ${name}`);
  // The example is narrative only: no reader editor, clipboard, crypto or package generation.
  assert.equal(await page.locator('#sample-pages [contenteditable], #sample-pages textarea, #sample-pages button').count(), 0, 'reading example has no editable/generate controls');
  await page.getByRole('link', {name:'翻开一页工作包'}).click();
  assert.equal(await first.evaluate(node => node.open), true, 'the story link opens its reading example');
  await summary.click();
  assert.equal(await first.evaluate(node => node.open), false);
  check('native reading: mouse, keyboard and story link open the explicit synthetic example');

  // --- #review and #continue state preview approval, accepted≠completed and query-only original IDs. ---
  const narrative = `${await page.locator('#review').innerText()}\n${await page.locator('#continue').innerText()}`;
  assert.match(narrative, /accepted/i, 'accepted state named');
  assert.match(narrative, /completed/i, 'completed state named');
  assert.match(narrative, /批准|预览|preview/i, 'preview approval is stated');
  assert.match(narrative, /只核对|不重放|查询/, 'original-ID lookup is query-only, never replayed');
  check('#review/#continue state preview approval, accepted≠completed and query-only original IDs');

  // --- #try carries scope/limits, the unverified-download boundary and real CN/EN + six docs links. ---
  const tryText = await page.locator('#try').innerText();
  assert.match(tryText, /未|没有|尚无|未验收|未验证|no .*download|not .*verified/i, '#try states unverified/absent delivery boundaries');
  for (const href of ['quickstart.md','quickstart.en.md','agents.md','operator-guide.md','current-state.md','architecture.md']) {
    assert.equal(await page.locator(`#try a[href*="${href}"]`).count() > 0, true, `#try links ${href}`);
  }
  check('#try exposes scope/limits, download boundary and CN/EN quickstart + six docs links');

  // --- Theme preference survives reload and swaps the hero night plate. ---
  await page.getByRole('button',{name:'切换到夜色'}).click();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
  await page.reload();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
  assert.match(await page.locator('.hero-art').evaluate(node=>getComputedStyle(node).backgroundImage), /lintel-landscape-night\.png/);
  const nightLoaded = await page.evaluate(async () => {
    const urls = [...getComputedStyle(document.querySelector('.hero-art')).backgroundImage.matchAll(/url\(["']?(.*?)["']?\)/g)].map(m => m[1]);
    const results = await Promise.all(urls.map(url => new Promise(resolve => { const img = new Image(); img.onload = () => resolve(img.naturalWidth > 0); img.onerror = () => resolve(false); img.src = url; })));
    return results.length > 0 && results.every(Boolean);
  });
  assert.equal(nightLoaded, true, 'night raster plate decodes');
  check('night theme, night plate and local preference survive reload');

  // --- Retained shared game: opt-in, pause on focus exit, resume. ---
  assert.equal(await page.locator('#clawd-game.clawd-game').count(), 1, 'single shared runner mounted at #clawd-game');
  const board = page.getByRole('group',{name:'Clawd 跳跃跑道'});
  assert.equal(await page.locator('.clawd-game').getAttribute('data-state'), 'ready', 'game rests until started');
  await board.scrollIntoViewIfNeeded(); await board.focus(); await page.keyboard.press('Space');
  assert.equal(await page.locator('.clawd-game').getAttribute('data-state'), 'running');
  await page.waitForFunction(()=>Number(document.querySelector('.cg-score').textContent)>0);
  await page.keyboard.press('p');
  assert.equal(await page.locator('.clawd-game').getAttribute('data-state'), 'paused');
  await page.getByRole('button',{name:'继续走',exact:true}).click();
  assert.equal(await page.locator('.clawd-game').getAttribute('data-state'), 'running');
  await page.getByRole('link',{name:'回到上面 ↑'}).focus();
  assert.equal(await page.locator('.clawd-game').getAttribute('data-state'), 'paused', 'focus exit pauses');
  check('opt-in shared game: rest, start, score, pause, resume and pause-on-focus-exit');

  // --- Desktop captures: names must match the actual theme, and the full-page plate is captured
  //     only after every story art has scrolled in and decoded. ---
  if (await page.locator('html').getAttribute('data-theme') !== 'dark') await page.getByRole('button',{name:'切换到夜色'}).click();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark', 'desktop-night capture is night');
  await capture(page, 'desktop-night.png');
  await page.getByRole('button',{name:'切换到日光'}).click();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'light');
  await capture(page, 'desktop-day.png');
  await captureFull(page, 'desktop-full.png');
  if (qa) for (const id of ['keep','review','continue']) await captureSection(page, `#${id}`, `section-${id}-1440.png`);

  // --- Reduced motion: reveals complete, decorative cursor static. ---
  await page.emulateMedia({reducedMotion:'reduce'});
  assert.equal(await page.locator('.cursor').first().evaluate(node=>getComputedStyle(node).animationName), 'none');
  assert.equal(await page.locator('#try').evaluate(node=>getComputedStyle(node).opacity), '1');
  for (const art of await page.locator('.story-art').all()) {
    assert.equal(await art.evaluate(node=>getComputedStyle(node).transform), 'none');
    assert.ok(await art.evaluate(node=>Number(getComputedStyle(node).opacity)) > 0, 'reduced motion shows every scene immediately');
  }
  check('reduced motion completes reveals and stills the decorative wordmark cursor');

  // --- Mobile 390 and 320: no horizontal overflow, no clipped paths. Names match the theme:
  //     switch to night and assert before mobile-night, switch back to day before mobile-day. ---
  await page.setViewportSize({width:390,height:844});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth > innerWidth), false, 'no 390px overflow');
  if (await page.locator('html').getAttribute('data-theme') !== 'dark') await page.getByRole('button',{name:'切换到夜色'}).click();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark', 'mobile-night capture is night');
  await capture(page, 'mobile-night.png');
  await captureFull(page, 'mobile-full.png');
  if (qa) for (const id of ['keep','review','continue']) await captureSection(page, `#${id}`, `section-${id}-390.png`);
  await page.locator('.cg-jump').click();
  assert.equal(await page.locator('.clawd-game').getAttribute('data-state'), 'running');
  await page.locator('.cg-pause').click();
  assert.equal(await page.locator('.clawd-game').getAttribute('data-state'), 'paused');
  if (await page.locator('html').getAttribute('data-theme') !== 'light') await page.getByRole('button',{name:'切换到日光'}).click();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'light', 'mobile-day capture is day');
  await capture(page, 'mobile-day.png');
  await page.setViewportSize({width:320,height:844});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth > innerWidth), false, 'no 320px overflow');
  check('390px and 320px mobile layout, no overflow and direct jump/pause controls');

  await context.close();

  // --- Blocked local storage: theme/game stay usable, scores temporary. ---
  const blocked = await browser.newContext({viewport:{width:390,height:844},hasTouch:true});
  await blocked.addInitScript(()=>{Object.defineProperty(window,'localStorage',{get(){throw new DOMException('Storage denied','SecurityError');}});});
  const blockedPage = await blocked.newPage();
  blockedPage.on('pageerror',error=>errors.push(error.message));
  await blockedPage.goto(base);
  assert.match(await blockedPage.locator('.cg-storage').innerText(), /存储不可用/);
  await blockedPage.locator('.cg-jump').tap();
  assert.equal(await blockedPage.locator('.clawd-game').getAttribute('data-state'), 'running');
  await blockedPage.locator('.cg-pause').tap();
  assert.equal(await blockedPage.locator('.clawd-game').getAttribute('data-state'), 'paused');
  check('touch jump/pause and blocked local storage with temporary scores');
  await blocked.close();

  assert.deepEqual(errors, []); assert.deepEqual(external, []);
  check('no runtime/asset errors or external requests');
  console.log(JSON.stringify({status:'passed',checks},null,2));
} finally {
  await browser?.close(); await new Promise(resolve=>server.close(resolve));
}

// Disposable browser context; website only. No Claude, native bridge or personal profile.
//
// New stable page contract (coordinator-owned markup):
//   header nav   #capabilities / #how-it-works / #try / the GitHub link.
//   #top       hero: native .brand-wordmark-svg lockup, the terracotta tail overlapping the
//              final l; day/night raster plates assets/lintel-landscape{,-night}.png; the
//              decorative .cursor blinks slowly in the header/footer and stays still under
//              prefers-reduced-motion.
//   #capabilities   six <article data-capability> entries whose ids must equal the six stable
//                   task ids in contracts/task-catalog.json; each entry title links to a real
//                   doc/anchor and each entry carries a concrete result plus a near-boundary.
//   #journey   start of the illustrated work journey.
//   #keep #review #continue   Chinese product-narrative sections (no motion gating).
//   #sample-pages   native <details>/<summary> reading example with explicit synthetic
//                   CLAUDE.md / MEMORY.md / session.jsonl snippets. No reader editing/writing,
//                   clipboard, crypto or package-generation control.
//   #how-it-works   four-step "preserve three synthetic originals" example with
//                   .execution-entries (App / CLI / SSH) and a site/browser boundary.
//   #try       scope/limits, the unverified-download boundary, CN/EN quickstart and docs links.
//   #clawd-game   the single shared clawd-game.mjs/.css runner, opt-in.
// The superseded six-chapter fake-window / checkbox / draft demo is gone and must not return.
// The retired .capability-line teaser must not return either.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const { chromium } = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE || path.join(repo, 'extensions/browser/node_modules/playwright/index.mjs')).href);
// Canonical task map + finite named doc link allowlist. The page is checked against these
// authoritative sources instead of embedding a copy of the copy.
const catalog = JSON.parse(await readFile(path.join(repo, 'contracts/task-catalog.json'), 'utf8'));
const resources = JSON.parse(await readFile(path.join(repo, 'contracts/documentation-resources.json'), 'utf8'));
const catalogIds = catalog.tasks.map(task => task.id);
assert.equal(catalogIds.length, 6, 'task catalog holds six stable task ids');
// Map each catalog task id to the doc link its site card may use, then build the finite set of
// real hrefs the page is allowed to link (docs/*.md plus the operator-guide anchors).
const cardLinks = {
  reduce_egress: ['operator-guide.md#protect'],
  preserve_work: ['operator-guide.md#work'],
  repair_cleanup_retire: ['operator-guide.md#cleanup'],
  browser_profile: ['browser.md'],
  ssh_remote: ['remote.md'],
  recover_results: ['operator-guide.md#recovery'],
};
const docBase = 'https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/';
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
  if (!qa) return;
  await page.locator(selector).evaluate(node => node.scrollIntoView({block:'start', behavior:'instant'}));
  const clip = await page.locator(selector).evaluate(node => {
    const rect = node.getBoundingClientRect();
    return {x:rect.x + scrollX, y:rect.y + scrollY, width:rect.width, height:rect.height};
  });
  // Full-page clipping preserves document coordinates for sections taller than the viewport;
  // centering a long element screenshot can otherwise include offscreen fixed controls.
  await page.screenshot({path:path.join(qa, name), clip, fullPage:true, animations:'disabled'});
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
// The new product explanation must retain visible titles and body copy at every checked width.
// Inspect both layout and ancestor styles; a non-zero box alone misses visibility/opacity hiding.
async function assertProductCopyVisible(page, label) {
  const copies = await page.locator('#capabilities article h3, #capabilities article h3 + p, #how-it-works .workflow h3, #how-it-works .workflow p').evaluateAll(nodes => nodes.map(node => {
    const rect = node.getBoundingClientRect();
    let visible = rect.width > 0 && rect.height > 0;
    for (let ancestor = node; ancestor && visible; ancestor = ancestor.parentElement) {
      const style = getComputedStyle(ancestor);
      visible = style.display !== 'none' && style.visibility === 'visible' && Number(style.opacity) > 0;
    }
    return {visible, text:node.textContent.trim()};
  }));
  for (const copy of copies) assert.ok(copy.visible && copy.text.length > 0, `${label}: product copy stays visible (${copy.text.slice(0, 24)})`);
}
try {
  // --- Without JavaScript every product section, reading example and doc link stays visible. ---
  browser = await chromium.launch({headless:true});
  {
    const noJs = await browser.newContext({javaScriptEnabled:false, viewport:{width:1280,height:900}});
    const page = await noJs.newPage();
    await page.goto(base, {waitUntil:'domcontentloaded'});
    for (const id of ['#top','#capabilities','#journey','#keep','#review','#how-it-works','#continue','#try']) {
      assert.equal(await page.locator(id).count() > 0, true, `no-JS: ${id} present`);
    }
    assert.equal(await page.locator('.brand-wordmark-svg').count(), 3, 'no-JS: three native wordmarks');
    const text = await page.locator('#main').innerText();
    assert.match(text, /Claude Code|Claude/, 'no-JS: product description visible');
    // The six capabilities are server-rendered: ids match the catalog and every card is readable.
    const capIds = await page.locator('#capabilities article[data-capability]').evaluateAll(nodes => nodes.map(node => node.dataset.capability));
    assert.deepEqual(capIds.slice().sort(), catalogIds.slice().sort(), 'no-JS: six capability cards match the catalog ids');
    assert.match(await page.locator('#capabilities').innerText(), /[\u4e00-\u9fff]/, 'no-JS: capability section carries Chinese copy');
    assert.equal(await page.locator('#how-it-works .workflow > li').count(), 4, 'no-JS: four workflow steps present');
    await assertProductCopyVisible(page, 'no-JS');
    assert.equal(await page.locator('#how-it-works .execution-entries > div').count(), 3, 'no-JS: three execution entries present');
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
    check('no-JS: capabilities, four steps, reading example and docs all visible; native reading opens without JavaScript');
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

  // --- Header navigation targets the real sections and every in-page anchor resolves. ---
  for (const target of ['#capabilities','#how-it-works','#try']) {
    assert.equal(await page.locator(`header nav a[href="${target}"]`).count() > 0, true, `nav links ${target}`);
  }
  const github = page.locator('header nav a[href*="github.com/IndelibleVivi/lintel-cc"]');
  assert.equal(await github.count() > 0, true, 'nav links the repo');
  assert.equal(await github.first().getAttribute('href'), resources.source, 'repo link matches the resource allowlist');
  // Every same-page href="#..." must point at an existing element id (no dead anchors).
  const deadAnchors = await page.evaluate(() => [...document.querySelectorAll('a[href^="#"]')]
    .map(a => a.getAttribute('href').slice(1)).filter(id => id && !document.getElementById(id)));
  assert.deepEqual(deadAnchors, [], 'no dead in-page anchors');
  check('header nav resolves to #capabilities/#how-it-works/#try and the repo; no dead in-page anchors');

  // --- Real header navigation: an actual click and a real keyboard Enter take the browser to the
  //     section (hash changes, target scrolls to its normal position). No evaluate/mock clicking. ---
  // scroll-padding-top is 28px at desktop width; the section top should settle just below the
  // viewport top once smooth scrolling has finished. Wait for an observable settled position
  // rather than a fixed sleep.
  const scrollPaddingTop = await page.evaluate(() => parseFloat(getComputedStyle(document.documentElement).scrollPaddingTop) || 0);
  const assertNavigated = async (hash) => {
    const id = hash.slice(1);
    assert.equal(new URL(page.url()).hash, hash, `click navigates to ${hash}`);
    // Bounded wait: the section reaches its normal scroll position (top near scroll-padding-top).
    await page.waitForFunction(([sel, pad]) => {
      const node = document.getElementById(sel);
      if (!node) return false;
      const top = node.getBoundingClientRect().top;
      return top >= -1 && top <= pad + 4;
    }, [id, scrollPaddingTop], {timeout:4000});
    // Confirm it really is the targeted section at a normal resting position (not mid-animation
    // far past it), and that it is actually visible in the viewport.
    const settled = await page.evaluate(([sel, pad]) => {
      const node = document.getElementById(sel);
      const rect = node.getBoundingClientRect();
      return {top: rect.top, visible: rect.bottom > 0 && rect.top < innerHeight, pad};
    }, [id, scrollPaddingTop]);
    assert.ok(settled.top >= -1 && settled.top <= scrollPaddingTop + 4, `${hash} rests at its normal scroll position (top=${settled.top.toFixed(1)})`);
    assert.equal(settled.visible, true, `${hash} section is in the viewport after navigation`);
  };
  // Click the #capabilities nav link.
  await page.evaluate(() => scrollTo({top:0, behavior:'instant'}));
  await page.locator('header nav a[href="#capabilities"]').click();
  await assertNavigated('#capabilities');
  // Keyboard: focus the #how-it-works nav link and press Enter.
  await page.evaluate(() => scrollTo({top:0, behavior:'instant'}));
  const hiwNav = page.locator('header nav a[href="#how-it-works"]');
  await hiwNav.focus();
  await page.keyboard.press('Enter');
  await assertNavigated('#how-it-works');
  check('real header nav click and keyboard Enter navigate to #capabilities/#how-it-works at their normal scroll position');

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
  // No legacy six-chapter fake-window / checkbox / draft demo, nor the retired capability teaser.
  for (const legacy of ['.demo-shell','[data-select-list]','[data-selection-summary]','[data-selection-frozen]','[data-package-files]','[data-draft]','#demo-panel','.feature-index','.capability-line']) {
    assert.equal(await page.locator(legacy).count(), 0, `legacy composition removed: ${legacy}`);
  }
  check('hero identity tail geometry, finite raster plate loads, no legacy demo or capability-line composition');

  // --- #capabilities: six cards, catalog-aligned ids, real doc links, result + near-boundary. ---
  const cards = page.locator('#capabilities article[data-capability]');
  assert.equal(await cards.count(), 6, 'six capability cards');
  const pageIds = await cards.evaluateAll(nodes => nodes.map(node => node.dataset.capability));
  assert.deepEqual(pageIds.slice().sort(), catalogIds.slice().sort(), 'capability ids equal the six stable catalog task ids');
  // Each card title link points at a real doc/anchor allowed for that exact task id.
  for (const id of catalogIds) {
    const card = page.locator(`#capabilities article[data-capability="${id}"]`);
    assert.equal(await card.count(), 1, `card ${id} present`);
    const link = card.locator('h3 a');
    assert.equal(await link.count(), 1, `card ${id} has one title link`);
    const href = await link.getAttribute('href');
    const allowed = cardLinks[id].map(suffix => `${docBase}${suffix}`);
    assert.ok(allowed.includes(href), `card ${id} links a real doc/anchor (${href})`);
    // The linked doc file exists in the working tree (no dangling guide); fragment stripped.
    const [filename, anchor] = href.slice(docBase.length).split('#');
    const doc = await readFile(path.join(repo, 'docs', filename), 'utf8');
    if (anchor) assert.ok(doc.includes(`<a id="${anchor}">`), `capability ${id} links an existing named anchor`);
    // Chinese copy plus a concrete result; the near-boundary paragraph is a specific limitation.
    assert.match(await card.innerText(), /[\u4e00-\u9fff]/, `card ${id} carries Chinese copy`);
    assert.equal(await card.locator('p.capability-boundary').count(), 1, `card ${id} carries a near-boundary note`);
    assert.ok((await card.locator('p.capability-boundary').innerText()).trim().length > 8, `card ${id} boundary names a concrete limit, not a slogan`);
  }
  check('#capabilities: six catalog-aligned cards, real doc links that resolve and per-card boundary notes');

  // --- Boundary keywords live near the relevant capability (presence, not verbatim copy). ---
  const boundaryOf = id => page.locator(`#capabilities article[data-capability="${id}"] .capability-boundary`).innerText();
  assert.match(await boundaryOf('preserve_work'), /续聊/, 'preserve card distinguishes package reading from real resume');
  assert.match(await boundaryOf('browser_profile'), /未验收|尚未|开发加载/, 'browser card states it is not yet accepted');
  assert.match(await boundaryOf('ssh_remote'), /重启|生产|独立验收/, 'ssh card does not promise restart survival');
  assert.match(await boundaryOf('reduce_egress'), /代理|运行效果|分别|不等于/, 'config card separates read-back from strong network enforcement');
  assert.match(await boundaryOf('repair_cleanup_retire'), /服务端|账号|撤销/, 'cleanup card separates local files from server state');
  assert.match(await boundaryOf('recover_results'), /恢复|不能/, 'recovery card states there is no universal restore');
  // The site-wide Preview boundary stays present near the top of the capabilities section.
  assert.match(await page.locator('#capabilities .section-status').innerText(), /预览|Preview|未/, 'capabilities section flags the source Preview stage');
  check('boundaries near each capability: package-read ≠ resume, browser unverified, ssh no restart promise, config ≠ network enforcement');

  // --- The two body landscape plates decode after their section scrolls into view (not just the
  //     hero background). They are loading="lazy", so assert after scrolling, not on first paint. ---
  await assertImageLoaded(page, '#keep img.story-art');
  await assertImageLoaded(page, '#review img.story-art');
  assert.match(await page.locator('#keep img.story-art').getAttribute('src'), /lintel-keep\.png/);
  assert.match(await page.locator('#review img.story-art').getAttribute('src'), /lintel-crossing\.png/);
  check('both story landscape images decode (lintel-keep in #keep, lintel-crossing in #review)');

  // --- #journey starts the illustrated journey; the narrative sections follow without motion gating. ---
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
  assert.match(await page.locator('#keep').innerText(), /[\u4e00-\u9fff]/, '#keep carries Chinese product copy');
  check('#journey begins the illustrated journey; #keep/#review/#continue/#try sections exist with Chinese copy');

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

  // --- #review states the preview→approval→original-task sequence; #continue keeps the closing note. ---
  const reviewText = await page.locator('#review').innerText();
  assert.match(reviewText, /预览/, '#review names the preview step');
  assert.match(reviewText, /批准|执行/, '#review names approval/execution of the same plan');
  assert.match(reviewText, /原任务|回执/, '#review points at the original-task receipt');
  assert.match(await page.locator('#continue').innerText(), /[\u4e00-\u9fff]/, '#continue keeps the Chinese closing note');
  check('#review states preview → approval → original-task result; #continue keeps the closing note');

  // --- #how-it-works: four-step preserve example, execution entries and site boundary. ---
  assert.equal(await page.locator('#how-it-works .workflow > li').count(), 4, 'four workflow steps');
  const stepText = (await page.locator('#how-it-works .workflow').innerText()).toLowerCase();
  for (const [token, label] of [[/对象|原件/, 'explicit objects'], [/预览|审阅/, 'preview'], [/批准/, 'approval'], [/原任务|回执|核对/, 'original-task receipt']]) {
    assert.match(stepText, token, `workflow states ${label}`);
  }
  const scopeText = await page.locator('#how-it-works .example-scope').innerText();
  for (const name of ['CLAUDE.md','MEMORY.md','session.jsonl']) assert.match(scopeText, new RegExp(name.replace('.','\\.')), `example scope names ${name}`);
  // Execution entries: App and CLI share the core; SSH executes on the target host.
  const entries = await page.locator('#how-it-works .execution-entries > div').evaluateAll(nodes => nodes.map(n => n.innerText));
  assert.equal(entries.length, 3, 'three execution entries (App / CLI / SSH)');
  assert.ok(entries.some(t => /共用|同一.*核心|shared/.test(t)), 'CLI entry states the shared core');
  assert.ok(entries.some(t => /目标主机|远端|runner/.test(t)), 'SSH entry states target-host execution');
  assert.match(await page.locator('#how-it-works .execution-boundary').innerText(), /独立|有限|不.*执行|不读取/, 'site/browser entry is bounded');
  // The accepted≠completed / query-only thread belongs to the execution model here.
  const thread = await page.locator('#how-it-works .thread-line').innerText();
  assert.match(thread, /accepted[\s\S]*completed/i, 'accepted ≠ completed is stated');
  assert.match(thread, /只核对|不重放/, 'original-ID lookup is query-only, never replayed');
  // Focus reachability: header nav and each capability link take keyboard focus.
  for (const sel of ['header nav a[href="#capabilities"]','header nav a[href="#how-it-works"]','#capabilities article[data-capability] h3 a']) {
    const el = page.locator(sel).first();
    await el.scrollIntoViewIfNeeded();
    await el.focus();
    assert.equal(await el.evaluate(node => node === document.activeElement), true, `focusable: ${sel}`);
  }
  check('#how-it-works: four explicit steps, shared-core execution entries and a bounded site/browser entry');

  // --- #try carries scope/limits, the unverified-download boundary and real CN/EN + docs links. ---
  const tryText = await page.locator('#try').innerText();
  assert.match(tryText, /未|没有|尚无|未验收|未验证|no .*download|not .*verified/i, '#try states unverified/absent delivery boundaries');
  assert.match(tryText, /Preview|预览/i, '#try keeps the Preview stage label');
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
  // Cover the new contract sections as well as the retained story sections.
  if (qa) for (const id of ['capabilities','how-it-works','keep','review','continue']) await captureSection(page, `#${id}`, `section-${id}-1440.png`);

  // --- Reduced motion: reveals complete, decorative cursor static. ---
  await page.emulateMedia({reducedMotion:'reduce'});
  assert.equal(await page.locator('.cursor').first().evaluate(node=>getComputedStyle(node).animationName), 'none');
  assert.equal(await page.locator('#try').evaluate(node=>getComputedStyle(node).opacity), '1');
  for (const art of await page.locator('.story-art').all()) {
    assert.equal(await art.evaluate(node=>getComputedStyle(node).transform), 'none');
    assert.ok(await art.evaluate(node=>Number(getComputedStyle(node).opacity)) > 0, 'reduced motion shows every scene immediately');
  }
  check('reduced motion completes reveals and stills the decorative wordmark cursor');

  // --- Mobile 390 and 320: no horizontal overflow, all six capabilities and four steps keep
  //     their content, no clipped paths. Names match the theme: switch to night and assert before
  //     mobile-night, switch back to day before mobile-day. ---
  await page.setViewportSize({width:390,height:844});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth > innerWidth), false, 'no 390px overflow');
  assert.equal(await page.locator('#capabilities article[data-capability]').count(), 6, '390px keeps all six capability cards');
  await assertProductCopyVisible(page, '390px');
  assert.equal(await page.locator('#how-it-works .workflow > li').count(), 4, '390px keeps all four workflow steps');
  assert.equal(await page.locator('#how-it-works .execution-entries > div').count(), 3, '390px keeps all three execution entries');
  if (await page.locator('html').getAttribute('data-theme') !== 'dark') await page.getByRole('button',{name:'切换到夜色'}).click();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark', 'mobile-night capture is night');
  await capture(page, 'mobile-night.png');
  await captureFull(page, 'mobile-full.png');
  if (qa) for (const id of ['capabilities','how-it-works','keep','review','continue']) await captureSection(page, `#${id}`, `section-${id}-390.png`);
  await page.locator('.cg-jump').click();
  assert.equal(await page.locator('.clawd-game').getAttribute('data-state'), 'running');
  await page.locator('.cg-pause').click();
  assert.equal(await page.locator('.clawd-game').getAttribute('data-state'), 'paused');
  if (await page.locator('html').getAttribute('data-theme') !== 'light') await page.getByRole('button',{name:'切换到日光'}).click();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'light', 'mobile-day capture is day');
  await capture(page, 'mobile-day.png');
  await page.setViewportSize({width:320,height:844});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth > innerWidth), false, 'no 320px overflow');
  assert.equal(await page.locator('#capabilities article[data-capability]').count(), 6, '320px keeps all six capability cards');
  await assertProductCopyVisible(page, '320px');
  assert.equal(await page.locator('#how-it-works .workflow > li').count(), 4, '320px keeps all four workflow steps');
  check('390px and 320px mobile reflow, six capabilities + four steps retained, no overflow and direct jump/pause controls');

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

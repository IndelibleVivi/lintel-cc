// Disposable browser context; website only. No Claude, native bridge or personal profile.
//
// New stable page contract (coordinator-owned markup):
//   header nav   #capabilities / #how-it-works / #try / the GitHub link.
//   #top       hero: native .brand-wordmark-svg lockup, the whole orange final-l foot and detached slow-blinking header cursor; day/night raster plates assets/lintel-landscape{,-night}.png; the
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
import { spawnSync } from 'node:child_process';
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const { checkSiteLanguages } = await import('../scripts/prepare-site-languages.mjs');
await checkSiteLanguages();
const { GUIDE_ARTICLES, SITE_ROUTES, localePath } = await import('../apps/site/site-routes.mjs');
const languageChecks = spawnSync(process.execPath, ['--test','tests/site_languages.test.mjs'], {cwd:repo,encoding:'utf8'});
assert.equal(languageChecks.status,0,languageChecks.stdout + languageChecks.stderr);
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
const localTaskGuide = (suffix, language='zh') => `${language==='en'?'/en':''}/guide/${suffix.replace('.md','/')}`;
// Finite named asset map only: the server never serves arbitrary files.
const assets = new Map([['/', 'index.html'], ['/en/', 'en/index.html'], ...[
  'index.html','en/index.html','styles.css','site.mjs','site-copy.mjs','site-language.mjs','site-routes.mjs','site-docs.mjs',
  'film.mjs','film.vtt',
  'workflow-trail.mjs','site-world.mjs',
  'clawd-game.mjs','clawd-game.css',
  'assets/favicon.svg','assets/favicon.png',
  'assets/lintel-landscape.png','assets/lintel-landscape-night.png',
  'assets/lintel-keep.png','assets/lintel-crossing.png',
].map(file => [`/${file}`, file])]);
for(const route of SITE_ROUTES.filter(route=>route.path!=='/')){
  const zh=localePath(route.path,'zh');
  assets.set(zh,`${zh.slice(1)}index.html`);
  assets.set(localePath(route.path,'en'),`en/${zh.slice(1)}index.html`);
}
// The short film and its poster never enter Git. main builds them into an ignored media dir; when
// LINTEL_SITE_MEDIA_DIR holds lintel-intro.mp4 + lintel-film-poster.jpg this finite map serves
// exactly those two names so the actual-media playthrough can run. Without them the page-level
// markup checks still pass and real playback stays an independent actual-media verification. The
// synthetic fallback below is explicitly fake bytes; it only lets the request-timing check run and
// is never used when a real media dir is supplied.
const mediaDir = process.env.LINTEL_SITE_MEDIA_DIR;
const mediaAssets = new Map();
const syntheticMedia = new Map();
if (mediaDir) for (const name of ['lintel-intro.mp4', 'lintel-film-poster.jpg']) mediaAssets.set(`/media/${name}`, path.join(mediaDir, name));
else for (const name of ['lintel-intro.mp4', 'lintel-film-poster.jpg']) syntheticMedia.set(`/media/${name}`, name);
const types = { '.html':'text/html; charset=utf-8', '.css':'text/css', '.mjs':'text/javascript', '.svg':'image/svg+xml', '.png':'image/png', '.vtt':'text/vtt' };
const server = createServer(async (request, response) => {
  const pathname = new URL(request.url, 'http://localhost').pathname;
  const file = assets.get(pathname);
  const media = mediaAssets.get(pathname);
  const fake = syntheticMedia.get(pathname);
  if (!file && !media && !fake) { response.writeHead(404).end(); return; }
  try {
    if (media) {
      const data = await readFile(media);
      const type = path.extname(media) === '.mp4' ? 'video/mp4' : 'image/jpeg';
      response.writeHead(200, {'Content-Type':type, 'Content-Length':data.length});
      response.end(request.method === 'HEAD' ? undefined : data);
      return;
    }
    if (fake) { response.writeHead(200, {'Content-Type': path.extname(fake) === '.mp4' ? 'video/mp4' : 'image/jpeg'}).end('synthetic'); return; }
    const data = await readFile(path.join(repo, 'apps/site', file));
    response.writeHead(200, {'Content-Type':types[path.extname(file)]}).end(data);
  } catch { response.writeHead(500).end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const base = `http://127.0.0.1:${server.address().port}`;
let browser;
const checks = [], errors = [], external = [];
const requests = [];
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
// The small title icon and the larger explanatory scene serve different jobs. Check both
// together, including actual layout order and the scene's local SVG label bounds.
async function assertCapabilityComposition(page, label, width) {
  const cards = page.locator('#capabilities article[data-capability]');
  assert.equal(await cards.count(),6,`${label}: six combined capability entries`);
  for (const card of await cards.all()) {
    const icon = card.locator(':scope > .capability-copy > svg.capability-icon');
    const scene = card.locator(':scope > figure.capability-visual > svg[role="img"]');
    assert.equal(await icon.count(),1,`${label}: one small icon inside each copy`);
    assert.equal(await icon.getAttribute('viewBox'),'0 0 32 32');
    assert.equal(await icon.getAttribute('aria-hidden'),'true');
    assert.equal(await icon.getAttribute('focusable'),'false');
    assert.equal(await icon.evaluate(node=>node.tabIndex),-1,'decorative title icons stay outside keyboard tab order');
    assert.equal(await scene.count(),1,`${label}: one large explanatory scene`);
    assert.equal(await card.getByRole('img').count(),1,'only the large scene is an accessible image');
    assert.ok((await scene.getAttribute('aria-label'))?.trim().length>10,'large scenes retain their own localized accessible description');
    assert.ok((await card.locator(':scope > figure.capability-visual > figcaption').innerText()).trim().length>10,'large scenes retain their visible localized caption');
  }
  const entries = await cards.evaluateAll(nodes=>nodes.map(node=>{
    const rect=element=>{const box=element.getBoundingClientRect();return {x:box.x,y:box.y,right:box.right,bottom:box.bottom,width:box.width,height:box.height};};
    return {id:node.dataset.capability,entry:rect(node),copy:rect(node.querySelector(':scope > .capability-copy')),
      icon:rect(node.querySelector(':scope > .capability-copy > svg.capability-icon')),title:rect(node.querySelector('h3')),
      figure:rect(node.querySelector(':scope > figure.capability-visual'))};
  }));
  for (const [index,row] of entries.entries()) {
    assert.equal(row.icon.width,32); assert.equal(row.icon.height,32,'title icons remain small at every viewport');
    assert.ok(row.icon.x>=row.copy.x && row.icon.right<row.title.x && Math.abs(row.icon.y-row.title.y)<=8,`${label} ${row.id}: icon sits beside the title without overlap`);
    assert.equal(row.entry.width,entries[0].entry.width,'capabilities retain six full-width rows');
    if(index)assert.ok(row.entry.y>=entries[index-1].entry.bottom-1,'capability rows never overlap or share a grid row');
    if(width>700){
      assert.ok(index%2 ? row.figure.right<=row.copy.x : row.copy.right<=row.figure.x,`${label} ${row.id}: large scenes alternate around the copy`);
    }else{
      assert.ok(row.figure.y>=row.copy.bottom,`${label} ${row.id}: mobile keeps copy before its large scene`);
      assert.ok(Math.abs(row.copy.x-row.figure.x)<=1,'mobile copy and figure share the same column');
    }
  }
  const labels=await page.locator('#capabilities figure.capability-visual svg text').evaluateAll(nodes=>nodes.map(node=>{
    const box=node.getBBox(),viewport=node.ownerSVGElement.viewBox.baseVal;
    return {text:node.textContent,x:box.x,y:box.y,right:box.x+box.width,bottom:box.y+box.height,width:viewport.width,height:viewport.height};
  }));
  assert.ok(labels.length>=12,'large scene drawings retain their meaningful text labels');
  for(const text of labels)assert.ok(text.x>=0 && text.y>=0 && text.right<=text.width && text.bottom<=text.height,`${label}: large SVG label fits (${text.text})`);
}
// Read the actual browser composite, including SVG clipping, filters and responsive cropping.
// The PNG is a disposable screenshot; it never becomes a product asset or leaves this context.
async function assertNightClawdEyes(page, label) {
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
  for (const [section, eyes, body] of [
    ['keep', [[1474,670],[1493,670]], [[1480,674],[1484,674]]],
    ['review', [[529,507],[550,507]], [[538,515],[542,515]]],
  ]) {
    // The hotspot stays hidden until its lazy raster loads. Scroll the visible plate first;
    // waiting to scroll a hidden hotspot can otherwise prevent that very load after reload.
    await page.locator(`#${section} .story-world`).scrollIntoViewIfNeeded();
    await page.locator(`#${section} .story-art`).evaluate(img => img.decode());
    await page.locator(`#${section} .story-clawd`).waitFor({state:'visible'});
    await page.waitForFunction(id => Number(getComputedStyle(document.querySelector(`#${id} .story-world`)).opacity) === 1, section);
    const png = (await page.screenshot({animations:'disabled'})).toString('base64');
    const colors = await page.evaluate(async ({png, section, eyes, body}) => {
      const image = new Image(); image.src = `data:image/png;base64,${png}`; await image.decode();
      const canvas = document.createElement('canvas'); canvas.width = image.width; canvas.height = image.height;
      const ctx = canvas.getContext('2d'); ctx.drawImage(image, 0, 0);
      const art = document.querySelector(`#${section} .story-art`), box = art.getBoundingClientRect();
      const scale = (getComputedStyle(art).objectFit === 'cover' ? Math.max : Math.min)(box.width / art.naturalWidth, box.height / art.naturalHeight);
      const x = box.x + (box.width - art.naturalWidth * scale) / 2, y = box.y + (box.height - art.naturalHeight * scale) / 2;
      const sample = points => points.map(([sx,sy]) => [...ctx.getImageData(Math.floor(x + sx * scale), Math.floor(y + sy * scale), 1, 1).data].slice(0,3));
      return {eyes:sample(eyes), body:sample(body)};
    }, {png, section, eyes, body});
    const brightness = color => color.reduce((sum, channel) => sum + channel, 0) / 3;
    const face = colors.body.reduce((sum, color) => sum + brightness(color), 0) / colors.body.length;
    for (const eye of colors.eyes) assert.ok(brightness(eye) < face - 20, `${label} ${section}: both original eyes stay dark, not inverted white holes`);
    for (const color of colors.body) assert.ok(color[0] - color[1] > 30 && color[1] - color[2] > 10, `${label} ${section}: the original body stays warm orange`);
  }
}
try {
  // --- Without JavaScript every product section, reading example and doc link stays visible. ---
  browser = await chromium.launch({headless:true});
  {
    const routeContext = await browser.newContext({javaScriptEnabled:false,viewport:{width:1280,height:900}});
    const page = await routeContext.newPage();
    page.on('request',request=>{if(!request.url().startsWith(base))external.push(request.url());});
    page.on('response',response=>{if(response.status()>=400)errors.push(`${response.status()} ${response.url()}`);});
    for(const routePath of ['/download/','/guide/','/inside/','/releases/','/guide/start-here/','/guide/concepts/','/guide/troubleshooting/','/guide/operator-guide/','/guide/browser/','/guide/remote/','/guide/agents/']){
      await page.goto(base+routePath,{waitUntil:'domcontentloaded'});
      assert.ok((await page.locator('#main').innerText()).trim().length>200,`no-JS ${routePath}: substantive body`);
      assert.equal(await page.locator('link[rel="canonical"]').getAttribute('href'),`https://lintel.page${routePath}`);
      assert.equal(await page.locator('html').getAttribute('lang'),'zh-CN');
      assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,`no-JS ${routePath}: no overflow`);
      const dead=await page.evaluate(()=>[...document.querySelectorAll('a[href^="#"]')].map(a=>a.getAttribute('href').slice(1)).filter(id=>id&&!document.getElementById(id)));
      assert.deepEqual(dead,[],`no-JS ${routePath}: same-page anchors resolve`);
      await page.locator('[data-site-route-language="en"]').click();
      assert.equal(new URL(page.url()).pathname,`/en${routePath}`);
      assert.equal(await page.locator('html').getAttribute('lang'),'en');
      assert.ok((await page.locator('#main').innerText()).trim().length>200,`no-JS English mirror ${routePath}: substantive body`);
      await page.locator('[data-site-route-language="zh"]').click();
      assert.equal(new URL(page.url()).pathname,routePath);
    }
    for(const article of GUIDE_ARTICLES){
      await page.goto(base+article.path,{waitUntil:'domcontentloaded'});
      assert.ok(await page.locator('.guide-body h1').count()===1,`${article.key}: source heading rendered`);
      assert.ok((await page.locator('.guide-body p').first().innerText()).length>20,`${article.key}: source prose rendered`);
      assert.equal(await page.locator('.guide-body script').count(),0,`${article.key}: source markup escaped`);
      assert.equal(await page.locator('.guide-body[data-guide-document]').count(),1,`${article.key}: exactly one guide body`);
      for (const language of ['zh','en']) {
        // Navigate at the target viewport and wait for the no-JS document's
        // styles/resources; do not sample an in-flight desktop-to-phone resize.
        await page.setViewportSize({width:390,height:844});
        await page.goto(base+localePath(article.path,language),{waitUntil:'load'});
        const geometry = await page.evaluate(()=>({page:document.documentElement.scrollWidth,width:innerWidth}));
        assert.equal(geometry.page>geometry.width,false,`${article.key}/${language}: no-JS article fits 390px; code and tables scroll locally`);
      }
      await page.setViewportSize({width:1280,height:900});
    }
    await page.goto(base+'/guide/start-here/',{waitUntil:'domcontentloaded'});
    await page.locator('.guide-body h2').first().scrollIntoViewIfNeeded();
    const headingId=await page.locator('.guide-body h2').first().getAttribute('id');
    assert.ok(headingId);
    await page.goto(`${base}/guide/start-here/#${encodeURIComponent(headingId)}`,{waitUntil:'domcontentloaded'});
    assert.equal(await page.locator(`#${headingId}`).count(),1,'generated article anchor resolves');
    check('no-JS: all five route families and guide articles are direct-loadable, mirrored, anchored and substantive');
    await routeContext.close();
  }

  {
    const noJs = await browser.newContext({javaScriptEnabled:false, viewport:{width:1280,height:900}});
    const page = await noJs.newPage();
    await page.goto(base, {waitUntil:'domcontentloaded'});
    for (const id of ['#top','#capabilities','#journey','#keep','#review','#how-it-works','#continue','#try']) {
      assert.equal(await page.locator(id).count() > 0, true, `no-JS: ${id} present`);
    }
    assert.equal(await page.locator('.brand-wordmark-svg').count(), 3, 'no-JS: three native wordmarks');
    for (const id of ['lake-light','lake-touch','moon-toggle','clawd-hello','tail-secret','motion-toggle','keep-clawd','crossing-clawd']) assert.equal(await page.locator(`#${id}`).isVisible(), false, `no-JS: ${id} does not leave an inert control`);
    const text = await page.locator('#main').innerText();
    assert.match(text, /Claude Code|Claude/, 'no-JS: product description visible');
    // The six capabilities are server-rendered: ids match the catalog and every card is readable.
    const capIds = await page.locator('#capabilities article[data-capability]').evaluateAll(nodes => nodes.map(node => node.dataset.capability));
    assert.deepEqual(capIds.slice().sort(), catalogIds.slice().sort(), 'no-JS: six capability cards match the catalog ids');
    assert.match(await page.locator('#capabilities').innerText(), /[\u4e00-\u9fff]/, 'no-JS: capability section carries Chinese copy');
    assert.equal(await page.locator('#how-it-works .workflow > li').count(), 4, 'no-JS: four workflow steps present');
    await assertProductCopyVisible(page, 'no-JS');
    await assertCapabilityComposition(page,'Chinese no-JS',1280);
    assert.equal(await page.locator('#how-it-works .execution-entries > div').count(), 3, 'no-JS: three execution entries present');
    // The interactive workflow trail ships hidden: without JS its host stays hidden and the
    // four-step explanation is still the authoritative, visible copy.
    const noJsTrail = page.locator('#how-it-works #workflow-trail');
    assert.equal(await noJsTrail.count(), 1, 'no-JS: workflow trail host is present in the markup');
    assert.equal(await noJsTrail.evaluate(node => node.hidden), true, 'no-JS: workflow trail host stays hidden');
    const noJsTrailVisible = await noJsTrail.evaluate(node => {
      const style = getComputedStyle(node);
      return style.display !== 'none' && node.getClientRects().length > 0 && !node.hidden;
    });
    assert.equal(noJsTrailVisible, false, 'no-JS: no interactive trail surface is shown');
    assert.match(await page.locator('#how-it-works .workflow').innerText(), /[\u4e00-\u9fff]{4,}/, 'no-JS: the four-step text is still readable');
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
    // The short film keeps a plain, clickable entrance without JavaScript: an anchor straight to the
    // real same-origin media file, with a caption track and no preloaded player, dialog or autoplay.
    const noJsFilm = page.locator('#film-open');
    assert.equal(await noJsFilm.count(), 1, 'no-JS: a native film link exists');
    assert.equal(await noJsFilm.evaluate(node => node.tagName === 'A' && node.isConnected), true, 'no-JS: the film entrance is a real anchor');
    const noJsFilmHref = await noJsFilm.getAttribute('href');
    assert.match(noJsFilmHref, /media\/lintel-intro\.mp4$/, 'no-JS: the link points at the same-origin film');
    assert.equal(await noJsFilm.isVisible(), true, 'no-JS: the film link is visible');
    assert.equal(await page.locator('#film-dialog[open], dialog#film-dialog[open]').count(), 0, 'no-JS: nothing pre-opens the film dialog');
    assert.equal(await page.locator('video').first().evaluate(node => node.autoplay), false, 'no-JS: the film never autoplays');
    assert.match(await page.locator('video source').first().getAttribute('src'), /media\/lintel-intro\.mp4$/, 'no-JS: the video source is the same-origin film');
    assert.equal(await page.locator('#film-dialog video').count(), 1, 'no-JS: the film dialog markup is server-rendered');
    check('no-JS: capabilities, four steps, reading example and docs all visible; native reading opens without JavaScript');
    await noJs.close();
  }

  // Both locale URLs ship complete static text. The real language links navigate without JS.
  {
    const noJs = await browser.newContext({javaScriptEnabled:false, viewport:{width:390,height:844}});
    const page = await noJs.newPage();
    page.on('request',request=>{if(!request.url().startsWith(base))external.push(request.url());});
    page.on('response',response=>{if(response.status()>=400)errors.push(`${response.status()} ${response.url()}`);});
    await page.goto(base + '/en/');
    assert.equal(await page.locator('html').getAttribute('lang'),'en');
    assert.equal(await page.title(),'Lintel — Claude Code environment and data manager');
    assert.equal(await page.locator('link[rel="canonical"]').getAttribute('href'),'https://lintel.page/en/');
    assert.match(await page.locator('meta[name="description"]').getAttribute('content'),/Claude Code/);
    assert.match(await page.locator('#try .preview-note').innerText(),/No formal download/);
    assert.equal(await page.locator('.language-links [data-site-language="en"]').getAttribute('aria-current'),'page');
    for (const route of ['/download/','/guide/','/inside/','/releases/']) {
      assert.ok(await page.locator(`a[href="/en${route}"]`).count(), `English home exposes /en${route}`);
    }
    await assertCapabilityComposition(page,'English no-JS',390);
    assert.equal(await page.locator('.hero-actions [data-site-quickstart="primary"]').getAttribute('href'),docBase+'quickstart.en.md');
    assert.equal(await page.locator('.hero-actions [data-site-quickstart="alternate"]').getAttribute('href'),docBase+'quickstart.md');
    assert.match(await page.locator('#try .reading-links:not(.site-route-links)').innerText(),/Current capabilities and limits \(Chinese\)/);
    for (const name of ['operator-guide','agents','architecture']) assert.ok(await page.locator(`#try a[href="${docBase}${name}.en.md"]`).count());
    for (const [id,links] of Object.entries(cardLinks)) {
      const href = await page.locator(`#capabilities [data-capability="${id}"] h3 a`).getAttribute('href');
      assert.ok(links.map(link=>localTaskGuide(link,'en')).includes(href),`English ${id} links its translated guide`);
    }
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'English no-JS mobile has no horizontal overflow');
    await assertProductCopyVisible(page,'English no-JS');
    await page.locator('#sample-pages summary').click();
    assert.equal(await page.locator('#sample-pages').evaluate(node=>node.open),true);
    assert.equal(await page.locator('#workflow-trail').evaluate(node=>node.hidden),true);
    assert.match(await page.locator('#film-open-label').innerText(),/20-second/);
    await page.locator('.language-links [data-site-language="zh"]').click();
    assert.equal(new URL(page.url()).pathname,'/');
    assert.equal(await page.locator('html').getAttribute('lang'),'zh-CN');
    await page.locator('.language-links [data-site-language="en"]').click();
    assert.equal(new URL(page.url()).pathname,'/en/');
    check('English no-JS page: complete narration/scenes/limits/metadata, real locale links and English-first guides');
    await noJs.close();
  }

  const context = await browser.newContext({viewport:{width:1440,height:900},colorScheme:'light'});
  const page = await context.newPage();
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => { requests.push(request.url()); if (!request.url().startsWith(base)) external.push(request.url()); });
  page.on('response', response => { if (response.status() >= 400) errors.push(`${response.status()} ${response.url()}`); });
  await page.goto(base, {waitUntil:'networkidle'});
  assert.match(await page.title(), /Lintel/);
  assert.equal(await page.locator('h1').count(), 1, 'a single page headline');
  // The film is never requested merely by loading the page: capture the request log now, before any
  // film interaction, so this is a real "before opening" observation (the log only grows later).
  const initialRequests = requests.slice();
  assert.equal(initialRequests.some(url => /lintel-intro\.mp4$/.test(url)), false, 'the film is not requested on initial page load');

  const scenery = await page.evaluate(async () => {
    const game = await import('/clawd-game.mjs');
    const canvas = document.createElement('canvas'); canvas.width = 320; canvas.height = 230;
    const ctx = canvas.getContext('2d');
    const style = getComputedStyle(document.querySelector('.clawd-game'));
    const palette = Object.fromEntries(['soft','line','muted','accent'].map(key=>[key,style.getPropertyValue('--'+key).trim()]));
    game.drawScenery(ctx, 320, { distance: game.PLACE_LENGTH * .91 }, { ...palette, night: false }, false);
    const mixed = ctx.getImageData(0, 0, 320, 230).data;
    const opaque = mixed.every((v, i) => i % 4 !== 3 || v === 255);
    game.drawScenery(ctx, 320, { distance: 1500 }, { ...palette, night: true }, true);
    const frozen = canvas.toDataURL();
    game.drawScenery(ctx, 320, { distance: 2000 }, { ...palette, night: true }, true);
    const reducedStatic = canvas.toDataURL() === frozen;
    const places = [0, 1, 2].map(index => {
      game.drawScenery(ctx, 320, { distance: index * game.PLACE_LENGTH + 350 }, { ...palette, night: false }, false);
      return canvas.toDataURL();
    });
    return { opaque, reducedStatic, distinctPlaces: new Set(places).size };
  });
  assert.deepEqual(scenery, { opaque: true, reducedStatic: true, distinctPlaces: 3 }, 'canonical game scenery blends opaquely, freezes reduced-motion parallax and renders three distinct places');
  check('shared runner scenery: three places, opaque crossfade and static reduced-motion pixels');

  // Local visual responses: no coach, discovery count, note or automatic game start.
  const scene = page.locator('.hero-overlay');
  const canvasPixels = () => page.locator('#lake-light').evaluate(node => node.toDataURL());
  const waterInk = () => page.locator('#lake-light').evaluate(node => {
    const ctx = node.getContext('2d'), top = Math.floor(node.height * .7);
    const {data} = ctx.getImageData(0, top, node.width, node.height - top);
    let peak = 0, left = node.width, right = -1;
    for (let i = 3; i < data.length; i += 4) {
      if (!data[i]) continue;
      peak = Math.max(peak, data[i]);
      const x = ((i - 3) / 4) % node.width; left = Math.min(left, x); right = Math.max(right, x);
    }
    return {peak, span:right < 0 ? 0 : right - left};
  });
  await page.waitForFunction(() => document.querySelector('.hero-overlay').dataset.animation === 'running');
  assert.equal(await page.locator('#scene-note,#scene-discoveries,#sky-toggle,.scene-invitation,.water-hint').count(), 0, 'superseded guidance has been removed');
  const initialPixels = await canvasPixels(); await page.waitForTimeout(180);
  assert.notEqual(await canvasPixels(), initialPixels, 'automatic character light actually changes across time');
  const lockupBefore = await page.locator('.hero-lockup').boundingBox();
  const frameBox = await scene.boundingBox();
  assert.ok(lockupBefore.width / frameBox.width < .35, 'desktop identity leaves more landscape breathing room');
  const previewBox = await page.locator('.hero-preview').boundingBox();
  assert.ok(previewBox.y - lockupBefore.y - lockupBefore.height > 24, 'Preview has its own space below the lockup');
  await page.mouse.move(1240, 540);
  await page.waitForFunction(() => Math.abs(parseFloat(document.querySelector('.hero-overlay').style.getPropertyValue('--scene-x'))) > .4);
  assert.deepEqual(await page.locator('.hero-lockup').boundingBox(), lockupBefore, 'pointer depth keeps native identity anchored');
  assert.equal((await waterInk()).peak, 0, 'daylight water canvas begins clear');
  await page.locator('#lake-touch').click({position:{x:940,y:45}});
  assert.equal(await scene.getAttribute('data-ripple'), '1', 'a real lake click creates a wave');
  await page.waitForTimeout(180); const earlyWave = await waterInk();
  assert.ok(earlyWave.peak > 0);
  await page.waitForTimeout(700); const laterWave = await waterInk();
  assert.ok(laterWave.span > earlyWave.span, 'the actual water rings expand');
  assert.ok(laterWave.peak < earlyWave.peak, 'the actual ring opacity fades');
  await page.waitForFunction(() => document.querySelector('.hero-overlay').dataset.ripples === '0', null, {timeout:3500});
  assert.equal((await waterInk()).peak, 0, 'expired rings really disappear from the canvas');
  await page.locator('#moon-toggle').focus(); await page.keyboard.press('Enter');
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark', 'moon and header share the theme owner');
  assert.equal(await page.locator('#moon-toggle').evaluate(node => node === document.activeElement), true, 'a visual response leaves keyboard focus at its trigger');
  await page.locator('#clawd-hello').focus(); await page.keyboard.press('Space');
  assert.equal(await page.locator('#clawd-hello').getAttribute('data-reacting'), 'true');
  assert.equal(await page.locator('#clawd-hello .scene-response.heart').isVisible(), true, 'Clawd responds in the picture');
  await page.locator('#clawd-hello').click();
  assert.equal(await page.locator('#clawd-hello .scene-response').count(), 1, 'repeated pokes replace their transient response');
  assert.equal(await page.locator('.clawd-game').getAttribute('data-state'), 'ready', 'Clawd never recruits or starts the runner');
  await page.locator('#tail-secret').focus(); await page.keyboard.press('Enter');
  assert.equal(await page.locator('.hero-word .tail-eyes').isVisible(), true, 'the quiet orange foot briefly opens its eyes');
  await page.waitForFunction(() => !document.querySelector('.hero-word').dataset.reacting && !document.querySelector('#clawd-hello').dataset.reacting);
  assert.equal(await page.locator('.tail-eyes').isVisible(), false, 'the wordmark returns to its minimal resting shape');
  await page.mouse.click(8, 400); await page.keyboard.type('lintel');
  assert.equal(await scene.getAttribute('data-sky'), 'stars', 'the unadvertised word reveals the sky');
  await page.keyboard.type('lintel');
  assert.equal(await scene.getAttribute('data-sky'), 'quiet', 'the same word puts the sky away');
  await page.locator('#motion-toggle').click();
  assert.equal(await page.locator('html').getAttribute('data-motion'), 'paused');
  const still = await canvasPixels(); await page.mouse.move(100, 500); await page.waitForTimeout(180);
  assert.equal(await canvasPixels(), still, 'pause actually freezes ambient drawing and pointer depth');
  assert.equal(await page.locator('.hero-reflection').evaluate(node => getComputedStyle(node).animationName), 'none');
  await page.locator('#lake-touch').focus(); await page.keyboard.press('Enter');
  assert.equal(await scene.getAttribute('data-ripple'), '2', 'keyboard makes a static water response while paused');
  const pausedWave = await canvasPixels(); await page.waitForTimeout(180);
  assert.equal(await canvasPixels(), pausedWave, 'paused waves do not animate');
  await page.waitForFunction(() => document.querySelector('.hero-overlay').dataset.ripples === '0', null, {timeout:3500});
  assert.notEqual(await canvasPixels(), pausedWave, 'static responses still expire rather than accumulating forever');
  await page.locator('#motion-toggle').click();
  await page.waitForFunction(() => document.querySelector('.hero-overlay').dataset.animation === 'running');
  // Headless tabs stay visible: this is a synthetic branch check, not an OS background claim.
  await page.evaluate(() => {
    Object.defineProperty(document, 'hidden', {configurable:true, value:true});
    document.dispatchEvent(new Event('visibilitychange'));
  });
  assert.equal(await scene.getAttribute('data-animation'), 'resting');
  const hiddenPixels = await canvasPixels(); await page.waitForTimeout(180);
  assert.equal(await canvasPixels(), hiddenPixels, 'hidden visibility cancels ambient work');
  await page.evaluate(() => { delete document.hidden; document.dispatchEvent(new Event('visibilitychange')); });
  await page.waitForFunction(() => document.querySelector('.hero-overlay').dataset.animation === 'running');
  check('visual waves expand, fade and expire independently; quiet responses keep focus; pause/reduced lifetimes are finite');
  // Both lower Clawds remain tied to their raster subject, including responsive crops.
  for (const [section,id,kind] of [['keep','keep-clawd','boat'],['review','crossing-clawd','sparkles']]) {
    await page.locator(`#${section} .story-world`).scrollIntoViewIfNeeded();
    await page.locator(`#${section} img`).evaluate(img => img.decode());
    const button = page.locator(`#${id}`); await button.waitFor({state:'visible'});
    await button.click();
    assert.equal(await button.locator(`.scene-response.${kind}`).isVisible(), true);
    await button.focus(); await page.keyboard.press('Enter');
    assert.equal(await button.locator('.scene-response').count(), 1, 'keyboard and repeat clicks share one response');
    assert.equal(await button.evaluate(node => document.activeElement === node), true);
    await page.waitForFunction(id => !document.getElementById(id).dataset.reacting, id, {timeout:3000});
    assert.equal(await button.locator('.scene-response').count(), 0, 'lower-scene response cleans itself up');
  }
  await page.locator('#how-it-works').evaluate(node => node.scrollIntoView({block:'start',behavior:'instant'}));
  await page.waitForFunction(() => document.querySelector('.hero-overlay').dataset.animation === 'resting');
  const away = await canvasPixels(); await page.waitForTimeout(180);
  assert.equal(await canvasPixels(), away, 'leaving the scene stops ambient work');
  await page.evaluate(() => scrollTo({top:0,behavior:'instant'}));
  await page.waitForFunction(() => document.querySelector('.hero-overlay').dataset.animation === 'running');
  await page.locator('#theme-toggle').click();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'light');
  assert.equal(await page.locator('.clawd-game').getAttribute('data-state'), 'ready');
  check('scene: anchored identity, hidden keyboard sky, two lower Clawd reactions, offscreen stop and no guidance');

  for (const route of ['/download/','/guide/','/inside/','/releases/']) {
    assert.ok(await page.locator(`a[href="${route}"]`).count(), `home exposes ${route}`);
  }
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
  // viewport top once smooth scrolling has finished. Entering the tolerance band alone is still
  // mid-animation: resetting immediately can race a pending compositor scroll and cancel the
  // following native fragment scroll. Observe three unchanged RAF samples at each boundary.
  // This is fixture sequencing; href/Enter, smooth scrolling and the 4s landing gate stay real.
  const scrollPaddingTop = await page.evaluate(() => parseFloat(getComputedStyle(document.documentElement).scrollPaddingTop) || 0);
  const resetNavigationOrigin = async () => {
    await page.evaluate(() => scrollTo({top:0, left:0, behavior:'instant'}));
    await page.waitForFunction(state => {
      const x = scrollX, y = scrollY;
      state.frames = state.x === x && state.y === y ? state.frames + 1 : 1;
      state.x = x; state.y = y;
      return x === 0 && y === 0 && state.frames >= 3;
    }, {x:null,y:null,frames:0}, {timeout:4000});
  };
  const assertNavigated = async (hash) => {
    const id = hash.slice(1);
    assert.equal(new URL(page.url()).hash, hash, `click navigates to ${hash}`);
    // Bounded wait: the section reaches its normal scroll position (top near scroll-padding-top).
    try {
      await page.waitForFunction(([sel, pad, state]) => {
        const node = document.getElementById(sel);
        if (!node) return false;
        const top = node.getBoundingClientRect().top;
        const y = scrollY;
        state.frames = state.y === y && state.top === top ? state.frames + 1 : 1;
        state.y = y; state.top = top;
        return top >= -1 && top <= pad + 4 && state.frames >= 3;
      }, [id, scrollPaddingTop, {y:null,top:null,frames:0}], {timeout:4000});
    } catch (error) {
      const observed = await page.evaluate(sel => {
        const rect = document.getElementById(sel)?.getBoundingClientRect();
        return {hash:location.hash,scrollY,viewport:innerHeight,documentHeight:document.documentElement.scrollHeight,
          target:rect?{top:rect.top,bottom:rect.bottom}:null,focus:document.activeElement?.getAttribute('href'),
          scrollPadding:getComputedStyle(document.documentElement).scrollPaddingTop,
          scrollBehavior:getComputedStyle(document.documentElement).scrollBehavior,
          visibility:document.visibilityState};
      }, id);
      throw new Error(`navigation did not settle: ${JSON.stringify(observed)}`, {cause:error});
    }
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
  await resetNavigationOrigin();
  await page.locator('header nav a[href="#capabilities"]').click();
  await assertNavigated('#capabilities');
  // Keyboard: focus the #how-it-works nav link and press Enter.
  await resetNavigationOrigin();
  const hiwNav = page.locator('header nav a[href="#how-it-works"]');
  await hiwNav.focus();
  assert.equal(await hiwNav.evaluate(node => node === document.activeElement),true,'Enter begins on the real focused navigation link');
  await page.keyboard.press('Enter');
  await assertNavigated('#how-it-works');
  check('real header nav click and keyboard Enter navigate to #capabilities/#how-it-works at their normal scroll position');

  // --- Hero: native identity, tail geometry, raster plate actually loads. ---
  assert.equal(await page.locator('.brand-wordmark-svg').count(), 3, 'header, hero and footer wordmarks');
  for (const logo of await page.locator('.brand-wordmark-svg').all()) {
    const relation = await logo.evaluate(svg => {
      const glyph = svg.querySelector('.wordmark-glyphs'), b = glyph.getBBox();
      const foot = glyph.getPointAtLength(glyph.getTotalLength());
      const tail = svg.querySelector('.wordmark-foot,.cursor'), c = tail.getBBox();
      return {variant:svg.dataset.variant, gap:c.x-b.x-b.width, extension:c.x+c.width-b.x-b.width,
        baselineGap:Math.abs(c.y+c.height-foot.y), footLeftGap:Math.abs(c.x-foot.x),
        height:b.height, round:Number(tail.getAttribute('rx')), mask:glyph.getAttribute('mask')};
    });
    assert.ok(relation.baselineGap < .02, 'every cursor/foot retains the final l baseline');
    assert.ok(relation.round > 0 && relation.round < relation.height * .03, 'rounding stays restrained');
    if (relation.variant === 'integrated') {
      assert.ok(relation.extension > 0 && relation.extension < relation.height * .25, 'the integrated orange tail remains short');
      assert.ok(relation.footLeftGap < .02, 'hero orange covers the whole final l foot');
      assert.match(relation.mask, /hero-word-foot-cut/, 'old foot is cut away so rounded corners cannot reveal it');
    } else {
      assert.equal(relation.variant, 'detached');
      assert.ok(relation.gap > 0 && relation.gap < relation.height * .1, 'header/footer cursor stands just clear of the glyph');
      assert.equal(relation.mask, null, 'detached cursor keeps the complete original glyph');
    }
  }
  const cursorStyle = await page.locator('.cursor').first().evaluate(node => {
    const s = getComputedStyle(node); return {name:s.animationName,duration:s.animationDuration};
  });
  assert.deepEqual(cursorStyle,{name:'blink',duration:'2.6s'},'the independent underscore keeps the App slow blink');
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
    const icon = card.locator(':scope > .capability-copy > svg.capability-icon');
    assert.equal(await icon.count(),1,`card ${id} has one small title icon`);
    assert.equal(await icon.getAttribute('aria-hidden'),'true');
    assert.equal(await icon.getAttribute('viewBox'),'0 0 32 32');
    assert.equal(await icon.evaluate(node=>node.tabIndex),-1);
    assert.equal(await card.getByRole('img').count(),1,'only the large scene is announced as an image');
    const link = card.locator('h3 a');
    assert.equal(await link.count(), 1, `card ${id} has one title link`);
    const href = await link.getAttribute('href');
    const allowed = cardLinks[id].map(suffix => localTaskGuide(suffix));
    assert.ok(allowed.includes(href), `card ${id} links a real doc/anchor (${href})`);
    // The linked doc file exists in the working tree (no dangling guide); fragment stripped.
    const [filename, anchor] = cardLinks[id].find(suffix=>localTaskGuide(suffix)===href).split('#');
    const doc = await readFile(path.join(repo, 'docs', filename), 'utf8');
    if (anchor) assert.ok(doc.includes(`<a id="${anchor}">`), `capability ${id} links an existing named anchor`);
    // Chinese copy plus a concrete result; the near-boundary paragraph is a specific limitation.
    assert.match(await card.innerText(), /[\u4e00-\u9fff]/, `card ${id} carries Chinese copy`);
    assert.equal(await card.locator('p.capability-boundary').count(), 1, `card ${id} carries a near-boundary note`);
    assert.ok((await card.locator('p.capability-boundary').innerText()).trim().length > 8, `card ${id} boundary names a concrete limit, not a slogan`);
  }
  check('#capabilities: six catalog-aligned cards, real doc links that resolve and per-card boundary notes');
  await assertCapabilityComposition(page,'Chinese 1440px',1440);

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

  // --- #workflow-trail: the four-step "work package journey" interaction (workflow-trail.mjs). ---
  // Contract: host #workflow-trail inside #how-it-works, four button[data-trail-step=0..3] with
  // aria-pressed (exactly one true), button[data-trail-next], p[data-trail-caption][role=status].
  // host.dataset.step drives the coordinator's SVG/CSS picture. Native click/Enter/Space open a
  // step; stepping only updates the explicitly synthetic caption and never fabricates a real
  // result receipt or approval. The module owns this JS state; the coordinator owns HTML/CSS.
  const trail = page.locator('#how-it-works #workflow-trail');
  assert.equal(await trail.count(), 1, 'single #workflow-trail host inside #how-it-works');
  assert.equal(await trail.evaluate(node => node.hidden), false, 'workflow trail reveals itself once initialised');
  const trailSteps = page.locator('#workflow-trail [data-trail-step]');
  assert.equal(await trailSteps.count(), 4, 'four interactive steps');
  assert.deepEqual(await trailSteps.evaluateAll(nodes => nodes.map(n => n.dataset.trailStep)), ['0','1','2','3'], 'steps are ordered 0..3');
  const trailCaption = page.locator('#workflow-trail [data-trail-caption]');
  assert.equal(await trailCaption.count(), 1, 'one caption line');
  assert.equal(await trailCaption.getAttribute('role'), 'status', 'caption is a polite status region');
  const trailNext = page.locator('#workflow-trail [data-trail-next]');
  assert.equal(await trailNext.count(), 1, 'one next-step control');
  // The four plain <li> steps keep their text; the interaction only adds a near-by highlight.
  assert.equal(await page.locator('#how-it-works .workflow > li').count(), 4, 'plain four-step list retained beside the interaction');
  // Initial state is step 0 with exactly one pressed step.
  assert.equal(await trail.evaluate(node => node.dataset.step), '0', 'starts on step 0');
  assert.deepEqual(await trailSteps.evaluateAll(nodes => nodes.map(n => n.getAttribute('aria-pressed'))), ['true','false','false','false'], 'only step 0 pressed initially');
  assert.match(await trailCaption.innerText(), /第 1 步/, 'caption explains step 1');
  assert.match(await trailCaption.innerText(), /合成/, 'caption is explicitly synthetic');

  // Latest-state wins: click step 3, then immediately reselect another step; the final state must
  // reflect only the last selection (one pressed step, matching dataset.step and caption).
  const settleTrail = async step => {
    await page.waitForFunction(s => document.querySelector('#workflow-trail')?.dataset.step === String(s), step, {timeout:2000});
    const pressed = await trailSteps.evaluateAll(nodes => nodes.map(n => n.getAttribute('aria-pressed')));
    assert.equal(pressed.filter(v => v === 'true').length, 1, `exactly one step pressed at step ${step}`);
    assert.equal(pressed[step], 'true', `aria-pressed follows step ${step}`);
  };
  await trailSteps.nth(3).click();
  await settleTrail(3);
  await trailSteps.nth(1).click();
  await settleTrail(1);
  // Step 2 wording: the same plan is approved; a condition change needs a new preview.
  await trailSteps.nth(2).click();
  await settleTrail(2);
  const approveCopy = await trailCaption.innerText();
  assert.match(approveCopy, /批准/, 'approval step explains approval of the same plan');
  assert.match(approveCopy, /条件|变化/, 'approval step states a condition change needs a new preview');
  assert.match(approveCopy, /新.*预览|重新预览/, 'approval step names a fresh preview');
  // Step 4 wording: accepted ≠ completed, query the original ID and never replay; originals remain.
  await trailSteps.nth(3).click();
  await settleTrail(3);
  const receiptCopy = await trailCaption.innerText();
  assert.match(receiptCopy, /已接收|接收/, 'receipt step separates accepted');
  assert.match(receiptCopy, /已完成|完成/, 'receipt step separates completed');
  assert.match(receiptCopy, /原 ID|原任务|原 id/i, 'receipt step points at the original ID');
  assert.match(receiptCopy, /不重(新)?提交|不重放/, 'receipt step says no replay/re-submit');
  assert.match(receiptCopy, /原件/, 'receipt step keeps the originals present');
  // The narrative explanation never claims the website performed a real operation.
  for (const copy of [approveCopy, receiptCopy]) {
    assert.match(copy, /合成|示例|只讲|不读取|不生成|不执行|不接受/, 'caption stays visibly synthetic');
    assert.doesNotMatch(copy, /已批准|执行成功|已执行|权限已|operation succeeded/i, 'no fabricated approval/success receipt');
  }
  check('#workflow-trail initial step 0, latest-state reselection, and honest approve/receipt captions');

  // Keyboard parity: real Enter and Space on a focused step select it (native button behaviour).
  await trailSteps.nth(0).click(); await settleTrail(0);
  await trailSteps.nth(2).focus();
  await page.keyboard.press('Enter');
  await settleTrail(2);
  await trailSteps.nth(1).focus();
  await page.keyboard.press('Space');
  await settleTrail(1);

  // next advances 0→1→2→3 then loops back to step 0 with a changed label.
  await trailSteps.nth(0).click(); await settleTrail(0);
  await trailNext.click(); await settleTrail(1);
  await trailNext.click(); await settleTrail(2);
  await trailNext.click(); await settleTrail(3);
  const lastNextLabel = await trailNext.innerText();
  assert.match(lastNextLabel, /回到|第 1 步|第 一 步|↺/, 'next relabels to return at the last step');
  await trailNext.click(); await settleTrail(0);
  check('#workflow-trail keyboard Enter/Space select a step and next advances then loops home');

  // --- #try carries scope/limits, the unverified-download boundary and real CN/EN + docs links. ---
  const tryText = await page.locator('#try').innerText();
  assert.match(tryText, /未|没有|尚无|未验收|未验证|no .*download|not .*verified/i, '#try states unverified/absent delivery boundaries');
  assert.match(tryText, /Preview|预览/i, '#try keeps the Preview stage label');
  for (const href of ['quickstart.md','quickstart.en.md','agents.md','operator-guide.md','current-state.md','architecture.md']) {
    assert.equal(await page.locator(`#try a[href*="${href}"]`).count() > 0, true, `#try links ${href}`);
  }
  check('#try exposes scope/limits, download boundary and CN/EN quickstart + six docs links');

  // --- Theme preference survives reload and swaps the hero night plate. ---
  await page.locator('#theme-toggle').click();
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
  await assertNightClawdEyes(page, '1440px cover');
  await page.setViewportSize({width:900,height:900});
  await assertNightClawdEyes(page, '900px contain');
  await page.setViewportSize({width:1440,height:900});
  check('night story composites preserve orange bodies and dark eyes through cover/contain cropping');

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
  if (await page.locator('html').getAttribute('data-theme') !== 'dark') await page.locator('#theme-toggle').click();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark', 'desktop-night capture is night');
  await capture(page, 'desktop-night.png');
  if(qa)await captureSection(page,'#capabilities','capabilities-zh-1440-dark.png');
  await page.locator('#theme-toggle').click();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'light');
  for (const character of await page.locator('.story-character').all()) assert.equal(await character.isVisible(), false, 'day keeps only the accepted original illustration');
  await capture(page, 'desktop-day.png');
  if(qa)await captureSection(page,'#capabilities','capabilities-zh-1440-light.png');
  await captureFull(page, 'desktop-full.png');
  // Cover the new contract sections as well as the retained story sections.
  if (qa) for (const id of ['capabilities','how-it-works','keep','review','continue']) await captureSection(page, `#${id}`, `section-${id}-1440.png`);

  // --- Reduced motion: reveals complete, decorative cursor static. ---
  await page.emulateMedia({reducedMotion:'reduce'});
  await page.evaluate(() => scrollTo({top:0,behavior:'instant'}));
  // Poll the reduced-motion contract on a fixed interval. This wait observes the settled state of
  // static, non-animated behaviour, so it should not hinge on the browser's frame cadence: the
  // default 'raf' polling only advances when a frame is produced, which is fragile for a state
  // wait. 50ms is only the observation cadence; the original timeout is unchanged.
  await page.waitForFunction(() => document.documentElement.dataset.motion === 'reduced', undefined, {polling:50});
  assert.equal(await page.locator('#motion-toggle').isDisabled(), true, 'system reduced motion stays authoritative');
  const reducedPixels = await canvasPixels(); await page.waitForTimeout(180);
  assert.equal(await canvasPixels(), reducedPixels, 'reduced motion freezes actual ambient drawing');
  await page.locator('#lake-touch').click();
  assert.equal(await scene.getAttribute('data-ripples'), '1');
  const reducedWave = await canvasPixels(); await page.waitForTimeout(180);
  assert.equal(await canvasPixels(), reducedWave, 'reduced-motion response stays static');
  await page.waitForFunction(() => document.querySelector('.hero-overlay').dataset.ripples === '0', null, {timeout:3500});

  assert.equal(await page.locator('.cursor').first().evaluate(node=>getComputedStyle(node).animationName), 'none');
  assert.equal(await page.locator('#try').evaluate(node=>getComputedStyle(node).opacity), '1');
  for (const art of await page.locator('.story-art').all()) {
    assert.equal(await art.evaluate(node=>getComputedStyle(node).transform), 'none');
    assert.ok(await art.evaluate(node=>Number(getComputedStyle(node).opacity)) > 0, 'reduced motion shows every scene immediately');
  }
  // The workflow trail stays usable with motion reduced: it is a state switch, not an animation.
  await page.locator('#workflow-trail [data-trail-step="2"]').scrollIntoViewIfNeeded();
  await page.locator('#workflow-trail [data-trail-step="2"]').click();
  await page.waitForFunction(() => document.querySelector('#workflow-trail')?.dataset.step === '2', null, {timeout:2000});
  assert.equal(await page.locator('#workflow-trail [data-trail-step="2"]').getAttribute('aria-pressed'), 'true', 'workflow trail still switches steps under reduced motion');
  await page.locator('#workflow-trail [data-trail-step="0"]').click();
  check('reduced motion completes reveals, stills the decorative cursor and keeps the workflow trail usable');

  // --- Mobile 390 and 320: no horizontal overflow, all six capabilities and four steps keep
  //     their content, no clipped paths. Names match the theme: switch to night and assert before
  //     mobile-night, switch back to day before mobile-day. ---
  await page.setViewportSize({width:390,height:844});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth > innerWidth), false, 'no 390px overflow');
  assert.equal(await page.locator('#capabilities article[data-capability]').count(), 6, '390px keeps all six capability cards');
  await assertProductCopyVisible(page, '390px');
  await assertCapabilityComposition(page,'Chinese 390px',390);
  assert.equal(await page.locator('#how-it-works .workflow > li').count(), 4, '390px keeps all four workflow steps');
  assert.equal(await page.locator('#how-it-works .execution-entries > div').count(), 3, '390px keeps all three execution entries');
  // The interaction stays reachable and functional at mobile width.
  assert.equal(await page.locator('#workflow-trail [data-trail-step]').count(), 4, '390px keeps all four trail steps');
  await page.locator('#workflow-trail [data-trail-step="1"]').scrollIntoViewIfNeeded();
  await page.locator('#workflow-trail [data-trail-step="1"]').click();
  await page.waitForFunction(() => document.querySelector('#workflow-trail')?.dataset.step === '1', null, {timeout:2000});
  assert.equal(await page.locator('#workflow-trail [data-trail-step="1"]').getAttribute('aria-pressed'), 'true', '390px trail switches steps');
  await page.locator('#workflow-trail [data-trail-step="0"]').click();
  if (await page.locator('html').getAttribute('data-theme') !== 'dark') await page.locator('#theme-toggle').click();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark', 'mobile-night capture is night');
  await assertNightClawdEyes(page, '390px mobile crop');
  await capture(page, 'mobile-night.png');
  if(qa)await captureSection(page,'#capabilities','capabilities-zh-390-dark.png');
  await captureFull(page, 'mobile-full.png');
  if (qa) for (const id of ['capabilities','how-it-works','keep','review','continue']) await captureSection(page, `#${id}`, `section-${id}-390.png`);
  await page.locator('.cg-jump').click();
  assert.equal(await page.locator('.clawd-game').getAttribute('data-state'), 'running');
  await page.locator('.cg-pause').click();
  assert.equal(await page.locator('.clawd-game').getAttribute('data-state'), 'paused');
  if (await page.locator('html').getAttribute('data-theme') !== 'light') await page.locator('#theme-toggle').click();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'light', 'mobile-day capture is day');
  await capture(page, 'mobile-day.png');
  if(qa)await captureSection(page,'#capabilities','capabilities-zh-390-light.png');
  await page.setViewportSize({width:320,height:844});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth > innerWidth), false, 'no 320px overflow');
  assert.equal(await page.locator('#capabilities article[data-capability]').count(), 6, '320px keeps all six capability cards');
  await assertProductCopyVisible(page, '320px');
  await assertCapabilityComposition(page,'Chinese 320px',320);
  assert.equal(await page.locator('#how-it-works .workflow > li').count(), 4, '320px keeps all four workflow steps');
  await page.locator('#theme-toggle').click();
  await assertNightClawdEyes(page, '320px mobile crop');
  check('390px and 320px mobile reflow, six capabilities + four steps retained, no overflow and direct jump/pause controls');

  // --- The opt-in short film: opened only by an explicit action, never before. This runs INSIDE the
  //     live desktop context (before it closes); the non-persisted pagehide is the last functional
  //     action, since the shared site.mjs pagehide handler disposes the film player. ---
  const filmDialog = page.locator('#film-dialog');
  assert.equal(await filmDialog.count(), 1, 'native <dialog> film surface present');
  assert.equal(await filmDialog.evaluate(node => node.open), false, 'film dialog starts closed');
  assert.equal(await page.locator('#film-dialog video').count(), 1, 'one native <video>');
  for (const attr of ['controls', 'playsinline']) assert.equal(await page.locator('#film-dialog video').evaluate((node, a) => node.hasAttribute(a), attr), true, `film video has ${attr}`);
  assert.equal(await page.locator('#film-dialog video').evaluate(node => node.preload), 'none', 'film video uses preload="none"');
  assert.equal(await page.locator('#film-dialog video').evaluate(node => node.autoplay), false, 'film video never autoplays');
  assert.equal(await page.locator('#film-dialog video[autoplay]').count(), 0, 'no autoplay attribute on the film video');
  assert.equal(await page.locator('#film-dialog track[kind="captions"]').count(), 1, 'native <track> captions present');
  assert.equal(await page.locator('#film-dialog video').evaluate(node => node.classList.contains('film-video')), true, 'film video uses the bare cinematic surface');
  assert.equal(await page.locator('#film-dialog .film-close').count(), 1, 'one close control');
  assert.ok(((await page.locator('#film-dialog .film-close').getAttribute('aria-label')) || '').trim().length > 0, 'close control is labelled');
  assert.ok(((await filmDialog.getAttribute('aria-label')) || '').trim().length > 0, 'film dialog has an accessible name');
  // A closed dialog is explicitly inert for assistive tech; opening must clear that synchronously so
  // a just-opened dialog is never left hidden in the accessibility tree.
  assert.equal(await filmDialog.getAttribute('aria-hidden'), 'true', 'closed dialog is aria-hidden');
  await page.locator('#film-open').scrollIntoViewIfNeeded();
  await page.locator('#film-open').focus();
  assert.equal(await page.locator('#film-open').evaluate(node => node === document.activeElement), true, 'film trigger is keyboard focusable');
  await page.keyboard.press('Enter');
  assert.equal(await filmDialog.evaluate(node => node.open), true, 'Enter opens the native modal dialog');
  assert.notEqual(await filmDialog.getAttribute('aria-hidden'), 'true', 'the open dialog is not hidden from assistive tech');
  assert.equal(await page.locator('#film-dialog video').evaluate(node => node.paused), true, 'opening never starts playback');
  assert.equal(requests.some(url => /lintel-intro\.mp4$/.test(url)), false, 'opening the dialog alone requests no film');
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => { const d = document.getElementById('film-dialog'); return d && !d.open; }, null, {timeout:2000});
  assert.equal(await page.locator('#film-dialog video').evaluate(node => node.paused), true, 'Escape stops and leaves the film paused');
  assert.equal(await page.locator('#film-open').evaluate(node => node === document.activeElement), true, 'closing restores focus to the trigger');
  assert.equal(await filmDialog.getAttribute('aria-hidden'), 'true', 'closing restores the inert aria-hidden state');
  await page.locator('#film-open').click();
  const blockedClick = await page.locator('#theme-toggle').click({timeout:1200}).then(() => false).catch(() => true);
  assert.equal(blockedClick, true, 'the obscured page is not clickable while the dialog is open');
  await page.locator('#film-dialog .film-close').click();
  await page.waitForFunction(() => { const d = document.getElementById('film-dialog'); return d && !d.open; }, null, {timeout:2000});
  assert.equal(await filmDialog.evaluate(node => node.open), false, 'the visible close button dismisses the film');
  assert.equal(await page.locator('#film-open').evaluate(node => node === document.activeElement), true, 'close button restores trigger focus');
  check('film: opt-in dialog opens by explicit action only, honours Escape/close/focus restore and accessible hidden state');
  assert.equal(await page.locator('iframe').count(), 0, 'no iframe (no YouTube/CDN embed)');
  assert.equal(await page.evaluate(() => !/(youtube|vimeo|cdn\.|googleapis)/i.test(document.documentElement.outerHTML)), true, 'no third-party video host referenced in markup');

  // --- Layout: the film surface keeps 16:9 with no overflow at 390/320/1440. ---
  for (const [w, h] of [[1440, 900], [390, 844], [320, 844]]) {
    await page.setViewportSize({width: w, height: h});
    await page.evaluate(() => scrollTo({top:0, behavior:'instant'}));
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false, `no horizontal overflow at ${w}px`);
    await page.locator('#film-open').click();
    const vbox = await page.locator('#film-dialog video').boundingBox();
    assert.ok(vbox && vbox.width > 0, `${w}px: the video surface is laid out`);
    assert.ok(Math.abs(vbox.width / vbox.height - 16 / 9) < 0.03, `${w}px: the video keeps a 16:9 box (${(vbox.width / vbox.height).toFixed(3)})`);
    assert.ok(vbox.width <= w && vbox.height <= h, `${w}px: the video fits the viewport without overflow`);
    assert.equal(await page.locator('#film-dialog .film-close').isVisible(), true, `${w}px: the close control stays visible`);
    const closeBox = await page.locator('#film-dialog .film-close').boundingBox();
    assert.ok(closeBox.x >= 0 && closeBox.x + closeBox.width <= w && closeBox.y >= 0 && closeBox.y + closeBox.height <= h, `${w}px: close control is inside the viewport`);
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false, `no horizontal overflow with the film open at ${w}px`);
    await page.keyboard.press('Escape');
    await page.waitForFunction(() => { const d = document.getElementById('film-dialog'); return d && !d.open; }, null, {timeout:2000});
  }
  await page.setViewportSize({width:1440, height:900});
  await page.evaluate(() => scrollTo({top:0, behavior:'instant'}));
  check('film layout: 16:9 with a visible close control and no overflow at 1440/390/320');

  // Locale changes update existing nodes and module labels. The URL owns the choice; no
  // language storage, no second game runner, no remount and no loss of open/selected state.
  {
    const bilingual = await browser.newContext({viewport:{width:1440,height:900},colorScheme:'light'});
    const live = await bilingual.newPage();
    live.on('pageerror',error=>errors.push(error.message));
    live.on('request',request=>{if(!request.url().startsWith(base))external.push(request.url());});
    live.on('response',response=>{if(response.status()>=400)errors.push(`${response.status()} ${response.url()}`);});
    await live.goto(base+'/?trial=synthetic#how-it-works');
    await live.locator('#sample-pages').evaluate(node=>{node.open=true;});
    await live.locator('#workflow-trail [data-trail-step="3"]').click();
    await live.locator('#motion-toggle').click();
    await live.locator('#theme-toggle').click();
    await live.locator('.cg-start').click();
    await live.waitForFunction(()=>Number(document.querySelector('.cg-score').textContent)>0);
    await live.locator('.cg-pause').click();
    const score = await live.locator('.cg-score').innerText();
    await live.evaluate(()=>{
      window.__siteLanguageNodes = ['#clawd-game canvas','#film-dialog video','#sample-pages','#workflow-trail','.hero-overlay'].map(selector=>document.querySelector(selector));
      window.__siteLanguageDocument = document;
    });
    await live.locator('[data-site-language="en"]').click();
    assert.equal(new URL(live.url()).pathname,'/en/');
    assert.equal(new URL(live.url()).search,'?trial=synthetic');
    assert.equal(new URL(live.url()).hash,'#how-it-works');
    assert.equal(await live.locator('html').getAttribute('lang'),'en');
    assert.equal(await live.title(),'Lintel — Claude Code environment and data manager','live locale updates the document title');
    assert.equal(await live.locator('meta[property="og:title"]').getAttribute('content'),'Lintel — Claude Code environment and data manager');
    assert.equal(await live.locator('meta[name="twitter:title"]').getAttribute('content'),'Lintel — Claude Code environment and data manager');
    assert.equal(await live.locator('meta[name="twitter:description"]').getAttribute('content'),await live.locator('meta[name="description"]').getAttribute('content'));
    assert.equal(await live.locator('meta[property="og:image:alt"]').getAttribute('content'),'Lintel — Claude Code environment and data manager');
    assert.deepEqual(await live.locator('#site-structured-data').evaluate(node => {
      const page = JSON.parse(node.textContent)['@graph'][1];
      return [page.url,page.inLanguage,page.name];
    }), ['https://lintel.page/en/','en','Lintel — Claude Code environment and data manager']);
    assert.equal(await live.locator('html').getAttribute('data-theme'),'dark');
    assert.equal(await live.locator('html').getAttribute('data-motion'),'paused');
    assert.equal(await live.locator('#motion-toggle').getAttribute('aria-label'),'Resume the scenery');
    assert.equal(await live.locator('#motion-toggle').getAttribute('title'),'Resume the scenery');
    assert.equal(await live.locator('#motion-toggle').getAttribute('aria-pressed'),'true');
    assert.equal(await live.locator('#moon-toggle').getAttribute('aria-label'),'Touch the moon to switch to daylight');
    assert.equal(await live.locator('#theme-toggle').getAttribute('aria-label'),'Switch to daylight');
    assert.equal(await live.locator('#theme-toggle').getAttribute('title'),'Switch to daylight');
    assert.equal(await live.locator('#workflow-trail').getAttribute('data-step'),'3');
    assert.match(await live.locator('[data-trail-caption]').innerText(),/Step 4:.*accepted.*completed/);
    assert.match(await live.locator('[data-trail-next]').innerText(),/Back to step 1/);
    assert.equal(await live.locator('#sample-pages').evaluate(node=>node.open),true);
    assert.equal(await live.locator('.clawd-game').getAttribute('data-state'),'paused');
    assert.equal(await live.locator('.clawd-game').getAttribute('lang'),'en');
    assert.equal(await live.locator('.cg-score').innerText(),score);
    assert.match(await live.locator('.cg-instructions-text').innerText(),/hop immediately; keep holding/);
    assert.equal(await live.evaluate(()=>document===window.__siteLanguageDocument && window.__siteLanguageNodes.every((node,index)=>node===document.querySelector(['#clawd-game canvas','#film-dialog video','#sample-pages','#workflow-trail','.hero-overlay'][index]))),true);
    assert.deepEqual(await live.evaluate(()=>Object.keys(localStorage).filter(key=>!['lintel.site.theme','lintel.site.clawd.best'].includes(key))),[],'locale switching writes no language or other new storage');
    await live.goBack();
    assert.equal(await live.locator('html').getAttribute('lang'),'zh-CN');
    assert.equal(await live.title(),'Lintel — Claude Code 环境与资料管理器','history restores the Chinese document title');
    assert.equal(await live.locator('meta[name="twitter:title"]').getAttribute('content'),'Lintel — Claude Code 环境与资料管理器');
    assert.deepEqual(await live.locator('#site-structured-data').evaluate(node => {
      const page = JSON.parse(node.textContent)['@graph'][1];
      return [page.url,page.inLanguage];
    }), ['https://lintel.page/','zh-CN']);
    assert.equal(await live.locator('meta[name="twitter:description"]').getAttribute('content'),await live.locator('meta[name="description"]').getAttribute('content'));
    assert.equal(await live.locator('html').getAttribute('data-motion'),'paused');
    assert.equal(await live.locator('#motion-toggle').getAttribute('aria-label'),'让山湖动起来');
    assert.equal(await live.locator('#theme-toggle').getAttribute('aria-label'),'切换到日光');
    assert.equal(await live.locator('#workflow-trail').getAttribute('data-step'),'3');
    assert.equal(await live.locator('.cg-score').innerText(),score);
    await live.goForward();
    assert.equal(await live.locator('html').getAttribute('lang'),'en');
    assert.equal(await live.locator('html').getAttribute('data-motion'),'paused');
    assert.equal(await live.locator('#motion-toggle').getAttribute('aria-label'),'Resume the scenery');
    // The modal makes the page behind it inert. Browser history still changes the page language
    // while preserving the exact video, open dialog and focus; it never starts/stops playback.
    await live.locator('#film-open').click();
    await live.goBack();
    assert.equal(await live.locator('#film-dialog').evaluate(node=>node.open),true);
    assert.equal(await live.locator('#film-dialog video').evaluate(node=>node===window.__siteLanguageNodes[1]),true);
    assert.match(await live.locator('#film-dialog').getAttribute('aria-label'),/看 20 秒/);
    assert.equal(await live.locator('.film-close').evaluate(node=>node===document.activeElement),true);
    await live.goForward();
    assert.match(await live.locator('#film-dialog').getAttribute('aria-label'),/Watch the 20-second/);
    await live.keyboard.press('Escape');
    assert.equal(await live.locator('#film-open').evaluate(node=>node===document.activeElement),true);
    check('live locale + back/forward: same DOM/game/video; selected step, open reading, score, scenery/theme, query/hash and modal focus preserved');
    // English visual acceptance keeps small decorative title icons together with six full-width
    // alternating scene rows, stacked copy-first on mobile, in both themes and reduced motion.
    for (const width of [1440,390,320]) {
      await live.setViewportSize({width,height:900});
      assert.equal(await live.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,`English ${width}px has no page overflow`);
      assert.match(await live.locator('#play-title').innerText(), /Stay here\s+a little longer\./, 'responsive line break preserves English word spacing');
      await assertProductCopyVisible(live,`English ${width}px`);
      await assertCapabilityComposition(live,`English ${width}px`,width);
      for (const theme of ['dark','light']) {
        if(await live.locator('html').getAttribute('data-theme')!==theme)await live.locator('#theme-toggle').click();
        assert.equal(await live.locator('html').getAttribute('data-theme'),theme);
        if(qa)await captureSection(live,'#capabilities',`capabilities-en-${width}-${theme}.png`);
      }
      if(qa)await captureFull(live,`full-en-${width}.png`);
    }
    await live.emulateMedia({reducedMotion:'reduce'});
    await live.waitForFunction(()=>document.documentElement.dataset.motion==='reduced',undefined,{polling:50});
    assert.equal(await live.locator('#motion-toggle').getAttribute('aria-label'),'System reduced motion is on');
    await live.locator('#workflow-trail [data-trail-step="2"]').click();
    assert.match(await live.locator('[data-trail-caption]').innerText(),/Step 3:.*new preview and approval/);
    await live.locator('[data-site-language="zh"]').click();
    assert.equal(await live.locator('#workflow-trail').getAttribute('data-step'),'2');
    assert.equal(await live.locator('#motion-toggle').getAttribute('aria-label'),'系统已减少动效');
    await live.reload();
    assert.equal(await live.locator('html').getAttribute('lang'),'zh-CN','explicit locale URL survives reload');
    await live.goto(base+'/en/');
    assert.equal(await live.locator('html').getAttribute('lang'),'en','direct English URL mounts in English');
    check('English 1440/390/320 day/night: six small title icons and alternating large scenes, mobile copy-first layout, readable labels/copy, no overflow and reduced-motion locale changes');
    await bilingual.close();
  }

  // --- The non-persisted pagehide is the LAST functional action in this context. ---
  await page.locator('#film-open').click();
  assert.equal(await filmDialog.evaluate(node => node.open), true, 'the film is open before the unload lifecycle');
  await page.evaluate(() => window.dispatchEvent(new PageTransitionEvent('pagehide', {persisted:false})));
  assert.equal(await filmDialog.evaluate(node => node.open), false, 'pagehide lifecycle closes the film');
  assert.equal(await page.locator('#film-dialog video').evaluate(node => node.paused), true, 'pagehide lifecycle leaves the film paused');
  check('film: non-persisted pagehide disposes and closes the film');

  assert.deepEqual(errors, []); assert.deepEqual(external, []);
  // Scripted open/close (and this open) never call play(), so the film is never fetched in this
  // context; only an explicit play() loads it (covered in the actual-media context below).
  assert.equal(requests.some(url => /lintel-intro\.mp4$/.test(url)), false, 'scripted open/close never fetches the film; only play does');
  await context.close();

  // --- Actual media (independent): only runs when main supplies the built film + poster via
  //     LINTEL_SITE_MEDIA_DIR. These are real bytes: opening stays no-autoplay/no-fetch, and only an
  //     explicit play() loads and advances the film. ---
  if (mediaDir) {
    const real = await browser.newContext({viewport:{width:1440,height:900}});
    const realPage = await real.newPage();
    realPage.on('pageerror', error => errors.push(error.message));
    const filmRequests = [];
    realPage.on('request', request => { if (/lintel-intro\.mp4$/.test(request.url())) filmRequests.push(request.url()); });
    await realPage.goto(base, {waitUntil:'domcontentloaded'});
    await realPage.locator('[data-site-language="en"]').click();
    assert.equal(await realPage.locator('html').getAttribute('lang'),'en');
    await realPage.locator('#film-open').scrollIntoViewIfNeeded();
    const beforeOpen = await realPage.locator('#film-dialog video').evaluate(v => ({readyState:v.readyState, paused:v.paused}));
    assert.equal(beforeOpen.paused, true, 'actual media: the film never autoplays on load');
    await realPage.locator('#film-open').click();
    // Opening asserts the no-autoplay contract without demanding a loaded video.
    assert.equal(await realPage.locator('#film-dialog video').evaluate(v => v.paused), true, 'actual media: opening does not start playback');
    assert.deepEqual(filmRequests, [], 'actual media: no film request until the reader presses play');
    // Explicitly start playback; only then must the real bytes download and the timeline advance.
    await realPage.locator('#film-dialog video').evaluate(v => v.play());
    await realPage.waitForFunction(() => { const v = document.querySelector('#film-dialog video'); return v && v.readyState >= 1; }, null, {timeout:20000});
    await realPage.waitForFunction(() => { const v = document.querySelector('#film-dialog video'); return v.currentTime > 0 && !v.paused; }, null, {timeout:15000});
    const meta = await realPage.locator('#film-dialog video').evaluate(v => ({w:v.videoWidth, h:v.videoHeight, d:v.duration, t:v.currentTime}));
    assert.equal(meta.w, 1920, 'actual media is 1920px wide');
    assert.equal(meta.h, 1080, 'actual media is 1080px tall');
    assert.ok(meta.d >= 19.9 && meta.d <= 20.3, `actual media duration is ~20s (got ${meta.d.toFixed(3)})`);
    assert.ok(meta.t > 0, 'actual media timeline advances when explicitly played');
    assert.ok(filmRequests.length >= 1, 'actual media: play() actually fetched the film (request evidence)');
    const beforeLanguage = await realPage.locator('#film-dialog video').evaluate(video=>{window.__playingFilm=video;return video.currentTime;});
    await realPage.goBack();
    assert.equal(await realPage.locator('html').getAttribute('lang'),'zh-CN');
    assert.equal(await realPage.locator('#film-dialog').evaluate(node=>node.open),true,'locale history leaves the playing modal open');
    assert.equal(await realPage.locator('#film-dialog video').evaluate(video=>video===window.__playingFilm && !video.paused),true,'locale history keeps the exact playing video');
    await realPage.waitForFunction(time=>document.querySelector('#film-dialog video').currentTime>time+.1,beforeLanguage);
    await realPage.goForward();
    assert.equal(await realPage.locator('html').getAttribute('lang'),'en');
    assert.equal(await realPage.locator('#film-dialog video').evaluate(video=>video===window.__playingFilm && !video.paused),true);
    await realPage.locator('#film-dialog video').evaluate(v => v.pause());
    await realPage.locator('#film-dialog .film-close').click();
    await realPage.waitForFunction(() => { const d = document.getElementById('film-dialog'); return d && !d.open; }, null, {timeout:3000});
    assert.equal(await realPage.locator('#film-dialog video').evaluate(v => v.paused && v.currentTime === 0), true, 'closing the film stops and resets actual playback');
    check('actual media: explicit play() advances the 1920x1080 ~20s film, live locale history preserves playback, opening never fetches/autoplays and close resets');
    await real.close();
  }

  // --- Blocked local storage: theme/game stay usable, scores temporary. ---
  const blocked = await browser.newContext({viewport:{width:390,height:844},hasTouch:true});
  await blocked.addInitScript(()=>{Object.defineProperty(window,'localStorage',{get(){throw new DOMException('Storage denied','SecurityError');}});});
  const blockedPage = await blocked.newPage();
  blockedPage.on('pageerror',error=>errors.push(error.message));
  await blockedPage.goto(base);
  const themeBeforeTouch = await blockedPage.locator('html').getAttribute('data-theme');
  await blockedPage.locator('#theme-toggle').tap();
  assert.notEqual(await blockedPage.locator('html').getAttribute('data-theme'), themeBeforeTouch, 'mobile theme works with blocked storage');
  await blockedPage.locator('#clawd-hello').tap();
  // Touch dispatch can finish before Chromium delivers the synthesized click.
  // Require the same visible feedback within less than its finite lifetime.
  await blockedPage.locator('#clawd-hello .scene-response.heart').waitFor({state:'visible',timeout:1000});
  await blockedPage.locator('#lake-touch').tap();
  assert.equal(await blockedPage.locator('.hero-overlay').getAttribute('data-ripple'), '1', 'touch creates the same local water response');
  assert.equal(await blockedPage.locator('.clawd-game').getAttribute('data-state'), 'ready', 'touch discoveries keep the game opt-in');
  for (const [section,id,kind] of [['keep','keep-clawd','boat'],['review','crossing-clawd','sparkles']]) {
    await blockedPage.locator(`#${section} .story-world`).scrollIntoViewIfNeeded();
    await blockedPage.locator(`#${section} img`).evaluate(img => img.decode());
    const button = blockedPage.locator(`#${id}`); await button.waitFor({state:'visible'});
    const box = await button.boundingBox();
    assert.ok(box.x >= 0 && box.x + box.width <= 390, 'the raster Clawd hit area fits the mobile crop');
    await button.tap();
    await button.locator(`.scene-response.${kind}`).waitFor({state:'visible',timeout:1000});
  }
  check('390px real touch on hero and both lower raster Clawds works with blocked storage');

  assert.match(await blockedPage.locator('.cg-storage').innerText(), /存储不可用/);
  await blockedPage.locator('.cg-jump').tap();
  assert.equal(await blockedPage.locator('.clawd-game').getAttribute('data-state'), 'running');
  await blockedPage.locator('.cg-pause').tap();
  assert.equal(await blockedPage.locator('.clawd-game').getAttribute('data-state'), 'paused');
  check('touch jump/pause and blocked local storage with temporary scores');
  await blocked.close();

  {
    const routes = await browser.newContext({viewport:{width:1440,height:900},colorScheme:'light'});
    const routePage = await routes.newPage();
    routePage.on('pageerror', error => errors.push(error.message));
    routePage.on('request', request => { if (!request.url().startsWith(base)) external.push(request.url()); });
    routePage.on('response', response => { if (response.status() >= 400) errors.push(`${response.status()} ${response.url()}`); });
    for (const routePath of ['/download/','/guide/','/inside/','/releases/']) {
      await routePage.goto(base+routePath, {waitUntil:'networkidle'});
      assert.equal(await routePage.locator('html').getAttribute('lang'),'zh-CN');
      assert.equal(await routePage.locator('#theme-toggle').count(),1,`${routePath}: shared theme control`);
      assert.equal(await routePage.locator('.hero-overlay, #workflow-trail, #film-dialog, #clawd-game').count(),0,`${routePath}: home widgets are not mounted`);
      assert.equal(await routePage.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,`${routePath}: desktop no overflow`);
      await routePage.locator('[data-site-route-language="en"]').click();
      assert.equal(new URL(routePage.url()).pathname,`/en${routePath}`);
      assert.equal(await routePage.locator('html').getAttribute('lang'),'en');
      assert.equal(await routePage.locator('link[rel="canonical"]').getAttribute('href'),`https://lintel.page/en${routePath}`);
      await routePage.locator('#theme-toggle').click();
      const theme=await routePage.locator('html').getAttribute('data-theme');
      await routePage.goBack();
      assert.equal(new URL(routePage.url()).pathname,routePath);
      assert.equal(await routePage.locator('html').getAttribute('data-theme'),theme,'route history preserves the same document theme');
      assert.equal(await routePage.locator('html').getAttribute('lang'),'zh-CN');
      await routePage.goForward();
      assert.equal(await routePage.locator('html').getAttribute('lang'),'en');
      await routePage.setViewportSize({width:390,height:844});
      assert.equal(await routePage.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,`${routePath}: mobile no overflow`);
      assert.ok((await routePage.locator('#main').innerText()).trim().length>200,`${routePath}: bodies remain readable at mobile width`);
      await routePage.setViewportSize({width:1440,height:900});
    }
    await routePage.goto(base+'/guide/start-here/?synthetic=route#create-an-archive-only',{waitUntil:'networkidle'});
    const articleThemeBefore=await routePage.locator('html').getAttribute('data-theme');
    await routePage.locator('[data-site-route-language="en"]').click();
    assert.equal(new URL(routePage.url()).pathname,'/en/guide/start-here/');
    assert.equal(new URL(routePage.url()).search,'?synthetic=route');
    assert.equal(new URL(routePage.url()).hash,'#create-an-archive-only');
    assert.equal(await routePage.locator('html').getAttribute('data-theme'),articleThemeBefore);
    assert.match(await routePage.locator('.guide-body h1').innerText(),/First use of Lintel/);
    assert.ok((await routePage.locator('.guide-body [href^="/en/guide/"]').count())>0,'article links switch to English guides');
    assert.match(await routePage.locator('a[data-route-copy="repositoryOriginal"]').getAttribute('href'),/start-here\.en\.md$/);
    assert.equal(await routePage.locator('.guide-body script, .guide-body style').count(),0);
    assert.equal(await routePage.locator('#sample-pages, #workflow-trail, #film-dialog, #clawd-game').count(),0,'article does not mount home interactions');
    assert.equal(await routePage.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
    await routePage.setViewportSize({width:390,height:844});
    assert.equal(await routePage.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'article 390px no overflow');
    assert.ok((await routePage.locator('.guide-body p').first().innerText()).length>20);
    await routePage.emulateMedia({reducedMotion:'reduce'});
    assert.equal(await routePage.locator('.cursor').evaluate(node=>getComputedStyle(node).animationName),'none','shared brand cursor respects reduced motion');
    check('routes: exact mirrors, scoped same-page language switch, query/hash, theme/history, no home widgets and 390px readability');
    await routes.close();
  }

  assert.deepEqual(errors, []); assert.deepEqual(external, []);
  check('no runtime/asset errors or external requests');
  console.log(JSON.stringify({status:'passed',checks},null,2));
} finally {
  await browser?.close(); await new Promise(resolve=>server.close(resolve));
}

// Deterministic identity composition; the illustration is a selected local raster plate.
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const identity = path.join(repo, 'apps/desktop/assets/identity/source');
const wordmark = await readFile(path.join(identity, 'lintel-wordmark.svg'), 'utf8');
const mark = await readFile(path.join(identity, 'lintel-mark.svg'), 'utf8');
const glyphs = [...wordmark.matchAll(/<path\b[^>]*\bd="([^"]+)"[^>]*\/>/g)];
const glyph = glyphs[0]?.[1];
const foot = wordmark.match(/<rect\b[^>]*\bid="wordmark-foot"[^>]*\/>/)?.[0];
const footMask = wordmark.match(/<mask\b[^>]*\bid="wordmark-foot-cut"[\s\S]*?<\/mask>/)?.[0];
const supports = [...mark.matchAll(/<rect\b[^>]*\/>/g)];
if (glyphs.length !== 1 || !foot || !footMask || supports.length !== 4) throw new Error('Review the canonical identity shapes before regenerating.');
const attr = (markup, name) => {
  const value = markup.match(new RegExp(`\\b${name}="([^"]+)"`))?.[1];
  if (value === undefined || !Number.isFinite(Number(value))) throw new Error(`Invalid identity ${name}`);
  return Number(value);
};
const geometry = markup => ['x', 'y', 'width', 'height', 'rx'].map(name => `${name}="${attr(markup, name)}"`).join(' ');
const wordWidth = attr(foot, 'x') + attr(foot, 'width');
const glyphEnd = attr(wordmark, 'data-glyph-end');
const cursorHeight = attr(foot, 'height') * .7;
const cursorWidth = attr(foot, 'height') * 2.6;
const cursorX = glyphEnd + attr(foot, 'height') * .8;
const detachedWidth = cursorX + cursorWidth;
const palette = {
  day: { paper: '#FAF9F5', text: '#3D3D3A', accent: '#C16A47', rule: '#BEB9AF' },
  night: { paper: '#24231F', text: '#EAE5DA', accent: '#E0A485', rule: '#716C61' },
};
function wordShapes(p, variant = 'integrated', id = 'wordmark-foot-cut', eyes = false) {
  if (variant === 'detached') {
    return `<path class="wordmark-glyphs" d="${glyph}" fill="${p.text}"/><rect class="cursor" x="${cursorX}" y="${attr(foot, 'y') + attr(foot, 'height') - cursorHeight}" width="${cursorWidth}" height="${cursorHeight}" rx="${attr(foot, 'rx') * .7}" fill="${p.accent}"/>`;
  }
  const eyeY = attr(foot, 'y') + 4;
  const eyeX = glyphEnd + 5;
  const wink = eyes ? `<path class="tail-eyes" d="M${eyeX} ${eyeY}h2.3v5.6h-2.3z m9.4 0h2.3v5.6h-2.3z" fill="#252320"/>` : '';
  return `<defs>${footMask.replaceAll('wordmark-foot-cut', id)}</defs><path class="wordmark-glyphs" d="${glyph}" mask="url(#${id})" fill="${p.text}"/><rect class="wordmark-foot" ${geometry(foot)} fill="${p.accent}"/>${wink}`;
}
function markShapes(p) {
  return supports.map((shape, index) => `<rect ${geometry(shape[0])} fill="${index === 0 || index === 3 ? p.accent : p.text}"/>`).join('');
}
const nativePalette = {text:'currentColor', accent:'var(--brand-accent)'};
const siteWordmark = variant => `<svg class="brand-wordmark-svg" data-variant="${variant}" viewBox="0 0 ${variant === 'detached' ? detachedWidth : wordWidth} 148.2" aria-hidden="true" focusable="false">${wordShapes(nativePalette, variant, 'hero-word-foot-cut', variant === 'integrated')}</svg>`;
const siteMark = `<svg class="brand-mark-svg" viewBox="0 0 690 410" aria-hidden="true" focusable="false">${markShapes(nativePalette)}</svg>`;

// Explicit blocks are projections of the identity sources, not another logo master.
const sitePath = path.join(repo, 'apps/site/index.html');
let site = await readFile(sitePath, 'utf8');
for (const [kind, markup, count] of [['wordmark', null, 3], ['mark', siteMark, 3]]) {
  const expression = new RegExp(`<!-- lintel-${kind}:start -->[\\s\\S]*?<!-- lintel-${kind}:end -->`, 'g');
  if ([...site.matchAll(expression)].length !== count) throw new Error(`Expected ${count} site ${kind} projections.`);
  let index = 0;
  site = site.replace(expression, () => {
    const projection = kind === 'wordmark' ? siteWordmark(index++ === 1 ? 'integrated' : 'detached') : markup;
    return `<!-- lintel-${kind}:start -->${projection}<!-- lintel-${kind}:end -->`;
  });
}

function exportSvg(theme, share = false) {
  const p = palette[theme];
  const width = share ? 1280 : 1600, height = share ? 720 : 900;
  const scaleWord = .625, scaleMark = 117 / 410, gap = 25;
  const lockupWidth = 690 * scaleMark + gap + wordWidth * scaleWord;
  const left = (1600 - lockupWidth) / 2;
  const portalScale = 99 / 410, portalLeft = 800 - 690 * portalScale / 2;
  const art = theme === 'night' ? 'lintel-landscape-night.png' : 'lintel-landscape.png';
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 1600 900" role="img" aria-labelledby="title desc" lang="en">
<title id="title">Lintel 0.1.0 Preview</title>
<desc id="desc">Keep your work. Know what changes. The terracotta tail extends from the last l. A fine ASCII forest, mountains and reflected lake surround Lintel's beam and a tiny Clawd.</desc>
<defs>
  <pattern id="ripples" width="4" height="5" patternUnits="userSpaceOnUse"><rect width="2" height="1" fill="white"/></pattern>
  <linearGradient id="fade" x2="0" y2="1"><stop stop-color="white"/><stop offset="1" stop-color="black"/></linearGradient>
  <linearGradient id="sky-paper" gradientUnits="userSpaceOnUse" x2="0" y2="270"><stop stop-color="black"/><stop offset="1" stop-color="white"/></linearGradient>
  <mask id="sky-fade"><rect width="1600" height="900" fill="url(#sky-paper)"/></mask>
  <mask id="water"><rect x="710" y="692" width="180" height="99" fill="url(#ripples)"/></mask>
  <mask id="reflection-fade"><rect x="710" y="692" width="180" height="99" fill="url(#fade)"/></mask>
</defs>
<rect width="1600" height="900" fill="${p.paper}"/>
<image href="../../apps/site/assets/${art}" y="100" width="1600" height="800" mask="url(#sky-fade)" style="mix-blend-mode:${theme === 'day' ? 'multiply' : 'normal'}"/>
<g id="header-mark" transform="translate(${left} 144) scale(${scaleMark})">${markShapes(p)}</g>
<g id="wordmark" transform="translate(${left + 690 * scaleMark + gap} 168.375) scale(${scaleWord})">${wordShapes(p)}</g>
<g id="preview" fill="${p.text}" text-anchor="middle" font-family="Menlo,Consolas,monospace">
  <path d="M584 304H674 M926 304H1016" stroke="${p.rule}"/>
  <text x="800" y="309" font-size="15" letter-spacing="5">0.1.0 PREVIEW</text>
</g>
<g id="copy" fill="${p.text}" text-anchor="middle" font-family="Baskerville,Georgia,serif" font-weight="400" font-size="42">
  <text x="800" y="389">Keep your work.</text>
  <text x="800" y="435">Know what changes.</text>
</g>
<g id="island-mark" transform="translate(${portalLeft} 592) scale(${portalScale})">${markShapes(p)}</g>
<g mask="url(#reflection-fade)" opacity=".6"><g mask="url(#water)"><g transform="translate(${portalLeft} 791) scale(${portalScale} ${-portalScale})">${markShapes(p)}</g></g></g>
</svg>\n`;
}

const output = path.join(repo, 'assets/preview');
await mkdir(output, {recursive:true});
const exports = [
  ['lintel-banner', 'day', false],
  ['lintel-banner-night', 'night', false],
  ['lintel-social-preview', 'day', true],
];
for (const [name, theme, share] of exports) await writeFile(path.join(output, `${name}.svg`), exportSvg(theme, share));
await writeFile(sitePath, site);

// Repository's disposable Chromium; no personal browser or remote asset.
const {chromium} = await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE || path.join(repo, 'extensions/browser/node_modules/playwright/index.mjs')).href);
const browser = await chromium.launch({headless:true});
try {
  for (const [name, , share] of exports) {
    const page = await browser.newPage({viewport:{width:share ? 1280 : 1600, height:share ? 720 : 900},deviceScaleFactor:1});
    await page.goto(pathToFileURL(path.join(output, `${name}.svg`)).href, {waitUntil:'load'});
    await page.evaluate(() => document.fonts.ready);
    await page.screenshot({path:path.join(output, `${name}.png`)});
    await page.close();
    console.log(`assets/preview/${name}.{svg,png}`);
  }
} finally { await browser.close(); }

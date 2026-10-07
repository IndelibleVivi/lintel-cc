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
const cursor = [...wordmark.matchAll(/<rect\b[^>]*\/>/g)];
const supports = [...mark.matchAll(/<rect\b[^>]*\/>/g)];
if (glyphs.length !== 1 || cursor.length !== 1 || supports.length !== 4) throw new Error('Review the canonical identity shapes before regenerating.');
const attr = (markup, name) => {
  const value = markup.match(new RegExp(`\\b${name}="([^"]+)"`))?.[1];
  if (value === undefined || !Number.isFinite(Number(value))) throw new Error(`Invalid identity ${name}`);
  return Number(value);
};
const geometry = markup => ['x', 'y', 'width', 'height', 'rx'].map(name => `${name}="${attr(markup, name)}"`).join(' ');
const wordWidth = attr(cursor[0][0], 'x') + attr(cursor[0][0], 'width');
const palette = {
  day: { paper: '#FAF9F5', text: '#3D3D3A', accent: '#C16A47', rule: '#BEB9AF' },
  night: { paper: '#24231F', text: '#EAE5DA', accent: '#E0A485', rule: '#716C61' },
};
function wordShapes(p) {
  return `<path class="wordmark-glyphs" d="${glyph}" fill="${p.text}"/><rect class="cursor" ${geometry(cursor[0][0])} fill="${p.accent}"/>`;
}
function markShapes(p) {
  return supports.map((shape, index) => `<rect ${geometry(shape[0])} fill="${index === 0 || index === 3 ? p.accent : p.text}"/>`).join('');
}
const nativePalette = {text:'currentColor', accent:'var(--brand-accent)'};
const siteWordmark = `<svg class="brand-wordmark-svg" viewBox="0 0 ${wordWidth} 148.2" aria-hidden="true" focusable="false">${wordShapes(nativePalette)}</svg>`;
const siteMark = `<svg class="brand-mark-svg" viewBox="0 0 690 410" aria-hidden="true" focusable="false">${markShapes(nativePalette)}</svg>`;

// Explicit blocks are projections of the identity sources, not another logo master.
const sitePath = path.join(repo, 'apps/site/index.html');
let site = await readFile(sitePath, 'utf8');
for (const [kind, markup, count] of [['wordmark', siteWordmark, 3], ['mark', siteMark, 3]]) {
  const expression = new RegExp(`<!-- lintel-${kind}:start -->[\\s\\S]*?<!-- lintel-${kind}:end -->`, 'g');
  if ([...site.matchAll(expression)].length !== count) throw new Error(`Expected ${count} site ${kind} projections.`);
  site = site.replace(expression, `<!-- lintel-${kind}:start -->${markup}<!-- lintel-${kind}:end -->`);
}

function exportSvg(theme, share = false) {
  const p = palette[theme];
  const width = share ? 1280 : 1600, height = share ? 640 : 800;
  const scaleWord = .76, scaleMark = 142 / 410, gap = 30;
  const lockupWidth = 690 * scaleMark + gap + wordWidth * scaleWord;
  const left = (1600 - lockupWidth) / 2;
  const portalScale = 114 / 410, portalLeft = 800 - 690 * portalScale / 2;
  const art = theme === 'night' ? 'lintel-landscape-night.png' : 'lintel-landscape.png';
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 1600 800" role="img" aria-labelledby="title desc" lang="en">
<title id="title">Lintel 0.1.0 Preview</title>
<desc id="desc">Keep your work. Know what changes. The terracotta tail extends from the last l. A fine ASCII forest, mountains and reflected lake surround Lintel's beam and a tiny Clawd.</desc>
<defs>
  <pattern id="ripples" width="4" height="5" patternUnits="userSpaceOnUse"><rect width="2" height="1" fill="white"/></pattern>
  <linearGradient id="fade" x2="0" y2="1"><stop stop-color="white"/><stop offset="1" stop-color="black"/></linearGradient>
  <mask id="water"><rect x="690" y="595" width="225" height="114" fill="url(#ripples)"/></mask>
  <mask id="reflection-fade"><rect x="690" y="595" width="225" height="114" fill="url(#fade)"/></mask>
</defs>
<rect width="1600" height="800" fill="${p.paper}"/>
<image href="../../apps/site/assets/${art}" width="1600" height="800"/>
<g id="header-mark" transform="translate(${left} 105) scale(${scaleMark})">${markShapes(p)}</g>
<g id="wordmark" transform="translate(${left + 690 * scaleMark + gap} 135.812) scale(${scaleWord})">${wordShapes(p)}</g>
<g id="preview" fill="${p.text}" text-anchor="middle" font-family="Menlo,Consolas,monospace">
  <path d="M550 272H674 M926 272H1050" stroke="${p.rule}"/>
  <text x="800" y="277" font-size="17" letter-spacing="5">0.1.0 PREVIEW</text>
</g>
<g id="copy" fill="${p.text}" text-anchor="middle" font-family="Baskerville,Georgia,serif" font-weight="400" font-size="42">
  <text x="800" y="352">Keep your work.</text>
  <text x="800" y="398">Know what changes.</text>
</g>
<g id="island-mark" transform="translate(${portalLeft} 474) scale(${portalScale})">${markShapes(p)}</g>
<g mask="url(#reflection-fade)" opacity=".6"><g mask="url(#water)"><g transform="translate(${portalLeft} 702) scale(${portalScale} ${-portalScale})">${markShapes(p)}</g></g></g>
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
    const page = await browser.newPage({viewport:{width:share ? 1280 : 1600, height:share ? 640 : 800},deviceScaleFactor:1});
    await page.goto(pathToFileURL(path.join(output, `${name}.svg`)).href, {waitUntil:'load'});
    await page.evaluate(() => document.fonts.ready);
    await page.screenshot({path:path.join(output, `${name}.png`)});
    await page.close();
    console.log(`assets/preview/${name}.{svg,png}`);
  }
} finally { await browser.close(); }

// Render a selected local SVG with the repository's existing Playwright runtime.
// Usage: node scripts/render-preview-svg.mjs SOURCE.svg OUTPUT.png [CSS_WIDTH]
import {chromium} from '../extensions/browser/node_modules/playwright/index.mjs';
import {mkdir, readFile} from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';

const [source, output, displayWidth] = process.argv.slice(2);
if (!source || !output) throw new Error('Expected SOURCE.svg OUTPUT.png [CSS_WIDTH]');
await readFile(source); // Fail before launching the browser for an absent input.
await mkdir(path.dirname(path.resolve(output)), {recursive: true});
const browser = await chromium.launch({headless: true});
try {
  const page = await browser.newPage({viewport: {width: 1800, height: 1100}});
  await page.goto(pathToFileURL(path.resolve(source)).href);
  await page.evaluate(() => document.fonts.ready);
  if (displayWidth) {
    const width = Number(displayWidth);
    if (!Number.isFinite(width) || width <= 0 || width > 4096) throw new Error('Invalid CSS width');
    await page.locator('svg').evaluate((svg, width) => {
      const ratio = svg.viewBox.baseVal.height / svg.viewBox.baseVal.width;
      svg.style.width = `${width}px`;
      svg.style.height = `${width * ratio}px`;
    }, width);
  }
  await page.locator('svg').screenshot({path: path.resolve(output)});
} finally {
  await browser.close();
}

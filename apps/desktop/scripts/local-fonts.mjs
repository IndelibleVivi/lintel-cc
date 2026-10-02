import { copyFile, mkdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// Explicit local-only asset preparation. No private project data is inspected.
const source = process.argv[2];
if (!source) throw new Error('Usage: node scripts/local-fonts.mjs /absolute/path/to/authorized-font-directory');
const target = fileURLToPath(new URL('../public/local-fonts/', import.meta.url));
await mkdir(target, { recursive: true });
for (const family of ['Sans', 'Serif', 'Mono']) {
  for (const style of ['Roman', 'Italic']) {
    const file = `Anthropic${family}-${style}.woff2`;
    await copyFile(path.join(source, file), path.join(target, file));
  }
}
console.log('Prepared six ignored fonts for this local candidate. Do not publish builds containing them without redistribution rights.');

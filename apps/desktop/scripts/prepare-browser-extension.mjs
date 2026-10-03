import { mkdir, readdir, readFile, writeFile, cp, rm } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const out = path.join(root, 'apps/desktop/src-tauri/browser-extension-bundle');
const build = spawnSync(process.execPath, [path.join(root, 'extensions/browser/scripts/build.mjs')], { cwd: root, encoding: 'utf8' });
if (build.stdout) process.stdout.write(build.stdout);
if (build.stderr) process.stderr.write(build.stderr);
if (build.status !== 0) throw new Error(`Canonical browser extension build failed: ${build.error?.message || build.status}`);

async function filesIn(directory, relative = '') {
  const files = [];
  for (const entry of await readdir(path.join(directory, relative), { withFileTypes: true })) {
    const name = relative ? `${relative}/${entry.name}` : entry.name;
    if (entry.isDirectory()) files.push(...await filesIn(directory, name));
    else if (entry.isFile()) files.push(name);
    else throw new Error(`Extension source contains a nonregular file: ${name}`);
  }
  return files.sort();
}
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
// Copy only canonical source files plus the build's generated config/manifest.
// Existing dist fixture directories and stale dist files never enter the App.
const sourceFiles = await filesIn(path.join(root, 'extensions/browser/src'));
const expected = [...new Set([...sourceFiles, 'manifest.json'])].sort();
const packages = {};
await mkdir(out, { recursive: true });
for (const browser of ['chromium', 'firefox']) {
  const built = path.join(root, 'extensions/browser/dist', browser);
  const manifest = JSON.parse(await readFile(path.join(built, 'manifest.json'), 'utf8'));
  const configText = await readFile(path.join(built, 'config.js'), 'utf8');
  const config = JSON.parse(configText.replace(/^export const CONFIG = /, '').replace(/;\s*$/, ''));
  if (config.fixture !== false || config.browser !== browser || manifest.name !== 'Lintel') throw new Error(`Refusing fixture extension: ${browser}`);
  const destination = path.join(out, browser);
  // This is an ignored, generated App resource directory, never an installed extension.
  await rm(destination, { recursive: true, force: true });
  await mkdir(destination, { recursive: true });
  const files = [];
  for (const name of expected) {
    const bytes = await readFile(path.join(built, name));
    await mkdir(path.dirname(path.join(destination, name)), { recursive: true });
    await cp(path.join(built, name), path.join(destination, name));
    files.push({ path: name, sha256: sha256(bytes), bytes: bytes.length });
  }
  const identity = files.map(file => `${file.path}\0${file.sha256}\0${file.bytes}\n`).join('');
  packages[browser] = { version: manifest.version, sha256: sha256(Buffer.from(identity)), bytes: files.reduce((sum, file) => sum + file.bytes, 0), files };
}
await writeFile(path.join(out, 'manifest.json'), JSON.stringify({ schema: 1, packages }, null, 2) + '\n');
console.log(`Prepared canonical bundled browser extensions ${packages.chromium.version} (Chromium + Firefox; no fixture).`);

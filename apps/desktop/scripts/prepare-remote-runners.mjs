import { readFile, mkdir, writeFile, copyFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
const root = fileURLToPath(new URL('../../../', import.meta.url));
const out = path.join(root, 'apps/desktop/src-tauri/runner-bundles');
const pkg = JSON.parse(await readFile(path.join(root, 'apps/desktop/package.json'), 'utf8'));
const runners = [];
// Both supported architectures must exist; no downloads or remote compiler.
for (const [target, machine] of [['x86_64-unknown-linux-musl', 62], ['aarch64-unknown-linux-musl', 183]]) {
  const source = path.join(root, 'target', target, 'release/lintel');
  const bytes = await readFile(source);
  if (!bytes.subarray(0, 4).equals(Buffer.from([127, 69, 76, 70])) || bytes[4] !== 2 || bytes[5] !== 1 || bytes.readUInt16LE(18) !== machine) throw new Error(`Wrong ELF architecture: ${target}`);
  const phoff = Number(bytes.readBigUInt64LE(32)), phsize = bytes.readUInt16LE(54), phcount = bytes.readUInt16LE(56);
  for (let i = 0; i < phcount; i++) if (bytes.readUInt32LE(phoff + i * phsize) === 3) throw new Error(`Runner needs a dynamic loader: ${target}`);
  runners.push({ target, version: pkg.version, protocol: 1, sha256: createHash('sha256').update(bytes).digest('hex'), bytes: bytes.length });
  await mkdir(path.join(out, target), { recursive: true });
  await copyFile(source, path.join(out, target, 'lintel'));
}
await writeFile(path.join(out, 'manifest.json'), JSON.stringify({ runners }, null, 2) + '\n');
console.log(`Prepared ${runners.length} static Linux runners for the desktop bundle.`);

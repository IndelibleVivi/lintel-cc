#!/usr/bin/env node
// Verify the complete macOS App code signature before a local Preview is kept.
import {access, lstat, readdir, realpath} from 'node:fs/promises';
import {constants} from 'node:fs';
import {spawnSync} from 'node:child_process';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const fail = message => { throw new Error(message); };
const MAX_FILES = 20_000;
const MAX_BYTES = 1024 * 1024 * 1024;

function run(command, args) {
  const result = spawnSync(command, args, {encoding: 'utf8', timeout: 120_000, maxBuffer: 8 * 1024 * 1024});
  if (result.error) fail(`${command} failed to run: ${result.error.message}`);
  return result;
}

async function inventory(bundle) {
  let files = 0;
  let bytes = 0;
  async function visit(directory) {
    const entries = await readdir(directory, {withFileTypes: true});
    for (const entry of entries) {
      if (++files > MAX_FILES) fail('App file budget exceeded');
      const item = path.join(directory, entry.name);
      const stat = await lstat(item);
      if (stat.isSymbolicLink()) {
        const absolute = await realpath(item);
        if (!absolute.startsWith(bundle + path.sep)) {
          fail(`App symlink escapes the bundle: ${path.relative(bundle, item)}`);
        }
        continue;
      }
      if (stat.isDirectory()) await visit(item);
      else if (stat.isFile()) {
        bytes += stat.size;
        if (bytes > MAX_BYTES) fail('App byte budget exceeded');
      } else fail(`App contains an unsupported entry: ${path.relative(bundle, item)}`);
    }
  }
  await visit(bundle);
  return {files, bytes};
}

export async function verifyMacosBundle(bundle) {
  if (process.platform !== 'darwin') fail('macOS bundle verification requires Darwin');
  if (typeof bundle !== 'string' || !path.isAbsolute(bundle)) fail('Expected an absolute App bundle path');
  if (path.basename(bundle) !== 'Lintel.app') fail('Expected the canonical Lintel.app bundle name');
  if (!(await lstat(bundle)).isDirectory()) fail('Expected a real App bundle directory');
  const executable = path.join(bundle, 'Contents', 'MacOS', 'lintel-desktop');
  const info = path.join(bundle, 'Contents', 'Info.plist');
  const resources = path.join(bundle, 'Contents', 'Resources');
  await access(executable, constants.R_OK | constants.X_OK);
  await access(info, constants.R_OK);
  await access(resources, constants.R_OK | constants.X_OK);

  const display = run('/usr/bin/codesign', ['-dvvv', bundle]);
  if (display.status !== 0) fail(`codesign could not inspect the App: ${display.stderr.trim()}`);
  const facts = display.stderr;
  if (/flags=.*linker-signed/m.test(facts)) fail('Executable has only a linker signature; Info.plist is not bound and Resources are not sealed');
  if (!/^Identifier=app\.lintel\.desktop$/m.test(facts)) fail('Signed bundle identifier is not app.lintel.desktop');
  if (!/^Info\.plist entries=\d+$/m.test(facts)) fail('Info.plist has no signed entries');
  if (/^Sealed Resources=none$/m.test(facts)) fail('No sealed Resources are present');
  if (!/^Sealed Resources version=[1-9]\d* .*files=[1-9]\d*$/m.test(facts)) fail('Sealed Resources metadata is missing');

  const strict = run('/usr/bin/codesign', ['--verify', '--deep', '--strict', '--verbose=4', bundle]);
  if (strict.status !== 0) fail(`codesign strict verification failed: ${strict.stderr.trim()}`);
  const id = run('/usr/bin/plutil', ['-extract', 'CFBundleIdentifier', 'raw', '-o', '-', info]);
  if (id.status !== 0 || id.stdout.trim() !== 'app.lintel.desktop') fail('Info.plist Bundle ID differs from the signed identity');
  const name = run('/usr/bin/plutil', ['-extract', 'CFBundleExecutable', 'raw', '-o', '-', info]);
  if (name.status !== 0 || name.stdout.trim() !== 'lintel-desktop') fail('Info.plist executable differs from the signed executable');
  const version = run('/usr/bin/plutil', ['-extract', 'CFBundleShortVersionString', 'raw', '-o', '-', info]);
  if (version.status !== 0 || !/^\d+\.\d+\.\d+(?:[-+][A-Za-z0-9.+-]+)?$/.test(version.stdout.trim())) fail('Invalid signed App version');
  const tree = await inventory(await realpath(bundle));
  const signature = facts.includes('Authority=Developer ID Application:') ? 'developer-id'
    : /^Signature=adhoc$/m.test(facts) ? 'ad-hoc' : 'other';
  return {bundle, version: version.stdout.trim(), files: tree.files, bytes: tree.bytes, signature};
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  verifyMacosBundle(process.argv[2]).then(result => console.log(JSON.stringify(result)))
    .catch(error => { console.error(error.message); process.exitCode = 1; });
}

import { readFile, mkdir, writeFile, copyFile, chmod } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const sourceManifest = path.join(root, 'extensions/browser/native-host/Cargo.toml');
const out = path.join(root, 'apps/desktop/src-tauri/browser-host-bundle');
const rustc = spawnSync('rustc', ['-vV'], { encoding: 'utf8' });
if (rustc.status !== 0) throw new Error(rustc.stderr || 'rustc is required to prepare the bundled browser host');
const hostTarget = rustc.stdout.match(/^host: (.+)$/m)?.[1];
if (!hostTarget) throw new Error('rustc did not report its host target');
// Tauri documents this hook environment variable as the actual build triple:
// https://v2.tauri.app/reference/environment-variables/#tauri-cli-hook-commands
const target = process.env.TAURI_ENV_TARGET_TRIPLE || hostTarget;
if (target !== hostTarget) throw new Error(`Bundled browser host currently requires a native target build (${hostTarget}); cross-target or universal App builds (${target}) are not supported. Build on the matching macOS architecture without --target.`);
const platform = target.endsWith('apple-darwin') ? 'macos' : target.includes('-linux-') ? 'linux' : null;
const architecture = target.split('-')[0];
if (!platform || !['x86_64', 'aarch64'].includes(architecture)) throw new Error(`Unsupported browser host build target: ${target}`);

// Build the existing dedicated stdio executable, never the desktop binary.
// Cargo owns dependency resolution; this script downloads no external executable.
const build = spawnSync('cargo', ['build', '--manifest-path', sourceManifest, '--release', '--locked', '--bin', 'lintel-browser-host', '--target', target, '--message-format=json'], { cwd: root, encoding: 'utf8', maxBuffer: 8 * 1024 * 1024 });
if (build.stderr) process.stderr.write(build.stderr);
if (build.status !== 0) throw new Error(`Browser host build failed (${build.status}): ${build.error?.message || ''}`);
const artifact = build.stdout.split('\n').filter(Boolean).map(line => JSON.parse(line)).findLast(item => item.reason === 'compiler-artifact' && item.target?.name === 'lintel-browser-host' && item.executable);
if (!artifact) throw new Error('Cargo did not report the browser host executable');
const cargoText = await readFile(sourceManifest, 'utf8');
const version = cargoText.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
if (!version) throw new Error('Native host package version is missing');
const bytes = await readFile(artifact.executable);
await mkdir(out, { recursive: true });
await copyFile(artifact.executable, path.join(out, 'lintel-browser-host'));
await chmod(path.join(out, 'lintel-browser-host'), 0o700);
await writeFile(path.join(out, 'manifest.json'), JSON.stringify({ schema: 1, version, platform, architecture, target, sha256: createHash('sha256').update(bytes).digest('hex'), bytes: bytes.length }, null, 2) + '\n');
console.log(`Prepared bundled browser host ${version} for ${target}.`);

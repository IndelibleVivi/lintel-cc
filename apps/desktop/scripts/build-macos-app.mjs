#!/usr/bin/env node
// The single macOS App build path. Native targets are accepted unchanged, but
// the final App is always gated and unsafe no-output/no-sign streams are refused.
import {cp, lstat, mkdir, mkdtemp, readFile, readdir, rm, writeFile} from 'node:fs/promises';
import {spawnSync} from 'node:child_process';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {verifyMacosBundle} from './verify-macos-bundle.mjs';

const appRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const baseConfig = path.join(appRoot, 'src-tauri/tauri.conf.json');
const platformConfig = path.join(appRoot, 'src-tauri/tauri.macos.conf.json');
const tauri = path.join(appRoot, 'node_modules/.bin/tauri');
const valueOptions = new Set([
  '-r', '--runner', '-t', '--target', '-f', '--features', '-b', '--bundles', '-c', '--config',
]);
const flagOptions = new Set([
  '-d', '--debug', '--ci', '--skip-stapling', '--ignore-version-mismatches',
  '--no-binary-patching', '-v', '--verbose',
]);
const helpOptions = new Set(['-h', '--help', '-V', '--version']);

function fail(message) { throw new Error(message); }

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {encoding: 'utf8', ...options});
  if (result.error) fail(`${command} failed to run: ${result.error.message}`);
  return result;
}

function parseArgs(args) {
  const parsed = {tauri: [], target: undefined, profile: 'release', configs: [], help: false, requireDeveloperId: false, bundles: undefined};
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--') {
      parsed.tauri.push(...args.slice(i));
      break;
    }
    if (arg === '--require-developer-id') {
      parsed.requireDeveloperId = true;
      continue;
    }
    if (helpOptions.has(arg)) {
      parsed.help = true;
      parsed.tauri.push(arg);
      continue;
    }
    if (arg === '--no-bundle' || arg === '--no-sign') {
      fail(`${arg} is unsafe for the signed canonical App build`);
    }
    if (valueOptions.has(arg)) {
      const value = args[++i];
      if (value === undefined || value.startsWith('-')) fail(`${arg} requires a value`);
      parsed.tauri.push(arg, value);
      if (arg === '-t' || arg === '--target') parsed.target = value;
      if (arg === '-c' || arg === '--config') parsed.configs.push(value);
      if (arg === '-b' || arg === '--bundles') parsed.bundles = value;
      continue;
    }
    const equals = arg.indexOf('=');
    const name = equals === -1 ? arg : arg.slice(0, equals);
    if (valueOptions.has(name)) {
      if (equals === -1) fail(`${name} requires a value`);
      const value = arg.slice(equals + 1);
      if (!value) fail(`${name} requires a value`);
      parsed.tauri.push(arg);
      if (name === '-t' || name === '--target') parsed.target = value;
      if (name === '-c' || name === '--config') parsed.configs.push(value);
      if (name === '-b' || name === '--bundles') parsed.bundles = value;
      continue;
    }
    if (flagOptions.has(arg)) {
      parsed.tauri.push(arg);
      if (arg === '-d' || arg === '--debug') parsed.profile = 'debug';
      continue;
    }
    // Tauri's Cargo runner arguments do not alter the bundle location.
    parsed.tauri.push(arg);
  }
  return parsed;
}

function withTauriOptions(args, options) {
  const separator = args.indexOf('--');
  return separator === -1 ? [...args, ...options]
    : [...args.slice(0, separator), ...options, ...args.slice(separator)];
}

function mergeObjects(base, override) {
  if (!override || typeof override !== 'object' || Array.isArray(override)) return base;
  const merged = {...base};
  for (const [key, value] of Object.entries(override)) {
    merged[key] = value && typeof value === 'object' && !Array.isArray(value)
      ? mergeObjects(base?.[key] || {}, value) : value;
  }
  return merged;
}

async function readJsonIfPresent(file) {
  try { return JSON.parse(await readFile(file, 'utf8')); }
  catch (error) { if (error.code === 'ENOENT') return null; fail(`Invalid Tauri config: ${file}`); }
}

export async function effectiveIdentity(args = []) {
  const parsed = parseArgs(args);
  let merged = await readJsonIfPresent(baseConfig) || {};
  const platform = await readJsonIfPresent(platformConfig);
  if (platform) merged = mergeObjects(merged, platform);
  for (const value of parsed.configs) {
    const inline = /^\s*[{\[]/.test(value);
    let override;
    try { override = JSON.parse(inline ? value : await readFile(path.resolve(appRoot, value), 'utf8')); }
    catch (error) {
      if (error.code === 'ENOENT') fail(`Tauri config override not found: ${value}`);
      return {identity: undefined, unknown: true};
    }
    merged = mergeObjects(merged, override);
  }
  return {identity: merged?.bundle?.macOS?.signingIdentity, unknown: false};
}

export async function targetRoot(env = process.env, root = appRoot) {
  const configured = env.CARGO_TARGET_DIR || path.join(root, 'src-tauri/target');
  return path.isAbsolute(configured) ? configured : path.resolve(appRoot, configured);
}

export async function buildArguments(extraArgs = [], {env = process.env, dryRun = false} = {}) {
  const parsed = parseArgs(extraArgs);
  if (parsed.help) return {args: ['build', ...parsed.tauri], fallback: null, output: null};
  const inspected = await effectiveIdentity(extraArgs);
  const configured = inspected.identity;
  const nonempty = typeof configured === 'string' && configured.trim() !== '';
  const invalid = configured !== undefined && configured !== null && typeof configured !== 'string';
  const formal = inspected.unknown || invalid || nonempty ||
    Boolean(env.APPLE_SIGNING_IDENTITY || env.APPLE_CERTIFICATE);
  if ((env.LINTEL_REQUIRE_FORMAL_SIGNING === '1' || parsed.requireDeveloperId) &&
      (!formal || (typeof configured === 'string' && configured.trim() === '-'))) {
    fail('A non-adhoc formal App signing identity is required for this build');
  }
  let fallback = null;
  if (!formal) {
    if (dryRun) {
      parsed.tauri = withTauriOptions(parsed.tauri, ['--config', '<local-adhoc-fallback>']);
      fallback = {output: '<local-adhoc-fallback>'};
    } else {
      const directory = await mkdtemp(path.join(os.tmpdir(), 'lintel-signing-'));
      const output = path.join(directory, 'tauri.local.json');
      await writeFile(output, JSON.stringify({bundle: {macOS: {signingIdentity: '-', hardenedRuntime: false}}}, null, 2) + '\n');
      parsed.tauri = withTauriOptions(parsed.tauri, ['--config', output]);
      fallback = {directory, output};
    }
  }
  const root = await targetRoot(env);
  const output = parsed.target
    ? path.join(root, parsed.target, parsed.profile, 'bundle/macos/Lintel.app')
    : path.join(root, parsed.profile, 'bundle/macos/Lintel.app');
  return {args: ['build', ...parsed.tauri], fallback, requireDeveloperId: parsed.requireDeveloperId, output: {target: parsed.target, profile: parsed.profile, app: output}};
}

export async function buildMacosApp(extraArgs = [], {
  guard = true, execute = run, env = process.env, verify = verifyMacosBundle, requireDeveloperId = false,
} = {}) {
  if (guard && process.platform !== 'darwin') fail('The macOS App build requires Darwin');
  const planned = await buildArguments(extraArgs, {env});
  try {
    const result = execute(tauri, planned.args, {cwd: appRoot, stdio: 'inherit'});
    if (result.status !== 0) fail(`Tauri App build failed (${result.status})`);
    if (!planned.output) return {app: null, verified: null};
    const verified = await verify(planned.output.app);
    if ((requireDeveloperId || planned.requireDeveloperId) && verified.signature !== 'developer-id') fail('Formal App build requires a Developer ID signature');
    return {app: planned.output.app, verified};
  } finally {
    if (planned.fallback?.directory) await rm(planned.fallback.directory, {recursive: true, force: true}).catch(() => {});
  }
}

export async function ensureNewDirectory(output) {
  if (!output || !path.isAbsolute(output)) fail('Usage: build-macos-app.mjs --out NEW_ABSOLUTE_DIRECTORY');
  try { await lstat(output); fail('Candidate output must not already exist'); }
  catch (error) { if (error.code !== 'ENOENT') throw error; }
}

export async function previewArguments(args = []) {
  const outputIndex = args.indexOf('--out');
  const output = outputIndex === -1 ? undefined : args[outputIndex + 1];
  if (outputIndex !== -1 && (!output || output.startsWith('-') || args.lastIndexOf('--out') !== outputIndex)) fail('--out requires one candidate directory');
  const tauri = outputIndex === -1 ? [...args] : args.filter((_, i) => i !== outputIndex && i !== outputIndex + 1);
  return {output, tauri};
}

export async function preview(output, args, {build = buildMacosApp, verify = verifyMacosBundle, quiet = false} = {}) {
  await ensureNewDirectory(output);
  const parsed = parseArgs(args);
  if (parsed.bundles === undefined) args = withTauriOptions(args, ['--bundles', 'app,dmg', '--ci']);
  else if (!parsed.bundles.split(',').includes('dmg')) fail('Preview requires both app and dmg bundles');
  const {app, verified} = await build(args);
  if (!app) fail('Preview requires an App build, not help/version');
  const dmgDir = path.join(path.dirname(app), '../dmg');
  if (typeof verified.version !== 'string') fail('Verified App version is required before selecting a DMG');
  const dmgs = (await readdir(dmgDir)).filter(name => name.startsWith(`Lintel_${verified.version}_`) && name.endsWith('.dmg'));
  if (dmgs.length !== 1) fail(`Expected exactly one DMG, found ${dmgs.length}`);
  await mkdir(output);
  await cp(app, path.join(output, 'Lintel.app'), {recursive: true, verbatimSymlinks: true});
  const copied = await verify(path.join(output, 'Lintel.app'));
  await writeFile(path.join(output, dmgs[0]), await readFile(path.join(dmgDir, dmgs[0])), {flag: 'wx'});
  const result = {output, app: path.join(output, 'Lintel.app'), dmg: path.join(output, dmgs[0]), app_verified: verified, copied_app_verified: copied, dmg_checked: false};
  if (!quiet) console.log(JSON.stringify(result));
  return result;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const {output, tauri: args} = await previewArguments(process.argv.slice(2));
  const promise = output ? preview(path.resolve(output), args) : buildMacosApp(args);
  promise.catch(error => { console.error(error.message); process.exitCode = 1; });
}

import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdir, mkdtemp, readFile, writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {buildArguments, buildMacosApp, effectiveIdentity, ensureNewDirectory, preview, previewArguments, targetRoot} from '../apps/desktop/scripts/build-macos-app.mjs';

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('preview accepts npm argv and forwards remaining Tauri options', async () => {
  const split = await previewArguments(['--bundles', 'app,dmg', '--ci', '--out', '/tmp/new-preview']);
  assert.equal(split.output, '/tmp/new-preview');
  assert.deepEqual(split.tauri, ['--bundles', 'app,dmg', '--ci']);
  const planned = await buildArguments(split.tauri, {env: {}, dryRun: true});
  assert.deepEqual(planned.args.slice(0, 5), ['build', '--bundles', 'app,dmg', '--ci', '--config']);
  await assert.rejects(() => previewArguments(['--out']), /requires one candidate directory/);
});

test('generated fallback config is valid JSON and uses signingIdentity -', async () => {
  const planned = await buildArguments([], {env: {}});
  assert.ok(planned.fallback.directory);
  assert.deepEqual(JSON.parse(await readFile(planned.fallback.output, 'utf8')), {bundle: {macOS: {signingIdentity: '-', hardenedRuntime: false}}});
  await import('node:fs/promises').then(({rm}) => rm(planned.fallback.directory, {recursive: true, force: true}));
});

test('ordinary App build retains the configured bundle targets', async () => {
  const planned = await buildArguments([], {env: {}, dryRun: true});
  assert.deepEqual(planned.args, ['build', '--config', '<local-adhoc-fallback>']);
});

test('preview rejects a bundle selection without dmg before building', async () => {
  const directory = await mkdtemp(path.join(tmpdir(), 'lintel-preview-bundles-'));
  let built = false;
  await assert.rejects(
    () => preview(path.join(directory, 'out'), ['--bundles', 'app'], {
      build: async () => { built = true; return {app: '/unused/Lintel.app', verified: {}}; },
    }),
    /requires both app and dmg/,
  );
  assert.equal(built, false);
});

test('preview reads app and dmg from the planned target/profile directory', async () => {
  const directory = await mkdtemp(path.join(tmpdir(), 'lintel-preview-layout-'));
  const app = path.join(directory, 'target/aarch64-apple-darwin/debug/bundle/macos/Lintel.app');
  const dmgDir = path.join(directory, 'target/aarch64-apple-darwin/debug/bundle/dmg');
  await mkdir(app, {recursive: true});
  await mkdir(dmgDir, {recursive: true});
  await writeFile(path.join(app, 'marker'), 'app\n');
  await writeFile(path.join(dmgDir, 'Lintel_0.1.1_aarch64.dmg'), 'synthetic dmg\n');
  await writeFile(path.join(dmgDir, 'Lintel_0.1.0_aarch64.dmg'), 'preserve old candidate\n');
  const output = path.join(directory, 'candidate');
  const result = await preview(output, ['--target', 'aarch64-apple-darwin', '--debug'], {
    build: async () => ({app, verified: {signature: 'ad-hoc', version: '0.1.1'}}),
    verify: async () => ({signature: 'ad-hoc', version: '0.1.1'}),
    quiet: true,
  });
  assert.equal(result.dmg_checked, false);
  assert.equal(path.basename(result.dmg), 'Lintel_0.1.1_aarch64.dmg');
  assert.equal(await import('node:fs/promises').then(({readFile}) => readFile(path.join(output, 'Lintel.app/marker'), 'utf8')), 'app\n');
});

test('target, debug, aliases and relative CARGO_TARGET_DIR select the real App path', async () => {
  assert.equal(await targetRoot({CARGO_TARGET_DIR: 'cache'}), path.resolve(repo, 'apps/desktop/cache'));
  const target = await buildArguments(['-t', 'aarch64-apple-darwin', '-d', '-c', '{"bundle":{"macOS":{"signingIdentity":"Developer ID Application: Test"}}}'], {env: {}, dryRun: true});
  assert.equal(target.output.profile, 'debug');
  assert.equal(target.output.target, 'aarch64-apple-darwin');
  assert.equal(target.output.app, path.resolve(repo, 'apps/desktop/src-tauri/target/aarch64-apple-darwin/debug/bundle/macos/Lintel.app'));
  const release = await buildArguments(['--target=x86_64-apple-darwin'], {env: {}, dryRun: true});
  assert.equal(release.output.app, path.resolve(repo, 'apps/desktop/src-tauri/target/x86_64-apple-darwin/release/bundle/macos/Lintel.app'));
});

test('unsafe and incomplete options are rejected before Tauri', async () => {
  for (const arg of ['--no-bundle', '--no-sign']) {
    await assert.rejects(() => buildArguments([arg], {dryRun: true}), /unsafe/);
  }
  for (const arg of ['-t', '-c', '-c=', '--config=', '--bundles=']) {
    await assert.rejects(() => buildArguments([arg], {dryRun: true}), /requires a value/);
  }
});

test('runner arguments after -- do not change target/profile selection', async () => {
  const planned = await buildArguments(['--target', 'aarch64-apple-darwin', '--', '--debug', '--target', 'not-a-tauri-target'], {env: {}, dryRun: true});
  assert.equal(planned.output.target, 'aarch64-apple-darwin');
  assert.equal(planned.output.profile, 'release');
  assert.deepEqual(planned.args, ['build', '--target', 'aarch64-apple-darwin', '--config', '<local-adhoc-fallback>', '--', '--debug', '--target', 'not-a-tauri-target']);
});

test('help/version pass through without inspecting or gating an App', async () => {
  for (const arg of ['--help', '-h', '--version', '-V']) {
    const planned = await buildArguments([arg], {env: {}, dryRun: true});
    assert.equal(planned.output, null);
    assert.equal(planned.fallback, null);
    assert.deepEqual(planned.args, ['build', arg]);
  }
});

test('formal release rejects adhoc and accepts only developer-id gate output', async () => {
  await assert.rejects(
    () => buildArguments(['--config', '{"bundle":{"macOS":{"signingIdentity":"-"}}}', '--require-developer-id'], {env: {}, dryRun: true}),
    /non-adhoc formal/,
  );
  const envIdentity = await buildArguments(['--require-developer-id'], {env: {APPLE_SIGNING_IDENTITY: 'Developer ID Application: CI'}, dryRun: true});
  assert.equal(envIdentity.requireDeveloperId, true);
  const calls = [];
  const checked = [];
  await buildMacosApp(['--config', '{"bundle":{"macOS":{"signingIdentity":"Developer ID Application: Test"}}}', '--require-developer-id'], {
    guard: false,
    execute: (command, args) => { calls.push({command, args}); return {status: 0}; },
    verify: async app => { checked.push(app); return {signature: 'developer-id'}; },
  });
  assert.equal(calls.length, 1);
  assert.equal(checked.length, 1);
  assert.equal(calls[0].args.includes('--require-developer-id'), false);
  await assert.rejects(() => buildMacosApp(['--config', '{"bundle":{"macOS":{"signingIdentity":"Developer ID Application: Test"}}}', '--require-developer-id'], {
    guard: false,
    execute: () => ({status: 0}),
    verify: async () => ({signature: 'ad-hoc'}),
  }), /requires a Developer ID signature/);
});

test('verifier injection proves exact target path and gate propagation', async () => {
  const calls = [];
  const checked = [];
  await assert.rejects(() => buildMacosApp(['--target', 'aarch64-apple-darwin', '--debug'], {
    guard: false,
    execute: (command, args) => { calls.push({command, args}); return {status: 0}; },
    verify: async app => { checked.push(app); throw new Error('synthetic gate failure'); },
  }), /synthetic gate failure/);
  assert.equal(calls.length, 1);
  assert.deepEqual(checked, [path.resolve(repo, 'apps/desktop/src-tauri/target/aarch64-apple-darwin/debug/bundle/macos/Lintel.app')]);
});

test('explicit config priority is respected and missing config fails', async () => {
  const directory = await mkdtemp(path.join(tmpdir(), 'lintel-build-priority-'));
  const override = path.join(directory, 'release.json');
  await writeFile(override, JSON.stringify({bundle: {macOS: {signingIdentity: 'Developer ID Application: Release'}}}));
  const inspected = await effectiveIdentity(['-c', override]);
  assert.equal(inspected.identity, 'Developer ID Application: Release');
  assert.equal((await buildArguments(['-c', override], {env: {}, dryRun: true})).fallback, null);
  await assert.rejects(() => effectiveIdentity(['--config', path.join(directory, 'absent.json')]), /not found/);
});

test('preview output is new and never overwritten', async () => {
  const directory = await mkdtemp(path.join(tmpdir(), 'lintel-build-output-'));
  await ensureNewDirectory(path.join(directory, 'new'));
  await writeFile(path.join(directory, 'occupied'), 'keep\n');
  await assert.rejects(() => ensureNewDirectory(path.join(directory, 'occupied')), /must not already exist/);
  assert.equal(await import('node:fs/promises').then(({readFile}) => readFile(path.join(directory, 'occupied'), 'utf8')), 'keep\n');
});

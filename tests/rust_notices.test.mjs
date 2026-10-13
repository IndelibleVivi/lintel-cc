import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import os from 'node:os';
import path from 'node:path';
import { collectNativeRuntime, collectNotices, noticeKind, reachablePackages, renderMarkdown } from '../scripts/prepare-rust-notices.mjs';

const registry = 'registry+https://github.com/rust-lang/crates.io-index';
const terms = 'Permission is hereby granted. Synthetic license text for temporary tests only.\n';

async function fixture(t) {
  const root = await mkdtemp(path.join(os.tmpdir(), 'lintel-notices-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const source = path.join(root, 'crate');
  const supplementRoot = path.join(root, 'supplements');
  await mkdir(source);
  await mkdir(supplementRoot);
  const pkg = { id: 'crate-id', name: 'example', version: '1.0.0', source: registry, license: 'MIT', license_file: null, repository: 'https://example.test/upstream', manifest_path: path.join(source, 'Cargo.toml') };
  const own = { id: 'root-id', name: 'lintel-desktop', version: '0.1.0', source: null, manifest_path: path.join(root, 'Cargo.toml') };
  const metadata = { packages: [own, pkg], workspace_members: [own.id], resolve: { nodes: [{ id: own.id, deps: [{ pkg: pkg.id, dep_kinds: [{ kind: null }] }] }, { id: pkg.id, deps: [] }] } };
  const input = { profiles: [{ name: 'desktop', package: own.name, metadata }], supplements: { schema: 1, packages: [] }, supplementRoot, projectRoot: root };
  return { root, source, supplementRoot, pkg, metadata, input };
}

test('includes normal/build closure and excludes dev-only packages', async t => {
  const { metadata } = await fixture(t);
  metadata.packages.push({ id: 'build-id', name: 'build-example' }, { id: 'dev-id', name: 'dev-example' });
  metadata.resolve.nodes[0].deps.push({ pkg: 'build-id', dep_kinds: [{ kind: 'build' }] }, { pkg: 'dev-id', dep_kinds: [{ kind: 'dev' }] });
  metadata.resolve.nodes.push({ id: 'build-id', deps: [] }, { id: 'dev-id', deps: [] });
  assert.deepEqual(reachablePackages(metadata, 'lintel-desktop').map(pkg => pkg.id), ['root-id', 'crate-id', 'build-id']);
});

test('retains nested license directories and NOTICE, without treating copying.rs as a license', async t => {
  const { source, input } = await fixture(t);
  await mkdir(path.join(source, 'vendor/LICENSES'), { recursive: true });
  await mkdir(path.join(source, 'src'));
  await writeFile(path.join(source, 'vendor/LICENSES/MIT.txt'), terms);
  await writeFile(path.join(source, 'vendor/NOTICE'), 'Copyright Synthetic Contributor\n');
  await writeFile(path.join(source, 'src/copying.rs'), '// no legal text\n');
  const [pkg] = await collectNotices(input);
  assert.deepEqual(pkg.documents.map(document => document.file), ['vendor/LICENSES/MIT.txt', 'vendor/NOTICE']);
  assert.equal(noticeKind('src/copying.rs'), null);
  assert.equal(noticeKind('LICENSE.spdx'), 'declaration');
});

test('a NOTICE or SPDX declaration alone cannot satisfy missing license text', async t => {
  const { source, input } = await fixture(t);
  await writeFile(path.join(source, 'NOTICE'), 'Copyright Synthetic Contributor\n');
  await writeFile(path.join(source, 'LICENSE.spdx'), 'PackageLicenseDeclared: MIT\n');
  await assert.rejects(collectNotices(input), /example@1\.0\.0: missing license text/);
});

test('an explicit license-file is included and must exist even when another license exists', async t => {
  const { source, pkg, input } = await fixture(t);
  pkg.license_file = 'TERMS.txt';
  await writeFile(path.join(source, 'LICENSE'), terms);
  await assert.rejects(collectNotices(input), /example@1\.0\.0/);
  await writeFile(path.join(source, 'TERMS.txt'), terms);
  const [result] = await collectNotices(input);
  assert.deepEqual(result.documents.map(document => document.file), ['LICENSE', 'TERMS.txt']);
});

test('an empty notice fails, and a license-file cannot escape its source root', async t => {
  const { source, pkg, input } = await fixture(t);
  await writeFile(path.join(source, 'LICENSE'), terms);
  await writeFile(path.join(source, 'NOTICE'), '');
  await assert.rejects(collectNotices(input), /notice is empty/);
  await writeFile(path.join(source, 'NOTICE'), 'Synthetic retained notice\n');
  pkg.license_file = '../outside.txt';
  await assert.rejects(collectNotices(input), /notice path escapes/);
});

test('supplements bind version, upstream commit, declaration and exact text bytes', async t => {
  const { source, pkg, input, supplementRoot } = await fixture(t);
  await writeFile(path.join(source, '.cargo_vcs_info.json'), JSON.stringify({ git: { sha1: 'a'.repeat(40) } }));
  await writeFile(path.join(supplementRoot, 'license.txt'), terms);
  input.supplements.packages.push({ name: pkg.name, version: pkg.version, license: pkg.license, repository: pkg.repository, upstream_commit: 'a'.repeat(40), reason: 'Synthetic version-bound evidence', files: [{ file: 'license.txt', source: 'https://example.test/upstream/license', kind: 'license', sha256: createHash('sha256').update(terms).digest('hex') }] });
  assert.equal((await collectNotices(input)).length, 1);
  await writeFile(path.join(supplementRoot, 'license.txt'), `${terms}altered`);
  await assert.rejects(collectNotices(input), /supplement integrity mismatch/);
  await writeFile(path.join(supplementRoot, 'license.txt'), terms);
  pkg.license = 'BSD-3-Clause';
  await assert.rejects(collectNotices(input), /does not match this exact upstream package/);
  pkg.license = 'MIT';
  await writeFile(path.join(source, '.cargo_vcs_info.json'), JSON.stringify({ git: { sha1: 'b'.repeat(40) } }));
  await assert.rejects(collectNotices(input), /does not match this exact upstream package/);
});

test('rendered notices retain original expressions, exact-version source links and no local paths', async t => {
  const { root, source, pkg, input } = await fixture(t);
  pkg.license = 'MPL-2.0';
  await writeFile(path.join(source, 'LICENSE'), terms);
  const packages = await collectNotices(input);
  const markdown = renderMarkdown(packages, [{ name: 'desktop', package: 'lintel-desktop', target: 'aarch64-apple-darwin', manifest: 'apps/desktop/src-tauri/Cargo.toml' }], { release: 'synthetic', commit: 'test' });
  assert.match(markdown, /not an exact linked SBOM/);
  assert.match(markdown, /Declared license: MPL-2\.0/);
  assert.match(markdown, /https:\/\/crates.io\/api\/v1\/crates\/example\/1\.0\.0\/download/);
  assert.ok(!markdown.includes(root));
  assert.ok(!JSON.stringify(packages).includes(root));
});

test('native toolchain notices require the reviewed version and retained text', async t => {
  const { supplementRoot } = await fixture(t);
  const manifest = { schema: 1, toolchain: 'ziglang', version: '0.16.0', files: [{ file: 'native.md', sha256: createHash('sha256').update(terms).digest('hex') }] };
  await writeFile(path.join(supplementRoot, 'native-runtime.json'), JSON.stringify(manifest));
  await assert.rejects(collectNativeRuntime(supplementRoot), /ENOENT/);
  await writeFile(path.join(supplementRoot, 'native.md'), terms);
  assert.equal((await collectNativeRuntime(supplementRoot)).text, terms);
  await writeFile(path.join(supplementRoot, 'native.md'), `${terms}changed`);
  await assert.rejects(collectNativeRuntime(supplementRoot), /integrity mismatch/);
  manifest.version = 'future';
  await writeFile(path.join(supplementRoot, 'native-runtime.json'), JSON.stringify(manifest));
  await assert.rejects(collectNativeRuntime(supplementRoot), /Unreviewed native runtime/);
});

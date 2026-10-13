#!/usr/bin/env node
// Offline Cargo notices for the desktop, native host and bundled Linux runners.
// This is deliberately a normal/build dependency superset, not a linked SBOM.
import { readFile, readdir, realpath, mkdir, writeFile, rename, stat } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = fileURLToPath(new URL('../', import.meta.url));
const outputRelative = 'apps/desktop/src-tauri/third-party-notices';
const supplementRelative = 'contracts/third-party-notices';
const ownPackages = new Set(['lintel-desktop', 'lintel-browser-host', 'lintel-runner', 'lintel-core', 'lintel-remote', 'lintel-egress', 'lintel-operations']);
const textLimit = 4 * 1024 * 1024;
const lexical = (a, b) => a < b ? -1 : a > b ? 1 : 0;

function within(root, filename) {
  const relative = path.relative(root, filename);
  return relative !== '..' && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);
}

async function textFile(root, relative, label) {
  const filename = path.resolve(root, relative);
  if (!within(root, filename) || !within(await realpath(root), await realpath(filename))) {
    throw new Error(`${label}: notice path escapes its source directory`);
  }
  const info = await stat(filename);
  if (!info.isFile() || info.size === 0 || info.size > textLimit) throw new Error(`${label}: notice is empty, not a file, or exceeds 4 MiB`);
  const bytes = await readFile(filename);
  let text;
  try { text = new TextDecoder('utf-8', { fatal: true }).decode(bytes); }
  catch { throw new Error(`${label}: notice is not UTF-8 text`); }
  if (!text.trim() || text.includes('\0')) throw new Error(`${label}: notice is empty or binary`);
  return { text, bytes };
}

// Do not mistake objc2's copying.rs (or other source code) for COPYING.
export function noticeKind(relative) {
  const name = path.basename(relative);
  if (/\.(rs|c|h|cpp|py|js|toml|json)$/i.test(name)) return null;
  if (/^licen[cs]es?(?:[._-]|$)/i.test(name)) return /\.spdx$/i.test(name) ? 'declaration' : 'license';
  if (/^copying(?:[._-]|$)/i.test(name)) return 'license';
  if (/^(notice|copyright|authors)(?:[._-]|$)/i.test(name)) return 'notice';
  if (relative.split(/[\\/]/).slice(0, -1).some(part => /^licen[cs]es?$/i.test(part))) return 'license';
  return null;
}

async function noticePaths(root, relative = '') {
  const found = [];
  for (const entry of await readdir(path.join(root, relative), { withFileTypes: true })) {
    const child = path.join(relative, entry.name);
    if (entry.isDirectory()) found.push(...await noticePaths(root, child));
    else if (noticeKind(child)) found.push(child);
  }
  return found.sort(lexical);
}

export function reachablePackages(metadata, rootName) {
  if (!metadata.resolve?.nodes) throw new Error(`${rootName}: cargo metadata has no resolved dependency graph`);
  const roots = metadata.packages.filter(pkg => pkg.name === rootName && metadata.workspace_members.includes(pkg.id));
  if (roots.length !== 1) throw new Error(`${rootName}: expected exactly one workspace root package`);
  const nodes = new Map(metadata.resolve.nodes.map(node => [node.id, node]));
  const visited = new Set();
  function visit(id) {
    if (visited.has(id)) return;
    visited.add(id);
    const node = nodes.get(id);
    if (!node) throw new Error(`${rootName}: unresolved dependency node`);
    for (const dependency of node.deps) {
      if (dependency.dep_kinds.some(kind => kind.kind === null || kind.kind === 'normal' || kind.kind === 'build')) visit(dependency.pkg);
    }
  }
  visit(roots[0].id);
  const packages = metadata.packages.filter(pkg => visited.has(pkg.id));
  if (packages.length !== visited.size) throw new Error(`${rootName}: resolved package is missing metadata`);
  return packages;
}

export async function collectNotices({ profiles, supplements, supplementRoot, projectRoot }) {
  if (supplements.schema !== 1 || !Array.isArray(supplements.packages)) throw new Error('Unsupported Rust notice supplement schema');
  const overrides = new Map();
  for (const entry of supplements.packages) {
    const key = `${entry.name}@${entry.version}`;
    if (overrides.has(key)) throw new Error(`Duplicate Rust notice supplement: ${key}`);
    overrides.set(key, entry);
  }
  const all = new Map();
  for (const profile of profiles) {
    for (const pkg of reachablePackages(profile.metadata, profile.package)) {
      if (pkg.source === null) {
        if (!ownPackages.has(pkg.name) || !within(projectRoot, pkg.manifest_path)) throw new Error(`${pkg.name}@${pkg.version}: unreviewed local dependency`);
        continue;
      }
      if (pkg.source !== 'registry+https://github.com/rust-lang/crates.io-index') throw new Error(`${pkg.name}@${pkg.version}: unreviewed non-crates.io source`);
      const key = `${pkg.name}@${pkg.version}`;
      let item = all.get(key);
      if (!item) { item = { pkg, profiles: new Set() }; all.set(key, item); }
      if (item.pkg.license !== pkg.license || item.pkg.repository !== pkg.repository) throw new Error(`${key}: inconsistent package metadata`);
      item.profiles.add(profile.name);
    }
  }
  const packages = [];
  const issues = [];
  for (const [key, { pkg, profiles: includedProfiles }] of [...all].sort(([a], [b]) => lexical(a, b))) {
    try {
      const sourceRoot = path.dirname(pkg.manifest_path);
      const paths = new Set(await noticePaths(sourceRoot));
      if (pkg.license_file) paths.add(path.relative(sourceRoot, path.resolve(sourceRoot, pkg.license_file)));
      const documents = [];
      for (const relative of [...paths].sort(lexical)) {
        const { text } = await textFile(sourceRoot, relative, `${key}/${relative}`);
        documents.push({ file: relative.split(path.sep).join('/'), kind: relative === pkg.license_file ? 'license' : noticeKind(relative) || 'license', source: `https://docs.rs/crate/${pkg.name}/${pkg.version}/source/${relative.split(path.sep).join('/')}`, text });
      }
      const extra = overrides.get(key);
      if (extra) {
        const vcs = JSON.parse(await readFile(path.join(sourceRoot, '.cargo_vcs_info.json'), 'utf8'));
        if (extra.license !== pkg.license || extra.repository !== pkg.repository || extra.upstream_commit !== vcs.git?.sha1) throw new Error('supplement does not match this exact upstream package');
        for (const file of extra.files) {
          if (!['license', 'notice', 'declaration'].includes(file.kind) || !file.source.startsWith('https://')) throw new Error('invalid supplement source or document kind');
          const { text, bytes } = await textFile(supplementRoot, file.file, `${key} supplement ${file.file}`);
          if (createHash('sha256').update(bytes).digest('hex') !== file.sha256) throw new Error(`supplement integrity mismatch: ${file.file}`);
          documents.push({ file: file.file, kind: file.kind, source: file.source, ...(file.excerpt ? { excerpt: file.excerpt } : {}), text });
        }
      }
      if (!pkg.license && !pkg.license_file) throw new Error('no declared license or license-file');
      if (!documents.some(document => document.kind === 'license')) throw new Error('missing license text; add reviewed version-bound upstream evidence');
      packages.push({ name: pkg.name, version: pkg.version, license: pkg.license, ...(extra?.selected_license ? { selected_license: extra.selected_license } : {}), repository: pkg.repository, source_archive: `https://crates.io/api/v1/crates/${pkg.name}/${pkg.version}/download`, profiles: [...includedProfiles].sort(lexical), ...(extra ? { supplemental_evidence: extra.reason, upstream_commit: extra.upstream_commit } : {}), documents });
    } catch (error) { issues.push(`${key}: ${error.message}`); }
  }
  if (issues.length) throw new Error(`Rust notices incomplete (${issues.length} package(s)):\n${issues.join('\n')}`);
  return packages;
}

function command(program, args) {
  return execFileSync(program, args, { cwd: repoRoot, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 });
}

export async function collectFrontendNotice(frontendDist) {
  return (await textFile(frontendDist, 'THIRD_PARTY_FRONTEND.md', 'Frontend distribution notice')).text;
}

export async function collectNativeRuntime(supplementRoot) {
  const manifest = JSON.parse(await readFile(path.join(supplementRoot, 'native-runtime.json'), 'utf8'));
  if (manifest.schema !== 1 || manifest.toolchain !== 'ziglang' || manifest.version !== '0.16.0' || manifest.files?.length !== 1) throw new Error('Unreviewed native runtime notice profile');
  const file = manifest.files[0];
  const { text, bytes } = await textFile(supplementRoot, file.file, 'Native runtime notice');
  if (createHash('sha256').update(bytes).digest('hex') !== file.sha256) throw new Error('Native runtime notice integrity mismatch');
  return { manifest, text };
}

export function renderMarkdown(packages, profiles, toolchain, nativeRuntime) {
  const lines = ['# Rust third-party notices', '',
    'This distribution includes third-party Rust software. The following is a conservative, target-filtered Cargo normal/build dependency collection, including build tools and procedural macros. It is not an exact linked SBOM and does not assert that every listed crate is present in a shipped executable.', '',
    'Cargo resolves each manifest with its locked default features, offline. Dev-only edges and dependencies for other target platforms are excluded; feature unification can still retain a superset. Original license expressions are preserved. Where upstream offers alternatives, any explicit choice below applies only to the named upstream component, never to Lintel original material.', '',
    'Exact-version source archives are linked for every crate, including MPL-covered components. Those source files remain available under their upstream terms; Lintel terms do not restrict those rights.', '',
    '## Included build profiles', '', ...profiles.map(profile => `- ${profile.name}: ${profile.package}, ${profile.target}, ${profile.manifest}`), '',
    '## Rust standard library', '',
    `Current preparation toolchain: ${toolchain.release} (${toolchain.commit}). The unmodified [Rust standard library copyright notice](rust-standard-library.html) and its [license texts](licenses/) accompany this file. This toolchain-provided notice is also a superset, not evidence of which compiler/runtime produced a previously built runner. Reused binaries require their original toolchain notices.`, ''];
  if (nativeRuntime) lines.push('## Native runtime', '', `The [native runtime notice snapshot](rust-native-runtime.md) retains the Zig ${nativeRuntime.version} distribution's musl, compiler runtime, unwind and supporting source notices. This is the reviewed native toolchain profile for the Linux runners. Notice generation does not inspect a binary's compiler provenance; a different native toolchain requires its corresponding notices.`, '');
  lines.push('## Crates', '');
  for (const pkg of packages) {
    lines.push(`### ${pkg.name} ${pkg.version}`, '', `Declared license: ${pkg.license || 'see license-file'}`, '', `Build profiles: ${pkg.profiles.join(', ')}`, '', `[Exact-version source archive](${pkg.source_archive})`, '');
    if (pkg.repository) lines.push(`Upstream: ${pkg.repository}`, '');
    if (pkg.selected_license) lines.push(`Selected offered license: ${pkg.selected_license}`, '');
    if (pkg.supplemental_evidence) lines.push(pkg.supplemental_evidence, '');
    for (const document of pkg.documents) {
      const fence = '`'.repeat(Math.max(3, ...[...document.text.matchAll(/`+/g)].map(match => match[0].length + 1)));
      lines.push(`#### ${document.file}`, '', `Source: ${document.source}`, '');
      if (document.excerpt) lines.push(document.excerpt, '');
      lines.push(`${fence}text`, document.text.trimEnd(), fence, '');
    }
  }
  return lines.join('\n');
}

export async function prepare() {
  const args = process.argv.slice(2);
  if (args.length && (args.length !== 2 || args[0] !== '--output')) throw new Error('Usage: node scripts/prepare-rust-notices.mjs [--output DIRECTORY]');
  const output = args.length ? path.resolve(args[1]) : path.join(repoRoot, outputRelative);
  const version = command('rustc', ['-vV']);
  const host = version.match(/^host: (.+)$/m)?.[1];
  const target = process.env.TAURI_ENV_TARGET_TRIPLE || host;
  if (!target || !/^(aarch64|x86_64)-(apple-darwin|unknown-linux-(gnu|musl))$/.test(target)) throw new Error('Unsupported desktop target for Rust notices');
  const toolchain = { release: version.match(/^release: (.+)$/m)?.[1], commit: version.match(/^commit-hash: (.+)$/m)?.[1], host };
  if (!toolchain.release || !toolchain.commit) throw new Error('rustc did not identify its toolchain');
  const profiles = [
    { name: 'desktop', package: 'lintel-desktop', manifest: 'apps/desktop/src-tauri/Cargo.toml', target },
    { name: 'browser-native-host', package: 'lintel-browser-host', manifest: 'extensions/browser/native-host/Cargo.toml', target },
    ...['x86_64-unknown-linux-musl', 'aarch64-unknown-linux-musl'].map(target => ({ name: `runner-${target}`, package: 'lintel-runner', manifest: 'Cargo.toml', target })),
  ];
  for (const profile of profiles) profile.metadata = JSON.parse(command('cargo', ['metadata', '--format-version', '1', '--locked', '--offline', '--manifest-path', profile.manifest, '--filter-platform', profile.target]));
  const supplementRoot = path.join(repoRoot, supplementRelative);
  const supplements = JSON.parse(await readFile(path.join(supplementRoot, 'rust-supplements.json'), 'utf8'));
  const packages = await collectNotices({ profiles, supplements, supplementRoot, projectRoot: repoRoot });
  const nativeRuntime = await collectNativeRuntime(supplementRoot);
  const publicProfiles = profiles.map(({ metadata, ...profile }) => profile);
  const files = new Map();
  // Vite owns the source text. Gather it with the native notices only after the
  // frontend build, so a missing/empty source cannot produce a partial bundle.
  files.set('frontend.md', await collectFrontendNotice(path.join(repoRoot, 'apps/desktop/dist')));
  const sysroot = command('rustc', ['--print', 'sysroot']).trim();
  const rustDocs = path.join(sysroot, 'share/doc/rust');
  // The distributed rust-docs component owns these exact version-specific texts.
  // Missing docs fail preparation instead of silently claiming runtime coverage.
  files.set('rust-standard-library.html', (await textFile(rustDocs, 'COPYRIGHT-library.html', 'Rust standard library copyright notice')).text);
  for (const name of (await readdir(path.join(rustDocs, 'licenses'))).sort(lexical)) {
    if (!name.endsWith('.txt')) continue;
    files.set(`licenses/${name}`, (await textFile(rustDocs, `licenses/${name}`, `Rust standard library ${name}`)).text);
  }
  if (!files.has('licenses/MIT.txt') || !files.has('licenses/Apache-2.0.txt')) throw new Error('Rust standard library license texts are incomplete');
  files.set('rust-native-runtime.md', nativeRuntime.text);
  files.set('rust.md', renderMarkdown(packages, publicProfiles, toolchain, nativeRuntime.manifest));
  const inventory = { schema: 1, coverage: 'target-filtered Cargo normal/build dependency superset; not an exact linked SBOM', profiles: publicProfiles, standard_library: { ...toolchain, notice: 'rust-standard-library.html', licenses: [...files.keys()].filter(file => file.startsWith('licenses/')), coverage: 'Current preparation toolchain notice; reused binaries require their original build toolchain notice.' }, native_runtime: { ...nativeRuntime.manifest, notice: 'rust-native-runtime.md' }, package_count: packages.length, packages: packages.map(({ documents, ...pkg }) => ({ ...pkg, documents: documents.map(({ text, ...document }) => document) })) };
  files.set('inventory.json', `${JSON.stringify(inventory, null, 2)}\n`);
  // Gather and validate every source before touching existing generated resources.
  for (const [relative, contents] of files) {
    if (contents.includes(repoRoot) || contents.includes(sysroot) || /(?:\/Users\/|\/home\/)[^\s<]+/.test(contents)) throw new Error(`${relative}: local absolute path in public notices`);
  }
  for (const [relative, contents] of files) {
    const destination = path.join(output, relative);
    await mkdir(path.dirname(destination), { recursive: true });
    const temporary = `${destination}.tmp-${process.pid}`;
    await writeFile(temporary, contents);
    await rename(temporary, destination);
  }
  console.log(`Prepared Rust notices for ${packages.length} third-party crates across ${profiles.length} build profiles, plus current-toolchain standard-library notices (dependency superset, not a linked SBOM).`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  prepare().catch(error => { console.error(error.message); process.exitCode = 1; });
}

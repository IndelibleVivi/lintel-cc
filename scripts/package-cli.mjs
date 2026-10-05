#!/usr/bin/env node
// Canonical packager for portable, independently installable Lintel CLI
// candidate archives. It never compiles, downloads, signs or resolves a remote
// host: every binary input is supplied explicitly by the caller.
//
// Usage (from the repository root, after the canonical builds exist):
//   node scripts/package-cli.mjs \
//     --macos-arm64 target/release/lintel \
//     --linux-x86_64 target/x86_64-unknown-linux-musl/release/lintel \
//     --linux-aarch64 target/aarch64-unknown-linux-musl/release/lintel \
//     --linux-runners apps/desktop/src-tauri/runner-bundles \
//     --revision "$(git rev-parse HEAD)" \
//     --out candidate-packages
//
// Inputs are the three CLI executables (one macOS arm64 Mach-O and two static
// Linux musl ELF) plus the canonical `remote-runners` directory the CLI itself
// resolves next to its executable. Each Linux input must be byte-identical to
// that target's canonical runner, so a stale or mixed set cannot be packaged as
// one candidate. Missing, wrong-architecture or dynamically-linked Linux inputs
// are rejected specifically. Packaging is an unsigned candidate artifact, not a
// signed or formal release.
//
// Archive layout (flat, exactly what the CLI resolves at runtime):
//   bin/lintel                          selected CLI executable
//   bin/remote-runners/manifest.json    canonical runner manifest
//   bin/remote-runners/<triple>/lintel  both static Linux runners
//   candidate.json                      truthful candidate metadata
//   SHA256SUMS                          shasum/sha256sum -c verifiable digests
//   README.txt                          install/upgrade instructions
//
// The output directory is created only after every input validates. Archives
// and the identity-specific index are published with true no-replace semantics,
// so a previous candidate is never overwritten. The repository-relative
// `candidate-packages/` output directory is ignored by Git; a caller-selected
// temporary directory is equally acceptable.
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { link, lstat, mkdir, mkdtemp, open, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

// --- canonical identities -----------------------------------------------------
// Product/protocol/catalog identity is a fixed contract the caller checks with
// `lintel version --json` and `lintel capabilities --json`.
const PRODUCT = 'Lintel';
const PROTOCOL = 1;
const CATALOG_VERSION = 1;
// crates/remote/src/remote_install.rs caps each uploaded runner at 32 MiB.
const MAX_RUNNER_BYTES = 32 * 1024 * 1024;
const VERSION_PATTERN = /^[0-9][0-9A-Za-z.\-+]*$/;

const TARGETS = {
  'macos-arm64': { platform: 'macos', architecture: 'aarch64', triple: 'aarch64-apple-darwin' },
  'linux-x86_64': { platform: 'linux', architecture: 'x86_64', triple: 'x86_64-unknown-linux-musl' },
  'linux-aarch64': { platform: 'linux', architecture: 'aarch64', triple: 'aarch64-unknown-linux-musl' },
};
const LINUX_TARGETS = {
  'linux-x86_64': 'x86_64-unknown-linux-musl',
  'linux-aarch64': 'aarch64-unknown-linux-musl',
};

function fail(code, message) {
  const error = new Error(message);
  error.code = code;
  throw error;
}

function parseArgs(argv) {
  const flags = new Map();
  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i];
    if (!token.startsWith('--')) fail('invalid_argument', `unexpected argument: ${token}`);
    const name = token.slice(2);
    if (name === 'help') {
      flags.set('help', 'true');
      continue;
    }
    const value = argv[i + 1];
    if (value === undefined || value.startsWith('--')) fail('invalid_argument', `--${name} needs a value`);
    flags.set(name, value);
    i += 1;
  }
  return flags;
}

function required(flags, name) {
  const value = flags.get(name);
  if (!value) fail('invalid_argument', `--${name} is required`);
  return path.resolve(value);
}

const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

// --- binary format checks -----------------------------------------------------
// Mach-O arm64: complete mach_header_64, CPU_TYPE_ARM64 and MH_EXECUTE.
function verifyMachOArm64(bytes, label) {
  if (bytes.length < 32 || bytes.readUInt32LE(0) !== 0xfeedfacf) {
    fail('wrong_format', `${label}: not a 64-bit little-endian Mach-O executable`);
  }
  if (bytes.readUInt32LE(4) !== 0x0100000c) {
    fail('wrong_architecture', `${label}: Mach-O is not arm64 (cputype=0x${bytes.readUInt32LE(4).toString(16)})`);
  }
  if (bytes.readUInt32LE(12) !== 2) {
    fail('not_executable', `${label}: Mach-O filetype must be MH_EXECUTE`);
  }
  const count = bytes.readUInt32LE(16);
  const end = 32 + bytes.readUInt32LE(20);
  if (end > bytes.length) {
    fail('wrong_format', `${label}: Mach-O load commands are truncated`);
  }
  if (count === 0) fail('not_executable', `${label}: Mach-O has no load/entry commands`);
  const segments = [];
  let entryOffset = null;
  let entryAddress = null;
  let macOSPlatform = false;
  let cursor = 32;
  for (let i = 0; i < count; i += 1) {
    if (cursor + 8 > end) fail('wrong_format', `${label}: Mach-O command header is truncated`);
    const command = bytes.readUInt32LE(cursor);
    const size = bytes.readUInt32LE(cursor + 4);
    if (size < 8 || size % 8 !== 0 || cursor + size > end) {
      fail('wrong_format', `${label}: invalid Mach-O command size`);
    }
    if (command === 0x19) { // LC_SEGMENT_64
      if (size < 72 || 72 + bytes.readUInt32LE(cursor + 64) * 80 > size) {
        fail('wrong_format', `${label}: Mach-O segment/sections are truncated`);
      }
      const address = bytes.readBigUInt64LE(cursor + 24);
      const memorySize = bytes.readBigUInt64LE(cursor + 32);
      const offset = bytes.readBigUInt64LE(cursor + 40);
      const fileSize = bytes.readBigUInt64LE(cursor + 48);
      if (offset + fileSize > BigInt(bytes.length) || fileSize > memorySize) {
        fail('wrong_format', `${label}: Mach-O segment is not file-backed`);
      }
      if ((bytes.readUInt32LE(cursor + 60) & 4) !== 0) segments.push({ address, offset, fileSize });
    } else if (command === 0x32) { // LC_BUILD_VERSION
      if (size < 24 || 24 + bytes.readUInt32LE(cursor + 20) * 8 !== size) {
        fail('wrong_format', `${label}: invalid platform build command`);
      }
      if (bytes.readUInt32LE(cursor + 8) !== 1) fail('wrong_platform', `${label}: not built for macOS`);
      macOSPlatform = true;
    } else if (command === 0x24) { // LC_VERSION_MIN_MACOSX
      if (size !== 16) fail('wrong_format', `${label}: invalid macOS version command`);
      macOSPlatform = true;
    } else if ([0x25, 0x2f, 0x30].includes(command)) { // Other legacy Apple platforms
      fail('wrong_platform', `${label}: not built for macOS`);
    } else if (command === 0x80000028) { // LC_MAIN
      if (size !== 24 || entryOffset !== null) fail('wrong_format', `${label}: invalid LC_MAIN`);
      entryOffset = bytes.readBigUInt64LE(cursor + 8);
    } else if (command === 5) { // LC_UNIXTHREAD, ARM_THREAD_STATE64
      if (size !== 288 || bytes.readUInt32LE(cursor + 8) !== 6
        || bytes.readUInt32LE(cursor + 12) !== 68 || entryAddress !== null) {
        fail('wrong_format', `${label}: invalid ARM64 thread entry state`);
      }
      entryAddress = bytes.readBigUInt64LE(cursor + 272);
    }
    cursor += size;
  }
  if (cursor !== end) fail('wrong_format', `${label}: Mach-O command count/size disagree`);
  if (!macOSPlatform) fail('wrong_platform', `${label}: Mach-O lacks a macOS platform command`);
  const loadedEntry = segments.some(({ address, offset, fileSize }) =>
    (entryOffset !== null && entryOffset > 0n && entryOffset >= offset && entryOffset < offset + fileSize)
    || (entryAddress !== null && entryAddress > 0n && entryAddress >= address && entryAddress < address + fileSize));
  if (!loadedEntry) fail('not_executable', `${label}: Mach-O lacks a file-backed executable entry point`);
}

// ELF64 executable/static PIE: loaded entry point, no PT_INTERP or DT_NEEDED.
// Field layouts/tags follow https://gabi.xinuos.com/elf/.
function verifyStaticElf(bytes, machine, label) {
  if (bytes.length < 64 || bytes.readUInt32BE(0) !== 0x7f454c46) {
    fail('wrong_format', `${label}: not an ELF executable`);
  }
  if (bytes[4] !== 2 || bytes[5] !== 1) {
    fail('wrong_format', `${label}: not a 64-bit little-endian ELF`);
  }
  if (bytes[6] !== 1 || bytes.readUInt32LE(20) !== 1 || bytes.readUInt16LE(52) !== 64) {
    fail('wrong_format', `${label}: invalid ELF64 header version/size`);
  }
  const found = bytes.readUInt16LE(18);
  if (found !== machine) {
    fail('wrong_architecture', `${label}: ELF e_machine=${found}, expected ${machine}`);
  }
  const fileType = bytes.readUInt16LE(16);
  const entry = bytes.readBigUInt64LE(24);
  if (![2, 3].includes(fileType) || entry === 0n) {
    fail('not_executable', `${label}: ELF must be an executable/static PIE with an entry point`);
  }
  const phoff = Number(bytes.readBigUInt64LE(32));
  const phentsize = bytes.readUInt16LE(54);
  const phnum = bytes.readUInt16LE(56);
  if (phentsize < 56 || phnum === 0 || phoff + phentsize * phnum > bytes.length) {
    fail('wrong_format', `${label}: ELF program headers are unreadable`);
  }
  let loadedEntry = false;
  for (let i = 0; i < phnum; i += 1) {
    const header = phoff + i * phentsize;
    const type = bytes.readUInt32LE(header);
    if (type === 3) {
      fail('dynamic_loader', `${label}: ELF requests a dynamic loader (PT_INTERP); a static musl build is required`);
    }
    if (type === 1 || type === 2) {
      const offset = bytes.readBigUInt64LE(header + 8);
      const size = bytes.readBigUInt64LE(header + 32);
      if (offset + size > BigInt(bytes.length)) {
        fail('wrong_format', `${label}: ELF segment is truncated`);
      }
      if (type === 1) {
        if (size > bytes.readBigUInt64LE(header + 40)) {
          fail('wrong_format', `${label}: ELF load segment exceeds its memory mapping`);
        }
        const address = bytes.readBigUInt64LE(header + 16);
        if ((bytes.readUInt32LE(header + 4) & 1) !== 0 && entry >= address && entry < address + size) {
          loadedEntry = true;
        }
      } else {
        if (size % 16n !== 0n) fail('wrong_format', `${label}: ELF dynamic entries are truncated`);
        let terminated = false;
        for (let pos = Number(offset); pos < Number(offset + size); pos += 16) {
          const tag = bytes.readBigUInt64LE(pos);
          if (tag === 0n) { terminated = true; break; }
          if (tag === 1n) {
            fail('dynamic_dependency', `${label}: ELF has DT_NEEDED dependencies; a static build is required`);
          }
        }
        if (!terminated) fail('wrong_format', `${label}: ELF dynamic entries lack DT_NULL`);
      }
    }
  }
  if (!loadedEntry) fail('not_executable', `${label}: ELF entry point is outside executable file-backed segments`);
}

// Validate the caller-supplied canonical remote-runners directory the way the
// CLI resolves it and the shared Rust installer re-checks it, and return its
// per-target bytes so the Linux CLI inputs can be proven identical.
async function verifyRunnerBundles(dir) {
  const manifestPath = path.join(dir, 'manifest.json');
  let manifest;
  let manifestBytes;
  try {
    manifestBytes = await readFile(manifestPath);
    manifest = JSON.parse(manifestBytes.toString('utf8'));
  } catch {
    fail('missing_runner_bundles', `canonical remote-runners manifest not readable: ${manifestPath}`);
  }
  const entries = manifest.runners;
  if (!Array.isArray(entries)) fail('invalid_runner_bundles', 'remote-runners manifest lacks a runners array');
  const files = [];
  const bytesByTarget = {};
  for (const [targetId, triple] of Object.entries(LINUX_TARGETS)) {
    const meta = entries.find((r) => r.target === triple);
    if (!meta) fail('missing_runner_bundles', `remote-runners manifest is missing ${triple}`);
    if (meta.protocol !== PROTOCOL) fail('invalid_runner_bundles', `${triple}: manifest protocol is not ${PROTOCOL}`);
    const runnerPath = path.join(dir, triple, 'lintel');
    const bytes = await readFile(runnerPath).catch(() => {
      fail('missing_runner_bundles', `remote-runners file not readable: ${runnerPath}`);
    });
    if (bytes.length > MAX_RUNNER_BYTES) {
      fail('runner_too_large', `${triple}: runner exceeds the installer's 32 MiB limit`);
    }
    verifyStaticElf(bytes, targetId === 'linux-x86_64' ? 62 : 183, `remote-runners/${triple}/lintel`);
    const digest = sha256(bytes);
    if (meta.bytes !== bytes.length || meta.sha256 !== digest) {
      fail('invalid_runner_bundles', `${triple}: manifest size/digest does not match the actual bytes`);
    }
    files.push({ source: runnerPath, archive: `bin/remote-runners/${triple}/lintel`, bytes });
    bytesByTarget[targetId] = { bytes, digest };
  }
  files.push({ source: manifestPath, archive: 'bin/remote-runners/manifest.json', bytes: manifestBytes });
  return { files, manifest, bytesByTarget };
}

const HELP = 'See the header of scripts/package-cli.mjs for usage.';

// Ask the macOS binary for its true static identity. This executes only the
// "version" command, which is documented not to initialize state. Returns null
// when it cannot run here or returns no successful identity data.
function readBinaryIdentity(binary) {
  try {
    const out = execFileSync(binary, ['version', '--json'], {
      encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'], timeout: 15000,
    });
    const result = JSON.parse(out);
    return result?.ok === true && result.data !== null
      && typeof result.data === 'object' && !Array.isArray(result.data)
      ? result.data : null;
  } catch {
    return null;
  }
}

// Does any filesystem entry (regular file, directory, or symlink — including a
// dangling one) exist at this path? lstat never follows the final symlink.
async function entryExists(absolutePath) {
  try {
    await lstat(absolutePath);
    return true;
  } catch (error) {
    if (error.code === 'ENOENT') return false;
    throw error;
  }
}

// Publish bytes without ever replacing an existing entry and without ever
// exposing a partially written final file. The complete bytes go to an
// exclusive private temporary file in `tempDir` (a mkdtemp directory this
// process owns), fsynced, then hard-linked to the final path. A link is atomic
// and fails EEXIST if anything already occupies the final path, so an
// interrupted or failed write cannot surface a truncated final archive. The
// owned temporary is unlinked afterward. No journal or broader installer.
async function publishNoReplace(absolutePath, bytes, mode, tempDir, published) {
  if (await entryExists(absolutePath)) {
    fail('output_exists', `refusing to overwrite an existing path: ${absolutePath}`);
  }
  const staged = path.join(tempDir, `publish-${process.pid}-${Math.random().toString(36).slice(2)}`);
  let handle;
  let owned;
  try {
    handle = await open(staged, 'wx', mode);
    await handle.writeFile(bytes);
    await handle.sync();
    owned = await handle.stat({ bigint: true });
    await handle.close();
    handle = undefined;
  } finally {
    if (handle) await handle.close();
  }
  try {
    await link(staged, absolutePath);
    published.push({ path: absolutePath, dev: owned.dev, ino: owned.ino,
      size: owned.size, mtimeNs: owned.mtimeNs });
  } catch (error) {
    if (error.code === 'EEXIST') {
      fail('output_exists', `refusing to overwrite an existing path: ${absolutePath}`);
    }
    throw error;
  } finally {
    await rm(staged, { force: true });
  }
}

async function main() {
  const flags = parseArgs(process.argv.slice(2));
  if (flags.has('help')) return console.log(HELP);

  const binaries = {
    'macos-arm64': required(flags, 'macos-arm64'),
    'linux-x86_64': required(flags, 'linux-x86_64'),
    'linux-aarch64': required(flags, 'linux-aarch64'),
  };
  const runnerDir = required(flags, 'linux-runners');
  const outDir = required(flags, 'out');
  // The source revision is an explicit caller declaration used for build
  // identity; it is never inferred from a digest or probe.
  const revision = flags.get('revision');
  if (!revision || !/^(?:[0-9a-f]{40}|[0-9a-f]{64})$/.test(revision)) {
    fail('invalid_revision', '--revision must be the exact full lowercase hex git object id (40 or 64 chars)');
  }

  const cli = {};
  const macosBytes = await readFile(binaries['macos-arm64']).catch(() => {
    fail('missing_input', `macOS arm64 input not readable: ${binaries['macos-arm64']}`);
  });
  verifyMachOArm64(macosBytes, 'macos-arm64/lintel');
  cli['macos-arm64'] = { bytes: macosBytes, source: binaries['macos-arm64'] };
  // Probing happens in a private temp directory so a later rejection leaves the
  // caller's output path untouched.
  const probeDir = await mkdtemp(path.join(tmpdir(), 'lintel-package-probe-'));
  const probe = path.join(probeDir, 'lintel');
  await writeFile(probe, macosBytes, { mode: 0o755 });
  const probed = readBinaryIdentity(probe);
  await rm(probeDir, { recursive: true, force: true });

  const runners = await verifyRunnerBundles(runnerDir);
  const declared = flags.get('version');
  if (declared && !VERSION_PATTERN.test(declared)) {
    fail('invalid_argument', '--version must be a version string like 0.1.0');
  }
  if (probed) {
    if (!['Lintel', 'lintel'].includes(probed.product)) {
      fail('invalid_identity', `macOS binary reports unexpected product: ${probed.product}`);
    }
    if (probed.protocol !== PROTOCOL || probed.catalog_version !== CATALOG_VERSION) {
      fail('invalid_identity', `macOS binary protocol/catalog ${probed.protocol}/${probed.catalog_version} != expected ${PROTOCOL}/${CATALOG_VERSION}`);
    }
  }
  // When the macOS binary runs here its self-report is authoritative and any
  // --version must agree. Without verified identity, an explicit --version is required
  // and every target records that its identity was declared, not executed.
  if (probed && declared && probed.version !== declared) {
    fail('version_mismatch', `--version ${declared} does not match the macOS binary self-report ${probed.version}`);
  }
  const version = probed ? probed.version : declared;
  if (typeof version !== 'string' || version.length === 0) {
    fail('version_unverified', 'cannot determine candidate version; pass --version (the macOS input returned no verified identity)');
  }
  if (!VERSION_PATTERN.test(version)) {
    fail('invalid_argument', 'candidate version must be a safe version string like 0.1.0');
  }
  // Both canonical runner manifest entries must declare the candidate version.
  for (const entry of runners.manifest.runners) {
    if (entry.version !== version) {
      fail('version_mismatch', `remote-runners ${entry.target} manifest version ${entry.version} does not match candidate version ${version}`);
    }
  }
  for (const targetId of Object.keys(LINUX_TARGETS)) {
    const bytes = await readFile(binaries[targetId]).catch(() => {
      fail('missing_input', `${targetId} input not readable: ${binaries[targetId]}`);
    });
    verifyStaticElf(bytes, targetId === 'linux-x86_64' ? 62 : 183, `${targetId}/lintel`);
    const canonical = runners.bytesByTarget[targetId];
    if (sha256(bytes) !== canonical.digest) {
      fail('input_runner_mismatch', `${targetId} CLI input is not byte-identical to the canonical ${LINUX_TARGETS[targetId]} runner; stale or mixed inputs cannot form one candidate`);
    }
    cli[targetId] = { bytes, source: binaries[targetId], identitySource: 'declared_static' };
  }
  cli['macos-arm64'].identitySource = probed ? 'macos_input_executed' : 'declared_static';

  // The candidate carries an explicit build identity so that a bare product
  // version (e.g. 0.1.0) is never the only identity a caller sees.
  const identity = `${version}-candidate-${revision.slice(0, 12)}`;
  const indexPath = path.join(outDir, `candidates-${identity}.json`);

  await mkdir(outDir, { recursive: true });
  // Refuse before writing any archive if this identity's index already exists —
  // including a dangling symlink — so a rejected second call never writes
  // archives or leaves an index claiming older outputs.
  if (await entryExists(indexPath)) {
    fail('output_exists', `refusing to overwrite an existing candidate index: ${indexPath}`);
  }
  // A private mkdtemp directory this process owns holds staged publications.
  // Hard-link publication requires the staged file and destination to share a
  // filesystem; keep our private staging directory inside the selected output.
  const publishDir = await mkdtemp(path.join(outDir, '.lintel-publish-'));
  const published = [];
  try {
    const archives = await buildArchives({ outDir, identity, version, revision, cli, runners, publishDir, probed, published });
    const summary = {
      schema: 'lintel.cli-candidates/1',
      version,
      candidate_identity: identity,
      protocol: PROTOCOL,
      catalog_version: CATALOG_VERSION,
      source_revision: revision,
      source_revision_note: 'caller-declared build identity; not derived from a digest or probe',
      archives,
    };
    await publishNoReplace(indexPath, Buffer.from(`${JSON.stringify(summary, null, 2)}\n`), 0o644, publishDir, published);
    console.log(JSON.stringify(summary, null, 2));
    return undefined;
  } catch (error) {
    // A reported failure must not strand our partial candidate set. Preserve
    // pre-existing entries and any published path subsequently changed by
    // another writer; only unchanged links still owned by this call are removed.
    const failures = [];
    for (const entry of published.reverse()) {
      try {
        const current = await lstat(entry.path, { bigint: true });
        if (current.dev === entry.dev && current.ino === entry.ino
          && current.size === entry.size && current.mtimeNs === entry.mtimeNs) {
          await rm(entry.path);
        }
      } catch (cleanupError) {
        if (cleanupError.code !== 'ENOENT') failures.push(`${entry.path}: ${cleanupError.message}`);
      }
    }
    if (failures.length) error.message += `; candidate rollback incomplete: ${failures.join('; ')}`;
    throw error;
  } finally {
    await rm(publishDir, { recursive: true, force: true });
  }
}

async function buildArchives({ outDir, identity, version, revision, cli, runners, publishDir, probed, published }) {

  const archives = [];
  // Refuse every occupied archive path (regular file, directory, dangling
  // symlink) before publishing any of them, so a conflicting call cannot leave
  // a partial candidate set behind.
  for (const targetId of Object.keys(TARGETS)) {
    const archivePath = path.join(outDir, `lintel-cli-${identity}-${targetId}.tar.gz`);
    if (await entryExists(archivePath)) {
      fail('output_exists', `refusing to overwrite an existing archive: ${archivePath}`);
    }
  }
  for (const [targetId, target] of Object.entries(TARGETS)) {
    const archiveName = `lintel-cli-${identity}-${targetId}.tar.gz`;
    const archivePath = path.join(outDir, archiveName);

    const stage = path.join(publishDir, `.stage-${targetId}-${process.pid}`);
    await rm(stage, { recursive: true, force: true });
    await mkdir(path.join(stage, 'bin'), { recursive: true });

    // The CLI resolves its remote runners at `dirname(current_exe)/remote-runners`,
    // so the executable lives at bin/lintel and the bundles at bin/remote-runners.
    const entries = [{ archive: 'bin/lintel', bytes: cli[targetId].bytes }];
    for (const file of runners.files) {
      const destination = path.join(stage, file.archive);
      await mkdir(path.dirname(destination), { recursive: true });
      await writeFile(destination, file.bytes, { mode: 0o700 });
      entries.push({ archive: file.archive, bytes: file.bytes });
    }
    await writeFile(path.join(stage, 'bin', 'lintel'), cli[targetId].bytes, { mode: 0o755 });

    const fileEntries = entries.map((entry) => ({
      path: entry.archive,
      bytes: entry.bytes.length,
      sha256: sha256(entry.bytes),
    }));
    const manifest = {
      schema: 'lintel.cli-candidate/1',
      product: PRODUCT,
      version,
      candidate_identity: identity,
      protocol: PROTOCOL,
      catalog_version: CATALOG_VERSION,
      source_revision: revision,
      source_revision_note: 'caller-declared build identity; not derived from a digest or probe',
      target: targetId,
      platform: target.platform,
      architecture: target.architecture,
      triple: target.triple,
      // Honest per-target provenance: only the macOS input is executed here.
      identity_source: targetId === 'macos-arm64' ? cli[targetId].identitySource : 'declared_static',
      identity_verified_executed: targetId === 'macos-arm64' && probed !== null,
      signed: false,
      release: false,
      limits: target.platform === 'macos'
        ? ['requires macOS arm64 (Apple silicon); not notarized or signed']
        : ['static musl build; remote browser component reports browser_component_unavailable'],
      files: fileEntries,
    };
    const manifestBytes = Buffer.from(`${JSON.stringify(manifest, null, 2)}\n`);
    await writeFile(path.join(stage, 'candidate.json'), manifestBytes);
    entries.push({ archive: 'candidate.json', bytes: manifestBytes });

    const readme = [
      `${PRODUCT} CLI candidate ${identity} (${targetId})`,
      '',
      'This is an unsigned development candidate, not a formal release.',
      `Product version: ${version}  Protocol: ${PROTOCOL}  Catalog: ${CATALOG_VERSION}`,
      `Source revision (declared build identity): ${revision}`,
      `Input identity source: ${manifest.identity_source} (executed=${manifest.identity_verified_executed})`,
      '',
      'Install without Rust/GUI into an explicit NEW identity directory:',
      '  1. Create only the parent, then require the identity directory to be absent',
      '     (plain `mkdir` fails if it already exists, so a prior version is never overwritten):',
      '       parent="$' + '{LINTEL_CLI_ROOT:-$HOME/.local/share/lintel-cli}"',
      '       mkdir -p "$parent"',
      `       dest="$parent/${identity}"`,
      `       mkdir "$dest" && tar -xzf ${archiveName} -C "$dest"`,
      '  2. Verify the installed bytes from the selected directory (no Node required):',
      '       cd "$dest"',
      `       ${target.platform === 'macos' ? 'shasum -a 256 -c SHA256SUMS' : 'sha256sum -c SHA256SUMS'}`,
      '  3. Run the absolute executable path and inspect identity/capabilities:',
      '       "$dest/bin/lintel" version --json',
      '       "$dest/bin/lintel" capabilities --json',
      '',
      'Upgrade by selecting another candidate identity directory; never replace this one in place.',
      'This archive changes no PATH, shell rc, App, service, Claude, credentials or browser registration.',
      'Both canonical Linux remote runners ship at bin/remote-runners, resolved next to bin/lintel.',
      '',
    ].join('\n');
    const readmeBytes = Buffer.from(readme);
    await writeFile(path.join(stage, 'README.txt'), readmeBytes);
    entries.push({ archive: 'README.txt', bytes: readmeBytes });

    // Verify every archived file except the checksum list itself, including
    // the candidate identity/limitations and the installation instructions.
    const sums = entries
      .map((entry) => `${sha256(entry.bytes)}  ${entry.archive}`)
      .sort()
      .join('\n');
    const sumsBytes = Buffer.from(`${sums}\n`);
    await writeFile(path.join(stage, 'SHA256SUMS'), sumsBytes);
    entries.push({ archive: 'SHA256SUMS', bytes: sumsBytes });

    // Build the archive in a private temp file, then publish with true
    // no-replace so an existing archive (regular file, directory or dangling
    // symlink) is refused before anything is written and no partial final
    // archive can appear.
    const tempArchive = path.join(publishDir, `.tmp-${archiveName}-${process.pid}`);
    await rm(tempArchive, { force: true });
    const tarEnv = { ...process.env, COPYFILE_DISABLE: '1' };
    delete tarEnv.TAR_OPTIONS;
    const expectedMembers = entries.map((e) => e.archive).sort();
    execFileSync('tar', [
      '-czf', tempArchive,
      '-C', stage,
      ...expectedMembers,
    ], {
      // COPYFILE_DISABLE suppresses macOS AppleDouble `._*` members so the
      // archive contains exactly the payload the manifest lists.
      // GNU TAR_OPTIONS must not exclude/transform explicit payload members.
      env: tarEnv,
      stdio: ['ignore', 'inherit', 'inherit'],
    });
    const members = execFileSync('tar', ['-tzf', tempArchive], {
      env: tarEnv, encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'],
    }).trim().split('\n').sort();
    if (members.length !== expectedMembers.length || members.some((name, i) => name !== expectedMembers[i])) {
      fail('invalid_archive', 'tar output does not contain exactly the declared candidate files');
    }
    const archiveBytes = await readFile(tempArchive);
    try {
      await publishNoReplace(archivePath, archiveBytes, 0o644, publishDir, published);
    } finally {
      await rm(tempArchive, { force: true });
      await rm(stage, { recursive: true, force: true });
    }

    archives.push({
      target: targetId,
      archive: archiveName,
      bytes: archiveBytes.length,
      sha256: sha256(archiveBytes),
      cli_sha256: sha256(cli[targetId].bytes),
    });
  }

  return archives;
}

main().catch((error) => {
  console.error(`${error.code ?? 'error'}: ${error.message}`);
  process.exitCode = 1;
});

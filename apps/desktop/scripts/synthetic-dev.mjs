import { mkdtemp, mkdir, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

// Explicit opt-in developer transport. All core state and discovery use a fresh
// synthetic home. No real credentials, shell settings, or browser profiles.
const fixture = await mkdtemp(path.join(tmpdir(), 'lintel-ui-fixture-'));
const home = path.join(fixture, 'home');
const root = path.join(home, '.claude');
await mkdir(path.join(root, 'projects'), { recursive: true });
await mkdir(path.join(root, 'projects/synthetic/memory'), { recursive: true });
await writeFile(path.join(root, 'settings.json'), JSON.stringify({ env: { DISABLE_TELEMETRY: '0' }, permissions: { allow: [] } }, null, 2));
await writeFile(path.join(root, 'CLAUDE.md'), '# Synthetic writing environment\nPreserve these instructions.\n');
await writeFile(path.join(root, 'projects/synthetic/memory', 'notes.md'), 'Synthetic working note.\n');
const bin = path.resolve(fileURLToPath(new URL('../../../target/debug/lintel', import.meta.url)));
const env = { ...process.env, HOME: home, LINTEL_TEST_HOME: home, LINTEL_STATE_DIR: path.join(fixture, 'state'), LINTEL_FIXTURE_ROOT: fixture, LINTEL_FIXTURE_RUNNER: bin };
console.log(`Synthetic UI session: ${fixture}\nRunner: ${bin}\nNo production state is used. The fixture is retained for inspection.`);
const child = spawn(process.execPath, ['node_modules/vite/bin/vite.js', '--mode', 'fixture', '--host', '127.0.0.1'], { env, stdio: 'inherit' });
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => child.kill(signal));
child.on('exit', code => process.exitCode = code ?? 1);

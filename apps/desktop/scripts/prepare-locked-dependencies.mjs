// Explicit network preparation; notice collection remains locked and offline.
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
const root = fileURLToPath(new URL('../../../', import.meta.url));
export const commands = [
  ['rustup', ['component', 'add', 'rust-docs']],
  ...['Cargo.toml', 'apps/desktop/src-tauri/Cargo.toml', 'extensions/browser/native-host/Cargo.toml'].map(manifest => ['cargo', ['fetch', '--locked', '--manifest-path', manifest]]),
];
export function checkPrerequisites({version=process.versions.node,platform=process.platform,run=spawnSync}={}) {
  const [major,minor] = version.split('.').map(Number);
  if (!(major>22 || (major===22 && minor>=12))) throw new Error('Node.js >=22.12.0 is required; select a supported Node.js before npm ci.');
  for (const [tool,args,fix] of [
    ['npm',['--version'],'Install npm >=10 with the supported Node.js runtime.'],
    ['rustc',['--version'],'Install Rust stable with rustup, then retry.'],
    ['cargo',['--version'],'Install Rust stable with rustup, then retry.'],
    ['rustup',['--version'],'Install rustup to prepare rust-docs for the active toolchain.'],
    ...(platform==='darwin' ? [['xcode-select',['--print-path'],'Install Xcode Command Line Tools with xcode-select --install.']] : []),
  ]) {
    const result=run(tool,args,{cwd:root,encoding:'utf8'});
    if(result.error || result.status!==0) throw new Error(`${tool} is unavailable. ${fix}`);
    if(tool==='npm' && Number(result.stdout.trim().split('.')[0])<10) throw new Error('npm >=10 is required; update npm for the supported Node.js runtime.');
  }
}
if (process.argv[1] && path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  if (process.argv.includes('--dry-run')) console.log(JSON.stringify(commands));
  else {
    checkPrerequisites();
    if (!process.argv.includes('--check')) for (const [command, args] of commands) {
      const result = spawnSync(command, args, {cwd:root,stdio:'inherit'});
      if (result.error) throw result.error;
      if (result.status!==0) process.exit(result.status??1);
    }
  }
}

import {execFileSync} from 'node:child_process';
import {mkdir,rm} from 'node:fs/promises';
import path from 'node:path';import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
execFileSync(process.execPath,[path.join(root,'scripts/build.mjs')],{stdio:'inherit'});
await mkdir(path.join(root,'artifacts'),{recursive:true});
for(const browser of ['chromium','firefox']){
 const zip=path.join(root,'artifacts',`lintel-${browser}-0.1.0-dev.zip`);
 await rm(zip,{force:true});
 execFileSync('zip',['-qr',zip,'.'],{cwd:path.join(root,'dist',browser)});
 console.log(zip);
}

// Disposable synthetic signing fixture. Not a runnable/signed Lintel build.
import {generateKeyPairSync,createHash,sign,randomBytes} from 'node:crypto';
import {readFileSync,mkdirSync,writeFileSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
export function fixture(directory,version='0.2.0'){
 mkdirSync(directory,{recursive:true});
 const archive=path.join(directory,'Lintel_0.2.0_aarch64.app.tar.gz');
 const build=spawnSync('python3',['-c',`import io,plistlib,struct,sys,tarfile
with tarfile.open(sys.argv[1], 'w:gz') as t:
 for name,body,mode in [('Lintel.app/Contents/Info.plist',plistlib.dumps({'CFBundleIdentifier':'app.lintel.desktop','CFBundleExecutable':'lintel-desktop','CFBundleShortVersionString':sys.argv[2]}),0o644),('Lintel.app/Contents/MacOS/lintel-desktop',bytes.fromhex('cffaedfe')+struct.pack('<I',0x0100000C)+b'SYNTHETIC-INERT-FORMAT-FIXTURE',0o755)]:
  e=tarfile.TarInfo(name);e.size=len(body);e.mode=mode;t.addfile(e,io.BytesIO(body))`,archive,version],{encoding:'utf8'});
 if(build.status!==0)throw new Error(build.stderr);
 const {privateKey,publicKey}=generateKeyPairSync('ed25519'),id=randomBytes(8);
 const keyBytes=publicKey.export({format:'der',type:'spki'}).subarray(-32);
 const pubkey=Buffer.from('untrusted comment: synthetic disposable test key\n'+Buffer.concat([Buffer.from('Ed'),id,keyBytes]).toString('base64')+'\n').toString('base64');
 const bytes=readFileSync(archive),digest=createHash('blake2b512').update(bytes).digest(),sig=sign(null,digest,privateKey);
 const trusted=`timestamp:1\tfile:Lintel.app.tar.gz\tversion:${version}`;
 const signed=Buffer.concat([Buffer.from('ED'),id,sig]);
 const global=sign(null,Buffer.concat([sig,Buffer.from(trusted)]),privateKey);
 const signature=Buffer.from(`untrusted comment: synthetic fixture\n${signed.toString('base64')}\ntrusted comment: ${trusted}\n${global.toString('base64')}\n`).toString('base64');
 const signaturePath=archive+'.sig';writeFileSync(signaturePath,signature,{mode:0o600});
 const dmg=path.join(directory,'Lintel_0.2.0_aarch64.dmg');writeFileSync(dmg,'SYNTHETIC DMG BYTE FIXTURE');
 return {archive,signaturePath,dmg,pubkey,signature};
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url))console.log(JSON.stringify(fixture(process.argv[2],process.argv[3]??'0.2.0')));

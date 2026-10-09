// Local candidate preparation only. Never signs, uploads or activates a feed.
import {readFile,writeFile,mkdir,lstat,open} from 'node:fs/promises';
import {constants} from 'node:fs';
import {createHash,createPublicKey,verify} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import path from 'node:path';

const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const feed=channel=>`https://lintel.page/updates/${channel}.json`;
const fail=message=>{throw new Error(message);};
const MAX_BYTES=512*1024*1024;
function exact(value,keys,label){if(!value||typeof value!=='object'||Array.isArray(value)||Object.keys(value).some(k=>!keys.includes(k)))fail(`Invalid ${label} fields`);}
function version(value){
 const match=typeof value==='string'&&value.match(/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/);
 if(!match||value.length>128)fail('Expected strict SemVer without a v prefix');
 if(match.slice(1,4).some(v=>BigInt(v)>18446744073709551615n))fail('SemVer component exceeds the updater u64 limit');
 const pre=match[4]?.split('.')??[];if(pre.some(v=>/^\d+$/.test(v)&&v.length>1&&v.startsWith('0')))fail('Invalid prerelease numeric identifier');
 return {parts:match.slice(1,4).map(BigInt),pre};
}
export function newer(candidate,previous){
 const a=version(candidate),b=version(previous);
 for(let i=0;i<3;i++)if(a.parts[i]!==b.parts[i])return a.parts[i]>b.parts[i];
 if(!a.pre.length||!b.pre.length)return !a.pre.length&&!!b.pre.length;
 for(let i=0;i<Math.max(a.pre.length,b.pre.length);i++){
  if(a.pre[i]===undefined)return false;if(b.pre[i]===undefined)return true;if(a.pre[i]===b.pre[i])continue;
  const an=/^\d+$/.test(a.pre[i]),bn=/^\d+$/.test(b.pre[i]);
  if(an&&bn)return BigInt(a.pre[i])>BigInt(b.pre[i]);if(an!==bn)return !an;return a.pre[i]>b.pre[i];
 }return false;
}
function base64(value,label){
 if(typeof value!=='string'||value.length>8192||!value.length||!/^[-A-Za-z0-9+/=\r\n]+$/.test(value))fail(`Invalid ${label}`);
 const clean=value.replace(/\s/g,'');const result=Buffer.from(clean,'base64');if(result.toString('base64')!==clean)fail(`Noncanonical ${label}`);return result;
}
function publicKey(value){
 const lines=base64(value,'public key').toString('utf8').trimEnd().split('\n');
 if(lines.length!==2||!lines[0].startsWith('untrusted comment: '))fail('Invalid Minisign public key');
 const bytes=base64(lines[1],'public key payload');
 if(bytes.length!==42||!['Ed','ED'].includes(bytes.subarray(0,2).toString()))fail('Invalid Minisign public key payload');
 return {id:bytes.subarray(2,10),key:createPublicKey({key:Buffer.concat([Buffer.from('302a300506032b6570032100','hex'),bytes.subarray(10)]),format:'der',type:'spki'})};
}
function identity(stat){return [stat.dev,stat.ino,stat.size,stat.mtimeNs,stat.ctimeNs].map(String).join(':');}
async function digestFile(file,algorithm){
 const handle=await open(file,constants.O_RDONLY|constants.O_NOFOLLOW|constants.O_NONBLOCK);
 try{
  const before=await handle.stat({bigint:true});
  if(!before.isFile()||before.size<=0n||before.size>BigInt(MAX_BYTES))fail('Expected bounded regular App artifact');
  const hash=createHash(algorithm),sha256=createHash('sha256');let bytes=0;
  for await(const chunk of handle.createReadStream({autoClose:false})){
   bytes+=chunk.length;if(bytes>MAX_BYTES)fail('App artifact exceeds 512 MiB admission');hash.update(chunk);sha256.update(chunk);
  }
  if(bytes!==Number(before.size)||identity(before)!==identity(await handle.stat({bigint:true}))||identity(before)!==identity(await lstat(file,{bigint:true})))fail('App artifact changed during verification');
  return {bytes,digest:hash.digest(),sha256:sha256.digest('hex'),identity:identity(before)};
 }finally{await handle.close();}
}
// Minisign Ed25519 prehash and its authenticated trusted comment. Uses Node's
// crypto primitives; the App independently verifies again with official Tauri.
export async function verifyArtifact(file,signature,pubkey,appVersion){
 const key=publicKey(pubkey),lines=base64(signature.trim(),'signature').toString('utf8').trimEnd().split('\n');
 if(lines.length!==4||!lines[0].startsWith('untrusted comment: ')||!lines[2].startsWith('trusted comment: '))fail('Invalid Minisign signature');
 const payload=base64(lines[1],'signature payload'),global=base64(lines[3],'comment signature');
 if(payload.length!==74||global.length!==64||payload.subarray(0,2).toString()!=='ED'||!payload.subarray(2,10).equals(key.id))fail('Expected matching prehashed update signature');
 const hashed=await digestFile(file,'blake2b512');
 if(!verify(null,hashed.digest,key.key,payload.subarray(10)))fail('Update artifact signature failed');
 const comment=lines[2].slice('trusted comment: '.length);
 if(!verify(null,Buffer.concat([payload.subarray(10),Buffer.from(comment)]),key.key,global))fail('Update trusted comment signature failed');
 const versions=comment.split('\t').filter(v=>v.startsWith('version:')).map(v=>v.slice(8));
 if(versions.length!==1||versions[0]!==appVersion)fail('Signed version differs from release version');
 return hashed;
}
export function validateRecord(record){
 exact(record,['schema','version','previous_version','channel','source_revision','pubkey','notes','pub_date','platforms'],'release record');
 if(record.schema!=='lintel.app-release/1'||!['preview','stable'].includes(record.channel))fail('Invalid release schema/channel');
 if(!newer(record.version,record.previous_version))fail('Release must increase SemVer precedence; build metadata is not an upgrade');
 if(record.channel==='stable'&&version(record.version).pre.length)fail('Stable cannot publish prerelease');
 if(!/^[a-f0-9]{40}$/.test(record.source_revision))fail('Expected source revision declaration');
 publicKey(record.pubkey);
 if(typeof record.notes!=='string'||Buffer.byteLength(record.notes)>16*1024||typeof record.pub_date!=='string'||!/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d{3})?Z$/.test(record.pub_date)||!Number.isFinite(Date.parse(record.pub_date)))fail('Invalid release notes/date');
 exact(record.platforms,['darwin-aarch64','darwin-x86_64'],'platforms');
 if(!Object.keys(record.platforms).length)fail('Select at least one macOS architecture');
 for(const [platform,asset] of Object.entries(record.platforms)){
  exact(asset,['archive_path','signature_path','dmg_path','archive_name','dmg_name'],platform);
  for(const key of ['archive_path','signature_path','dmg_path'])if(typeof asset[key]!=='string'||!path.isAbsolute(asset[key]))fail('Artifact input paths must be explicit and absolute');
  for(const key of ['archive_name','dmg_name'])if(typeof asset[key]!=='string'||!/^[-A-Za-z0-9_.]+$/.test(asset[key])||!asset[key].includes(record.version))fail('Public asset names must include exact version and safe characters');
  if(!asset.archive_name.endsWith('.app.tar.gz')||!asset.dmg_name.endsWith('.dmg'))fail('DMG and updater archive must be separate');
 }
 return record;
}
export function buildConfiguration(record){
 validateRecord(record);
 return {version:record.version,bundle:{targets:['app','dmg'],createUpdaterArtifacts:true},plugins:{updater:{pubkey:record.pubkey,endpoints:[feed(record.channel)],requireSignedVersion:true,allowDowngrades:false}}};
}
async function absent(out){
 try{await lstat(out);fail('Output directory must be absent')}catch(error){if(error.code!=='ENOENT')throw error;}
 if(out.startsWith(repo+path.sep)&&spawnSync('git',['check-ignore','-q',out],{cwd:repo}).status!==0)fail('Repository output must be ignored or outside Git');
}
async function regular(file,max=MAX_BYTES){const s=await lstat(file);if(!s.isFile()||!s.size||s.size>max)fail('Expected bounded regular artifact');}
export async function prepare(record){
 validateRecord(record);const platforms={},downloads=[];
 for(const [platform,asset] of Object.entries(record.platforms)){
  await regular(asset.archive_path);await regular(asset.signature_path,8192);await regular(asset.dmg_path);
  const frozen=identity(await lstat(asset.archive_path,{bigint:true}));
  const signature=(await readFile(asset.signature_path,'utf8')).trim();
  const archive=await verifyArtifact(asset.archive_path,signature,record.pubkey,record.version);
  const inspected=spawnSync('python3',[path.join(repo,'scripts/inspect-app-archive.py'),asset.archive_path,record.version,platform],{encoding:'utf8',timeout:60000,maxBuffer:16384});
  if(inspected.status!==0)fail(`App archive rejected: ${inspected.stderr.trim()}`);
  if(archive.identity!==frozen||identity(await lstat(asset.archive_path,{bigint:true}))!==frozen)fail('App archive changed during signature/format inspection');
  const dmg=await digestFile(asset.dmg_path,'sha256');
  const url=name=>`https://github.com/IndelibleVivi/lintel-cc/releases/download/v${record.version}/${name}`;
  platforms[platform]={url:url(asset.archive_name),signature,bytes:archive.bytes,sha256:archive.sha256};
  downloads.push({platform,url:url(asset.dmg_name),bytes:dmg.bytes,sha256:dmg.digest.toString('hex')});
 }
 const release={schema:'lintel.app-release-public/1',version:record.version,previous_version:record.previous_version,channel:record.channel,source_revision:record.source_revision,notes:record.notes,pub_date:record.pub_date,pubkey:record.pubkey,platforms,downloads,status:'prepared_local',publication_verified:false,apple_security:{developer_id:'unverified',notarization:'unverified'}};
 validatePublicRelease(release);
 const updaterFeed={version:record.version,notes:record.notes,pub_date:record.pub_date,platforms:Object.fromEntries(Object.entries(platforms).map(([key,{url,signature}])=>[key,{url,signature}]))};
 return {release,updaterFeed};
}
export function validatePublicRelease(release){
 exact(release,['schema','version','previous_version','channel','source_revision','notes','pub_date','pubkey','platforms','downloads','status','publication_verified','apple_security','public_checked_at'],'public release record');
 if(release.schema!=='lintel.app-release-public/1'||!['preview','stable'].includes(release.channel))fail('Invalid public release record');
 if(!newer(release.version,release.previous_version)||release.channel==='stable'&&version(release.version).pre.length)fail('Invalid public release version');
 publicKey(release.pubkey);
 if(!/^[a-f0-9]{40}$/.test(release.source_revision)||typeof release.notes!=='string'||Buffer.byteLength(release.notes)>16*1024||typeof release.pub_date!=='string'||!/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d{3})?Z$/.test(release.pub_date)||!Number.isFinite(Date.parse(release.pub_date)))fail('Invalid public release metadata');
 if(!['prepared_local','public_bytes_verified'].includes(release.status)||typeof release.publication_verified!=='boolean'||release.publication_verified!==(release.status==='public_bytes_verified'))fail('Invalid publication status');
 if(release.public_checked_at!==undefined && (typeof release.public_checked_at!=='string'||!Number.isFinite(Date.parse(release.public_checked_at))))fail('Invalid public verification date');
 if(release.publication_verified && !release.public_checked_at)fail('Missing public verification date');
 exact(release.apple_security,['developer_id','notarization'],'Apple security status');
 if(!['unverified','verified'].includes(release.apple_security.developer_id)||!['unverified','verified'].includes(release.apple_security.notarization))fail('Invalid Apple security declaration');
 exact(release.platforms,['darwin-aarch64','darwin-x86_64'],'public platforms');
 const platforms=Object.keys(release.platforms);
 if(!platforms.length||!Array.isArray(release.downloads)||release.downloads.length!==platforms.length)fail('Invalid release assets');
 const seen=new Set(),names=new Set();
 const asset=(item,suffix)=>{
  const u=new URL(item.url),name=u.pathname.split('/').at(-1);
  if(u.origin!=='https://github.com'||u.username||u.password||u.port||u.search||u.hash||u.pathname!==`/IndelibleVivi/lintel-cc/releases/download/v${release.version}/${name}`||!/^[-A-Za-z0-9_.]+$/.test(name)||!name.includes(release.version)||!name.endsWith(suffix)||names.has(name)||!Number.isSafeInteger(item.bytes)||item.bytes<=0||item.bytes>MAX_BYTES||!/^[a-f0-9]{64}$/.test(item.sha256))fail('Invalid declared release asset');
  names.add(name);
 };
 for(const [platform,item] of Object.entries(release.platforms)){
  exact(item,['url','signature','bytes','sha256'],platform);asset(item,'.app.tar.gz');base64(item.signature,'update signature');
 }
 for(const item of release.downloads){
  exact(item,['platform','url','bytes','sha256'],'download');
  if(!platforms.includes(item.platform)||seen.has(item.platform))fail('Invalid download platform');seen.add(item.platform);asset(item,'.dmg');
 }
 return release;
}
async function verifyPublic(release){
 validatePublicRelease(release);
 for(const asset of [...Object.values(release.platforms),...release.downloads]){
  const response=await fetch(asset.url,{signal:AbortSignal.timeout(300000)});
  if(!response.ok)fail('Release artifact is not publicly available');
  if(new URL(response.url).protocol!=='https:')fail('Insecure release redirect');
  const hash=createHash('sha256');let bytes=0;
  for await(const chunk of response.body){bytes+=chunk.length;if(bytes>asset.bytes||bytes>MAX_BYTES)fail('Published artifact bytes differ');hash.update(chunk);}
  if(bytes!==asset.bytes||hash.digest('hex')!==asset.sha256)fail('Published artifact digest differs');
 }
 return {...release,status:'public_bytes_verified',publication_verified:true,public_checked_at:new Date().toISOString()};
}
async function main(){
 const args=process.argv.slice(2),mode=args.shift();let input,out,platform;
 for(let i=0;i<args.length;i++){if(args[i]==='--record')input=args[++i];else if(args[i]==='--out')out=args[++i];else if(args[i]==='--platform')platform=args[++i];else fail('Unknown release argument');}
 if(!['configure','build','prepare','verify-public'].includes(mode)||!input||(mode!=='build'&&!out))fail('Usage: app-release.mjs configure|prepare|verify-public --record FILE --out ABSENT_DIRECTORY; build --record FILE');
 await regular(input,64*1024);const record=JSON.parse(await readFile(input,'utf8'));
 if(mode==='build'){
  validateRecord(record);
  platform=platform??(Object.keys(record.platforms).length===1?Object.keys(record.platforms)[0]:undefined);
  if(!platform||!Object.hasOwn(record.platforms,platform))fail('Build requires one platform selected from the record');
  const target={'darwin-aarch64':'aarch64-apple-darwin','darwin-x86_64':'x86_64-apple-darwin'}[platform];
  const current=JSON.parse(await readFile(path.join(repo,'apps/desktop/src-tauri/tauri.conf.json'),'utf8')).version;
  if(record.previous_version!==current)fail('Build previous_version must match current source App version');
  const actual=spawnSync('git',['rev-parse','HEAD'],{cwd:repo,encoding:'utf8'}),dirty=spawnSync('git',['status','--porcelain'],{cwd:repo,encoding:'utf8'});
  if(actual.status!==0||dirty.status!==0||actual.stdout.trim()!==record.source_revision||dirty.stdout.trim())fail('Release build requires a clean exact source revision declaration');
  const result=spawnSync(process.execPath,[path.join(repo,'apps/desktop/node_modules/@tauri-apps/cli/tauri.js'),'build','--target',target,'--config',JSON.stringify(buildConfiguration(record))],{cwd:path.join(repo,'apps/desktop'),stdio:'inherit',env:{...process.env,LINTEL_APP_VERSION:record.version}});
  if(result.status!==0)fail('Release build failed');return;
 }
 out=path.resolve(out);await absent(out);
 const result=mode==='configure'?{configuration:buildConfiguration(record)}:mode==='prepare'?await prepare(record):{release:await verifyPublic(record)};
 await mkdir(out,{recursive:true});
 if(result.configuration)await writeFile(path.join(out,'tauri-release.json'),JSON.stringify(result.configuration,null,2),{flag:'wx'});
 if(result.release)await writeFile(path.join(out,'release.json'),JSON.stringify(result.release,null,2),{flag:'wx'});
 if(result.updaterFeed){await mkdir(path.join(out,'updates'));await writeFile(path.join(out,'updates',`${record.channel}.json`),JSON.stringify(result.updaterFeed,null,2),{flag:'wx'});}
 console.log(JSON.stringify({output:out,mode,version:record.version,channel:record.channel,publication_verified:result.release?.publication_verified??false}));
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url))main().catch(error=>{process.stderr.write(error.message+'\n');process.exitCode=1;});

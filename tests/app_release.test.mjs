import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,writeFile,readFile,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {newer,validateRecord,buildConfiguration,prepare,verifyArtifact,validatePublicRelease} from '../scripts/app-release.mjs';
import {fixture} from './fixtures/app_update_fixture.mjs';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
function record(f){return {schema:'lintel.app-release/1',version:'0.2.0',previous_version:'0.1.0',channel:'preview',source_revision:'a'.repeat(40),pubkey:f.pubkey,notes:'Synthetic test only',pub_date:'2026-10-09T00:00:00Z',platforms:{'darwin-aarch64':{archive_path:f.archive,signature_path:f.signaturePath,dmg_path:f.dmg,archive_name:'Lintel_0.2.0_aarch64.app.tar.gz',dmg_name:'Lintel_0.2.0_aarch64.dmg'}}};}
test('strict SemVer and frozen release configuration',()=>{
 assert.equal(newer('0.1.0-preview.1','0.1.0'),false);assert.equal(newer('0.1.0+build.2','0.1.0+build.1'),false);
 assert.equal(newer('0.2.0-preview.2','0.1.0'),true);assert.equal(newer('0.2.0-preview.10','0.2.0-preview.9'),true);
 assert.throws(()=>newer('18446744073709551616.0.0','0.1.0'),/u64/);
 assert.throws(()=>newer('0.2.0-preview.01','0.1.0'));assert.throws(()=>newer('01.2.0','0.1.0'));
});
test('real Ed25519 artifact/comment/version checks on synthetic archive',async()=>{
 const dir=await mkdtemp(path.join(tmpdir(),'lintel-update-signature-')),f=fixture(dir),r=record(f);
 assert.deepEqual(buildConfiguration(r).bundle,{targets:['app','dmg'],createUpdaterArtifacts:true});
 assert.equal(buildConfiguration(r).plugins.updater.requireSignedVersion,true);
 await verifyArtifact(f.archive,f.signature,f.pubkey,'0.2.0');
 await assert.rejects(verifyArtifact(f.archive,f.signature,f.pubkey,'0.3.0'),/Signed version/);
 const forged=Buffer.from(f.signature,'base64').toString().replace('version:0.2.0','version:0.3.0');
 await assert.rejects(verifyArtifact(f.archive,Buffer.from(forged).toString('base64'),f.pubkey,'0.3.0'),/comment signature/);
 const outputs=await prepare(r);assert.equal(outputs.release.status,'prepared_local');assert.equal(outputs.release.publication_verified,false);
 assert.equal(outputs.updaterFeed.platforms['darwin-aarch64'].signature,f.signature);assert.equal(outputs.release.downloads[0].url.endsWith('.dmg'),true);
 assert.equal(JSON.stringify(outputs).includes(dir),false,'Private artifact input paths leaked');
 const wrong=structuredClone(r);wrong.platforms={'darwin-x86_64':r.platforms['darwin-aarch64']};await assert.rejects(prepare(wrong),/Mach-O/);
 const invalid=structuredClone(r);invalid.channel='stable';invalid.version='0.2.0-preview.1';assert.throws(()=>validateRecord(invalid),/Stable/);
 invalid.version='0.2.0';invalid.pubkey='invalid';assert.throws(()=>validateRecord(invalid),/public key/);
 const unsafe=path.join(dir,'unsafe.app.tar.gz');
 const made=spawnSync('python3',['-c',`import tarfile,io,sys
with tarfile.open(sys.argv[1],'w:gz') as t:
 link=tarfile.TarInfo('Lintel.app/alias');link.type=tarfile.SYMTYPE;link.linkname='.';t.addfile(link)
 f=tarfile.TarInfo('Lintel.app/alias/child');f.size=1;t.addfile(f,io.BytesIO(b'x'))`,unsafe],{encoding:'utf8'});assert.equal(made.status,0);
 const inspected=spawnSync('python3',[path.join(repo,'scripts/inspect-app-archive.py'),unsafe,'0.2.0','darwin-aarch64'],{encoding:'utf8'});
 assert.notEqual(inspected.status,0);assert.match(inspected.stderr,/descends through a symlink/);
 const corrupted=path.join(dir,'corrupted.tar.gz');const bytes=await readFile(f.archive);bytes[20]^=1;await writeFile(corrupted,bytes);await assert.rejects(verifyArtifact(corrupted,f.signature,f.pubkey,'0.2.0'),/signature failed/);
});
test('site packages only explicit verified metadata, no binaries or invented Release',async()=>{
 const dir=await mkdtemp(path.join(tmpdir(),'lintel-app-site-')),f=fixture(path.join(dir,'fixture')),outputs=await prepare(record(f));
 const media=path.join(dir,'media');await mkdir(media);await writeFile(path.join(media,'lintel-intro.mp4'),'synthetic media');await writeFile(path.join(media,'lintel-film-poster.jpg'),'synthetic poster');
 const rec=path.join(dir,'release.json');await writeFile(rec,JSON.stringify(outputs.release));
 const pack=(out,extra=[])=>spawnSync(process.execPath,[path.join(repo,'scripts/prepare-site.mjs'),'--media-dir',media,'--out',path.join(dir,out),...extra],{cwd:repo,encoding:'utf8'});
 assert.equal(pack('plain').status,0);const plain=await readFile(path.join(dir,'plain/index.html'),'utf8');assert.match(plain,/没有正式 Release/);
 const refused=pack('unverified',['--app-release',rec]);assert.notEqual(refused.status,0);assert.match(refused.stderr,/Verify public release bytes/);
 const reviewed={...outputs.release,publication_verified:true,status:'public_bytes_verified',public_checked_at:'2026-10-09T00:00:00Z'};await writeFile(rec,JSON.stringify(reviewed));
 validatePublicRelease(reviewed);
 const leaked={...reviewed,private_path:dir};assert.throws(()=>validatePublicRelease(leaked),/fields/);
 const duplicate=structuredClone(reviewed);duplicate.downloads[0].platform='darwin-x86_64';assert.throws(()=>validatePublicRelease(duplicate),/platform/);
 assert.equal(pack('metadata',['--app-release',rec]).status,0);
 const html=await readFile(path.join(dir,'metadata/index.html'),'utf8');assert.match(html,/Apple Silicon/);assert.match(html,/公证状态：unverified/);
 const feed=JSON.parse(await readFile(path.join(dir,'metadata/updates/preview.json')));assert.equal(feed.version,'0.2.0');assert.deepEqual(feed.platforms,outputs.updaterFeed.platforms);
 const build=JSON.parse(await readFile(path.join(dir,'metadata/site-build.json')));assert.equal(build.files.some(f=>f.path.endsWith('.dmg')||f.path.endsWith('.app.tar.gz')),false);
 const headers=await readFile(path.join(dir,'metadata/_headers'),'utf8'),rules=new Map();let current;
 for(const line of headers.split('\n')){if(line.startsWith('/')){current=line;rules.set(current,[]);}else if(line.trim().startsWith('Cache-Control:'))rules.get(current).push(line.trim());}
 assert.deepEqual(rules.get('/*'),['Cache-Control: public, max-age=0, must-revalidate']);
 for(const route of ['/updates/*','/app-downloads.json'])assert.deepEqual(rules.get(route),['Cache-Control: no-cache'],'Feed/download metadata must have one explicit cache rule');
 assert.equal(JSON.stringify(build).includes(dir),false);assert.notEqual(pack('metadata').status,0,'Must not overwrite existing output');
});

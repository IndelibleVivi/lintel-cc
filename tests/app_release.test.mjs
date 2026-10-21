import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,writeFile,readFile,mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {newer,validateRecord,buildConfiguration,prepare,prepareManual,verifyArtifact,validatePublicRelease,validateManualRecord,validateManualPublicRelease} from '../scripts/app-release.mjs';
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
test('manual download records: strict shape, verified DMG metadata, no feed',async()=>{
 const dir=await mkdtemp(path.join(tmpdir(),'lintel-manual-release-')),f=fixture(path.join(dir,'fixture'));
 const manual=f=>({schema:'lintel.app-release-manual/1',version:'0.2.0',previous_version:'0.1.0',channel:'preview',source_revision:'a'.repeat(40),notes:'Synthetic test only',pub_date:'2026-10-09T00:00:00Z',platforms:{'darwin-aarch64':{dmg_path:f.dmg,dmg_name:'Lintel_0.2.0_aarch64.dmg'}}});
 const outputs=await prepareManual(manual(f));
 assert.equal(outputs.updaterFeed,undefined,'Manual prepare must not produce an updater feed');
 assert.equal(outputs.release.schema,'lintel.app-release-manual-public/1');assert.equal(outputs.release.status,'prepared_local');
 assert.equal(outputs.release.downloads.length,1);assert.match(outputs.release.downloads[0].url,/releases\/download\/v0\.2\.0\/Lintel_0\.2\.0_aarch64\.dmg$/);
 assert.equal(JSON.stringify(outputs).includes(dir),false,'Private artifact input paths leaked');
 assert.throws(()=>validateManualRecord({...manual(f),pubkey:'x'}),/fields/);
 const relative=manual(f);relative.platforms['darwin-aarch64'].dmg_path='relative.dmg';assert.throws(()=>validateManualRecord(relative),/absolute/);
 const renamed=manual(f);renamed.platforms['darwin-aarch64'].dmg_name='Lintel.dmg';assert.throws(()=>validateManualRecord(renamed),/version/);
 const missing=manual(f);missing.platforms['darwin-aarch64'].dmg_path=path.join(dir,'absent.dmg');await assert.rejects(prepareManual(missing),/ENOENT|regular artifact/);
 validateManualPublicRelease(outputs.release);
 const tampered=structuredClone(outputs.release);tampered.downloads[0].sha256='x'.repeat(64);assert.throws(()=>validateManualPublicRelease(tampered));
 const dup=structuredClone(outputs.release);dup.downloads.push({...dup.downloads[0]});assert.throws(()=>validateManualPublicRelease(dup),/platform/);
 const extra={...outputs.release,private_path:dir};assert.throws(()=>validateManualPublicRelease(extra),/fields/);
});
test('verify-public CLI dispatches prepared manual and signed records and rejects wrong bytes',async()=>{
 const dir=await mkdtemp(path.join(tmpdir(),'lintel-release-cli-')),f=fixture(path.join(dir,'fixture'));
 const signed=await prepare(record(f));
 const manual=await prepareManual({schema:'lintel.app-release-manual/1',version:'0.2.0',previous_version:'0.1.0',channel:'preview',source_revision:'a'.repeat(40),notes:'Synthetic CLI test only',pub_date:'2026-10-09T00:00:00Z',platforms:{'darwin-aarch64':{dmg_path:f.dmg,dmg_name:'Lintel_0.2.0_aarch64.dmg'}}});
 // Exercise the real child-process entrypoint without any public requests.
 // Each finite release URL maps to its matching synthetic artifact bytes.
 const preload=path.join(dir,'synthetic-fetch.mjs');
 const mapping=Object.fromEntries([...Object.values(signed.release.platforms).map(a=>[a.url,f.archive]),...signed.release.downloads.map(a=>[a.url,f.dmg])]);
 await writeFile(preload,`import {readFileSync} from 'node:fs';const files=${JSON.stringify(mapping)};globalThis.fetch=async url=>{if(!Object.hasOwn(files,url))throw new Error('Unexpected synthetic URL');return {ok:true,url,body:[readFileSync(files[url])]};};`);
 for(const [name,release] of [['manual',manual.release],['signed',signed.release]]){
  const input=path.join(dir,`${name}.json`),output=path.join(dir,`${name}-verified`);
  await writeFile(input,JSON.stringify(release));
  const run=out=>spawnSync(process.execPath,['--import',preload,path.join(repo,'scripts/app-release.mjs'),'verify-public','--record',input,'--out',out],{cwd:repo,encoding:'utf8',timeout:15000});
  const result=run(output);assert.equal(result.status,0,result.stderr);
  const verified=JSON.parse(await readFile(path.join(output,'release.json')));
  assert.equal(verified.schema,release.schema);assert.equal(verified.status,'public_bytes_verified');assert.equal(verified.publication_verified,true);assert.ok(verified.public_checked_at);
  const names=await(await import('node:fs/promises')).readdir(output);assert.deepEqual(names,['release.json'],'Verification alone must not activate an updater feed');
  const changed=structuredClone(release);changed.downloads[0].sha256='0'.repeat(64);await writeFile(input,JSON.stringify(changed));
  const refused=run(path.join(dir,`${name}-wrong`));assert.notEqual(refused.status,0);assert.match(refused.stderr,/digest differs/);
 }
});
test('site projects manual downloads without any updater feed',async()=>{
 const dir=await mkdtemp(path.join(tmpdir(),'lintel-manual-site-')),f=fixture(path.join(dir,'fixture'));
 const outputs=await prepareManual({schema:'lintel.app-release-manual/1',version:'0.2.0',previous_version:'0.1.0',channel:'preview',source_revision:'a'.repeat(40),notes:'Synthetic test only',pub_date:'2026-10-09T00:00:00Z',platforms:{'darwin-aarch64':{dmg_path:f.dmg,dmg_name:'Lintel_0.2.0_aarch64.dmg'}}});
 const media=path.join(dir,'media');await mkdir(media);await writeFile(path.join(media,'lintel-intro.mp4'),'synthetic media');await writeFile(path.join(media,'lintel-film-poster.jpg'),'synthetic poster');
 const rec=path.join(dir,'release.json');await writeFile(rec,JSON.stringify(outputs.release));
 const pack=(out,extra=[])=>spawnSync(process.execPath,[path.join(repo,'scripts/prepare-site.mjs'),'--media-dir',media,'--out',path.join(dir,out),...extra],{cwd:repo,encoding:'utf8'});
 const refused=pack('unverified',['--app-manual',rec]);assert.notEqual(refused.status,0);assert.match(refused.stderr,/Verify public release bytes/);
 const reviewed={...outputs.release,publication_verified:true,status:'public_bytes_verified',public_checked_at:'2026-10-09T00:00:00Z'};await writeFile(rec,JSON.stringify(reviewed));
 assert.equal(pack('manual',['--app-manual',rec]).status,0);
 const html=await readFile(path.join(dir,'manual/index.html'),'utf8');assert.match(html,/Apple Silicon/);assert.match(html,/未启用 App 内更新/);assert.match(html,/公证状态：unverified/);
 const en=await readFile(path.join(dir,'manual/en/index.html'),'utf8');assert.match(en,/In-app updates are not enabled/);
 const names=(await import('node:fs/promises')).readdir;assert.equal((await names(path.join(dir,'manual'))).includes('updates'),false,'Manual packaging must not write an updater feed');
 const build=JSON.parse(await readFile(path.join(dir,'manual/site-build.json')));assert.match(build.kind,/no updater feed/);assert.deepEqual(build.app_release_versions,[{channel:'preview',version:'0.2.0'}]);
 const downloads=JSON.parse(await readFile(path.join(dir,'manual/app-downloads.json')));assert.equal(downloads.releases[0].schema,'lintel.app-release-manual-public/1');
 const downloadPage=await readFile(path.join(dir,'manual/download/index.html'),'utf8');
 const downloadEn=await readFile(path.join(dir,'manual/en/download/index.html'),'utf8');
 const releasesPage=await readFile(path.join(dir,'manual/releases/index.html'),'utf8');
 for(const page of [downloadPage,downloadEn,releasesPage]){
  assert.match(page,/data-download-app="verified"/);
  assert.match(page,/data-release-version="0\.2\.0"/);
  assert.match(page,/\b[a-f0-9]{64}\b/,'verified page carries the asset digest');
  assert.match(page,/Manual download|仅手动|notarization|公证|In-app updates|未启用 App 内更新/);
  assert.ok(page.includes(outputs.release.source_revision));
  assert.ok(page.includes('Synthetic test only'),'release notes remain visible');
  assert.ok(page.includes('2026-10-09T00:00:00Z'),'the declared record date remains visible');
  assert.doesNotMatch(page,/现在还没有公开二进制|尚无公开 App 下载|没有一个可下载的正式 App Release|there is no public App binary/,'verified downloads do not retain unavailable page claims');
 }
 for(const page of [downloadPage,releasesPage])assert.match(page,/Apple Silicon/);
 assert.match(downloadEn,/In-app updates are not enabled/);
 const signedRec=path.join(dir,'signed.json');await writeFile(signedRec,JSON.stringify({...reviewed,schema:'lintel.app-release/1'}));
 assert.notEqual(pack('mixed',['--app-release',signedRec,'--app-manual',rec]).status,0,'Signed and manual download stories must not mix');
});
test('site packages only explicit verified metadata, no binaries or invented Release',async()=>{
 const dir=await mkdtemp(path.join(tmpdir(),'lintel-app-site-')),f=fixture(path.join(dir,'fixture')),outputs=await prepare(record(f));
 const media=path.join(dir,'media');await mkdir(media);await writeFile(path.join(media,'lintel-intro.mp4'),'synthetic media');await writeFile(path.join(media,'lintel-film-poster.jpg'),'synthetic poster');
 const rec=path.join(dir,'release.json');await writeFile(rec,JSON.stringify(outputs.release));
 const pack=(out,extra=[])=>spawnSync(process.execPath,[path.join(repo,'scripts/prepare-site.mjs'),'--media-dir',media,'--out',path.join(dir,out),...extra],{cwd:repo,encoding:'utf8'});
 assert.equal(pack('plain').status,0);const plain=await readFile(path.join(dir,'plain/index.html'),'utf8');assert.match(plain,/没有正式 Release/);
 const plainBuild=JSON.parse(await readFile(path.join(dir,'plain/site-build.json')));
 assert.equal(plainBuild.files.length,65);
 assert.deepEqual(await readFile(path.join(dir,'plain/assets/favicon.png')),await readFile(path.join(repo,'apps/desktop/src-tauri/icons/128x128.png')),'payload preserves the canonical raster identity export');
 assert.equal(plainBuild.locales.length,40);
 assert.equal(plainBuild.files.filter(file=>file.path==='index.html'||file.path.endsWith('/index.html')).length,40);
 assert.deepEqual(await readFile(path.join(dir,'plain/sitemap.xml')),await readFile(path.join(repo,'apps/site/sitemap.xml')),'payload sitemap matches the canonical generated artifact');
 const packagedSitemap=await readFile(path.join(dir,'plain/sitemap.xml'),'utf8');
 assert.equal([...packagedSitemap.matchAll(/<loc>/g)].length,40);
 assert.equal([...packagedSitemap.matchAll(/hreflang="x-default"/g)].length,40);
 const packagedGuide=await readFile(path.join(dir,'plain/en/guide/start-here/index.html'),'utf8');
 assert.match(packagedGuide,/<meta property="og:site_name" content="Lintel">/);
 assert.match(packagedGuide,/<meta name="twitter:title" content="First use · Lintel">/);
 assert.match(packagedGuide,/<meta name="description" content="Start with one synthetic package: archive it, approve the task and read it again.">/);
 assert.equal(plainBuild.files.some(file=>/\/(?:lintel-intro\.mp4|lintel-film-poster\.jpg)$/.test(file.path)),true);
 assert.equal(plainBuild.files.some(file=>/(?:^|\/)\.\.\/|(?:^|\/)qa\/|\.app\.tar\.gz$|\.dmg$/.test(file.path)),false);
 assert.equal(JSON.stringify(plainBuild).includes(repo),false);
 for(const name of ['download/index.html','en/download/index.html','releases/index.html','en/releases/index.html','guide/start-here/index.html','en/guide/start-here/index.html','guide/operator-guide/index.html','en/guide/operator-guide/index.html']){
  assert.ok(plainBuild.files.some(file=>file.path===name),`plain payload includes ${name}`);
 }
 const plainDownload=await readFile(path.join(dir,'plain/download/index.html'),'utf8');
 assert.match(plainDownload,/data-download-app="unavailable"/);
 assert.doesNotMatch(plainDownload,/releases\/download\/v|\b[a-f0-9]{64}\b/);
 assert.match(plainDownload,/0\.1\.0 Preview/);
 const plainArticle=await readFile(path.join(dir,'plain/guide/operator-guide/index.html'),'utf8');
 assert.match(plainArticle,/Lintel 操作指南/);
 assert.match(plainArticle,/data-guide-document="operator-guide"/);
 const refused=pack('unverified',['--app-release',rec]);assert.notEqual(refused.status,0);assert.match(refused.stderr,/Verify public release bytes/);
 const reviewed={...outputs.release,publication_verified:true,status:'public_bytes_verified',public_checked_at:'2026-10-09T00:00:00Z'};await writeFile(rec,JSON.stringify(reviewed));
 validatePublicRelease(reviewed);
 const leaked={...reviewed,private_path:dir};assert.throws(()=>validatePublicRelease(leaked),/fields/);
 const duplicate=structuredClone(reviewed);duplicate.downloads[0].platform='darwin-x86_64';assert.throws(()=>validatePublicRelease(duplicate),/platform/);
 assert.equal(pack('metadata',['--app-release',rec]).status,0);
 const html=await readFile(path.join(dir,'metadata/index.html'),'utf8');assert.match(html,/Apple Silicon/);assert.match(html,/公证状态：unverified/);
 const verifiedDownload=await readFile(path.join(dir,'metadata/download/index.html'),'utf8');
 const verifiedReleases=await readFile(path.join(dir,'metadata/en/releases/index.html'),'utf8');
 for(const page of [verifiedDownload,verifiedReleases]){
  assert.match(page,/data-download-app="verified"/);
  assert.match(page,/data-release-version="0\.2\.0"/);
  assert.ok(page.includes(outputs.release.source_revision));
 }
 const feed=JSON.parse(await readFile(path.join(dir,'metadata/updates/preview.json')));assert.equal(feed.version,'0.2.0');assert.deepEqual(feed.platforms,outputs.updaterFeed.platforms);
 const build=JSON.parse(await readFile(path.join(dir,'metadata/site-build.json')));assert.equal(build.files.some(f=>f.path.endsWith('.dmg')||f.path.endsWith('.app.tar.gz')),false);
 const headers=await readFile(path.join(dir,'metadata/_headers'),'utf8'),rules=new Map();let current;
 for(const line of headers.split('\n')){if(line.startsWith('/')){current=line;rules.set(current,[]);}else if(line.trim().startsWith('Cache-Control:'))rules.get(current).push(line.trim());}
 assert.deepEqual(rules.get('/*'),['Cache-Control: public, max-age=0, must-revalidate']);
 for(const route of ['/updates/*','/app-downloads.json'])assert.deepEqual(rules.get(route),['Cache-Control: no-cache'],'Feed/download metadata must have one explicit cache rule');
 assert.equal(JSON.stringify(build).includes(dir),false);assert.notEqual(pack('metadata').status,0,'Must not overwrite existing output');
});

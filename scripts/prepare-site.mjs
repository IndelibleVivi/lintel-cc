// Build a finite public static payload. No deployment, account access, or media generation.
import {readFile,writeFile,mkdir,lstat,copyFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {validatePublicRelease,validateManualPublicRelease} from './app-release.mjs';
import {checkSiteLanguages} from './prepare-site-languages.mjs';
import {RELEASE_COPY,releaseSecurityCopy} from '../apps/site/site-copy.mjs';
import {SITE_ROUTES,localePath} from '../apps/site/site-routes.mjs';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
let mediaDir,out;const appRecords=[];const manualRecords=[];
const args=process.argv.slice(2);
for(let i=0;i<args.length;i++){
  if(args[i]==='--media-dir')mediaDir=args[++i];
  else if(args[i]==='--out')out=args[++i];
  else if(args[i]==='--app-release')appRecords.push(args[++i]);
  else if(args[i]==='--app-manual')manualRecords.push(args[++i]);
  else throw new Error('Usage: prepare-site.mjs --media-dir DIRECTORY --out ABSENT_DIRECTORY');
}
if(!mediaDir||!out)throw new Error('Both --media-dir and --out are required');
mediaDir=path.resolve(mediaDir);out=path.resolve(out);
try{await lstat(out);throw new Error('Output must be absent')}catch(e){if(e.code!=='ENOENT')throw e;}
if(out.startsWith(repo+path.sep)&&spawnSync('git',['check-ignore','-q',out],{cwd:repo}).status!==0)throw new Error('Repository output must be ignored; use candidate-packages');
const site=path.join(repo,'apps/site');
await checkSiteLanguages();
const escape=value=>String(value).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
function releaseDirectoryBlock(release,language,updater){
 const links=release.downloads.map(a=>{
  const platform=a.platform==='darwin-aarch64'?'Apple Silicon':'Intel';
  return `<a class="button primary" href="${escape(a.url)}">macOS ${platform} · ${escape(release.version)} DMG <span aria-hidden="true">↗</span></a>`;
 }).join('');
 const security=`<span data-site-release-security data-developer-id="${escape(release.apple_security.developer_id)}" data-notarization="${escape(release.apple_security.notarization)}">${escape(releaseSecurityCopy(language,release.apple_security.developer_id,release.apple_security.notarization))}</span>`;
 const facts=release.downloads.map(a=>`<div><dt>${a.platform==='darwin-aarch64'?'Apple Silicon':'Intel'} DMG</dt><dd>${escape(a.bytes)} bytes · <code>${escape(a.sha256)}</code></dd></div>`).join('');
 const source=`<div><dt>Source revision</dt><dd><code>${escape(release.source_revision)}</code></dd></div><div><dt>${language==='en'?'Record date':'记录日期'}</dt><dd><time datetime="${escape(release.pub_date)}">${escape(release.pub_date)}</time></dd></div>`;
 const update=`<div><dt>${language==='en'?'Updates':'更新'}</dt><dd>${updater?(language==='en'?'Signed updater feed':'已签名 updater feed'):(language==='en'?'Manual download only; no updater feed':'仅手动下载；没有 updater feed')}</dd></div>`;
 return `<!-- lintel-app-release:start --><div class="release-facts" data-download-app="verified" data-release-channel="${escape(release.channel)}" data-release-version="${escape(release.version)}"><p class="release-state"><span>${release.channel==='stable'?'STABLE':'PREVIEW'}</span><strong>${escape(release.version)}</strong></p><div class="download-choice"><h3>${language==='en'?'Verified manual download record':'已核对的手动下载记录'}</h3><div class="trial-actions">${links}</div><p>${updater?(language==='en'?'These records also include a signed updater feed.':'这些记录同时包含已签名 updater feed。'):RELEASE_COPY.manual[language]}</p></div><dl class="fact-list">${facts}${source}${update}</dl><p class="release-notes">${escape(release.notes)}</p><p class="route-note">${security}</p><p class="route-note">${RELEASE_COPY.privacy[language]}<a href="${language==='en'?'/en/guide/app-updates/':'/guide/app-updates/'}">${language==='en'?'Installation and updates':'安装与更新'} ↗</a></p></div><!-- lintel-app-release:end -->`;
}
const routeFiles=[];
for(const route of SITE_ROUTES){
  const zh=localePath(route.path,'zh'),en=localePath(route.path,'en');
  routeFiles.push([zh,zh==='/'?path.join(repo,'apps/site/index.html'):path.join(repo,'apps/site',...zh.split('/').filter(Boolean),'index.html')]);
  routeFiles.push([en,zh==='/'?path.join(repo,'apps/site/en/index.html'):path.join(repo,'apps/site/en',...zh.split('/').filter(Boolean),'index.html')]);
}
const names=['styles.css','site.mjs','site-copy.mjs','site-language.mjs','site-routes.mjs','site-docs.mjs','film.mjs','film.vtt','site-world.mjs','workflow-trail.mjs','clawd-game.mjs','clawd-game.css','404.html','_headers','robots.txt','sitemap.xml','assets/favicon.svg','assets/favicon.png','assets/lintel-landscape.png','assets/lintel-landscape-night.png','assets/lintel-keep.png','assets/lintel-crossing.png'];
const inputs=[...routeFiles.map(([name,file])=>[name==='/'?'index.html':name==='/en/'?'en/index.html':`${name.slice(1)}index.html`,file]),...names.map(name=>[name,path.join(site,name)])];
inputs.push(['assets/lintel-social-preview.png',path.join(repo,'assets/preview/lintel-social-preview.png')]);
for(const name of ['lintel-intro.mp4','lintel-film-poster.jpg'])inputs.push(['media/'+name,path.join(mediaDir,name)]);
const files=[];
for(const [name,file] of inputs){
  const s=await lstat(file);
  if(!s.isFile()||s.size===0||s.size>25*1024*1024)throw new Error('Expected a nonempty regular public asset below the 25 MiB limit: '+name);
  files.push({path:name,bytes:s.size});
}
const html=await readFile(path.join(site,'index.html'),'utf8');
if(!html.includes('https://lintel.page/')||!html.includes('media/lintel-intro.mp4'))throw new Error('Review changed site identity or film paths before packaging');
const head=spawnSync('git',['rev-parse','HEAD'],{cwd:repo,encoding:'utf8'}),status=spawnSync('git',['status','--porcelain'],{cwd:repo,encoding:'utf8'});
if(head.status!==0||status.status!==0)throw new Error('Cannot declare current source revision');
const releases=[];
for(const file of appRecords){
 const s=await lstat(file);if(!s.isFile()||s.size>64*1024)throw new Error('Expected a bounded public release record');
 const release=validatePublicRelease(JSON.parse(await readFile(file,'utf8')));
 if(release.publication_verified!==true||release.status!=='public_bytes_verified'||!release.public_checked_at)throw new Error('Verify public release bytes before enabling website download/feed');
 if(releases.some(previous=>previous.channel===release.channel))throw new Error('Duplicate release channel');
 releases.push(release);
}
const manualReleases=[];
for(const file of manualRecords){
 const s=await lstat(file);if(!s.isFile()||s.size>64*1024)throw new Error('Expected a bounded manual release record');
 const release=validateManualPublicRelease(JSON.parse(await readFile(file,'utf8')));
 if(release.publication_verified!==true||release.status!=='public_bytes_verified'||!release.public_checked_at)throw new Error('Verify public release bytes before enabling website download');
 if(manualReleases.some(previous=>previous.channel===release.channel))throw new Error('Duplicate manual release channel');
 manualReleases.push(release);
}
if(releases.length&&manualReleases.length)throw new Error('Choose one download story: signed updater release or manual download, not both');
await mkdir(out,{recursive:true});
for(const [name,file] of inputs){const target=path.join(out,name);await mkdir(path.dirname(target),{recursive:true});await copyFile(file,target);}
if(releases.length){
 const primary=releases.find(r=>r.channel==='stable')??releases[0];
 const links=primary.downloads.map(a=>`<a class="button primary" href="${escape(a.url)}">macOS ${a.platform==='darwin-aarch64'?'Apple Silicon':'Intel'} · ${escape(primary.version)} DMG <span aria-hidden="true">↗</span></a>`).join('');
 if(!html.includes('<!-- lintel-app-release:start -->')||!html.includes('<!-- lintel-app-release:end -->'))throw new Error('Missing canonical App release block');
 for(const [language,name] of [['zh','index.html'],['en','en/index.html']]){
  const copy=key=>`<span data-site-release-copy="${key}">${escape(RELEASE_COPY[key][language])}</span>`;
  const security=`<span data-site-release-security data-developer-id="${escape(primary.apple_security.developer_id)}" data-notarization="${escape(primary.apple_security.notarization)}">${escape(releaseSecurityCopy(language,primary.apple_security.developer_id,primary.apple_security.notarization))}</span>`;
  const block=`<!-- lintel-app-release:start --><div class="preview-note"><span>${primary.channel==='stable'?'STABLE':'PREVIEW'} / ${escape(primary.version)}</span><div class="trial-actions">${links}</div><p>${copy('installation')} ${security}</p><p>${copy('privacy')}<a href="https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/app-updates${language==='en'?'.en':''}.md">${copy('guide')} ↗</a></p></div><!-- lintel-app-release:end -->`;
  const source=language==='zh'?html:await readFile(path.join(site,name),'utf8');
  const projected=source.replace(/<!-- lintel-app-release:start -->[\s\S]*?<!-- lintel-app-release:end -->/,block).replace(/0\.1\.0 PREVIEW/g,`${escape(primary.version)} ${primary.channel.toUpperCase()}`);
  await writeFile(path.join(out,name),projected);files.find(f=>f.path===name).bytes=Buffer.byteLength(projected);
 }
 await mkdir(path.join(out,'updates'));
 for(const release of releases){
  const platformFeed=Object.fromEntries(Object.entries(release.platforms).map(([p,a])=>[p,{url:a.url,signature:a.signature}]));
  const value={version:release.version,notes:release.notes,pub_date:release.pub_date,platforms:platformFeed};
  const name=`updates/${release.channel}.json`,bytes=JSON.stringify(value,null,2);await writeFile(path.join(out,name),bytes,{flag:'wx'});files.push({path:name,bytes:Buffer.byteLength(bytes)});
 }
 const data=JSON.stringify({schema:'lintel.app-downloads/1',releases},null,2);await writeFile(path.join(out,'app-downloads.json'),data,{flag:'wx'});files.push({path:'app-downloads.json',bytes:Buffer.byteLength(data)});
 for(const [language,routePath] of [['zh','download/'],['en','en/download/'],['zh','releases/'],['en','en/releases/']]){
  const target=path.join(out,routePath,'index.html');
  const source=await readFile(target,'utf8');
  const block=releaseDirectoryBlock(primary,language,true);
  await writeFile(target,source.replace(/<!-- lintel-app-release:start -->[\s\S]*?<!-- lintel-app-release:end -->/,block));
  const item=files.find(file=>file.path===`${routePath}index.html`);item.bytes=(await readFile(target)).length;
 }
}
if(manualReleases.length){
 const primary=manualReleases.find(r=>r.channel==='stable')??manualReleases[0];
 const links=primary.downloads.map(a=>`<a class="button primary" href="${escape(a.url)}">macOS ${a.platform==='darwin-aarch64'?'Apple Silicon':'Intel'} · ${escape(primary.version)} DMG <span aria-hidden="true">↗</span></a>`).join('');
 if(!html.includes('<!-- lintel-app-release:start -->')||!html.includes('<!-- lintel-app-release:end -->'))throw new Error('Missing canonical App release block');
 for(const [language,name] of [['zh','index.html'],['en','en/index.html']]){
  const copy=key=>`<span data-site-release-copy="${key}">${escape(RELEASE_COPY[key][language])}</span>`;
  const security=`<span data-site-release-security data-developer-id="${escape(primary.apple_security.developer_id)}" data-notarization="${escape(primary.apple_security.notarization)}">${escape(releaseSecurityCopy(language,primary.apple_security.developer_id,primary.apple_security.notarization))}</span>`;
  const block=`<!-- lintel-app-release:start --><div class="preview-note"><span>${primary.channel==='stable'?'STABLE':'PREVIEW'} / ${escape(primary.version)}</span><div class="trial-actions">${links}</div><p>${copy('installation')} ${copy('manual')} ${security}</p><p>${copy('privacy')}<a href="https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/app-updates${language==='en'?'.en':''}.md">${copy('guide')} ↗</a></p></div><!-- lintel-app-release:end -->`;
  const source=language==='zh'?html:await readFile(path.join(site,name),'utf8');
  const projected=source.replace(/<!-- lintel-app-release:start -->[\s\S]*?<!-- lintel-app-release:end -->/,block).replace(/0\.1\.0 PREVIEW/g,`${escape(primary.version)} ${primary.channel.toUpperCase()}`);
  await writeFile(path.join(out,name),projected);files.find(f=>f.path===name).bytes=Buffer.byteLength(projected);
 }
 const data=JSON.stringify({schema:'lintel.app-downloads/1',releases:manualReleases},null,2);await writeFile(path.join(out,'app-downloads.json'),data,{flag:'wx'});files.push({path:'app-downloads.json',bytes:Buffer.byteLength(data)});
 for(const [language,routePath] of [['zh','download/'],['en','en/download/'],['zh','releases/'],['en','en/releases/']]){
  const target=path.join(out,routePath,'index.html');
  const source=await readFile(target,'utf8');
  const block=releaseDirectoryBlock(primary,language,false);
  await writeFile(target,source.replace(/<!-- lintel-app-release:start -->[\s\S]*?<!-- lintel-app-release:end -->/,block));
  const item=files.find(file=>file.path===`${routePath}index.html`);item.bytes=(await readFile(target)).length;
 }
}
const locales=[{language:'zh-CN',path:'/'},{language:'en',path:'/en/'},...SITE_ROUTES.filter(route=>route.path!=='/').flatMap(route=>[{language:'zh-CN',path:route.path},{language:'en',path:localePath(route.path,'en')}])];
const manifest={schema:'lintel.site-build/1',source_revision:head.stdout.trim(),source_dirty:status.stdout.trim().length>0,created_at:new Date().toISOString(),origin:'https://lintel.page',locales,kind:releases.length?'Static website with explicitly verified App release metadata; binaries stay on GitHub':manualReleases.length?'Static website with explicitly verified manual App download metadata; binaries stay on GitHub; no updater feed':'Static product website; concept movie; no native operations or App Release',app_release_versions:[...releases,...manualReleases].map(r=>({channel:r.channel,version:r.version})),files};
await writeFile(path.join(out,'site-build.json'),JSON.stringify(manifest,null,2),{flag:'wx'});
console.log(JSON.stringify({output:out,source_revision:manifest.source_revision,source_dirty:manifest.source_dirty,files:files.length,total_bytes:files.reduce((n,f)=>n+f.bytes,0)},null,2));

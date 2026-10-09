// Build a finite public static payload. No deployment, account access, or media generation.
import {readFile,writeFile,mkdir,lstat,copyFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {validatePublicRelease} from './app-release.mjs';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
let mediaDir,out;const appRecords=[];
const args=process.argv.slice(2);
for(let i=0;i<args.length;i++){
  if(args[i]==='--media-dir')mediaDir=args[++i];
  else if(args[i]==='--out')out=args[++i];
  else if(args[i]==='--app-release')appRecords.push(args[++i]);
  else throw new Error('Usage: prepare-site.mjs --media-dir DIRECTORY --out ABSENT_DIRECTORY');
}
if(!mediaDir||!out)throw new Error('Both --media-dir and --out are required');
mediaDir=path.resolve(mediaDir);out=path.resolve(out);
try{await lstat(out);throw new Error('Output must be absent')}catch(e){if(e.code!=='ENOENT')throw e;}
if(out.startsWith(repo+path.sep)&&spawnSync('git',['check-ignore','-q',out],{cwd:repo}).status!==0)throw new Error('Repository output must be ignored; use candidate-packages');
const site=path.join(repo,'apps/site');
const names=['index.html','styles.css','site.mjs','film.mjs','film.vtt','site-world.mjs','workflow-trail.mjs','clawd-game.mjs','clawd-game.css','404.html','_headers','robots.txt','sitemap.xml','assets/favicon.svg','assets/lintel-landscape.png','assets/lintel-landscape-night.png','assets/lintel-keep.png','assets/lintel-crossing.png'];
const inputs=names.map(name=>[name,path.join(site,name)]);
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
await mkdir(out,{recursive:true});
for(const [name,file] of inputs){const target=path.join(out,name);await mkdir(path.dirname(target),{recursive:true});await copyFile(file,target);}
if(releases.length){
 const escape=value=>String(value).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
 const primary=releases.find(r=>r.channel==='stable')??releases[0];
 const links=primary.downloads.map(a=>`<a class="button primary" href="${escape(a.url)}">macOS ${a.platform==='darwin-aarch64'?'Apple Silicon':'Intel'} · ${escape(primary.version)} DMG <span aria-hidden="true">↗</span></a>`).join('');
 const block=`<!-- lintel-app-release:start --><div class="preview-note"><span>${primary.channel==='stable'?'STABLE':'PREVIEW'} / ${escape(primary.version)}</span><div class="trial-actions">${links}</div><p>初次安装使用 DMG；已带 updater 的 App 在“设置与模块 → Lintel 版本与更新”检查。版本记录已核对公开二进制字节；Developer ID 与公证状态：${escape(primary.apple_security.developer_id)} / ${escape(primary.apple_security.notarization)}。更新签名不替代 Apple 信任验证。</p><p>网站不连接执行核心。无默认分析上传；托管方处理普通 HTTP 请求。<a href="https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/app-updates.md">安装与更新说明 ↗</a></p></div><!-- lintel-app-release:end -->`;
 if(!html.includes('<!-- lintel-app-release:start -->')||!html.includes('<!-- lintel-app-release:end -->'))throw new Error('Missing canonical App release block');
 const projected=html.replace(/<!-- lintel-app-release:start -->[\s\S]*?<!-- lintel-app-release:end -->/,block).replace(/0\.1\.0 PREVIEW/g,`${escape(primary.version)} ${primary.channel.toUpperCase()}`);
 await writeFile(path.join(out,'index.html'),projected);files.find(f=>f.path==='index.html').bytes=Buffer.byteLength(projected);
 await mkdir(path.join(out,'updates'));
 for(const release of releases){
  const platformFeed=Object.fromEntries(Object.entries(release.platforms).map(([p,a])=>[p,{url:a.url,signature:a.signature}]));
  const value={version:release.version,notes:release.notes,pub_date:release.pub_date,platforms:platformFeed};
  const name=`updates/${release.channel}.json`,bytes=JSON.stringify(value,null,2);await writeFile(path.join(out,name),bytes,{flag:'wx'});files.push({path:name,bytes:Buffer.byteLength(bytes)});
 }
 const data=JSON.stringify({schema:'lintel.app-downloads/1',releases},null,2);await writeFile(path.join(out,'app-downloads.json'),data,{flag:'wx'});files.push({path:'app-downloads.json',bytes:Buffer.byteLength(data)});
}
const manifest={schema:'lintel.site-build/1',source_revision:head.stdout.trim(),source_dirty:status.stdout.trim().length>0,created_at:new Date().toISOString(),origin:'https://lintel.page',kind:releases.length?'Static website with explicitly verified App release metadata; binaries stay on GitHub':'Static product website; concept movie; no native operations or App Release',app_release_versions:releases.map(r=>({channel:r.channel,version:r.version})),files};
await writeFile(path.join(out,'site-build.json'),JSON.stringify(manifest,null,2),{flag:'wx'});
console.log(JSON.stringify({output:out,source_revision:manifest.source_revision,source_dirty:manifest.source_dirty,files:files.length,total_bytes:files.reduce((n,f)=>n+f.bytes,0)},null,2));

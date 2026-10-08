// Build a finite public static payload. No deployment, account access, or media generation.
import {readFile,writeFile,mkdir,lstat,copyFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
let mediaDir,out;
const args=process.argv.slice(2);
for(let i=0;i<args.length;i++){
  if(args[i]==='--media-dir')mediaDir=args[++i];
  else if(args[i]==='--out')out=args[++i];
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
await mkdir(out,{recursive:true});
for(const [name,file] of inputs){const target=path.join(out,name);await mkdir(path.dirname(target),{recursive:true});await copyFile(file,target);}
const manifest={schema:'lintel.site-build/1',source_revision:head.stdout.trim(),source_dirty:status.stdout.trim().length>0,created_at:new Date().toISOString(),origin:'https://lintel.page',kind:'Static product website; concept movie; no native operations or App Release',files};
await writeFile(path.join(out,'site-build.json'),JSON.stringify(manifest,null,2),{flag:'wx'});
console.log(JSON.stringify({output:out,source_revision:manifest.source_revision,source_dirty:manifest.source_dirty,files:files.length,total_bytes:files.reduce((n,f)=>n+f.bytes,0)},null,2));

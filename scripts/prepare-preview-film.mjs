// Independent Remotion production entry. All supplied media and renders remain local.
import {readFile,writeFile,mkdir,access,copyFile} from 'node:fs/promises';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {createRequire} from 'node:module';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {BODY,clawdPose,newRun} from '../apps/site/clawd-game.mjs';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const args=process.argv.slice(2);let out,nativeDir,audioDir,stills=false,runtime=path.join(repo,'candidate-packages/film-runtime');
for(let i=0;i<args.length;i++){
  if(args[i]==='--out')out=args[++i];else if(args[i]==='--native-captures')nativeDir=args[++i];
  else if(args[i]==='--audio-dir')audioDir=args[++i];else if(args[i]==='--runtime')runtime=args[++i];else if(args[i]==='--stills')stills=true;
  else throw new Error('Usage: prepare-preview-film.mjs --out ABSENT_DIRECTORY [--runtime DIRECTORY] [--audio-dir DIRECTORY] [--native-captures DIRECTORY] [--stills]');
}
if(!out)throw new Error('--out must name an absent export directory');
if(!stills&&!audioDir)throw new Error('Supply the prepared local audio directory for MP4 export');
out=path.resolve(out);runtime=path.resolve(runtime);
try{await access(out);throw new Error(`Refusing to overwrite ${out}`)}catch(e){if(e.code!=='ENOENT')throw e;}
const require=createRequire(path.join(runtime,'package.json'));
const {bundle}=require('@remotion/bundler');const {openBrowser,selectComposition,renderStill,renderMedia}=require('@remotion/renderer');
const identity=path.join(repo,'apps/desktop/assets/identity/source');
const [word,mark,source]=await Promise.all([readFile(path.join(identity,'lintel-wordmark.svg'),'utf8'),readFile(path.join(identity,'lintel-mark.svg'),'utf8'),readFile(path.join(repo,'assets/preview/film.tsx'),'utf8')]);
const attribute=(source,key)=>{const m=source.match(new RegExp(`\\b${key}="([^"]+)"`)),n=Number(m?.[1]);if(!m||!Number.isFinite(n))throw new Error(`Invalid canonical ${key}`);return n;};
const foot=word.match(/<rect\b[^>]*\bid="wordmark-foot"[^>]*\/>/)?.[0];
if(!foot||!word.includes('wordmark-foot-cut')||(word.match(/<path\b/g)||[]).length!==1||(mark.match(/<rect\b/g)||[]).length!==4)throw new Error('Review changed canonical identity before film export');
const night=svg=>svg.replaceAll('#3d3d3a','#eae5da').replaceAll('#3D3D3A','#eae5da').replaceAll('#c16a47','#e0a485').replaceAll('#C16A47','#e0a485');
const media={audio:!!audioDir,word:{width:attribute(word,'width'),height:attribute(word,'height'),foot:Object.fromEntries(['x','y','width','height'].map(k=>[k,attribute(foot,k)]))},native:[]};
await mkdir(out);const publicDir=path.join(out,'public'),sourceDir=path.join(out,'source');await mkdir(publicDir);await mkdir(sourceDir);
await writeFile(path.join(publicDir,'word-night.svg'),night(word.replace(foot,foot.replace('/>',' opacity="0"/>'))),{flag:'wx'});
// The film carries the fourth rect as its moving cursor; the master mark remains unchanged.
let markRect=0;const animatedMark=mark.replace(/<rect\b[^>]*\/>/g,rect=>++markRect===4?rect.replace('/>',' opacity="0"/>'):rect);
await writeFile(path.join(publicDir,'mark-night.svg'),night(animatedMark).replace('viewBox="0 0 720 430"','width="2160" height="1290" viewBox="0 0 720 430"'),{flag:'wx'});
const nightPlate=await readFile(path.join(repo,'apps/site/assets/lintel-landscape-night.png'));
await writeFile(path.join(publicDir,'night-horizon.svg'),`<svg xmlns="http://www.w3.org/2000/svg" width="1774" height="660" viewBox="0 0 1774 660"><defs><linearGradient id="x"><stop stop-color="white" stop-opacity="0"/><stop offset=".13" stop-color="white"/><stop offset=".87" stop-color="white"/><stop offset="1" stop-color="white" stop-opacity="0"/></linearGradient><linearGradient id="y" x2="0" y2="1"><stop stop-color="white" stop-opacity="0"/><stop offset=".14" stop-color="white"/><stop offset=".8" stop-color="white"/><stop offset="1" stop-color="white" stop-opacity="0"/></linearGradient><mask id="horizontal"><rect width="1774" height="660" fill="url(#x)"/></mask><mask id="vertical"><rect width="1774" height="660" fill="url(#y)"/></mask></defs><g mask="url(#horizontal)"><image href="data:image/png;base64,${nightPlate.toString('base64')}" width="1774" height="887" mask="url(#vertical)"/></g></svg>`,{flag:'wx'});
// Reuse the existing connected sprite and eyes, rather than magnifying a tiny raster crop.
const eyes=clawdPose(newRun(),true).eyes.map(([x,y,w,h])=>`<rect x="${x}" y="${y}" width="${w}" height="${h}" fill="#252320"/>`).join('');
await writeFile(path.join(publicDir,'clawd-crossing.svg'),`<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="1000" viewBox="0 0 80 50"><path d="${BODY}" fill="#e0a485"/>${eyes}</svg>`,{flag:'wx'});
if(audioDir){for(const file of ['mix.wav','mix-voice.wav','audio-manifest.json'])await copyFile(path.join(path.resolve(audioDir),file),path.join(publicDir,file));}
if(nativeDir){
  const shots=[['01-home.png',2,'从安静的首页开始'],['02-task-choice.png',3,'选择要完成的任务'],['05-three-selected-settled.png',4,'精确选择三份原件'],['06-archive-preview.png',5,'审阅冻结范围与实际步骤'],['08-ready-to-approve.png',3,'输入临时口令，明确批准'],['09-native-execution.png',2,'真实执行中的反馈'],['10-native-result.png',4,'核对原任务的完成结果'],['14-native-session-reader.png',7,'解锁独立工作包，按需阅读']];
  for(const [file,seconds,caption]of shots){await copyFile(path.join(path.resolve(nativeDir),file),path.join(publicDir,file));media.native.push({file,seconds,caption});}
}
await writeFile(path.join(sourceDir,'film.tsx'),source,{flag:'wx'});await writeFile(path.join(sourceDir,'film-media.json'),JSON.stringify(media),{flag:'wx'});
const revision=spawnSync('git',['rev-parse','HEAD'],{cwd:repo,encoding:'utf8'}).stdout.trim(),dirty=spawnSync('git',['status','--porcelain'],{cwd:repo,encoding:'utf8'}).stdout.trim().length>0;
const serveUrl=await bundle({entryPoint:path.join(sourceDir,'film.tsx'),outDir:path.join(out,'bundle'),publicDir,enableCaching:false,rootDir:runtime,webpackOverride:config=>({...config,resolve:{...config.resolve,modules:[path.join(runtime,'node_modules'),...(config.resolve?.modules||[])]}})});
const {chromium}=await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE||path.join(repo,'extensions/browser/node_modules/playwright/index.mjs')).href);
const chromiumOptions={gl:'angle'};
const openPreviewBrowser=()=>openBrowser('chrome',{browserExecutable:chromium.executablePath(),chromiumOptions});
let browser=await openPreviewBrowser();
const errors=[];const common={serveUrl,puppeteerInstance:browser,chromiumOptions,onBrowserLog:log=>{if(log.type==='error')errors.push(log.text);}};
try{
  const intro=await selectComposition({...common,id:'LintelIntro'});
  for(const [t,name]of [[1.6,'opening'],[7.2,'keep'],[10.7,'preview'],[12.3,'verify'],[15.6,'crossing'],[19.2,'closing']])await renderStill({...common,composition:intro,frame:Math.round(t*intro.fps),output:path.join(out,`${name}.png`),overwrite:false});
  await copyFile(path.join(out,'closing.png'),path.join(out,'poster.png'));
  if(!stills){
    // Export uses a fresh navigation context after the separate still-review server has closed.
    await browser.close({silent:true});browser=await openPreviewBrowser();common.puppeteerInstance=browser;
    const renders=[['LintelIntro','lintel-intro.mp4']];
    if(media.native.length)renders.push(['LintelNative','lintel-native-trial.mp4']);
    for(const [id,file]of renders){
      const composition=await selectComposition({...common,id});let progress=-1;
      await renderMedia({...common,composition,codec:'h264',crf:18,audioCodec:'aac',audioBitrate:'192k',pixelFormat:'yuv420p',concurrency:4,outputLocation:path.join(out,file),overwrite:false,onProgress:p=>{const n=Math.floor(p.progress*10);if(n>progress){progress=n;console.log(`${id}: ${Math.round(p.progress*100)}%`);}}});
    }
    // The voiced variant uses the exact same encoded frames; replace only the audio track.
    const compositor=require.resolve('@remotion/compositor-darwin-arm64/package.json');
    const mux=spawnSync(path.join(path.dirname(compositor),'ffmpeg'),['-hide_banner','-loglevel','error','-n','-i',path.join(out,'lintel-intro.mp4'),'-i',path.join(publicDir,'mix-voice.wav'),'-map','0:v:0','-map','1:a:0','-c:v','copy','-c:a','aac','-b:a','192k','-t','20','-movflags','+faststart',path.join(out,'lintel-intro-voice.mp4')],{encoding:'utf8',env:{...process.env,DYLD_LIBRARY_PATH:path.dirname(compositor)}});
    if(mux.status!==0)throw new Error(`Voice mux failed: ${mux.stderr||mux.error}`);
  }
  if(errors.length)throw new Error(errors.join('\n'));
  const manifest={schema:'lintel.preview-film/2',source_revision:revision,source_dirty:dirty,created_at:new Date().toISOString(),engine:'Remotion 4.0.534 / Three.js',width:1920,height:1080,fps:60,duration:20,audio:audioDir?'Original Tone.js score/SFX; local Kokoro speech; retained independent mixes':'none (still review)',identity:['apps/desktop/assets/identity/source/lintel-wordmark.svg','apps/desktop/assets/identity/source/lintel-mark.svg'],landscapes:['Authored spatial character terrain','apps/site/assets/lintel-landscape-night.png (upper horizon only)'],clawd_source:'apps/site/clawd-game.mjs BODY / clawdPose; connected silhouette from apps/desktop/src/Clawd.tsx',native:media.native.length?{duration:30,width:1600,height:1000,provenance:'Caller-supplied real native screenshots; synthetic data; edited holds, not continuous recording',frames:media.native}:null,render_errors:errors};
  await writeFile(path.join(out,'manifest.json'),JSON.stringify(manifest,null,2),{flag:'wx'});
  if(!stills)await copyFile(path.join(repo,'assets/preview/player.html'),path.join(out,'index.html'));
  console.log(`Exported ${stills?'review frames':'MP4 with audio and review frames'}: ${out}`);
}finally{await browser.close({silent:true});}

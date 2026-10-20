// Export dedicated Lintel concept-film covers using canonical identity and a real film still.
// Usage: node scripts/prepare-video-covers.mjs --frame FRAME.png --out ABSENT_DIRECTORY
import {chromium} from '../extensions/browser/node_modules/playwright/index.mjs';
import {mkdir, readFile, writeFile} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const args=process.argv.slice(2);
const option=(name)=>{const i=args.indexOf(name);return i<0?null:args[i+1];};
const framePath=option('--frame'), outArg=option('--out');
if(!framePath||!outArg) throw new Error('Expected --frame FRAME.png --out ABSENT_DIRECTORY');
const out=path.resolve(outArg);
await mkdir(out); // Preserve prior exports; never overwrite a populated release folder.
await mkdir(path.join(out,'qa'));
const frame=await readFile(path.resolve(framePath));
let mark=await readFile(path.join(repo,'apps/desktop/assets/identity/source/lintel-wordmark.svg'),'utf8');
mark=mark.replace(/<\?xml[^>]*>/,'').replaceAll('#3d3d3a','#EFEBDD').replaceAll('#c16a47','#DCA382');
const frameURL='data:image/png;base64,'+frame.toString('base64');
const variants=[['landscape',1920,1080],['portrait',1080,1440],['vertical',1080,1920]];
const records=[];
const browser=await chromium.launch({headless:true});
try{
 for(const lang of ['zh-CN','en']) for(const [format,width,height] of variants){
  const zh=lang==='zh-CN';
  const title=zh?'Claude Code<br>环境与资料管理器':'Claude Code<br>environment &amp;<br>data manager';
  const sub=zh?'配置 · 资料 · 本地状态 · 远端环境':'Configuration, data, local & remote';
  const caption=zh?'概念短片':'CONCEPT FILM';
  const html=`<!doctype html><html lang="${lang}"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Lintel · ${caption} · ${format}</title><style>
  *{box-sizing:border-box}html,body{margin:0;background:#20241D;color:#EFEBDD}body{font-family:'PingFang SC',Arial,sans-serif}.cover{width:${width}px;height:${height}px;position:relative;overflow:hidden;background:#20241D}.brand{position:absolute;z-index:2}.brand svg{display:block;width:100%;height:auto}.edition,.label,.site{position:absolute;z-index:2;font-family:CommitMono,Menlo,monospace;letter-spacing:.12em;color:#D6D4C5}.edition{color:#DCA382}.title{position:absolute;z-index:2;margin:0;font-family:${zh?"'LXGW WenKai','Source Han Serif SC',serif":"Georgia,serif"};font-weight:400;line-height:1.16;letter-spacing:${zh?'0':'.005em'}}.sub{position:absolute;z-index:2;margin:0;line-height:1.5;color:#D9D6C8}.scene{position:absolute;overflow:hidden}.scene img{position:absolute;max-width:none;display:block}.scene:after{content:'';position:absolute;inset:0;pointer-events:none;background:linear-gradient(90deg,#20241D,transparent 16%),linear-gradient(0deg,#20241D,transparent 13%,transparent 91%,#20241D)}.rule{position:absolute;height:1px;background:#8B8E7B;opacity:.45}.dot{display:inline-block;width:8px;height:8px;margin-right:14px;background:#DCA382;vertical-align:middle}
  .landscape .brand{left:112px;top:92px;width:365px}.landscape .edition{left:112px;top:237px;font-size:26px}.landscape .title{left:112px;top:340px;font-size:${zh?80:72}px}.landscape .sub{left:116px;top:650px;font-size:34px;max-width:675px}.landscape .scene{left:780px;top:205px;width:1140px;height:790px}.landscape .scene img{width:1920px;height:1080px;left:-260px;top:-110px}.landscape .label{left:116px;bottom:92px;font-size:25px}.landscape .site{right:112px;bottom:92px;font-size:25px}.landscape .rule{left:112px;right:112px;bottom:155px}
  .portrait .brand,.vertical .brand{left:80px;top:100px;width:460px}.portrait .edition,.vertical .edition{left:84px;top:278px;font-size:27px}.portrait .title,.vertical .title{left:76px;top:390px;font-size:${zh?100:118}px}.portrait .sub,.vertical .sub{left:84px;top:655px;font-size:37px;max-width:912px}.portrait .scene{left:0;top:760px;width:1080px;height:530px}.portrait .scene img{width:2160px;height:1215px;left:-390px;top:-210px}.portrait .label{left:84px;bottom:66px;font-size:26px}.portrait .site{right:84px;bottom:66px;font-size:26px}.portrait .rule{left:84px;right:84px;bottom:125px}
  .vertical .brand{top:160px;width:500px}.vertical .edition{top:350px;font-size:30px}.vertical .title{top:495px;font-size:${zh?108:126}px}.vertical .sub{top:780px;font-size:40px}.vertical .scene{left:0;top:945px;width:1080px;height:735px}.vertical .scene img{width:2160px;height:1215px;left:-390px;top:-210px}.vertical .label{left:84px;bottom:135px;font-size:28px}.vertical .site{right:84px;bottom:135px;font-size:28px}.vertical .rule{left:84px;right:84px;bottom:205px}
  .portrait.en .title{font-size:84px}.portrait.en .sub{top:735px}.portrait.en .scene{top:840px;height:450px}.vertical.en .title{font-size:100px}.vertical.en .sub{top:900px}.vertical.en .scene{top:1045px;height:635px}
  </style><main class="cover ${format} ${zh?'zh':'en'}"><div class="brand" aria-label="Lintel">${mark}</div><div class="edition">0.1.0 · DEVELOPER PREVIEW</div><h1 class="title">${title}</h1><p class="sub">${sub}</p><div class="scene"><img src="${frameURL}" alt="Actual concept-film frame: Clawd carries a work package across the character bridge."></div><div class="rule"></div><div class="label"><span class="dot"></span>${caption} · 20s</div><div class="site">lintel.page</div></main></html>`;
  const basename=`lintel-film-cover.${lang}.${format}`;
  await writeFile(path.join(out,basename+'.html'),html);
  const page=await browser.newPage({viewport:{width,height},deviceScaleFactor:1});
  await page.setContent(html);
  await page.evaluate(()=>document.fonts.ready);
  await page.locator('img').evaluate(e=>e.decode());
  const checks=await page.evaluate(()=>{
   const card=document.querySelector('.cover'), r=card.getBoundingClientRect();
   const nodes=[...document.querySelectorAll('.brand,.edition,.title,.sub,.label,.site')].map(el=>{const a=el.getBoundingClientRect();return {role:el.className,x:a.x,y:a.y,width:a.width,height:a.height,fontSize:getComputedStyle(el).fontSize,inside:a.x>=0&&a.y>=0&&a.right<=r.right&&a.bottom<=r.bottom};});
   return {width:r.width,height:r.height,nodes,missingImage:[...document.images].some(el=>!el.complete||!el.naturalWidth),title:document.querySelector('h1').innerText};
  });
  if(checks.missingImage||checks.nodes.some(n=>!n.inside))throw new Error('Invalid cover '+basename+' '+JSON.stringify(checks));
  await page.locator('.cover').screenshot({path:path.join(out,basename+'.png')});
  await page.evaluate((w)=>{const card=document.querySelector('.cover');document.body.style.width='390px';document.body.style.height=`${card.offsetHeight*390/w}px`;card.style.transformOrigin='top left';card.style.transform=`scale(${390/w})`;},width);
  await page.setViewportSize({width:390,height:Math.ceil(height*390/width)});
  await page.screenshot({path:path.join(out,'qa',basename+'.390.png')});
  records.push({file:basename+'.png',lang,format,width,height,source_frame_time_seconds:14.5,kind:'concept film cover; actual frame with independently composed identity and typography',checks});
  await page.close();
 }
}finally{await browser.close()}
await writeFile(path.join(out,'qa','covers-check.json'),JSON.stringify({records,source_frame:path.basename(framePath),identity:'canonical wordmark geometry with night-theme paint; masks unchanged',platform_upload:false},null,2)+'\n');
await writeFile(path.join(out,'README.md'),'# Lintel video covers\n\nDedicated PNGs in Chinese and English, each landscape 1920×1080, portrait 1080×1440 and vertical 1080×1920. These are independently composed covers using the original concept-film bridge frame at 14.5 seconds; the film is not native App footage. HTML siblings are editable self-contained sources. qa contains local rendering checks, not upload media. Platform crop remains subject to the actual composer. Lintel is independent of Anthropic; Clawd belongs to Anthropic.\n');
console.log(JSON.stringify({out,covers:records.map(({file,width,height})=>({file,width,height}))}));

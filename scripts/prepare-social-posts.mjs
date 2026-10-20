#!/usr/bin/env node
// Local bilingual social artwork. Generated files stay outside Git.
import { chromium } from '../extensions/browser/node_modules/playwright/index.mjs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { cards, groups } from '../assets/preview/launch/cards.mjs';

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const arg = name => { const i = process.argv.indexOf(name); return i < 0 ? null : process.argv[i + 1]; };
const outputArg = arg('--output');
const checkOnly = process.argv.includes('--check-only');
const slice = arg('--slice')?.split(',');
const esc = value => String(value ?? '').replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;');
const t = (value, lang) => value?.[lang] ?? '';
const markSource = await readFile(path.join(repo,'apps/desktop/assets/identity/source/lintel-wordmark.svg'),'utf8');
const markInner = markSource.replace(/^.*?<svg[^>]*>/su,'').replace(/<\/svg>\s*$/u,'');
const mark = `<svg class="wordmark" viewBox="0 0 505.45183 148.2" aria-label="lintel">${markInner}</svg>`;
const imageCache = new Map();
async function image(file) {
  if (!imageCache.has(file)) {
    const bytes = await readFile(path.join(repo,file));
    imageCache.set(file, `data:image/${file.endsWith('.png')?'png':'jpeg'};base64,${bytes.toString('base64')}`);
  }
  return imageCache.get(file);
}
const native = {
  'native-home': ['assets/preview/screenshots/native-home.zh-CN.jpg','assets/preview/screenshots/native-home.en.jpg'],
  'native-archive-selection': ['assets/preview/screenshots/native-archive-selection-crop.zh-CN.png'],
  'native-reader': ['assets/preview/screenshots/native-reader.zh-CN.jpg'],
};
async function capture(card, lang) {
  const files = native[card.media];
  if (!files) throw new Error(`Missing reviewed media for ${card.id}`);
  const caption = lang === 'zh' ? '真实 App · 合成数据' : 'Native app · synthetic data';
  const qualifier = card.media === 'native-archive-selection' ? (lang==='zh'?' · 已裁切／编辑':' · Chinese UI · cropped/edited') : card.media==='native-reader'&&lang==='en'?' · Chinese UI':'';
  return `<figure class="capture"><img src="${await image(files[lang==='en'&&files.length>1?1:0])}" alt=""><figcaption>${caption}${qualifier} · 0.1.0</figcaption></figure>`;
}
function note(value, lang, cls='note') { return value ? `<p class="${cls}">${esc(t(value,lang))}</p>` : ''; }
function rows(items, lang, {numbers=true}={}) {
  return `<div class="rows">${items.map((item,i)=>`<article class="row">${numbers?`<span class="num">${String(i+1).padStart(2,'0')}</span>`:''}<div><h3>${esc(t(item.title||item.label||item.name,lang))}</h3><p>${esc(t(item.text||item.detail,lang))}</p>${item.limit?`<p class="limit">${esc(t(item.limit,lang))}</p>`:''}</div></article>`).join('')}</div>`;
}
function packageObject(lang) {
  return `<div class="package-scene" aria-label="${lang==='zh'?'加密工作包示意':'Illustrative encrypted work package'}"><div class="sheets"><span>CLAUDE.md</span><span>MEMORY.md</span><span>session.jsonl</span></div><span class="flow-arrow">↓</span><div class="package"><b>.age</b><span>lintel.work/1</span></div></div>`;
}
async function body(card,lang) {
  switch(card.kind) {
    case 'cover': return `${await capture(card,lang)}${note(card.body,lang,'body-copy')}`;
    case 'map': return `<div class="six-grid">${card.items.map((item,i)=>`<article><span class="num">0${i+1}</span><h3>${esc(t(item.title,lang))}</h3><p>${esc(t(item.text,lang))}</p></article>`).join('')}</div>${note(card.footer,lang)}`;
    case 'work-overview': return `${await capture(card,lang)}${rows(card.steps,lang)}${note(card.note,lang)}`;
    case 'local-state': return `${rows(card.stateItems,lang)}${note(card.note,lang)}`;
    case 'surfaces': return `<div class="terminal"><span>CLI · ${lang==='zh'?'命令入口示例':'command examples'}</span>${card.terminal.lines.slice(0,2).map(line=>`<code>${esc(line)}</code>`).join('')}</div>${rows(card.panels,lang)}${note(card.note,lang)}`;
    case 'closing': return `<div class="facts-grid">${card.facts.map(f=>`<article><h3>${esc(t(f.title,lang))}</h3><p>${esc(t(f.text,lang))}</p></article>`).join('')}</div><div class="download"><span>↓</span><strong>lintel.page</strong><p>${lang==='zh'?'下载与文档':'Downloads & documentation'}</p></div>${note(card.notice,lang,'notice')}`;
    case 'work-cover': return `<figure class="art-plate"><img src="${await image('apps/site/assets/lintel-keep.png')}" alt=""><figcaption>${lang==='zh'?'概念插画 · 非 App 界面':'Concept illustration · not an app screen'}</figcaption></figure>${note(card.body,lang)}`;
    case 'work-selection': return `${await capture(card,lang)}${note(card.callout,lang,'body-copy')}${note(card.note,lang)}`;
    case 'work-package': return `${packageObject(lang)}${rows(card.steps,lang)}${note(card.note,lang)}`;
    case 'work-reader': return `${await capture(card,lang)}<div class="reader-facts">${card.facts.map(f=>note(f,lang,'reader-fact')).join('')}</div>`;
    case 'work-import': return `<div class="origin">.age <span>${esc(t(card.branch.from,lang))}</span></div><div class="branches">${card.branch.paths.map((p,i)=>`<div><span class="num">0${i+1}</span><p>${esc(t(p.label,lang))}</p></div>`).join('')}</div>${note(card.callout,lang,'body-copy')}${note(card.note,lang)}`;
    case 'settings-cover': return `<div class="settings-sheet">${card.examples.map((e,i)=>`<article><span class="slider s${i}"><i></i></span><div><h3>${esc(t(e.value,lang))}</h3><code>${esc(e.key)}</code></div></article>`).join('')}</div>${note(card.note,lang)}`;
    case 'settings-actions': return `${rows(card.actions,lang)}${note(card.note,lang,'notice')}`;
    case 'settings-diff': return `<div class="target"><span>${esc(t(card.target,lang))}</span><code>${esc(card.root)}</code></div><div class="diffs">${card.diffs.map(d=>`<article><code class="key">${esc(d.key)}</code><div><code>${esc(d.before)}</code><span>→</span><code class="after">${esc(d.after)}</code><b>${esc(t(d.action,lang))}</b></div></article>`).join('')}</div>${note(card.note,lang)}`;
    case 'settings-effects': return `${rows(card.outcomes,lang)}${note(card.note,lang)}`;
    case 'settings-restore': return `<div class="restore-nodes">${[card.graph.original,card.graph.later].map(n=>`<article><h3>${esc(t(n.label,lang))}</h3><code>${esc(n.value)}</code></article>`).join('')}</div><div class="restore-result"><span>≠</span><div><h3>${esc(t(card.graph.conclusion,lang))}</h3><p>${esc(t(card.graph.result,lang))}</p></div></div>${note(card.note,lang)}${note(card.qualifier,lang,'body-copy')}`;
    default: throw new Error(`Unknown card kind ${card.kind}`);
  }
}
const css = `
*{box-sizing:border-box}html,body{margin:0;background:#faf9f5;color:#3d3d3a}body{font-family:"PingFang SC","Helvetica Neue",Arial,sans-serif;-webkit-font-smoothing:antialiased}h1,h3,p,figure{margin:0}code{font-family:Menlo,monospace;font-weight:400}svg{display:block}img{display:block;width:100%;height:auto}
.card{width:1080px;height:1440px;padding:48px 64px 38px;display:grid;grid-template-rows:66px auto minmax(0,1fr) 48px;gap:30px;overflow:hidden;position:relative;background:radial-gradient(ellipse at 95% 0%,#f0e3d34d,transparent 55%),#faf9f5}
.top{display:flex;align-items:center;justify-content:space-between;border-bottom:1px solid #c6bfb2;padding-bottom:18px}.wordmark{width:176px;height:52px}.edition{font:24px/1.3 Menlo,monospace;letter-spacing:.01em;color:#746e64}.head{padding-top:2px}.eyebrow{font-size:25px;letter-spacing:.06em;color:#a6593b;margin-bottom:22px}.title{font:78px/1.18 "Songti SC","STSong",Georgia,serif;letter-spacing:-.035em;font-weight:700;overflow-wrap:normal}.lead{font-size:40px;line-height:1.48;margin-top:24px;max-width:950px}.content{min-height:0;display:flex;flex-direction:column;justify-content:center;gap:28px}.footer{display:flex;align-items:end;justify-content:space-between;border-top:1px solid #c6bfb2;font:25px/1.3 Menlo,monospace;padding-top:16px;color:#777066}.body-copy{font-size:41px;line-height:1.45}.note,.notice{font-size:35px;line-height:1.45;color:#686257;border-left:3px solid #b8aa98;padding-left:20px}.notice{color:#76543e;border-color:#c16a47}.credit,figcaption{font-size:25px;line-height:1.4;color:#7e776c}.capture img{border:1px solid #d8d1c4;border-radius:12px}.capture figcaption{margin-top:12px}.capture{flex:none}.num{color:#b56545;font:25px/1.2 Menlo,monospace}.rows{display:flex;flex-direction:column}.row{display:grid;grid-template-columns:50px 1fr;gap:18px;padding:24px 0;border-bottom:1px solid #d3cbbd}.row:first-child{padding-top:0}.row:last-child{border-bottom:0;padding-bottom:0}h3{font-size:43px;line-height:1.25;letter-spacing:-.015em;font-weight:600}.row p{font-size:39px;line-height:1.4;margin-top:12px}.row .limit{font-size:35px;color:#786b5a}.six-grid{display:grid;grid-template-columns:1fr 1fr;gap:0 38px}.six-grid article{position:relative;padding:24px 0;border-top:1px solid #d0c6b7}.six-grid h3{font-size:40px;margin:12px 0}.six-grid p{font-size:36px;line-height:1.4}.terminal{background:#282720;color:#eae5da;border-radius:12px;padding:30px 34px;display:grid;gap:20px}.terminal>span{font-size:25px;color:#c1b6a6}.terminal code{font-size:34px}.facts-grid{display:grid;grid-template-columns:1fr 1fr;gap:32px}.facts-grid article{border-top:1px solid #c9c1b3;padding-top:22px}.facts-grid h3{font-size:38px}.facts-grid p{font-size:35px;line-height:1.4;margin-top:12px}.download{padding:24px 0;text-align:center}.download>span{display:block;color:#c16a47;font-size:68px}.download strong{font:74px/1.2 Georgia,serif;letter-spacing:-.03em}.download p{font-size:35px;margin-top:12px}.art-plate img{max-height:430px;object-fit:cover;object-position:center;border-radius:12px}.art-plate figcaption{margin-top:12px}.package-scene{display:flex;align-items:center;justify-content:center;gap:42px;padding:30px 0}.sheets{transform:rotate(-3deg);display:flex;flex-direction:column;gap:12px;border:1px solid #cfc5b6;background:#fffcf5;padding:24px;box-shadow:10px 10px 0 #e9e1d5}.sheets span{font:29px/1.3 Menlo,monospace}.flow-arrow{font-size:62px;color:#c16a47;transform:rotate(-90deg)}.package{border:2px solid #b16a4d;padding:24px 34px;background:#eee1ca;display:flex;flex-direction:column;gap:16px;transform:rotate(2deg)}.package b{font:68px/1 Georgia,serif;color:#9b4d31}.package>span{font:29px/1.3 Menlo,monospace}.file-row{display:grid;grid-template-columns:38px 1fr auto;align-items:center;gap:12px;border-bottom:1px solid #d6cdbf;padding:17px 0}.file-row>span{color:#ad5d3d;font-size:32px}.file-row code{font-size:28px;overflow-wrap:anywhere}.file-row b{font-size:28px;font-weight:400}.file-row.off{color:#9b9488}.compact-facts{display:grid;gap:14px}.compact-facts p{font-size:34px;line-height:1.4}.reader-facts{display:grid;gap:15px}.reader-fact{font-size:38px;line-height:1.35;border-left:3px solid #b96b49;padding-left:20px}.origin{font:62px/1.2 Georgia,serif;color:#a15436;display:flex;gap:30px;align-items:center}.origin span{font:35px/1.4 "PingFang SC",Arial,sans-serif;color:#5e574c}.branches{display:grid;gap:0;margin-left:20px;border-left:2px solid #c4b5a1;padding-left:30px}.branches>div{display:flex;gap:25px;align-items:center;padding:22px 0;border-bottom:1px solid #d4cbbc}.branches p{font-size:38px;line-height:1.35}.settings-sheet{border-top:1px solid #bfb3a2}.settings-sheet article{display:flex;gap:35px;align-items:center;padding:38px 0;border-bottom:1px solid #d0c5b6}.settings-sheet code{font-size:29px;color:#8c7c68;display:block;margin-top:14px}.slider{width:106px;height:52px;border:2px solid #b1a798;border-radius:27px;position:relative;flex:none}.slider i{display:block;width:34px;height:34px;border-radius:50%;background:#a49783;position:absolute;top:7px;left:9px}.slider.s1{background:#b96845;border-color:#b96845}.slider.s1 i{left:59px;background:#faf3e7}.target{padding:22px 28px;background:#f0eadf;display:grid;gap:13px;border-left:4px solid #bc6a48}.target span{font-size:29px;color:#746655}.target code{font-size:36px}.diffs article{border-bottom:1px solid #cfc5b6;padding:27px 0}.diffs .key{font-size:35px;color:#635646;letter-spacing:-.03em}.diffs article>div{display:flex;align-items:center;gap:35px;margin-top:18px;font-size:42px}.diffs code:not(.key){font-size:42px}.diffs .after{color:#9e5133}.diffs b{margin-left:auto;font-size:34px;font-weight:400}.restore-nodes{display:grid;gap:28px}.restore-nodes article{border-top:1px solid #cbbfad;padding-top:24px}.restore-nodes h3{font-size:37px}.restore-nodes code{display:block;font-size:33px;margin-top:18px;letter-spacing:-.035em}.restore-result{display:flex;align-items:center;gap:28px;background:#ece4d8;padding:30px}.restore-result>span{font:92px/1 Georgia,serif;color:#a45938}.restore-result h3{font-size:38px}.restore-result p{font-size:37px;line-height:1.35;margin-top:12px}
.en .title{font-family:Georgia,serif;font-size:72px;line-height:1.08;letter-spacing:-.035em}.en .lead{font-size:39px;line-height:1.35}.en h3{font-size:39px;line-height:1.18}.en .row p{font-size:36px}.en .note,.en .notice{font-size:32px;line-height:1.35}.en .six-grid h3{font-size:35px}.en .six-grid p{font-size:33px}.en .row .limit{font-size:32px}.en .body-copy{font-size:38px}.en .facts-grid p{font-size:33px}.en .reader-fact{font-size:36px}.en .branches p{font-size:36px}.en .settings-sheet h3{font-size:39px}
.cover .title{font-size:82px}.en.cover .title{font-size:80px;line-height:1.08}.cover .content{gap:20px}.cover .lead{font-size:40px}.cover .capture{width:100%}.portrait.cover .capture img{max-height:535px;object-fit:contain}.portrait.cover .content{justify-content:center}.cover .body-copy{font-size:38px}.cover .credit{font-size:25px}.work-overview .capture img,.work-selection .capture img{max-height:285px;object-fit:contain;background:#faf9f5}.work-overview .row{padding:19px 0}.work-overview .row:first-child{padding-top:0}.work-overview .row:last-child{padding-bottom:0}.work-overview .row p{font-size:36px}.work-overview .row h3{font-size:40px}.work-selection .file-list{padding:0 12px}.work-selection .content{gap:24px}.work-package .content{gap:22px}.work-package .row{padding:16px 0}.work-package .row:first-child{padding-top:0}.work-package .row h3{font-size:39px}.work-package .row p{font-size:34px}.work-package .note{font-size:31px}.work-reader .lead{font-size:38px}.work-reader .content{gap:24px}.surfaces .row{padding:19px 0}.surfaces .row h3{font-size:39px}.surfaces .row p{font-size:35px}.surfaces .note{font-size:32px}.local-state .row p{font-size:38px}.en.local-state .row p{font-size:36px}.map .content{gap:23px}.map .note{font-size:31px}.en.map .note{font-size:30px}
.landscape{width:1920px;height:1080px;padding:48px 72px 36px;grid-template-columns:650px minmax(0,1fr);grid-template-rows:66px minmax(0,1fr) 48px;column-gap:84px;row-gap:34px}.landscape .top,.landscape .footer{grid-column:1/-1}.landscape .head{grid-column:1;grid-row:2;align-self:center}.landscape .content{grid-column:2;grid-row:2;gap:25px}.landscape .title{font-size:78px}.landscape.en .title{font-size:73px}.landscape .lead{font-size:36px;line-height:1.45}.landscape .eyebrow{font-size:24px}.landscape .capture img{max-height:600px;object-fit:contain}.landscape .body-copy{font-size:35px}.landscape .note,.landscape .notice{font-size:30px;line-height:1.35}.landscape .row p{font-size:33px}.landscape .row h3{font-size:38px}.landscape .row{padding:23px 0}.landscape .row:first-child{padding-top:0}.landscape .row:last-child{padding-bottom:0}.landscape .row .limit{font-size:30px}.landscape .six-grid h3{font-size:34px}.landscape .six-grid p{font-size:32px}.landscape .six-grid article{padding:20px 0}.landscape .terminal code{font-size:31px}.landscape .terminal{padding:23px 28px;gap:15px}.landscape.surfaces .row{padding:16px 0}.landscape.surfaces .row p{font-size:31px}.landscape .facts-grid h3{font-size:35px}.landscape .facts-grid p{font-size:31px}.landscape .download{padding:5px 0}.landscape .download>span{font-size:49px}.landscape .download strong{font-size:68px}.landscape .art-plate img{max-height:365px}.landscape.work-cover .package-scene{padding:6px 0}.landscape .package-scene{padding:12px 0}.landscape .compact-facts p{font-size:31px}.landscape.work-package .row{padding:13px 0}.landscape.work-package .row p{font-size:31px}.landscape.work-package .row h3{font-size:35px}.landscape .reader-fact{font-size:32px}.landscape .branches p{font-size:33px}.landscape .branches>div{padding:21px 0}.landscape .settings-sheet article{padding:32px 0}.landscape .restore-nodes{gap:23px}.landscape .restore-result{padding:25px}.landscape .restore-result p{font-size:33px}.landscape.settings-diff .diffs article{padding:21px 0}.landscape .diffs .key{font-size:32px}.landscape.work-overview .capture img,.landscape.work-selection .capture img{max-height:235px}.landscape.work-selection .content{gap:20px}.landscape .file-row{padding:13px 0}.landscape .file-row code,.landscape .file-row b{font-size:26px}.landscape .credit{font-size:24px}
.portrait .content{gap:24px}.portrait.cover .capture img{max-height:485px}.portrait.en.cover .capture img{max-height:455px}.portrait.work-cover .art-plate img{max-height:520px}.portrait.work-reader .capture img{max-height:510px;object-fit:contain}.landscape.work-reader .capture img{max-height:530px}.landscape .content{gap:20px}.portrait.work-overview .capture img{max-height:220px}.portrait.work-overview .row{padding:16px 0}.portrait.en.surfaces .row{padding:15px 0}.portrait.en.surfaces .row p{font-size:35px}.portrait.en.surfaces .terminal{padding:24px 30px;gap:15px}.portrait.work-package .package-scene{padding:20px 0}.landscape.work-package .content{gap:22px}
.portrait.en.work-overview .content{gap:20px}.portrait.en.work-overview .capture img{max-height:180px}.portrait.en.surfaces .content{gap:20px}
`;
function title(card,lang,format) {
  if(card.id==='overview-01') return lang==='zh'?'Claude Code<br>环境与资料管理器':'Claude Code<br>environment &amp;<br>data manager';
  const breaks={'overview-03':'指令、记忆与<br>会话资料','overview-04':'本地状态与<br>变化记录','settings-01':'环境配置与<br>本地状态','settings-03':'预览目标与<br>字段变化','settings-04':'配置读回与<br>网络观察'};
  if(lang==='zh'&&format==='landscape'&&breaks[card.id])return breaks[card.id];
  return esc(t(card.title,lang));
}
function lead(card,lang) {
  if(card.id==='overview-01'&&lang==='zh')return '管理配置、工作资料与本地状态。<br>支持本机与 SSH 远端环境。';
  return esc(t(card.lead,lang));
}
async function html(card,lang,format) {
  const group=groups.find(g=>g.id===card.group);
  return `<!doctype html><html lang="${lang==='zh'?'zh-CN':'en'}"><meta charset="utf-8"><title>${esc(t(card.title,lang))}</title><style>${css}</style><body><main class="card ${lang} ${format} ${card.kind}"><div class="top">${mark}<span class="edition">0.1.0 · PREVIEW</span></div><header class="head"><p class="eyebrow">${esc(t(card.eyebrow,lang))}</p><h1 class="title">${title(card,lang,format)}</h1><p class="lead">${lead(card,lang)}</p></header><section class="content">${await body(card,lang)}</section><footer class="footer"><span>lintel.page</span><span>${esc(t(group.label,lang))} · ${String(card.page).padStart(2,'0')} / ${card.group==='overview'?'06':'05'}</span></footer></main></body></html>`;
}
function validateSource() {
  if(cards.length!==16 || new Set(cards.map(c=>c.id)).size!==16)throw new Error('Expected 16 unique cards');
  for(const [group,count] of [['overview',6],['work',5],['settings',5]])if(cards.filter(c=>c.group===group).length!==count)throw new Error(`Wrong group count: ${group}`);
  for(const card of cards)for(const lang of ['zh','en'])for(const key of ['title','lead','eyebrow'])if(!t(card[key],lang))throw new Error(`Missing ${card.id}.${key}.${lang}`);
  if(!markInner.includes('mask='))throw new Error('Canonical wordmark mask missing');
  if(slice?.some(id=>!cards.some(c=>c.id===id)))throw new Error('Unknown slice card');
}
async function inspect(page) {
  return page.evaluate(()=>{
    const problems=[];
    const region=document.querySelector('.content').getBoundingClientRect();
    for(const el of document.querySelectorAll('.content *,.head *,.footer *')){
      if(el.closest('svg'))continue;
      const r=el.getBoundingClientRect(),s=getComputedStyle(el);
      if(!r.width||!r.height||s.display==='none')continue;
      if(r.left<0||r.top<0||r.right>innerWidth+.8||r.bottom>innerHeight+.8)problems.push({type:'canvas',class:el.className,text:el.textContent.slice(0,55)});
      if(el.closest('.content')&&(r.top<region.top-.8||r.bottom>region.bottom+.8))problems.push({type:'content-region',class:el.className,text:el.textContent.slice(0,55),bottom:r.bottom,regionBottom:region.bottom});
      if(el.scrollWidth>el.clientWidth+2&&s.overflowX==='visible')problems.push({type:'text-overflow',class:el.className,text:el.textContent.slice(0,55)});
    }
    for(const img of document.images)if(!img.complete||!img.naturalWidth)problems.push({type:'broken-image'});
    const head=document.querySelector('.head').getBoundingClientRect();
    if(!document.querySelector('.landscape')&&head.bottom>region.top+.8)problems.push({type:'head-body-overlap'});
    return {problems,bodyRegion:{top:region.top,bottom:region.bottom,height:region.height},headingFont:parseFloat(getComputedStyle(document.querySelector('.title')).fontSize),leadFont:parseFloat(getComputedStyle(document.querySelector('.lead')).fontSize)};
  });
}
async function contact(browser,files,out,label,columns=4){
  const images=await Promise.all(files.map(async file=>`data:image/png;base64,${(await readFile(file)).toString('base64')}`));
  const page=await browser.newPage({viewport:{width:1600,height:900}});
  await page.setContent(`<html><style>*{box-sizing:border-box}body{margin:0;padding:20px;background:#e9e3d9;font:18px Arial}h1{font:28px Georgia}.grid{display:grid;grid-template-columns:repeat(${columns},1fr);gap:18px}figure{margin:0}img{display:block;width:100%}figcaption{font-size:14px;margin-top:6px}</style><h1>${esc(label)}</h1><div class="grid">${files.map((f,i)=>`<figure><img src="${images[i]}"><figcaption>${esc(path.basename(f))}</figcaption></figure>`).join('')}</div></html>`);
  await page.evaluate(()=>Promise.all([...document.images].map(i=>i.decode())));
  await page.screenshot({path:out,fullPage:true});await page.close();
}
validateSource();
if(checkOnly){
  for(const c of cards)for(const l of ['zh','en'])for(const f of ['portrait','landscape'])await html(c,l,f);
  console.log('Source preflight passed: 16 cards, paired copy, 64 HTML documents, reviewed local media, canonical wordmark mask. No files written.');
}else{
  if(!outputArg)throw new Error('Use --output <fresh-directory>');
  const output=path.resolve(outputArg);await mkdir(output,{recursive:false});
  for(const dir of ['zh','en','zh-landscape','en-landscape','editable','qa','qa/phone','qa/landscape'])await mkdir(path.join(output,dir));
  const browser=await chromium.launch({headless:true,executablePath:process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE||undefined});
  const reports=[];const selected=slice?cards.filter(c=>slice.includes(c.id)):cards;
  try{
    for(const lang of ['zh','en'])for(const format of ['portrait','landscape']){
      const dims=format==='portrait'?{width:1080,height:1440}:{width:1920,height:1080};
      const filesOut=[];
      for(const card of selected){
        const source=await html(card,lang,format),base=`${card.id}.${lang}`;
        const editable=path.join(output,'editable',`${base}.${format}.html`);await writeFile(editable,source);
        const page=await browser.newPage({viewport:dims,deviceScaleFactor:1});await page.setContent(source,{waitUntil:'load'});
        await page.evaluate(()=>document.fonts.ready);await page.evaluate(()=>Promise.all([...document.images].map(i=>i.decode())));
        const report=await inspect(page);const dest=path.join(output,lang+(format==='landscape'?'-landscape':''),`${base}.png`);
        await page.screenshot({path:dest,animations:'disabled'});filesOut.push(dest);
        const viewWidth=format==='portrait'?390:900,scale=viewWidth/dims.width;
        await page.addStyleTag({content:`html,body{width:${viewWidth}px;height:${dims.height*scale}px;overflow:hidden}.card{transform:scale(${scale});transform-origin:0 0}`});
        await page.setViewportSize({width:viewWidth,height:Math.ceil(dims.height*scale)});
        const preview=path.join(output,'qa',format==='portrait'?'phone':'landscape',`${base}.png`);await page.screenshot({path:preview});
        reports.push({id:card.id,lang,format,file:path.relative(output,dest),...report,preview:path.relative(output,preview),viewWidth,leadEffectivePx:report.leadFont*scale});
        await page.close();
      }
      await contact(browser,filesOut,path.join(output,'qa',`contact-${lang}${format==='portrait'?'':'-landscape'}.png`),`Lintel · ${lang} · ${dims.width} × ${dims.height}`);
    }
    const failures=reports.filter(r=>r.problems.length);
    await writeFile(path.join(output,'qa/qa.json'),JSON.stringify({schema:'lintel.social-cards.qa/2',generatedAt:new Date().toISOString(),pass:!failures.length,count:reports.length,failures,reports},null,2)+'\n');
    await writeFile(path.join(output,'manifest.json'),JSON.stringify({schema:'lintel.social-cards.manifest/2',product:'Claude Code environment and data manager',version:'0.1.0 developer Preview',ownerAcceptance:'pending',groups,cards:cards.map(c=>({id:c.id,group:c.group,page:c.page,title:c.title})),files:reports.map(r=>r.file)},null,2)+'\n');
    console.log(JSON.stringify({output,count:reports.length,pass:!failures.length,failures:failures.map(f=>({id:f.id,lang:f.lang,format:f.format,problems:f.problems}))},null,2));
    if(failures.length)process.exitCode=2;
  }finally{await browser.close();}
}

// Built desktop UI + illustrative empty inventory; no core process or native WebKit.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdtemp,writeFile} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {stripVTControlCharacters} from 'node:util';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),desktop=path.join(repo,'apps/desktop');
const {chromium}=await import(pathToFileURL(process.env.PLAYWRIGHT_MODULE||path.join(repo,'extensions/browser/node_modules/playwright/index.mjs')).href);
const output=await mkdtemp(path.join(os.tmpdir(),'lintel-clawd-ui-'));
const preview=spawn(process.execPath,[path.join(desktop,'node_modules/vite/bin/vite.js'),'preview','--host','127.0.0.1','--port','0','--strictPort'],{cwd:desktop,stdio:['ignore','pipe','pipe']});
let browser,page;
try{
  const url=await new Promise((resolve,reject)=>{let text='';const timer=setTimeout(()=>reject(new Error('preview timeout')),15000);preview.once('error',reject);preview.once('exit',code=>reject(new Error(`preview exited ${code}`)));for(const stream of [preview.stdout,preview.stderr])stream.on('data',chunk=>{text+=chunk;const match=stripVTControlCharacters(text).match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0]);}});});
  browser=await chromium.launch({headless:true});page=await browser.newPage({viewport:{width:1120,height:800},colorScheme:'light'});
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.addInitScript(()=>{
    try{localStorage.setItem('lintel.language','zh-CN')}catch{};window.__preferred=['zh-Hans'];window.__menuCalls=[];window.isTauri=true;
    window.__TAURI_INTERNALS__={invoke:async(command,args)=>{
      if(command==='app_preferred_languages')return window.__preferred;
      if(command==='app_set_language'){window.__menuCalls.push(args.language);return null;}
      if(command==='request'&&args.payload.command==='discover')return {ok:true,data:{environments:[],capabilities:[]}};
      if(command==='request'&&args.payload.command==='jobs')return {ok:true,data:{jobs:[]}};
      throw new Error('Unexpected native call in playroom-only fixture: '+command);
    }};
    localStorage.setItem('lintel.clawd.runner.best','123');
    const pending=new Set(),parked=new Map(),request=window.requestAnimationFrame.bind(window),cancel=window.cancelAnimationFrame.bind(window);
    let frameClock=null,freezeAt=null,frozen=false;
    function deliver(id,callback,time){
      if(frozen){parked.set(id,callback);return;}
      pending.delete(id);
      callback(frameClock===null?time:(frameClock+=1000/60));
      if(freezeAt!==null&&document.querySelector('.cg-hold progress')?.value>freezeAt){
        frozen=true;freezeAt=null;
        window.__touchFreeze={at:performance.now(),progress:document.querySelector('.cg-hold progress').value,frames:window.__touchFrames.length};
      }
    }
    window.requestAnimationFrame=callback=>{const id=request(time=>deliver(id,callback,time));pending.add(id);return id;};
    window.cancelAnimationFrame=id=>{pending.delete(id);parked.delete(id);cancel(id);};
    // Test-only frame control, not a product pause/state API. Native RAF still
    // schedules every callback; only this touch case fixes simulation deltas and
    // parks after a rendered partial hold, before the driver sends real cancel.
    window.__touchFrameControl={
      arm(){frameClock=performance.now();freezeAt=20;frozen=false;window.__touchFreeze=null;},
      resume(){frozen=false;const frames=[...parked];parked.clear();for(const [id,callback] of frames)if(pending.has(id))deliver(id,callback,performance.now());},
      reset(){frameClock=null;freezeAt=null;frozen=false;window.__touchFreeze=null;},
    };
    window.pendingAnimationFrames=()=>pending.size;
    // Observe actual canvas placement, without a product game-state getter.
    window.__clawdMinY=188;window.__clawdY=188;window.__firstLiftProgress=null;window.__touchFrames=[];window.__touchEvents=[];
    window.addEventListener('pointercancel',event=>{if(event.pointerType==='touch')window.__touchEvents.push({type:event.type,at:performance.now(),progress:document.querySelector('.cg-hold progress')?.value,y:window.__clawdY,peak:188-window.__clawdMinY,frames:window.__touchFrames.length})},{capture:true});
    const translate=CanvasRenderingContext2D.prototype.translate;
    CanvasRenderingContext2D.prototype.translate=function(x,y){
      if(x===80&&this.canvas.closest('.clawd-game')){
        if(y<188&&window.__firstLiftProgress===null)window.__firstLiftProgress=document.querySelector('.cg-hold progress').value;
        window.__clawdY=y;window.__clawdMinY=Math.min(window.__clawdMinY,y);
        if(window.__observeTouch)window.__touchFrames.push({at:performance.now(),y,progress:document.querySelector('.cg-hold progress').value});
      }
      return translate.call(this,x,y);
    };
  });
  await page.goto(url);
  async function openGame(){await page.getByRole('button',{name:'陪它玩，打开 Clawd 的口袋',exact:true}).click();await page.getByRole('button',{name:'Clawd 跳一跳',exact:true}).click();await page.getByRole('group',{name:'Clawd 跳跃跑道'}).waitFor();}
  await page.locator('.language-switch select').selectOption('system');
  await page.getByRole('button',{name:/本地工作空间/}).click();
  const motionSettings=page.getByRole('dialog');
  await motionSettings.getByRole('checkbox',{name:'界面动画',exact:true}).uncheck();
  await motionSettings.getByRole('checkbox',{name:'Logo 光标慢闪',exact:true}).uncheck();
  await page.keyboard.press('Escape');
  await openGame();
  const dialog=page.getByRole('dialog'),game=dialog.locator('.clawd-game');
  assert.equal(await game.count(),1);assert.equal(await game.locator('.cg-best').textContent(),'00123');
  await dialog.screenshot({path:path.join(output,'app-day.png')});
  await game.locator('canvas').evaluate(canvas=>{window.__quietScenery=canvas.getContext('2d').getImageData(0,0,canvas.width,50).data;});
  await dialog.locator('.cg-jump').click();await page.waitForFunction(()=>Number(document.querySelector('.cg-score').textContent)>0);
  await page.waitForFunction(()=>Number(document.querySelector('.cg-score').textContent)>15);
  assert.ok(await game.locator('canvas').evaluate(canvas=>canvas.getContext('2d').getImageData(0,0,canvas.width,50).data.every((value,index)=>value===window.__quietScenery[index])),'quiet mode keeps decorative scenery still while the game continues');
  assert.equal(await game.getAttribute('data-state'),'running');
  assert.equal(await page.evaluate(()=>window.pendingAnimationFrames()),1);
  await page.keyboard.press('p');
  const beforeLanguage=await game.evaluate(node=>{window.__gameNode=node;window.__gameCanvas=node.querySelector('canvas');return {phase:node.dataset.state,score:node.querySelector('.cg-score').textContent,stars:node.querySelector('.cg-stars').textContent,best:node.querySelector('.cg-best').textContent};});
  assert.equal(beforeLanguage.phase,'paused');assert.equal(await page.evaluate(()=>window.pendingAnimationFrames()),0);
  await page.evaluate(()=>{window.__preferred=['en-US'];window.dispatchEvent(new Event('languagechange'));});
  await dialog.getByRole('group',{name:'Clawd jump track',exact:true}).waitFor();
  await dialog.locator('.cg-jump').getByText('Jump',{exact:true}).waitFor();
  const afterLanguage=await game.evaluate(node=>({phase:node.dataset.state,score:node.querySelector('.cg-score').textContent,stars:node.querySelector('.cg-stars').textContent,best:node.querySelector('.cg-best').textContent}));
  assert.deepEqual(afterLanguage,beforeLanguage,'language switch preserves game phase, score, collected stars and best');
  assert.equal(await game.evaluate(node=>node===window.__gameNode&&node.querySelector('canvas')===window.__gameCanvas),true,'same game and canvas remain mounted');
  assert.equal(await page.evaluate(()=>window.pendingAnimationFrames()),0,'paused game stays paused after relabeling');
  await page.evaluate(()=>{window.__preferred=['zh-Hans'];window.dispatchEvent(new Event('languagechange'));});
  await dialog.getByRole('group',{name:'Clawd 跳跃跑道',exact:true}).waitFor();
  await dialog.getByRole('tab',{name:'点阵风景册',exact:true}).click();
  assert.equal(await game.count(),0);assert.equal(await page.evaluate(()=>window.pendingAnimationFrames()),0);
  await dialog.getByRole('button',{name:'第 2 幅：雨夜小屋',exact:true}).click();
  await dialog.getByRole('tab',{name:'跳一小段',exact:true}).click();
  assert.equal(await game.count(),1);assert.equal(await game.getAttribute('data-state'),'ready');
  async function freshHoldRun(){
    await dialog.getByRole('tab',{name:'点阵风景册',exact:true}).click();
    await dialog.getByRole('tab',{name:'跳一小段',exact:true}).click();
    await page.evaluate(()=>{window.__clawdMinY=188;window.__clawdY=188;window.__firstLiftProgress=null;window.__touchFrameControl.reset();});
  }
  await dialog.locator('.cg-jump').click();await page.waitForTimeout(650);
  const tapPeak=await page.evaluate(()=>188-window.__clawdMinY);
  assert.ok(tapPeak>70&&tapPeak<85,'pointer tap retains the original jump height');
  await freshHoldRun();
  const jumpBox=await dialog.locator('.cg-jump').boundingBox();
  await page.mouse.move(jumpBox.x+jumpBox.width/2,jumpBox.y+jumpBox.height/2);await page.mouse.down();
  await page.waitForFunction(()=>window.__clawdMinY<180);
  assert.equal(await page.evaluate(()=>window.__firstLiftProgress),0,'the first visible lift precedes the hold window');
  await page.waitForFunction(()=>document.querySelector('.cg-hold progress').value===100);
  assert.ok(await page.evaluate(()=>window.__clawdMinY)<188,'pointer down takes off before pointer up');
  assert.equal(await game.getAttribute('data-holding'),'true');
  await page.mouse.up();await page.waitForFunction(()=>window.__clawdMinY<70);
  assert.equal(await game.getAttribute('data-holding'),'false','release ends the lift without a second takeoff');
  await page.keyboard.press('p');await dialog.screenshot({path:path.join(output,'app-held-jump.png')});
  await freshHoldRun();const holdBoard=dialog.locator('.cg-board');await holdBoard.focus();
  await page.keyboard.down('Space');await page.waitForFunction(()=>document.querySelector('.cg-hold progress').value>20);
  await game.evaluate(node=>{window.__holdingCanvas=node.querySelector('canvas')});
  await page.evaluate(()=>{window.__preferred=['en-US'];window.dispatchEvent(new Event('languagechange'));});
  await dialog.getByRole('progressbar',{name:'Higher',exact:true}).waitFor();
  assert.equal(await game.getAttribute('data-holding'),'true');
  assert.equal(await game.evaluate(node=>node.querySelector('canvas')===window.__holdingCanvas),true,'relabeling keeps the held input and canvas');
  assert.ok(await game.locator('progress').evaluate(node=>node.value>=20),'language change does not clear the active hold');
  await page.evaluate(()=>{window.__preferred=['zh-Hans'];window.dispatchEvent(new Event('languagechange'));});
  await dialog.getByRole('progressbar',{name:'高跳',exact:true}).waitFor();
  await page.keyboard.press('p');
  assert.equal(await game.getAttribute('data-state'),'paused');assert.equal(await game.getAttribute('data-holding'),'false');
  assert.equal(await game.locator('progress').evaluate(node=>node.value),0);
  await page.keyboard.up('Space');await dialog.locator('.cg-start').click();await page.waitForFunction(()=>window.__clawdY===188);await page.waitForTimeout(100);
  assert.equal(await page.evaluate(()=>window.__clawdY),188,'cancelled key release cannot trigger a delayed takeoff');
  await freshHoldRun();await holdBoard.focus();await page.keyboard.down('ArrowUp');
  await page.waitForFunction(()=>document.querySelector('.cg-hold progress').value===100);
  assert.ok(await page.evaluate(()=>window.__clawdMinY)<188,'keyboard down takes off while the key remains held');
  await page.keyboard.up('ArrowUp');await page.waitForFunction(()=>window.__clawdMinY<70);
  assert.equal(await game.getAttribute('data-holding'),'false','keyboard release ends the existing lift');
  await freshHoldRun();const touchBox=await holdBoard.boundingBox();const touch=await page.context().newCDPSession(page);
  await page.evaluate(()=>{window.__observeTouch=true;window.__touchFrames=[];window.__touchEvents=[];window.__touchFrameControl.arm();});
  await touch.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:touchBox.x+110,y:touchBox.y+150,id:1}]});
  await page.waitForFunction(()=>window.__touchFreeze!==null,null,{polling:10});
  const frozenTouch=await page.evaluate(()=>window.__touchFreeze);
  assert.ok(frozenTouch.progress>20&&frozenTouch.progress<50,'cancel opportunity is a rendered partial hold, before full ascent');
  if(process.env.LINTEL_CLAWD_CANCEL_DELAY_MS)await page.waitForTimeout(Number(process.env.LINTEL_CLAWD_CANCEL_DELAY_MS));
  await touch.send('Input.dispatchTouchEvent',{type:'touchCancel',touchPoints:[]});
  assert.equal(await game.getAttribute('data-holding'),'false');
  assert.equal(await page.evaluate(()=>window.__touchEvents.at(-1).progress),frozenTouch.progress,'CDP cancel reaches the same rendered partial hold despite driver latency');
  await page.evaluate(()=>window.__touchFrameControl.resume());await page.waitForFunction(()=>window.__clawdY===188);
  const cancelledPeak=await page.evaluate(()=>188-window.__clawdMinY);
  await writeFile(path.join(output,'touch-evidence.json'),JSON.stringify(await page.evaluate(()=>({freeze:window.__touchFreeze,cancelEvents:window.__touchEvents,peak:188-window.__clawdMinY,frames:window.__touchFrames})),null,2));
  assert.ok(cancelledPeak>70&&cancelledPeak<130,'touch down jumps immediately; cancellation stops extra lift');
  await page.waitForTimeout(100);assert.equal(await page.evaluate(()=>window.__clawdY),188,'cancelled touch cannot jump again');
  await touch.detach();await freshHoldRun();
  await dialog.locator('.cg-jump').click();assert.equal(await page.evaluate(()=>window.pendingAnimationFrames()),1);
  await dialog.getByRole('button',{name:'关闭面板',exact:true}).click();
  await page.waitForFunction(()=>window.pendingAnimationFrames()===0);
  assert.equal(await page.getByRole('button',{name:'陪它玩，打开 Clawd 的口袋',exact:true}).evaluate(node=>node===document.activeElement),true);
  await page.getByRole('button',{name:'深色 Night',exact:true}).click();
  await page.setViewportSize({width:900,height:640});await openGame();
  assert.equal(await page.locator('html').getAttribute('data-theme'),'dark');
  assert.equal(await game.evaluate(node=>node.scrollWidth<=node.clientWidth),true);
  assert.equal(await dialog.locator('.cg-jump').isVisible(),true);
  await dialog.screenshot({path:path.join(output,'app-night.png')});
  await dialog.locator('.cg-jump').click();await page.keyboard.press('p');
  assert.equal(await game.getAttribute('data-state'),'paused');
  await dialog.getByRole('button',{name:'关闭面板',exact:true}).click();
  await page.getByRole('button',{name:'跟随系统 System',exact:true}).click();
  await page.emulateMedia({colorScheme:'dark'});await openGame();
  assert.equal(await game.evaluate(node=>getComputedStyle(node).getPropertyValue('--bg').trim().toLowerCase()),'#24231f');
  assert.deepEqual(errors,[]);
  const report={status:'passed',runtime:'built App in isolated Chromium; synthetic inventory, not native WebKit',checks:['shared runner opens via existing pocket menu','existing App best score retained','one RAF while playing; zero after tab switch and modal close','landscape tab remains interactive','remount creates one fresh game','focus returns to pocket trigger','Day/Night, 1120x800 and 900x640, System dark palette','keyboard pause','pointer tap retains original height; pointer and keyboard down jump immediately, holding raises that same jump; pause and touch cancellation clear held input; live language switch preserves active hold and canvas','live zh/en relabeling preserves game/canvas identity, paused phase, score, collected stars, best and RAF count'],screenshots:output};
  await writeFile(path.join(output,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));
}finally{console.log('Clawd UI evidence: '+output);await browser?.close();preview.kill('SIGTERM');}

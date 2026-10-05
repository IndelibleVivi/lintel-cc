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
let browser;
try{
  const url=await new Promise((resolve,reject)=>{let text='';const timer=setTimeout(()=>reject(new Error('preview timeout')),15000);preview.once('error',reject);preview.once('exit',code=>reject(new Error(`preview exited ${code}`)));for(const stream of [preview.stdout,preview.stderr])stream.on('data',chunk=>{text+=chunk;const match=stripVTControlCharacters(text).match(/http:\/\/127\.0\.0\.1:\d+\//);if(match){clearTimeout(timer);resolve(match[0]);}});});
  browser=await chromium.launch({headless:true});const page=await browser.newPage({viewport:{width:1120,height:800},colorScheme:'light'});
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.addInitScript(()=>{
    window.isTauri=true;
    window.__TAURI_INTERNALS__={invoke:async(command,args)=>{
      if(command==='request'&&args.payload.command==='discover')return {ok:true,data:{environments:[],capabilities:[]}};
      if(command==='request'&&args.payload.command==='jobs')return {ok:true,data:{jobs:[]}};
      throw new Error('Unexpected native call in playroom-only fixture: '+command);
    }};
    localStorage.setItem('lintel.clawd.runner.best','123');
    const pending=new Set(),request=window.requestAnimationFrame.bind(window),cancel=window.cancelAnimationFrame.bind(window);
    window.requestAnimationFrame=callback=>{const id=request(time=>{pending.delete(id);callback(time);});pending.add(id);return id;};
    window.cancelAnimationFrame=id=>{pending.delete(id);cancel(id);};
    window.pendingAnimationFrames=()=>pending.size;
  });
  await page.goto(url);
  async function openGame(){await page.getByRole('button',{name:'陪它玩，打开 Clawd 的口袋',exact:true}).click();await page.getByRole('button',{name:'Clawd 跳一跳',exact:true}).click();await page.getByRole('group',{name:'Clawd 跳跃跑道'}).waitFor();}
  await openGame();
  const dialog=page.getByRole('dialog'),game=dialog.locator('.clawd-game');
  assert.equal(await game.count(),1);assert.equal(await game.locator('.cg-best').textContent(),'00123');
  await dialog.screenshot({path:path.join(output,'app-day.png')});
  await dialog.locator('.cg-jump').click();await page.waitForFunction(()=>Number(document.querySelector('.cg-score').textContent)>0);
  assert.equal(await game.getAttribute('data-state'),'running');
  assert.equal(await page.evaluate(()=>window.pendingAnimationFrames()),1);
  await dialog.getByRole('tab',{name:'点阵风景册',exact:true}).click();
  assert.equal(await game.count(),0);assert.equal(await page.evaluate(()=>window.pendingAnimationFrames()),0);
  await dialog.getByRole('button',{name:'第 2 幅：雨夜小屋',exact:true}).click();
  await dialog.getByRole('tab',{name:'跳一小段',exact:true}).click();
  assert.equal(await game.count(),1);assert.equal(await game.getAttribute('data-state'),'ready');
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
  const report={status:'passed',runtime:'built App in isolated Chromium; synthetic inventory, not native WebKit',checks:['shared runner opens via existing pocket menu','existing App best score retained','one RAF while playing; zero after tab switch and modal close','landscape tab remains interactive','remount creates one fresh game','focus returns to pocket trigger','Day/Night, 1120x800 and 900x640, System dark palette','keyboard pause'],screenshots:output};
  await writeFile(path.join(output,'report.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));
}finally{await browser?.close();preview.kill('SIGTERM');}

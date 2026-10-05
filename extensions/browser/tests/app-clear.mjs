// Rendered App + real native host + real Chromium startup, all in the smoke
// test's disposable home. The official origin is fulfilled with synthetic HTML;
// no Claude service or personal profile is contacted.
import assert from 'node:assert/strict';
import http from 'node:http';
import path from 'node:path';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';

export async function appClearJourney({root,profile,state,host,context,launch,extensionId,extensionWorker,browserSnapshot,nativeControl,paired}) {
  const repo=path.resolve(root,'../..'),dist=path.join(repo,'apps/desktop/dist');
  const runner=path.join(repo,'target/debug/lintel');
  execFileSync('cargo',['build','-p','lintel-runner'],{cwd:repo,stdio:'inherit'});
  await mkdir(path.join(profile,'.claude'),{recursive:true});
  await writeFile(path.join(profile,'.claude/settings.json'),'{}\n');
  const env={...process.env,HOME:profile,LINTEL_TEST_HOME:profile,LINTEL_STATE_DIR:path.join(profile,'core-state'),LINTEL_BROWSER_STATE:state};
  const calls=[];
  const server=http.createServer(async(req,res)=>{
    const url=new URL(req.url,'http://localhost');
    if(url.pathname!=='/'&&!/^\/assets\/[\w.-]+$/.test(url.pathname)){res.writeHead(404);res.end();return;}
    try{const file=url.pathname==='/'?'index.html':url.pathname.slice(1);res.setHeader('Content-Type',file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':'text/html');res.end(await readFile(path.join(dist,file)));}
    catch{res.writeHead(404);res.end();}
  });
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  const url=`http://127.0.0.1:${server.address().port}/`;
  const intercept=ctx=>ctx.route('https://claude.ai/**',route=>route.fulfill({contentType:'text/html',body:'<!doctype html><title>Synthetic App clear target</title><p>SYNTHETIC ONLY</p>'}));
  const openApp=async ctx=>{
    const app=await ctx.newPage();
    await app.exposeFunction('actualInvoke',(command,args)=>{
      if(command==='remote_request'&&args.payload.op==='hosts')return {ok:true,data:{hosts:[],tasks:[],installations:[]}};
      assert(['request','browser_request'].includes(command),command);
      calls.push({command,payload:args.payload});
      return JSON.parse(execFileSync(command==='request'?runner:host,[command==='request'?'request':'control'],{env,input:JSON.stringify(args.payload)+'\n',encoding:'utf8'}));
    });
    await app.addInitScript(()=>{window.isTauri=true;window.__TAURI_INTERNALS__={invoke:(command,args)=>window.actualInvoke(command,args)};});
    await app.goto(url);await app.getByRole('button',{name:'浏览器',exact:true}).click();return app;
  };
  const confirm=async popup=>{
    await popup.getByLabel('我确认以上范围与功能影响').check();
    await popup.getByRole('button',{name:'授予必要权限并执行一次',exact:true}).click();
  };
  try {
    await intercept(context);
    const target=await context.newPage();await target.goto('https://claude.ai/');
    await target.evaluate(()=>{localStorage.setItem('fixture','SYNTHETIC');document.cookie='fixture=SYNTHETIC; Path=/; Secure; Max-Age=3600';});
    let popup=await context.newPage();await popup.goto(`chrome-extension://${extensionId}/popup.html`);
    await popup.getByRole('button',{name:'刷新状态',exact:true}).click();
    let app=await openApp(context);
    await app.getByRole('button',{name:'送到浏览器预览',exact:true}).click();
    await app.getByText('等待在浏览器中确认',{exact:true}).waitFor();
    const original=await app.evaluate(()=>JSON.parse(localStorage.getItem('lintel.browser.operation')));
    assert.equal(original.instance_id,paired.instance_id);
    await popup.getByRole('button',{name:'刷新状态',exact:true}).click();
    const originalBox=popup.locator('.receipt').filter({has:popup.locator('small',{hasText:original.id})});
    await originalBox.getByRole('button',{name:'查看来自桌面的待确认操作',exact:true}).click();await confirm(popup);
    await originalBox.getByText('clear · 等待浏览器重启后继续',{exact:true}).waitFor();
    assert(target.isClosed());
    await app.getByRole('button',{name:'查询原任务',exact:true}).click();
    await app.getByText('等待浏览器重启后继续',{exact:true}).waitFor();
    const generation=await popup.evaluate(async()=>(await chrome.storage.local.get('browserStartupGeneration')).browserStartupGeneration);
    const before=await browserSnapshot(context);await context.close();
    assert.throws(()=>process.kill(before.processId,0),{code:'ESRCH'});
    context=await launch();await intercept(context);await extensionWorker(context,extensionId);
    const after=await browserSnapshot(context);assert.notEqual(after.processId,before.processId);assert.deepEqual(after.extensionLoadFlags,[]);
    popup=await context.newPage();await popup.goto(`chrome-extension://${extensionId}/popup.html`);
    await popup.waitForFunction(async old=>(await chrome.storage.local.get('browserStartupGeneration')).browserStartupGeneration!==old,generation);
    await popup.getByRole('button',{name:'刷新状态',exact:true}).click();
    const resumed=popup.locator('.receipt').filter({has:popup.locator('small',{hasText:original.id})});
    await resumed.getByRole('button',{name:'预览重启后继续删除',exact:true}).click();await confirm(popup);
    await resumed.getByText('clear · 已完成',{exact:true}).waitFor();
    await popup.getByRole('button',{name:'刷新状态',exact:true}).click();
    const final=nativeControl({op:'query',instance_id:original.instance_id,operation_id:original.id});
    assert.equal(final.phase,'completed');assert.equal(final.receipt.result.verification,'browser-acknowledged');
    assert(final.receipt.result.continuedBy);assert.notEqual(final.receipt.result.continuedBy,original.id);
    app=await openApp(context);await app.getByRole('button',{name:'查询原任务',exact:true}).click();
    await app.getByText('浏览器已完成',{exact:true}).waitFor();
    assert.equal(calls.filter(call=>call.payload.op==='submit').length,1,'popup continuation must not submit another App operation');
    assert.equal((await app.evaluate(()=>JSON.parse(localStorage.getItem('lintel.browser.operation')))).id,original.id);
    const childId=final.receipt.result.continuedBy;
    const release=popup.locator('.receipt').filter({has:popup.locator('small',{hasText:childId})}).getByRole('button',{name:'已核对结果，解除站点隔离',exact:true});
    await release.click();await release.waitFor({state:'hidden'});
    const clean=await context.newPage();await clean.goto('https://claude.ai/');
    assert.deepEqual(await clean.evaluate(()=>({value:localStorage.getItem('fixture'),cookie:document.cookie})),{value:null,cookie:''});
    await clean.evaluate(()=>{localStorage.setItem('fixture','NEW_SYNTHETIC');document.cookie='fixture=NEW_SYNTHETIC; Path=/; Secure';});
    for(const id of [original.id,childId])assert.equal((await popup.evaluate(id=>chrome.runtime.sendMessage({type:'commit',id}),id)).data.phase,'completed');
    assert.equal(await clean.evaluate(()=>localStorage.getItem('fixture')),'NEW_SYNTHETIC');
    await app.getByRole('button',{name:'WebRTC',exact:true}).click();
    await app.getByRole('button',{name:'送到浏览器预览',exact:true}).click();
    await app.getByText('等待在浏览器中确认',{exact:true}).waitFor();
    const canceled=await app.evaluate(()=>JSON.parse(localStorage.getItem('lintel.browser.operation')));
    await popup.getByRole('button',{name:'刷新状态',exact:true}).click();
    const cancelBox=popup.locator('.receipt').filter({has:popup.locator('small',{hasText:canceled.id})});
    await cancelBox.getByRole('button',{name:'查看来自桌面的待确认操作',exact:true}).click();
    await popup.getByRole('button',{name:'取消这份预览',exact:true}).click();
    await cancelBox.getByText('webrtc · 已取消（未执行）',{exact:true}).waitFor();
    await app.getByRole('button',{name:'查询原任务',exact:true}).click();
    await app.getByText('已取消（未执行）',{exact:true}).waitFor();
    assert.equal(await app.getByRole('button',{name:'送到浏览器预览',exact:true}).isEnabled(),true);
    assert.equal((await popup.evaluate(id=>chrome.runtime.sendMessage({type:'commit',id}),canceled.id)).data.phase,'canceled');
    return {context,evidence:{originalOperationId:original.id,childOperationId:childId,phase:final.phase,priorBrowserExited:true,realStartup:true,source:'rendered App invoke shim; real host/Chromium; locally fulfilled synthetic HTTPS page',clearAppSubmitCount:1,cancellationRoundTrip:true}};
  } catch(error){await context.close();throw error;}
  finally {await new Promise(resolve=>server.close(resolve));}
}

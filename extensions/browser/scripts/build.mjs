import {mkdir,cp,writeFile,readFile} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const fixture=process.argv.includes('--fixture');
for(const browser of ['chromium','firefox']){
 const out=path.join(root,'dist',fixture?`${browser}-fixture`:browser);await mkdir(out,{recursive:true});await cp(path.join(root,'src'),out,{recursive:true});
 const config={browser,fixture,sites:fixture?[{origin:'http://localhost:18765',domain:'localhost',label:'Synthetic target',default:true}]:[{origin:'https://claude.ai',domain:'claude.ai',label:'Claude',default:true},{origin:'https://console.anthropic.com',domain:'anthropic.com',label:'Anthropic Console',default:false}]};
 await writeFile(path.join(out,'config.js'),`export const CONFIG = ${JSON.stringify(config,null,2)};\n`);
 const hostPermissions=fixture?['http://localhost/*']:['https://claude.ai/*','https://*.claude.ai/*','https://anthropic.com/*','https://*.anthropic.com/*'];
 const manifest={manifest_version:3,name:fixture?'Lintel — SYNTHETIC TEST ONLY':'Lintel',version:'0.1.0',description:'Local browser privacy controls, scoped site cleanup, and paired Lintel receipts.',permissions:['storage','browsingData','privacy','nativeMessaging','alarms'],optional_permissions:['cookies','declarativeNetRequest',...(browser==='chromium'?['contentSettings','proxy']:[])],optional_host_permissions:hostPermissions,action:{default_popup:'popup.html',default_title:'Lintel · 当前浏览器'},options_ui:{page:'popup.html',open_in_tab:true},content_security_policy:{extension_pages:"script-src 'self'; object-src 'none'"}};
 if(browser==='chromium'){manifest.minimum_chrome_version='120';manifest.background={service_worker:'background.js',type:'module'};}
 else{manifest.background={scripts:['background.js'],type:'module'};manifest.browser_specific_settings={gecko:{id:'lintel@lintel.local',strict_min_version:'128.0',data_collection_permissions:{required:['none']}}};}
 manifest.optional_permissions.push('webNavigation');
 if(fixture){manifest.permissions.push('cookies','declarativeNetRequest','webNavigation',...(browser==='chromium'?['contentSettings','proxy']:[]));manifest.optional_permissions=manifest.optional_permissions.filter(v=>!manifest.permissions.includes(v));manifest.host_permissions=hostPermissions;manifest.optional_host_permissions=[];}
 await writeFile(path.join(out,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
 console.log(path.relative(root,out));
}

import {useEffect,useRef,useState} from 'react';
import {request} from './api';
import type {Environment,Plan} from './types';
import {useI18n} from './i18n-provider';
import RequestFailure,{asError} from './RequestFailure';
import {formatBytes,Notice} from './ui';

export default function ResidueArchivePanel({send=request,environment,disabled,onPlan}:{send?:typeof request;environment:Environment;disabled:boolean;onPlan:(plan:Plan,passphrase?:string)=>void}){
  const {t}=useI18n();
  const [source,setSource]=useState('');const [sourceKind,setSourceKind]=useState<'job_id'|'archive_path'>('job_id');
  const [passphrase,setPassphrase]=useState('');
  const [manifest,setManifest]=useState<null|{files:{path:string;bytes:number;role:string|null;restore_available?:boolean}[]}>(null);
  const [selected,setSelected]=useState<Set<string>>(new Set());const [busy,setBusy]=useState(false);const [error,setError]=useState<Error|null>(null);
  const generation=useRef(0);const lock=useRef(false);
  useEffect(()=>{generation.current++;setManifest(null);setSelected(new Set());setPassphrase('');setError(null);return()=>{generation.current++;};},[environment.id,environment.root,send]);
  async function run(restore:boolean){
    if(lock.current||!source.trim()||!passphrase||(restore&&!selected.size))return;
    lock.current=true;setBusy(true);setError(null);const current=generation.current;
    const fields={ [sourceKind]:source.trim(),archive_passphrase:passphrase };
    try {
      if(restore){const plan=await send('plan_residue_restore',{...fields,environment_id:environment.id,selected_paths:[...selected].sort()});if(current===generation.current)onPlan(plan,passphrase);}
      else {const result=await send('residue_archive_inspect',fields);if(current===generation.current){setManifest(result);setSelected(new Set());}}
    }catch(e){if(current===generation.current)setError(asError(e));}finally{lock.current=false;if(current===generation.current)setBusy(false);}
  }
  return <details className="surface residue-archive-panel"><summary className="surface-heading"><h2>{t('residueArchive.title')}</h2></summary><div className="padded">
    <p>{t('residueArchive.note')}</p>
    <label className="field">{t('residueArchive.sourceKind')}<select value={sourceKind} disabled={busy||disabled} onChange={e=>{setSourceKind(e.target.value as typeof sourceKind);setManifest(null);}}><option value="job_id">{t('residueArchive.job')}</option><option value="archive_path">{t('residueArchive.path')}</option></select></label>
    <label className="field">{t(sourceKind==='job_id'?'residueArchive.job':'residueArchive.path')}<input value={source} spellCheck={false} disabled={busy||disabled} onChange={e=>{setSource(e.target.value);setManifest(null);setSelected(new Set());}}/></label>
    <label className="field">{t('residueArchive.passphrase')}<input type="password" autoComplete="off" value={passphrase} disabled={busy||disabled} onChange={e=>{setPassphrase(e.target.value);setManifest(null);setSelected(new Set());}}/></label>
    <button disabled={busy||disabled||!source.trim()||!passphrase} onClick={()=>void run(false)}>{t(busy?'residueArchive.reading':'residueArchive.read')}</button>
    {error&&<RequestFailure error={error} context={t('residueArchive.title')}/>}
    {manifest&&<><ul className="residue-entry-list">{manifest.files.map(file=><li key={file.path}><input type="checkbox" checked={selected.has(file.path)} disabled={busy||disabled||file.restore_available===false} aria-label={t('residueArchive.select',{path:file.path})} onChange={e=>setSelected(current=>{const next=new Set(current);e.target.checked?next.add(file.path):next.delete(file.path);return next;})}/><code>{file.path}</code><small>{formatBytes(file.bytes)} · {file.role??''}{file.restore_available===false&&` · ${t('residueArchive.occupied')}`}</small></li>)}</ul><Notice>{t('residueArchive.restoreNote')}</Notice><button disabled={busy||disabled||!selected.size} onClick={()=>void run(true)}>{t('residueArchive.preview',{count:selected.size})}</button></>}
  </div></details>;
}

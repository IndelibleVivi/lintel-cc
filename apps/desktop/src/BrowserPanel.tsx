import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { RequestError, transport } from './api';
import { Icon, Notice } from './ui';
import Clawd from './Clawd';
import BrowserHostSetup from './BrowserHostSetup';
import { ResourceLink } from './Resources';
import RequestFailure, {asError} from './RequestFailure';
import { useI18n } from './i18n-provider';

type Instance = { instance_id: string; label: string; browser: string; extension_id: string; paired: boolean; conflict: boolean; last_seen: number; online: boolean };
type SavedOperation = {id:string;instance_id:string;kind?:string};
function history(): SavedOperation[] { try { const saved = JSON.parse(localStorage.getItem('lintel.browser.history') ?? 'null'); const previous = JSON.parse(localStorage.getItem('lintel.browser.operation') ?? 'null'); return saved ?? (previous ? [previous] : []); } catch { return []; } }
type Pairing = { code: string; expires_at: number };
type Pending = { challenge: string; code: string; instance_id: string; label: string; browser: string; extension_id: string };
type Operation = { instance_id: string; id: string; phase: string; action: { kind: string; setting?: string; receiptId?: string }; created_at: number; receipt?: { id: string; phase: string; result?: Record<string, unknown>; error?: { code: string; message: string } } };
async function browserRequest<T>(payload: Record<string, unknown>): Promise<T> {
  const result = await invoke<{ok:true;data:T}|{ok:false;error:{code:string;message:string}}>('browser_request', { payload });
  if (!result.ok) throw new RequestError(result.error.code, result.error.message);
  return result.data;
}
const phaseKeys = (phase: string) => ({ 'awaiting-browser-confirmation': 'bpanel.phaseAwaitingConfirm', 'awaiting-browser-restart': 'bpanel.phaseAwaitingRestart', running: 'bpanel.phaseRunning', completed: 'bpanel.phaseCompleted', uncertain: 'bpanel.phaseUncertain', failed: 'bpanel.phaseFailed', canceled: 'bpanel.phaseCanceled', expired: 'bpanel.phaseExpired' } as Record<string,string>)[phase];
export default function BrowserPanel() {
  const { t, formatDateTime } = useI18n();
  const phaseName = (phase: string) => { const key = phaseKeys(phase); return key ? t(key) : phase; };
  const [instances, setInstances] = useState<Instance[]>([]);
  const [pairing, setPairing] = useState<Pairing | null>(null);
  const [pending, setPending] = useState<Pending[]>([]);
  const [operation, setOperation] = useState<Operation | null>(null);
  const [selectedInstance, setSelectedInstance] = useState('');
  const [kind, setKind] = useState('clear');
  const [origins, setOrigins] = useState(['https://claude.ai']);
  const [types, setTypes] = useState(['cookies', 'localStorage', 'indexedDB', 'serviceWorkers']);
  const [permission, setPermission] = useState('location');
  const [port, setPort] = useState('8080');
  const [minutes, setMinutes] = useState('10');
  const [cookieStoreId, setCookieStoreId] = useState('');
  const [uncertain, setUncertain] = useState(false);
  const [setting, setSetting] = useState('disable_non_proxied_udp');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<Error|null>(null);
  const [note, setNote] = useState<''|'submitted'|'copiedCode'|'approvedPairing'|'installedVerified'|'installedUpdated'|'installedNew'>('');
  const feedback = useRef<HTMLDivElement>(null);
  const [operationHistory, setOperationHistory] = useState<SavedOperation[]>(history);
  const [savedOperation, setSavedOperation] = useState<{id:string;instance_id:string}|null>(() => { try { return JSON.parse(localStorage.getItem('lintel.browser.operation') ?? 'null'); } catch { return null; } });
  async function refresh() {
    const [items, requests] = await Promise.all([browserRequest<Instance[]>({ op: 'instances' }), browserRequest<Pending[]>({ op: 'pair_pending' })]);
    setInstances(items); setPending(requests); setSelectedInstance(current => items.some(item => item.instance_id === current) ? current : items[0]?.instance_id ?? '');
  }
  const actionLock=useRef(false);
  async function run(work: () => Promise<void>) { if(actionLock.current)return;actionLock.current=true;setBusy(true); setError(null); setNote(''); try { await work(); } catch (error) { setError(asError(error)); } finally { actionLock.current=false;setBusy(false); } }
  useEffect(() => { if (transport === 'native') void run(refresh); }, []);
  useEffect(() => { if (error || note) feedback.current?.scrollIntoView({ block: 'nearest' }); }, [error, note]);
  async function query(chosen?: SavedOperation) { const target = chosen ?? operation ?? savedOperation; if (target) { setSavedOperation(target); setOperation(await browserRequest<Operation>({ op: 'query', instance_id: target.instance_id, operation_id: target.id })); setUncertain(false); } }
  async function submit(restore = false, finishClear = false) {
    const instanceId = (restore || finishClear) && operation ? operation.instance_id : selectedInstance;
    const id = crypto.randomUUID();
    const receiptId = operation?.id;
    const saved = { id, instance_id: instanceId, kind:finishClear ? 'finishClear' : restore ? 'restore' : kind }; setSavedOperation(saved); localStorage.setItem('lintel.browser.operation', JSON.stringify(saved)); const records=[saved,...operationHistory]; setOperationHistory(records); localStorage.setItem('lintel.browser.history',JSON.stringify(records)); setOperation(null); setUncertain(true);
    const action = finishClear ? { kind:'finishClear', receiptId } : restore ? { kind: 'restore', receiptId } : kind === 'clear' ? { kind, origins, types, ...(cookieStoreId.trim() && firefox ? { cookieStoreId: cookieStoreId.trim() } : {}) } : kind === 'webrtc' ? { kind, setting } : kind === 'sitePermission' ? { kind, origins, setting: permission } : kind === 'proxy' ? { kind, origins, port: Number(port) } : kind === 'blockSites' ? { kind, origins } : kind === 'pauseRules' ? { kind, minutes: Number(minutes) } : { kind };
    const result = await browserRequest<Operation>({ op: 'submit', instance_id: instanceId, operation_id: id, action }); setOperation(result); setUncertain(false); setNote('submitted');
  }
  const selected = instances.find(item => item.instance_id === selectedInstance);
  const native = transport === 'native';
  const firefox = !!selected?.browser.toLowerCase().includes('firefox');
  const needsOrigins = ['clear','sitePermission','proxy','blockSites'].includes(kind);
  const awaiting = uncertain || (!!savedOperation && (!operation || !['completed','failed','canceled','expired','rejected'].includes(operation.phase)));
  const unsupported = firefox && ['sitePermission','proxy'].includes(kind);
  const invalid = (needsOrigins && !origins.length) || (kind === 'clear' && (!types.length || (firefox && types.some(type => ['cacheStorage','cache'].includes(type))))) || (kind === 'proxy' && (!Number.isInteger(Number(port)) || Number(port) < 1 || Number(port) > 65535)) || (kind === 'pauseRules' && (!Number.isInteger(Number(minutes)) || Number(minutes) < 1 || Number(minutes) > 60));
  return <section className="browser-panel"><div className="browser-local-scope"><Icon name="terminal" size={16}/><strong>{t('bpanel.localScope')}</strong><span>{t('bpanel.localScopeNote')}</span></div><div className="module-intro"><Clawd small mood="care"/><div><h3>{t('bpanel.introTitle')}</h3><p>{t('bpanel.introNote')}</p></div></div>{!native && <Notice>{t('bpanel.syntheticNote')}</Notice>}<div className="browser-panel-heading"><h3>{t('bpanel.companionTitle')}</h3><button disabled={busy || !native} onClick={() => void run(refresh)}><Icon name="refresh" size={14}/>{t('bpanel.checkPairing')}</button></div><p>{t('bpanel.pairingNote')}</p>
    {error && <div ref={feedback}><RequestFailure error={error} context={t('bpanel.context')}/></div>}{note && <div ref={feedback} role="status"><Notice>{t(`bpanel.note.${note}`)}</Notice></div>}
    <details className="browser-setup"><summary>{t('bpanel.connectNew')}</summary><p className="small-print">{t('bpanel.connectNote1')} <ResourceLink resource="browser-setup">{t('bpanel.browserGuide')}</ResourceLink>{t('bpanel.connectNote2')}</p><BrowserHostSetup native={native} busy={busy} request={browserRequest} run={run} onInstalled={setNote}/><h4 className="browser-setup-step browser-pairing-heading"><span>3</span>{t('bpanel.verifyProfile')}</h4><div className="button-row section-action"><button disabled={busy || !native} onClick={() => void run(async () => { setPairing(await browserRequest<Pairing>({ op: 'pair_create' })); })}>{t('bpanel.createPairing')}</button></div>{pairing && <div className="pairing-code"><span>{t('bpanel.pairingCode')}</span><strong>{pairing.code}</strong><p>{t('bpanel.pairingCodeNote',{time:formatDateTime(String(pairing.expires_at))})}</p><button className="text-button" onClick={() => void run(async () => { await navigator.clipboard.writeText(pairing.code); setNote('copiedCode'); })}><Icon name="copy" size={14}/>{t('bpanel.copyCode')}</button></div>}{pending.map(item => <div className="pending-pair" key={item.challenge}><strong>{item.label} · {item.browser}</strong><p>{t('bpanel.pendingLine',{code:item.code,instance:item.instance_id})}</p><code>{item.extension_id}</code><button disabled={busy || !native} onClick={() => void run(async () => { await browserRequest({ op: 'pair_approve', challenge: item.challenge }); setPairing(null); await refresh(); setNote('approvedPairing'); })}>{t('bpanel.approveProfile')}</button></div>)}</details>
    <div className="browser-instances">{instances.length === 0 ? <p className="empty-inline">{t('bpanel.noInstances')}</p> : instances.map(item => <div className="browser-instance" key={item.instance_id}><div><strong>{item.label}</strong><span>{item.browser} · {t(item.paired ? 'bpanel.paired' : 'bpanel.unpaired')} · {t(item.online ? 'bpanel.online' : 'bpanel.offline')}</span><code>{item.instance_id}</code></div>{item.conflict && <span className="conflict-label">{t('bpanel.conflict')}</span>}</div>)}</div>
    <div className="browser-actions"><label className="field">{t('bpanel.targetProfile')}<select disabled={!instances.length || busy} value={selectedInstance} onChange={event => setSelectedInstance(event.target.value)}>{!instances.length && <option value="">{t('bpanel.waitingProfile')}</option>}{instances.map(item => <option key={item.instance_id} value={item.instance_id}>{item.label} · {item.browser}</option>)}</select></label>
      <div className="browser-kind-picker" role="group" aria-label={t('bpanel.actionsAria')}>{[['clear','bpanel.kindClear'],['webrtc','bpanel.kindWebrtc'],['sitePermission','bpanel.kindSitePermission'],['proxy','bpanel.kindProxy'],['blockSites','bpanel.kindBlockSites'],['pauseRules','bpanel.kindPauseRules'],['clearProfileCache','bpanel.kindClearProfileCache']].map(([value,key]) => <button aria-pressed={kind === value} key={value} onClick={() => setKind(value)}>{t(key)}</button>)}</div>
      {needsOrigins && <fieldset className="compact-options"><legend>{t('bpanel.targetSites')}</legend>{['https://claude.ai','https://console.anthropic.com'].map(origin => <label key={origin}><input type="checkbox" checked={origins.includes(origin)} onChange={event => setOrigins(current => event.target.checked ? [...current,origin] : current.filter(item => item !== origin))}/><code>{origin}</code></label>)}</fieldset>}
      {kind === 'clear' && <><fieldset className="compact-options"><legend>{t('bpanel.dataTypes')}</legend>{[['cookies','bpanel.cookieLogin'],['localStorage','bpanel.localStorage'],['indexedDB','bpanel.indexedDB'],['serviceWorkers','bpanel.serviceWorkers'],['cacheStorage','bpanel.cacheStorage'],['cache','bpanel.httpCache']].map(([id,key]) => <label key={id}><input type="checkbox" checked={types.includes(id)} disabled={firefox && ['cacheStorage','cache'].includes(id)} onChange={event => setTypes(current => event.target.checked ? [...current,id] : current.filter(item => item !== id))}/>{t(key)}{firefox && ['cacheStorage','cache'].includes(id) ? t('bpanel.unsupportedSiteClear') : ''}</label>)}</fieldset>{firefox && <label className="field">{t('bpanel.firefoxStore')}<input value={cookieStoreId} onChange={event => setCookieStoreId(event.target.value)} placeholder={t('bpanel.firefoxStorePlaceholder')}/><small>{t('bpanel.firefoxStoreNote')}</small></label>}<Notice tone="warning">{t('bpanel.clearWarning')}</Notice></>}
      {kind === 'webrtc' && <><p>{t('bpanel.webrtcNote')}</p><label className="field">{t('bpanel.policy')}<select value={setting} onChange={event => setSetting(event.target.value)}><option value="disable_non_proxied_udp">{t('bpanel.webrtcLimitedUdp')}</option><option value="default_public_interface_only">{t('bpanel.webrtcPublicOnly')}</option><option value="default">{t('bpanel.webrtcDefault')}</option></select></label></>}
      {kind === 'sitePermission' && <label className="field">{t('bpanel.blockPermission')}<select value={permission} onChange={event => setPermission(event.target.value)}>{[['location','bpanel.permLocation'],['camera','bpanel.permCamera'],['microphone','bpanel.permMicrophone'],['notifications','bpanel.permNotifications']].map(([value,key]) => <option key={value} value={value}>{t(key)}</option>)}</select></label>}
      {kind === 'proxy' && <label className="field">{t('bpanel.proxyPort')}<input type="number" min={1} max={65535} value={port} onChange={event => setPort(event.target.value)}/><small>{t('bpanel.proxyPortNote')}</small></label>}
      {kind === 'blockSites' && <Notice tone="warning">{t('bpanel.blockSitesWarning')}</Notice>}
      {kind === 'pauseRules' && <label className="field">{t('bpanel.pauseMinutes')}<input type="number" min={1} max={60} value={minutes} onChange={event => setMinutes(event.target.value)}/><small>{t('bpanel.pauseMinutesNote')}</small></label>}
      {kind === 'clearProfileCache' && <Notice tone="warning">{t('bpanel.clearCacheWarning')}</Notice>}
      {unsupported && <Notice>{t('bpanel.unsupportedNote')}</Notice>}
      {awaiting && <Notice>{t('bpanel.awaitingNote')}</Notice>}
      <button className="primary section-action" disabled={busy || !native || !selected?.paired || selected.conflict || !selected.online || unsupported || invalid || awaiting} onClick={() => void run(() => submit())}>{t('bpanel.sendPreview')}<Icon name="arrow" size={14}/></button><p className="small-print">{t('bpanel.sendNote')}</p>
    </div>
    {(operation || savedOperation) && <div className="browser-operation"><div className="browser-panel-heading"><h4>{operation ? phaseName(operation.phase) : t('bpanel.hasSubmitted')}</h4><button disabled={busy || !native} onClick={() => void run(() => query())}>{t('app.queryOriginalTask')}</button></div><code>{operation?.id ?? savedOperation?.id}</code>{operationHistory.length > 1 && <label className="field">{t('bpanel.otherTasks')}<select value={savedOperation?.id ?? operation?.id} disabled={busy || !native} onChange={event => { const record=operationHistory.find(item=>item.id===event.target.value)!; setOperation(null); void run(() => query(record)); }}>{operationHistory.map(item=><option key={item.id} value={item.id}>{item.kind ?? t('bpanel.operation')} · {item.id}</option>)}</select></label>}{operation?.receipt?.result && <dl className="browser-result">{Object.entries(operation.receipt.result).map(([key,value]) => <div className="browser-result-field" key={key}><dt>{({verification:'bpanel.resultVerification',configured:'bpanel.resultConfigured',effective:'bpanel.resultEffective',controller:'bpanel.resultController',scope:'bpanel.resultScope',restored:'bpanel.resultRestored'} as Record<string,string>)[key] ? t(({verification:'bpanel.resultVerification',configured:'bpanel.resultConfigured',effective:'bpanel.resultEffective',controller:'bpanel.resultController',scope:'bpanel.resultScope',restored:'bpanel.resultRestored'} as Record<string,string>)[key]) : key}</dt><dd>{typeof value === 'string' ? value : JSON.stringify(value)}</dd></div>)}</dl>}{operation?.receipt?.error && <Notice tone="warning">{t('app.diagnosticWithCode',{message:operation.receipt.error.message,code:operation.receipt.error.code})}</Notice>}{operation?.phase === 'awaiting-browser-restart' && <Notice tone="warning">{t('bpanel.restartNote')}<button className="section-action" disabled={busy || !native} onClick={() => void run(() => submit(false, true))}>{t('bpanel.sendRestartContinue')}</button></Notice>}{operation?.phase === 'completed' && ['webrtc','sitePermission','proxy','blockSites'].includes(operation.action.kind) && <button disabled={busy || !native} onClick={() => void run(() => submit(true))}>{t('bpanel.submitRestore')}</button>}</div>}
  </section>;
}

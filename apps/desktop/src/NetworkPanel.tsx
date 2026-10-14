import RequestFailure, { asError } from './RequestFailure';
import { LocalizedError } from './i18n';
import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { transport } from './api';
import { Icon, Notice, Status } from './ui';
import { useI18n } from './i18n-provider';
import IpNetworkPanel from './IpNetworkPanel';
import TelemetryPanel from './TelemetryPanel';
import telemetryCatalog from '../../../contracts/telemetry-destinations.json';
import type { Inspection, Plan, Receipt } from './types';

type Rule = { host: string; ports: number[] };
type Config = {
  environment_id: string; bind: string; default_action: 'allow' | 'deny';
  blocked: Rule[]; allowed: Rule[]; upstream: string | null;
  address_family: 'system' | 'ipv4_only';
  max_connections: number; connect_timeout_seconds: number; connection_lifetime_seconds: number;
};
type Connection = { timestamp_unix_ms: number; destination_host: string | null; destination_port: number | null; outcome: string; decision: string; peer_family?: string; origin?: 'rule_test' | null; test_id?: string | null };
type Channel = { running: boolean; address: string | null; channel_binding?: string; active_config: Config | null; events: Connection[]; coverage: string; direct_connections_enforced: false; message?: string };
type Envelope<T> = { ok: true; data: T } | { ok: false; error: { code: string; message: string } };
type Draft = { defaultAction: Config['default_action']; blocked: string; allowed: string; upstream: string; addressFamily: Config['address_family']; limits?: Pick<Config, 'max_connections' | 'connect_timeout_seconds' | 'connection_lifetime_seconds'>; telemetry?: string[] };
// Keep edits for each environment while this webview is open, without persisting connection details.
const drafts = new Map<string, Draft>();
const emptyDraft = (): Draft => ({ defaultAction: 'allow', blocked: '', allowed: '', upstream: '', addressFamily: 'system', telemetry: [] });
const TELEMETRY_RULES = telemetryCatalog.destinations.filter(entry => entry.blockable);
// Merge selected telemetry destinations into the blocked draft text without
// dropping any user rule and without duplicating a host already present.
function withTelemetryRules(blockedText: string, ids: string[]): string {
  const lines = blockedText.split('\n').map(line => line.trim()).filter(Boolean);
  const rules = parseRules(blockedText);
  for (const id of ids) {
    const entry = TELEMETRY_RULES.find(value => value.id === id);
    if (entry && !rules.some(rule => rule.host.toLowerCase().replace(/\.$/, '') === entry.host && (!rule.ports.length || rule.ports.includes(entry.port)))) {
      lines.push(`${entry.host} ${entry.port}`);
      rules.push({host: entry.host, ports: [entry.port]});
    }
  }
  return lines.join('\n');
}
const ruleText = (rules: Rule[]) => rules.map(rule => `${rule.host}${rule.ports.length ? ` ${rule.ports.join(',')}` : ''}`).join('\n');
function fromConfig(config: Config): Draft {
  return {
    defaultAction: config.default_action, blocked: ruleText(config.blocked), allowed: ruleText(config.allowed), upstream: config.upstream ?? '', addressFamily: config.address_family ?? 'system',
    limits: { max_connections: config.max_connections, connect_timeout_seconds: config.connect_timeout_seconds, connection_lifetime_seconds: config.connection_lifetime_seconds },
  };
}
// Keep the finite validation identity; render its wording in the current locale.
function parseRules(value: string): Rule[] {
  return value.split('\n').map(line => line.trim()).filter(Boolean).map(line => {
    const [host, portsText, extra] = line.split(/\s+/);
    if (extra || (portsText && !/^\d+(,\d+)*$/.test(portsText))) throw new LocalizedError('netPanel.ruleError');
    const ports = portsText ? portsText.split(',').map(Number) : [];
    if (ports.some(port => !Number.isInteger(port) || port < 1 || port > 65535)) throw new LocalizedError('netPanel.rulePortError');
    return { host, ports };
  });
}
async function network<T>(payload: object): Promise<T> {
  const result = await invoke<Envelope<T>>('network_request', { payload });
  if (!result.ok) throw new Error(result.error.message);
  return result.data;
}
function RuleReadback({ rules }: { rules: Rule[] }) {
  const { t } = useI18n();
  return rules.length ? <ul className="compact-list">{rules.map((rule, index) => <li key={index}><code>{rule.host}</code> · {rule.ports.length ? t('netPanel.ports',{ports:rule.ports.join(t('list.separator'))}) : t('netPanel.allPorts')}</li>)}</ul> : <span>{t('service.none')}</span>;
}
// The key owns the whole request lifetime, so even start/stop replies from the
// previous environment cannot update the newly selected environment's panel.
type Props = { environmentId: string; executable: boolean; onLaunch:(proxyUrl:string)=>void; hostAlias:string|null; receipts:Receipt[]; disabled:boolean; onPlan:(plan:Plan)=>void;onReceipt:(receipt:Receipt)=>void; settings?:Inspection['settings']; onPrivacySettings?:()=>void };
export default function NetworkPanel(props: Props) {
  const { t } = useI18n();
  return (props.hostAlias || !props.environmentId) ? <><IpNetworkPanel key={`${props.hostAlias}:${props.environmentId}`} {...props} proxyUrl={null} proxyBinding={null}/><Notice>{t('netPanel.readonlyNotice')}</Notice><TelemetryPanel key={`telemetry:${props.hostAlias}:${props.environmentId}`} hostAlias={props.hostAlias} selected={[]} onChange={()=>{}} disabled settings={props.settings} onPrivacySettings={props.onPrivacySettings}/></> : <EnvironmentNetworkPanel key={props.environmentId} {...props}/>;
}
function EnvironmentNetworkPanel({ environmentId, executable,onLaunch, ...props }: Props) {
  const { t } = useI18n();
  const [channel, setChannel] = useState<Channel | null>(null);
  const [draft, setDraft] = useState<Draft>(() => drafts.get(environmentId) ?? emptyDraft());
  const [busy, setBusy] = useState(transport === 'native');
  const [message, setMessage] = useState('');
  const [error, setError] = useState<Error | null>(null);
  const mounted = useRef(false);
  function updateDraft(value: Partial<Draft>) {
    setDraft(current => { const next = { ...current, ...value }; drafts.set(environmentId, next); return next; });
  }
  function receive(value: Channel) {
    setChannel(value); setMessage(value.message ?? '');
    if (value.active_config) {
      const adopted = fromConfig(value.active_config);
      drafts.set(environmentId, adopted); setDraft(adopted);
    }
  }
  useEffect(() => {
    mounted.current = true;
    let active = true;
    if (transport === 'native') network<Channel>({ op: 'status', environment_id: environmentId })
      .then(value => { if (active) receive(value); })
      .catch(err => { if (active) setError(asError(err)); })
      .finally(() => { if (active) setBusy(false); });
    return () => { active = false; mounted.current = false; };
  }, [environmentId]);
  async function act(op: 'start' | 'stop' | 'status') {
    setBusy(true); setError(null); setMessage('');
    try {
      const payload = { op, environment_id: environmentId, ...(op === 'start' ? { config: { ...draft.limits, default_action: draft.defaultAction, blocked: parseRules(withTelemetryRules(draft.blocked, draft.telemetry ?? [])), allowed: parseRules(draft.allowed), upstream: draft.upstream.trim() || null, address_family: draft.addressFamily } } : {}) };
      const result = await network<Channel>(payload); if (mounted.current) receive(result);
    } catch (err) { if (mounted.current) setError(asError(err)); }
    finally { if (mounted.current) setBusy(false); }
  }
  // Real controlled test on the current running channel. The native op returns
  // the verified outcome; a failure is surfaced, never treated as a pass.
  async function ruleTest(id: string, binding: string) {
    const result = await network<{ outcome: { result: string; explicit_block: boolean; connection_attempted: boolean; provenance: string; destination_host: string; destination_port: number; test_id: string } }>({ op: 'rule_test', environment_id: environmentId, channel_binding: binding, telemetry_id: id });
    const status = await network<Channel>({ op: 'status', environment_id: environmentId });
    if (mounted.current) receive(status);
    return result.outcome;
  }
  const active = channel?.running ? channel.active_config : null;
  // Catalog ids currently explicitly blocked by the *running* channel.
  const activeBlocked = active
    ? TELEMETRY_RULES.filter(entry => active.blocked.some(rule => rule.host === entry.host && (!rule.ports.length || rule.ports.includes(entry.port)))).map(entry => entry.id)
    : [];
  const locked = busy || props.disabled || !channel || !!channel.address;
  return <><IpNetworkPanel {...props} proxyBinding={channel?.running ? channel.channel_binding ?? null : null} disabled={props.disabled || busy} proxyUrl={channel?.running && channel.address ? `http://${channel.address}` : null}/><section className="network-panel">
    <div className="surface-heading"><h3>{t('netPanel.channelTitle')}</h3><Status value={channel?.running ? 'available' : channel ? 'not_run' : 'unknown'}/></div>
    <div className="padded"><p>{t('netPanel.channelIntro')}</p>
      {transport !== 'native' ? <Notice>{t('netPanel.noNative')}</Notice> : <>
        {active && <section aria-label={t('netPanel.activeAria')}>
          <h4>{t('netPanel.activeTitle')}</h4><p className="small-print">{t('netPanel.activeNote')}</p>
          <div className="fact-row"><span>{t('netPanel.defaultAction')}</span><strong>{t(active.default_action === 'deny' ? 'netPanel.denyUnmatched' : 'netPanel.allowUnmatched')}</strong></div>
          <div className="fact-row"><span>{t('netPanel.explicitBlocked')}</span><RuleReadback rules={active.blocked}/></div>
          <div className="fact-row"><span>{t('netPanel.explicitAllowed')}</span><RuleReadback rules={active.allowed}/></div>
          <div className="fact-row"><span>{t('netPanel.upstream')}</span><code className="full-path">{active.upstream ?? t('netPanel.noUpstream')}</code></div>
          <div className="fact-row"><span>{t('netPanel.addressFamily',{path:t(active.upstream ? 'netPanel.pathUpstream' : 'netPanel.pathTarget')})}</span><strong>{t(active.address_family === 'ipv4_only' ? 'netPanel.strictIpv4' : 'netPanel.followSystem')}</strong></div>
          <p className="small-print">{t('netPanel.ruleNote')}</p>
        </section>}
        {channel?.address && !channel.running && <Notice tone="warning">{t('netPanel.staleChannel')}</Notice>}
        <details className="network-options"><summary>{t('netPanel.draftTitle')}</summary>
          <p className="small-print">{t(channel?.address ? 'netPanel.draftNoteStopped' : 'netPanel.draftNote')}</p>
          <label className="field">{t('netPanel.defaultAction')}<select disabled={locked} value={draft.defaultAction} onChange={e => updateDraft({ defaultAction: e.target.value as Config['default_action'] })}><option value="allow">{t('netPanel.allowUnmatched')}</option><option value="deny">{t('netPanel.denyUnmatched')}</option></select></label>
          <label className="field">{t('netPanel.blockedRules')}<textarea rows={3} disabled={locked} value={draft.blocked} onChange={e => updateDraft({ blocked: e.target.value })} placeholder={t('netPanel.rulePlaceholder')} spellCheck={false}/><small>{t('netPanel.blockedNote')}</small></label>
          <label className="field">{t('netPanel.allowedRules')}<textarea rows={3} disabled={locked} value={draft.allowed} onChange={e => updateDraft({ allowed: e.target.value })} placeholder={t('netPanel.allowedPlaceholder')} spellCheck={false}/><small>{t('netPanel.allowedNote')}</small></label>
          <label className="field">{t('netPanel.upstreamLabel')}<input disabled={locked} value={draft.upstream} onChange={e => updateDraft({ upstream: e.target.value })} placeholder={t('netPanel.upstreamPlaceholder')} spellCheck={false}/><small>{t('netPanel.upstreamNote')}</small></label>
          <label className="field">{t('netPanel.addressFamilyLabel')}<select aria-label={t('netPanel.addressFamilyLabel')} disabled={locked} value={draft.addressFamily} onChange={event => updateDraft({ addressFamily: event.target.value as Config['address_family'] })}><option value="system">{t('netPanel.familySystem')}</option><option value="ipv4_only">{t('netPanel.familyIpv4')}</option></select><small>{t('netPanel.familyNote')}</small></label>
          {draft.limits && <p className="small-print">{t('netPanel.limits',{connections:draft.limits.max_connections,timeout:draft.limits.connect_timeout_seconds,lifetime:draft.limits.connection_lifetime_seconds})}</p>}
          <TelemetryPanel hostAlias={props.hostAlias} selected={draft.telemetry ?? []} disabled={locked} settings={props.settings} onPrivacySettings={props.onPrivacySettings} activeBinding={channel?.running ? channel.channel_binding ?? null : null} activeBlocked={activeBlocked} onRuleTest={ruleTest} onChange={ids => updateDraft({ telemetry: ids })}/>
        </details>
        {channel?.address && <div className="fact-row"><span>{t(channel.running ? 'netPanel.localAddress' : 'netPanel.oldAddress')}</span><code>http://{channel.address}</code></div>}
        <div className="button-row network-controls">{channel?.address ? <button disabled={busy || props.disabled} onClick={() => void act('stop')}>{t(channel.running ? 'netPanel.stopChannel' : 'netPanel.clearOld')}</button> : <button disabled={busy || props.disabled || !channel} onClick={() => void act('start')}>{t('netPanel.startChannel')}</button>}<button className="primary" disabled={busy || props.disabled || !channel?.running || !channel.address || !executable} onClick={() => onLaunch('http://' + channel!.address)}>{t('netPanel.openViaChannel')}<Icon name="arrow" size={15}/></button><button className="text-button" disabled={busy || props.disabled} onClick={() => void act('status')}>{t('netPanel.refreshConnections')}</button></div>
        <h4>{t('netPanel.clientLog')}</h4>
        {channel?.events.some(event => event.origin !== 'rule_test') ? <div className="connection-log">{channel.events.filter(event => event.origin !== 'rule_test').slice(-12).reverse().map((event,index) => <div key={`${event.timestamp_unix_ms}-${index}`}><code>{event.destination_host ?? t('netPanel.unknownTarget')}{event.destination_port ? `:${event.destination_port}` : ''}</code><span>{event.outcome === 'blocked' ? t('netPanel.blocked') : event.outcome}{event.peer_family ? t('netPanel.actualConnection',{family:event.peer_family === 'ipv4' ? 'IPv4' : 'IPv6'}) : ''}</span></div>)}</div> : <p className="small-print">{t('netPanel.noClientLog')}</p>}
        {channel?.events.some(event => event.origin === 'rule_test') && <section aria-label={t('netPanel.controlledAria')}><h4>{t('netPanel.controlledTitle')}</h4><div className="connection-log">{channel.events.filter(event => event.origin === 'rule_test').slice(-12).reverse().map(event => <div key={event.test_id}><code>{event.destination_host}:{event.destination_port}</code><span>{event.outcome} · {event.test_id}</span></div>)}</div></section>}
      </>}
      {message && <p role="status" className="network-message">{message}</p>}{error && <RequestFailure error={error}/>}
      <p className="small-print">{t('netPanel.footer')}</p>
    </div>
  </section></>;
}

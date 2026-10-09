import { useEffect, useMemo, useState } from 'react';
import { requester, transport } from './api';
import { Notice, Status } from './ui';
import { ResourceLink } from './Resources';
import type { Inspection, TelemetryCatalog } from './types';
import './telemetry.css';

type Props = {
  hostAlias: string | null;
  /** Selected catalog ids merged into the stopped-channel blocked draft. */
  selected: string[];
  onChange: (ids: string[]) => void;
  disabled?: boolean;
  /** Observed Claude settings, when the caller has a real `inspect` result. */
  settings?: Inspection['settings'];
  /** Ask the caller to open the existing privacy-settings review (no mutation). */
  onPrivacySettings?: () => void;
  /** Exact running channel binding, when a channel is live. */
  activeBinding?: string | null;
  /** Catalog ids currently explicitly blocked by the running channel. */
  activeBlocked?: string[];
  /** Run a real controlled test on the current running channel. */
  onRuleTest?: (id: string, binding: string) => Promise<ControlledOutcome>;
};

// A controlled test outcome as returned by the native `rule_test` op. It is
// shown separately from observed client traffic and from settings readback.
export type ControlledOutcome = {
  result: string; explicit_block: boolean; connection_attempted: boolean;
  provenance: string; destination_host: string; destination_port: number; test_id: string;
};

// The observed NONESSENTIAL total-switch value is shown separately from the
// fine-grained switches. A missing readback is `not_read`, never "disabled".
const NONESSENTIAL = 'CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC';
const FINE_GRAINED = ['DISABLE_TELEMETRY', 'DISABLE_ERROR_REPORTING', 'CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY', 'DISABLE_FEEDBACK_COMMAND', 'DO_NOT_TRACK', 'DISABLE_GROWTHBOOK'];

// The telemetry catalog is static metadata from the shared contract. This panel
// only reads it and lets the user select blockable destinations into the
// existing stopped-channel draft; it never starts or changes a running channel.
export default function TelemetryPanel({ hostAlias, selected, onChange, disabled, settings, onPrivacySettings, activeBinding, activeBlocked, onRuleTest }: Props) {
  const send = useMemo(() => requester(hostAlias), [hostAlias]);
  const [catalog, setCatalog] = useState<TelemetryCatalog | null>(null);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(transport !== 'synthetic');
  // Controlled-test results are keyed by the exact channel binding so a stop or
  // replacement cannot show a stale pass for a new instance.
  const [tested, setTested] = useState<{ binding: string; id: string; outcome: ControlledOutcome | null; error?: string } | null>(null);
  const [testBusy, setTestBusy] = useState('');

  useEffect(() => {
    let active = true;
    setBusy(true); setError('');
    send('telemetry_catalog', {})
      .then(value => { if (active) setCatalog(value); })
      .catch(err => { if (active) setError(err instanceof Error ? err.message : String(err)); })
      .finally(() => { if (active) setBusy(false); });
    return () => { active = false; };
  }, [hostAlias, send]);

  const blockable = catalog?.destinations.filter(entry => entry.blockable) ?? [];
  const informational = catalog?.destinations.filter(entry => !entry.blockable) ?? [];
  const notRead = settings === undefined;
  const observed = (key: string) => settings?.find(setting => setting.key === key);
  const nonessential = observed(NONESSENTIAL);

  function toggle(id: string, checked: boolean) {
    onChange(checked ? [...selected.filter(value => value !== id), id] : selected.filter(value => value !== id));
  }

  async function runTest(id: string) {
    if (!onRuleTest || !activeBinding) return;
    setTestBusy(id);
    try {
      const outcome = await onRuleTest(id, activeBinding);
      setTested({ binding: activeBinding, id, outcome });
    } catch (err) {
      setTested({ binding: activeBinding, id, outcome: null, error: err instanceof Error ? err.message : String(err) });
    } finally { setTestBusy(''); }
  }
  const activeBlockedSet = new Set(activeBlocked ?? []);
  // Only a stale result (different binding) is dropped; a same-instance result
  // remains until the channel changes.
  const currentTest = tested && activeBinding === tested.binding ? tested : null;

  return <section className="telemetry-panel" aria-label="Telemetry 目标目录">
    <div className="surface-heading"><h3>Telemetry 目标目录</h3><Status value={catalog ? 'available' : busy ? 'pending' : 'unknown'}/></div>
    <div className="padded">
      <p>官方文档列出的可选 Claude Code telemetry 目标。只有“可阻止”的确切 host 能加入草案的阻止规则；混合用途域名仅作说明，不能一键阻止。</p>
      {error && <Notice tone="error">{error}</Notice>}
      {!catalog && busy && <p className="small-print">正在读取静态目录…</p>}
      {catalog && <>
        <p className="small-print"><ResourceLink resource="claude-network-reference">Claude 官方网络说明</ResourceLink> · 核对日期 {catalog.checked_date} · schema {catalog.schema}</p>
        <section className="telemetry-privacy" aria-label="已观察的隐私设置">
          <h4>已观察的隐私设置（读回值，非全局强制）</h4>
          {notRead ? <p className="small-print" role="status">not_read · 尚未读取当前设置。这不代表已关闭或已强制。</p> : <>
            <div className="fact-row"><span>非必要流量总开关 <code>{NONESSENTIAL}</code></span><strong>{(nonessential?.value ?? '未设置')} · {nonessential ? settingState(nonessential) : 'unknown'}</strong></div>
            <p className="small-print">按官方 env-vars：此变量只要非空即生效（<code>0</code>／<code>false</code> 也会关闭），并同时关闭自动更新、release notes 与 feature flag 获取。它只描述已读回的值，不等于当前运行证明。</p>
            <ul className="compact-list telemetry-info">{FINE_GRAINED.map(key => { const setting = observed(key); return <li key={key}><code>{key}</code> · {setting ? `${setting.value ?? '未设置'} · ${settingState(setting)}` : 'not_read'}</li>; })}</ul>
          </>}
          {onPrivacySettings && <button className="text-button" type="button" onClick={onPrivacySettings}>查看现有隐私设置</button>}
        </section>
        <h4>可加入阻止规则</h4>
        {blockable.map(entry => <label className="telemetry-row" key={entry.id}>
          <input type="checkbox" disabled={disabled} checked={selected.includes(entry.id)} onChange={event => toggle(entry.id, event.target.checked)}/>
          <span><strong>{entry.purpose}</strong>
            <span className="telemetry-detail"><code>{entry.host}:{entry.port}</code> · {entry.provider} · 客户端 {entry.clients.join('、')}</span>
            <span className="telemetry-detail">适用性：{entry.applicability} · 版本：{entry.version_applicability}（核对 {entry.checked_date}）</span>
            <span className="telemetry-detail telemetry-impact">影响：{entry.collateral_impact}</span>
          </span>
        </label>)}
        {blockable.length === 0 && <p className="small-print">当前目录没有可阻止的目标。</p>}
        <h4>仅信息（混合或必要用途，不提供一键阻止）</h4>
        <ul className="compact-list telemetry-info">{informational.map(entry => <li key={entry.id}>
          <code>{entry.host}:{entry.port}</code> · {entry.purpose}
          <span className="telemetry-detail">适用性：{entry.applicability}</span>
          <span className="telemetry-detail telemetry-impact">影响：{entry.collateral_impact}</span>
        </li>)}</ul>
        <Notice>选中目标在下次显式启动时合并，取消选择只撤回这次选择；草案中已有的规则由上方文本编辑。启动后在读回配置中核对。通道仅覆盖经过它的连接。</Notice>
        <section className="telemetry-test" aria-label="受控规则测试">
          <h4>受控规则测试（当前活动通道）</h4>
          {!activeBinding ? <p className="small-print" role="status">当前没有正在运行的通道；请先显式启动通道，再测试已阻止的目标。</p> : <>
            <p className="small-print">在活动通道上向本代理自身发送一次真实 CONNECT，验证确切目标被显式阻止。它不会连接目标、不会解析公网、也不代表系统级强制。</p>
            {blockable.map(entry => {
              const blockedNow = activeBlockedSet.has(entry.id);
              return <div className="telemetry-test-row" key={entry.id}>
                <code>{entry.host}:{entry.port}</code>
                {blockedNow
                  ? <button className="text-button" type="button" disabled={!!testBusy} onClick={() => void runTest(entry.id)}>{testBusy === entry.id ? '测试中…' : '测试该阻止'}</button>
                  : <span className="telemetry-detail">当前通道未显式阻止；先加入草案并重新启动通道后再测试。</span>}
              </div>;
            })}
            {currentTest && <div className="fact-row telemetry-test-result" role="status">
              <span>受控测试 {currentTest.outcome?.destination_host || currentTest.id}</span>
              <strong>{currentTest.outcome?.result === 'blocked_explicit' ? '已确认显式阻止' : '未通过'}</strong>
              {currentTest.outcome ? <span className="small-print">test_id {currentTest.outcome.test_id} · 连接尝试 {currentTest.outcome.connection_attempted ? '是' : '否'} · 与客户端事件分列。</span> : <span className="small-print">{currentTest.error}</span>}
            </div>}
            <p className="small-print">真实客户端事件与受控测试分列：受控测试带 owner-origin 标记，普通 Claude/代理流量不带。通道停止或被替换后需对新实例重新测试。</p>
          </>}
        </section>
      </>}
    </div>
  </section>;
}

// A finite, honest label for one observed setting. `status` is the readback's
// own state (`configured`/`unchanged`/`uncertain`); nothing is asserted when the
// value was not read.
function settingState(setting: Inspection['settings'][number]): string {
  return setting.status;
}

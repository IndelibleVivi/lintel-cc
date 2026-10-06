import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Api } from './types';

export const transport = isTauri() ? 'native' : import.meta.env.MODE === 'fixture' ? 'synthetic' : 'unavailable';
export type RequestDiagnostic = {
  stage: 'local' | 'ssh' | 'runner' | 'response'; reason: string; summary: string;
  next_steps: string[]; command?: string; exit_code?: number; stderr_excerpt?: string;
  stderr_truncated?: boolean; submission_uncertain: boolean;
  alias?: string; install_id?: string;
};
export class RequestError extends Error {
  constructor(public code: string, message: string, public diagnostic?: RequestDiagnostic) { super(message); }
}
export type Envelope<T> = { ok: true; data: T } | { ok: false; error: { code: string; message: string; diagnostic?: RequestDiagnostic } };
async function send<C extends keyof Api>(command: C, fields: Api[C]['request']): Promise<Api[C]['response']> {
  const payload = { command, ...fields };
  let envelope: Envelope<Api[C]['response']>;
  if (transport === 'native') {
    envelope = await invoke('request', { payload });
  } else if (transport === 'synthetic') {
    const response = await fetch('/__lintel_fixture/request', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(payload) });
    if (!response.ok) throw new RequestError('BRIDGE_UNAVAILABLE', '合成测试执行器暂不可用。请保留任务 ID，重连后查询原任务。');
    envelope = await response.json();
  } else {
    throw new RequestError('NATIVE_REQUIRED', '请在 Lintel 桌面应用中使用本机功能。浏览器页面没有获得本机执行权限。');
  }
  if (!envelope.ok) throw new RequestError(envelope.error.code, envelope.error.message, envelope.error.diagnostic);
  return envelope.data;
}

// Core holds one transaction lock for its local journal. Serialize this window
// so its own independent reads do not compete with discovery or execution.
let coreQueue: Promise<unknown> = Promise.resolve();
// Only finite operation names and host labels leave the transport closure.
// Paths, passphrases, request fields and decoded content never enter this store.
const packageLabels: Partial<Record<keyof Api, string>> = {
  archive_inspect: '核验并解锁工作包', session_read: '核验工作包并读取一页',
  plan_import: '核验工作包并冻结迁入落点', plan_resume: '核验工作包并检查续聊',
};
type QueueItem = { id: number; hostAlias: string | null; command: keyof Api; running: boolean; queue: object };
export type PackageActivity = { id: number; hostAlias: string | null; label: string; running: boolean; waiting: number };
const localQueue = {};
let nextActivityId = 0;
const queueItems = new Map<number, QueueItem>();
const activityListeners = new Set<() => void>();
let activitySnapshot: PackageActivity[] = [];
function publishActivity() {
  activitySnapshot = [...queueItems.values()].filter(item => packageLabels[item.command]).map(item => ({
    id: item.id, hostAlias: item.hostAlias, label: packageLabels[item.command]!, running: item.running,
    waiting: [...queueItems.values()].filter(other => other.queue === item.queue && other.id > item.id).length,
  }));
  activityListeners.forEach(listener => listener());
}
export function subscribePackageActivity(listener: () => void) {
  activityListeners.add(listener); return () => { activityListeners.delete(listener); };
}
export function getPackageActivity() { return activitySnapshot; }
function queued<C extends keyof Api>(queue: Promise<unknown>, owner: object, hostAlias: string | null, command: C, action: () => Promise<Api[C]['response']>) {
  const item: QueueItem = { id: ++nextActivityId, hostAlias, command, running: false, queue: owner };
  queueItems.set(item.id, item); publishActivity();
  return queue.then(async () => {
    item.running = true; publishActivity();
    try { return await action(); }
    finally { queueItems.delete(item.id); publishActivity(); }
  });
}
export function request<C extends keyof Api>(command: C, fields: Api[C]['request']): Promise<Api[C]['response']> {
  const result = queued(coreQueue, localQueue, null, command, () => send(command, fields));
  coreQueue = result.catch(() => undefined);
  return result;
}

// Remote calls share the same typed product contract and retain their host scope.
export function requester(alias: string | null): typeof request {
  if (!alias) return request;
  let remoteQueue: Promise<unknown> = Promise.resolve();
  const owner = {};
  return <C extends keyof Api>(command: C, fields: Api[C]['request']): Promise<Api[C]['response']> => {
    const result = queued(remoteQueue, owner, alias, command, async () => {
      // Remote resume asks for the package secret in its real PTY. The App's
      // ephemeral secret is used only by the separately approved read/preview.
      const remoteFields = command === 'resume_request'
        ? Object.fromEntries(Object.entries(fields).filter(([key]) => key !== 'archive_passphrase'))
        : fields;
      const payload = command === 'execute' || command === 'launch' || command === 'launch_request' || command === 'resume_request' || command === 'launch_query' || command === 'launches' ? { op:command, alias, ...remoteFields } : { op:'request', alias, request:{command,...remoteFields} };
      const envelope = await invoke<Envelope<Api[C]['response']>>('remote_request', {payload});
      if (!envelope.ok) throw new RequestError(envelope.error.code,envelope.error.message,envelope.error.diagnostic);
      return envelope.data;
    });
    remoteQueue = result.catch(() => undefined);
    return result;
  };
}

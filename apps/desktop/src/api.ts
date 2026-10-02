import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Api } from './types';

export const transport = isTauri() ? 'native' : import.meta.env.MODE === 'fixture' ? 'synthetic' : 'unavailable';
export type RequestDiagnostic = {
  stage: 'local' | 'ssh' | 'runner' | 'response'; reason: string; summary: string;
  next_steps: string[]; command?: string; exit_code?: number; stderr_excerpt?: string;
  stderr_truncated?: boolean; submission_uncertain: boolean;
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
export function request<C extends keyof Api>(command: C, fields: Api[C]['request']): Promise<Api[C]['response']> {
  const result = coreQueue.then(() => send(command, fields));
  coreQueue = result.catch(() => undefined);
  return result;
}

// Remote calls share the same typed product contract and retain their host scope.
export function requester(alias: string | null): typeof request {
  if (!alias) return request;
  let remoteQueue: Promise<unknown> = Promise.resolve();
  return <C extends keyof Api>(command: C, fields: Api[C]['request']): Promise<Api[C]['response']> => {
    const result = remoteQueue.then(async () => {
      const payload = command === 'execute' ? { op:'execute', alias, ...fields } : { op:'request', alias, request:{command,...fields} };
      const envelope = await invoke<Envelope<Api[C]['response']>>('remote_request', {payload});
      if (!envelope.ok) throw new RequestError(envelope.error.code,envelope.error.message,envelope.error.diagnostic);
      return envelope.data;
    });
    remoteQueue = result.catch(() => undefined);
    return result;
  };
}

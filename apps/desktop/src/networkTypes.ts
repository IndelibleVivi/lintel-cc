export type ProbePath = 'host_default' | 'lintel_channel';
export type ProbeFamily = 'ipv4' | 'ipv6';
export interface ProbeOptions {
  ipv4_url: string; ipv6_url: string; proxy_url?: string;
  timeout_seconds?: number; proxy_binding?: string;
}
export interface ProbeCell {
  path: ProbePath; family: ProbeFamily; status: string; public_ip: string | null;
  elapsed_ms: number; peer_family?: string | null; message?: string;
}
export interface NetworkProbe {
  schema: string; id: string; executed_at: string; platform: string; execution_host?: string | null;
  network_revision: string; endpoints: { ipv4: string; ipv6: string };
  proxy_url: string | null; proxy_binding?: string | null; timeout_seconds?: number; stale?: boolean; cells: ProbeCell[];
}
export interface NetworkService {
  service_id: string; name: string; interface: string; mode: string; enabled: boolean;
  ipv4_addresses: string[]; ipv6_addresses: string[]; ipv6_enabled?: boolean | null;
}
export interface NetworkInspection {
  schema: string; platform: string; network_revision: string; network_revision_complete?: boolean;
  interfaces?: Array<{ interface: string; flags: number; up: boolean; ipv4_addresses: string[]; ipv6_addresses: string[] }>;
  services: NetworkService[]; limitations: string[];
}
export interface NetworkPlan {
  scope: 'host_shared'; service_id: string; service_name: string; interface: string; service_enabled?: boolean;
  before: unknown; after: unknown; probe: ProbeOptions; before_probe: NetworkProbe;
}
export interface NetworkChange {
  scope: 'host_shared'; service_id: string; service_name?: string; interface?: string;
  before?: unknown; after?: unknown; configuration_verified?: boolean;
}

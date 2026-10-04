export interface Environment {
  id: string; name: string; host: string; surface: string; root: string;
  executable: string | null; ownership: string; status: string;
  product_version?: string | null; product_evidence?: ProductEvidence;
}
export interface ProductEvidence { version: string | null; source: 'native_version_path' | 'npm_package' | 'unknown'; executable: string | null }
export type TrustedDevices = 'unknown' | 'required' | 'not_required';
export type PolicyPreset = 'preserve' | 'reduce' | 'custom';
export type PolicyChoice = 'keep' | 'disable' | 'remove';
export type CustomSettings = Record<string, PolicyChoice>;
export interface PolicyAssessment {
  rule_version: string; product: ProductEvidence;
  supported_presets?: PolicyPreset[]; preset?: PolicyPreset; custom_settings?: CustomSettings;
  rules: { key: string; label: string; value: string | null; disabled: boolean | null; semantics: 'nonempty' | 'boolean'; scope: string; effect_timing: string }[];
  remote_control: { status: 'blocked' | 'conditional' | 'configuration_compatible'; summary: string; trusted_devices: TrustedDevices; trusted_devices_source: string; version_family: string; runtime_verified: false; blockers: { key: string; value: string | null; status: string; reason: string; source: string }[]; unverified: string[] };
  keep_remote_control?: boolean; release_settings?: string[];
}
export interface Capability { name: string; status: string; reason: string }
export interface Setting { key: string; label: string; value: string | null; source: string; effect_timing: string; status: string }
export interface Inspection { environment: Environment; settings: Setting[]; assets: { category: string; count: number; bytes: number; complete?: boolean }[]; warnings: string[]; policy?: PolicyAssessment }
export interface Plan {
  id: string; hash: string; environment_id: string; title: string;
  changes: { key: string; label: string; before: string | null; after: string | null; path: string }[];
  preserves: string[]; warnings: string[]; actions: { id: string; label: string; reversible: boolean }[];
  created_at: string; status: string; archive_passphrase_required?: boolean; file_count?: number;
  policy?: PolicyAssessment;
  service?: ServicePlan;
}
export interface Receipt {
  id: string; plan_id: string; environment_id: string; title: string; status: string;
  steps: { id: string; label: string; status: string; message: string }[];
  created_at: string; restorable: boolean; warnings: string[];
  policy?: PolicyAssessment;
  service?: ServicePlan & { observed?: { active_state: string; main_pid: number; quiesced: boolean } };
  service_restorable?: boolean;
  execution?: { mode: 'setsid' | 'system_manager' | 'user_manager'; manager: 'system' | 'user' | null; unit: string | null; continuation: string; limitation: string | null; reboot_survival: false };
  local_cleanup?: string; remote_revocation?: string; state_archive_path?: string; new_environment_id?: string; new_root?: string; archive_path?: string;
}
export interface CleanupInspection { environment_id: string; files: {path: string; category: string; present: boolean}[]; writers: {pid: string; name: string; scope: string}[]; shared_profile_present: boolean; official_logout_available: boolean; coverage: string }
export interface ArchiveManifest { job_id: string; created_at: string; files: {path: string; category: string; bytes: number; digest: string}[]; notes: string }
export interface Drift { changes: Setting[]; status: string }
export type ServiceManager = 'user' | 'system';
export interface ServicePlan {
  manager: ServiceManager; unit: string; root: string;
  before: { active_state: string; sub_state: string; unit_file_state: string; restart: string };
  after: { active_state: string; hold: boolean };
  hold: { path: string; persistent: true }; original_job: string | null;
}
export interface ServiceInspection {
  environment_id: string; manager: ServiceManager; unit: string; root: string;
  active_state: string; sub_state: string; unit_file_state: string; restart: string;
  main_pid: number; control_group: string; triggered_by: string[];
  bound: boolean; quiesced: boolean; quiesce_job_id: string | null;
  hold: { path: string; persistent: true } | null; limitations: string[];
}
export interface Api {
  discover: { request: {}; response: { environments: Environment[]; capabilities: Capability[] } };
  register: { request: { name: string; root: string }; response: Environment };
  create_environment: { request: { name: string }; response: Environment };
  inspect: { request: { environment_id: string; trusted_devices?: TrustedDevices }; response: Inspection };
  plan_policy: { request: { environment_id: string; preset: PolicyPreset; keep_remote_control: boolean; trusted_devices?: TrustedDevices; release_settings?: string[]; custom_settings?: CustomSettings }; response: Plan };
  plan_reset: { request: { environment_id: string; recipe: 'rebuild'; categories: string[] }; response: Plan };
  plan_restore: { request: { job_id: string }; response: Plan };
  execute: { request: { plan_id: string; approval: string; archive_passphrase?: string }; response: Receipt };
  cleanup_inspect: { request: { environment_id: string }; response: CleanupInspection };
  service_inspect: { request: { environment_id: string; manager: ServiceManager; unit: string }; response: ServiceInspection };
  plan_service_quiesce: { request: { environment_id: string; manager: ServiceManager; unit: string }; response: Plan };
  plan_service_resume: { request: { job_id: string }; response: Plan };
  auth_probe: { request: { environment_id: string }; response: { auth_method: string; logged_in: boolean; remote_revocation: string } };
  plan_cleanup: { request: { environment_id: string; recipe: 'repair_login' | 'reset_client' | 'retire'; writers_confirmed_stopped: boolean; official_logout: boolean; categories: string[] }; response: Plan };
  reactivate_environment: { request: { environment_id: string }; response: { status: string } };
  archive_inspect: { request: { job_id: string; archive_passphrase: string }; response: ArchiveManifest };
  archive_read: { request: { job_id: string; archive_passphrase: string; path: string }; response: { path: string; text: string; bytes: number; truncated: boolean } };
  plan_import: { request: { environment_id: string; job_id: string; categories: string[]; archive_passphrase: string }; response: Plan };
  jobs: { request: {}; response: { jobs: Receipt[] } };
  job: { request: { job_id: string }; response: Receipt };
  drift: { request: { environment_id: string }; response: Drift };
  accept_drift: { request: { environment_id: string }; response: unknown };
  launch: { request: { environment_id: string }; response: { status: string; message: string } };
  export_support: { request: {}; response: unknown };
}
export type Draft = { preset: PolicyPreset; keepRemoteControl: boolean; trustedDevices?: TrustedDevices; releaseSettings?: string[]; customSettings?: CustomSettings };

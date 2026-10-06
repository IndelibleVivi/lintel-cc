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
export interface ComponentInspection {
  environment_id:string;root:string;project_cwd:string|null;checked_at:string;note:string;
  items:{id:string;title:string;state:string;source:string;scope:string;detail:string;facts:{
    selected_executable?:string|null;
    sources?:{scope:string;path:string;state:string;content_state?:string;hooks_declared?:boolean;mcp_declared?:boolean}[];
    credential_file?:{state:string};shared_profile?:{state:string};
    services?:{manager:string;unit:string;original_job_id:string;recorded_status:string;state:string;current?:{active_state:string;quiesced:boolean}}[];
    services_truncated?:boolean;
  };next_action:string}[];
  records:{id:string;title:string;status:string;created_at:string;coverage?:{scope:string;state:string;detail:string}[]|null}[];
  records_complete:boolean;records_truncated:boolean;
}
export interface WorkPreflight {
  environment_id:string;root:string;checked_at:string;complete:boolean;eligible:boolean;
  totals:{files:number;bytes:number};limits:{file_bytes:number;total_bytes:number;files:number;entries:number};
  categories:{category:string;count:number;bytes:number}[];
  blockers:{code:string;path?:string|null;message:string}[];blockers_truncated:boolean;
}
export interface Plan {
  id: string; hash: string; environment_id: string; title: string;
  changes: { key: string; label: string; before: string | null; after: string | null; path: string }[];
  preserves: string[]; warnings: string[]; actions: { id: string; label: string; reversible: boolean }[];
  created_at: string; status: string; archive_passphrase_required?: boolean; file_count?: number;
  policy?: PolicyAssessment;
  service?: ServicePlan;
  plan_revision?: string;
  planned_target?: { new_environment_id: string|null; new_root: string; create: boolean; files: {source:string;destination:string;category:string;size:number;sha256:string}[]; purposes: Record<string,string>; activation: Record<string,boolean> };
  launch_request?: { id:string;environment_id:string;project_cwd:string;config_root:string;executable:string;client_version:string|null;mode:string;input_reference?:unknown;created_at:string };
  resume?: { supported:boolean;reason:string|null;mode:string;source:unknown;transcript_path:string;private_copy_path:string;config_root:string;project_cwd:string;client_version:string|null;client_support:unknown;archive_unmodified:boolean;auth_unverified:boolean;attach_risk:string;write_scope?:{config_root:string;project_cwd?:string;note:string} };
  import_manifest?: {
    package: { format: string; generator: string | null; sha256: string };
    files: { source: string; destination: string; category: string; size: number; sha256: string }[];
  };
}
export interface LaunchRecord {
  request_id:string; status:string; mode?:string; environment_id?:string; root?:string; config_root?:string; project_cwd?:string; executable?:string; recorded_at?:string; created_at?:string; message?:string; observed?:string; private_copy_path?:string; alias?:string; runner_digest?:string|null; binding_resolution?:string; error?:{code:string;message:string};
}
export interface Receipt {
  id: string; plan_id: string; environment_id: string; title: string; status: string;
  steps: { id: string; label: string; status: string; message: string }[];
  created_at: string; restorable: boolean; warnings: string[];
  policy?: PolicyAssessment;
  service?: ServicePlan & { observed?: { active_state: string; main_pid: number; quiesced: boolean } };
  service_restorable?: boolean;
  task_result?: { outcome:string; title?:string; primary?:string; selected_steps:{id:string;done:boolean;note:string}[]; coverage:{scope:string;state:string;detail:string}[];next_actions:{label:string;entry:string}[] };
  settings_recovery?: { state: "written" | "not_written" | "ownership_unproven"; reason: string; message: string };
  execution?: { mode: 'setsid' | 'system_manager' | 'user_manager'; manager: 'system' | 'user' | null; unit: string | null; continuation: string; limitation: string | null; reboot_survival: false };
  migration_probe?: { path: string; status: 'executing' | 'removed' | 'retained' };
  error?: { code: string; message: string; phase: string; step_id?: string | null; recovery?: string; next_action?: string; uncertain_side_effects?: boolean };
  coverage?: Record<string, unknown>; next_steps?: string[]; task_outcome?: string; outcome?: string;
  local_cleanup?: string; remote_revocation?: string; state_archive_path?: string; new_environment_id?: string; new_root?: string; archive_path?: string; archive_digest?: string;
}
export interface CleanupInspection { environment_id: string; files: {path: string; category: string; present: boolean}[]; writers: {pid: string; name: string; scope: string}[]; shared_profile_present: boolean; official_logout_available: boolean; coverage: string }
export interface ArchiveManifest { job_id?: string | null; archive_path?: string; created_at: string; files: {path: string; category: string; bytes: number; digest: string}[]; notes: string }
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
export interface SessionRecord { index:number; kind:string; text?:unknown;tool?:unknown;record?:unknown;block?:unknown;raw?:string;timestamp?:string;name?:string;opaque?:boolean;unknown?:boolean }
export interface SessionPage { path:string;content_kind:string;records:SessionRecord[];raw_text?:string;next_offset:number|null;done:boolean;total_bytes:number;digest:string;source:{archive_path:string;job_id?:string;package_digest:string} }
export interface ExecutionContext { product:string;version:string;protocol:number;platform:string;architecture:string;user:{uid:number;euid:number;home:string};state:{source:string;path:string;exists:boolean};config_home:{path:string;exists:boolean};executable:string|null;initialized:boolean }
export interface Api {
  context: {request:{};response:ExecutionContext};
  session_read: { request:{job_id?:string;archive_path?:string;archive_passphrase:string;path:string;offset?:number;expected_digest?:string};response:SessionPage };
  plan_launch: { request:{environment_id:string;project_cwd:string;mode:'interactive';input_reference?:unknown;proxy_url?:string};response:Plan };
  plan_resume: { request:{environment_id:string;project_cwd:string;job_id?:string;archive_path?:string;archive_passphrase:string;path:string};response:Plan };
  launch_query: { request:{request_id:string};response:LaunchRecord };
  launches: { request:{};response:{launches:LaunchRecord[]} };
  launch_request: { request:{request_id:string;approval:string};response:{status:string;request_id:string;project_cwd:string;root:string;executable:string;message:string} };
  resume_request: { request:{request_id:string;approval:string;archive_passphrase:string};response:{status:string;request_id:string;project_cwd:string;root:string;executable:string;message:string} };
  discover: { request: {}; response: { environments: Environment[]; capabilities: Capability[] } };
  register: { request: { name: string; root: string }; response: Environment };
  create_environment: { request: { name: string }; response: Environment };
  inspect: { request: { environment_id: string; trusted_devices?: TrustedDevices }; response: Inspection };
  inspect_components:{request:{environment_id:string;project_cwd?:string};response:ComponentInspection};
  work_preflight:{request:{environment_id:string;categories:string[]};response:WorkPreflight};
  plan_policy: { request: { environment_id: string; preset: PolicyPreset; keep_remote_control: boolean; trusted_devices?: TrustedDevices; release_settings?: string[]; custom_settings?: CustomSettings }; response: Plan };
  plan_archive: { request: { environment_id: string; categories: string[]; output_path?: string }; response: Plan };
  plan_preserve: { request: { environment_id: string; categories: string[]; activate?:{instructions:boolean}; name?: string }; response: Plan };
  plan_show: { request: { plan_id: string }; response: Plan };
  plan_reset: { request: { environment_id: string; recipe: 'rebuild'; categories: string[]; activate?:{instructions:boolean} }; response: Plan };
  plan_restore: { request: { job_id: string }; response: Plan };
  execute: { request: { plan_id: string; approval: string; archive_passphrase?: string }; response: Receipt };
  cleanup_inspect: { request: { environment_id: string }; response: CleanupInspection };
  service_inspect: { request: { environment_id: string; manager: ServiceManager; unit: string }; response: ServiceInspection };
  plan_service_quiesce: { request: { environment_id: string; manager: ServiceManager; unit: string }; response: Plan };
  plan_service_resume: { request: { job_id: string }; response: Plan };
  auth_probe: { request: { environment_id: string }; response: { auth_method: string; logged_in: boolean; identity_observed?: boolean; remote_revocation: string } };
  plan_cleanup: { request: { environment_id: string; recipe: 'repair_login' | 'reset_client' | 'retire'; writers_confirmed_stopped: boolean; official_logout: boolean; categories: string[]; activate?:{instructions:boolean} }; response: Plan };
  reactivate_environment: { request: { environment_id: string }; response: { status: string } };
  archive_inspect: { request: { job_id?: string; archive_path?: string; archive_passphrase: string }; response: ArchiveManifest };
  archive_read: { request: { job_id?: string; archive_path?: string; archive_passphrase: string; path: string }; response: { path: string; text: string; bytes: number; truncated: boolean } };
  plan_import: { request: { environment_id: string; job_id?: string; archive_path?: string; categories: string[]; activate?:{instructions:boolean}; archive_passphrase: string }; response: Plan };
  jobs: { request: {}; response: { jobs: Receipt[] } };
  job: { request: { job_id: string }; response: Receipt };
  drift: { request: { environment_id: string }; response: Drift };
  accept_drift: { request: { environment_id: string }; response: unknown };
  launch: { request: { environment_id: string }; response: { status: string; message: string } };
  export_support: { request: {}; response: unknown };
}
export type Draft = { preset: PolicyPreset; keepRemoteControl: boolean; trustedDevices?: TrustedDevices; releaseSettings?: string[]; customSettings?: CustomSettings };

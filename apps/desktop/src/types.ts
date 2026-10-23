export type JobSummary = {id:string;title:string;title_truncated:boolean;created_at:string;updated_at_ns:string;environment_id:string|null;status:string;host_shared_network:boolean;has_work_archive:boolean;restorable:boolean;service_restorable:boolean};
export type JobsPage = {jobs:JobSummary[];record_errors:{job_id:string|null;code:string}[];records_complete:boolean;digest:string;next_offset:number|null;scan_complete:boolean;scanned:number;total_candidates:number;order:'updated_desc_id'};
export type JobDetailPage = {job_id:string;section:string;items:unknown[];digest:string;next_offset:number|null;total_items:number;complete:boolean};
import type { NetworkChange, NetworkInspection, NetworkPlan, NetworkProbe, ProbeOptions } from './networkTypes';
export interface Environment {
  id: string; name: string; host: string; surface: string; root: string;
  executable: string | null; ownership: string; status: string;
  product_version?: string | null; product_evidence?: ProductEvidence;
}
export interface ProductEvidence { version: string | null; source: 'native_version_path' | 'npm_package' | 'unknown'; executable: string | null }
export type TrustedDevices = 'unknown' | 'required' | 'not_required';
export type PolicyPreset = 'preserve' | 'reduce' | 'custom';
export type PolicyChoice = 'keep' | 'disable' | 'enable' | 'remove' | 'set';
export type CustomSettings = Record<string, PolicyChoice>;
// A `set` choice needs its value; masked keys never accept one.
export type CustomValues = Record<string, string>;
export interface PolicyRule {
  key: string; label: string; value: string | null; disabled: boolean | null;
  semantics: 'nonempty' | 'boolean' | 'default_on' | 'value' | 'raw_api_bodies';
  scope: string; effect_timing: string;
  category?: string; sources?: { turn_on: string[]; turn_off: string[] }; default_state?: string;
  impact?: string; version_floor?: string | null; signal_enabled?:boolean|null; masked?: boolean; value_masked?: boolean;
  depends_on?: string[]; local_dump?: string; local_dump_note?: string;
}
export interface PolicyAssessment {
  rule_version: string; product: ProductEvidence;
  supported_presets?: PolicyPreset[]; preset?: PolicyPreset; custom_settings?: CustomSettings;
  custom_values?: CustomValues;
  rules: PolicyRule[];
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
export interface WorkFile {path:string;category:string;bytes:number}
export interface StartupSource {
  path:string;scope:string;state:string;content_state?:string;
  hooks_declared?:boolean;mcp_declared?:boolean;api_key_helper_declared?:boolean;
  policy_helper_declared?:boolean;plugins_declared?:boolean;auth_selectors_declared?:string[];
}
export interface StartupInspection {
  schema:'lintel.startup/1';config_root:string;project_cwd:string|null;
  sources:StartupSource[];candidate_scan_complete:boolean;content_limited:boolean;
  actual_loaded:'unverified';note:string;unknown_sources:string[];next_steps:string[];
  authentication:{state:'unverified';observation:'metadata_only';runner_environment:{name:string;present:boolean}[];terminal_environment:'unverified';keychain:'not_checked';account:'not_checked'};
}
export interface WorkInventory {
  environment_id:string;root:string;complete:boolean;digest:string;
  files:WorkFile[];total_files:number;next_offset:number|null;
  reason?:string|null;unobserved?:string|null;
}
export interface Plan {
  network?: NetworkPlan;
  lightweight_changes?:{status:string;changes:unknown[]};
  id: string; hash: string; environment_id: string | null; title: string;
  changes: { key: string; label: string; before: string | null; after: string | null; path: string;masked?:boolean;before_present?:boolean;after_present?:boolean;operation?:string }[];
  configuration?:{path:string;scope:string;field_count:number};
  residue_selection?:{files:{path:string;bytes:number}[];archive_before_clear:boolean};
  identity_reset?:{path:string;remove_keys:string[];retained_keys:string[];backup_paths:string[];logout:false};
  residue_restore?:{files:{path:string;destination:string;bytes:number;role?:string|null}[]};
  workflow?:Plan[];
  preserves: string[]; warnings: string[]; actions: { id: string; label: string; reversible: boolean }[];
  created_at: string; status: string; archive_passphrase_required?: boolean; file_count?: number;
  work_selection?:{mode:'paths'|'categories';paths?:string[]};
  policy?: PolicyAssessment;
  service?: ServicePlan;
  plan_revision?: string;
  planned_target?: { new_environment_id: string|null; new_root: string; create: boolean; files: {source:string;destination:string;category:string;size:number;sha256:string}[]; purposes: Record<string,string>; activation: Record<string,boolean> };
  launch_request?: { id:string;environment_id:string;project_cwd:string;config_root:string;executable:string;client_version:string|null;mode:string;input_reference?:InputReference|null;created_at:string;startup?:StartupInspection };
  resume?: { supported:boolean;reason:string|null;mode:string;source:unknown;transcript_path:string;private_copy_path:string;config_root:string;project_cwd:string;executable?:string;client_version:string|null;client_support:unknown;archive_unmodified:boolean;auth_unverified:boolean;attach_risk:string;startup?:StartupInspection;write_scope?:{config_root:string;project_cwd?:string;note:string} };
  import_manifest?: {
    package: { format: string; generator: string | null; sha256: string };
    files: { source: string; destination: string; category: string; size: number; sha256: string }[];
  };
}
export interface LaunchRecord {
  request_id:string; status:string; mode?:string; environment_id?:string; root?:string; config_root?:string; project_cwd?:string; executable?:string; recorded_at?:string; created_at?:string; message?:string; observed?:string; private_copy_path?:string; alias?:string; runner_digest?:string|null; binding_resolution?:string; error?:{code:string;message:string};
}
export interface Receipt {
  network_change?: NetworkChange;
  before_probe?: NetworkProbe;
  after_probe?: NetworkProbe;
  id: string; plan_id: string; environment_id: string | null; title: string; status: string;
  config_archive_path?:string;identity_archive_path?:string;residue_archive_path?:string;
  identity_reset?:{removal_verified:boolean;regenerated:string};
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
// Bounded read-only local-data / identity observation (core `residue.rs`).
// Every location comes from a finite known table; `class` decides the category
// grouping in the App and, with `state`, what an action may do. Nothing outside
// the table is scanned, so an unclassified file is one the user located
// explicitly, never an inferred one.
export interface ResidueEntry { path: string; bytes: number; mtime: number; work_category?: string | null }
export interface ResidueLocation {
  path: string; category: string; class: 'work' | 'residue' | 'identity' | 'config' | string;
  state: 'present' | 'not_found' | 'access_limited' | string;
  file_count: number; total_bytes: number; newest_mtime: number | null;
  entries: ResidueEntry[]; truncated: boolean; skipped_special: number; skipped_unsafe: number;
}
export interface ResidueInventory {
  schema: 'lintel.residue/1' | string; root: string; checked_at: string;
  locations: ResidueLocation[]; totals: { file_count: number; total_bytes: number };
  scan_complete: boolean; note: string;digest?:string;next_offset?:number|null;matched_rows?:number;coverage?:{additional_sources:{extra_roots:string[]}};
}
// Masked identity observation (core `residue.rs`). Presence/persistence flags
// only until the user explicitly reveals; revealed values stay in memory.
export interface IdentityField { key: string; present: boolean; persistence: string; storage: string; masked: true }
export interface IdentityInspection {
  schema: 'lintel.identity/1' | string; path: string;
  state: 'observed' | 'not_found' | 'too_large' | 'unreadable' | 'unknown' | string;
  fields: IdentityField[]; note: string;
  // Present only after an explicit reveal: raw values for this in-memory view.
  revealed?: { key: string; value: string }[] | null;
  reveal_available?: boolean;
}
// Sanitized semantic snapshots (core `snapshot.rs`). A snapshot carries switch
// values, finite declaration flags, a caller-salted identity digest and residue
// counts; no secret body or raw identity is ever included.
export interface SnapshotSummary { snapshot_id: string; created_at: string; rule_version: string; product_version?: string | null; coverage?: Record<string, unknown>;baseline?:{kind:string;not_a_fresh_capture?:boolean}|null }
export interface Snapshot extends SnapshotSummary {
  schema: 'lintel.env-snapshot/1' | string; environment_id: string;
  settings: Record<string, { value: string | null; status: string; masked?: boolean }>;
  declarations: { scope?: string; path?: string; state?: string; hooks?: boolean | null; mcp?: boolean | null; plugins?: boolean | null; api_key_helper?: boolean | null }[];
  identity: { userID: { digest: string | null; state: string } };
  residue: Record<string, unknown>;
}
// One aligned item from same-environment (`compare`) or cross-environment
// (`compare_pair`) comparison. Verdicts only; never a digest.
export interface SnapshotChange {
  area: 'settings' | 'declarations' | 'identity' | 'residue' | string;
  state: string; id?:string;key?: string; label?: string; category?: string; subject?: string; scope?: string; path?: string;
  old?: unknown; new?: unknown; masked?: boolean; flags?: { flag: string; from: unknown; to: unknown }[];
  old_count?: number; new_count?: number; count_delta?: number; old_bytes?: number; new_bytes?: number; bytes_delta?: number;
  reinterpreted_under_new_rules?: boolean;
}
export interface SnapshotComparison {
  status: 'changed' | 'rules_updated' | 'incomplete' | 'unchanged' | string;
  rule_change: { state: string; old: string; new: string } | null;
  changes: SnapshotChange[]; summary: string;
}
// `snapshot_save` returns the summary plus the public identity projection;
// `snapshot_list` paginates read-only and reports its own coverage/errors.
export interface SnapshotSaveResult { snapshot: SnapshotSummary; identity: IdentityInspection }
export interface SnapshotListResult { snapshots: SnapshotSummary[]; total: number; listed: number; scan_complete: boolean; errors: string[];accepted_snapshot_id?:string|null }
// `compare_pair` output as published under `comparison`, with the two frozen
// side summaries. A masked setting publishes `same: null` (unknown), never a
// green "same"; partial coverage stays visible via each side's `captured_at`.
export interface EnvComparisonSide { environment_id: string; name: string; captured_at: string }
export interface EnvComparison {
  a: EnvComparisonSide; b: EnvComparisonSide;
  comparison: {
    identity: { userID: 'same' | 'different' | 'unknown' | string };
    settings: Record<string, { a: string | null; b: string | null; same: boolean | null; masked?: boolean }>;
    declarations: { only_a: { scope: string; path: string }[]; only_b: { scope: string; path: string }[]; both: { scope: string; path: string }[] };
    residue: { a: unknown; b: unknown };
    note: string;
  };
}
// Advanced config editing on one explicitly selected user-writable file.
export interface ConfigFile { path: string; scope: string; state: string; writable: boolean; bytes?: number | null; digest?: string | null }
export interface ConfigInspection { environment_id: string; root: string; project_cwd?: string | null; files: ConfigFile[]; settings?: { key: string; value: string | null; source: string;masked?:boolean;impact?:string }[];conflicts?:{key:string;state:string;sources:string[]}[]; policy?: PolicyAssessment | null; note?: string }
export interface ConfigDocument { path: string; document: string; digest: string }
// Read-only projection of one original config task's recoverable fields. The App
// uses it to offer a real multi-select before plan_config_restore; it never
// carries a secret body.
export interface ConfigRestoreInspection {
  environment_id?: string; job_id?: string; path?: string | null;
  fields: { key: string; label?: string; before: string | null; after: string | null; masked?: boolean; available: boolean; reason?: string | null }[];
  note?: string;
}
export interface ArchiveManifest { job_id?: string | null; archive_path?: string; created_at: string; files: {path: string; category: string; bytes: number; digest: string}[]; notes: string }
export interface Drift { changes: Setting[]; status: string;scope?:string }
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
// index is page-local; offset is the returned record/fragment byte start and
// block_index is the original zero-based content block (including opaque blocks).
export interface SessionRecord { index:number; offset?:number; block_index?:number; kind:string; text?:unknown;tool?:unknown;record?:unknown;block?:unknown;raw?:string;timestamp?:string;name?:string;opaque?:boolean;unknown?:boolean }
interface SessionPageBase { path:string;offset?:number;raw_text?:string;next_offset:number|null;done:boolean;total_bytes:number;digest:string;source:{archive_path:string;job_id?:string|null;package_digest:string} }
// Text pages have no records. Older runners may omit records even on a
// messages page; the reader exposes that limitation rather than assuming it.
export type SessionPage = SessionPageBase & ({content_kind:'messages';records?:SessionRecord[]} | {content_kind:'text';records?:never});
export interface InputReference { files:{path:string;digest:string;package_digest?:string;index?:number;offset?:number;block_index?:number}[] }
export interface ExecutionContext { product:string;version:string;protocol:number;platform:string;architecture:string;user:{uid:number;euid:number;home:string};state:{source:string;path:string;exists:boolean};config_home:{path:string;exists:boolean};executable:string|null;initialized:boolean }
// Static telemetry-destination catalog shared by App/CLI (contracts/telemetry-destinations.json).
export interface TelemetryDestination {
  id: string; purpose: string; provider: string; clients: string[]; applicability: string;
  version_applicability: string; source_url: string; checked_date: string;
  collateral_impact: string; host: string; port: number; blockable: boolean;
}
export interface TelemetryCatalog {
  schema: string; version: number; notes?: string; source: string; checked_date: string;
  destinations: TelemetryDestination[];
}
export interface Api {
  telemetry_catalog: { request: {}; response: TelemetryCatalog };
  network_inspect: { request: {}; response: NetworkInspection };
  network_probe: { request: ProbeOptions; response: NetworkProbe };
  plan_network_ipv6: { request: { service_id: string; mode: 'off' | 'link_local'; probe: ProbeOptions; baseline_id?: string }; response: Plan };
  plan_network_restore: { request: { job_id: string; baseline_id?: string; probe?: ProbeOptions }; response: Plan };
  context: {request:{};response:ExecutionContext};
  session_read: { request:{job_id?:string;archive_path?:string;archive_passphrase:string;path:string;offset?:number;expected_digest?:string};response:SessionPage };
  plan_launch: { request:{environment_id:string;project_cwd:string;mode:'interactive';input_reference?:InputReference;proxy_url?:string};response:Plan };
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
  work_inventory:{request:{environment_id:string;categories:string[];offset?:number;expected_digest?:string};response:WorkInventory};
  work_preflight:{request:{environment_id:string;categories:string[];selected_paths?:string[]};response:WorkPreflight};
  plan_policy: { request: { environment_id: string; preset: PolicyPreset; keep_remote_control: boolean; trusted_devices?: TrustedDevices; release_settings?: string[]; custom_settings?: CustomSettings }; response: Plan };
  plan_archive: { request: { environment_id: string; categories: string[]; selected_paths?:string[]; output_path?: string }; response: Plan };
  plan_preserve: { request: { environment_id: string; categories: string[]; selected_paths?:string[]; activate?:{instructions:boolean}; name?: string }; response: Plan };
  plan_show: { request: { plan_id: string }; response: Plan };
  plan_reset: { request: { environment_id: string; recipe: 'rebuild'; categories: string[]; activate?:{instructions:boolean} }; response: Plan };
  plan_restore: { request: { job_id: string }; response: Plan };
  execute: { request: { plan_id: string; approval: string; archive_passphrase?: string }; response: Receipt };
  cleanup_inspect: { request: { environment_id: string }; response: CleanupInspection };
  service_inspect: { request: { environment_id: string; manager: ServiceManager; unit: string }; response: ServiceInspection };
  plan_service_quiesce: { request: { environment_id: string; manager: ServiceManager; unit: string }; response: Plan };
  plan_service_resume: { request: { job_id: string }; response: Plan };
  auth_probe: { request: { environment_id: string }; response: { auth_method: string; logged_in: boolean; identity_observed?: boolean; remote_revocation: string } };
  plan_cleanup: { request: { environment_id: string; recipe: 'repair_login' | 'reset_client' | 'retire' | 'clear_residue'; writers_confirmed_stopped: boolean; official_logout: boolean; categories: string[]; residue_paths?: string[]; archive_before_clear?: boolean;archive_only?:boolean;extra_roots?:string[];delete_work?:boolean; activate?:{instructions:boolean} }; response: Plan };
  reactivate_environment: { request: { environment_id: string }; response: { status: string } };
  archive_inspect: { request: { job_id?: string; archive_path?: string; archive_passphrase: string }; response: ArchiveManifest };
  archive_read: { request: { job_id?: string; archive_path?: string; archive_passphrase: string; path: string }; response: { path: string; text: string; bytes: number; truncated: boolean } };
  plan_import: { request: { environment_id: string; job_id?: string; archive_path?: string; categories: string[]; activate?:{instructions:boolean}; archive_passphrase: string }; response: Plan };
  jobs_page: {request:{offset?:number;limit?:number;expected_digest?:string;filter?:'all'|'attention'|'archives'|'network';environment_id?:string;query?:string};response:JobsPage};
  job_detail_page: {request:{job_id:string;section?:'header'|'receipt'|'steps'|'warnings'|'coverage'|'task_result'|'result_coverage'|'next_actions'|'next_steps'|'error';offset?:number;limit?:number;expected_digest?:string};response:JobDetailPage};
  jobs: { request: {}; response: { jobs: Receipt[]; records_complete?: boolean; record_errors?: {job_id:string|null;code:string}[] } };
  job: { request: { job_id: string }; response: Receipt };
  drift: { request: { environment_id: string;lightweight?:boolean }; response: Drift };
  accept_drift: { request: { environment_id: string;lightweight?:boolean }; response: unknown };
  launch: { request: { environment_id: string }; response: { status: string; message: string } };
  residue_inventory: { request: { environment_id: string;extra_roots?:string[];limit?:number;offset?:number;expected_digest?:string;class?:string;project?:string;modified_after?:number;min_bytes?:number;max_bytes?:number }; response: ResidueInventory };
  residue_archive_inspect:{request:{job_id?:string;archive_path?:string;archive_passphrase:string};response:{files:{path:string;bytes:number;role:string|null;restore_available?:boolean}[]}};
  plan_residue_restore:{request:{environment_id:string;job_id?:string;archive_path?:string;archive_passphrase:string;selected_paths:string[]};response:Plan};
  identity_inspect: { request: { environment_id: string; reveal?: true }; response: IdentityInspection };
  snapshot_save: { request: { environment_id: string;project_cwd?:string }; response: SnapshotSaveResult };
  snapshot_list: { request: { environment_id: string }; response: SnapshotListResult };
  snapshot_accept:{request:{environment_id:string;old_snapshot_id:string;new_snapshot_id:string;selected_changes:string[]};response:{status:string;baseline:SnapshotSummary}};
  snapshot_compare: { request: { environment_id: string; old_snapshot_id: string; new_snapshot_id: string }; response: SnapshotComparison };
  env_compare: { request: { environment_id: string; other_environment_id: string;project_cwd?:string;other_project_cwd?:string }; response: EnvComparison };
  plan_identity_reset: { request: { environment_id: string; writers_confirmed_stopped: true; identity_backup_paths?: string[] }; response: Plan };
  // One synthesized plan for a combined client-control journey (reduce content
  // recording + optional identity reset + residue clear). Core builds a single
  // plan and approval; the App never loops several approved plans.
  plan_client_workflow: { request: { environment_id: string; policy?: { preset: PolicyPreset; keep_remote_control: boolean; custom_settings: CustomSettings; custom_values?: CustomValues }; residue_paths?: string[]; archive_before_clear?: boolean;extra_roots?:string[];delete_work?:boolean; writers_confirmed_stopped: true; reset_identity?: boolean; identity_backup_paths?: string[] }; response: Plan };
  config_inspect: { request: { environment_id: string; project_cwd?: string }; response: ConfigInspection };
  config_read: { request: { environment_id: string; config_path: string }; response: ConfigDocument };
  plan_config_edit: { request: { environment_id: string; config_path: string; document: string; expected_digest: string }; response: Plan };
  plan_config_apply: { request: { environment_id: string; source_environment_id: string; selected_keys: string[]; config_path?: string }; response: Plan };
  config_restore_inspect: { request: { environment_id: string; job_id: string }; response: ConfigRestoreInspection };
  plan_config_restore: { request: { environment_id: string; job_id: string; selected_keys: string[] }; response: Plan };
  export_support: { request: {}; response: unknown };
}
export type Draft = { preset: PolicyPreset; keepRemoteControl: boolean; trustedDevices?: TrustedDevices; releaseSettings?: string[]; customSettings?: CustomSettings; customValues?: CustomValues };

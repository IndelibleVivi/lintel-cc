# Lintel JSON protocol 1

All requests: JSON object with `command`. All responses: `{ "ok": true, "data": ... }` or `{ "ok": false, "error": { "code": "...", "message": "..." } }`. Rust shared public entry: `lintel_core::handle_request(request: serde_json::Value) -> serde_json::Value`. State uses LINTEL_STATE_DIR override, otherwise an OS user app-data directory. Synthetic tests use LINTEL_TEST_HOME only in CLI/development, never embed user fixtures.

Commands and data:
- `discover` → `{environments: Environment[], capabilities: {name, status, reason}[]}`. May register discovered default root in owned inventory; never execute target software during scan.
- `register` + `name`, `root` → Environment. `create_environment` + `name` → Environment with fresh owned root.
- `inspect` + `environment_id` → `{environment, settings: Setting[], assets: {category,count,bytes}[], warnings: string[]}`. Never return secrets or whole JSON settings.
- `plan_policy` + `environment_id`, `preset` (`preserve`|`reduce`), `keep_remote_control` boolean → Plan.
- `plan_reset` + `environment_id`, `recipe` (`rebuild`), `categories` (selected work classes) → Plan. Whole reset must not claim credential logout if unsupported; untouched old state clearly reported. The current rebuild leaves the original root intact; real credential reset/writer quiescence remain unimplemented.
- `plan_restore` + `job_id` → Plan with field-level conflict checks.
- `execute` + `plan_id`, `approval` (=plan.hash), optional `archive_passphrase` → Receipt. Rebuild requires a passphrase of at least 12 characters; never persist or log it. Persist plan/journal before side effect; repeated ID returns original receipt, never repeats.
- `jobs` → `{jobs: Receipt[]}`; `job` + `job_id` → Receipt.
- `drift` + `environment_id` → `{changes: Setting[], status}`; `accept_drift` + environment_id records current owned settings as baseline.
- `launch_context` + environment_id → `{root,executable}` for the interactive CLI.
- `launch` + environment_id, optional `proxy_url` (loopback HTTP address only) → `{status,message}` using exact executable/root; never shell-interpolate user text. UI action is explicit launch approval.
- `export_support` → structured redacted data, no auto upload.

Environment: `{id,name,host,surface,root,executable: string|null,ownership,status}` plus additive fields.
Setting: `{key,label,value: string|null,source,effect_timing,status}` plus additive fields; four known telemetry flags only for values.
Plan: `{id,hash,environment_id,title,changes: {key,label,before: string|null,after: string|null,path}[],preserves: string[],warnings: string[],actions: {id,label,reversible}[],created_at,status}`. Additional fields include `archive_passphrase_required` and `file_count` for rebuild; immutable target snapshot held privately.
Receipt: `{id,plan_id,environment_id,title,status,steps: {id,label,status,message}[],created_at,restorable:boolean,warnings:string[]}`. Store per-step status and uncertain effects. No unverified success.

Desktop Tauri command `request` accepts `{payload: request}` and returns the envelope. Frontend adapter throws useful errors. Browser preview may use a visibly marked isolated synthetic transport; production uses Tauri only. No silent fake-data fallback.

Browser native and network modules use explicit Tauri `browser_request` and `network_request` commands, documented in docs/browser.md and docs/network.md. They cannot invent generic system exec/read/delete endpoints.

## Integration decisions

- CLI binary is `lintel`; `lintel request` reads one JSON request from stdin, prints one envelope. No startup chatter on stdout.
- `job` accepts `job_id`; lookup should also accept `plan_id` when execute response was lost. Stable ID must be computable before submission (prefer receipt.id = plan.id).
- Reset/migration work selection uses category identifiers `instructions`, `memory`, `sessions`. Unsupported categories must return explicit validation error, not silently drop.
- Desktop Tauri is a standalone Cargo workspace with path dependency to crates/core; root default members core+runner+egress.
- Browser native host remains standalone under extensions/browser/native-host, with a path dependency from the desktop and a finite browser_request control adapter.
- Remote transport calls fixed `lintel request` with JSON stdin, never user-interpolated shell commands.

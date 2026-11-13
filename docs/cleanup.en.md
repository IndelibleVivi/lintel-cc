# Authentication sources, cleanup and independent verification

[简体中文](cleanup.md) · English · [Operator guide](operator-guide.en.md#cleanup)

**This guide describes the November 13, 2026 development source. The published App 0.1.1 and CLI 0.1.0 do not include this work.** It extends Lintel's environment management, work preservation, rebuilding and original-job recovery. Every operation retains an explicit host and configuration root.

## Understand the inventory

`claude setup-token` prints a long-lived OAuth token without saving it. A shell, Claude settings, an IDE, service or CI system may later inject that token. There is no fixed setup-token file to remove. Ordinary login files and Keychain are separate sources. See the [engineering evidence record](credential-provenance-evidence.en.md) for official sources and version conditions.

The authentication list in Cleanup and rebuild reads necessary content only at bounded, known locations. It does not execute shell startup files, helpers, hooks, MCP or Claude, scan history or the whole disk, or display tokens. Environment and Components lead into the same cleanup workflow.

| Observation | What it establishes | What it does not establish |
| --- | --- | --- |
| Identified local source | An exact file, JSON field or supported shell assignment exists | Claude is currently using it, or the token remains valid |
| Observed in the current execution environment | The Lintel App/current runner inherited the variable; an empty value is labeled separately | Which declaration supplied it, whether its value authenticates, or whether other terminals and IDEs have it |
| Declaration only | A helper, dynamic shell expression or authentication selector is declared | The script ran, returned a valid token, or was selected by this client |
| Inaccessible or unattributed | Permissions, format, file type, budget or ownership prevents a conclusion | No credential exists, or deletion is safe |
| Other contexts outside coverage | Other processes, IDEs, services, machines, CI or administrator systems may supply credentials | This local check covered those systems |

Stored credentials and credentials in use are distinct facts. Several sources may coexist; a simplified priority table does not determine the active account. Explicitly request the official CLI report when needed. It represents only this executor, exact root and observation time.

## Complete a cleanup in the App

1. Confirm the host, Claude Code environment and configuration directory in the target bar. Check the full path for a custom `CLAUDE_CONFIG_DIR`; project working directory and configuration root are separate.
2. In Cleanup and rebuild, choose login repair, rebuild or retirement. Refresh sources and inspect their locations, evidence and reasons an item cannot be handled. Select supported sources. Shell and user configuration can affect several environments belonging to this user; review that shared scope.
3. Close the related terminals, IDEs and automatic restart sources. Explicitly bound systemd services retain the existing separately approved pause operation. Known writers or an incomplete safety check block execution; the stopped-writers checkbox never overrides them.
4. Preview one cleanup plan. Review selected sources, exact editing scope, preserved work, unknowns and any preservation needed for rebuilding or retirement. Those two recipes still require at least one work category and an encryption passphrase.
5. Approve the reviewed plan once. Execution rechecks selected files and their frozen scope. An observed external edit stops subsequent actions instead of being overwritten. Credential editing does not create plaintext or encrypted token-recovery backups.
6. Read step receipts and the independent check. Selected local sources, current-executor authentication, other contexts and server revocation have separate results. Refresh again as needed; after an uncertain submission, query the original job first.

One approval combines only sources this plan can safely handle. Unsupported objects retain explicit explanations and next steps. The plan never silently expands to another machine or account.

When post-cleanup runtime status is explicitly selected, the plan freezes the authentication executable’s path and identity, including its absence. Installing or discovering a client after preview makes the old approval return `executable_changed` and requires a fresh preview. For a plan that selected runtime status, rechecking its original job also cannot silently adopt a later executable. A new explicit check without an original job ID may use the currently discovered client.

## Supported and preserved scope

| Source | Development-source behavior |
| --- | --- |
| Recognized `claudeAiOauth` in the target `.credentials.json` | Removes only that login field, preserving other fields and the file's other uses. Unknown or malformed schemas are not deleted wholesale |
| Recognized `primaryApiKey` in the target client state | Uses `~/.claude.json` for the default root or `.claude.json` within a custom root. Removes only that field and preserves other state and MCP configuration |
| Known authentication variables in target user settings | Removes selected fields while preserving unrelated configuration. Empty values, unexpected types and effective use remain separate observations |
| `apiKeyHelper` in user settings | Removes the declaration without executing or deleting the script, vault or external credentials |
| Supported standalone literal assignments in known shell startup files | Removes the frozen assignment scope, preserving the rest of the file. Complex syntax and dynamic injection remain with an explicit limitation |
| Variables already inherited by Lintel/the runner | Observation only; cannot change a parent shell, other processes or permanent environment. Restart the relevant launcher and check again |
| Project/managed settings, unknown shell syntax, IDEs, services and external profiles | Bounded observations describe candidates and gaps. Rows without safe-edit support cannot be selected |
| Keychain | No guessed service names or bulk deletion; automatic Keychain cleanup is unavailable in this development |
| Sessions, instructions, memory, projects, MCP/plugin configuration and unknown credential fields | Preserved; work preservation and import retain their existing content contracts |

This is versioned, field-level recognition. Removing `claudeAiOauth` or `primaryApiKey` does not prove other credential types are absent. Mixed `.claude.json` retains everything except the selected supported field, including MCP and other working state. Rebuilding prepares a new root and imports approved work; retirement disables the old Lintel launch entry. Neither uninstalls Claude or empties the original directory.

Shell support applies a conservative gate to the entire file: multiline syntax, control structures, functions, subshells, heredocs, continuations, pipes or redirection make every assignment in that file observation-only. Ordinary complete single-line `source`, alias and PATH declarations may coexist but are never executed; only standalone literal authentication assignments can be selected. Fish is observation-only. Limits are 1 MiB per JSON file, 256 KiB per shell file, 8 MiB total for the source module, 256 sources and 128 shell mention locations; startup candidate discovery has a separate 16 MiB budget. Exceeding a limit reports incomplete coverage rather than absence.

### Why automatic official logout is unavailable

Anthropic's current [Keychain recovery instructions](https://code.claude.com/docs/en/troubleshoot-install) say `/logout` also removes saved MCP logins and plugin sensitive values. CLI `auth logout` has no verified adapter meeting this development's preservation scope. New automatic-logout requests therefore return `official_logout_scope_unsafe` without invoking it. This decision follows official documentation and scope requirements, not a real-client trial in this development.

Review the official account-management procedure and its MCP/plugin reauthorization effects separately when handling Keychain. The local result continues to show this scope as unverified. Lintel does not bypass that limit by deleting a whole Keychain item.

## Inspect, select and approve with the CLI

IDs below are placeholders. Use the environment, source, plan ID and hash returned by the current host. These requests contain no tokens. `call` uses the same operation schema and core as the App; JSON stdin is not a separate cleanup engine.

```sh
lintel describe auth_inventory --json
lintel schema plan_cleanup
lintel call auth_inventory <<'JSON'
{"environment_id":"<environment-id>","project_cwd":"/synthetic/project"}
JSON
```

`project_cwd` is optional. When supplied, it must name an existing directory on the target host. It expands read-only source context without making project files cleanup targets.

The named CLI maps to the same operations:

```sh
lintel auth inventory --environment '<environment-id>' --json
lintel auth cleanup plan --environment '<environment-id>' --recipe repair_login \
  --writers-confirmed-stopped --auth-source '<source-id-from-inventory>' --json
lintel auth verify --environment '<environment-id>' --job '<original-job-id>' --json
```

Repeat `--auth-source` for additional supported sources. Optional `--project-cwd` adds read-only project context. Plan flag `--verify-auth-status` and independent verification flag `--observe-runtime` explicitly request official runtime status; both default off. The raw example below exposes the same fields; use either form to create the plan.

Select original IDs of supported rows and create a plan:

```sh
lintel call plan_cleanup <<'JSON'
{
  "environment_id":"<environment-id>",
  "recipe":"repair_login",
  "writers_confirmed_stopped":true,
  "official_logout":false,
  "auth_source_ids":["<source-id-from-inventory>"],
  "verify_auth_status":false
}
JSON
lintel plan show '<returned-plan-id>'
lintel job submit --plan '<returned-plan-id>' --approval '<returned-plan-hash>'
lintel job show '<returned-plan-id>'
```

`auth_source_ids` freezes the selection. Refresh observations and create a new preview after a relevant file change or stale plan. Source IDs are not arbitrary deletion paths. `verify_auth_status:true` explicitly requests the official status command; the default static check does not run Claude. Remote use retains the finite SSH request/submit and original-job query mechanisms. Source IDs are not portable across hosts.

An independent recheck can reference the original job. By default it only rereads bounded sources:

```sh
lintel call auth_verify <<'JSON'
{"environment_id":"<environment-id>","job_id":"<original-job-id>","observe_runtime":false}
JSON
```

Only explicit `observe_runtime:true` invokes official `auth status`. It may access authentication facilities, execute helpers or contact networks; raw tokens and account details are not returned. Verification runs the selected status check before the final file reread so credentials recreated by status/helpers can be observed. If the CLI version or output cannot establish the exact root, the operation reports the limitation. Exit codes, unknown schemas and network failures are never converted into an all-authentication-cleared claim.

## Interpret the result

| Result layer | Next step |
| --- | --- |
| Selected local sources handled | Check exact source rows and completed steps; other rows may still exist |
| A source reappears, an edit fails or a file cannot be read | Preserve the current file and inspect the original error and location. Identify the writer, then preview again; never automatically delete again |
| The current executor still inherits authentication variables or can authenticate | Stop old processes, handle remaining sources and restart the actual terminal, IDE or service. Repeatedly deleting an already-handled file does not fix this |
| Keychain, profiles or another context is unknown | Check its own interface or involve its administrator. The local result does not extend to it |
| Server revocation is unverified | Use the credential provider's normal revocation/rotation mechanism independently. Local removal does not invalidate copies on another machine, in CI or elsewhere |

Editing a shell file does not change existing processes. Removing a variable from Claude settings likewise does not prove a running session has forgotten it. A new check after restarting produces a new observation.

## Old plans, interruptions and preservation

Old, unexecuted cleanup plans without the new authentication binding require a fresh preview. Previously accepted tasks and receipts remain queryable by their original IDs and are not replayed. Old raw callers may omit the additive fields, but newly generated plans default only to precisely recognized target-local login fields. They no longer treat a mixed credential file or `.claude.json` as a whole-file login cleanup target.

If submission returns `submission_uncertain`, keep the original plan ID and query that task. While safety checks are still running, a query may temporarily return `job_not_found`; this means durable acceptance has not yet been observed and does not authorize another submission or a new execution ID. Wait and query again. If the result remains unresolved, preserve the uncertainty and original ID instead of treating a missing receipt as completion or proof that execution never started.

Rebuilding and retirement retain encrypted work preservation, exact new-root planning, conflict detection and original-job queries. New authentication cleanup does not copy old tokens into plans, logs, receipts or state-recovery backups. Existing historical backups, shell history, sync copies and other machines are not silently swept. After interruption, inspect completed archives, new roots and source steps in the original receipt.

An exact edit moves the original into a private staging directory beside the source, then publishes the edited file without overwriting another writer. The original is moved, not copied as a backup, and is removed after successful readback. Interruption or conflict may retain credential-containing originals/staging, listed with locations and states in `retained_sensitive_material`, preventing local completion. If staging is renamed/replaced and its identity is lost, a `location_unknown` path is only the last known location, not evidence of destruction; Lintel does not scan to follow it. A missing field at its former path does not resolve those materials; inspect the original task. Lintel neither sweeps them automatically nor replays cleanup.

Safe editing still requires stopping external writers. Frozen bindings and rechecks reject observed edits but cannot provide an atomic transaction across files against arbitrary uncooperative processes, or prove physical destruction on an SSD, snapshot or server. Actual macOS Keychain and normal-user authentication acceptance remain separate from synthetic tests; see [current state (Chinese)](current-state.md).

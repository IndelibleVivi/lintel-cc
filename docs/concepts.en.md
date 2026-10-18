# Core concepts

[简体中文](concepts.md) · English · [Documentation](index.en.md)

These distinctions explain where an operation runs, what it targets and how to retrieve its result after changing windows or losing a connection.

## Host, configuration root, project directory and browser

The **host** selects the execution machine and user. Local means the current Mac; an SSH workspace selects the remote host and login user. Remote environments, plans and records belong to that host.

An **environment** is a Claude Code configuration root registered with Lintel. The **project cwd** is the code or working directory used when launching Claude. They can differ; changing the root does not move project code.

A **browser profile** is selected and paired independently. The app's local browser does not move to a VPS when the SSH target changes. Separate configuration directories do not automatically establish account, credential or operating-system isolation.

## Preview, plan, task and result

A **preview** exposes the actual scope. For plan-based core tasks, a **frozen plan** records the exact target, selected content or field changes, with an ID and hash. Approval must match that plan. A change in relevant target/content requires a fresh preview.

A **task** is the persistent record of a submitted operation. Core jobs use the original plan ID. `accepted` means durably received; `completed` means the task achieved its own outcome. Inspect steps, coverage and errors for the result of each part.

Querying the original task does not replay destructive actions, but it can persist interruption-reconciliation information. Restoration requires a separate reviewed and approved plan. Browser operations, launch requests and app-update intents have their own records; their IDs cannot be exchanged or submitted as generic core jobs.

## Work package, import and session continuation

A **work package** is an encrypted `.age` file containing selected originals. Archive-only keeps the old directory/login and creates no new configuration root. Preservation with a new environment adds new-root preparation and selected import steps.

A work package can open in independent state without the source inventory/job. Choose a system file-transfer tool explicitly when carrying it between machines; the SSH controller does not transfer packages implicitly.

**Import** publishes selected material to the previewed destinations without overwriting same-name content. Preserving a transcript, reading it, drafting a handoff and resuming a native session are separate operations. Native resume has its own format, version and authentication conditions; see [current status (Chinese)](current-state.md).

**Observation, baseline acceptance and restoration.** Sanitized semantic snapshots support comparison. Accepting current values item by item only establishes a comparison baseline and does not create recoverable work content. Configuration restore, residue-backup restore and work import read different sources and each requires its own preview and approval; one original ID cannot substitute for another kind of restoration evidence.

## Presets, ordinary editing and bulk changes

A protection draft belongs to one host and environment. Ordinary editing chooses an action on the row for that field. Bulk edit offers only actions common to every selected item, without silently omitting selections outside the filter. A personal default stores non-secret intent only; if it differs from a built-in recipe, the label describes the fields actually enabled, disabled or removed, and the next application still reads the target and generates a fresh plan.

## Configuration readback and actual outbound behaviour

Settings readback verifies that the target fields were handled as planned; an existing Claude process is not restarted automatically. Controlled-channel observations cover connections through that proxy, not direct outbound traffic. An explicit IPv4/IPv6 probe describes the tested route at that time.

Continue with [Protection](operator-guide.en.md#protect), [Network](network.en.md) or [Telemetry](telemetry.en.md).

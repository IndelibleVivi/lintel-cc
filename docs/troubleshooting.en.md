# Troubleshooting by observed result

[简体中文](troubleshooting.md) · English · [Documentation](index.en.md)

Keep the host, configuration path, original task/request ID and error code first. Do not delete Lintel state merely to start over; the original record may be the evidence needed to resolve the result.

| What you see | Next action | Explanation |
| --- | --- | --- |
| Records reports unreadable entries, or `invalid_job_record` | Preserve the damaged original and ID; query readable tasks separately and report the error code | Do not delete jobs/plans to retry; healthy records remain visible; pagination is not delivered |
| A web page displays the UI but cannot operate locally | Use `npm run dev:synthetic` for the source trial, or the native app for real environments | Plain frontend development and the website have no real execution channel |
| The app works, but the shell cannot find `lintel` | Select the standalone CLI's absolute path; inspect `version --json` and `context --json` | Installing the app does not modify PATH; see [Agent CLI](agents.en.md) |
| `accepted` or `ok:true`, with no clear outcome | Query the original job and inspect status, steps and coverage | Reception, request handling and task completion are distinct |
| SSH disconnects, waiting times out, or `needs_reconciliation` appears | Return to the original host/state and query the original ID; decide on restoration separately | Do not resubmit the same destructive action; see [Remote](remote.en.md) |
| Approval is rejected, a preview expires or the target changes | Read the target again, generate and review a new plan | Do not reuse a stale hash or bypass target checks |
| Restore is blocked by a field conflict | Inspect edits made after the task and decide from current values | Later edits are preserved; restore applies only to owned fields |
| Package reading is slow or preflight rejects capacity | Check the current runner limits, exact selection and temporary disk space; reduce the selection precisely | Each page requires whole-package decryption/verification. Archive and native-resume limits are separate; see [Work](operator-guide.en.md#work) |
| Browser cleanup waits for a follow-up | Verify the original profile/operation, then perform the documented full restart and second approval | Quitting the app or closing a tab is not a full browser restart |
| A running Claude session does not reflect changed settings | Inspect readback and launch a new session with the selected root | Existing processes do not refresh their environment automatically |
| Custom protection has many rows, or only a few should change | Filter first and choose actions per row; enter Bulk edit explicitly when needed | Bulk offers only common applicable actions and counts selections outside the filtered result; see [Protection](operator-guide.en.md#protect) |
| A personal default's name does not match its configured intent | Read the "personal default · name" and actual description, not only the recipe slot | Saved intent that differs from a built-in recipe is labelled from its semantics and every run still needs a fresh preview |
| IPv6 or proxy results say untested or failed | Check the host, channel and echo target; test only after explicitly choosing it | Untested does not mean blocked or leak-free; see [Network](network.en.md) |
| App updates, remote installation or browser host is unavailable | Check release configuration, platform and bundled resources | Source support, packaged resources and a working instance are checked separately |

## A useful issue report

Include version/candidate source, OS/architecture, app or CLI entry point, target type, reproduction steps, original ID and error code. Redact paths while retaining useful structure. Do not attach credentials, passphrases, raw sessions, work packages or complete state to a public issue; prefer a synthetic reproduction.

Consult [Core / CLI](core.en.md) for mechanisms and [current status (Chinese)](current-state.md) for verified platforms and candidates.

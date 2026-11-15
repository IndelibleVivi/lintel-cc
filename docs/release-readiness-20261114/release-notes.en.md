# Lintel 0.2.0 Preview — release notes

These notes apply to Lintel 0.2.0 Preview. The actual attachments on the [v0.2.0 release page](https://github.com/IndelibleVivi/lintel-cc/releases/tag/v0.2.0) provide the packages, checksums and bilingual installation steps; source or a version number alone does not establish a public download. This version uses manual downloads and upgrades, without automatic updates.

## What this version improves

Lintel helps you choose a Claude Code environment, manage outbound settings, preserve and read working materials, precisely remove old local authentication, operate a selected browser profile or remote host, and find or recover the original task. Version 0.2.0 brings the recent credential-source, Keychain, task experience and bilingual interface work into one App/CLI candidate generation.

### Credential sources and precise cleanup

- Review recognized files, Keychain items, settings/shell declarations and current-executor injection in one preview. Saved logins and shared declarations remain distinct.
- On macOS, update only the recognized Claude login field in the original Keychain item. Preserve MCP, plugin and unknown data and item identity. Cleanup does not export a credential-recovery backup.
- Preserve the different credential namespaces of the default Claude directory and an explicitly selected directory. Refuse a cross-environment or unverified independent secure-storage selector instead of silently selecting another login.
- Report selected local sources, the current executor, other contexts and server revocation separately. Removing a local login neither revokes a token nor removes an environment variable from an already running terminal.

### Original tasks and recovery

- Keep readable remote tasks visible when another record is damaged, and identify records that could not be read. An incomplete list no longer appears to be complete history.
- Block a new runner installation when the original installation record cannot be reconciled, preventing a second action over uncertain state.
- Fail finitely on abnormal file substitutions such as pipes. Retain uncertainty when settings have been published but the durability confirmation fails.
- Query accepted tasks by their original ID without re-executing them. Unattempted old previews that lack the new authentication-directory binding need a fresh preview; already accepted originals remain query-only.

### A clearer App and stronger packages

- Improve Chinese/English, Day/Night, small windows, complete labels, cleanup entry points, archive results and keyboard focus. Errors and partial results explain the next useful action.
- Give the toolbar’s Check environments and the record page’s single Refresh records action distinct scopes. Record refresh preserves search/filter; empty, unmatched, incomplete and unreadable records remain distinct.
- Prioritize saved logins and select injections when needed. Group project/file metadata, read-only context and absent locations behind supporting checks; align icons, chevrons, work-capacity facts and explanatory spacing.
- Show the frozen action scope for zero-source login verification, rebuilding and retirement; the latter two retain work preservation and archive-passphrase requirements. Correct or clear an invalid project directory and recheck on the same page.
- Align the App and CLI product version at 0.2.0 while retaining the Preview channel. Packages keep project terms separate from third-party notices.
- Check architecture, actual executable permissions, complete internal checksums, resources, native version metadata and final DMG contents at candidate admission.
- Update the TLS roots and certificate-verification dependencies used by HTTPS upstream proxying and probes to reject distrusted certificate chains.

## Supported scope and installation choices

Download targets are the Apple Silicon macOS App/CLI and Linux x86_64/aarch64 CLI; the actual release attachments and corresponding native reports define the provided platforms. Linux CLI remains experimental. Cross-compilation is not target-machine execution.

Automatic updates are not configured. Upgrade by manual download and a separate version directory, retaining Lintel state, work archives, original task IDs and the previous installation. Browser companion extensions retain their existing developer/temporary installation route; this version does not add browser-store distribution.

Lintel is an independent project without official Anthropic affiliation or endorsement. Its wordmark and App icons were made by the project maintainers. Clawd rights remain with the respective rights holder and are excluded from Lintel’s software and documentation grants; no Anthropic redistribution permission is claimed.

The App is a technical Preview with a complete bundle-level ad-hoc signature, without Developer ID signing or Apple notarization. macOS may block first launch. After checking the source, the user decides whether to use Apple's per-App exception procedure. Lintel does not disable Gatekeeper or automatically remove quarantine. This version does not promise first installation without system trust prompts.

Results cover selected, recognized sources and supported operations. Unknown providers/helpers, other processes, other devices and server state remain separate observations. Source inventory does not run a helper. A separately selected official status command may invoke the client's configured helper and recreate credentials; final local readback follows that command. Lintel does not sweep all Keychain accounts or promise account unblocking or complete removal of server-side associations.

## Upgrading and recovering from a problem

Record the original IDs of unfinished tasks and query those tasks before closing the App. Extract the new CLI into a new version directory, verify it using the bundled README and run its explicit path; do not overwrite an executable in use. Quit the old App and retain it before replacing the installed copy. If installation or upgrade fails, preserve state and archives and query the original ID. Deleting task records or resubmitting uncertain work is not recovery. Retaining an older executable does not establish safe writes to every future data format; verify the supported compatibility before making new changes with it.

See [current state](../current-state.md) and the engineering handoff in this directory for fixes and evidence boundaries; historical candidate reports retain their own source scope. Manual installation notes accompany this Release. Updater feeds and website deployment are recorded separately.

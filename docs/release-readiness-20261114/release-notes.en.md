# Lintel 0.2.0 Preview — release notes awaiting approval

These notes describe an unpublished candidate. A version number does not mean that it has shipped. The final handoff in this directory owns the actual artifacts, digests and acceptance results. Public downloads remain App 0.1.1 Preview and CLI 0.1.0.

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
- Align the App and CLI product version at 0.2.0 while retaining the Preview channel. Packages keep project terms separate from third-party notices.
- Check architecture, actual executable permissions, complete internal checksums, resources, native version metadata and final DMG contents at candidate admission.

## Supported scope and installation choices

Candidate targets are the Apple Silicon macOS App/CLI and Linux x86_64/aarch64 CLI. The final native reports determine which platforms may ship; cross-compilation is not target-machine execution.

Automatic updates are not configured. Upgrade by manual download and a separate version directory, retaining Lintel state, work archives, original task IDs and the previous installation. Browser companion extensions retain their existing developer/temporary installation route; this version does not add browser-store distribution.

If the approved release remains an ad-hoc App Preview, its notes must explicitly state that it has no Developer ID signature or Apple notarization and that macOS may block first launch. After checking the source, the user decides whether to use Apple's per-App exception procedure. Lintel does not disable Gatekeeper or automatically remove quarantine. General distribution for ordinary users should first complete signing, notarization and real download/first-install acceptance.

Results cover selected, recognized sources and supported operations. Unknown providers/helpers, other processes, other devices and server state remain separate observations. Lintel does not run an unknown helper, sweep all Keychain accounts or promise account unblocking or complete removal of server-side associations.

## Upgrading and recovering from a problem

Record the original IDs of unfinished tasks and query those tasks before closing the App. Extract the new CLI into a new version directory, verify it using the bundled README and run its explicit path; do not overwrite an executable in use. Quit the old App and retain it before replacing the installed copy. If installation or upgrade fails, preserve state and archives and query the original ID. Deleting task records or resubmitting uncertain work is not recovery. Retaining an older executable does not establish safe writes to every future data format; verify the supported compatibility before making new changes with it.

See the engineering handoff in this directory for fixes, evidence scope and remaining acceptance conditions. These draft notes do not authorize a GitHub Release, production updater feed or website deployment.

# Lintel

[简体中文 (canonical)](README.md) · English · [Documentation](docs/index.en.md) · [Website](https://lintel.page/en/)

![Lintel Preview: the terracotta l tail above a fine ASCII landscape](assets/preview/lintel-banner.png)

A clean, personal tool for Claude Code users. Review configuration changes,
preserve selected instructions, memories and sessions, and check operation results.
The macOS app and standalone CLI share the same execution core.

**Lintel 0.1.0 Preview / development preview. The source is public; you can try
it from source in an isolated test environment. There is currently no public
Release download and no signed or notarized distribution.** The full SPEC is
not delivered. Limited local cleanup, the official logout entry, work
archive/migration and SSH control are wired up. Real authentication, production
Chrome/Edge/Firefox/AdsPower, OS-level network enforcement, and production
remote environments remain unverified. See
[current state (Chinese)](docs/current-state.md) for the current candidate source and
layered evidence. Public source is not a general reuse licence; a licence for
the original materials is still to be chosen.

## First trial: get the source, then one guided journey

You can try the real CLI today from public source. There is no public Release
download. Get it and build it:

```sh
git clone https://github.com/IndelibleVivi/lintel-cc.git
cd lintel-cc
cargo build -p lintel-runner
```

Then follow the **[English quickstart](docs/quickstart.en.md)** (or the
[Chinese quickstart](docs/quickstart.md)). In one disposable synthetic
home/state it registers an environment, previews an exact selection, reviews
and approves a frozen archive-only plan, queries the original job, and
re-inspects/reads the carried `.age` package in a **separate independent
state** — without touching your real Claude login, settings or browser data.

## Three core values

1. **Preserve selected work.** Generate an encrypted archive only, or archive
   and then prepare a new config root and migrate the content you select. The
   independent `.age` package can be read and selectively imported without the
   original job or state. Archiving keeps content as data; native continuation requires a separate supported, approved request and remains unverified with real authentication.
2. **Review changes.** Preview the exact settings changes before approving
   them, then read back the result. Seven recognized controls can be kept,
   disabled or removed per field, with current value, source, impact and
   restoration shown. Reading configuration is separate from what actually
   runs.
3. **Check original results.** Build a frozen, hashed plan, review it, approve
   the exact hash, and query the original job afterwards. Interrupted work is
   checked by querying the original ID (query-only, never auto-replayed or
   auto-recovered). Field restore detects later external edits.

## Start from the goal in front of you

| What you want | Current entry | How-to |
| --- | --- | --- |
| Send less data out while keeping the features you need | Protect plan | [Choose a plan and check the result](docs/operator-guide.en.md#protect) |
| Collect instructions, memory and sessions, or prepare a new environment | Work preservation | [Archive-only, preserve and portable work](docs/operator-guide.en.md#work) |
| Fix login, clean client state, or retire an environment | Cleanup and rebuild | [Exact scope and preserve-first order](docs/operator-guide.en.md#cleanup) |
| Manage a specific browser profile | Browser on the home screen | [Load, pair and two-phase clear](docs/browser.en.md) |
| Manage SSH hosts and explicitly bound services | Hosts and SSH | [Connect, query original jobs and services](docs/remote.en.md) |
| Check for changes, restore settings or recover an interrupted result | Records and recovery | [Original-job query and conflict handling](docs/operator-guide.en.md#recovery) |

For a first run, start with the [English quickstart](docs/quickstart.en.md);
the [human operator guide](docs/operator-guide.en.md) covers the GUI
workflow task by task. The sidebar "Terminal and Agent" lets you choose the
exact CLI, check HOME/state and execution user, and review and copy the current
task's operation handoff. The [Agent CLI guide](docs/agents.en.md) covers obtaining
a candidate, installing it independently, and automation. The home screen, help
and `lintel tasks --json` share one
[six-task map](contracts/task-catalog.json).

A browser clear started from the App can continue from a popup after a full
browser restart, and the final result returns to the original App task. An
unexecuted preview can be cancelled or expires after five minutes; retrying
requires a new preview and approval, and old IDs are never redone.

## What you can do today

- Current App source supports Simplified Chinese / English, follows the macOS App language by default, and remembers an explicit choice in Settings. UI text, Clawd greetings and dates change together while the current task, drafts and approved plan remain intact; paths, commands and original work content retain their exact bytes. Translations ship with the App without an online translation service. See [current state (Chinese)](docs/current-state.md) for which built candidates include this change. The CLI and browser extension retain their independent languages. Website source now has Simplified Chinese/English entrances; deployment status is separate in current state.
- Select exact optional telemetry destinations from a static official-source catalog in the existing controlled-channel draft. Claude settings, active rules, controlled rule tests and observed requests remain separate. Mixed model/authentication/download hosts are informational; the NONESSENTIAL switch also disables Claude Code updates. See [telemetry control](docs/telemetry.en.md).
- Open Settings → Lintel version and updates for the App version and manual checking. Background checks default off; download/signature verification, approved installation and explicit restart are separate. Source builds without a release key/feed show an explicit limitation. There is no formal Release or activated feed. [App updates and release preparation](docs/app-updates.en.md) describes the common version record for future builds, website DMGs and updater archives.
- Register a Claude Code config directory, or create a dedicated environment.
  Select the environment at the top, open the protect plan, preview the exact
  changes, then approve execution.
- Choose "keep features", "reduce egress" or "custom". Custom lets you keep,
  disable or remove each of seven recognized controls, showing current value,
  source, impact and how to restore; other settings, general proxy and custom
  OTel are preserved. Installed versions are identified statically. Remote Control is evaluated against version and declared Trusted Devices requirements; exact conflicts enter the reviewed diff, and unknown conditions remain unknown. Configuration readback and runtime behavior stay separate.
- View durable task results, check for settings drift, accept the current
  values, and restore Lintel's field changes from its own records. After an
  interruption, querying the original job checks the concrete settings
  publication evidence before an independent restore preview is offered;
  later conflicting edits block restore.
- Work preservation can archive only, or archive and prepare a new config root
  and migrate the selected content. Both report completion against their own
  goal; the old directory, login and settings are preserved. An independent
  `.age` package remains readable off the original job/state.
- Before preserving, select by whole category, by project path group, session
  file group, or individual file. Oversized sessions can be explicitly
  excluded while other items are fully preserved. A frozen plan also has a
  16 MiB persisted-record budget; deep paths and many destinations can be
  rejected while still within the body limits, in which case narrow the exact
  selection. The paged file inventory reads metadata only; approval freezes exact paths and complete digests without adding unselected files. Current source admits 256 MiB per file, 1 GiB total and 10,000 files, preserving `lintel.work/1` original bytes through streaming and private temporary-disk verification without truncation. Older candidates/runners retain their own limits. Every page first decrypts/verifies the complete package, requiring temporary disk and time. The UI shows ongoing reads and queue waits, labels previous pages while loading, and clears page text/passphrases on leaving; already submitted reads continue before queued operations. Finite SSH limits remain independent.
- Environment details show connected components by exact host/root: limited
  CLI, config sources, auth location metadata, original service records and an
  independent browser entry. A read-only capacity preflight flags over-limit
  files and scan gaps; the real plan still re-reads all selected originals. Unknown and unsupported components differ. The UI uses the current runner's reported limits to exclude oversized originals, without guessing missing capacity reports.
- Unlock an archive in the current task or an independent work package on a
  target host, page through messages, tools and unknown records, generate an
  editable handoff draft, and separately create an import plan. The approval
  view and CLI show each selected file's source, final destination and digest.
  Ciphertext transfer uses explicitly selected system tools. Import never overwrites a same-name file and requires filesystem/kernel atomic no-replace rename support, otherwise refusing while preserving existing content. Cleanup offers login repair, rebuild and retirement with exact authentication/file scope. Official logout needs an observable logged-in subject and rechecks the frozen authentication state before execution and after preservation. Local handling never proves server revocation.
- Open the selected config environment in an independent project cwd. Launch
  and resume previews list limited roots, project/ancestor candidates and
  system managed config sources, and flag hooks/MCP/helper declarations and
  unknown auth. They do not execute config content or read credential bodies,
  and do not prove actual loading or isolation. Native resume is a separate
  approved finite entry using a byte-identical private transcript copy, static version support and original-request checks. Unknown versions refuse; real authentication and actual recovery remain unaccepted. Reviewed handoff text can be copied before opening a blank session; users paste and send it themselves.
- The default home screen is a quiet Clawd companion, with original time-of-day
  greetings; an explicit action opens six task choices. Day / Night / System
  themes are available. Clawd has moon and snack surprises, and the
  pocket menu includes a moon-picking action. The pocket playroom shares one opt-in runner with the
  website: Clawd keeps its connected short legs, bounces over obstacles, and
  collects stars at different heights. Current source jumps immediately on press; short taps keep the original height and holding adds lift to that same jump. Starting speed is 260px/s, increasing smoothly to 350px/s, with ordinary rewards placed along the tap arc and adjusted above taller obstacles, plus a high +50 star every fourth star. The original monochrome pixel/ASCII-like hills remain, with forest and courtyard marks appearing as distance grows. Stones, books, plant pots and fences vary in size and spacing; existing App candidates retain the older game. Scores stay local to each client.
- Explicitly test IPv4/IPv6 from the host-default path and the selected Lintel channel, with echoed IP, elapsed time and specific failures. An absent channel stays untested. macOS supports an approved shared-service IPv6 off/link-local change, automatic comparison against the same targets and complete original-configuration restore. Linux offers readonly inspection and probes. Real macOS configuration round trips remain unverified; see the [IPv4/IPv6 Leaf (Chinese)](docs/specs/ip-network.md).
- Optionally start a loopback proxy in the desktop environment detail, set a
  default allow/block and exact host/port rules, read back the live config, and
  explicitly request opening Claude through that channel. Only connections
  through the proxy are covered. The additive `address_family` option selects system routing or strict `ipv4_only` for sockets Lintel creates toward the destination or upstream; it does not prove the upstream’s final public address.
- From the home screen "Browser" entry, prepare the companion extension,
  install the local connection, pair and view instances. The App ships
  Chromium/Firefox extension and Native Messaging host. The extension still
  needs a developer load; signed/store distribution is not done. Site clear is
  two-phase: isolated preparation, a full browser restart, then a separate
  confirm to delete.
- Register, remove and undo removal of an SSH alias on the desktop, reusing the
  same environment, cleanup, archive and task views. Removing an alias affects
  only Lintel's host list; system SSH config and original tasks are retained.
  Linux x86_64/arm64 hosts can "check and prepare the runner" from the host
  panel: preview, then approve installing the App's built-in static runner.
  Only the target user's dedicated version directory is written; no compiler or
  sudo is needed on the VPS.

Remote connection errors distinguish SSH, runner and response failures, with bounded diagnostics and a manually copyable summary. Installation persists intent before upload, then checks bytes and actual capability before binding. Interrupted installation queries the original ID, newer bindings are never rolled back by historical queries, and unresolved installs block another upload. Existing PATH runners remain compatible. Interactive Open Claude uses the same strict SSH alias, selected root, pinned runner and Terminal PTY without a prompt. Mutation jobs ACK only after durable acceptance. Already-root systemd or an existing lingering current-UID user manager can own a per-job transient service; otherwise setsid explicitly limits continuation. Lintel never enables linger, invokes sudo or changes login policy. Lost ACKs query originals, without replay or reboot-survival promises. Independent VM evidence covers eligible logout continuation and original-ID reconciliation after real reboot; production VPS acceptance remains separate. On reopening the App, persistent host records can query the original and open its full receipt; settings and service restoration still require separate approval. Removed hosts retain query-only recovery until registered again.

App help offers the six goals, human and Agent guides, source information and the independent [Infra Field Guide](https://github.com/IndelibleVivi/infra-field-guide). User-selected links open in the system browser without environment/diagnostic payloads. New independent-cwd launch/resume previews require filesystem birth time; missing evidence is an explicit limitation while reading remains available. Old unattempted previews need new checks; attempted originals remain query-only. Ordinary project-file edits do not change directory identity.

See the [operator guide](docs/operator-guide.en.md) for the human workflow and
[remote](docs/remote.en.md), [browser](docs/browser.en.md) and [services](docs/services.en.md)
for the specific limitations.

## Local build and trial

Requires Rust stable, Node.js and npm; the macOS desktop build also needs Xcode
Command Line Tools. The current local build and test host is macOS arm64. Linux
source has passed synthetic journeys, independent OpenSSH and real
systemd/PAM/reboot VM checks in Ubuntu CI; see the
[verification guide (Chinese)](docs/verification.md). Real authentication and production
VPS are verified separately.

```sh
cargo build --workspace
cd apps/desktop
npm ci
npm run dev:synthetic
```

Open the local address the dev server prints. `dev:synthetic` creates an
isolated temporary home/state and calls the real CLI; the interface clearly
labels it a "test space" and never uses your Claude login or browser data. The
generated temporary directory is retained for inspection. Plain `npm run dev`
does not provide a browser-to-host execution channel.

To build the desktop App, first prepare both static Linux runners following [remote resources](docs/remote.en.md#prepare-app-resources) if installation support is needed. Missing resources produce an explicit App limitation:

```sh
cd apps/desktop
# Without Linux runners, create the empty resource directory; installation reports missing resources.
mkdir -p src-tauri/runner-bundles
npm run desktop:build
```

The build prepares Chromium/Firefox extensions and compiles/packages the current-platform browser host. It supports native builds matching the Rust host; cross-architecture/universal builds refuse. Default frontend builds exclude optional local fonts and use system fallbacks. Output is `apps/desktop/src-tauri/target/release/bundle/macos/Lintel.app`: a local candidate without Developer ID, notarization or non-developer installation acceptance, never automatically copied into Applications. App users can prepare extension files from bundled resources; [browser installation](docs/browser.en.md) retains separate developer-loading limits.

Standalone CLI (run these from the repository root):

```sh
./target/debug/lintel --help                  # static; no state, no discovery
./target/debug/lintel version --json          # static; no state, no discovery
./target/debug/lintel context --json          # read-only state/user/exe facts
./target/debug/lintel tasks --json            # static; the six-task map
./target/debug/lintel work --help             # static help; no state
./target/debug/lintel capabilities --json     # static catalog; optional target inspection
./target/debug/lintel describe plan_policy    # static operation contract
./target/debug/lintel env list                # may register discovered roots + save inventory
./target/debug/lintel tui                      # interactive; uses real state
```

`version`, `context`, `tasks`, `help`, `describe`, `schema`, and `capabilities`
without a target are **static**: they do not run Claude, discover personal directories, or
initialize state. `env list` and `tui` are different — `env list` may register
discovered default roots and save inventory, and `tui` operates on real state,
so do not run them against your real HOME just to read help.

Named commands or the compatible `lintel request` share plan / approval /
execute / query / recovery. **Rejected submissions exit non-zero; an accepted
ACK is not task completion.** Interactive start uses `lintel launch ID` on a
real TTY; generic `call launch` is explicitly refused.
Remote interactive launch uses `lintel remote launch ALIAS ENVIRONMENT_ID`, also on a real TTY; JSON `remote control` never launches an interactive session. Discovery inspects only known directories and executes no Claude, shell rc, hooks or MCP.

Independent CLI candidates can be packaged for macOS arm64 and Linux
x86_64/aarch64, each carrying both static Linux remote runners, candidate
identity and a file checksum manifest. There is no formal download link. See
[obtaining and installing a candidate](docs/agents.en.md#install-candidate).

## Data and boundaries

No Lintel account, model key or license key is needed. The app sends no
analytics, crash uploads or remote font requests by default; there is no online
activation service. The browser host and core each expose only their own
limited operations, with no arbitrary shell/file interface. Data is previewed
locally and copied manually; there is no automatic upload. Explicit IP tests and approved post-change probes contact the frozen HTTPS echo targets (ipify IPv4-only/IPv6-only by default), exposing the request’s public IP to them. No background public probe runs. A host IPv6 change affects every application using that service, persists until explicitly restored and may require macOS system authorization.

Config-directory isolation is not an OS sandbox and does not prove login
credentials are independent. Proxy environment variables are not OS-level
process network enforcement. Browser deletion cannot be undone; settings
restore checks current values. Lintel does not promise to change server-side
account state.

Linux cleanup can independently inspect and approve an exact root-bound systemd service pause or restoration. Pause writes only the original job's persistent startup blocker and stops that target; restore checks outside edits, removes only the owned blocker and starts only if originally active. Neighbor services and enablement remain. Current-user and already-root system managers have finite scope; other supervisors, containers and macOS services are unsupported. See [services](docs/services.en.md). Core locks, frozen snapshots and prewrite checks do not provide atomic compare-and-swap with outside editors; one paused unit cannot prove every Claude/IDE/interactive writer stopped. See [concurrency and recovery](docs/core.en.md#concurrency-and-recovery).

## Feedback and issues

Use the project's GitHub Issues for bug reports and questions:
<https://github.com/IndelibleVivi/lintel-cc/issues>. For a first-trial
roadblock, the [Preview feedback template](.github/ISSUE_TEMPLATE/preview-feedback.yml)
asks for source revision, platform, and steps. Attach only synthetic or
redacted information; **never** attach credentials, tokens, raw conversations,
work packages, personal browser data, an unredacted Lintel state export, or
private paths.

## Development and documentation

Use the [English documentation hub](docs/index.en.md) for the maintained reader path. Chinese is canonical; the eleven user/operator guides have English siblings. Specifications, current state and developer evidence retain Chinese authority and are labeled below.

The standalone static website source is in [`apps/site` (Chinese README)](apps/site/README.md).
It explains the six current tasks and uses a synthetic preservation journey to
show preview, approved execution, and original-job verification. The spacious character
landscape uses the native wordmark with a full orange final-l foot and a separate
slow-blinking header underscore. Lake taps make ripples that expand, fade and
expire; the three Clawds have quiet, temporary visual responses, with no discovery
counter or coaching prompts. Night scenery uses soft warm-gray tones while keeping
the original orange bodies and dark eyes. A hidden star map remains in the scenery.
An interactive four-step work-package journey explains the
flow; scenery can be paused and respects reduced motion. Source-reading examples,
Chinese and English trial links, and the opt-in Clawd game remain available.
A quiet link opens the 20-second English concept film with speech, music and sound
effects; playback starts only on request. The existing static site is live at
[lintel.page](https://lintel.page/), with same-origin hosting on Cloudflare Pages. The site provides `/` Simplified Chinese and `/en/` English, six tasks with original title icons and alternating SVG scenes, and tap or hold-and-release jumps. Shared track source now uses immediate press-and-hold jumps with the original monochrome scenery, three places, four obstacle families and varied spacing. [Current state (Chinese)](docs/current-state.md) records the live revision and candidate status.
It does not connect to the local execution core or offer a formal app download.
See the [site README (Chinese)](apps/site/README.md) to start it and the
[hosting guide (Chinese)](docs/site-hosting.md) to update it.

```sh
python3 tests/verify.py
python3 tests/verify.py --list
python3 tests/verify.py --json /tmp/lintel-verify/evidence.json
```

Daily GitHub CI selects frontend/UI, native/CLI, heavy work-package and Linux VM checks by changed paths; weekly and manual runs retain full cross-platform integration coverage. The local verifier without selection arguments still runs the complete synthetic default group. See the [verification guide (Chinese)](docs/verification.md) for groups, dependencies and evidence boundaries. Cloudflare Pages website updates remain separate; CI does not deploy or release the App.

- [Product target (Chinese)](docs/SPEC.md), [product baseline and 18 behavior checks (Chinese)](docs/specs/product-baseline-status.md), [86 full acceptance checks (Chinese)](docs/ACCEPTANCE.md)
- [Unified verification and independent runtime gates (Chinese)](docs/verification.md)
- [Current implementation state (Chinese)](docs/current-state.md), [desktop usage and visual conventions](docs/desktop.en.md), [visual identity and source assets (Chinese)](docs/visual-language.md)
- [Human operator guide](docs/operator-guide.en.md), [Agent CLI and installation](docs/agents.en.md)
- [Shared core / CLI](docs/core.en.md), [browser](docs/browser.en.md), [network](docs/network.en.md), [remote](docs/remote.en.md), [Linux service pause/resume](docs/services.en.md)
- [Current implementation architecture](docs/architecture.en.md), [protocol (Chinese)](contracts/protocol.md), [research basis (Chinese)](docs/RESEARCH.md)

### Source layout

`crates/core` owns plans and file operations, `apps/runner` provides the CLI,
`apps/desktop` is Tauri + React, `extensions/browser` holds the extension and
Native Messaging host, `crates/egress` is the controlled proxy,
`crates/operations` owns static operation schemas and strict field contracts,
`crates/remote` is the shared limited SSH controller for GUI and CLI, and the
desktop `src-tauri/src/remote.rs` only locates App resources and adapts native
commands.
`platform/ssh` preserves the existing Python callers' compatibility subset; new capabilities have one canonical shared Rust owner.

## Platform support

- **macOS desktop GUI**: Tauri app, currently built and checked as an unsigned
  arm64 candidate.
- **macOS arm64 CLI** and **Linux headless CLI** (x86_64 / aarch64): standalone
  CLI; Linux source is built and checked synthetically.
- Three-platform candidate package records exist, but that does not mean they
  are downloadable or that the Linux runtime fully passes. Only source code is
  public.

## Source, licensing and rights

Lintel is an independent tool, not affiliated with Anthropic. The Clawd
character belongs to Anthropic; the optional local font is not distributed
with the repository. Product name **Lintel**, repository currently named
**lintel-cc**. Source is available at
<https://github.com/IndelibleVivi/lintel-cc>. Only the **source code is
public**; a general reuse licence for the project's original code,
documentation and identity assets has not yet been chosen, and source
visibility does not grant a general reuse licence. Third-party dependencies
and the Clawd character retain their own rights.

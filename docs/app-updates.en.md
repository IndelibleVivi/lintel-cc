# Lintel App updates and release preparation

[简体中文 (canonical)](app-updates.md) · English · [Documentation](index.en.md)

This guide describes the macOS App updater in current source and the future release procedure. The public entrance remains **the 0.1.0 development preview, with no formal Release download, release public key or activated feed**. Source integration, synthetic tests, builds, installation, public Releases and production feeds are separate facts; see [current state (Chinese)](current-state.md) for exact evidence. CLI version installation retains the separate [Agent CLI](agents.en.md) procedure.

## Using the App

Open Settings and modules → Lintel version and updates. The version comes from native App package metadata. The build fixes the channel; Webview requests cannot select arbitrary URLs, public keys, downgrades or installer arguments.

1. **Check for updates.** A manual check accesses the fixed `https://lintel.page/updates/preview.json` or `stable.json`. An unconfigured build explicitly reports unavailable. Offline operation, absent feeds and invalid responses are errors, never “up to date.” A successful check establishes only that the channel has no higher version or has a candidate.
2. **Download and verify.** The official Tauri updater verifies downloaded bytes and signatures, then checks the version in the signature's trusted comment. A download-complete event is not verification success. The candidate freezes its version, signature, URL and this attempt's ID; checking again invalidates the old candidate.
3. **Review and install.** App replacement requires selecting and approving the current candidate. Native code excludes this App's running core, browser, SSH and network operations, and refuses installation while a controlled proxy remains active. Checking and downloading can proceed independently. The original intent is persisted before installation; unconfirmed intent publication prevents installation.
4. **Save your work, then restart.** Installation does not restart automatically. An explicit restart is accepted only after checking native activity and proxies again; save any handoff draft first. The updater does not kill external Claude sessions or detached remote tasks; query original tasks by their original IDs.

Background checks default off. When explicitly enabled, they run only while the App is open and visible, at most once an hour. Closing Settings does not stop a selected check; deselecting stops it immediately. They never automatically download, install or restart, and are not a menu-bar/system background service. Preferences reside only in the current user's App data; they do not initialize Claude/core state.

Requests expose the egress IP and ordinary HTTP information to the Lintel feed/GitHub host. They upload no Claude roots, accounts, sessions, environment inventory or device identifiers. They are managed separately from Claude Code telemetry settings. Package admission is 512 MiB, with a five-minute total download deadline and a 25-second total check deadline. The official updater parses check responses; notes/signature and candidate origin are bounded after a successful response. This is not a generic feed or arbitrary HTTP client.

## Uncertain results and version boundaries

Update records retain the original ID, starting/target versions and attempt result. On reopening, incomplete installation intents show interrupted; failures or unconfirmed record publication show uncertain, with no automatic replay. A further installation is refused while the old version is still running with an unconfirmed installation. Check the App location and version, and if needed repair manually using the official DMG. Observing the target or a higher running version can unblock future upgrades, but never rewrites the original interrupted record as completed.

When multiple App copies under one user share records, locking and readback comparison prevent an old copy from overwriting a newer intent. Activity/proxy checks cover only the current App process; they do not prove that other App copies, CLIs or external programs have stopped. Close other Lintel copies before installation. Damaged or externally edited records cause explicit refusal and remain available for inspection.

Cancelling an operation wait does not release a still-running blocking core check early. If a channel-stop wait is cancelled, the unfinished original channel continues to block installation; a later explicit stop continues to reconcile the same task.

SemVer comparisons use precedence only: `0.1.0-preview.1` is lower than `0.1.0`; `+build` does not increase precedence; an equal version is not an upgrade. A next preview can use `0.2.0-preview.1`, higher than the current version. Preview and stable feeds are separate; stable accepts no prereleases. Preview selection does not use GitHub `releases/latest`. Older installed Apps without the updater/public key need one manual upgrade.

## Local unsigned DMG and third-party notices

A DMG can still be built locally without Developer ID for retention and inspection. It remains a local development candidate; packaging does not establish Gatekeeper first-install acceptance or activate downloads/the updater. Prepare both Linux runners first if bundling their installation capability, then use clean source:

```sh
cd apps/desktop
npm run desktop:build
npm run tauri -- bundle --bundles dmg --ci --no-sign
```

The second command only packages the just-built native App; do not reuse an old executable after changing source. Default output is `src-tauri/target/release/bundle/dmg/`. Preserve each candidate in a new ignored directory with its source revision, architecture and actual checks, retaining previous packages. This neither replaces Applications nor bypasses Gatekeeper. Apple signing, notarization, real first installation, GitHub Release, website downloads and feed activation remain separate steps.

`build:desktop-resources` automatically runs `scripts/prepare-rust-notices.mjs` to collect target-relevant normal/build dependencies from three locked Cargo manifests offline, plus current Rust standard-library notices. It requires `rust-docs` and cached dependencies; missing text or exact upstream evidence refuses preparation. The collection includes build/proc-macro dependencies and is not an exact linked SBOM. `contracts/third-party-notices` retains version-bound supplements and sources. Reused runners require their original toolchain notices; current toolchain text is not proof of how an older binary was built.

Vite's built-in `build.license` generates frontend licence and copyright texts. The full build runs Vite before the notice generator validates and collects its output; missing or empty text refuses preparation. Native tests may use the resource directory containing only its tracked README, which is not a distribution. Both sets ship under `Contents/Resources/third-party-notices`; retain subdirectories and HTML relative links. Generated output stays out of Git. Third-party terms do not select a licence for Lintel original code, documentation or independent assets. Rights for third-party identity/assets such as Clawd still require separate confirmation.

## Candidate first-use acceptance

Use a Mac matching the candidate architecture and record the exact DMG, source revision, App version and macOS version. The following is a repeatable native procedure; see [current state](current-state.md) for the latest candidate results. Existing CLI or Chromium checks do not replace native acceptance.

1. **Identify the tested package.** Mount the local candidate read-only and copy the App into a new test directory, preserving the DMG and existing installations. Test actual download/Gatekeeper behavior separately on a clean Mac. Record the original system message and stop if blocked; do not remove quarantine or disable security checks. Website download acceptance first requires authorization to publish downloads, then verifies that the downloaded bytes match this accepted candidate.
2. **Prepare six synthetic originals.** In a separate child shell, follow [quickstart step 1](quickstart.en.md#1-open-a-separate-interactive-shell), creating only the fixture without continuing to CLI registration or submission. Retain the printed `ROOT`; never point at a real Claude root. Replace `APP` below with the absolute path of the copied candidate and launch through macOS LaunchServices in the background from the same child shell. Fully quit the test App before reopening with the same command and HOME/state:

   ```sh
   APP="/path/to/test-copy/Lintel.app"
   env -i PATH=/usr/bin:/bin:/usr/sbin:/sbin /usr/bin/open -g -n -a "$APP" \
     --env "HOME=$ROOT/home" --env "CFFIXED_USER_HOME=$ROOT/home" \
     --env "LINTEL_TEST_HOME=$ROOT/home" --env "LINTEL_STATE_DIR=$ROOT/state" \
     --env "LINTEL_BROWSER_STATE=$ROOT/browser-state" --env "TMPDIR=$ROOT"
   ```

   A successful `open` only confirms that the launch request was sent; separately observe the actual window and selected environment. `-g` does not activate the App. Do not run multiple copies against the same test state. These variables isolate the specified HOME/state for this trial; they are not an OS sandbox and do not replace first-install acceptance on a clean Mac. The quickstart resolves `ROOT` to a physical path. Preserve that form for output paths as well; do not substitute a shorter path containing a symlink.
3. **Register from an empty workspace.** Add an existing environment named `Synthetic native trial`, using the expanded absolute path of `$ROOT/claude-root` as its config directory. Register only; do not run Claude. Confirm that the selected host is this Mac and the environment is this fixture.
4. **Preview exact selection.** Open Preserve work, choose Archive only and enable individual-file selection. Keep only `CLAUDE.md`, `projects/example/memory/MEMORY.md` and `projects/example/session.jsonl`, deselecting `NOT_SELECTED.md`. Use the absolute path of `$ROOT/carried.age` for output. Both capacity preflight and the approval plan must name exactly three files. No ciphertext should exist at preview time; settings and placeholder credentials must be excluded.
5. **Approve once and query the original task.** Enter a passphrase used only for this synthetic trial and approve the current plan. Retain its original ID/hash. Wait for actual completion; acceptance is not completion. If uncertain, query the original ID without resubmission. The receipt must show archive-only with no new environment/root. Use the byte check in [quickstart step 6](quickstart.en.md#6-confirm-the-source-is-intact-and-no-root-was-created) to verify all six originals, including the unselected file.
6. **Quit, reopen and read.** Fully quit the App and relaunch with the same HOME/state. Check the original task in Records & recovery, then choose Standalone encrypted package in Sessions & data, entering the absolute ciphertext path and the passphrase again. The index must contain exactly the three selected files; instruction, memory and raw session text must be readable. Do not approve import, launch or cleanup. Separately verify reading in an independent HOME/state with [quickstart step 7](quickstart.en.md#7-inspect--read-in-a-separate-independent-state). Reopening the same state is not an independent installation.
7. **Record each result separately.** Record observed startup, registration, exact selection, preview, approval, original-task completion, quit/reopen, file-based package reading and six-file byte checks. Missing automation windows, failed input or screenshot errors mean unverified; retain the error without labeling the product successful or broken. Keep raw screenshots, fixture, state, passphrase and internal records local; never upload the entire candidate evidence directory.

## One record prepares three outputs

[`app-release.mjs`](../scripts/app-release.mjs) is the local release-preparation entry. It creates no keys, signs nothing, uploads nothing, creates no GitHub Release and deploys no Pages site. Its input is a local `lintel.app-release/1` record. Inputs containing actual artifact paths stay outside Git or in ignored directories; public outputs contain no local paths.

First choose the exact source revision, a strictly higher new version, channel, architecture and reviewed **public** update key. The following illustrates fields and is not real release data:

```json
{
  "schema": "lintel.app-release/1",
  "version": "0.2.0-preview.1",
  "previous_version": "0.1.0",
  "channel": "preview",
  "source_revision": "<40-character-source-commit>",
  "pubkey": "<base64-encoded-public-key-file-content>",
  "notes": "Public changes and compatibility notes for this release",
  "pub_date": "2026-10-09T00:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "archive_path": "/path/to/Lintel.app.tar.gz",
      "signature_path": "/path/to/Lintel.app.tar.gz.sig",
      "dmg_path": "/path/to/Lintel.dmg",
      "archive_name": "Lintel_0.2.0-preview.1_aarch64.app.tar.gz",
      "dmg_name": "Lintel_0.2.0-preview.1_aarch64.dmg"
    }
  }
}
```

`darwin-x86_64` is the separate Intel macOS target. The current inspector accepts single-architecture Mach-O; universal archives are outside this preparation path. Every public filename must contain the exact version; DMGs and updater archives are separate.

```sh
# Review build configuration first; the output directory must not exist.
node scripts/app-release.mjs configure \
  --record candidate-packages/app-release-input.json \
  --out candidate-packages/app-config

# Build from clean source with required static resources and toolchain prepared.
node scripts/app-release.mjs build \
  --record candidate-packages/app-release-input.json \
  --platform darwin-aarch64

# Verify actual built, signed files offline; outputs remain local candidates.
node scripts/app-release.mjs prepare \
  --record candidate-packages/app-release-input.json \
  --out candidate-packages/app-prepared
```

`build` checks that the current clean Git revision matches the declaration and that previous_version matches the current source App version. It fixes the Tauri target, `app`/`dmg`, `createUpdaterArtifacts`, public key and feed, and passes the same version to the frontend. Update-signing secrets use Tauri's supported local secret/environment mechanisms, never the input record, command line, source, frontend or public manifest. This command may invoke local signing tools and must run only under corresponding build/signing authorization.

`prepare` verifies the actual `.app.tar.gz` finite paths, entry types/capacity, Bundle ID, Info.plist version, Mach-O architecture of `lintel-desktop`, Minisign artifact/trusted-comment signatures and exact version, then computes SHA-256 for uploaded-byte comparison. It executes neither archive nor DMG. DMG byte checks do not prove installation works. source_revision is a declaration, not source authentication; Apple Developer ID/notarization remain unverified. Outputs include:

- `release.json`: public candidate record, platforms, first-download information and byte evidence, without local paths.
- `updates/preview.json` or `updates/stable.json`: a static Tauri feed candidate for that channel.
- `tauri-release.json` from configure: a reviewable build override. Formal builds should use the build entry above to keep frontend/native versions aligned.

Update signatures protect the package and version accepted by the updater. Apple Developer ID code signing and notarization form a separate layer of macOS installation trust. Non-developer distribution must independently complete [Apple notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution), without bypassing Gatekeeper.

### Manual download shape (no updater feed)

A Preview without in-app updates uses a `lintel.app-release-manual/1` record: same fields as the signed record, but **without** `pubkey`, `archive_path`, `signature_path` or `archive_name`; each platform declares only `dmg_path` and `dmg_name`. `configure`/`build` reject manual records (they pin the signed build configuration); `prepare` dispatches on the record schema and, for the manual shape, performs only the DMG frozen-identity and SHA-256 byte checks, emitting `release.json` (schema `lintel.app-release-manual-public/1`) and **no** `updates/` feed. `verify-public` dispatches the same way and checks only the public DMG bytes.

`prepare-site.mjs --app-manual <release.json>` requires verified public bytes and projects the download buttons plus the "no in-app updates" note, but writes no updater feed; `--app-release` and `--app-manual` cannot be combined — one site payload carries one download story. The manual shape does not disable or loosen any existing signature check; once signing is in place, the updater is enabled through the original `lintel.app-release/1` path.

```sh
node scripts/app-release.mjs prepare \
  --record candidate-packages/app-release-manual-input.json \
  --out candidate-packages/app-prepared-manual
```

## Activation order when release is authorized

These external actions are not part of the present preparation. With exact release/hosting authorization in future, first complete signing/notarization and candidate-installation acceptance, then create the matching GitHub Release and upload each architecture's DMG, `.app.tar.gz` and `.sig` under the public names in the record. Keep version/tag/filenames immutable; never reuse a released version.

After public artifact upload, verify the exact bytes first; **activate the feed last**:

```sh
node scripts/app-release.mjs verify-public \
  --record candidate-packages/app-prepared/release.json \
  --out candidate-packages/app-public-verified

node scripts/prepare-site.mjs \
  --media-dir candidate-packages/site-media \
  --app-release candidate-packages/app-public-verified/release.json \
  --out candidate-packages/site-with-app
```

`verify-public` reads only the HTTPS URLs for the fixed repository/tag in the record, compares actual byte counts and SHA-256, and writes another directory without changing old records. It requires large network downloads; reachability and identical bytes do not establish Apple signing, notarization or native use. `prepare-site` generates download links and feeds only from a record with verified public bytes. Binaries stay on GitHub, outside the Pages payload's 25 MiB file limit. Without a record, the site continues to show the source preview without a fake download.

If two channels are activated, **pass both `--app-release` records to the same payload**, retaining each original record so a whole-site update does not remove the other feed. First installation on the website prefers stable, otherwise preview. Version, download details and feed come from one record. `updates/*` and download metadata disable immutable caching. Follow [site-hosting (Chinese)](site-hosting.md) for preview/production/TLS/readback upload, then test checking, downloading, installing and original-task recovery from an older App on each channel. A source push activates none of this.

A feed failure leaves the current App in place. Signature/version mismatches refuse installation. For an incorrectly published candidate, stop new recommendations on that feed first, then publish a higher fixed version. Do not enable downgrades or replay old packages by changing their version number. Preserve earlier public files and original update records for manual recovery.

Browser extension/native host and remote runner installation are outside automatic App updates. A new App may carry new resources, but they still require their own frozen preview, explicit approval and original-installation reconciliation. Older remote tasks keep their frozen runner.

Protocol/API references: [Tauri updater](https://v2.tauri.app/plugin/updater/). Runtime behavior follows the pinned `tauri-plugin-updater`, the current CLI's `signer sign --app-version`, and repository checks. Source acceptance criteria are in the [telemetry and updates Leaf (Chinese)](specs/telemetry-updates.md).

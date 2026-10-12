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

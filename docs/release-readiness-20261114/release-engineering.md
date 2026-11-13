# 2026-11-14 Release Engineering Audit and Handoff

## Decision and scope

The next candidate is **Lintel 0.2.0 Preview**, with the same numeric product version for App and CLI. This is preparatory source metadata, not a published version. The public App 0.1.1 Preview and CLI 0.1.0 remain separate, older downloads. Keep the GitHub release prerelease flag and the Preview channel when a release is eventually authorized.

The repository has a strong foundation for a controlled Preview: explicit binary inputs, architecture admission, frozen runner projections, locked builds, exact-source candidate receipts, strict full-CI evidence checking, a bundle-level signature verifier and finite package contents. This audit found two concrete ways those release checks could still report success for a defective user-facing result. Both are fixed below. Native execution of the final archives and a copied final-DMG App are now executable acceptance procedures, separate from canonical model tests.

The default macOS App distribution is still **ad-hoc signed, manually downloaded Preview**. It is not Developer ID distribution, notarized distribution, or proven first-download installation. The updater remains unconfigured. These are different release decisions from whether the CLI's portable archives work. Do not hold a proven CLI Preview indefinitely for a future App updater, but do not describe an ad-hoc App as an ordinary frictionless download.

This lane reviewed `scripts/prepare-candidate.py`, `scripts/package-cli.mjs`, `scripts/app-release.mjs`, `scripts/inspect-app-archive.py`, the desktop build/resource/signature entries, packaging tests, updater configuration, current-state/verification/app-updates documentation, project licence boundaries and the relevant CI build paths. The initial inspected source was `73f34ed06b5d630d2a36910baf88ff5713aede66`. The final source SHA and actual workflow outcomes belong to the coordinator's exact-revision evidence record; this document does not relabel an earlier successful run as verification of later edits.

## Confirmed defects and fixes

### RE-01 — A consistent CLI archive could be impossible to execute

**Location:** `scripts/prepare-candidate.py::read_tar_members`; `tests/candidate_gate_test.py`.

The original candidate verifier compared actual member bytes with recorded bytes, but ignored tar member permissions. A package with `bin/lintel` mode `0644` had exactly the same executable byte hash as the build receipt, so it passed the full consistency check even though normal extraction produced a non-executable CLI. The same omission applied to the packaged Linux runners. An input that asks the installer to create privileged or writable executables should not be accepted merely because its bytes are recorded.

This was reproduced against the actual baseline verifier, not only inferred from its code. Using an otherwise accepted complete synthetic matrix and full canonical CI fixture, changing only the macOS archive's CLI mode to `0644` yielded `consistency_passed: true` at the baseline. The patched verifier rejects it with `CLI executable has no owner executable mode`.

The verifier now requires owner-readable regular files, executable owner mode for the three finite executable paths, no setuid/setgid/sticky bits, and no group/other write permission. Controlled extraction preserves the checked mode. The regression also covers a non-executable remote runner, setuid executable and world-writable text member. This closes a release admission hole; it is not evidence that the previously public archives had these defective modes. The actual old public Linux x86_64 archive passes the new mode check.

### RE-02 — Broken installation checksums could pass the candidate gate

**Location:** `scripts/prepare-candidate.py::read_tar_members`; `tests/candidate_gate_test.py`.

`SHA256SUMS` was formerly treated as another opaque file. A receipt and tar archive that consistently recorded the literal text `synthetic sums` passed the gate. The baseline's own accepted fixture used that content. A user following the bundled verification instructions would nevertheless fail before running Lintel.

The explicit before/after experiment confirms baseline success and patched rejection for the malformed file. The gate now requires a bounded UTF-8 checksum file containing unique safe relative paths, exact lowercase SHA-256 digests and a complete mapping of every regular package file except `SHA256SUMS` itself. Missing, extra, duplicate, self-referential, malformed or wrong-digest entries fail. Tests preserve otherwise consistent outer archive/receipt metadata so the regression distinguishes this defect from the already-covered stale-byte case.

**Impact of RE-01/02:** a defective build/package could receive false release readiness evidence. They did not establish attacker-controlled code execution or a defect in the actual public release. Their practical importance is that installation success must be checked independently from a self-consistent receipt.

The machine-readable before/after result retained in the audit evidence is `candidate-gate-before-after.json`, bound to the baseline SHA above. Unit fixtures are synthetic and never executed.

### RE-03 — An explicit App release version override did not reach the frontend

**Location:** `scripts/app-release.mjs::build`, new `releaseBuildEnvironment`; `apps/desktop/vite.config.ts`; `tests/app_release.test.mjs`.

The release builder set Tauri's version through its generated configuration, while Vite read the source `tauri.conf.json` unless `LINTEL_APP_VERSION` was set. The builder did not set that environment variable. Building an explicit new version from the allowed previous-version source could therefore produce a correctly versioned native bundle whose App information showed the previous version.

`releaseBuildEnvironment` now supplies the exact release version to Vite and preserves the caller's explicitly supplied signing environment. It accepts a source already committed at the target version or the established previous-version source override; unrelated source versions fail. Tests check both paths, preservation of the input environment, actual canonical spawn wiring, and refusal of an unrelated version. The paired app-updates documents now describe the actual behavior.

### RE-04 — Preview suffixes are inappropriate for the current macOS plist projection

**Location:** App/CLI package metadata; `scripts/prepare-candidate.py::require_app_version`; final-DMG acceptance.

During candidate preparation, the provisional `0.2.0-preview.1` was checked against the pinned Tauri CLI 2.12.1 implementation. That implementation copies its version verbatim into `CFBundleShortVersionString` and, without a separate override, `CFBundleVersion`. Apple's current documentation requires three numeric components for the short version, and one to three for the build version. A valid Cargo/SemVer prerelease string is therefore not sufficient justification for these macOS fields.

The coordinator selected numeric `0.2.0` for App and CLI, retaining Preview as the distribution channel. This avoids introducing a new version projection into bundle/updater semantics. The candidate builder and mounted-DMG verifier now require both plist versions to equal the numeric frozen source version. The actual formal release build also rejects a nonnumeric version before invoking Tauri; SemVer comparison and historic feed-record support remain intact. Tests reject a suffix, a stale build version and a missing build version. The native App report records both verified values.

This was addressed before distributing the candidate; it is not a claim that the operating system would necessarily refuse every malformed local App. The release requirement is to produce documented metadata and inspect the actual bundle.

## Existing protections independently retained

`package-cli.mjs` requires all three actual binary inputs and both packaged remote runners. Mach-O checks include CPU type, macOS platform commands, loadable executable entry and bounded command structure. ELF checks reject the wrong architecture, missing loaded entry, interpreter or dynamic dependencies. Both Linux CLI binaries must equal their canonical runner bytes. These checks are materially stronger than accepting a magic header or trusting a filename. They remain static admission; they do not establish that a Linux executable ran on its target CPU.

The candidate builder requires a clean exact Git SHA, creates a fresh clone with no local object hardlinks, owns a fresh target directory, runs locked builds, and hashes component files and final packages. It rejects absent toolchain prerequisites before creating output. It preserves separate App and CLI runner version projections. The gate recomputes the full canonical CI plan; renaming a reduced plan or substituting a different head does not suffice. The final DMG is mounted read-only and its App inventory compared to the canonical App. The artifact is copied into a private temporary location before mount and digest-checked, reducing source replacement exposure.

The App build entry signs the fully assembled bundle and verifies `Info.plist` plus sealed resources with `codesign --verify --strict`. A linker-only Mach-O signature is not accepted as a bundle signature. The added installed-copy path verifies the copy again after it is made from the mounted DMG.

Updater preparation verifies the Ed25519/Minisign prehash signature and authenticated trusted comment, binds the signed version, inspects the actual updater archive, restricts public URLs to the selected repository/tag, and checks published byte counts and digests before generating download metadata. A digest is used as a byte identity check, not as independent publisher authentication. The current empty public key/endpoints intentionally mean no updater is active.

Packages include the project terms separately from third-party notices. Rust notices are built from locked dependency graphs and reviewed supplements; standard-library/native runtime terms must match the selected toolchain. This is a notice inventory, not a claim of an exact linked SBOM. Original-code and documentation licences do not grant rights to excluded third-party marks/assets. The separate Clawd/Anthropic permission question must be resolved before a package containing that asset is publicly distributed; notice completeness cannot answer it.

## New final-package native acceptance

### Workflow and toolchain

`.github/workflows/candidate-readiness.yml` runs on the audit branch or an explicit workflow dispatch. It has read-only repository permissions and uploads only finite **unpublished** packages and evidence. It does not create a release, sign with production keys, deploy a website, activate an updater or change any account.

The macOS arm64 job runs the existing canonical builder against `$GITHUB_SHA`. Prerequisites are Rust 1.100.0 with all three targets and rust-docs, Node 22, Python 3.12, cargo-zigbuild 0.23.4 and Zig 0.16.0. The Zig pair is the one recorded in `docs/current-state.md` for the earlier real runner build, and matches the existing canonical candidate preflight and native-runtime notice assumptions. Dependencies remain lockfile controlled. The job uploads the receipt, three CLI archives/index, DMG and native reports for 14 days. Preserve the accepted package and workflow provenance beyond that retention before any future publication decision.

The Linux jobs consume the exact uploaded archives on `ubuntu-24.04` and `ubuntu-24.04-arm`. They do not rebuild or repackage them. The arm job is a native CPU check, distinct from cross-compilation. The full product CI remains the separate `verify.yml` workflow; this acceptance workflow does not assert its success.

### CLI acceptance

`scripts/accept-cli-candidate.py` checks the selected artifact digest against the exact build receipt before safe extraction, checks target/identity/complete declared file inventory/runner metadata and invokes the resulting native executable from a disposable directory with an isolated HOME/state. No personal credential environment is inherited. The binary is not allowed to depend on its source checkout.

It checks `version --json`, capabilities and a command schema; `version` must not create state. A digest-pinned real public CLI 0.1.0 creates the old synthetic environment, a completed policy job and an encrypted archive. The new binary must retain the original job IDs, plan reference and archive receipt. Repeating the accepted original archive request must return the original task, preserve the actual ciphertext digest and preserve the archive directory inventory. This detects a deceptive response that returns the old ID while actually executing the archive again.

An independent fresh HOME/state then inspects and reads the old encrypted archive, imports selected instructions and sessions, compares their exact bytes, and confirms the originals remain unchanged. The old executable's digest is checked again so manual selection of the prior installation remains possible. The report explicitly says whether a distinct old version was tested. Old-version writes against newly changed state, in-flight old App replacement, interrupted OS installation, remote-host deployment and production browser/authentication behavior are outside this experiment.

The pinned old public archives are:

| Platform | SHA-256 |
| --- | --- |
| macos-arm64 | `83b6fe24751cc6a1c8617245913da0a8515914c9f0933fb814b540fa83a162e1` |
| linux-x86_64 | `5d780c50452022efefb62bb18d6ab491b0e2b414351fc215ae30702595972ce6` |
| linux-aarch64 | `299291030c91d52741b8083e614fab15cbd10c50b165e35b272ef0d34d6c6fc3` |

They use `https://github.com/IndelibleVivi/lintel-cc/releases/download/v0.1.0/lintel-cli-0.1.0-candidate-fcae6cb1efb5-<platform>.tar.gz`. The coordinator compared authenticated GitHub asset metadata digests with the release checksum file. The x86_64 archive was also downloaded in this audit and independently inspected: 9,286,789 bytes, version 0.1.0, source `fcae6cb1efb50c6384a31ddce161bee6a39646e0`, 22 declared package files, notices present, executable permissions and internal sums valid.

### macOS App acceptance

`scripts/accept-macos-candidate.py` consumes the exact final DMG, verifies its mounted App, copies it into a fresh temporary installation, compares its complete inventory and reruns strict native bundle verification. It records read-only Gatekeeper assessment, stapled notarization ticket validation and the presence/absence of quarantine. Rejection is retained as evidence. The script neither removes quarantine nor creates an exception.

It directly launches the installed native executable in a synthetic HOME, observes five seconds of process liveness, and terminates only that spawned process. The report explicitly marks the visible window as unobserved and ordinary download installation as unaccepted. A live process is not proof that a WebKit screen rendered, a user completed a task, VoiceOver worked, LaunchServices accepted the App, or Gatekeeper accepted a newly downloaded file. A dead process fails this check even if its signature passed.

## Evidence boundaries and remaining release conditions

| Layer | Evidence and treatment |
| --- | --- |
| Source | Actual baseline reviewed; release scripts and regressions changed; numeric 0.2.0 metadata prepared. |
| Synthetic release tests | Candidate/acceptance regression suite passes 23 tests; baseline false-positive reproductions retained. App release tests include frontend/native version binding; generated-site tests require regenerated paired docs. |
| Local native runtime | Linux x86_64 debug runner built. The synthetic receipt/package trial used the actual previous public executable and current debug executable. It reached current-binary old-state queries, but a later encrypted-archive request exceeded its 180-second timeout; it did not pass the complete acceptance procedure. No measured cause is assigned. This trial is not RC provenance; the optimized final archives must resolve the remaining acceptance result. |
| Canonical full CI | Coordinator owns the exact run/plan/results/evidence aggregation. An earlier PR7 full success is valuable baseline evidence but cannot certify this branch. |
| Final package/native | New exact-source workflow and three native acceptance procedures are prepared. Use the reports from the selected commit; a workflow file alone is not a passing result. |
| Installed App | Temporary DMG copy, strict signature and process result are narrower than a Finder/Applications install and actual GUI use. |
| Published/deployed | No new release, feed or site deployment authorized or performed. The coordinator records observed public downloads/site separately. |

For **CLI Preview**, require the final exact-source artifact native reports on all claimed architectures, the exact-head full canonical CI, retained notices/terms, and truthful manual installation instructions. An individual platform may remain experimental or be held if its runtime report fails; another platform's success cannot stand in for it.

For an **ad-hoc App Preview**, require the final DMG/copy/native reports and a clearly disclosed manual trust path, plus native visible-window/task acceptance on the selected candidate. This is a legitimate limited distribution choice if the owner explicitly wants it. It is not the release mode implied by “ordinary users download, install and trust without researching system exceptions.”

For **general macOS distribution**, require owner-managed Developer ID credentials, hardened runtime, successful notarization and ticket handling, followed by download/install/first-launch and upgrade acceptance on a controlled Mac. Apple Developer ID signing and Tauri updater signing serve different purposes; one cannot substitute for the other. No new signing identity or private key was created here.

For **automatic updates**, choose and protect the production update signing key, commit/embed its reviewed public key, produce a version-bound signed updater archive, retain a manual recovery download, and test from the real old installed App through interruption and original-task recovery before activating a channel. The public Preview with no configured key/feed cannot magically discover a new updater just because a later build supports it. A documented manual upgrade is a valid current route.

## Commands for the next maintainer

On a controlled native macOS arm64 machine with the listed prerequisites, select and verify the actual clean commit, then run the existing canonical entry:

```sh
python3 scripts/prepare-candidate.py build --revision <exact-40-character-SHA> --out ../lintel-candidate --app
python3 scripts/prepare-candidate.py verify --receipt ../lintel-candidate/build-receipt.json --ci-plan ../lintel-ci/plan.json --ci-results ../lintel-ci/results.json --ci-evidence ../lintel-ci/evidence --out ../lintel-candidate-verified.json
python3 scripts/accept-macos-candidate.py --receipt ../lintel-candidate/build-receipt.json --revision <same-SHA> --out ../native-app.json
```

On each target's native OS/CPU, consume the same receipt/artifacts and the pinned previous archive; do not substitute a local rebuilt executable:

```sh
python3 scripts/accept-cli-candidate.py --receipt ../lintel-candidate/build-receipt.json --revision <same-SHA> --target <platform> --previous-archive ../previous-cli.tar.gz --previous-sha256 <pinned-platform-digest> --out ../native-cli.json
```

The workflow already performs these build/native steps except the separate exact-head full-CI aggregation. Candidate and acceptance output paths must be absent. Failure means inspect the existing original report/log and correct the specific cause; do not relabel, edit reports to pass, retry a completed state-changing task, or overwrite a previously identified package.

For local regressions:

```sh
python3 tests/candidate_gate_test.py
node --test tests/app_release.test.mjs
```

App-updates documentation changes require the repository's normal bilingual site generator before the latter command. Generating tracked pages is not website deployment.

## Primary references checked

- [Pinned Tauri CLI 2.12.1 macOS bundler source](https://raw.githubusercontent.com/tauri-apps/tauri/tauri-cli-v2.12.1/crates/tauri-bundler/src/bundle/macos/app.rs): exact plist version projection, rather than assuming every SemVer is an Apple bundle version.
- [Apple CFBundleShortVersionString](https://developer.apple.com/documentation/bundleresources/information-property-list/cfbundleshortversionstring.md) and [CFBundleVersion](https://developer.apple.com/documentation/BundleResources/Information-Property-List/CFBundleVersion): documented numeric fields.
- [Apple notarizing macOS software before distribution](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution) and [Developer ID](https://developer.apple.com/developer-id/): Developer ID/hardened runtime/notarization requirements and their distribution role.
- [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/): local ad-hoc signing versus Developer ID distribution.
- [Tauri updater](https://v2.tauri.app/plugin/updater/) and [CLI changelog](https://v2.tauri.app/release/%40tauri-apps/cli/all-versions/): signed updater artifacts and authenticated version handling.
- [GitHub-hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners) and [runner images](https://github.com/actions/runner-images/blob/main/README.md): native macOS arm64 and Linux arm64 labels. A changing runner image still needs the actual recorded run; its label alone is not evidence of a passing package.

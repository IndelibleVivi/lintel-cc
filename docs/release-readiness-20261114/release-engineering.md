# 2026-11-14 Release Engineering Audit and Handoff

## Decision and scope

The next candidate is **Lintel 0.2.0 Preview**, with the same numeric product version for App and CLI. This is preparatory source metadata, not a published version. The public App 0.1.1 Preview and CLI 0.1.0 remain separate, older downloads. Keep the GitHub release prerelease flag and the Preview channel when a release is eventually authorized.

The repository has a strong foundation for a controlled Preview: explicit binary inputs, architecture admission, frozen runner projections, locked builds, exact-source candidate receipts, strict full-CI evidence checking, a bundle-level signature verifier and finite package contents. This audit found two concrete ways those release checks could still report success for a defective user-facing result. Both are fixed below. Native execution of the final archives and a copied final-DMG App are now executable acceptance procedures, separate from canonical model tests.

The default macOS App distribution is still **ad-hoc signed, manually downloaded Preview**. It is not Developer ID distribution, notarized distribution, or proven first-download installation. The updater remains unconfigured. These are different release decisions from whether the CLI's portable archives work. Do not hold a proven CLI Preview indefinitely for a future App updater, but do not describe an ad-hoc App as an ordinary frictionless download.

This lane reviewed `scripts/prepare-candidate.py`, `scripts/package-cli.mjs`, `scripts/app-release.mjs`, `scripts/inspect-app-archive.py`, the desktop build/resource/signature entries, packaging tests, updater configuration, current-state/verification/app-updates documentation, project licence boundaries and the relevant CI build paths. The initial inspected source was `73f34ed06b5d630d2a36910baf88ff5713aede66`. The final source SHA and actual workflow outcomes belong to the coordinator's exact-revision evidence record; this document does not relabel an earlier successful run as verification of later edits.

## Final candidate closure — source 353536cc

The final product source is **`353536cc7dc3c8c63baa628955ee66332d95bcbb`**, tree `7c12bee277e84c85052d146738bbb395bd9b25b7`. The [exact-head push canonical run 43526964668](https://github.com/IndelibleVivi/lintel-cc/actions/runs/43526964668) has all 14 jobs successful, 12 canonical reports and all 120 platform-counted selected check executions passed; root independently reread the evidence and reran the production aggregate with no problems. The [final candidate run 43526964671](https://github.com/IndelibleVivi/lintel-cc/actions/runs/43526964671) built the packages afresh, completed the actual three native old-public-version upgrade consumers, and produced the final mandatory exact-source canonical/DMG gate: passed, including the mandatory macOS production verification of the actual DMG and the same-source full canonical CI.

| Final package | Bytes | SHA-256 |
| --- | ---: | --- |
| macOS arm64 CLI | 9,556,992 | `ac62ded7cdbe377d517068ffb55d4db5db0f69245e5177fe45e1c01733579a9e` |
| Linux x86_64 CLI | 9,807,758 | `a611f5ec8a8067f0c983a93af6756827bdb20aa31b8f786b0a2bd3795f04d98b` |
| Linux aarch64 CLI | 9,542,757 | `6ac7aa1308c485dbabb89305eba126ff8e8ed9f210c622e29f0bc55abbf77793` |
| macOS arm64 DMG | 13,753,858 | `9865fc72ee05d01b1fb579b5333b579772319debe18efe9198a136c1c86bbf18` |

The unchanged final GitHub artifact **14085474075** ZIP is 45,199,043 bytes, SHA-256 `ebd9f53edae5b7cdf88bf0acab0f941c7d2b6ace7dc6891f07ec0cafa2b52230`. Raw metadata, logs, receipt, actual packages, native reports and local independent verification are under `evidence/rc-final-candidate-ci`. Each CLI archive's 28 regular members, executable/safe modes and complete internal checksums were reread; three `upgrade_tested:true` reports bind the same bytes to native Darwin arm64, Linux x86_64 and Linux aarch64. The old 0.1.0 public binaries are distinct, digest-pinned inputs, not current-build aliases.

The final DMG contains a 64-file / 39,934,188-byte App. Both plist fields are 0.2.0; mounted contents, copied contents and strict ad-hoc signature checks passed. Actual Gatekeeper **exit 3 rejection**, stapler **exit 65/no ticket**, and absent quarantine remain release-significant observations. Native runtime is macOS 15 / Darwin 24.6.0 arm64, separate from the final source tests on macOS 26.7.2. Neither accepts macOS 12, the declared minimum.

The selected final WebKit window additionally passed **11 finite AX actions and nine resulting-state assertions**, with ten target-window images. The observer and root independently checked the recorded theme/language state changes, history, missing-environment work guidance, task chooser and restored state. At the observed 1024×681 window the main actions were readable without overlap; this is not a native resize matrix, keyboard/VoiceOver audit, complete archive/cleanup/SSH journey or App upgrade. The two existing prepared-resource desktop fixtures were still ignored in ordinary native tests; exact-DMG-resource consumer commands remain in the [handoff](handoff.md).

The final manual App record was produced by the real `app-release.mjs prepare` against the exact DMG. It remains `prepared_local` / `publication_verified:false`, with Apple identity/notarization unverified and no updater feed. A separate old-public 0.1.1 record was rebuilt and actually passed `verify-public`; that can support a documentation-only site preparation without pretending 0.2.0 is public. The complete real-media site payload still requires the reviewed MP4/poster that could not be retrieved in the current environment; 42 generated pages and final site-ui success are narrower evidence.

**Release decisions:** Linux CLI Preview GO; macOS CLI Preview conditional on first-download/trust handling; general-user App NO-GO; limited technical App Preview conditional on its explicit trust path, complete final native task/upgrade and applicable graphic redistribution rights. Automatic updates may remain disabled. See the [handoff](handoff.md) for the exact remaining programs rather than rebuilding or broadening tests without a concrete reason.

The following preliminary section intentionally preserves the earlier candidate and failure chronology. Its hashes are not in the final installation manifest.

### Preliminary exact-source native result

[Candidate run 43520398419](https://github.com/IndelibleVivi/lintel-cc/actions/runs/43520398419) completed successfully against **`e573ce228e15ee6cb03c2705baf70a8bab1a8c75`**. It produced a fresh App/DMG and all three CLI archives; each platform's actual final CLI archive passed native execution and the real public 0.1.0 upgrade/archive experiment. Linux aarch64 ran on an aarch64 Linux host, not an emulator. The earlier local debug timeout is retained as a distinct unsuccessful experiment; the actual optimized archives passed without requiring the acceptance limit to be relaxed.

| First intermediate artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| macOS arm64 CLI | 9,546,898 | `78ad2bb666c2e9ad61f3e6e211242a4868e2162a480e24cb65d5c4e7426650b8` |
| Linux x86_64 CLI | 9,805,835 | `3fadc7214383918e9c10af30d945275383a6ac52339c15f55c395088a8615fb3` |
| Linux aarch64 CLI | 9,541,623 | `0cb141913ebe4575a2d6cad6a7fa30cca1c259819c3b693c3b7076061278a259` |
| macOS arm64 DMG | 13,756,590 | `592989724b31cfc09397bfb4081a05c9b84b031da76721230dd0328a06865a42` |

The candidate ZIP is GitHub artifact **14083043095**, 42,564,088 bytes, SHA-256 `720f659b3ad1f1f36e6139a1923ebdcf3a93fbd63f96076845ca631a30f4045f`. Linux runtime report artifacts are 14083151594 (x86_64) and 14084205040 (aarch64). All three ZIPs were downloaded through the connector and checked against GitHub's authenticated artifact metadata digests. The downloaded CLI archives were independently re-read locally, checking actual bytes, executable modes and internal sums against the build receipt. Full job logs, raw run/job/artifact metadata, exact ZIPs and extracted reports are retained under the audit's `evidence/candidate-ci` directory.

The actual DMG App copy contained **64 files / 39,919,470 bytes**, passed strict bundle verification, and reported both plist versions as 0.2.0. Its signature is **ad-hoc**. Direct native launch remained alive after five seconds. Gatekeeper assessment **rejected it (exit 3)**, stapler reported **no ticket (exit 65)**, and the extracted copy had no quarantine attribute. These observations support the limited Preview build/runtime conclusion; they do not establish ordinary first-download trust. The macOS CLI also ran from an unquarantined extracted package: its browser-download/Gatekeeper acceptance remains separate from native CLI functionality.

**This is a preliminary candidate, not the final fully admitted release candidate.** The separate exact-head canonical full CI encountered Mac PTY-test and Linux component-test harness failures. Those harness causes were corrected and the next da7ebd2 source passed their paths; its unrelated numeric-port assertion and the subsequently found TLS dependency issue required the final 353536cc source. Neither this successful package workflow nor its three native results overrides the failed full-CI condition at `e573ce2`.

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

The macOS arm64 job runs the existing canonical builder against `$GITHUB_SHA`. Prerequisites are Rust 1.100.0 with all three targets and rust-docs, Node 22, Python 3.12, cargo-zigbuild 0.23.4 and Zig 0.16.0. The Zig pair is the one recorded in `docs/current-state.md` for the earlier real runner build, and matches the existing canonical candidate preflight and native-runtime notice assumptions. Dependencies remain lockfile controlled. The job uploads the receipt, three CLI archives/index, DMG, native reports and any finite optional window evidence for 14 days. Preserve the accepted package and workflow provenance beyond that retention before any future publication decision.

The Linux jobs consume the exact uploaded archives on `ubuntu-24.04` and `ubuntu-24.04-arm`. They do not rebuild or repackage them. The arm job is a native CPU check, distinct from cross-compilation. The full product CI remains the separate `verify.yml` workflow; this acceptance workflow does not assert its success.

A mandatory final `gate` job on native macOS waits up to 40 minutes for the **same-SHA push** canonical verification run. `scripts/collect-candidate-ci.py` reads authenticated GitHub metadata using the official `gh` client, rejects wrong-head/PR/ambiguous/missing/failed runs, and checks each actual matrix sibling's name, run, SHA and successful conclusion. It obtains `ci-plan` and finite evidence artifacts using `gh run download`; it does not hand-forward authentication headers through redirects. It then invokes the actual production `prepare-candidate.py verify` from a clean checkout of the candidate SHA, including native DMG readback. The gate does not relabel a reduced plan, synthesize missing successes or weaken the canonical check set. Raw run/job/artifact metadata, actual grouped results, downloaded reports, final gate result and a separate API provenance observation are uploaded. Existing consistency-gate origin flags remain unchanged; the authenticated API observation is retained separately.

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

The final workflow also runs `scripts/observe-macos-candidate.py` with the tiny `scripts/macos-window-observer.swift` helper as a **separate optional observation**. It checks the same exact DMG/copy, observes public CoreGraphics window metadata filtered to the spawned PID, records existing screen-capture authorization through `CGPreflightScreenCaptureAccess`, and uses read-only AX queries only when already trusted. It requests no permissions, changes no privacy settings and modifies no candidate bundle. After a render allowance it refreshes window ownership; only an existing authorized Lintel window may be captured. The report distinguishes a visible normal window, a captured selected window, absent authorization, unavailable GUI session and early exit. The read-only observer remains separate from `macos-ax-action.swift`, which can use only exact role/title allowlisted controls in the launched App: theme, history, the new-user work target prompt, task chooser and language. The finite experiment requires a resulting AX state, records unsupported boundaries and retains selected-window captures; a successful AXPress alone is not a passed interaction. It does not execute an archive, cleanup, approval, SSH task or upgrade. Only this supplemental observation can continue on error; package/runtime/full-CI gates remain mandatory. Actual per-control outcomes belong to the final run, rather than to the existence of this program.

## Evidence boundaries and remaining release conditions

| Layer | Evidence and treatment |
| --- | --- |
| Source | Actual baseline reviewed; release scripts and regressions changed; numeric 0.2.0 metadata prepared. |
| Synthetic release tests | Candidate/acceptance/CI-collection regression suite passes 25 tests; baseline false-positive reproductions retained. Collector tests reject cross-head, PR, missing, duplicate, skipped, cancelled and failed matrix evidence. App release tests include frontend/native version binding; generated-site tests require regenerated paired docs. |
| Local native runtime | Linux x86_64 debug runner built. The synthetic receipt/package trial used the actual previous public executable and current debug executable. It reached current-binary old-state queries, but a later encrypted-archive request exceeded its 180-second timeout; it did not pass the complete acceptance procedure. No measured cause is assigned. This trial is not RC provenance; the optimized final archives must resolve the remaining acceptance result. |
| Canonical full CI | Coordinator owns the exact run/plan/results/evidence aggregation. An earlier PR7 full success is valuable baseline evidence but cannot certify this branch. |
| Final package/native | Final 353536cc archives passed all three native old-public-version upgrade consumers. Mandatory full-CI binding and final-DMG result are recorded in the final closure above; preliminary e573ce2 is historical only. |
| Installed App | Final 353536cc DMG copy passed strict signature/process checks and finite native WebKit/AX controls; Gatekeeper rejected it and no ticket existed. Complete tasks, Finder/Applications install and old-App upgrade remain separate. |
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

The final workflow performs these build/native steps and the exact-head full-CI production gate. Candidate and acceptance output paths must be absent. Failure means inspect the existing original report/log and correct the specific cause; do not relabel, edit reports to pass, retry a completed state-changing task, or overwrite a previously identified package. For a controlled manual run with authenticated read-only `gh` access, the same collector is executable directly:

```sh
python3 scripts/collect-candidate-ci.py --receipt ../lintel-candidate/build-receipt.json --revision <same-SHA> --out ../lintel-candidate-ci-gate --wait-seconds 2400
```

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
- [Apple window metadata API](https://developer.apple.com/documentation/coregraphics/cgwindowlistcopywindowinfo%28_%3A_%3A%29), [AX trust query](https://developer.apple.com/documentation/applicationservices/1460720-axisprocesstrusted) and [Apple DTS clarification of screen-capture preflight](https://developer.apple.com/forums/thread/839069?answerId=898801022): the optional native observer queries existing permission and does not infer it from missing window names.

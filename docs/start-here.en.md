# First use of Lintel

[简体中文](start-here.md) · English · [Documentation](index.en.md)

Begin with one small task: **select two synthetic files, create an encrypted work package and open it again.** This shows targets, previews and results without configuring your own Claude installation or browser.

## Choose an entry point

| What you have | How to start |
| --- | --- |
| Source code, and you want to explore the UI | Follow the synthetic interface trial below |
| A macOS app built by you or a trusted provider | Open it and register a test configuration directory you explicitly chose. Verify candidate source/version against [current status (Chinese)](current-state.md) |
| Linux, or a CLI-only workflow | Use [Agent CLI](agents.en.md) for installation and the [complete CLI trial](quickstart.en.md) for a synthetic example |

This remains 0.1.0 Preview, without a public Release download, formal signing or activated update feed. The latest local macOS arm64 release-preparation candidate is the 10-19 `e822d74` build, including current client controls and UI updates. Plain `npm run dev` starts only the frontend and does not provide the execution channel below.

## Synthetic interface trial

Prepare Git, Rust stable and Node.js/npm compatible with the repository dependencies:

```sh
git clone https://github.com/IndelibleVivi/lintel-cc.git
cd lintel-cc
cargo build -p lintel-runner --locked
cd apps/desktop
npm ci
npm run dev:synthetic
```

Open the URL printed by the terminal and confirm the interface identifies a **test space**. This starts Vite fixture mode with the real CLI and a fresh HOME/state. The script creates `CLAUDE.md`, `projects/synthetic/memory/notes.md` and synthetic settings. It prints the temporary directory path.

This is a browser-based development UI, not evidence of native WebKit, real authentication or production browser integration. Your personal login and browser profile are outside this trial path. The `e822d74` native app completed exact three-file archiving, quit/reopen, original-job lookup and independent-file reading in another isolated HOME/state. A separate CLI state verified all three originals byte for byte. That still does not establish personal authentication or download/install on a clean Mac.

### Create an archive only

1. Select the synthetic environment, open Work preservation and choose the archive-only outcome.
2. Select the synthetic instructions and memory. Review the file list, count and archive destination. This task requires neither cleanup nor a new configuration root.
3. Generate the preview. Review the host, complete root, two selected originals and destination. Enter a temporary archive passphrase of at least 12 characters, then approve the reviewed task.
4. Inspect the original task result and wait for the archive step to finish. Keep its ID and the package location, then unlock the package with the same passphrase and read the selected material.

**Completion means:** the original task says `completed`, its archive step succeeded, the package opens with the selected instructions and memory, the originals remain, and archive-only did not create a new environment. `accepted` means the task was received; you still need its completion result.

For independent-state reading, exclusion of unselected files and byte-for-byte preservation checks, continue with the [complete CLI trial](quickstart.en.md). That longer page retains the full verification procedure.

### End the trial

Press `Ctrl-C` in the terminal running Vite. The synthetic directory remains for inspection. After confirming it is no longer needed and no related process uses it, remove only the temporary directory printed by this run. Lintel cannot recover the contents of an encrypted package without its passphrase.

<a id="build-native"></a>

## Build the native macOS app

On macOS, prepare Rust, Node.js/npm and Xcode Command Line Tools. The build also needs `rust-docs` for the current Rust toolchain and locked dependencies to generate third-party notices. After `npm ci`, run inside `apps/desktop`:

```sh
mkdir -p src-tauri/runner-bundles
npm run desktop:build
```

An empty `runner-bundles` directory permits a local app build without bundled Linux runners; the remote installer reports missing resources. To include them, follow [Remote](remote.en.md) to prepare both static Linux runners first.

The app is produced at:

```text
apps/desktop/src-tauri/target/release/bundle/macos/Lintel.app
```

The build prepares the native browser host, Chromium/Firefox extensions and third-party notices. It does not install into Applications, perform Developer ID signing/notarization or publish a release. Only a native build matching the Rust host is currently supported. See [Desktop](desktop.en.md) and [App updates](app-updates.en.md).

A local build uses the source you checked out. Matching version numbers do not establish identical files: check the source revision, bundled resources and actual operation coverage. [Current state](current-state.md) keeps native, Chromium and CLI evidence for the current and older candidates separate.

## Connect your own environment

Use Add existing environment to register an explicit Claude Code configuration directory. Registration records its location without changing its settings. Open the orange `>_` in the toolbar to inspect the host, configuration root and environment ID, then inspect the configuration sources under Environment details before choosing a task.

Create an empty environment for new local state, or preserve work to carry instructions, memory and session material forward. Select the configuration root, project cwd and browser profile separately; see [Core concepts](concepts.en.md).

For subsequent work, go directly to the [Operator guide](operator-guide.en.md). Preserve the original ID when investigating a problem and use [Troubleshooting](troubleshooting.en.md).

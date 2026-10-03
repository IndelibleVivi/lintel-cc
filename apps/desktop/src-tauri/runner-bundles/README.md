# Linux runner resources

Generated binaries and `manifest.json` are ignored. Build `lintel-runner` for
`x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl`, then run
`node apps/desktop/scripts/prepare-remote-runners.mjs` from the repository root.
The script accepts only static ELF binaries for the matching architecture,
records byte size and SHA-256, and fails if either target is missing.

Tauri packages this directory as `remote-runners`. No binary is downloaded by
the application. Desktop development without these resources remains usable;
remote installation returns `bundle_unavailable` until they are prepared.
See [remote guide](../../../../docs/remote.md) for the build and install contract.

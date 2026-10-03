# Browser host resource

`npm run desktop:build` builds the dedicated Native Messaging executable from
`extensions/browser/native-host` through `scripts/prepare-browser-host.mjs`.
Generated `lintel-browser-host` and `manifest.json` are ignored and packaged by
Tauri as `browser-host`. No executable is downloaded by the application.
The helper supports native target builds. It checks Tauri's target triple and
rejects cross target or universal builds before compilation; build on the
matching macOS architecture without `--target`.

Development can prepare the same resource with `npm run prepare:browser-host`.
Missing resources produce an explicit `bundle_unavailable` preview error.
After exact installation approval, the App copies this executable into a
version and digest directory under the current user's Lintel application
support directory. Browser manifests never depend on the App's movable path.

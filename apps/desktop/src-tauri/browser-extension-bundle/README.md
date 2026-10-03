# Browser extension resources

`npm run desktop:build` packages the canonical Chromium and Firefox extension
files through `scripts/prepare-browser-extension.mjs`, using the existing
`extensions/browser/scripts/build.mjs` builder. Chrome and Edge share the
Chromium package. Generated package directories and metadata are ignored and
packaged by Tauri as `browser-extensions`; fixture packages are excluded.

Development can prepare these resources with `npm run prepare:browser-extension`.
The desktop preview verifies every bundled file against the generated inventory
and rejects synthetic configuration. After approval of the complete preview,
the App copies files into a stable directory below the current user's Lintel
application support directory. Only an unchanged installation owned by this
installer may be upgraded. App moves do not change the extension directory.
An interrupted staging or directory switch remains visible in a read-only
preview. Its exact files and transaction must receive a fresh approval before
continuation; Finder stays blocked until the switch and reviewed temporary
directory cleanup both finish.

The App opens that installed directory in Finder on explicit request. Copying
files does not load, register, sign, pair, or modify a browser profile. The user
still loads the Chromium directory through the browser's developer interface;
Firefox temporary loading uses its `manifest.json` and is removed on browser
exit. Long-term Firefox distribution still requires Mozilla signing.

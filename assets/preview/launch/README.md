# Lintel launch cards

Editable Chinese and English social artwork for **Lintel, the Claude Code
environment and data manager**. Work archives are one module, alongside
configuration, local state, changes and recovery, browser/network operations,
and remote/Agent entry points.

- [`cards.mjs`](cards.mjs) owns paired copy: six overview pages, five work-data
  pages and five configuration/local-state pages.
- [`prepare-social-posts.mjs`](../../../scripts/prepare-social-posts.mjs) owns
  composition and export: 32 portrait PNGs at 1080×1440 and 32 separately
  reflowed landscape PNGs at 1920×1080.

From the repository root, using the existing browser-test Playwright and its
managed Chromium:

```sh
node scripts/prepare-social-posts.mjs --check-only
node scripts/prepare-social-posts.mjs --output "$PWD/candidate-packages/social-posts-0.1.0-preview"
```

The output folder must be absent; the generator never clears or overwrites an
existing export. `--check-only` writes nothing. `--slice overview-01,overview-02`
renders those pages in both languages and formats for a focused correction.
An optional `PLAYWRIGHT_CHROMIUM_EXECUTABLE` selects a reviewed test browser;
never use an everyday Chrome profile.

Each export includes self-contained editable HTML, a finite image manifest,
contact sheets, 390 px portrait previews, 900 px landscape previews, and
`qa/qa.json`. Geometry checks reject text overflow, missing media and content
crossing its allocated region, including the header/footer boundaries. Inspect
the actual PNGs as well; a passing check is not owner aesthetic acceptance.
Portrait explanatory text is sized for phone reading; screenshots retain their
own original UI scale and are evidence rather than substitutes for the captions.

The canonical outlined wordmark and foot mask are embedded unchanged. Native
captures come from the tested `fcae6cb` App with synthetic data; the selection
detail is visibly cropped/edited, and the reader remains Chinese UI in English
artwork. See [screenshot provenance](../screenshots/README.md). The work-cover
illustration reuses the existing local website plate and is labeled conceptual.
Configuration diffs and recovery scenes are illustrative, with synthetic values.
No remote requests, font redistribution or new third-party assets are required.

Generated images, HTML, contact sheets and QA reports stay outside Git or in
ignored candidate output. These files prepare artwork; they do not publish a
social post, change a release or activate hosting. Preview platform/signing
limits appear on the overview's release page and in the accompanying post copy.

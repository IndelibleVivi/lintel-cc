# User work journey

The selected diagram answers one user question: how do the original files
become a portable package, and what can be done with it afterwards?

`work-journey.zh-CN.svg` and `work-journey.en.svg` are the editable paired
sources, each 1600 × 900. Their PNG siblings are portable README exports.
The source files, selection/approval step, encrypted envelope and independent
reader form one horizontal journey. Importing into an environment is a
separately approved lower branch; SSH is a small scope note. The diagram does
not imply that opening a package imports or launches it.

Warm paper, charcoal ink, terracotta and restrained object outlines follow
Lintel’s [visual identity](../../../docs/visual-language.md). The upper-right
mark reuses the canonical geometry. Chinese display text uses locally available
LXGW WenKai and Songti/Source Han Serif fallbacks; English uses Georgia, with
sans body text and monospace file labels. No font files are bundled.

```sh
node scripts/render-preview-svg.mjs assets/preview/diagrams/work-journey.zh-CN.svg \
  assets/preview/diagrams/work-journey.zh-CN.png
node scripts/render-preview-svg.mjs assets/preview/diagrams/work-journey.en.svg \
  assets/preview/diagrams/work-journey.en.png
```

The optional third argument checks a particular CSS width. Both editions are
reviewed at the README reading width of 900 px; structure, localization and
readability checks remain distinct from owner aesthetic acceptance. Sources,
purpose and alt text are in the [asset index](../asset-index.json).

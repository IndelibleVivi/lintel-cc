# Preview presentation assets

Local presentation exports for Lintel 0.1.0 Preview. These files do not create
a Release, configure hosting or upload a GitHub social preview.

| Asset | Intended surface |
| --- | --- |
| `lintel-banner.svg` / `.png` | Warm-paper README banner, 1600 × 900 |
| `lintel-banner-night.svg` / `.png` | Night presentation sibling, 1600 × 900 |
| `lintel-social-preview.svg` / `.png` | Share composition, 1280 × 720 |
| `film.tsx` / `player.html` | Editable 20-second concept intro and opt-in local player; MP4/audio/bundle outputs stay ignored |

The composition follows the supplied concept's fine character forest, layered
mountains, moon and reflected lake. The tiny Clawd stays on the shore. Large
presentation art is independent of the App's smaller 72 × 24 landscape album.

The wordmark's terracotta base **covers the whole last `l` foot and extends a
short way beyond it**. The shared baseline remains exact; subtle rounded corners
are clean because a local SVG mask removes the old foot underneath. The
[standalone wordmark](../../apps/desktop/assets/identity/source/lintel-wordmark.svg)
and horizontal lockup own this geometry. Glyph outlines are retained. Website
header/footer use the same glyph with a detached slow-blinking underscore;
presentation exports keep the integrated base static. The smaller lockup,
additional sky space and lower landscape match the website's more open composition.
The [mark](../../apps/desktop/assets/identity/source/lintel-mark.svg) remains native
vector geometry. Image generation never redraws the identity.

## Concept film

The concept intro uses a continuous camera through a spatial character world,
file streams into a work package, a Preview/Approve/Verify sequence and a Clawd
crossing. Its clean vector sprite comes from the shared connected silhouette
and dark eyes, with deck-following feet, depth-aware rails and attached cargo.
The orange cursor returns to the exact canonical wordmark foot at the end.
Music/SFX are originally synthesized with Tone.js; optional English narration
is generated locally with Kokoro. Music-only and narrated exports share the
same encoded frames. They are conceptual animation, not a native UI recording.

See the [production guide](../../scripts/preview-film/README.md) for the pinned,
isolated runtime, audio stems, render command, local player and rights sources.
The separate 30-second native edit uses actual App screenshots with synthetic
data and a visible edited label; it does not add a continuous-recording claim.
Generated movies, WAVs, model cache, browser bundles and QA captures are not
presentation source assets to commit. Visual owner acceptance remains separate
from successful encoding and media validation.

## Sources and rebuilding

The composition and the website's marked identity projection blocks belong to
[`prepare-preview-art.mjs`](../../scripts/prepare-preview-art.mjs). Backgrounds
are selected local assets in [`apps/site/assets`](../../apps/site/assets):
`lintel-landscape.png` and `lintel-landscape-night.png`. The original supplied
concept is preserved separately; it is not a generated runtime resource.

From the repository root, with the existing browser-test Playwright and Chromium
installed (or `PLAYWRIGHT_MODULE` pointing to that installation):

```sh
node scripts/prepare-preview-art.mjs
```

This writes the three SVG/PNG pairs and refreshes the website's native vector
projections. Edit canonical identity or composition sources, then regenerate.
SVGs reference the repository-relative background files and need those files
when moved. **PNGs are portable single-file exports** and are used by READMEs.
These deliberate presentation exports are tracked; `qa/` screenshots are not.
Typography uses local system fonts. No remote asset or personal browser is used.

The warm banner was visually checked at 700 px wide and the share image at
640 px. Structure, rendered checks and subjective owner acceptance remain
separate. The rejected sparse small-album composition is superseded by these
exports; it is not an alternative active generator.

## Background generation

The backgrounds were edited with the built-in imagegen tool. Both preserve the
supplied concept's composition while leaving branding to native SVG. These are
the prompts used; rerunning them can produce a new image, so it does not replace
the deterministic rebuild above.

Day background prompt:

> EDIT TARGET: the supplied Lintel concept banner. Produce a background-only landscape plate, preserving the original composition, warm off-white paper, intricate fine monospaced-character / ASCII engraving texture, mountain ridges in the middle distance, dense pine forests along both shores, little rocky islands and glassy horizontal lake reflections. Keep the tiny terracotta pixel Clawd on the lower-left shore and the dotted typographic moon in the upper-right sky. Preserve the quiet upper-center negative space and full-bleed rich lower panorama; match the reference's finesse and depth, do not simplify to a sparse grid or enlarge the mascot. Remove ALL typography (the 'lintel' header, Preview text, slogan and rules) and BOTH large solid terracotta-and-charcoal architectural brand marks (the upper header mark and the one on the central island), including those marks' distinctive orange/black reflections. Reconstruct their backgrounds cleanly: uninterrupted warm paper in the upper area and a naturally empty central rocky island with neutral water below. We will overlay exact native vector branding afterwards. No words, letters used as readable labels, logos, captions, interface, border, new symbol, extra character, photographic texture, or painterly line illustration. Tiny scattered punctuation / ASCII glyphs are the actual landscape texture. Wide 2:1 composition, high detail, restrained charcoal and muted terracotta, opaque paper background.

Night background prompt:

> EDIT TARGET: the supplied background-only typographic landscape. Change ONLY its palette and light to Lintel's quiet night theme. Exact same wide 2:1 composition, exact same fine ASCII / tiny-character forest-and-mountain geometry, dotted moon, tiny pixel mascot on lower-left shore, empty central island, still-lake horizontal reflections, and empty upper-center area for vector text. Background dark warm charcoal #24231F, landscape glyphs soft muted warm grey #B8B4A6 with lighter moon and gentle pale sage shore variation, mascot muted terracotta #E0A485. Restraint and crisp typographic details, dense beautifully layered engraving, no painterly image, no newly invented contours. Opaque background. No titles, slogans, logos, rules, readable words, new symbol, extra object or interface. Preserve the layout exactly; this is the same plate after dusk, not a new scene.

## Website story illustrations

The below-hero website has two new local illustrations. They translate the
provided concept's fine character engraving into new scenes; they are not
App screens or a rendering of the small landscape album. Both were generated
with the built-in imagegen tool using the background-only day landscape as a
style reference. Source outputs are preserved separately; the selected files
are [`lintel-keep.png`](../../apps/site/assets/lintel-keep.png) and
[`lintel-crossing.png`](../../apps/site/assets/lintel-crossing.png). Website CSS
composes live text in their negative space and adapts their ink for night.
There is no raster-generated branding or text in either image.

Keep illustration prompt:

> Use case: illustration-story. Asset: a production illustration for the BELOW-HERO story of the Lintel website. Input image is a STYLE REFERENCE ONLY: preserve its exquisitely fine, dense tiny typewriter ASCII engraving, warm ivory paper, gray-black ink, layered pine/mountain/lake landscape and tiny terracotta pixel Clawd. Make a NEW 2:1 wide artwork: a quiet lakeshore reading spot. On the RIGHT HALF, a wooden bench under two tall pines holds a small open book, several slim paper sheets and a neatly tied archive folio. Tiny orange Clawd (the same simple rectangular crab from the reference) stands next to the folio. Behind them the lake and distant mountains give depth. The entire LEFT HALF and upper central area are almost empty warm ivory with only a few sparse fine dots, leaving a serene generous area for live website typography. All scenic objects, including book, pages, bench, pines, rocks and water, must be built visibly from fine ASCII characters, punctuation and dense ink microtexture, NOT cartoon line drawing, smooth 3D or a coarse 72-column console. Let the scene naturally disperse into blank paper around all outer edges, especially left and bottom. Tiny Clawd is the only colored subject, warm muted terracotta. Restrained, magical, literary, art-directed engraving with rich fine detail. NO text, titles, logo, watermark, UI panels, app windows or solid portal marks. Background opaque warm paper #faf9f5.

Crossing illustration prompt:

> Use case: illustration-story. Asset: the second continuous BELOW-HERO illustration for the Lintel website. Input image is a STYLE REFERENCE ONLY. Make a NEW 2:1 wide artwork matching its exquisitely fine tiny typewriter ASCII engraving, warm ivory paper, gray-black ink and layered pine/mountain/lake world. Scene: a delicate arched wooden footbridge over a mountain stream, seen in a poetic wide landscape. Main detailed scenery is on the LEFT HALF: distant mountains, pine forest, foreground grasses and a single elegant bridge running from lower-left shore to center-left. Tiny terracotta pixel Clawd, the same simple rectangular crab from the reference, is at the far end of the bridge carrying a very small tied paper folio. A few rocks and fine water reflections connect the scene. The entire RIGHT HALF and top central area are almost empty warm ivory, generous quiet paper for live website typography. Scenic shapes are built from fine dense ASCII punctuation and letters, with beautifully intricate engraving texture: never smooth line art, cartoon shading, 3D, or coarse 72-column console graphics. Fade/disperse into blank paper around all outer edges. The only color is tiny muted terracotta Clawd and a pinhead orange folio accent. Literary, serene, precise, rich character detail. NO text, headings, logos, portal marks, UI, windows or watermark. Opaque warm ivory background #faf9f5.

Lintel is an independent project. Clawd belongs to Anthropic. These exports do
not introduce new public licensing terms; the repository's current rights status
still applies. See the [visual contract](../../docs/visual-language.md) and
[product entrance](../../README.md).

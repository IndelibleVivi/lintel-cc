# Concept-film covers

`scripts/prepare-video-covers.mjs` composes dedicated Chinese and English covers
from the canonical outlined wordmark and an actual frame of the concept film.
It preserves the identity geometry and foot mask, applying the night palette.
The three sizes are 1920×1080, 1080×1440 and 1080×1920. Each has its own layout;
the portrait exports are not crops of a completed landscape cover.

Extract the 14.5-second bridge frame from the retained original film with the
existing isolated film runtime, then render into an absent local output folder:

```sh
node scripts/prepare-video-covers.mjs --frame /absolute/path/frame-14-5.png \
  --out /absolute/path/new-video-covers
```

The export uses the repository's existing browser-test Playwright/Chromium.
Local fonts are used and are not redistributed. PNGs, self-contained editable
HTML exports and QA previews stay outside Git. The original movie and frame
remain unchanged. Clawd belongs to Anthropic; Lintel is an independent project.
These are concept-film covers, not native App screenshots. Successful rendering
does not establish owner aesthetic acceptance or a platform upload.

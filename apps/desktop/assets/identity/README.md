# Lintel identity sources

Project identity assets supplied for Lintel in October 2026. These editable SVGs
share the load-bearing beam, two supports and lower-right cursor. They are source
materials, not generated runtime resources.

- The canonical App icon master is [`src/brand.svg`](../../src/brand.svg).
- Tauri consumes the five supplied exports in [`src-tauri/icons`](../../src-tauri/icons).
- `source/` contains transparent, monochrome and outlined display variants.
- `small/` contains optical 16, 24 and 32 px variants.
- `dark/` contains an optional presentation variant; the packaged App icon retains
  its light paper container in both OS themes.
- The live inline mark is `Brand()` in [`src/ui.tsx`](../../src/ui.tsx), with theme
  colors owned by [`src/styles.css`](../../src/styles.css).

The sidebar keeps its live `lintel_` wordmark and slow cursor blink. The outlined
wordmark does not replace UI typography. See the [visual contract](../../../../docs/visual-language.md)
for use and acceptance boundaries. These files do not introduce separate public
licensing terms; the repository's existing distribution status remains applicable.

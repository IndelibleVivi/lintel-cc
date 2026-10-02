// Optional, operator-supplied local font files. A clean checkout uses system fonts.
// Keep these assets ignored; no redistribution permission is implied.
declare const __LINTEL_LOCAL_FONTS__: string[];
for (const url of __LINTEL_LOCAL_FONTS__) {
  const match = url.match(/Anthropic(Sans|Serif|Mono)-(Roman|Italic)\.woff2$/);
  if (!match) continue;
  const font = new FontFace(`Anthropic ${match[1]}`, `url(${url})`, { style: match[2] === 'Italic' ? 'italic' : 'normal', weight: '100 900', display: 'swap' });
  document.fonts.add(font);
  void font.load().catch(() => document.fonts.delete(font));
}

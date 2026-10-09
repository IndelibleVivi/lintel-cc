export type LandscapeInk = 'sky' | 'far' | 'near' | 'light' | 'clawd' | 'eye' | 'warm';
export type LandscapeCell = { char: string; ink: LandscapeInk };
export const LANDSCAPE_COLS = 72;
export const LANDSCAPE_ROWS = 24;
export const landscapes = [
  { title: '月下山湖', subtitle: '月亮不用赶路。我们也不用。', description: 'Clawd 站在左岸，右上是一弯明月，远山在湖中留下安静的倒影。', place: 'A MOON TO KEEP', clawd: [9, 15], secret: 'Clawd 把湖边的一颗星星放进了你的口袋。' },
  { title: '雨夜小屋', subtitle: '灯亮着，茶也还温热。', description: '雨落在远处松林，Clawd 站在木屋门前，右边的窗户亮着一盏灯。', place: 'A LIGHT LEFT ON', clawd: [29, 15], secret: '给你留的那杯茶，一直都是热的。' },
  { title: '海边灯塔', subtitle: '再走一点点，就能听见海。', description: 'Clawd 在左边海岸等潮水，右侧灯塔向海面投出细光，几只飞鸟掠过天空。', place: 'WHERE THE SEA BEGINS', clawd: [9, 15], secret: '这枚小贝壳，说它想跟你回家。' },
  { title: '星野营火', subtitle: '今晚的待办：看星星。', description: 'Clawd 坐在帐篷和营火之间，头上是星座，远处的山安静地围住营地。', place: 'NOWHERE ELSE TO BE', clawd: [25, 15], secret: '一颗给你，一颗留给今晚。' },
] as const;

// Original terminal tableaux. Every mark has a fixed cell: shading characters
// make the scenery; solid text blocks keep Clawd's eyes and four feet readable.
export function composeLandscape(index: number, found: boolean): LandscapeCell[] {
  const cols = LANDSCAPE_COLS, rows = LANDSCAPE_ROWS;
  const grid: LandscapeCell[][] = Array.from({ length: rows }, () => Array.from({ length: cols }, () => ({ char: ' ', ink: 'sky' })));
  const point = (x: number, y: number, char: string, ink: LandscapeInk) => { if (x >= 0 && x < cols && y >= 0 && y < rows) grid[y][x] = { char, ink }; };
  const text = (x: number, y: number, value: string, ink: LandscapeInk) => Array.from(value).forEach((char, i) => point(x + i, y, char, ink));
  const art = (x: number, y: number, lines: string[], ink: LandscapeInk) => lines.forEach((line, i) => text(x, y + i, line, ink));
  const fill = (x: number, y: number, w: number, h: number, char: string, ink: LandscapeInk) => { for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) point(x + i, y + j, char, ink); };
  const cloud = (x: number, y: number, width: number) => {
    text(x + 4, y, '░'.repeat(width - 8), 'far');
    text(x + 2, y + 1, '░▒' + '▒'.repeat(width - 6) + '░', 'far');
    text(x, y + 2, '░'.repeat(width), 'far');
  };
  const mountain = (cx: number, top: number, height: number, ink: LandscapeInk) => {
    for (let row = 0; row < height; row++) {
      for (let dx = -row * 2; dx <= row * 2; dx++) {
        const edge = Math.abs(dx) >= row * 2 - 1;
        point(cx + dx, top + row, edge ? '░' : dx < 0 ? '▒' : '░', ink);
      }
      if (row < 2) text(cx - row, top + row, row ? '░▓░' : '▴', 'light');
    }
  };
  const pine = (x: number, y: number, size: number) => {
    for (let r = 0; r < size; r++) text(x - r, y + r, '░' + '▓'.repeat(r * 2) + '░', 'near');
    text(x, y + size, '▏', 'near');
  };
  const crescent = (x: number, y: number) => art(x, y, ['   ░▒▓▓▓░ ', ' ░▓██▒░   ', ' ▓██░     ', ' ▓██░     ', ' ░▓██▒░   ', '   ░▒▓▓▓░ '], 'light');
  const stars = (entries: [number, number, string][]) => entries.forEach(([x, y, mark]) => point(x, y, mark, 'light'));

  if (index === 0) {
    stars([[8, 2, '✦'], [32, 3, '·'], [46, 1, '+'], [28, 8, '·'], [65, 10, '·']]);
    crescent(53, 2); cloud(7, 6, 16);
    mountain(48, 9, 5, 'far'); mountain(60, 11, 3, 'far');
    text(30, 14, '· · · · · · · · · · · · · · · · · · ·', 'near');
    text(34, 16, '─ ─         ──             ─ ─', 'far');
    text(36, 18, '      ───          ──        ', 'far');
    text(46, 15, '        ░░░', 'light'); text(48, 17, '    · ░░ ·', 'light'); text(46, 19, '   ·  ·  ·', 'far');
    text(3, 21, '························', 'near');
    text(27, 22, ' ░ ░░▒▒░░░░░░ ░░░░░░░░░░░░░░░░░░ ░░░░░░░', 'far');
    text(3, 22, '   ˎ      ˎ              ˎ', 'near');
  } else if (index === 1) {
    cloud(5, 2, 23); cloud(43, 3, 19);
    for (const [x, y] of [[9, 7], [21, 5], [31, 7], [42, 6], [58, 7], [66, 9], [6, 12], [24, 11], [16, 9]]) text(x, y, '╱', 'far');
    pine(10, 12, 5); pine(19, 14, 4); pine(64, 13, 5);
    // A wide shelter, left porch, warm window and a small chimney.
    fill(28, 13, 28, 8, '░', 'far');
    art(25, 8, ['                 ░░', '                ▒▒', '             ▄▄▄▄▄▄▄', '         ▄▄▓▓▓▓▓▓▓▓▓▓▄▄', '     ▄▄▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▄▄', ' ▄▄▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▄▄', ' ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀'], 'near');
    fill(47, 16, 6, 4, '█', 'warm'); text(47, 17, '██┼███', 'near'); text(49, 16, '│', 'near'); text(49, 18, '│', 'near'); text(49, 19, '│', 'near');
    text(27, 21, '▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀', 'near');
    text(6, 22, ' ·  ·  ───    ·   ·                         ───   ·', 'far');
  } else if (index === 2) {
    cloud(7, 3, 18); stars([[39, 3, '·'], [67, 2, '·']]);
    art(32, 5, ['⌁       ⌁', '     ⌁'], 'near');
    // The lantern and its widening, quiet dotted beam.
    art(55, 5, ['    ▄', '  ▄▓▓▓▄', ' ▀▀▀▀▀▀▀', '  ▓███▓', ' ▀▀▀▀▀▀▀', '  ░▓▓▓░', '  ░▓▓▓░', '  ▒███▒', ' ░▒███▒░', ' ░▓▓▓▓▓░', ' ░▓▓▓▓▓░', ' ▒██▓██▒', '░▒██▓██▒░'], 'near');
    text(58, 8, '███', 'warm');
    text(35, 7, '·   ·   ·   ·   ·', 'light'); text(25, 10, '·    ·    ·    ·    ·    ·', 'light');
    text(2, 12, '· · · · · · · · · · · · · · · · · · · · · · · ·', 'far');
    text(29, 14, '    ────           ──', 'near'); text(28, 17, ' ──         ─────    ', 'far'); text(39, 19, '   ───       ──', 'far');
    text(3, 21, '·······················', 'near'); text(6, 22, ' ░░ ░░░▒░░░▒░░░░░░░░░ ░░      ░░░░░░░░░░░░░░░░░░▒▒▒▒▒▒▒▒▒░░', 'far');
    text(5, 20, '⌁', 'light');
  } else {
    stars([[7, 2, '·'], [15, 4, '✦'], [29, 2, '+'], [41, 4, '✦'], [59, 2, '·'], [66, 7, '+'], [52, 7, '·'], [26, 7, '·'], [9, 9, '·'], [35, 6, '·']]);
    text(44, 4, '· · ·', 'far'); text(51, 3, '+', 'light'); text(51, 5, '·', 'far'); text(53, 6, '✦', 'light');
    mountain(54, 10, 4, 'far'); mountain(64, 12, 2, 'far');
    pine(6, 12, 5); pine(67, 14, 5);
    art(12, 15, ['      ▄', '    ▄▒▓▒▄', '  ▄▒▒▓▓▒▒▒▄', '▄▒▒▒▒▓▓▓▒▒▒▒▄', '▀▀▀▀▀▀▀▀▀▀▀▀▀'], 'near');
    art(47, 16, ['  ░', ' ░▓░', '░▓█▓░', ' ▓▓▓', '╲▄▄▄╱'], 'warm');
    text(48, 14, '·', 'light');
    text(7, 22, '·   ˎ       ·           ˎ               ·     ˎ         ·', 'near');
    text(5, 23, '░ ░░░░░ ░░ ░░░░░░░░░░ ░░░░░░░░ ░░░░░░░░░ ░░ ░░░░░░ ░░░░░░░░░', 'far');
  }
  const [x, y] = landscapes[index].clawd;
  // Clear space around the hero: textures must never fill the eyes or feet.
  fill(x - 1, y - 1, 18, 6, ' ', 'sky');
  fill(x + 2, y, 12, 4, '█', 'clawd');
  fill(x, y + 2, 16, 1, '█', 'clawd');
  point(x + 4, y + 1, '█', 'eye'); point(x + 11, y + 1, '█', 'eye');
  for (const foot of [3, 5, 10, 12]) fill(x + foot, y + 4, 1, 1, '█', 'clawd');
  if (found) { point(x + 7, y - 3, '✦', 'light'); point(x + 15, y - 1, '+', 'light'); }
  return grid.flat();
}

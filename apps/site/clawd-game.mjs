// Clawd's connected silhouette, continuous short legs and eye marks are the canonical Sprite in
// apps/desktop/src/Clawd.tsx (hand-drawn from the Clawd silhouette and its terminal appearance).
// The four legs already descend from the body path, so the game never draws detached feet.
// Clawd belongs to Anthropic. This game never calls the native App or a remote service.
const STANDING_LEGS = [14, 24, 50, 60].map(x => ({ x, bottom: 50 }));
export function clawdBody(legs = STANDING_LEGS) {
  return 'M10 0H70V20H80V30H70V40' + [...legs].reverse()
    .map(({ x, bottom }) => `H${x + 6}V${bottom}H${x}V40`).join('') + 'H10V30H0V20H10Z';
}
export const BODY = clawdBody();
const GROUND = 188, PLAYER_X = 48, HEIGHT = 40, GRAVITY = 1450;
const EYES_OPEN = [[20, 10, 4, 10], [56, 10, 4, 10]];
const EYES_SHUT = [[19, 18, 8, 3], [55, 18, 8, 3]];
const STAR_HIT_X = 30, STAR_HIT_Y = 24;
// Two opposing pairs of short straight legs step once per 100px of travel.
const STRIDE_TRAVEL = 100;
const HOLD_SECONDS = .26, TAP_SECONDS = .09, HOLD_LIFT = 1400;
// Ordinary stars sit along the original tap arc, with bounded height variation. Tall obstacles
// lift their star too: clearing the obstacle must not leave its reward down by Clawd's feet.
// Every fourth star remains higher and double-value, asking for a held ascent.
const STAR_ROWS = [98, 106, 114];
const STAR_JITTER = [0, 5, -4, 3, -2, 6, -5, 4, -3, 2];
const STAR_OBSTACLE_GAP = HEIGHT / 2 + 18;
export function newRun() {
  return { status: 'ready', y: 0, velocity: 0, distance: 0, speed: 260, spawn: 1,
    obstacles: [], stars: [], collected: 0, elapsed: 0, landing: 0, buffer: 0,
    particles: [], dust: 0, cleared: 0, spawns: 0, holding: false, holdTime: 0,
    boosting: false, bonus: 0, lastReward: 25 };
}
export function points(run) { return Math.floor(run.distance / 10) + run.collected * 25 + (run.bonus || 0); }
// The star keeps its horizontal bond to the obstacle (12px in) but takes the next visible row.
export function spawnStar(obstacle, spawns = 0) {
  if (spawns % 4 === 3) return { y: 56 + Math.max(-4, Math.min(4, STAR_JITTER[spawns % STAR_JITTER.length])), x: obstacle.x + 12, value: 50 };
  const jitter = Math.max(-3, Math.min(3, STAR_JITTER[spawns % STAR_JITTER.length]));
  const y = Math.min(STAR_ROWS[spawns % STAR_ROWS.length] + jitter, GROUND - (obstacle.height ?? 0) - STAR_OBSTACLE_GAP);
  return { y, x: obstacle.x + 12 };
}

// ---- Obstacle families. Four real ground silhouettes, each with a fixed finite dimension band
// and its own jump rhythm. pot and fence are taller/wider but their heights stay inside the tap
// jump's reach; the held jump is what buys the high double-value star, not obstacle clearance.
// The kind ladder is deterministic, not random scatter: every third spawn may be harder once its
// distance gate opens, and a hard step is always followed by an easy one, so there is a genuine
// post-clear recovery window even at the densest spacing. Dimensions only jitter their band. --/
export const OBSTACLE_KINDS = ['stone', 'book', 'pot', 'fence'];
export const OBSTACLE_GATE = { stone: 0, book: 0, pot: 1400, fence: 2400 };
export const OBSTACLE_BOUNDS = {
  stone: { width: [22, 30], height: [16, 24] },
  book: { width: [26, 34], height: [14, 22] },
  pot: { width: [18, 24], height: [34, 44] },
  fence: { width: [32, 42], height: [30, 40] },
};
export function isHard(kind) { return kind === 'pot' || kind === 'fence'; }
export function pickKind(run) {
  const seq = run.spawns, d = run.distance;
  const hardPool = [];
  if (d >= OBSTACLE_GATE.pot) hardPool.push('pot');
  if (d >= OBSTACLE_GATE.fence) hardPool.push('fence');
  const last = run.obstacles.at(-1);
  if (hardPool.length && seq % 3 === 2 && !(last && isHard(last.kind))) return hardPool[seq % hardPool.length];
  return seq % 2 === 0 ? 'stone' : 'book';
}
export function spawnObstacle(run, random, boardWidth = 320) {
  const kind = pickKind(run);
  const [wLo, wHi] = OBSTACLE_BOUNDS[kind].width;
  const [hLo, hHi] = OBSTACLE_BOUNDS[kind].height;
  return { x: boardWidth + 16, width: wLo + Math.floor(random() * (wHi - wLo + 1)), height: hLo + Math.floor(random() * (hHi - hLo + 1)), kind, passed: false };
}
export function spawnInterval(run, random) {
  // Distance-driven difficulty densifies the field, and a slow sine gives short/long breathing
  // so the pacing never feels printed. A large post-clear gap follows a hard step, and a real
  // floor applied after the jitter keeps the consecutive-obstacle separation wide enough to
  // clear one and land before the next at every speed.
  const difficulty = Math.max(0, Math.min(1, run.distance / 3200));
  const base = 1.6 - 0.5 * difficulty;
  const breath = 1 + 0.3 * Math.sin(run.distance / 850);
  let interval = base * breath;
  const last = run.obstacles.at(-1);
  if (last && isHard(last.kind)) interval *= 1.28;
  interval *= 0.9 + random() * 0.28;
  return Math.max(interval, 360 / Math.max(run.speed, 120));
}

// Keep the original monochrome pixel/ASCII-like landscape: theme ink, stepped hills,
// small clouds and the same sun/moon. Places vary the marks, never the visual language.
export const PLACE_LENGTH = 1400;
export const PLACES = [{ id: 'lake' }, { id: 'forest' }, { id: 'court' }];
export function scene(run) {
  const pos = run.distance / PLACE_LENGTH, idx = Math.floor(pos), prog = pos - idx;
  return { index: idx % PLACES.length, t: prog > .82 ? (prog - .82) / .18 : 0 };
}
function wrap(v, span) { return ((v % span) + span) % span; }
function drawPlaceScene(ctx, width, place, p, distance, reduced, alpha = 1) {
  const drift = reduced ? 0 : distance * .055;
  ctx.save(); ctx.globalAlpha = alpha;
  ctx.fillStyle = p.soft; ctx.fillRect(0, 0, width, 230);
  ctx.fillStyle = p.line;
  for (let x = 0; x < width; x += 8) {
    const top = 143 - Math.round((Math.sin((x + drift) / 95) * 19 + Math.sin((x + drift) / 47 + 2) * 9) / 4) * 4;
    ctx.fillRect(x, top, 8, GROUND - top);
  }
  ctx.fillStyle = p.soft;
  for (let x = 0; x < width; x += 8) {
    const top = 165 - Math.round((Math.sin((x + drift * 1.8) / 65 + 1) * 8 + Math.sin((x + drift) / 37) * 4) / 4) * 4;
    ctx.fillRect(x, top, 8, GROUND - top);
  }
  // Quiet background marks share the old hill ink; obstacles retain the stronger foreground ink.
  ctx.fillStyle = p.line;
  if (place.id === 'forest') {
    for (let x = -152; x < width + 152; x += 152) {
      const centre = Math.round((x + 76 - wrap(drift, 152)) / 4) * 4;
      ctx.fillRect(centre - 2, 108, 4, 58);
      for (let tier = 0; tier < 3; tier++) for (let row = 0; row < 4; row++) {
        const half = (row + 1 + tier) * 4;
        ctx.fillRect(centre - half, 76 + tier * 20 + row * 4, half * 2, 4);
      }
    }
  } else if (place.id === 'court') {
    const x = Math.round((width * .62 - wrap(drift, width + 160) + 60) / 4) * 4;
    ctx.fillRect(x, 112, 4, 60); ctx.fillRect(x + 52, 112, 4, 60);
    ctx.fillRect(x - 12, 108, 80, 4); ctx.fillRect(x - 4, 104, 64, 4);
    ctx.fillRect(x + 4, 100, 48, 4);
    for (const side of [-16, 68]) {
      ctx.fillRect(x + side, 130, 4, 38);
      ctx.fillRect(x + side - 4, 132, 12, 12);
      ctx.fillStyle = p.soft; ctx.fillRect(x + side, 136, 4, 4); ctx.fillStyle = p.line;
    }
  }
  ctx.fillStyle = p.accent; ctx.globalAlpha = alpha * .55;
  ctx.fillRect(width - 109, 33, 22, 30); ctx.fillRect(width - 113, 38, 30, 20);
  ctx.globalAlpha = alpha;
  if (p.night) {
    ctx.fillStyle = p.soft; ctx.fillRect(width - 103, 28, 20, 25); ctx.fillRect(width - 98, 48, 16, 9);
    ctx.fillStyle = p.muted;
    for (let i = 0; i < 7; i++) {
      const x = 29 + i * 137, y = 26 + (i * 29) % 70;
      ctx.fillRect(x, y - 1, 1, 3); ctx.fillRect(x - 1, y, 3, 1);
    }
  }
  ctx.fillStyle = p.line;
  for (const [x, y] of [[width * .24, 51], [width * .64, 72]]) {
    ctx.fillRect(x, y, 36, 4); ctx.fillRect(x + 8, y - 5, 20, 5); ctx.fillRect(x + 13, y - 9, 10, 4);
  }
  ctx.fillRect(0, GROUND, width, 1);
  const groundOffset = reduced ? 0 : distance % 107;
  for (let x = -107; x < width + 107; x += 107) {
    ctx.fillRect(Math.round(x + 18 - groundOffset), GROUND + 10, 8, 2);
    ctx.fillRect(Math.round(x + 69 - groundOffset), GROUND + 22, 4, 2);
  }
  ctx.restore();
}
export function drawScenery(ctx, width, run, palette, reduced) {
  const place = scene(run);
  ctx.clearRect(0, 0, width, 230);
  drawPlaceScene(ctx, width, PLACES[place.index], palette, run.distance, reduced);
  if (!reduced && place.t > 0) drawPlaceScene(ctx, width, PLACES[(place.index + 1) % PLACES.length], palette, run.distance, false, place.t);
}
export function drawObstacle(ctx, obstacle, p) {
  const x = Math.round(obstacle.x), y = GROUND - obstacle.height;
  const base = p.line, fill = p.muted, top = p.bg;
  if (obstacle.kind === 'book') {
    ctx.fillStyle = fill; ctx.fillRect(x, y + 6, obstacle.width, obstacle.height - 6); ctx.fillRect(x + 3, y, obstacle.width - 4, 6);
    ctx.fillStyle = top; ctx.fillRect(x + 4, y + 9, obstacle.width - 7, 3); ctx.fillRect(x + 4, y + 15, obstacle.width - 7, 2);
  } else if (obstacle.kind === 'pot') {
    const rim = Math.round(obstacle.height * .55), middle = Math.round(x + obstacle.width / 2);
    ctx.fillStyle = base; ctx.fillRect(middle - 1, y, 2, rim);
    ctx.fillStyle = fill; ctx.fillRect(middle - 7, y + 5, 7, 3); ctx.fillRect(middle + 1, y + 10, 7, 3);
    ctx.fillRect(x, y + rim, obstacle.width, 4); ctx.fillRect(x + 2, y + rim + 4, obstacle.width - 4, obstacle.height - rim - 6);
    ctx.fillRect(x + 4, GROUND - 2, obstacle.width - 8, 2);
  } else if (obstacle.kind === 'fence') {
    const rail = Math.max(2, Math.round(obstacle.height * .16));
    const slats = Math.max(2, Math.round((obstacle.width - 4) / 7));
    for (let i = 0; i < slats; i++) {
      const sx = x + 2 + Math.round(i * (obstacle.width - 4) / (slats - 1)) - 1;
      if (sx >= x + obstacle.width - 3) continue;
      ctx.fillStyle = fill; ctx.fillRect(sx, y, 3, obstacle.height);
    }
    ctx.fillStyle = fill; ctx.fillRect(x, y + rail, obstacle.width, 2); ctx.fillRect(x, y + obstacle.height - rail, obstacle.width, 1);
  } else {
    ctx.fillStyle = fill; ctx.fillRect(x, y + 6, obstacle.width, obstacle.height - 6); ctx.fillRect(x + 5, y, obstacle.width - 10, 6);
    ctx.fillStyle = base; ctx.fillRect(x + 7, y + 6, 5, 3);
  }
}

function burst(run, x, y, kind, count) {
  for (let i = 0; i < count; i++) run.particles.push({ x, y, vx: (i - (count - 1) / 2) * 35, vy: -40 - (i % 3) * 25, life: .4 + i * .025, kind });
}
function takeoff(run) {
  run.velocity = 480; run.buffer = 0; run.landing = 0;
  run.holdTime = 0; run.boosting = run.holding;
  burst(run, PLAYER_X + 20, GROUND - 2, 'dust', 4);
}
export function jump(run) {
  if (run.status !== 'running') return;
  if (run.y === 0) takeoff(run);
  else run.buffer = .12; // A press just before landing is kept for the next step.
}
export function holdPower(run) { return Math.max(0, Math.min(1, (run.holdTime - TAP_SECONDS) / (HOLD_SECONDS - TAP_SECONDS))); }
export function pressJump(run) {
  if (run.status !== 'running' || run.holding) return;
  run.holding = true;
  jump(run); // Every input takes off now. Holding only extends this ascent.
}
export function releaseJump(run) {
  run.holding = false; run.boosting = false; run.holdTime = 0;
}
export function cancelJump(run) { releaseJump(run); run.buffer = 0; }
export function advance(run, dt, width, random = Math.random) {
  if (run.status !== 'running' || dt <= 0) return;
  let remaining = Math.min(dt, .1);
  while (remaining > 0 && run.status === 'running') {
    const step = Math.min(remaining, 1 / 120); remaining -= step;
    run.elapsed += step; run.distance += run.speed * step;
    run.speed = Math.min(350, 260 + run.distance / 300);
    run.buffer = Math.max(0, run.buffer - step); run.landing = Math.max(0, run.landing - step);
    const airborne = run.y > 0 || run.velocity > 0;
    if (airborne) {
      // Split at the tap/hold boundaries so the same gesture has the same flight at every FPS.
      let flight = step;
      while (flight > 1e-9) {
        const canLift = run.holding && run.boosting && run.velocity > 0 && run.holdTime < HOLD_SECONDS;
        const boundary = run.holdTime < TAP_SECONDS ? TAP_SECONDS : HOLD_SECONDS;
        const part = canLift ? Math.min(flight, boundary - run.holdTime) : flight;
        const gravity = GRAVITY - (canLift && run.holdTime >= TAP_SECONDS ? HOLD_LIFT : 0);
        run.y += run.velocity * part - gravity * part * part / 2; run.velocity -= gravity * part;
        if (canLift) run.holdTime = Math.min(HOLD_SECONDS, run.holdTime + part);
        flight -= part;
      }
      if (run.y <= 0) {
        run.y = 0; run.velocity = 0; run.landing = .14;
        burst(run, PLAYER_X + 22, GROUND - 2, 'dust', 5);
        run.boosting = false;
        if (run.buffer > 0) takeoff(run);
      }
    }
    run.dust -= step;
    if (run.y === 0 && run.dust <= 0) {
      run.particles.push({x:PLAYER_X + 9,y:GROUND - 2,vx:-36,vy:-16,life:.28,kind:'dust'}); run.dust = .095;
    }
    run.spawn -= step;
    if (run.spawn <= 0) {
      const obstacle = spawnObstacle(run, random, width);
      run.obstacles.push(obstacle);
      run.stars.push(spawnStar(obstacle, run.spawns));
      run.spawns++;
      run.spawn = spawnInterval(run, random);
    }
    for (const obstacle of run.obstacles) {
      obstacle.x -= run.speed * step;
      if (!obstacle.passed && obstacle.x + obstacle.width < PLAYER_X) { obstacle.passed = true; run.cleared++; }
    }
    run.obstacles = run.obstacles.filter(obstacle => obstacle.x + obstacle.width > 0);
    for (const star of run.stars) {
      star.x -= run.speed * step;
      if (Math.abs(star.x - (PLAYER_X + 32)) < STAR_HIT_X && Math.abs(star.y - (GROUND - HEIGHT / 2 - run.y)) < STAR_HIT_Y) {
        star.caught = true; run.collected++; run.lastReward = star.value || 25;
        run.bonus += run.lastReward - 25; burst(run, star.x, star.y, 'star', 7);
      }
    }
    run.stars = run.stars.filter(star => !star.caught && star.x > -10);
    for (const particle of run.particles) {
      particle.life -= step; particle.x += particle.vx * step; particle.y += particle.vy * step; particle.vy += 160 * step;
    }
    run.particles = run.particles.filter(particle => particle.life > 0);
    if (run.obstacles.some(obstacle => PLAYER_X + 52 > obstacle.x + 3 && PLAYER_X + 11 < obstacle.x + obstacle.width - 3 && run.y + 4 < obstacle.height)) { run.status = 'over'; cancelJump(run); }
  }
}
export function clawdPose(run, reduced = false) {
  const air = run.y > 0 || run.velocity > 0;
  // A paused run (including the pause-on-blur mid-jump and just-after-landing cases) freezes the
  // canonical shape exactly; only an actively running run deforms, stretches or squashes.
  if (reduced || run.status !== 'running') return { anchor: 0, scaleX: 1, scaleY: 1, legs: STANDING_LEGS, eyes: run.status === 'over' ? EYES_SHUT : EYES_OPEN };
  const pace = run.distance / STRIDE_TRAVEL * 2 * Math.PI;
  const anchored = !air && run.distance > 0;
  // Only a real landing arms this finite settle; buffered takeoff clears it.
  const landing = run.landing / .14;
  const ascending = air && run.velocity > 0;
  const stretch = ascending ? Math.min(1, run.velocity / 480) : 0;
  // Move only the continuous leg edges: the chest stays level and a supporting pair stays planted.
  const legs = anchored ? STANDING_LEGS.map(({ x }, i) => {
    const phase = pace + (i === 0 || i === 3 ? 0 : Math.PI);
    return { x: x - Math.cos(phase) * 1.5, bottom: 50 - Math.max(0, Math.sin(phase)) * 4 };
  }) : STANDING_LEGS;
  return {
    anchor: 0,
    scaleX: 1 + landing * .1,
    scaleY: 1 - landing * .1 + stretch * .1,
    legs,
    eyes: EYES_OPEN,
  };
}
// Finite, static copy for the shared runner. `zh-CN` is the default so the
// standalone website is unchanged; the App mounts with its resolved locale and
// can switch it live (labels/aria/status) without disposing the run.
const COPY = {
  'zh-CN': {
    topline: 'CLAWD / LITTLE RUN',
    thisRun: '这次', best: '最好',
    boardAria: 'Clawd 跳跃跑道',
    canvasAria: '四条腿的 Clawd 沿单色像素小路，经过山线、树林与庭院，跳过石头、书本、陶罐与栅栏，收集星星',
    jumpLabel: '跳一下', holdLabel: '高跳',
    pause: '暂停', resume: '继续',
    keys: 'SPACE / ↑ \u00a0 按下跳 · 按住更高 \u00a0 · \u00a0 P 暂停',
    instructions: '按下跑道或「跳一下」立即起跳；按住跳得更高，松手停止加高。空格 / ↑ 同样可用，P 暂停。普通星星 +25，高处的大星星 +50；离开游戏自动暂停。',
    storage: '最好成绩只留在这个浏览器里。',
    storageUnavailable: '浏览器存储不可用，成绩仅保留到关闭本页。',
    phaseReady: '把星星收进口袋。',
    phaseRunning: '嘿咻。星星在前面。',
    phasePausedTitle: '歇一下，路还在。',
    phasePausedSub: (n) => `已经收下 ${n} 颗星星。`,
    phasePausedStart: '继续走',
    phasePausedStatus: '已暂停。准备好再继续。',
    phaseOverTitle: '脚滑了一下。问题不大。',
    phaseOverSub: (pts, collected, cleared) => `${pts} 分 · ${collected} 颗星星 · 跳过 ${cleared} 个障碍`,
    phaseOverStart: '再来一次',
    phaseOverStatus: '碰到障碍了。星星先收好，再试一次。',
    collected: (n, reward) => `收到第 ${n} 颗星星，+${reward} 分。`,
    emptyMessage: '把星星收进口袋。',
    emptySub: '跳过小石头，顺便带一点光回来。',
    emptyStart: '出发吧',
    emptyStatus: '准备好了就出发。',
  },
  en: {
    topline: 'CLAWD / LITTLE RUN',
    thisRun: 'This run', best: 'Best',
    boardAria: 'Clawd jump track',
    canvasAria: 'Four-legged Clawd runs through monochrome pixel hills, woods and courtyard marks, jumping stones, books, pots and fences and collecting stars',
    jumpLabel: 'Jump', holdLabel: 'Higher',
    pause: 'Pause', resume: 'Resume',
    keys: 'SPACE / ↑ \u00a0 Press / hold \u00a0 · \u00a0 P Pause',
    instructions: 'Press the track or “Jump” to hop immediately; keep holding to jump higher, release to stop the lift. Space / ↑ work too; P pauses. Stars +25, large high stars +50. Leaving the game pauses it.',
    storage: 'Your best score stays only in this browser.',
    storageUnavailable: 'Browser storage is unavailable; the score is kept only until this page closes.',
    phaseReady: 'Pop the stars in your pocket.',
    phaseRunning: 'Heave-ho. The stars are ahead.',
    phasePausedTitle: 'A little break; the road is still here.',
    phasePausedSub: (n) => `Collected ${n} star(s) so far.`,
    phasePausedStart: 'Keep going',
    phasePausedStatus: 'Paused. Continue when ready.',
    phaseOverTitle: 'Slipped a little. No big deal.',
    phaseOverSub: (pts, collected, cleared) => `${pts} points · ${collected} star(s) · cleared ${cleared} obstacle(s)`,
    phaseOverStart: 'Try again',
    phaseOverStatus: 'Hit an obstacle. Pocket the stars and try once more.',
    collected: (n, reward) => `Collected star ${n}, +${reward} points.`,
    emptyMessage: 'Pop the stars in your pocket.',
    emptySub: 'Hop the little stones and bring a bit of light back.',
    emptyStart: 'Set off',
    emptyStatus: 'Set off whenever you are ready.',
  },
};
function resolveCopy(language) { return Object.hasOwn(COPY, language) ? COPY[language] : COPY['zh-CN']; }

export function mountClawdGame(root, { scoreKey = 'lintel.site.clawd.best', language = 'zh-CN' } = {}) {
  root.className = 'clawd-game';
  let copy = resolveCopy(language);
  root.innerHTML = `<div class="cg-topline"><span class="cg-topline-label">${copy.topline}</span><div><span class="cg-stars-label">✦ <b class="cg-stars">0</b></span><span><span class="cg-run-label">${copy.thisRun}</span> <b class="cg-score">00000</b></span><span><span class="cg-best-label">${copy.best}</span> <b class="cg-best">00000</b></span></div></div><div class="cg-board" tabindex="0" role="group" aria-label="${copy.boardAria}" aria-describedby="cg-instructions"><canvas aria-label="${copy.canvasAria}"></canvas><div class="cg-overlay"><p class="cg-message">${copy.emptyMessage}</p><p class="cg-submessage">${copy.emptySub}</p><button class="cg-start">${copy.emptyStart}</button></div><span class="cg-collect" aria-hidden="true"></span><label class="cg-hold"><span class="cg-hold-label">${copy.holdLabel}</span><progress max="100" value="0" aria-label="${copy.holdLabel}"></progress></label></div><div class="cg-controls"><div><button class="cg-jump"><span class="cg-jump-label">${copy.jumpLabel}</span> <span aria-hidden="true">↑</span></button><button class="cg-pause" disabled>${copy.pause}</button></div><p class="cg-status" role="status">${copy.emptyStatus}</p><span class="cg-keys" aria-hidden="true">${copy.keys}</span></div><p id="cg-instructions" class="cg-instructions"><span class="cg-instructions-text">${copy.instructions}</span><span class="cg-storage">${copy.storage}</span></p>`;
  const $ = selector => root.querySelector(selector);
  const board = $('.cg-board'), canvas = $('canvas'), ctx = canvas.getContext('2d');
  const score = $('.cg-score'), bestOutput = $('.cg-best'), overlay = $('.cg-overlay');
  const startButton = $('.cg-start'), pauseButton = $('.cg-pause'), status = $('.cg-status');
  // Initialise the visible width before accepting the first key. The observer's
  // first delivery is measurement, not a resize that should pause a new run.
  let run = newRun(), best = 0, frame = 0, last = 0, width = Math.round(board.clientWidth) || 800, input = null;
  let currentLanguage = Object.hasOwn(COPY, language) ? language : 'zh-CN';
  let storageBroken = false;
  const key = scoreKey;
  const controller = new AbortController();
  const listen = (target, event, listener) => target.addEventListener(event, listener, { signal: controller.signal });
  const reduced = matchMedia('(prefers-reduced-motion: reduce)');
  const darkScheme = matchMedia('(prefers-color-scheme: dark)');
  let palette;
  function setPalette() {
    const style = getComputedStyle(root); const value = name => style.getPropertyValue(name).trim();
    const theme = document.documentElement.dataset.theme;
    palette = {text:value('--text'),muted:value('--muted'),line:value('--line'),soft:value('--soft'),accent:value('--accent'),bg:value('--bg'),night:theme === 'dark' || (theme !== 'light' && darkScheme.matches)};
    draw();
  }
  try { const saved = Number(localStorage.getItem(key)); if (Number.isFinite(saved) && saved > 0) best = Math.floor(saved); }
  catch { storageBroken = true; $('.cg-storage').textContent = copy.storageUnavailable; }
  const pad = n => String(Math.floor(n)).padStart(5, '0');
  function saveBest() {
    best = Math.max(best, points(run)); bestOutput.textContent = pad(best);
    try { localStorage.setItem(key, String(best)); }
    catch { storageBroken = true; $('.cg-storage').textContent = copy.storageUnavailable; }
  }
  function statusCopy() {
    return {
      ready: [copy.phaseReady, copy.emptySub, copy.emptyStart, copy.emptyStatus],
      running: ['', '', '', copy.phaseRunning],
      paused: [copy.phasePausedTitle, copy.phasePausedSub(run.collected), copy.phasePausedStart, copy.phasePausedStatus],
      over: [copy.phaseOverTitle, copy.phaseOverSub(points(run), run.collected, run.cleared), copy.phaseOverStart, copy.phaseOverStatus],
    }[run.status];
  }
  function syncHold() {
    root.dataset.holding = String(run.holding && run.boosting && run.holdTime > TAP_SECONDS);
    $('.cg-hold').setAttribute('aria-hidden', String(!(run.holding && run.boosting && run.holdTime > TAP_SECONDS)));
    $('progress').value = Math.round(holdPower(run) * 100);
  }
  function sync() {
    syncHold();
    root.dataset.state = run.status;
    root.setAttribute('lang', currentLanguage);
    overlay.hidden = run.status === 'running';
    pauseButton.disabled = ['ready', 'over'].includes(run.status);
    pauseButton.textContent = run.status === 'paused' ? copy.resume : copy.pause;
    const text = statusCopy();
    $('.cg-message').textContent = text[0]; $('.cg-submessage').textContent = text[1]; startButton.textContent = text[2]; status.textContent = text[3];
    score.textContent = pad(points(run)); bestOutput.textContent = pad(best); $('.cg-stars').textContent = run.collected;
  }
  // Live language switch: relabel static text/aria and re-render the current
  // status/overlay without touching `run`, `best` or the animation loop.
  function applyLanguage(next) {
    currentLanguage = Object.hasOwn(COPY, next) ? next : 'zh-CN';
    copy = resolveCopy(currentLanguage);
    root.setAttribute('lang', currentLanguage);
    $('.cg-board').setAttribute('aria-label', copy.boardAria);
    $('canvas').setAttribute('aria-label', copy.canvasAria);
    $('.cg-topline-label').textContent = copy.topline;
    $('.cg-run-label').textContent = copy.thisRun;
    $('.cg-best-label').textContent = copy.best;
    $('.cg-jump-label').textContent = copy.jumpLabel;
    $('.cg-hold-label').textContent = copy.holdLabel;
    $('progress').setAttribute('aria-label', copy.holdLabel);
    $('.cg-keys').textContent = copy.keys;
    $('.cg-instructions-text').textContent = copy.instructions;
    $('.cg-storage').textContent = storageBroken ? copy.storageUnavailable : copy.storage;
    sync();
  }
  function pixelStar(x, y, size, color) {
    ctx.fillStyle = color;
    ctx.fillRect(Math.round(x - size / 2), Math.round(y - size * 1.5), size, size * 3);
    ctx.fillRect(Math.round(x - size * 1.5), Math.round(y - size / 2), size * 3, size);
  }
  function draw() {
    if (!palette) return;
    const p = palette;
    // The original theme ink and stepped landscape remain the same through each place.
    drawScenery(ctx, width, run, p, reduced.matches);
    ctx.fillStyle = p.line; ctx.fillRect(0, GROUND, width, 1);
    for (const obstacle of run.obstacles) drawObstacle(ctx, obstacle, p);
    for (const star of run.stars) {
      const bob = reduced.matches ? 0 : Math.sin(run.elapsed * 4 + star.x / 60) * 2;
      pixelStar(star.x, star.y + bob, star.value === 50 ? 6 : 4, p.night ? '#dec488' : '#9b762f');
      ctx.fillStyle = p.bg; ctx.fillRect(Math.round(star.x-1),Math.round(star.y+bob-1),2,2);
    }
    if (!reduced.matches) for (const particle of run.particles) {
      ctx.globalAlpha = Math.min(1, particle.life * 3);
      ctx.fillStyle = particle.kind === 'star' ? (p.night ? '#dec488' : '#9b762f') : p.muted;
      ctx.fillRect(Math.round(particle.x),Math.round(particle.y),particle.kind === 'star' ? 3 : 2,2);
    }
    ctx.globalAlpha = 1;
    const pose = clawdPose(run, reduced.matches);
    // One connected silhouette: the body path already ends in four short legs, so there are no
    // separately drawn feet. Scaling around the feet keeps ordinary running anchored on GROUND.
    ctx.save(); ctx.translate(PLAYER_X+32,GROUND-run.y-pose.anchor);ctx.scale(.8*pose.scaleX,.8*pose.scaleY);ctx.translate(-40,-50);
    ctx.fillStyle = p.accent; ctx.fill(new Path2D(clawdBody(pose.legs)));
    ctx.fillStyle = '#252320';
    for (const [x, y, w, h] of pose.eyes) ctx.fillRect(x, y, w, h);
    ctx.restore();
  }
  function tick(now) {
    if (run.status !== 'running') return;
    const collected = run.collected;
    advance(run,last ? (now-last)/1000 : 0,width);last=now;
    score.textContent=pad(points(run));$('.cg-stars').textContent=run.collected;
    $('.cg-collect').textContent=run.particles.some(p=>p.kind==='star') ? `✦ +${run.lastReward}` : '';
    syncHold();
    if (collected !== run.collected) status.textContent=copy.collected(run.collected, run.lastReward);
    draw();
    if (run.status === 'over') {clearInput();saveBest();sync();frame=0;}
    else frame=requestAnimationFrame(tick);
  }
  function start() {
    if (run.status === 'running') return;
    if (run.status !== 'paused') {run=newRun();$('.cg-collect').textContent='';}
    run.status='running';last=0;sync();board.focus({preventScroll:true});
    cancelAnimationFrame(frame);frame=requestAnimationFrame(tick);
  }
  function pause() {
    if (run.status !== 'running') return;
    clearInput();run.status='paused';cancelAnimationFrame(frame);frame=0;saveBest();sync();draw();
  }
  function clearInput() {
    const previous = input; input = null; cancelJump(run);
    if (previous?.kind === 'pointer' && previous.target.hasPointerCapture(previous.id)) previous.target.releasePointerCapture(previous.id);
    syncHold();
  }
  function hop() {start();jump(run);board.focus({preventScroll:true});}
  function pressPointer(event) {
    if (event.button !== 0 || !event.isPrimary || input || (event.currentTarget === board && event.target.closest('button'))) return;
    start(); pressJump(run);
    input = {kind:'pointer',id:event.pointerId,target:event.currentTarget};
    event.currentTarget.setPointerCapture(event.pointerId); syncHold();
  }
  function releasePointer(event) {
    if (input?.kind !== 'pointer' || input.id !== event.pointerId) return;
    const previous = input; input = null; releaseJump(run); syncHold();
    if (previous.target.hasPointerCapture(previous.id)) previous.target.releasePointerCapture(previous.id);
    board.focus({preventScroll:true});
  }
  function cancelPointer(event) { if (input?.kind === 'pointer' && input.id === event.pointerId) { clearInput(); draw(); } }
  const jumpButton = $('.cg-jump');
  listen(startButton,'click',start);
  // Assistive technology's semantic click is a normal hop. Pointer/key gestures
  // already committed on press and must never jump a second time on click.
  listen(jumpButton,'click',event=>{if(event.detail===0)hop();});
  listen(pauseButton,'click',()=>run.status==='paused'?start():pause());
  for (const target of [board,jumpButton]) {
    listen(target,'pointerdown',pressPointer);
    listen(target,'lostpointercapture',cancelPointer);
    listen(target,'keydown',event=>{
      if(event.target!==target || ![' ','ArrowUp','Enter','p','P'].includes(event.key))return;
      if(event.key==='Enter' && target===board)return;
      event.preventDefault(); if(event.repeat)return;
      if(event.key.toLowerCase()==='p'){if(run.status==='paused')start();else pause();return;}
      if(input)return;
      start(); pressJump(run);input={kind:'key',key:event.key};syncHold();
    });
  }
  listen(window,'pointerup',releasePointer);
  listen(window,'pointercancel',cancelPointer);
  listen(window,'keyup',event=>{
    if(input?.kind!=='key'||input.key!==event.key)return;
    event.preventDefault();input=null;releaseJump(run);syncHold();board.focus({preventScroll:true});
  });
  listen(root,'focusout',event=>{if(!root.contains(event.relatedTarget))pause();});
  listen(window,'blur',pause);
  listen(document,'visibilitychange',()=>{if(document.hidden)pause();});
  const visibility = new IntersectionObserver(entries=>{if(!entries[0].isIntersecting)pause();});
  visibility.observe(board);
  const resize = new ResizeObserver(()=>{
    const nextWidth=Math.round(board.clientWidth);if(nextWidth!==width)pause();width=nextWidth;
    const ratio=window.devicePixelRatio||1;canvas.width=width*ratio;canvas.height=230*ratio;
    ctx.setTransform(ratio,0,0,ratio,0,0);draw();
  });
  resize.observe(board);
  const themeChanges = new MutationObserver(setPalette);
  themeChanges.observe(document.documentElement,{attributes:true,attributeFilter:['data-theme']});
  listen(reduced,'change',draw);
  listen(darkScheme,'change',setPalette);
  sync();setPalette();
  const dispose = () => {
    clearInput();cancelAnimationFrame(frame);
    controller.abort(); visibility.disconnect(); resize.disconnect(); themeChanges.disconnect();
    if (run.distance > 0) saveBest();
    run.status = 'paused';
    root.replaceChildren();
  };
  // One interface for the website default (dispose fn) and the App (which also
  // needs a live language switch); both keep a single mount implementation.
  dispose.setLanguage = applyLanguage;
  dispose.language = () => currentLanguage;
  return dispose;
}

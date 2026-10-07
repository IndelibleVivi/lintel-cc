// A local illustrated world, separate from the single opt-in Clawd runner.
// No native bridge, network, audio, clipboard or persistent discovery state.
export function mountSiteWorld({ toggleTheme }) {
  const root = document.documentElement;
  const scene = document.querySelector('.hero-overlay');
  const canvas = document.querySelector('#lake-light');
  const ctx = canvas.getContext('2d');
  const motion = document.querySelector('#motion-toggle');
  const note = document.querySelector('#scene-note');
  const message = document.querySelector('#scene-message');
  const detour = document.querySelector('#scene-detour');
  const clawd = document.querySelector('#clawd-hello');
  const sky = document.querySelector('#sky-toggle');
  const reduced = matchMedia('(prefers-reduced-motion: reduce)');
  const finePointer = matchMedia('(hover: hover) and (pointer: fine)');
  const listeners = [];
  const discoveries = new Set();
  let paused = false, visible = false, disposed = false, constellation = false;
  let width = 0, height = 0, frame = 0, progressFrame = 0, last = 0, time = 0;
  let aim = { x: 0, y: 0 }, look = { x: 0, y: 0 }, ripples = [];
  let hello = 0, keys = '', lastKey = 0, paletteTheme = '', accent = '', ink = '', returnFocus = clawd;
  const stars = Array.from({ length: 27 }, (_, i) => ({
    x: (0.073 + i * 0.137) % 1,
    y: 0.19 + ((i * 0.071) % 0.33),
    phase: i * 1.7,
  }));
  const skyPoints = [[.065,.42],[.10,.31],[.155,.34],[.21,.22],[.26,.28]];
  const smallSkyPoints = [[.10,.045],[.27,.085],[.42,.045],[.66,.10],[.88,.045]];
  function on(target, event, handler, options) {
    target.addEventListener(event, handler, options);
    listeners.push(() => target.removeEventListener(event, handler, options));
  }
  function moving() { return !paused && !reduced.matches; }
  function active() { return !disposed && moving() && visible && !document.hidden && !!ctx; }
  function tell(text, offerGame = false) {
    if (scene.contains(document.activeElement) && !note.contains(document.activeElement)) returnFocus = document.activeElement;
    message.textContent = text;
    note.hidden = false;
    detour.hidden = !offerGame;
  }
  function discover(id) {
    discoveries.add(id);
    document.querySelector('#scene-discoveries').textContent = `发现 ${discoveries.size} / 3`;
    if (discoveries.size === 3) sky.hidden = false;
  }
  function setSky() {
    constellation = !constellation;
    sky.hidden = false;
    sky.setAttribute('aria-pressed', String(constellation));
    scene.dataset.sky = constellation ? 'stars' : 'quiet';
    tell(constellation ? '你找到了一条藏在天上的来路。星星不用被带走，抬头就好。' : '星图收起来了。山湖还在这里。');
    paint();
  }
  function stone(event) {
    const rect = scene.getBoundingClientRect();
    const fromWater = event.currentTarget.id === 'lake-touch' && event.detail > 0;
    const x = fromWater ? event.clientX - rect.left : width * .57;
    const y = fromWater ? event.clientY - rect.top : height * .82;
    ripples = [...ripples.slice(-5), { x, y, born: time }];
    scene.dataset.ripple = String(Number(scene.dataset.ripple || 0) + 1);
    if (!moving()) tell('∿ 石子落下了。停下动效时，湖面留下一圈静静的水纹。');
    paint();
  }
  function refreshMotion() {
    root.dataset.motion = reduced.matches ? 'reduced' : paused ? 'paused' : 'live';
    motion.setAttribute('aria-pressed', String(!moving()));
    motion.disabled = reduced.matches;
    motion.lastElementChild.textContent = reduced.matches ? '系统已减少动效' : paused ? '让山湖动起来' : '停下风景';
    motion.firstElementChild.textContent = moving() ? 'Ⅱ' : '▷';
    // Pausing also reveals any scenery whose entry animation had not happened yet.
    if (!moving()) {
      document.querySelectorAll('.story-art').forEach(node => node.classList.add('arrived'));
      aim = look = { x: 0, y: 0 };
      scene.style.setProperty('--scene-x', '0px');
      scene.style.setProperty('--scene-y', '0px');
    }
    schedule();
    paint();
  }
  function schedule() {
    if (active() && !frame) { last = 0; frame = requestAnimationFrame(tick); }
    if (!active() && frame) { cancelAnimationFrame(frame); frame = 0; last = 0; }
    scene.dataset.animation = active() ? 'running' : 'resting';
    root.dataset.pageVisibility = document.hidden ? 'hidden' : 'visible';
  }
  function resize() {
    const rect = scene.getBoundingClientRect();
    width = rect.width; height = rect.height;
    const dpr = Math.min(2, devicePixelRatio || 1);
    canvas.width = Math.round(width * dpr);
    canvas.height = Math.round(height * dpr);
    ctx?.setTransform(dpr, 0, 0, dpr, 0, 0);
    // Match the raster's center crop: desktop uses its full 2:1 plate, mobile crops it.
    const plateWidth = Math.max(width, height * 2);
    const crop = (width - plateWidth) / 2;
    clawd.style.setProperty('--clawd-x', `${crop + plateWidth * .155}px`);
    document.querySelector('#moon-toggle').style.setProperty('--moon-x', `${crop + plateWidth * .804}px`);
    paint();
  }
  function paint() {
    if (!ctx || !width || !height) return;
    ctx.clearRect(0, 0, width, height);
    if (paletteTheme !== root.dataset.theme) {
      paletteTheme = root.dataset.theme;
      const style = getComputedStyle(root);
      accent = style.getPropertyValue('--accent').trim();
      ink = style.getPropertyValue('--muted').trim();
    }
    const dark = paletteTheme === 'dark';
    ctx.font = '12px SFMono-Regular, Consolas, monospace';
    ctx.textAlign = 'center';
    // The center stays quiet around the native wordmark and live heading.
    for (const star of stars) {
      if (star.x > .30 && star.x < .70) continue;
      const glint = .5 + .5 * Math.sin(time * .6 + star.phase);
      ctx.globalAlpha = (dark ? .24 : .12) + glint * (dark ? .38 : .14);
      ctx.fillStyle = star.phase % 3 < 1 ? accent : ink;
      ctx.fillText(star.phase % 4 < 2 ? '+' : '·', star.x * width + look.x * 1.5, star.y * height + Math.sin(time * .3 + star.phase) * 2);
    }
    // A few wandering character fireflies at the shore; no whole-screen particle field.
    if (dark) for (let i = 0; i < 13; i++) {
      ctx.globalAlpha = .15 + .45 * (.5 + .5 * Math.sin(time * .8 + i));
      ctx.fillStyle = accent;
      ctx.fillText(i % 3 ? '·' : ':', (i * .163 % .94 + .03) * width + Math.sin(time * .23 + i) * 8, (.71 + (i % 4) * .026) * height + Math.sin(time * .37 + i * 2) * 5);
    }
    // Discrete character arcs echo the ASCII water; touch/keyboard makes the same ripple.
    ripples = ripples.filter(ripple => time - ripple.born < 3.8);
    for (const ripple of ripples) {
      const age = moving() ? time - ripple.born : .7;
      for (let ring = 0; ring < 3; ring++) {
        const r = 9 + age * 32 + ring * 16;
        const count = Math.max(16, Math.round(r / 3));
        ctx.fillStyle = accent;
        ctx.globalAlpha = Math.max(0, (1 - age / 3.8) * (.7 - ring * .14));
        for (let n = 0; n < count; n++) {
          const angle = n / count * Math.PI * 2;
          ctx.fillText('−', ripple.x + Math.cos(angle) * r, ripple.y + Math.sin(angle) * r * .22);
        }
      }
    }
    if (constellation) {
      const constellationPoints = width <= 700 ? smallSkyPoints : skyPoints;
      ctx.strokeStyle = accent; ctx.fillStyle = accent; ctx.globalAlpha = .6;
      ctx.setLineDash([2, 7]); ctx.beginPath();
      constellationPoints.forEach(([x, y], i) => i ? ctx.lineTo(x * width, y * height) : ctx.moveTo(x * width, y * height));
      ctx.stroke(); ctx.setLineDash([]);
      ctx.font = '16px SFMono-Regular, Consolas, monospace';
      constellationPoints.forEach(([x, y], i) => {
        ctx.globalAlpha = .6 + Math.sin(time * .7 + i) * .2;
        ctx.fillText(i === 3 ? '✧' : '+', x * width, y * height + 5);
      });
    }
    ctx.globalAlpha = 1;
  }
  function tick(now) {
    frame = 0;
    if (!active()) { schedule(); return; }
    // Bound work to about 30 updates/sec; frozen time does not jump on reentry.
    if (!last || now - last >= 32) {
      if (last) time += Math.min((now - last) / 1000, .06);
      last = now;
      look.x += (aim.x - look.x) * .10;
      look.y += (aim.y - look.y) * .10;
      scene.style.setProperty('--scene-x', `${look.x.toFixed(2)}px`);
      scene.style.setProperty('--scene-y', `${look.y.toFixed(2)}px`);
      paint();
    }
    frame = requestAnimationFrame(tick);
  }
  on(scene, 'pointermove', event => {
    if (!moving() || !finePointer.matches || event.pointerType === 'touch') return;
    const rect = scene.getBoundingClientRect();
    aim = { x: ((event.clientX - rect.left) / rect.width - .5) * 10, y: ((event.clientY - rect.top) / rect.height - .5) * 5 };
  });
  on(scene, 'pointerleave', () => { aim = { x: 0, y: 0 }; });
  on(document.querySelector('#lake-touch'), 'click', stone);
  on(clawd, 'click', () => {
    hello++;
    discover('clawd');
    clawd.dataset.mood = hello % 2 ? 'hello' : 'shy';
    const lines = [':3 工作带好了。你也要一起走吗？', '别戳啦，四只脚都站不稳了。', '偷偷告诉你：在空白处敲 L I N T E L，天上还有一点东西。'];
    tell(lines[(hello - 1) % lines.length], hello >= 2);
  });
  on(document.querySelector('#tail-secret'), 'click', () => {
    discover('tail');
    tell('这一小截橙色，是给下一步留的余地。小小的，也不能少。');
  });
  on(document.querySelector('#scene-note-close'), 'click', () => {
    note.hidden = true;
    // If dismissal held keyboard focus, put it back on a live scene control.
    returnFocus.focus({ preventScroll: true });
  });
  on(document, 'keydown', event => {
    if (event.key === 'Escape' && !note.hidden && visible) {
      note.hidden = true; returnFocus.focus({ preventScroll: true });
      return;
    }
    if (!visible) return;
    if (event.ctrlKey || event.altKey || event.metaKey || event.isComposing || event.repeat || event.key.length !== 1) return;
    if (event.target.closest('button, a, input, textarea, select, [contenteditable], #clawd-game')) return;
    if (performance.now() - lastKey > 2400) keys = '';
    lastKey = performance.now();
    keys = (keys + event.key.toLowerCase()).slice(-6);
    if (keys === 'lintel') { keys = ''; setSky(); }
  });
  on(sky, 'click', setSky);
  on(motion, 'click', () => { paused = !paused; refreshMotion(); });
  on(reduced, 'change', refreshMotion);
  on(document, 'visibilitychange', schedule);
  // Theme has one owner in site.mjs; the moon and scene toolbar use that same action.
  for (const button of document.querySelectorAll('.scene-moon, #scene-theme')) on(button, 'click', () => {
    toggleTheme(); discover('moon');
    tell(root.dataset.theme === 'dark' ? '夜色到了。Clawd 把一点光留在湖边。' : '天亮了。昨晚收好的东西，还在原处。');
    paint();
  });
  on(root, 'site-theme-change', paint);
  const sceneObserver = new IntersectionObserver(entries => {
    visible = entries[0].isIntersecting; schedule();
  }, { threshold: .05 });
  sceneObserver.observe(scene);
  const sceneryObserver = new IntersectionObserver(entries => {
    entries.forEach(entry => {
      entry.target.dataset.inView = String(entry.isIntersecting);
      if (entry.isIntersecting) entry.target.classList.add('arrived');
    });
  }, { threshold: .08 });
  document.querySelectorAll('.story-art, #workflow-trail').forEach(node => sceneryObserver.observe(node));
  root.classList.add('scene-motion');
  const sizeObserver = new ResizeObserver(resize);
  sizeObserver.observe(scene);
  const progress = document.querySelector('#reading-progress');
  function updateProgress() {
    progressFrame = 0;
    const max = document.documentElement.scrollHeight - innerHeight;
    progress.style.setProperty('--read', max > 0 ? Math.max(0, Math.min(1, scrollY / max)) : 0);
  }
  on(window, 'scroll', () => { if (!progressFrame) progressFrame = requestAnimationFrame(updateProgress); }, { passive: true });
  on(window, 'resize', updateProgress);
  [canvas, document.querySelector('#moon-toggle'), document.querySelector('#lake-touch'), clawd, document.querySelector('#tail-secret'), document.querySelector('#scene-tools'), progress].forEach(node => { node.hidden = false; });
  root.classList.add('world-ready');
  resize(); refreshMotion(); updateProgress();
  return () => {
    disposed = true;
    cancelAnimationFrame(frame); cancelAnimationFrame(progressFrame);
    sceneObserver.disconnect(); sceneryObserver.disconnect(); sizeObserver.disconnect();
    listeners.forEach(remove => remove());
    root.classList.remove('world-ready', 'scene-motion');
  };
}

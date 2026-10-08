// A local illustrated world, separate from the single opt-in Clawd runner.
// No native bridge, network, audio, clipboard or persistent discovery state.
export function mountSiteWorld({ toggleTheme }) {
  const root = document.documentElement;
  const scene = document.querySelector('.hero-overlay');
  const canvas = document.querySelector('#lake-light');
  const ctx = canvas.getContext('2d');
  const motion = document.querySelector('#motion-toggle');
  const clawd = document.querySelector('#clawd-hello');
  const word = document.querySelector('.hero-word');
  const reduced = matchMedia('(prefers-reduced-motion: reduce)');
  const finePointer = matchMedia('(hover: hover) and (pointer: fine)');
  const listeners = [], reactions = new Map();
  const scenery = [...document.querySelectorAll('.story-world')];
  const waveLifetime = 2600;
  let paused = false, visible = false, disposed = false, constellation = false;
  // Refresh the preference at mount/change, keeping animation frames off the live media query.
  // This also avoids a Chromium emulation race between CSS media updates and MQL notifications.
  let reducedOn = reduced.matches;
  let width = 0, height = 0, frame = 0, progressFrame = 0, waveTimer = 0, last = 0, time = 0;
  let aim = { x: 0, y: 0 }, look = { x: 0, y: 0 }, ripples = [];
  let keys = '', lastKey = 0, paletteTheme = '', accent = '', ink = '';
  const stars = Array.from({ length: 16 }, (_, i) => ({
    x: (0.073 + i * 0.137) % 1,
    y: 0.19 + ((i * 0.071) % 0.33), phase: i * 1.7,
  }));
  const skyPoints = [[.065,.42],[.10,.31],[.155,.34],[.21,.22],[.26,.28]];
  const smallSkyPoints = [[.10,.045],[.27,.085],[.42,.045],[.66,.10],[.88,.045]];
  const responseArt = {
    heart: '<svg viewBox="0 0 20 18"><path fill="currentColor" d="M2 2h6v3h4V2h6v7l-8 8-8-8Z"/></svg>',
    boat: '<svg viewBox="0 0 26 18"><path fill="var(--bg)" stroke="currentColor" stroke-width=".9" d="M2 10h22l-6 6H8ZM13 1v9H5ZM13 3l7 7h-7Z"/><path fill="none" stroke="currentColor" stroke-width=".7" d="M2 10l11 3 11-3"/></svg>',
    sparkles: '<svg viewBox="0 0 54 36"><path fill="currentColor" d="m10 12 2 5 5 2-5 2-2 5-2-5-5-2 5-2Zm28-9 2 4 4 2-4 2-2 4-2-4-4-2 4-2Zm9 22 1 3 3 1-3 1-1 3-1-3-3-1 3-1Z"/></svg>',
  };
  function on(target, event, handler, options) {
    target.addEventListener(event, handler, options);
    listeners.push(() => target.removeEventListener(event, handler, options));
  }
  function moving() { return !paused && !reducedOn; }
  function active() { return !disposed && moving() && visible && !document.hidden && !!ctx; }
  function react(target, kind) {
    clearTimeout(reactions.get(target));
    target.querySelector('.scene-response')?.remove();
    target.dataset.reacting = 'true';
    if (kind) {
      const response = document.createElement('span');
      response.className = `scene-response ${kind}`;
      response.setAttribute('aria-hidden', 'true');
      response.innerHTML = responseArt[kind];
      target.append(response);
    }
    reactions.set(target, setTimeout(() => {
      target.querySelector('.scene-response')?.remove();
      delete target.dataset.reacting;
      reactions.delete(target);
    }, kind === 'boat' ? 2300 : 1800));
  }
  function setSky() {
    constellation = !constellation;
    scene.dataset.sky = constellation ? 'stars' : 'quiet';
    paint();
  }
  function retireWaves(now) {
    ripples = ripples.filter(ripple => now - ripple.born < waveLifetime);
    scene.dataset.ripples = String(ripples.length);
  }
  function expireWaves() {
    waveTimer = 0;
    retireWaves(performance.now());
    paint();
    if (ripples.length) armWaveExpiry();
  }
  function armWaveExpiry() {
    clearTimeout(waveTimer);
    // One bounded expiry timer, independent of the ambient animation clock.
    // Waves also disappear while scenery is paused, offscreen or reduced.
    const deadline = Math.min(...ripples.map(ripple => ripple.born + waveLifetime));
    waveTimer = setTimeout(expireWaves, Math.max(0, deadline - performance.now()) + 1);
  }
  function stone(event) {
    const rect = scene.getBoundingClientRect();
    const water = event.currentTarget.getBoundingClientRect();
    const pointer = event.detail > 0;
    const x = pointer ? event.clientX - rect.left : water.left - rect.left + water.width * .57;
    const y = pointer ? event.clientY - rect.top : water.top - rect.top + water.height * .45;
    retireWaves(performance.now());
    ripples = [...ripples.slice(-5), { x, y, born: performance.now(), frozenAge: moving() ? undefined : 650 }];
    scene.dataset.ripple = String(Number(scene.dataset.ripple || 0) + 1);
    armWaveExpiry(); paint();
  }
  function refreshMotion() {
    reducedOn = reduced.matches;
    root.dataset.motion = reducedOn ? 'reduced' : paused ? 'paused' : 'live';
    motion.setAttribute('aria-pressed', String(!moving()));
    motion.disabled = reducedOn;
    const label = reducedOn ? '系统已减少动效' : paused ? '让山湖动起来' : '停下风景';
    motion.setAttribute('aria-label', label); motion.title = label;
    motion.lastElementChild.textContent = label;
    motion.firstElementChild.textContent = moving() ? 'Ⅱ' : '▷';
    if (!moving()) {
      const now = performance.now();
      ripples.forEach(ripple => { ripple.frozenAge ??= now - ripple.born; });
      scenery.forEach(node => node.classList.add('arrived'));
      aim = look = { x: 0, y: 0 };
      scene.style.setProperty('--scene-x', '0px');
      scene.style.setProperty('--scene-y', '0px');
    } else ripples.forEach(ripple => { delete ripple.frozenAge; });
    schedule(); paint();
  }
  function schedule() {
    if (active() && !frame) { last = 0; frame = requestAnimationFrame(tick); }
    if (!active() && frame) { cancelAnimationFrame(frame); frame = 0; last = 0; }
    scene.dataset.animation = active() ? 'running' : 'resting';
    root.dataset.pageVisibility = document.hidden ? 'hidden' : 'visible';
  }
  function placeClawd(world, sourceX, sourceY) {
    const img = world.querySelector('img'), button = world.querySelector('button');
    if (!img.naturalWidth) return;
    const box = img.getBoundingClientRect();
    const cover = getComputedStyle(img).objectFit === 'cover';
    const fit = cover ? Math.max : Math.min;
    world.querySelector('.story-character').setAttribute('preserveAspectRatio', `xMidYMid ${cover ? 'slice' : 'meet'}`);
    const scale = fit(box.width / img.naturalWidth, box.height / img.naturalHeight);
    const plateWidth = img.naturalWidth * scale, plateHeight = img.naturalHeight * scale;
    button.style.left = `${(box.width - plateWidth) / 2 + sourceX * scale}px`;
    button.style.top = `${(box.height - plateHeight) / 2 + sourceY * scale}px`;
    button.hidden = false;
  }
  function resize() {
    const rect = scene.getBoundingClientRect();
    width = rect.width; height = rect.height;
    const dpr = Math.min(2, devicePixelRatio || 1);
    canvas.width = Math.round(width * dpr); canvas.height = Math.round(height * dpr);
    ctx?.setTransform(dpr, 0, 0, dpr, 0, 0);
    const plateWidth = width <= 700 ? 840 : width, plateHeight = plateWidth / 2;
    const crop = (width - plateWidth) / 2, plateTop = height - plateHeight;
    clawd.style.setProperty('--clawd-x', `${crop + plateWidth * .155}px`);
    clawd.style.setProperty('--clawd-y', `${plateTop + plateHeight * .765}px`);
    const moon = document.querySelector('#moon-toggle');
    moon.style.setProperty('--moon-x', `${crop + plateWidth * .804}px`);
    moon.style.setProperty('--moon-y', `${plateTop + plateHeight * .367}px`);
    placeClawd(scenery[0], 1483, 677);
    placeClawd(scenery[1], 540, 512);
    paint();
  }
  function paint(now = performance.now()) {
    if (!ctx || !width || !height) return;
    ctx.clearRect(0, 0, width, height);
    if (paletteTheme !== root.dataset.theme) {
      paletteTheme = root.dataset.theme;
      const style = getComputedStyle(root);
      accent = style.getPropertyValue('--accent').trim(); ink = style.getPropertyValue('--muted').trim();
    }
    const dark = paletteTheme === 'dark';
    ctx.font = '12px SFMono-Regular, Consolas, monospace'; ctx.textAlign = 'center';
    for (const star of stars) {
      if (width <= 700 ? star.x > .12 && star.x < .88 : star.x > .30 && star.x < .70) continue;
      const glint = .5 + .5 * Math.sin(time * .6 + star.phase);
      ctx.globalAlpha = (dark ? .24 : .12) + glint * (dark ? .38 : .14);
      ctx.fillStyle = star.phase % 3 < 1 ? accent : ink;
      ctx.fillText(star.phase % 4 < 2 ? '+' : '·', star.x * width + look.x * 1.5, star.y * height + Math.sin(time * .3 + star.phase) * 2);
    }
    if (dark) for (let i = 0; i < 13; i++) {
      ctx.globalAlpha = .15 + .45 * (.5 + .5 * Math.sin(time * .8 + i)); ctx.fillStyle = accent;
      ctx.fillText(i % 3 ? '·' : ':', (i * .163 % .94 + .03) * width + Math.sin(time * .23 + i) * 8, (.76 + (i % 4) * .026) * height + Math.sin(time * .37 + i * 2) * 5);
    }
    retireWaves(now);
    ctx.strokeStyle = accent; ctx.lineWidth = .9; ctx.setLineDash([2, 4]);
    for (const ripple of ripples) {
      for (let ring = 0; ring < 3; ring++) {
        const age = ((moving() ? now - ripple.born : ripple.frozenAge ?? 650) - ring * 140) / 1000;
        if (age < 0 || age >= 2.3) continue;
        const progress = age / 2.3, radius = 8 + progress * 112;
        ctx.globalAlpha = .65 * (1 - progress) ** 1.5;
        ctx.beginPath(); ctx.ellipse(ripple.x, ripple.y, radius, radius * .22, 0, 0, Math.PI * 2); ctx.stroke();
      }
    }
    ctx.setLineDash([]);
    if (constellation) {
      const points = width <= 700 ? smallSkyPoints : skyPoints;
      ctx.strokeStyle = accent; ctx.fillStyle = accent; ctx.globalAlpha = .6;
      ctx.setLineDash([2, 7]); ctx.beginPath();
      points.forEach(([x, y], i) => i ? ctx.lineTo(x * width, y * height) : ctx.moveTo(x * width, y * height));
      ctx.stroke(); ctx.setLineDash([]); ctx.font = '16px SFMono-Regular, Consolas, monospace';
      points.forEach(([x, y], i) => {
        ctx.globalAlpha = .6 + Math.sin(time * .7 + i) * .2;
        ctx.fillText(i === 3 ? '✧' : '+', x * width, y * height + 5);
      });
    }
    ctx.globalAlpha = 1;
  }
  function tick(now) {
    frame = 0;
    if (!active()) { schedule(); return; }
    if (!last || now - last >= 32) {
      if (last) time += Math.min((now - last) / 1000, .06);
      last = now; look.x += (aim.x - look.x) * .10; look.y += (aim.y - look.y) * .10;
      scene.style.setProperty('--scene-x', `${look.x.toFixed(2)}px`);
      scene.style.setProperty('--scene-y', `${look.y.toFixed(2)}px`);
      paint(now);
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
  on(clawd, 'click', () => react(clawd, 'heart'));
  on(document.querySelector('#tail-secret'), 'click', () => react(word));
  on(document.querySelector('#keep-clawd'), 'click', event => react(event.currentTarget, 'boat'));
  on(document.querySelector('#crossing-clawd'), 'click', event => react(event.currentTarget, 'sparkles'));
  on(document, 'keydown', event => {
    if (!visible || event.ctrlKey || event.altKey || event.metaKey || event.isComposing || event.repeat || event.key.length !== 1) return;
    if (event.target.closest('button, a, input, textarea, select, [contenteditable], #clawd-game')) return;
    if (performance.now() - lastKey > 2400) keys = '';
    lastKey = performance.now(); keys = (keys + event.key.toLowerCase()).slice(-6);
    if (keys === 'lintel') { keys = ''; setSky(); }
  });
  on(motion, 'click', () => { paused = !paused; refreshMotion(); });
  on(reduced, 'change', refreshMotion);
  on(document, 'visibilitychange', () => { schedule(); paint(); });
  on(document.querySelector('#moon-toggle'), 'click', () => { toggleTheme(); paint(); });
  on(root, 'site-theme-change', () => paint());
  const sceneObserver = new IntersectionObserver(entries => {
    visible = entries[0].isIntersecting; schedule(); paint();
  }, { threshold: .05 });
  sceneObserver.observe(scene);
  const sceneryObserver = new IntersectionObserver(entries => {
    entries.forEach(entry => {
      entry.target.dataset.inView = String(entry.isIntersecting);
      if (entry.isIntersecting) entry.target.classList.add('arrived');
    });
  }, { threshold: .08 });
  [...scenery, document.querySelector('#workflow-trail')].forEach(node => sceneryObserver.observe(node));
  scenery.forEach(world => on(world.querySelector('img'), 'load', resize));
  root.classList.add('scene-motion');
  const sizeObserver = new ResizeObserver(resize);
  [scene, ...scenery].forEach(node => sizeObserver.observe(node));
  const progress = document.querySelector('#reading-progress');
  function updateProgress() {
    progressFrame = 0;
    const max = document.documentElement.scrollHeight - innerHeight;
    progress.style.setProperty('--read', max > 0 ? Math.max(0, Math.min(1, scrollY / max)) : 0);
  }
  on(window, 'scroll', () => { if (!progressFrame) progressFrame = requestAnimationFrame(updateProgress); }, { passive: true });
  on(window, 'resize', updateProgress);
  [canvas, document.querySelector('#moon-toggle'), document.querySelector('#lake-touch'), clawd, document.querySelector('#tail-secret'), motion, progress].forEach(node => { node.hidden = false; });
  root.classList.add('world-ready');
  resize(); refreshMotion(); updateProgress();
  return () => {
    disposed = true;
    cancelAnimationFrame(frame); cancelAnimationFrame(progressFrame); clearTimeout(waveTimer);
    reactions.forEach(clearTimeout); reactions.clear();
    document.querySelectorAll('.scene-response').forEach(node => node.remove());
    document.querySelectorAll('[data-reacting]').forEach(node => delete node.dataset.reacting);
    sceneObserver.disconnect(); sceneryObserver.disconnect(); sizeObserver.disconnect();
    listeners.forEach(remove => remove()); root.classList.remove('world-ready', 'scene-motion');
  };
}

import { mountClawdGame } from './clawd-game.mjs';

const root = document.documentElement;
const themeButton = document.querySelector('#theme-toggle');
let theme = matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
try {
  const saved = localStorage.getItem('lintel.site.theme');
  if (saved === 'light' || saved === 'dark') theme = saved;
} catch {}

function applyTheme() {
  root.dataset.theme = theme;
  const label = theme === 'light' ? '切换到夜色' : '切换到日光';
  themeButton.setAttribute('aria-label', label);
  themeButton.title = label;
  themeButton.firstElementChild.textContent = theme === 'light' ? '☾' : '☼';
}
applyTheme();
themeButton.addEventListener('click', () => {
  theme = theme === 'light' ? 'dark' : 'light';
  applyTheme();
  try { localStorage.setItem('lintel.site.theme', theme); } catch {}
});

// The page is a product story. Reading uses native <details>; no execution demo.
const readingExample = document.querySelector('#sample-pages');
document.querySelector('a[href="#sample-pages"]').addEventListener('click', () => {
  readingExample.open = true;
  readingExample.querySelector('summary').focus({ preventScroll: true });
});

// Only scenery moves on entry: prose and trial links stay available immediately.
const reduced = matchMedia('(prefers-reduced-motion: reduce)');
const scenery = [...document.querySelectorAll('.story-art')];
if (typeof IntersectionObserver === 'function' && !reduced.matches) {
  root.classList.add('scene-motion');
  const observer = new IntersectionObserver((entries, obs) => {
    for (const entry of entries) {
      if (entry.isIntersecting) {
        entry.target.classList.add('arrived');
        obs.unobserve(entry.target);
      }
    }
  }, { threshold: 0.08 });
  scenery.forEach(node => observer.observe(node));
  reduced.addEventListener('change', () => {
    if (reduced.matches) {
      observer.disconnect();
      scenery.forEach(node => node.classList.add('arrived'));
    }
  });
}

mountClawdGame(document.querySelector('#clawd-game'));

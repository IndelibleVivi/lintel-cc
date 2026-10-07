import { mountClawdGame } from './clawd-game.mjs';
import { mountSiteWorld } from './site-world.mjs';
import { mountWorkflowTrail } from './workflow-trail.mjs';

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
  const sceneTheme = document.querySelector('#scene-theme');
  const sceneLabel = theme === 'light' ? '切换夜色主题' : '切换日光主题';
  sceneTheme.setAttribute('aria-label', sceneLabel);
  sceneTheme.firstElementChild.textContent = theme === 'light' ? '☾' : '☼';
  document.querySelector('#moon-toggle').setAttribute('aria-label', `碰一下月亮，${sceneLabel}`);
  root.dispatchEvent(new Event('site-theme-change'));
}
applyTheme();
function toggleTheme() {
  theme = theme === 'light' ? 'dark' : 'light';
  applyTheme();
  try { localStorage.setItem('lintel.site.theme', theme); } catch {}
}
themeButton.addEventListener('click', toggleTheme);

// The page is a product story. Reading uses native <details>; no execution demo.
const readingExample = document.querySelector('#sample-pages');
document.querySelector('a[href="#sample-pages"]').addEventListener('click', () => {
  readingExample.open = true;
  readingExample.querySelector('summary').focus({ preventScroll: true });
});

// Only illustration moves. Copy and real trial links are available immediately.
const disposeTrail = mountWorkflowTrail(document.querySelector('#workflow-trail'));
const disposeWorld = mountSiteWorld({ toggleTheme });
const disposeGame = mountClawdGame(document.querySelector('#clawd-game'));
// Preserve live mounts across the browser back/forward cache; ordinary unload disposes them.
addEventListener('pagehide', event => {
  if (!event.persisted) { disposeWorld(); disposeTrail(); disposeGame(); }
});

import { mountClawdGame } from './clawd-game.mjs';
import { mountFilmPlayer } from './film.mjs';
import { mountSiteWorld } from './site-world.mjs';
import { mountWorkflowTrail } from './workflow-trail.mjs';
import { mountSiteLanguage, siteLanguageFromPath } from './site-language.mjs';
import { THEME_COPY } from './site-copy.mjs';

const root = document.documentElement;
const themeButton = document.querySelector('#theme-toggle');
let language = siteLanguageFromPath(location.pathname);
let theme = matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
try {
  const saved = localStorage.getItem('lintel.site.theme');
  if (saved === 'light' || saved === 'dark') theme = saved;
} catch {}

function refreshThemeLabels() {
  const copy = THEME_COPY[language];
  const label = theme === 'light' ? copy.night : copy.day;
  themeButton.setAttribute('aria-label', label);
  themeButton.title = label;
  themeButton.firstElementChild.textContent = theme === 'light' ? '☾' : '☼';
  document.querySelector('#moon-toggle').setAttribute('aria-label', theme === 'light' ? copy.moonNight : copy.moonDay);
}
function applyTheme() {
  root.dataset.theme = theme;
  refreshThemeLabels();
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
const disposeTrail = mountWorkflowTrail(document.querySelector('#workflow-trail'), { language });
const disposeWorld = mountSiteWorld({ toggleTheme, language });
const disposeGame = mountClawdGame(document.querySelector('#clawd-game'), { language: language === 'en' ? 'en' : 'zh-CN' });
const disposeFilm = mountFilmPlayer(document);
const disposeLanguage = mountSiteLanguage(next => {
  language = next;
  refreshThemeLabels();
  disposeTrail.setLanguage?.(language);
  disposeWorld.setLanguage?.(language);
  disposeGame.setLanguage?.(language === 'en' ? 'en' : 'zh-CN');
  disposeFilm.setLanguage?.(language);
});
// Preserve live mounts across the browser back/forward cache; ordinary unload disposes them.
addEventListener('pagehide', event => {
  if (!event.persisted) { disposeLanguage(); disposeWorld(); disposeTrail(); disposeGame(); disposeFilm(); }
});

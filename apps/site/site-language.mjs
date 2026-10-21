import { SITE_COPY, ENGLISH_GUIDES, RELEASE_COPY, releaseSecurityCopy } from './site-copy.mjs';
import { SITE_ROUTES, homeStructuredData, serializeStructuredData } from './site-routes.mjs';

export const siteLanguageFromPath = pathname => /^\/en(?:\/|$)/.test(pathname) ? 'en' : 'zh';
const routePath = (pathname, language = siteLanguageFromPath(pathname)) => {
  const logical = pathname.replace(/^\/en(?=\/|$)/, '') || '/';
  const normalized = logical === '/' || logical.endsWith('/') || /\.[a-z0-9]+$/i.test(logical) ? logical : `${logical}/`;
  return language === 'en' ? `/en${normalized}` : normalized;
};
const docBase = 'https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/';
const guidePattern = new RegExp(`^(${docBase.replace(/[.*+?^${}()|[\]\\]/g,'\\$&')})(${ENGLISH_GUIDES.join('|')})(?:\\.en)?\\.md(#[a-z-]+)?$`);

export function siteGuideHref(href, language) {
  return href.replace(guidePattern, (_, base, guide, anchor = '') => `${base}${guide}${language === 'en' ? '.en' : ''}.md${anchor}`);
}

export function siteRouteHref(href, language) {
  const match = /^(\/[^?#]*)([?#].*)?$/.exec(href);
  if (!match) return href;
  const logical = match[1].replace(/^\/en(?=\/|$)/, '');
  if (!SITE_ROUTES.some(route => route.path !== '/' && route.path === logical)) return href;
  return routePath(logical, language) + (match[2] || '');
}

// Change only marked copy, finite guide links and metadata. Existing DOM and module instances
// remain mounted: language changes cannot reset a run, close details or reopen the video.
export function applySiteLanguage(language, root = document) {
  language = language === 'en' ? 'en' : 'zh';
  for (const node of root.querySelectorAll('[data-site-copy]')) {
    const copy = SITE_COPY[node.dataset.siteCopy];
    if (copy) node.textContent = copy[language];
  }
  for (const node of root.querySelectorAll('[data-site-release-copy]')) {
    const copy = RELEASE_COPY[node.dataset.siteReleaseCopy];
    if (copy) node.textContent = copy[language];
  }
  for (const node of root.querySelectorAll('[data-site-release-security]')) {
    node.textContent = releaseSecurityCopy(language, node.dataset.developerId, node.dataset.notarization);
  }
  for (const node of root.querySelectorAll('*')) {
    for (const attr of [...node.attributes]) {
      if (!attr.name.startsWith('data-site-attr-')) continue;
      const copy = SITE_COPY[attr.value];
      if (copy) node.setAttribute(attr.name.slice('data-site-attr-'.length), copy[language]);
    }
  }
  for (const node of root.querySelectorAll('a[href]')) {
    node.setAttribute('href', siteRouteHref(siteGuideHref(node.getAttribute('href'), language), language));
  }
  for (const link of root.querySelectorAll('[data-site-quickstart]')) {
    const alternate = link.dataset.siteQuickstart === 'alternate';
    const english = (language === 'en') !== alternate;
    link.href = `${docBase}quickstart${english ? '.en' : ''}.md`;
    link.lang = english ? 'en' : 'zh-CN';
    if (alternate) link.querySelector('[data-site-quickstart-label]').textContent = english ? 'English quickstart' : '中文首次试用';
  }
  for (const link of root.querySelectorAll('[data-site-language]')) {
    if (link.dataset.siteLanguage === language) link.setAttribute('aria-current','page');
    else link.removeAttribute('aria-current');
  }
  const html = root.documentElement;
  if (html) {
    html.lang = language === 'en' ? 'en' : 'zh-CN';
    html.dataset.language = language;
  }
  const currentPath = routePath(location.pathname, language);
  root.querySelector('link[rel="canonical"]')?.setAttribute('href', `https://lintel.page${currentPath}`);
  root.querySelector('meta[property="og:url"]')?.setAttribute('content', `https://lintel.page${currentPath}`);
  root.querySelector('meta[property="og:locale"]')?.setAttribute('content', language === 'en' ? 'en_US' : 'zh_CN');
  root.querySelector('meta[property="og:locale:alternate"]')?.setAttribute('content', language === 'en' ? 'zh_CN' : 'en_US');
  const structuredData = root.querySelector('#site-structured-data');
  if (structuredData) structuredData.textContent = serializeStructuredData(homeStructuredData(language));
  return language;
}

export function mountSiteLanguage(onLanguageChange, root = document) {
  let current = siteLanguageFromPath(location.pathname);
  const render = language => {
    current = applySiteLanguage(language, root);
    onLanguageChange(current);
  };
  const click = event => {
    if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    const language = event.currentTarget.dataset.siteLanguage;
    if (language !== current) {
      // Preserve the reader's query and section anchor, as well as the current scroll/focus.
      const target = routePath(location.pathname, language);
      history.pushState(null, '', target + location.search + location.hash);
      render(language);
    }
  };
  const pop = () => render(siteLanguageFromPath(location.pathname));
  const links = [...root.querySelectorAll('[data-site-language]')];
  for (const link of links) link.addEventListener('click', click);
  addEventListener('popstate', pop);
  render(current);
  return () => {
    for (const link of links) link.removeEventListener('click', click);
    removeEventListener('popstate', pop);
  };
}

export function languagePath(pathname, language) { return routePath(pathname, language); }

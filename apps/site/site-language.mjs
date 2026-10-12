import { SITE_COPY, ENGLISH_GUIDES, RELEASE_COPY, releaseSecurityCopy } from './site-copy.mjs';

export const siteLanguageFromPath = pathname => /^\/en(?:\/|$)/.test(pathname) ? 'en' : 'zh';
const languagePath = language => language === 'en' ? '/en/' : '/';
const docBase = 'https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/';
const guidePattern = new RegExp(`^(${docBase.replace(/[.*+?^${}()|[\]\\]/g,'\\$&')})(${ENGLISH_GUIDES.join('|')})(?:\\.en)?\\.md(#[a-z-]+)?$`);

export function siteGuideHref(href, language) {
  return href.replace(guidePattern, (_, base, guide, anchor = '') => `${base}${guide}${language === 'en' ? '.en' : ''}.md${anchor}`);
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
    node.setAttribute('href', siteGuideHref(node.getAttribute('href'), language));
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
  root.querySelector('link[rel="canonical"]')?.setAttribute('href', `https://lintel.page${languagePath(language)}`);
  root.querySelector('meta[property="og:url"]')?.setAttribute('content', `https://lintel.page${languagePath(language)}`);
  root.querySelector('meta[property="og:locale"]')?.setAttribute('content', language === 'en' ? 'en_US' : 'zh_CN');
  root.querySelector('meta[property="og:locale:alternate"]')?.setAttribute('content', language === 'en' ? 'zh_CN' : 'en_US');
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
      history.pushState(null, '', languagePath(language) + location.search + location.hash);
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

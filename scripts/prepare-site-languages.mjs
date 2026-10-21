// Chinese route sources + finite paired copy + paired repository guides
// -> tracked no-JS English mirrors and generated Chinese guide articles.
// No network, deployment, execution or runtime translation.
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { SITE_COPY, ROUTE_COPY } from '../apps/site/site-copy.mjs';
import {
  GUIDE_ARTICLES,
  SITE_ROUTES,
  SITE_ORIGIN,
  localizedRoute,
  localePath,
  sitePageMetadata,
  homeStructuredData,
  serializeStructuredData,
} from '../apps/site/site-routes.mjs';
import { renderGuideMarkdown, escapeHtml } from '../apps/site/site-docs.mjs';
import { siteGuideHref, siteRouteHref } from '../apps/site/site-language.mjs';

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const site = path.join(repo, 'apps/site');
const guideArticleByKey = new Map(GUIDE_ARTICLES.map(article => [article.key, article]));
const decode = value => value.replace(/&(amp|lt|gt|quot|#39);/g,(_,c)=>({
  amp:'&', lt:'<', gt:'>', quot:'"', '#39':"'",
}[c]));
const socialImage = `${SITE_ORIGIN}/assets/lintel-social-preview.png`;

function metadataFor(route, language) {
  const metadata = sitePageMetadata(route, language);
  return {
    title: escapeHtml(metadata.title),
    description: escapeHtml(metadata.description),
  };
}

function socialImageAlt(route, language) {
  return escapeHtml(sitePageMetadata(route, language).imageAlt);
}

const homeStructuredDataPattern = /<script type="application\/ld\+json" id="site-structured-data">[\s\S]*?<\/script>/;
function projectHomeStructuredData(html, language) {
  if (!homeStructuredDataPattern.test(html)) throw new Error('Missing home structured-data projection');
  const script = `<script type="application/ld+json" id="site-structured-data">\n${serializeStructuredData(homeStructuredData(language))}\n  </script>`;
  return html.replace(homeStructuredDataPattern, script);
}

function replaceOrInsertMeta(html, marker, meta) {
  const existing = new RegExp(`<meta ${marker.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}[^>]*>`, 'i');
  if (existing.test(html)) return html.replace(existing, meta);
  return html.replace('</head>', `  ${meta}\n</head>`);
}

function applyPageMetadata(html, route, language) {
  const { title, description } = metadataFor(route, language);
  const imageAlt = socialImageAlt(route, language);
  const properties = [
    ['name="description"', `<meta name="description" content="${description}">`],
    ['property="og:title"', `<meta property="og:title" content="${title}">`],
    ['property="og:description"', `<meta property="og:description" content="${description}">`],
    ['property="og:site_name"', '<meta property="og:site_name" content="Lintel">'],
    ['property="og:image"', `<meta property="og:image" content="${socialImage}">`],
    ['property="og:image:alt"', `<meta property="og:image:alt" content="${imageAlt}">`],
    ['name="twitter:card"', '<meta name="twitter:card" content="summary_large_image">'],
    ['name="twitter:title"', `<meta name="twitter:title" content="${title}">`],
    ['name="twitter:description"', `<meta name="twitter:description" content="${description}">`],
    ['name="twitter:image"', `<meta name="twitter:image" content="${socialImage}">`],
    ['name="twitter:image:alt"', `<meta name="twitter:image:alt" content="${imageAlt}">`],
  ];
  for (const [marker, meta] of properties) html = replaceOrInsertMeta(html, marker, meta);
  return html.replace(/<title[^>]*>[\s\S]*?<\/title>/, `<title>${title}</title>`);
}

function phraseFor(language, key) {
  const entry = SITE_COPY[key] || ROUTE_COPY[key];
  if (!entry) throw new Error(`Missing paired website phrase: ${key}`);
  return entry[language];
}

export function projectHomeHtml(source) {
  const used = new Set();
  const phrase = (key, sourceValue) => {
    const entry = SITE_COPY[key];
    if (!entry || decode(sourceValue) !== entry.zh) throw new Error(`Missing or stale website copy binding: ${key}`);
    used.add(key);
    return escapeHtml(entry.en);
  };
  let html = source.replace(/<([a-z0-9:-]+)([^>]*\sdata-site-copy="([^"]+)"[^>]*)>([^<]*)<\/\1>/g,
    (_,tag,attrs,key,value) => `<${tag}${attrs}>${phrase(key,value)}</${tag}>`);
  html = html.replace(/<[a-z][^>]*>/g, tag => {
    for (const [,attribute,key] of tag.matchAll(/\sdata-site-attr-([a-z:-]+)="([^"]+)"/g)) {
      const match = new RegExp(`(?<=\\s)${attribute}="([^"]*)"`).exec(tag);
      if (!match) throw new Error(`Missing bound ${attribute} attribute in ${tag.slice(0,160)}`);
      tag = tag.replace(match[0], `${attribute}="${phrase(key,match[1])}"`);
    }
    return tag;
  });
  for (const key of Object.keys(SITE_COPY)) if (!used.has(key)) throw new Error(`Unreferenced website phrase: ${key}`);
  html = projectHomeStructuredData(html, 'en');
  const withoutIntentionalChinese = html.replace(/<!--[\s\S]*?-->/g,'').replace(/<nav class="language-links"[\s\S]*?<\/nav>/,'');
  if (/[\u4e00-\u9fff]/.test(withoutIntentionalChinese)) throw new Error('Unbound Chinese website copy');
  html = html.replace('<html lang="zh-CN"','<html lang="en"');
  html = html.replace('rel="canonical" href="https://lintel.page/"','rel="canonical" href="https://lintel.page/en/"')
    .replace('property="og:url" content="https://lintel.page/"','property="og:url" content="https://lintel.page/en/"')
    .replace('property="og:locale" content="zh_CN"','property="og:locale" content="en_US"')
    .replace('property="og:locale:alternate" content="en_US"','property="og:locale:alternate" content="zh_CN"');
  html = html.replace(/href="(https:\/\/github.com\/IndelibleVivi\/lintel-cc\/blob\/main\/docs\/[^"]+)"/g,(_,href)=>`href="${siteGuideHref(href,'en')}"`);
  html = html.replace(/href="(\/[^"#][^"]*)"/g,(_,href)=>`href="${siteRouteHref(href,'en')}"`);
  html = html.replace(/(<a\b[^>]*data-site-quickstart="primary"[^>]*)(>)/g,(_,tag,end)=>`${tag} lang="en"${end}`);
  html = html.replace(/(<a\b[^>]*data-site-quickstart="alternate"[^>]*>)([\s\S]*?)(<\/a>)/g,(_,tag,body,end)=>tag.replace('quickstart.en.md','quickstart.md').replace('lang="en"','lang="zh-CN"')+body.replace('English quickstart','中文首次试用')+end);
  html = html.replace(/(<a\b[^>]*data-site-language="zh") aria-current="page"/,'$1').replace(/(<a\b[^>]*data-site-language="en")>/,'$1 aria-current="page">');
  return '<!-- Generated by scripts/prepare-site-languages.mjs; edit index.html + site-copy.mjs. -->\n'+html;
}

function rewriteRouteLinks(html, language) {
  return html.replace(/href="(\/[^"]*)"/g, (_, href) => {
    const target = href === '/' || href === '/en/' ? localePath('/',language) : siteRouteHref(href,language);
    return `href="${target}"`;
  });
}

function projectRouteHtml(source, language, route = null) {
  let html = source;
  html = html.replace(/<([a-z0-9:-]+)([^>]*\sdata-route-guide-title="([^"]+)"[^>]*)>([^<]*)<\/\1>/g,
    (_,tag,attrs,key) => `<${tag}${attrs}>${escapeHtml(guideArticleByKey.get(key)?.title[language] || '')}</${tag}>`);
  html = html.replace(/<([a-z0-9:-]+)([^>]*\sdata-route-guide-summary="([^"]+)"[^>]*)>([^<]*)<\/\1>/g,
    (_,tag,attrs,key) => `<${tag}${attrs}>${escapeHtml(guideArticleByKey.get(key)?.summary[language] || '')}</${tag}>`);
  html = html.replace(/<([a-z0-9:-]+)([^>]*\sdata-route-copy="([^"]+)"[^>]*)>([^<]*)<\/\1>/g,
    (_,tag,attrs,key) => `<${tag}${attrs}>${escapeHtml(phraseFor(language,key))}</${tag}>`);
  html = html.replace(/<[a-z][^>]*>/g, tag => {
    for (const [,attribute,key] of tag.matchAll(/\sdata-route-attr-([a-z:-]+)="([^"]+)"/g)) {
      const match = new RegExp(`(?<=\\s)${attribute}="([^"]*)"`).exec(tag);
      if (!match) throw new Error(`Missing bound ${attribute} route attribute in ${tag.slice(0,160)}`);
      tag = tag.replace(match[0], `${attribute}="${escapeHtml(phraseFor(language,key))}"`);
    }
    return tag;
  });
  html = rewriteRouteLinks(html, language);
  if (route) {
    html = applyPageMetadata(html, route, language);
    html = html.replace('<html lang="zh-CN"', `<html lang="${language === 'en' ? 'en' : 'zh-CN'}"`);
    const zhPath = localePath(route.path,'zh');
    const enPath = localePath(route.path,'en');
    const current = language === 'en' ? enPath : zhPath;
    html = html.replace(`rel="canonical" href="https://lintel.page${zhPath}"`, `rel="canonical" href="https://lintel.page${current}"`);
    html = html.replace(`property="og:url" content="https://lintel.page${zhPath}"`, `property="og:url" content="https://lintel.page${current}"`);
    html = html.replace('property="og:locale" content="zh_CN"', `property="og:locale" content="${language === 'en' ? 'en_US' : 'zh_CN'}"`);
  }
  if (route) html = html.replace(/<a\b[^>]*data-site-route-language="(zh|en)"[^>]*>/g, (tag, targetLanguage) => {
    tag = tag.replace(/\s(?:aria-current|data-route-path|data-article-language)="[^"]*"/g, '')
      .replace(/href="[^"]*"/, `href="${localePath(route.path,targetLanguage)}"`);
    return targetLanguage === language ? tag.replace(/>$/, ' aria-current="page">') : tag;
  });
  return html;
}

function projectGuideHtml(shell, source, language, article) {
  const prose = source.replace(/```[\s\S]*?```/g,'')
    .replace(/\[([^\]]*[\u4e00-\u9fff][^\]]*)\]\([^)]+\)/g,'')
    .replace(/^\s*(?:简体中文[^·\n]*·\s*English|English ·\s*简体中文[^\n]*|English\s*·\s*简体中文[^\n]*).*$/gm,'')
    .replace(/`[^`\n]*[\u4e00-\u9fff][^`\n]*`/g,'')
    .replace(/\]\([^)\n]*[\u4e00-\u9fff][^)\n]*\)/g,'](external)');
  if (language === 'en' && /[\u4e00-\u9fff]/.test(prose)) {
    throw new Error(`Paired English guide still contains uncoded Chinese prose: ${article.doc}.en.md`);
  }
  const body = renderGuideMarkdown(source, language);
  const title = article.title[language];
  const summary = article.summary[language];
  const zhPath = article.path;
  const enPath = localePath(article.path,'en');
  const current = language === 'en' ? enPath : zhPath;
  const canonical = `${SITE_ORIGIN}${current}`;
  let html = shell
    .replace('property="og:locale" content="zh_CN"', `property="og:locale" content="${language === 'en' ? 'en_US' : 'zh_CN'}"`)
    .replace('data-guide-document="start-here"', `data-guide-document="${escapeHtml(article.doc)}"`)
    .replace(`<a href="${enPath}" lang="en"`, `<a href="${localePath(article.path,'en')}" lang="en"`)
    .replace('<html lang="zh-CN"', `<html lang="${language === 'en' ? 'en' : 'zh-CN'}"`)
    .replace('<link rel="canonical" href="https://lintel.page/guide/">', `<link rel="canonical" href="${canonical}">`)
    .replace('<meta property="og:url" content="https://lintel.page/guide/">', `<meta property="og:url" content="${canonical}">`)
    .replace('<link rel="alternate" hreflang="zh-CN" href="https://lintel.page/guide/">', `<link rel="alternate" hreflang="zh-CN" href="https://lintel.page${zhPath}">`)
    .replace('<link rel="alternate" hreflang="en" href="https://lintel.page/en/guide/">', `<link rel="alternate" hreflang="en" href="https://lintel.page${enPath}">`)
    .replace('<link rel="alternate" hreflang="x-default" href="https://lintel.page/guide/">', `<link rel="alternate" hreflang="x-default" href="https://lintel.page${zhPath}">`)
    .replace('<!-- guide article body is generated from docs/start-here.md -->', body);
  html = html.replace('<nav class="article-breadcrumb" aria-label="面包屑">', `<nav class="article-breadcrumb" aria-label="${language === 'en' ? 'Breadcrumb' : '面包屑'}">`)
    .replace('<nav class="article-pager" aria-label="指南导航">', `<nav class="article-pager" aria-label="${language === 'en' ? 'Guide navigation' : '指南导航'}">`);
  html = html.replace('<a href="/guide/" data-route-copy="guideCrumb">指南</a>', `<a href="${language === 'en' ? '/en/guide/' : '/guide/'}" data-route-copy="guideCrumb">${language === 'en' ? 'Guides' : '指南'}</a>`);
  html = html.replace('<a href="/guide/" data-route-copy="allGuides">全部指南</a>', `<a href="${language === 'en' ? '/en/guide/' : '/guide/'}" data-route-copy="allGuides">${language === 'en' ? 'All guides' : '全部指南'}</a>`);
  html = html.replace(/<a href="https:\/\/github.com\/IndelibleVivi\/lintel-cc\/blob\/main\/docs\/start-here.md" data-route-copy="repositoryOriginal">[^<]*<span aria-hidden="true">↗<\/span><\/a>/, `<a href="https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/${article.doc}${language === 'en' ? '.en' : ''}.md" data-route-copy="repositoryOriginal">${language === 'en' ? 'Repository source' : '仓库原文'} <span aria-hidden="true">↗</span></a>`);
  return projectRouteHtml(html, language, { key:`guide:${article.key}`, path:article.path, article });
}

function sitemapUrls() {
  return SITE_ROUTES.flatMap(route => ['zh', 'en'].map(language => ({
    route,
    location: `${SITE_ORIGIN}${localePath(route.path, language)}`,
  })));
}

function projectSitemap() {
  const urls = sitemapUrls().map(({ route, location }) => {
    const alternates = [
      ['zh-CN', `${SITE_ORIGIN}${localePath(route.path, 'zh')}`],
      ['en', `${SITE_ORIGIN}${localePath(route.path, 'en')}`],
      ['x-default', `${SITE_ORIGIN}${localePath(route.path, 'zh')}`],
    ].map(([language, href]) => `<xhtml:link rel="alternate" hreflang="${language}" href="${href}"/>`).join('');
    return `  <url><loc>${location}</loc>${alternates}</url>`;
  }).join('\n');
  return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">\n${urls}\n</urlset>\n`;
}

async function routeSources() {
  const result = [];
  const home = await readFile(path.join(site, 'index.html'), 'utf8');
  const wordmark = home.match(/<!-- lintel-wordmark:start -->[\s\S]*?<!-- lintel-wordmark:end -->/)?.[0];
  if (!wordmark) throw new Error('Missing canonical home header identity');
  for (const route of SITE_ROUTES) {
    if (route.key === 'home') continue;
    const source = await readFile(path.join(site, route.source), 'utf8');
    if (!source.includes('<!-- site-wordmark -->')) throw new Error(`Missing shared wordmark slot: ${route.source}`);
    const shell = source.replace('<!-- site-wordmark -->', wordmark);
    if (route.article) {
      const article = route.article;
      const zhDoc = await readFile(path.join(repo,'docs',`${article.doc}.md`),'utf8');
      const enDoc = await readFile(path.join(repo,'docs',`${article.doc}.en.md`),'utf8');
      result.push({route,zh:projectGuideHtml(shell,zhDoc,'zh',article),en:projectGuideHtml(shell,enDoc,'en',article)});
    } else {
      result.push({route,zh:projectRouteHtml(shell,'zh',route),en:projectRouteHtml(shell,'en',route)});
    }
  }
  return result;
}

function validateRoute(html, route, language) {
  const expected = `${SITE_ORIGIN}${localePath(route.path, language)}`;
  const { title, description } = metadataFor(route, language);
  const imageAlt = socialImageAlt(route, language);
  if (!html.includes(`rel="canonical" href="${expected}"`)) throw new Error(`Canonical mismatch for ${route.key}/${language}`);
  if (!html.includes(`property="og:url" content="${expected}"`)) throw new Error(`OG URL mismatch for ${route.key}/${language}`);
  const documentTitle = title;
  if (!html.includes(`<title>${documentTitle}</title>`)) throw new Error(`Title mismatch for ${route.key}/${language}`);
  for (const marker of [
    `<meta name="description" content="${description}">`,
    `<meta property="og:title" content="${documentTitle}">`,
    `<meta property="og:description" content="${description}">`,
    '<meta property="og:site_name" content="Lintel">',
    `<meta property="og:image" content="${socialImage}">`,
    `<meta property="og:image:alt" content="${imageAlt}">`,
    '<meta name="twitter:card" content="summary_large_image">',
    `<meta name="twitter:title" content="${documentTitle}">`,
    `<meta name="twitter:description" content="${description}">`,
    `<meta name="twitter:image" content="${socialImage}">`,
    `<meta name="twitter:image:alt" content="${imageAlt}">`,
  ]) {
    if (!html.includes(marker)) throw new Error(`Incomplete social metadata for ${route.key}/${language}: ${marker}`);
  }
  for (const [altLanguage, altPath] of [['zh-CN',localePath(route.path,'zh')],['en',localePath(route.path,'en')]]) {
    if (!html.includes(`hreflang="${altLanguage}" href="${SITE_ORIGIN}${altPath}"`)) {
      throw new Error(`hreflang mismatch for ${route.key}/${language}: ${altPath}`);
    }
  }
  const englishSurface = html.slice(html.indexOf('<body'))
    .replace(/<!--[\s\S]*?-->/g,'')
      .replace(/\s(?:lang|hreflang)="zh-CN"/g,'')
      .replace(/>中文<\/a>/g,'>Chinese</a>')
      .replace('aria-label="停下风景"','aria-label="Pause the scenery"')
      .replace('title="停下风景"','title="Pause the scenery"')
      .replace('>停下风景</span>','>Pause the scenery</span>')
      .replace(/<article class="guide-body"[\s\S]*?<\/article>/, '')
    .replace(/<([a-z0-9:-]+)([^>]*)>[^<]*<\/(\1)>/g,'<$1$2></$1>');
  if (language === 'en' && /[\u4e00-\u9fff]/.test(englishSurface)) {
    throw new Error(`Unbound Chinese route copy: ${route.key}`);
  }
  const ids = new Set([...html.matchAll(/\sid="([^"]+)"/g)].map(match => match[1]));
  const deadAnchors = [...html.matchAll(/href="#([^"]+)"/g)].map(match => match[1]).filter(id => id && !ids.has(id));
  if (deadAnchors.length) throw new Error(`Dead same-page anchors in ${route.key}/${language}: ${deadAnchors.join(', ')}`);
}

async function targetFor(route, language) {
  return language === 'en'
    ? path.join(site,'en',...route.path.split('/').filter(Boolean),'index.html')
    : path.join(site,...route.path.split('/').filter(Boolean),'index.html');
}

export async function checkSiteLanguages({write = false} = {}) {
  const originalHome = await readFile(path.join(site,'index.html'),'utf8');
  const homeSource = projectHomeStructuredData(originalHome, 'zh');
  if (homeSource !== originalHome) {
    if (write) await writeFile(path.join(site,'index.html'),homeSource);
    else throw new Error('Home structured data is stale; run node scripts/prepare-site-languages.mjs');
  }
  const faviconSource = await readFile(path.join(repo,'apps/desktop/src-tauri/icons/128x128.png'));
  const faviconTarget = path.join(site,'assets/favicon.png');
  if (write) await writeFile(faviconTarget,faviconSource);
  else if (!(await readFile(faviconTarget).catch(() => Buffer.alloc(0))).equals(faviconSource)) {
    throw new Error('Website favicon is stale; run node scripts/prepare-site-languages.mjs');
  }
  const homeEnglish = projectHomeHtml(homeSource);
  const generated = await routeSources();
  const sitemap = projectSitemap();
  if (write) await writeFile(path.join(site,'en/index.html'),homeEnglish);
  else {
    const actualHome = await readFile(path.join(site,'en/index.html'),'utf8');
    if (actualHome !== homeEnglish) throw new Error('English static page is stale; run node scripts/prepare-site-languages.mjs');
  }
  for (const entry of generated) {
    for (const [language, html] of [['zh',entry.zh],['en',entry.en]]) {
      validateRoute(html, entry.route, language);
      const target = await targetFor(entry.route, language);
      if (write) {
        await mkdir(path.dirname(target),{recursive:true});
        await writeFile(target,html);
      } else {
        const actual = await readFile(target,'utf8').catch(() => null);
        if (actual !== html) throw new Error(`Generated route is stale: ${target.slice(repo.length+1)}`);
      }
    }
  }
  if (write) await writeFile(path.join(site, 'sitemap.xml'), sitemap);
  else {
    const actualSitemap = await readFile(path.join(site, 'sitemap.xml'), 'utf8');
    if (actualSitemap !== sitemap) throw new Error('Sitemap is stale; run node scripts/prepare-site-languages.mjs');
  }
  return {
    languages:['zh-CN','en'],
    phrases:Object.keys(SITE_COPY).length,
    routePhrases:Object.keys(ROUTE_COPY).length,
    routes:SITE_ROUTES.length,
    guides:GUIDE_ARTICLES.length,
    sitemapUrls:sitemapUrls().length,
  };
}

export function projectSiteHtml(source, language = 'en') {
  if (language === 'zh') return source;
  if (language !== 'en') throw new Error('Unsupported website language');
  return projectHomeHtml(source);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.argv.slice(2).some(arg => arg !== '--check')) throw new Error('Usage: prepare-site-languages.mjs [--check]');
  console.log(JSON.stringify(await checkSiteLanguages({write:!process.argv.includes('--check')})));
}

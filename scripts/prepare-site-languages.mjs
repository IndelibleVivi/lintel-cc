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
} from '../apps/site/site-routes.mjs';
import { renderGuideMarkdown, escapeHtml } from '../apps/site/site-docs.mjs';
import { siteGuideHref, siteRouteHref } from '../apps/site/site-language.mjs';

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const site = path.join(repo, 'apps/site');
const guideArticleByKey = new Map(GUIDE_ARTICLES.map(article => [article.key, article]));
const decode = value => value.replace(/&(amp|lt|gt|quot|#39);/g,(_,c)=>({
  amp:'&', lt:'<', gt:'>', quot:'"', '#39':"'",
}[c]));
const metadataTitles = Object.freeze({
  download: Object.freeze({zh:'下载与安装 · Lintel',en:'Download and install · Lintel'}),
  guide: Object.freeze({zh:'使用指南 · Lintel',en:'Guides · Lintel'}),
  inside: Object.freeze({zh:'设计与实现 · Lintel',en:'Design and implementation · Lintel'}),
  releases: Object.freeze({zh:'版本状态 · Lintel',en:'Release status · Lintel'}),
});
const metadataDescriptions = Object.freeze({
  download:Object.freeze({zh:'Lintel 安装信息：核对公开下载记录与平台，或从源码试用 macOS App 与独立 CLI。',en:'Lintel installation: check public download records and platforms, or try the macOS App and standalone CLI from source.'}),
  guide:Object.freeze({zh:'Lintel 指南：第一次使用、基本概念、排错与六任务操作路径，正文来自维护中的仓库文档。',en:'Lintel guides for first use, concepts, troubleshooting and six practical tasks, projected from maintained repository documents.'}),
  inside:Object.freeze({zh:'Lintel 的设计与实现：一个 core、分明的执行路径、冻结计划、原任务记录与可独立阅读的工作包。',en:'Lintel design and implementation: one core, distinct execution paths, frozen plans, original tasks and independently readable work packages.'}),
  releases:Object.freeze({zh:'Lintel 版本状态：区分源码、本地候选、公开字节核对与正式发行。',en:'Lintel version status separating source, local candidates, public-byte verification and formal release.'}),
});

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
  if (route && metadataTitles[route.key]) {
    const title = escapeHtml(metadataTitles[route.key][language]);
    const description = escapeHtml(metadataDescriptions[route.key][language]);
    html = html.replace(/<title>[^<]*<\/title>/, `<title>${title}</title>`)
      .replace(/<meta name="description"[^>]*>/, `<meta name="description" content="${description}">`)
      .replace(/<meta property="og:title"[^>]*>/, `<meta property="og:title" content="${title}">`)
      .replace(/<meta property="og:description"[^>]*>/, `<meta property="og:description" content="${description}">`);
  }
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
    .replace('<meta property="og:title" content="使用指南 · Lintel">', `<meta property="og:title" content="${escapeHtml(title)} · Lintel">`)
    .replace('<meta property="og:description" content="Lintel 指南文章。">', `<meta property="og:description" content="${escapeHtml(summary)}">`)
    .replace('property="og:locale" content="zh_CN"', `property="og:locale" content="${language === 'en' ? 'en_US' : 'zh_CN'}"`)
    .replace(/<title[^>]*>[\s\S]*?<\/title>/, `<title>${escapeHtml(title)} · Lintel</title>`)
    .replace('data-guide-document="start-here"', `data-guide-document="${escapeHtml(article.doc)}"`)
    .replace(`<a href="${enPath}" lang="en"`, `<a href="${localePath(article.path,'en')}" lang="en"`)
    .replace('<html lang="zh-CN"', `<html lang="${language === 'en' ? 'en' : 'zh-CN'}"`)
    .replace('<meta name="description" content="Lintel 指南文章。">', `<meta name="description" content="${escapeHtml(summary)}">`)
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
  return projectRouteHtml(html, language, { path: article.path });
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
  if (!html.includes(`rel="canonical" href="${expected}"`)) throw new Error(`Canonical mismatch for ${route.key}/${language}`);
  if (!html.includes(`property="og:url" content="${expected}"`)) throw new Error(`OG URL mismatch for ${route.key}/${language}`);
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
  const homeSource = await readFile(path.join(site,'index.html'),'utf8');
  const homeEnglish = projectHomeHtml(homeSource);
  const generated = await routeSources();
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
  return {
    languages:['zh-CN','en'],
    phrases:Object.keys(SITE_COPY).length,
    routePhrases:Object.keys(ROUTE_COPY).length,
    routes:SITE_ROUTES.length,
    guides:GUIDE_ARTICLES.length,
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

// Finite source/generation checks are also imported by the canonical site-ui journey.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { SITE_COPY, ENGLISH_GUIDES } from '../apps/site/site-copy.mjs';
import { checkSiteLanguages, projectSiteHtml } from '../scripts/prepare-site-languages.mjs';
import { applySiteLanguage, siteGuideHref, siteRouteHref, siteLanguageFromPath } from '../apps/site/site-language.mjs';
import { GUIDE_ARTICLES, SITE_ROUTES, SITE_ORIGIN, SOCIAL_IMAGE_PATH, localePath, homeStructuredData, serializeStructuredData } from '../apps/site/site-routes.mjs';
import { renderGuideMarkdown } from '../apps/site/site-docs.mjs';
const source = await readFile(new URL('../apps/site/index.html',import.meta.url),'utf8');
const english = await readFile(new URL('../apps/site/en/index.html',import.meta.url),'utf8');

test('paired copy generates exactly the tracked no-JS English page', async () => {
  assert.deepEqual((await checkSiteLanguages()).languages,['zh-CN','en']);
  assert.equal(english,projectSiteHtml(source));
  for (const value of Object.values(SITE_COPY)) {
    assert.equal(typeof value.zh,'string'); assert.ok(value.zh.length);
    assert.equal(typeof value.en,'string'); assert.ok(value.en.length);
  }
  for (const [html,lang,url] of [[source,'zh-CN','https://lintel.page/'],[english,'en','https://lintel.page/en/']]) {
    assert.match(html,new RegExp(`<html lang="${lang}"`));
    assert.ok(html.includes(`rel="canonical" href="${url}"`));
    assert.ok(html.includes(`property="og:url" content="${url}"`));
    for (const language of ['zh-CN','en','x-default']) assert.ok(html.includes(`hreflang="${language}"`));
    assert.equal([...html.matchAll(/<article data-capability=/g)].length,6);
    assert.equal([...html.matchAll(/<svg class="capability-icon"[^>]*aria-hidden="true"/g)].length,6);
    assert.equal([...html.matchAll(/<figure class="capability-visual">/g)].length,6, 'small title icons and explanatory scenes coexist');
    assert.ok(html.includes('preload="none"')); assert.ok(!html.includes('autoplay'));
    assert.ok(!/(?:href|src|poster)="\.\//.test(html),'shared asset paths resolve at both locale URLs');
  }
});

test('source changes reject missing keys, stale pairs and uncovered Chinese copy', () => {
  assert.throws(()=>projectSiteHtml(source.replace('data-site-copy="c004"','data-site-copy="missing"')),/Missing or stale/);
  assert.throws(()=>projectSiteHtml(source.replace('跳到正文</span>','跳到新的正文</span>')),/Missing or stale/);
  assert.throws(()=>projectSiteHtml(source.replace('</main>','<p>新的未配对文案</p></main>')),/Unbound Chinese/);
  assert.throws(()=>projectSiteHtml(source,'fr'),/Unsupported website language/);
});

test('identity drawings and synthetic originals stay byte-identical across locales', () => {
  const blocks = html => [...html.matchAll(/<!-- lintel-(?:mark|wordmark):start -->([\s\S]*?)<!-- lintel-(?:mark|wordmark):end -->/g)].map(match=>match[1]);
  assert.deepEqual(blocks(english),blocks(source));
  const originals = html => [...html.matchAll(/<pre>([\s\S]*?)<\/pre>/g)].map(match=>match[1]);
  assert.deepEqual(originals(english),originals(source));
  assert.match(english,/Current capabilities and limits \(Chinese\)/);
  assert.match(english,/No formal download/);
});

test('language and guide routing are finite and preserve stable section anchors', async () => {
  assert.equal(siteLanguageFromPath('/'),'zh'); assert.equal(siteLanguageFromPath('/en/'),'en');
  assert.equal(siteLanguageFromPath('/english'),'zh');
  const base='https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/';
  for (const guide of ENGLISH_GUIDES) {
    assert.equal(siteGuideHref(`${base}${guide}.md#work`,'en'),`${base}${guide}.en.md#work`);
    assert.equal(siteGuideHref(`${base}${guide}.en.md#work`,'zh'),`${base}${guide}.md#work`);
    await readFile(new URL(`../docs/${guide}.en.md`,import.meta.url),'utf8');
  }
  assert.equal(siteGuideHref(`${base}current-state.md`,'en'),`${base}current-state.md`);
  assert.equal(siteGuideHref('https://example.com/operator-guide.md','en'),'https://example.com/operator-guide.md');
  const operator = await readFile(new URL('../docs/operator-guide.en.md',import.meta.url),'utf8');
  for (const anchor of ['protect','work','cleanup','recovery']) assert.ok(operator.includes(`<a id="${anchor}"></a>`),`English operator guide retains #${anchor}`);
});

test('actual home language application completes metadata without an undeclared route scope', () => {
  const previousLocation = globalThis.location;
  globalThis.location = { pathname: '/' };
  const canonical = new Map(), ogUrl = new Map(), locale = new Map(), alternate = new Map();
  const node = attributes => ({ setAttribute: (name, value) => attributes.set(name, value) });
  const html = { lang: '', dataset: {} };
  const structuredData = { textContent: '' };
  const root = {
    documentElement: html,
    querySelectorAll: () => [],
    querySelector: selector => ({
      'link[rel="canonical"]': node(canonical),
      'meta[property="og:url"]': node(ogUrl),
      'meta[property="og:locale"]': node(locale),
      'meta[property="og:locale:alternate"]': node(alternate),
      '#site-structured-data': structuredData,
    })[selector] ?? null,
  };
  try {
    assert.equal(applySiteLanguage('en', root), 'en');
    assert.equal(html.lang, 'en');
    assert.equal(html.dataset.language, 'en');
    assert.equal(canonical.get('href'), 'https://lintel.page/en/');
    assert.equal(ogUrl.get('content'), 'https://lintel.page/en/');
    assert.equal(locale.get('content'), 'en_US');
    assert.equal(alternate.get('content'), 'zh_CN');
    const englishData = JSON.parse(structuredData.textContent);
    assert.equal(englishData['@graph'][1].url, 'https://lintel.page/en/');
    assert.equal(englishData['@graph'][1].inLanguage, 'en');
    assert.equal(englishData['@graph'][1].name, SITE_COPY.c002.en);
    applySiteLanguage('zh', root);
    const chineseData = JSON.parse(structuredData.textContent);
    assert.equal(chineseData['@graph'][1].url, 'https://lintel.page/');
    assert.equal(chineseData['@graph'][1].description, SITE_COPY.c001.zh);
    assert.deepEqual(chineseData['@graph'][0], englishData['@graph'][0], 'both locales retain one site identity');
  } finally {
    if (previousLocation === undefined) delete globalThis.location;
    else globalThis.location = previousLocation;
  }
});

test('home schema projects only actual site and localized page facts with safe script bytes', async () => {
  for (const [html,language,url] of [[source,'zh',`${SITE_ORIGIN}/`],[english,'en',`${SITE_ORIGIN}/en/`]]) {
    const scripts = [...html.matchAll(/<script\b[^>]*type="application\/ld\+json"[^>]*>([\s\S]*?)<\/script>/g)];
    assert.equal(scripts.length, 1);
    const data = JSON.parse(scripts[0][1]);
    assert.deepEqual(data, homeStructuredData(language));
    assert.deepEqual(data['@graph'].map(node => node['@type']), ['WebSite','WebPage']);
    assert.equal(data['@graph'][0].name, 'Lintel');
    assert.equal(data['@graph'][0].url, `${SITE_ORIGIN}/`);
    assert.equal(data['@graph'][1].url, url);
    assert.equal(data['@graph'][1].name, SITE_COPY.c002[language]);
    assert.equal(data['@graph'][1].description, SITE_COPY.c001[language]);
    assert.doesNotMatch(scripts[0][1], /aggregateRating|offers|softwareVersion|license|datePublished/);
  }
  const unsafe = {name:'</script><script>alert(1)</script>'};
  assert.doesNotMatch(serializeStructuredData(unsafe), /</);
  assert.deepEqual(JSON.parse(serializeStructuredData(unsafe)), unsafe);
  assert.deepEqual(await readFile(new URL('../apps/site/assets/favicon.png',import.meta.url)),
    await readFile(new URL('../apps/desktop/src-tauri/icons/128x128.png',import.meta.url)), 'favicon reuses the supplied identity export byte for byte');
});

test('route map has exact locale mirrors, metadata and sitemap coverage', async () => {
  assert.equal(SITE_ROUTES.length,20);
  assert.equal(GUIDE_ARTICLES.length,15);
  assert.equal(new Set(SITE_ROUTES.map(route=>route.key)).size,SITE_ROUTES.length);
  assert.equal(new Set(SITE_ROUTES.map(route=>route.path)).size,SITE_ROUTES.length);
  const sitemap=await readFile(new URL('../apps/site/sitemap.xml',import.meta.url),'utf8');
  const titlesByLanguage = new Map([['zh', new Set()], ['en', new Set()]]);
  const descriptionsByLanguage = new Map([['zh', new Set()], ['en', new Set()]]);
  for(const route of SITE_ROUTES){
    for(const language of ['zh','en']){
      const routePath=localePath(route.path,language);
      const file=language==='en'
        ? new URL(`../apps/site/en${route.path}index.html`,import.meta.url)
        : new URL(`../apps/site${route.path}index.html`,import.meta.url);
      const html=await readFile(file,'utf8');
      const canonical=`https://lintel.page${routePath}`;
      assert.ok(html.includes(`rel="canonical" href="${canonical}"`),`${routePath} canonical`);
      assert.ok(html.includes(`property="og:url" content="${canonical}"`),`${routePath} OG URL`);
      assert.ok(html.includes(`hreflang="zh-CN" href="https://lintel.page${localePath(route.path,'zh')}"`),`${routePath} zh hreflang`);
      assert.ok(html.includes(`hreflang="en" href="https://lintel.page${localePath(route.path,'en')}"`),`${routePath} en hreflang`);
      assert.ok(html.includes(`<html lang="${language==='en'?'en':'zh-CN'}"`),`${routePath} document language`);
      assert.ok(sitemap.includes(`<loc>${canonical}</loc>`),`${routePath} sitemap`);
      assert.ok(html.includes(`<a href="${localePath(route.path,'zh')}" lang="zh-CN" hreflang="zh-CN"`),`${routePath} Chinese switch target`);
      assert.ok(html.includes(`<a href="${localePath(route.path,'en')}" lang="en" hreflang="en"`),`${routePath} English switch target`);
      assert.equal(html.includes('\0'),false,`${routePath} contains no NUL placeholder`);
      if(route.key!=='home'){
        const switches=[...html.matchAll(/<a\b[^>]*data-site-route-language="(zh|en)"[^>]*>/g)];
        assert.equal(switches.length,2,`${routePath} has two language choices`);
        for(const [tag,targetLanguage] of switches)assert.equal(tag.includes('aria-current="page"'),targetLanguage===language,`${routePath} announces only the actual locale`);
      }
      const title=/<title[^>]*>([^<]*)<\/title>/.exec(html)?.[1];
      const ogTitle=/<meta property="og:title" content="([^"]*)"/.exec(html)?.[1];
      assert.equal(ogTitle,title,`${routePath} share title matches the page language`);
      const description=html.match(/<meta name="description" content="([^"]*)"/)?.[1];
      const ogDescription=html.match(/<meta property="og:description" content="([^"]*)"/)?.[1];
      const twitterTitle=html.match(/<meta name="twitter:title" content="([^"]*)"/)?.[1];
      const twitterDescription=html.match(/<meta name="twitter:description" content="([^"]*)"/)?.[1];
      assert.equal(description,ogDescription,`${routePath} page and share descriptions agree`);
      assert.equal(twitterDescription,description,`${routePath} Twitter description agrees`);
      assert.equal(twitterTitle,title,`${routePath} Twitter title agrees`);
      assert.ok(html.includes('<meta property="og:site_name" content="Lintel">'),`${routePath} site name`);
      assert.ok(html.includes(`<meta property="og:image" content="${SITE_ORIGIN}${SOCIAL_IMAGE_PATH}">`),`${routePath} share image`);
      assert.ok(html.includes(`<meta name="twitter:image" content="${SITE_ORIGIN}${SOCIAL_IMAGE_PATH}">`),`${routePath} Twitter image`);
      assert.equal((html.match(/<meta property="og:image:alt" content="([^"]+)"[^>]*>/)?.[1] || '').length>0,true,`${routePath} share image alt`);
      assert.equal((html.match(/<meta name="twitter:image:alt" content="([^"]+)"[^>]*>/)?.[1] || '').length>0,true,`${routePath} Twitter image alt`);
      const noindex=html.match(/<meta name="robots" content="([^"]*)"/)?.[1];
      assert.equal(noindex,undefined,`${routePath} is indexable`);
      if (route.key !== 'home') assert.doesNotMatch(html, /application\/ld\+json/,`${routePath} adds no unsupported schema`);
      assert.match(html, /<link rel="icon" type="image\/png" sizes="128x128" href="\/assets\/favicon.png">/,`${routePath} has a crawlable raster favicon`);
      if(language==='en') for(const content of [description,ogDescription,twitterDescription]) assert.doesNotMatch(content,/[\u4e00-\u9fff]/,`${routePath} descriptions are English`);
      assert.ok(title?.trim(),`${routePath} has a descriptive title`);
      assert.ok(description?.trim(),`${routePath} has a useful description`);
      titlesByLanguage.get(language).add(title);
      descriptionsByLanguage.get(language).add(description);
    }
  }
  for (const language of ['zh','en']) {
    assert.equal(titlesByLanguage.get(language).size,SITE_ROUTES.length,`${language} titles are unique`);
    assert.equal(descriptionsByLanguage.get(language).size,SITE_ROUTES.length,`${language} descriptions are unique`);
  }
  for (const article of GUIDE_ARTICLES) {
    for (const language of ['zh','en']) {
      const html=await readFile(new URL(`../apps/site${language==='en'?`/en${article.path}`:article.path}index.html`,import.meta.url),'utf8');
      assert.equal(html.includes(`<meta name="description" content="${article.summary[language]}">`),true,`${article.doc}/${language} description comes from paired guide summary`);
    }
  }
  const sitemapUrls=[...sitemap.matchAll(/<loc>([^<]+)<\/loc>/g)].map(match=>match[1]);
  assert.equal(sitemapUrls.length,40);
  assert.equal(new Set(sitemapUrls).size,40);
  for (const block of [...sitemap.matchAll(/<url>(.*?)<\/url>/g)].map(match=>match[1])) {
    const location=/<loc>([^<]+)<\/loc>/.exec(block)?.[1];
    assert.equal((block.match(/<xhtml:link /g)||[]).length,3,`${location} has Chinese, English and default alternates`);
    for (const language of ['zh-CN','en','x-default']) assert.match(block,new RegExp(`hreflang="${language}"`),`${location} ${language}`);
  }
});

test('guide articles project maintained paired bodies through the finite map', async () => {
  for(const article of GUIDE_ARTICLES){
    const routePath=article.path;
    const zh=await readFile(new URL(`../apps/site${routePath}index.html`,import.meta.url),'utf8');
    const en=await readFile(new URL(`../apps/site/en${routePath}index.html`,import.meta.url),'utf8');
    const zhSource=await readFile(new URL(`../docs/${article.doc}.md`,import.meta.url),'utf8');
    const enSource=await readFile(new URL(`../docs/${article.doc}.en.md`,import.meta.url),'utf8');
    for(const [html,source,language] of [[zh,zhSource,'zh'],[en,enSource,'en']]){
      assert.ok(html.includes(renderGuideMarkdown(source,language).slice(0,400)),`${article.key}/${language} projected body`);
      assert.ok(html.includes(`data-guide-document="${article.doc}"`),`${article.key}/${language} source identity`);
      assert.ok(html.includes(`${article.title[language]} · Lintel`),`${article.key}/${language} title`);
      assert.ok(html.includes('/guide/'),`${article.key}/${language} guide navigation`);
      assert.ok(!html.includes('/Users/'),`${article.key}/${language} no absolute workstation path`);
    }
  }
});

test('Markdown projection escapes HTML and rejects unsupported blocks', () => {
  assert.equal(renderGuideMarkdown('A <script>alert(1)</script> value.','en'),'<p>A alert(1) value.</p>');
  assert.equal(renderGuideMarkdown('`<script>` and **safe**','en'),'<p><code>&lt;script&gt;</code> and <strong>safe</strong></p>');
  assert.match(renderGuideMarkdown('| A | B |\n| --- | --- |\n| `x|y` | z |','en'),/<td.*><code>x\|y<\/code><\/td>/s);
  assert.match(renderGuideMarkdown('[Start](start-here.md#choose)','en'),/href="\/en\/guide\/start-here\/#choose"/);
  assert.match(renderGuideMarkdown('[Code](https://example.com)','en'),/href="https:\/\/example.com"/);
  assert.throws(()=>renderGuideMarkdown('<div>unsafe</div>'),/Unsupported Markdown block/);
  assert.throws(()=>renderGuideMarkdown('~~~txt\nx\n~~~'),/Tilde fences/);
  assert.throws(()=>renderGuideMarkdown('```sh\nunclosed'),/Unclosed Markdown fence/);
});

test('Markdown headings retain readable unique Chinese IDs outside code', () => {
  assert.equal(renderGuideMarkdown('## 中文标题','zh'),'<h2 id="中文标题">中文标题</h2>');
  const headings=renderGuideMarkdown('## 中文标题\n\n## 中文标题','zh');
  assert.match(headings,/<h2 id="中文标题">中文标题<\/h2>/);
  assert.match(headings,/<h2 id="中文标题-2">中文标题<\/h2>/);
  assert.doesNotMatch(headings,/\\{#/);
});

test('Markdown links resolve inline code labels without leaking placeholders', () => {
  const html=renderGuideMarkdown('[`literal`](core.md)','zh');
  assert.equal(html,'<p><a href="/guide/core/"><code>literal</code></a></p>');
  assert.doesNotMatch(html,/[\uE000-\uF8FF]/);
});

test('Markdown fenced code remains byte-exact and never gains heading IDs', () => {
  const source='```sh\n# comment stays exact\necho ok\n```';
  assert.equal(renderGuideMarkdown(source,'zh'),
    '<pre class="guide-code" data-language="sh"><code># comment stays exact\necho ok</code></pre>');
});

// Exact local guide routes keep the same approved task/section in both editions.
test('home routes local task guides without rewriting language-switch roots', () => {
  assert.equal(siteRouteHref('/guide/operator-guide/#work','en'),'/en/guide/operator-guide/#work');
  assert.equal(siteRouteHref('/en/guide/browser/?source=home#install','zh'),'/guide/browser/?source=home#install');
  assert.equal(siteRouteHref('/en/','zh'),'/en/');
  assert.equal(siteRouteHref('/unknown/','en'),'/unknown/');
  for(const [html,prefix] of [[source,''],[english,'/en']]){
    for(const route of ['/download/','/guide/','/inside/','/releases/'])assert.ok(html.includes(`href="${prefix}${route}"`));
    assert.ok(html.includes(`href="${prefix}/guide/operator-guide/#work"`));
  }
});

test('maintained guide fragments and locale destinations resolve across all generated pages', async () => {
  const pages=new Map();
  for(const route of SITE_ROUTES)for(const language of ['zh','en']){
    const pathname=localePath(route.path,language);
    const html=await readFile(new URL(`../apps/site${pathname}index.html`,import.meta.url),'utf8');
    pages.set(pathname,{html,ids:new Set([...html.matchAll(/\sid="([^"]*)"/g)].map(m=>m[1]))});
  }
  for(const [pathname,{html}] of pages){
    for(const [,href]of html.matchAll(/<a\b[^>]*\shref="([^"]+)"/g)){
      const url=new URL(href,`https://lintel.page${pathname}`);
      if(url.origin!=='https://lintel.page'||!url.hash||!pages.has(url.pathname))continue;
      assert.ok(pages.get(url.pathname).ids.has(decodeURIComponent(url.hash.slice(1))),`${pathname} resolves ${href}`);
    }
    if(pathname.startsWith('/en/')&&pathname!=='/en/'){
      assert.ok(html.includes('href="/en/" data-route-copy="product"'),'English product navigation returns to English home');
      assert.doesNotMatch(html,/href="\/guide\/[^"#]+#[^"]*"/,'English article fragments retain the English destination');
    }
  }
  assert.match(renderGuideMarkdown('## 7. inspect / read in a separate independent state','en'),/id="7-inspect--read-in-a-separate-independent-state"/);
});

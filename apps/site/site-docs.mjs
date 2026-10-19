// Narrow, dependency-free Markdown projection for maintained repository guides.
//
// Supported deliberately: ATX headings, paragraphs, ordered/unordered lists,
// blockquotes, fenced code, GFM tables, inline code, strong text and links.
// Unsupported structure fails generation instead of shipping an ambiguous page.

import { GUIDE_ARTICLES, localizedRoute } from './site-routes.mjs';

const articlesByDoc = new Map(GUIDE_ARTICLES.map(article => [article.doc, article]));
const decodeEntities = value => value.replace(/&(amp|lt|gt|quot|#39);/g,(_,c)=>({
  amp:'&', lt:'<', gt:'>', quot:'"', '#39':"'",
}[c]));
const escapeHtml = value => String(value).replace(/[&<>"']/g,c=>({
  '&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;',
}[c]));

function plainHeading(value) {
  return decodeEntities(value
    .replace(/`([^`]+)`/g,'$1')
    .replace(/\[([^\]]+)\]\([^)]+\)/g,'$1')
    .replace(/\*\*([^*]+)\*\*/g,'$1')
    .trim());
}

function slug(value) {
  const text = plainHeading(value)
    .toLowerCase()
    .replace(/[^\p{Letter}\p{Mark}\p{Number}\s_-]/gu,'')
    .replace(/\s/g,'-');
  return text || 'section';
}

function localDocHref(href, language) {
  if (/^(?:[a-z]+:|#|\/)/i.test(href)) return href;
  const [target, fragment = ''] = href.split('#', 2);
  const [docBase] = target.split('/');
  const match = /^([a-z0-9-]+)(?:\.en)?\.md$/.exec(docBase);
  if (match && articlesByDoc.has(match[1])) {
    const linked = articlesByDoc.get(match[1]);
    return `${localizedRoute(`guide:${linked.key}`, language)}${fragment ? `#${fragment}` : ''}`;
  }
  const repositoryPath = target.startsWith('../')
    ? target.slice(3)
    : `docs/${target}`;
  return `https://github.com/IndelibleVivi/lintel-cc/blob/main/${repositoryPath}${fragment ? `#${fragment}` : ''}`;
}

function inline(value, language) {
  const tokens = [];
  const hold = markup => `\uE000${tokens.push(markup) - 1}\uE001`;
  let text = value;
  const code = [];
  text = text.replace(/`([^`\n]+)`/g, (_, body) => `\uE002${code.push(escapeHtml(body)) - 1}\uE003`);
  text = text.replace(/!\[([^\]]*)\]\(([^)\s]+)\)/g, (_, alt, href) =>
    hold(`<a class="guide-image-fallback" href="${localDocHref(href, language)}">${escapeHtml(alt)}</a>`));
  text = text.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_, label, href) =>
    hold(`<a href="${localDocHref(href, language)}">${inline(label.replace(/\uE002(\d+)\uE003/g, (_, index) => '`' + code[Number(index)] + '`'), language)}</a>`));
  text = text.replace(/<\/?[a-zA-Z][^>]*>/g, '');
  text = text.replace(/\uE002(\d+)\uE003/g, (_, index) => hold(`<code>${code[Number(index)]}</code>`));
  text = text.replace(/\*\*([^*\n]+)\*\*/g, (_, strong) => hold(`<strong>${escapeHtml(strong)}</strong>`));
  text = text.replace(/\*([^*\n]+)\*/g, (_, em) => hold(`<em>${escapeHtml(em)}</em>`));
  text = escapeHtml(text);
  const token = /\uE000(\d+)\uE001/g;
  for (let pass = 0; pass <= tokens.length; pass++) {
    let changed = false;
    text = text.replace(token, (_, index) => {
      changed = true;
      return tokens[Number(index)];
    });
    if (!changed) break;
  }
  if (token.test(text)) throw new Error('Unresolved inline Markdown placeholder');
  return text;
}

function tableCells(line) {
  const trimmed = line.trim().replace(/^\|/, '').replace(/\|$/, '');
  const cells = [];
  let current = '', code = false;
  for (const char of trimmed) {
    if (char === '`') code = !code;
    if (char === '|' && !code) { cells.push(current.trim()); current = ''; }
    else current += char;
  }
  cells.push(current.trim());
  return cells;
}

function isTableStart(lines, index) {
  return index + 1 < lines.length
    && /^\s*\|?.+\|.*$/.test(lines[index])
    && /^\s*\|?\s*:?-{3,}:?\s*(?:\|\s*:?-{3,}:?\s*)+\|?\s*$/.test(lines[index + 1]);
}

function renderList(lines, start, ordered, language) {
  const marker = ordered ? /^\s*\d+\.\s+(.+)$/ : /^\s*[-*]\s+(.+)$/;
  const items = [];
  let index = start;
  while (index < lines.length) {
    const match = marker.exec(lines[index]);
    if (!match) break;
    items.push(`<li>${inline(match[1], language)}</li>`);
    index++;
  }
  const tag = ordered ? 'ol' : 'ul';
  return { html:`<${tag}>${items.join('')}</${tag}>`, next:index };
}

export function renderGuideMarkdown(markdown, language = 'zh') {
  const lines = markdown.replace(/\r\n?/g,'\n').split('\n');
  const html = [];
  const headingIds = new Map();
  let index = 0;
  while (index < lines.length) {
    const line = lines[index];
    if (!line.trim()) { index++; continue; }

    const fence = /^\s*```([^`]*)$/.exec(line);
    if (fence) {
      const languageName = fence[1].trim().split(/[,\s]/)[0] || 'text';
      const body = [];
      index++;
      while (index < lines.length && !/^\s*```\s*$/.test(lines[index])) body.push(lines[index++]);
      if (index >= lines.length) throw new Error('Unclosed Markdown fence');
      index++;
      html.push(`<pre class="guide-code" data-language="${escapeHtml(languageName)}"><code>${escapeHtml(body.join('\n'))}</code></pre>`);
      continue;
    }
    if (/^\s*~~~/.test(line)) throw new Error('Tilde fences are unsupported');

    const heading = /^(#{1,6})\s+(.+?)\s*$/.exec(line);
    if (heading) {
      const level = heading[1].length;
      const titled = /^(.*?)\s*\{#([^{}]+)\}$/i.exec(heading[2]);
      const title = titled ? titled[1] : heading[2];
      const base = titled ? titled[2].trim() : slug(title);
      const count = (headingIds.get(base) || 0) + 1;
      headingIds.set(base, count);
      const id = count === 1 ? base : `${base}-${count}`;
      html.push(`<h${level} id="${escapeHtml(id)}">${inline(title, language)}</h${level}>`);
      index++;
      continue;
    }

    if (/^<a\s+id="[^"]+"><\/a>\s*$/.test(line.trim())) {
      html.push(line.trim());
      index++;
      continue;
    }

    if (isTableStart(lines, index)) {
      const head = tableCells(lines[index]);
      const align = tableCells(lines[index + 1]).map(cell =>
        cell.startsWith(':') && cell.endsWith(':') ? 'center'
          : cell.endsWith(':') ? 'right' : 'left');
      index += 2;
      const rows = [];
      while (index < lines.length && /^\s*\|/.test(lines[index])) {
        rows.push(tableCells(lines[index++]));
      }
      html.push(`<div class="guide-table-wrap"><table><thead><tr>${head.map((cell,i) =>
        `<th class="align-${align[i] || 'left'}">${inline(cell, language)}</th>`).join('')}</tr></thead><tbody>${
        rows.map(row => `<tr>${head.map((_,i) =>
          `<td class="align-${align[i] || 'left'}">${inline(row[i] || '', language)}</td>`).join('')}</tr>`).join('')
      }</tbody></table></div>`);
      continue;
    }

    if (/^\s*>/.test(line)) {
      const body = [];
      while (index < lines.length && /^\s*>/.test(lines[index])) {
        body.push(lines[index++].replace(/^\s*>\s?/,''));
      }
      html.push(`<blockquote><p>${inline(body.join(' '), language)}</p></blockquote>`);
      continue;
    }
    if (/^\s*\d+\.\s+/.test(line)) {
      const list = renderList(lines, index, true, language);
      html.push(list.html); index = list.next; continue;
    }
    if (/^\s*[-*]\s+/.test(line)) {
      const list = renderList(lines, index, false, language);
      html.push(list.html); index = list.next; continue;
    }
    if (/^\s*---\s*$/.test(line)) { html.push('<hr>'); index++; continue; }
    const image = /^\s*!\[([^\]]*)\]\(([^)\s]+)\)\s*$/.exec(line);
    if (image) {
      html.push(`<p><a class="guide-image-fallback" href="${localDocHref(image[2], language)}">${escapeHtml(image[1])}</a></p>`);
      index++;
      continue;
    }
    if (/^\s*(?:<img\b|<svg\b|<div\b|<table\b)/i.test(line)) {
      throw new Error(`Unsupported Markdown block: ${line.trim().slice(0,80)}`);
    }

    const paragraph = [];
    while (index < lines.length && lines[index].trim()) {
      const next = lines[index];
      if (/^(#{1,6})\s+/.test(next) || /^\s*```/.test(next) || /^\s*>/.test(next)
          || /^\s*[-*]\s+/.test(next) || /^\s*\d+\.\s+/.test(next)
          || isTableStart(lines, index) || /^\s*---\s*$/.test(next)) break;
      paragraph.push(next.trim());
      index++;
    }
    if (!paragraph.length) throw new Error(`Unsupported Markdown line: ${line.trim().slice(0,80)}`);
    html.push(`<p>${inline(paragraph.join(' '), language)}</p>`);
  }
  return html.join('\n');
}

export { escapeHtml, inline as renderGuideInline };

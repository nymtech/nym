// Anchor check for stage 8 (task 9.12): each link with a `#anchor` on a docs
// page must name a heading id of the page it goes to, as Nextra 4.6.1 makes
// the ids. A step heading change can break such a link on another page.
//
//   node lib/archetypes/anchors.mjs [--list] [--under <dir under content>]
//
// It reads each page with its imported `.mdx` files in place, because a partial
// shares the anchor space of the page that imports it. The count before and
// after an edit must not go up. A link to a route with no page (a redirect, an
// API route) is counted apart: this check cannot see its anchors.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import GithubSlugger from 'github-slugger';
import { parseFrontmatter, inlineMdxPartials, fenceTracker, HEADING_ANCHOR } from '../retrieval/mdx.mjs';
import { openRepo } from '../snippets/repo.mjs';

const DOCS_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const CONTENT = path.join(DOCS_ROOT, 'content');
const BASE_PATH = '/docs';

/** Each `.md` and `.mdx` page under the content directory, by route. */
export function pagesByRoute(dir = CONTENT) {
  const out = new Map();
  const walk = (rel) => {
    for (const e of fs.readdirSync(path.join(dir, rel), { withFileTypes: true })) {
      const r = path.join(rel, e.name);
      if (e.isDirectory()) walk(r);
      else if (/\.mdx?$/.test(e.name)) out.set(r.replace(/\.mdx?$/, '').replace(/(^|\/)index$/, ''), r);
    }
  };
  walk('');
  return out;
}

/** Heading text as Nextra flattens it for the id: no code marks, emphasis, or link targets. */
const flatten = (text) => text.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1').replace(/[`*]/g, '').trim();

/**
 * The ids of one page text, in the Nextra order: one slugger for the page, an
 * explicit `[#anchor]` goes through it too, and the title (depth 1) has none.
 * An `id="..."` attribute on an element is an anchor as well.
 */
export function anchorsOf(text) {
  const slugger = new GithubSlugger();
  const ids = new Set();
  const fence = fenceTracker();
  for (const line of text.split('\n')) {
    if (fence(line)) continue;
    const h = line.match(/^\s*(#{2,6})\s+(.+?)\s*#*\s*$/);
    if (h) {
      const explicit = h[2].match(HEADING_ANCHOR);
      ids.add(slugger.slug(explicit ? explicit[1] ?? explicit[2] : flatten(h[2])));
      continue;
    }
    for (const m of line.matchAll(/\bid=["']([^"']+)["']/g)) ids.add(m[1]);
  }
  return ids;
}

/** Each link with an anchor in a page text: [{ line, href }]. */
export function anchorLinks(text) {
  const out = [];
  const fence = fenceTracker();
  text.split('\n').forEach((line, i) => {
    if (fence(line)) return;
    const prose = line.replace(/`[^`]*`/g, '');
    for (const m of prose.matchAll(/\]\(([^)\s]*#[^)\s]+)\)|href=["']([^"']*#[^"']+)["']/g)) {
      const href = m[1] ?? m[2];
      if (!/^[a-z][a-z0-9+.-]*:/i.test(href)) out.push({ line: i + 1, href });
    }
  });
  return out;
}

/** The route and anchor a link from `route` goes to, as the browser resolves it. */
export function resolveLink(route, href) {
  const base = new URL(`http://docs${BASE_PATH}/${route}`);
  const url = new URL(href, base);
  let target = decodeURIComponent(url.pathname);
  if (target.startsWith(`${BASE_PATH}/`) || target === BASE_PATH) target = target.slice(BASE_PATH.length);
  target = target.replace(/\.mdx?$/, '').replace(/\/$/, '').replace(/(^|\/)index$/, '').replace(/^\//, '');
  return { route: target, anchor: decodeURIComponent(url.hash.slice(1)) };
}

export function checkAnchors({ under = '' } = {}) {
  const repo = openRepo();
  const pages = pagesByRoute();
  const texts = new Map();
  const textOf = (route) => {
    if (!texts.has(route)) {
      const file = path.join(CONTENT, pages.get(route));
      const { content } = parseFrontmatter(fs.readFileSync(file, 'utf-8'));
      texts.set(route, inlineMdxPartials(content, { filePath: file, root: DOCS_ROOT, aliases: repo.aliases }));
    }
    return texts.get(route);
  };
  const broken = [];
  const noPage = [];
  let checked = 0;
  for (const [route, rel] of pages) {
    if (!rel.startsWith(under)) continue;
    for (const { line, href } of anchorLinks(textOf(route))) {
      const to = resolveLink(route, href);
      if (!pages.has(to.route)) { noPage.push({ page: rel, line, href }); continue; }
      checked++;
      if (!anchorsOf(textOf(to.route)).has(to.anchor)) broken.push({ page: rel, line, href });
    }
  }
  return { checked, broken, noPage };
}

function main(argv) {
  const at = (name) => { const i = argv.indexOf(`--${name}`); return i === -1 ? null : argv[i + 1]; };
  const { checked, broken, noPage } = checkAnchors({ under: at('under') ?? '' });
  if (argv.includes('--list')) {
    // The line is in the page text with its partials in place, not in the file.
    for (const b of broken) console.log(`broken   ${b.page} (inlined line ${b.line})  ${b.href}`);
    for (const n of noPage) console.log(`no page  ${n.page} (inlined line ${n.line})  ${n.href}`);
  }
  console.log(`${checked} anchor links to a page: ${broken.length} name no heading of that page; ${noPage.length} links go to a route with no page`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2));
}

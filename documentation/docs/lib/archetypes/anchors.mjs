// Anchor check for stage 8 (task 9.12): each link with a `#anchor` on a docs
// page must name an id on the page it goes to, as Nextra 4.6.1 makes the ids.
// A step heading change can break such a link on another page.
//
//   node lib/archetypes/anchors.mjs [--list] [--under <dir under content>]
//
// It reads each page with its imported `.mdx` and `.md` files in place (the
// mdast of inline.mjs), because a partial shows its ids in the page that
// imports it. Nextra compiles each file alone, so each file has its own
// slugger (lib/archetypes/node_modules/nextra remark-headings). The count
// before and after an edit must not go up. Two kinds of link are counted
// apart: a link to a route with no page (a redirect, an API route), and a link
// to a page that renders a TSX component with no children, whose ids this
// check cannot see ("unseen"). The gate is the sum of broken and unseen links.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import GithubSlugger from 'github-slugger';
import { inlinePage } from './inline.mjs';
import { walk, explicitAnchor } from './rules/util.mjs';
import { openRepo, modelCitation } from '../snippets/repo.mjs';
import { loadTsModule } from '../retrieval/ts-data.mjs';

const DOCS_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const CONTENT = path.join(DOCS_ROOT, 'content');
const BASE_PATH = '/docs';

/** Each `.md` and `.mdx` page under the content directory, by route. */
export function pagesByRoute(dir = CONTENT) {
  const out = new Map();
  const walkDir = (rel) => {
    for (const e of fs.readdirSync(path.join(dir, rel), { withFileTypes: true })) {
      const r = path.join(rel, e.name);
      if (e.isDirectory()) walkDir(r);
      else if (/\.mdx?$/.test(e.name)) out.set(r.replace(/\.mdx?$/, '').replace(/(^|\/)index$/, ''), r);
    }
  };
  walkDir('');
  return out;
}

/** Nextra's getFlattenedValue: the text and inline code of a node, nothing else. */
const flattened = (node) => (node.children ?? [])
  .map((child) => ('children' in child ? flattened(child) : 'value' in child ? child.value : ''))
  .join('');

const isJsx = (node) => node.type === 'mdxJsxFlowElement' || node.type === 'mdxJsxTextElement';
const attribute = (node, name) => node.attributes?.find((a) => a.type === 'mdxJsxAttribute' && a.name === name);

/** The label of the tab panel `node` in its `Tabs` parent, as Nextra reads `items`. */
function tabLabel(node, parent) {
  const items = parent?.name === 'Tabs' && attribute(parent, 'items');
  if (!items) return null;
  const elements = items.value?.data?.estree?.body[0]?.expression?.elements ?? [];
  return { label: elements.map((el) => el?.value)[parent.children.indexOf(node)] };
}

/**
 * The ids of a page tree, in the Nextra order. Each file of the tree (the page
 * and each partial) has its own slugger. A heading id is the slug of its
 * explicit `[#anchor]` or of its text; the title (depth 1) has none. A tab
 * panel gets the slug of its label, a `<summary>` in `<details>` the slug of
 * its text, and an element its literal `id` attribute. A citation of a whole kind
 * (`<Snippet id=".../threat-model/actor" view="reference" />`) adds the ids its
 * table makes, from `kindIds` (type -> ids; see modelKindIds).
 */
export function anchorsOf(tree, kindIds = {}) {
  const sluggers = new Map();
  const slug = (node, value) => {
    const file = node.data?.file ?? '';
    if (!sluggers.has(file)) sluggers.set(file, new GithubSlugger());
    return sluggers.get(file).slug(value);
  };
  const ids = new Set();
  walk(tree, (node, ancestors) => {
    if (node.type === 'heading') {
      if (node.depth > 1) ids.add(slug(node, explicitAnchor(node) ?? flattened(node).replace(/\s*\[#[^]+?]\s*$/, '')));
      return;
    }
    if (!isJsx(node)) return;
    if (node.name === 'Tabs.Tab') {
      const tab = tabLabel(node, ancestors.at(-1));
      if (tab) ids.add(slug(node, tab.label));
    }
    if (node.name === 'details') {
      walk(node, (inner) => { if (inner.type === 'mdxJsxTextElement' && inner.name === 'summary') ids.add(slug(inner, flattened(inner))); });
    }
    const id = attribute(node, 'id');
    if (node.name === 'Snippet' && typeof id?.value === 'string') {
      const model = modelCitation(id.value);
      if (model?.kind === 'collection') for (const k of kindIds[model.type] ?? []) ids.add(k);
      return;
    }
    if (typeof id?.value === 'string') ids.add(id.value);
  });
  return ids;
}

/** Each link with an anchor in a page tree: [{ file, line, href, markdown }]. */
export function anchorLinks(tree) {
  const out = [];
  walk(tree, (node) => {
    let href = null;
    let markdown = false;
    if (node.type === 'link') { href = node.url; markdown = true; }
    else if (isJsx(node) && typeof attribute(node, 'href')?.value === 'string') href = attribute(node, 'href').value;
    if (!href || !href.includes('#') || /^[a-z][a-z0-9+.-]*:/i.test(href)) return;
    out.push({ file: node.data?.file, line: node.position?.start.line, href, markdown });
  });
  return out;
}

const decode = (text) => { try { return decodeURIComponent(text); } catch { return text; } };

/**
 * The route and anchor a link from `route` goes to, as the browser resolves it.
 * Nextra removes `.md` and `.mdx` from a Markdown link (remark-link-rewrite),
 * not from an `href` attribute. A link that leaves the base path has no route.
 */
export function resolveLink(route, href, { markdown = true } = {}) {
  // Next adds the base path to an absolute link; a relative one resolves from the page URL.
  const absolute = href.startsWith('/') && !href.startsWith(`${BASE_PATH}/`) ? `${BASE_PATH}${href}` : href;
  const url = new URL(absolute, `http://docs${route ? `${BASE_PATH}/${route}` : BASE_PATH}`);
  let target = decode(url.pathname);
  if (!target.startsWith(`${BASE_PATH}/`) && target !== BASE_PATH) return { route: null, anchor: decode(url.hash.slice(1)) };
  target = target.slice(BASE_PATH.length);
  if (markdown) target = target.replace(/\.mdx?$/, '');
  target = target.replace(/\/$/, '').replace(/(^|\/)index$/, '').replace(/^\//, '');
  return { route: target, anchor: decode(url.hash.slice(1)) };
}

/**
 * The ids that the tables of the threat model make on the page
 * (components/threat-model/ThreatModelReference.tsx), by kind.
 */
export async function modelKindIds() {
  const tm = await loadTsModule(path.join(DOCS_ROOT, 'lib', 'privacy-model', 'threat-model.ts'));
  return {
    actor: ['actor-L1', ...tm.ACTORS.map((a) => `actor-${a.id}`)],
    vector: tm.VECTORS.map((v) => `vector-${v.id}`),
    property: tm.PROPERTIES.map((p) => `prop-${p.id}`),
  };
}

export async function checkAnchors({ under = '' } = {}) {
  const kindIds = await modelKindIds();
  const pages = pagesByRoute();
  const { aliases } = openRepo();
  const parsed = new Map();
  const pageOf = (route) => {
    if (!parsed.has(route)) {
      const { tree, unchecked } = inlinePage(path.join(CONTENT, pages.get(route)), { root: DOCS_ROOT, aliases });
      parsed.set(route, { tree, unchecked, ids: anchorsOf(tree, kindIds) });
    }
    return parsed.get(route);
  };
  const broken = [];
  const unseen = [];
  const noPage = [];
  let checked = 0;
  for (const [route, rel] of pages) {
    if (!rel.startsWith(under)) continue;
    for (const link of anchorLinks(pageOf(route).tree)) {
      const at = { page: rel, file: path.relative(DOCS_ROOT, link.file ?? ''), line: link.line, href: link.href };
      const to = resolveLink(route, link.href, link);
      if (!pages.has(to.route)) { noPage.push(at); continue; }
      checked++;
      const target = pageOf(to.route);
      if (target.ids.has(to.anchor)) continue;
      (target.unchecked.length ? unseen : broken).push(at);
    }
  }
  return { checked, broken, unseen, noPage };
}

async function main(argv) {
  const at = (name) => { const i = argv.indexOf(`--${name}`); return i === -1 ? null : argv[i + 1]; };
  const { checked, broken, unseen, noPage } = await checkAnchors({ under: at('under') ?? '' });
  if (argv.includes('--list')) {
    const show = (kind, l) => console.log(`${kind}  ${l.file}:${l.line}  ${l.href}${l.file.endsWith(l.page) ? '' : `  (on ${l.page})`}`);
    for (const l of broken) show('broken  ', l);
    for (const l of unseen) show('unseen  ', l);
    for (const l of noPage) show('no page ', l);
  }
  console.log(`${checked} anchor links to a page: ${broken.length} name no id of that page; ${unseen.length} go to a page whose TSX components may make the id; ${noPage.length} go to a route with no page`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main(process.argv.slice(2));
}

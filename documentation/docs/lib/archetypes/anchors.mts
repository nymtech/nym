// Anchor check for stage 8 (task 9.12): each link with a `#anchor` on a docs
// page must name an id on the page it goes to, as Nextra 4.6.1 makes the ids.
// A step heading change can break such a link on another page.
//
//   node lib/archetypes/anchors.mts [--list] [--under <dir under content>]
//
// It reads each page with its imported `.mdx` and `.md` files in place (the
// mdast of inline.mts), because a partial shows its ids in the page that
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
import { openRepo, modelCitation } from '../snippets/repo.mts';
import { loadTsModule } from '../retrieval/ts-data.mjs';
import { inlinePage } from './inline.mts';
import type { JsxAttribute, JsxElement, Node, Parent, Root } from './mdast.mts';
import { fileOf, isJsx } from './mdast.mts';
import { walk, explicitAnchor } from './rules/util.mts';

const DOCS_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const CONTENT = path.join(DOCS_ROOT, 'content');
const BASE_PATH = '/docs';

/** The ids that the tables of a whole kind make, by kind (`actor`, `vector`, `property`). */
export type KindIds = Record<string, string[]>;

export interface AnchorLink {
  file: string | undefined;
  line: number | undefined;
  href: string;
  /** A Markdown link, not an `href` attribute: Nextra removes `.md` and `.mdx` from its path. */
  markdown: boolean;
}

/** A link that the check could not match, for the report. */
export interface FoundLink {
  page: string;
  file: string;
  line: number | undefined;
  href: string;
}

/** Each `.md` and `.mdx` page under the content directory, by route. */
export function pagesByRoute(dir: string = CONTENT): Map<string, string> {
  const out = new Map<string, string>();
  const walkDir = (rel: string): void => {
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
function flattened(node: Node): string {
  const children: Node[] = 'children' in node ? node.children : [];
  return children.map((child) => ('children' in child ? flattened(child) : 'value' in child ? child.value : '')).join('');
}

const attribute = (node: JsxElement, name: string): JsxAttribute | undefined =>
  node.attributes.find((a): a is JsxAttribute => a.type === 'mdxJsxAttribute' && a.name === name);

/** The literal string value of an attribute, or undefined for an expression or a missing one. */
const stringAttribute = (node: JsxElement, name: string): string | undefined => {
  const value = attribute(node, name)?.value;
  return typeof value === 'string' ? value : undefined;
};

/** The label of the tab panel `node` in its `Tabs` parent, as Nextra reads `items`. */
function tabLabel(node: Node, parent: Parent | undefined): string | null {
  if (!isJsx(parent) || parent.name !== 'Tabs') return null;
  const items = attribute(parent, 'items');
  if (!items) return null;
  const statement = typeof items.value === 'object' && items.value !== null ? items.value.data?.estree?.body[0] : undefined;
  const array = statement?.type === 'ExpressionStatement' && statement.expression.type === 'ArrayExpression' ? statement.expression.elements : [];
  const siblings: Node[] = parent.children;
  const element = array[siblings.indexOf(node)];
  // github-slugger gives '' for a value that is not a string, as it does for a missing label.
  return element?.type === 'Literal' && typeof element.value === 'string' ? element.value : '';
}

/**
 * The ids of a page tree, in the Nextra order. Each file of the tree (the page
 * and each partial) has its own slugger. A heading id is the slug of its
 * explicit `[#anchor]` or of its text; the title (depth 1) has none. A tab
 * panel gets the slug of its label, a `<summary>` in `<details>` the slug of
 * its text, and an element its literal `id` attribute. A citation of a whole kind
 * (`<Snippet id=".../threat-model/actor" view="reference" />`) adds the ids its
 * table makes, from `kindIds` (see modelKindIds).
 */
export function anchorsOf(tree: Root, kindIds: KindIds = {}): Set<string> {
  const sluggers = new Map<string, GithubSlugger>();
  const slug = (node: Node, value: string): string => {
    const file = fileOf(node) ?? '';
    let slugger = sluggers.get(file);
    if (!slugger) {
      slugger = new GithubSlugger();
      sluggers.set(file, slugger);
    }
    return slugger.slug(value);
  };
  const ids = new Set<string>();
  walk(tree, (node, ancestors) => {
    if (node.type === 'heading') {
      if (node.depth > 1) ids.add(slug(node, explicitAnchor(node) ?? flattened(node).replace(/\s*\[#[^]+?]\s*$/, '')));
      return;
    }
    if (!isJsx(node)) return;
    if (node.name === 'Tabs.Tab') {
      const label = tabLabel(node, ancestors.at(-1));
      if (label !== null) ids.add(slug(node, label));
    }
    if (node.name === 'details') {
      walk(node, (inner) => { if (inner.type === 'mdxJsxTextElement' && inner.name === 'summary') ids.add(slug(inner, flattened(inner))); });
    }
    const id = stringAttribute(node, 'id');
    if (node.name === 'Snippet' && id !== undefined) {
      const model = modelCitation(id);
      if (model?.kind === 'collection') for (const k of kindIds[model.type] ?? []) ids.add(k);
      return;
    }
    if (id !== undefined) ids.add(id);
  });
  return ids;
}

/** Each link with an anchor in a page tree. */
export function anchorLinks(tree: Root): AnchorLink[] {
  const out: AnchorLink[] = [];
  walk(tree, (node) => {
    let href: string | undefined;
    let markdown = false;
    if (node.type === 'link') { href = node.url; markdown = true; }
    else if (isJsx(node)) href = stringAttribute(node, 'href');
    if (!href || !href.includes('#') || /^[a-z][a-z0-9+.-]*:/i.test(href)) return;
    out.push({ file: fileOf(node), line: node.position?.start.line, href, markdown });
  });
  return out;
}

const decode = (text: string): string => { try { return decodeURIComponent(text); } catch { return text; } };

/**
 * The route and anchor a link from `route` goes to, as the browser resolves it.
 * Nextra removes `.md` and `.mdx` from a Markdown link (remark-link-rewrite),
 * not from an `href` attribute. A link that leaves the base path has no route.
 */
export function resolveLink(route: string, href: string, { markdown = true }: { markdown?: boolean } = {}): { route: string | null; anchor: string } {
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
 * The fields of lib/privacy-model/threat-model.ts that the ids come from. Nothing
 * checks this against the source: under `nodenext` that file is CommonJS, so a
 * `typeof import` of it does not compile.
 */
interface ThreatModelIds {
  ACTORS: { id: string }[];
  VECTORS: { id: string }[];
  PROPERTIES: { id: string }[];
}

/**
 * The ids that the tables of the threat model make on the page
 * (components/threat-model/ThreatModelReference.tsx), by kind.
 */
export async function modelKindIds(): Promise<KindIds> {
  const tm: ThreatModelIds = await loadTsModule(path.join(DOCS_ROOT, 'lib', 'privacy-model', 'threat-model.ts'));
  return {
    actor: ['actor-L1', ...tm.ACTORS.map((a) => `actor-${a.id}`)],
    vector: tm.VECTORS.map((v) => `vector-${v.id}`),
    property: tm.PROPERTIES.map((p) => `prop-${p.id}`),
  };
}

interface ParsedPage {
  tree: Root;
  unchecked: string[];
  ids: Set<string>;
}

export async function checkAnchors({ under = '' }: { under?: string } = {}): Promise<{ checked: number; broken: FoundLink[]; unseen: FoundLink[]; noPage: FoundLink[] }> {
  const kindIds = await modelKindIds();
  const pages = pagesByRoute();
  const { aliases } = openRepo();
  const parsed = new Map<string, ParsedPage>();
  const pageOf = (route: string): ParsedPage => {
    const cached = parsed.get(route);
    if (cached) return cached;
    const rel = pages.get(route);
    if (rel === undefined) throw new Error(`no page for the route "${route}"`);
    const { tree, unchecked } = inlinePage(path.join(CONTENT, rel), { root: DOCS_ROOT, aliases });
    const page = { tree, unchecked, ids: anchorsOf(tree, kindIds) };
    parsed.set(route, page);
    return page;
  };
  const broken: FoundLink[] = [];
  const unseen: FoundLink[] = [];
  const noPage: FoundLink[] = [];
  let checked = 0;
  for (const [route, rel] of pages) {
    if (!rel.startsWith(under)) continue;
    for (const link of anchorLinks(pageOf(route).tree)) {
      const at: FoundLink = { page: rel, file: path.relative(DOCS_ROOT, link.file ?? ''), line: link.line, href: link.href };
      const to = resolveLink(route, link.href, link);
      if (to.route === null || !pages.has(to.route)) { noPage.push(at); continue; }
      checked++;
      const target = pageOf(to.route);
      if (target.ids.has(to.anchor)) continue;
      (target.unchecked.length ? unseen : broken).push(at);
    }
  }
  return { checked, broken, unseen, noPage };
}

async function main(argv: string[]): Promise<void> {
  const at = (name: string): string | null => { const i = argv.indexOf(`--${name}`); return i === -1 ? null : argv[i + 1] ?? null; };
  const { checked, broken, unseen, noPage } = await checkAnchors({ under: at('under') ?? '' });
  if (argv.includes('--list')) {
    const show = (kind: string, l: FoundLink) => console.log(`${kind}  ${l.file}:${l.line}  ${l.href}${l.file.endsWith(l.page) ? '' : `  (on ${l.page})`}`);
    for (const l of broken) show('broken  ', l);
    for (const l of unseen) show('unseen  ', l);
    for (const l of noPage) show('no page ', l);
  }
  console.log(`${checked} anchor links to a page: ${broken.length} name no id of that page; ${unseen.length} go to a page whose TSX components may make the id; ${noPage.length} go to a route with no page`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main(process.argv.slice(2));
}

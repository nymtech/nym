// The `.nym-codex/` layout of one source repository (in this repository at
// documentation/docs/.nym-codex, inside the docs package, so the docs build can
// read it: P22), and the functions that
// every snippet tool shares: the citation resolver and the link map. The record
// builder, the compose step (inlineMdxPartials), and the site plugins call the
// same functions, so they cannot disagree about what an id means.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseFrontmatter, HEADING_ANCHOR } from '../retrieval/mdx.mjs';
import type { Snippet } from '../retrieval/types.ts';

/** One site that imports snippets (`manifest.json` `sites`). */
export interface SiteConfig {
  /** The page root, relative to the repository root. */
  pages: string;
  /** codex: the page directories under `content/` that this repository owns. */
  owns?: string[];
  /** docs: the page directories that split leaves alone. */
  skip?: string[];
  /** docs: the components a snippet may use, by name, with the module of each. */
  components?: Record<string, string>;
  /** Whether the site renders a diagram citation. */
  diagrams?: boolean;
}

export interface Manifest {
  repo: string;
  github: string;
  sites?: Record<string, SiteConfig>;
  /** The OpenSpec requirements that a check enforces (`enforced` status). */
  enforced?: string[];
}

/** The `.nym-codex/` layout of one repository. */
export interface Repo {
  root: string;
  dir: string;
  snippets: string;
  manifest: Manifest;
  repo: string;
  /** The import alias of the snippets, `@snippets/<repo>/`, to their directory. */
  aliases: Record<string, string>;
}

/** Where a snippet id is on one site: the page route, and the anchor (null for a snippet with no heading). */
export interface PageLink {
  page: string;
  anchor: string | null;
}

const HERE = path.dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = path.resolve(HERE, '../../../..');
/** The snippet directory, relative to the repository root. */
export const CODEX_DIR = 'documentation/docs/.nym-codex';

/** Route of a page file under a page root: `a/b.mdx` -> `a/b`, `a/index.mdx` -> `a`. */
export function routeOf(rel: string): string {
  return rel.replace(/\.mdx?$/, '').replace(/(^|\/)index$/, '');
}

/** The `.nym-codex/` layout under the repository root `root`. */
export function openRepo(root: string = REPO_ROOT): Repo {
  const dir = path.join(root, CODEX_DIR);
  const manifest: Manifest = JSON.parse(fs.readFileSync(path.join(dir, 'manifest.json'), 'utf-8'));
  const snippets = path.join(dir, 'snippets');
  return { root, dir, snippets, manifest, repo: manifest.repo, aliases: { [`@snippets/${manifest.repo}/`]: snippets } };
}

/** The file of an authored snippet id, or null when the id is not of this repo. */
export function snippetFile(repo: Repo, id: string, ext: '.mdx' | '.mmd'): string | null {
  const prefix = `${repo.repo}/`;
  if (!id.startsWith(prefix) || id.includes('#')) return null;
  return path.join(repo.snippets, `${id.slice(prefix.length)}${ext}`);
}

/**
 * The views of a threat-model scenario record on the docs site (components/snippet.tsx).
 * A scenario variant, another drawing of a scenario, has the diagram only.
 */
export const MODEL_VIEWS: readonly string[] = ['diagram', 'assessment', 'card'];
export const VARIANT_VIEWS: readonly string[] = ['diagram'];
/** The views of a whole kind of record (`<repo>/threat-model/actor`): one table of them all. */
export const COLLECTION_VIEWS: Readonly<Record<string, readonly string[]>> = { actor: ['reference'], vector: ['reference'], property: ['reference'], scenario: ['matrix'] };

export type ModelCitation =
  | { kind: 'collection'; type: string; views: readonly string[] }
  | { kind: 'scenario'; source: string; id: string; views: readonly string[] }
  | { kind: 'variant'; source: 'variant'; id: string; views: readonly string[] }
  | { kind: 'other'; views: readonly string[] };

/**
 * What a threat-model citation id names, or null for any other id:
 * `<repo>/threat-model/scenario/<id>`, `<repo>/threat-model/example/<name>/scenario/<id>`
 * (`source` is the example name, else `generic`), or `<repo>/threat-model/variant/<id>`.
 * A kind with no id (`<repo>/threat-model/actor`, `vector`, `property`, `scenario`)
 * is `{ kind: 'collection', type }`: the generic records of that kind, as one table.
 * Any other threat-model id (one actor, a claim) is `{ kind: 'other' }`: it has no view.
 */
export function modelCitation(id: string): ModelCitation | null {
  const rest = id.match(/^[^/]+\/threat-model\/(.+)$/)?.[1];
  if (rest === undefined) return null;
  const collection = Object.hasOwn(COLLECTION_VIEWS, rest) ? COLLECTION_VIEWS[rest] : undefined;
  if (collection) return { kind: 'collection', type: rest, views: collection };
  const scenario = rest.match(/^(?:example\/([^/]+)\/)?scenario\/([^/]+)$/);
  if (scenario?.[2] !== undefined) return { kind: 'scenario', source: scenario[1] ?? 'generic', id: scenario[2], views: MODEL_VIEWS };
  const variant = rest.match(/^variant\/([^/]+)$/)?.[1];
  if (variant !== undefined) return { kind: 'variant', source: 'variant', id: variant, views: VARIANT_VIEWS };
  return { kind: 'other', views: [] };
}

/**
 * Resolve a `<Snippet id>` citation to Markdown text. A diagram is a `mermaid`
 * fence, and a code item a fence in its language, from the records that
 * `extractCode` built. A prose snippet is not citable (D4). A threat-model citation
 * (`view` from MODEL_VIEWS) stays a tag, after a check against `models` when given.
 */
export function citationText(repo: Repo, id: string, { code, attrs = '', models }: { code?: Map<string, Pick<Snippet, 'body' | 'lang'>>; attrs?: string; models?: Set<string> } = {}): string {
  const view = attrs.match(/\bview="([^"]+)"/)?.[1];
  const model = id.startsWith(`${repo.repo}/`) ? modelCitation(id) : null;
  if (model) {
    // A model citation stays a tag: the text outputs project the record where it is
    // (projections.mjs), as they project the view components.
    if (model.kind === 'other') throw new Error(`model citation ${id}: only a scenario, a scenario variant, or a kind (actor, vector, property, scenario) has views`);
    if (view === undefined || !model.views.includes(view)) throw new Error(`model citation ${id} needs view="${model.views.join('" | "')}"`);
    if (models && model.kind !== 'collection' && !models.has(id)) throw new Error(`model citation ${id} has no record`);
    return `<Snippet id="${id}" view="${view}" />`;
  }
  if (view !== undefined) throw new Error(`citation ${id}: only a threat-model scenario or variant has a view`);
  if (id.includes('#')) {
    const record = code?.get(id);
    if (!record) throw new Error(`code citation ${id} has no extracted item`);
    // One backtick more than the longest run in the body, so a line of the body cannot close the fence.
    const fence = '`'.repeat(Math.max(3, ...(record.body.match(/`+/g) ?? []).map((run) => run.length + 1)));
    return `${fence}${record.lang ?? ''}\n${record.body}\n${fence}`;
  }
  const mmd = snippetFile(repo, id, '.mmd');
  if (mmd && fs.existsSync(mmd)) return `\`\`\`mermaid\n${fs.readFileSync(mmd, 'utf-8').replace(/\n$/, '')}\n\`\`\``;
  const mdx = snippetFile(repo, id, '.mdx');
  if (mdx && fs.existsSync(mdx)) throw new Error(`citation ${id} names a prose snippet; a body cites only code, diagrams, and models`);
  throw new Error(`citation ${id} has no snippet`);
}

/** The explicit anchor of a prose snippet file, or null when its body has no heading. */
export function snippetAnchor(file: string): string | null {
  const { content }: { content: string } = parseFrontmatter(fs.readFileSync(file, 'utf-8'));
  const first = content.split('\n').find((l) => l.trim() !== '');
  if (!first || !/^#{1,6}\s/.test(first)) return null;
  const m = first.match(HEADING_ANCHOR);
  return m ? (m[1] ?? m[2] ?? null) : null;
}

/** Every page file under a page root, relative to it. */
export function pageFiles(pagesDir: string): string[] {
  const out: string[] = [];
  const walk = (rel: string): void => {
    for (const e of fs.readdirSync(path.join(pagesDir, rel), { withFileTypes: true })) {
      const r = path.join(rel, e.name);
      if (e.isDirectory()) walk(r);
      else if (/\.mdx$/.test(e.name)) out.push(r);
    }
  };
  if (fs.existsSync(pagesDir)) walk('');
  return out.sort();
}

export interface SnippetImport {
  /** The local name of the import, which the page uses as a tag. */
  name: string;
  id: string;
  line: number;
}

/** The snippet imports of a page. */
export function snippetImports(text: string, repoName?: string): SnippetImport[] {
  const found: SnippetImport[] = [];
  text.split('\n').forEach((line, i) => {
    const [, name, id] = line.match(/^import\s+(\w+)\s+from\s+['"]@snippets\/(.+)\.mdx['"]\s*;?\s*$/) ?? [];
    if (name !== undefined && id !== undefined) found.push({ name, id, line: i + 1 });
  });
  return repoName ? found.filter((f) => f.id.startsWith(`${repoName}/`)) : found;
}

/** The manifest entry of a site; it throws on a site the manifest does not list. */
export function siteConfig(repo: Repo, site: string): SiteConfig {
  const conf = repo.manifest.sites?.[site];
  if (!conf) throw new Error(`manifest.json has no site "${site}"`);
  return conf;
}

/** The page root of a site, as an absolute path. */
export const sitePages = (repo: Repo, site: string): string => path.join(repo.root, siteConfig(repo, site).pages);

/**
 * The link map of one site: snippet id -> { page, anchor }. `page` is the route
 * of the first page in path order that imports the snippet; `anchor` is null for
 * a snippet with no heading, so a link to it goes to the top of the page.
 */
export function pageLinks(repo: Repo, site: string): Map<string, PageLink> {
  const pagesDir = sitePages(repo, site);
  const map = new Map<string, PageLink>();
  for (const rel of pageFiles(pagesDir)) {
    const text = fs.readFileSync(path.join(pagesDir, rel), 'utf-8');
    for (const { id } of snippetImports(text, repo.repo)) {
      if (map.has(id)) continue;
      const file = snippetFile(repo, id, '.mdx');
      map.set(id, { page: routeOf(rel), anchor: file && fs.existsSync(file) ? snippetAnchor(file) : null });
    }
  }
  return map;
}

/**
 * A `snippet:` link target as a site route: `/<page>#<anchor>`, or `#<anchor>`
 * alone when the target is on `fromPage`, the page being built.
 */
export function linkRoute(map: Map<string, PageLink>, id: string, fromPage?: string): string {
  const hit = map.get(id);
  if (!hit) throw new Error(`snippet:${id} is on no page of this site`);
  if (hit.page === fromPage && hit.anchor) return `#${hit.anchor}`;
  return `/${hit.page}${hit.anchor ? `#${hit.anchor}` : ''}`;
}

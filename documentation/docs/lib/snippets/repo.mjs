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

const HERE = path.dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = path.resolve(HERE, '../../../..');
/** The snippet directory, relative to the repository root. */
export const CODEX_DIR = 'documentation/docs/.nym-codex';

/** Route of a page file under a page root: `a/b.mdx` -> `a/b`, `a/index.mdx` -> `a`. */
export function routeOf(rel) {
  return rel.replace(/\.mdx?$/, '').replace(/(^|\/)index$/, '');
}

/**
 * @param {string} root  the repository root
 * @returns {{ root, dir, snippets, manifest, repo, aliases }}
 */
export function openRepo(root = REPO_ROOT) {
  const dir = path.join(root, CODEX_DIR);
  const manifest = JSON.parse(fs.readFileSync(path.join(dir, 'manifest.json'), 'utf-8'));
  const snippets = path.join(dir, 'snippets');
  return { root, dir, snippets, manifest, repo: manifest.repo, aliases: { [`@snippets/${manifest.repo}/`]: snippets } };
}

/** The file of an authored snippet id, or null when the id is not of this repo. */
export function snippetFile(repo, id, ext) {
  const prefix = `${repo.repo}/`;
  if (!id.startsWith(prefix) || id.includes('#')) return null;
  return path.join(repo.snippets, `${id.slice(prefix.length)}${ext}`);
}

/**
 * Resolve a `<Snippet id>` citation to Markdown text. A diagram is a `mermaid`
 * fence, and a code item a fence in its language, from the records that
 * `extractCode` built. A prose snippet is not citable (D4).
 * @param {{ code?: Map<string, object> }} opts
 */
export function citationText(repo, id, { code } = {}) {
  if (id.includes('#')) {
    const record = code?.get(id);
    if (!record) throw new Error(`code citation ${id} has no extracted item`);
    // One backtick more than the longest run in the body, so a line of the body cannot close the fence.
    const fence = '`'.repeat(Math.max(3, ...(record.body.match(/`+/g) ?? []).map((run) => run.length + 1)));
    return `${fence}${record.lang}\n${record.body}\n${fence}`;
  }
  const mmd = snippetFile(repo, id, '.mmd');
  if (mmd && fs.existsSync(mmd)) return `\`\`\`mermaid\n${fs.readFileSync(mmd, 'utf-8').replace(/\n$/, '')}\n\`\`\``;
  const mdx = snippetFile(repo, id, '.mdx');
  if (mdx && fs.existsSync(mdx)) throw new Error(`citation ${id} names a prose snippet; a body cites only code, diagrams, and models`);
  throw new Error(`citation ${id} has no snippet`);
}

/** The explicit anchor of a prose snippet file, or null when its body has no heading. */
export function snippetAnchor(file) {
  const { content } = parseFrontmatter(fs.readFileSync(file, 'utf-8'));
  const first = content.split('\n').find((l) => l.trim() !== '');
  if (!first || !/^#{1,6}\s/.test(first)) return null;
  const m = first.match(HEADING_ANCHOR);
  return m ? (m[1] ?? m[2]) : null;
}

/** Every page file under a page root, relative to it. */
export function pageFiles(pagesDir) {
  const out = [];
  const walk = (rel) => {
    for (const e of fs.readdirSync(path.join(pagesDir, rel), { withFileTypes: true })) {
      const r = path.join(rel, e.name);
      if (e.isDirectory()) walk(r);
      else if (/\.mdx$/.test(e.name)) out.push(r);
    }
  };
  if (fs.existsSync(pagesDir)) walk('');
  return out.sort();
}

/** The snippet imports of a page: `[{ name, id, line }]`. */
export function snippetImports(text, repoName) {
  const found = [];
  text.split('\n').forEach((line, i) => {
    const m = line.match(/^import\s+(\w+)\s+from\s+['"]@snippets\/(.+)\.mdx['"]\s*;?\s*$/);
    if (m) found.push({ name: m[1], id: m[2], line: i + 1 });
  });
  return repoName ? found.filter((f) => f.id.startsWith(`${repoName}/`)) : found;
}

/**
 * The link map of one site: snippet id -> { page, anchor }. `page` is the route
 * of the first page in path order that imports the snippet; `anchor` is null for
 * a snippet with no heading, so a link to it goes to the top of the page.
 */
export function pageLinks(repo, site) {
  const conf = repo.manifest.sites?.[site];
  if (!conf) throw new Error(`manifest.json has no site "${site}"`);
  const pagesDir = path.join(repo.root, conf.pages);
  const map = new Map();
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
export function linkRoute(map, id, fromPage) {
  const hit = map.get(id);
  if (!hit) throw new Error(`snippet:${id} is on no page of this site`);
  if (hit.page === fromPage && hit.anchor) return `#${hit.anchor}`;
  return `/${hit.page}${hit.anchor ? `#${hit.anchor}` : ''}`;
}

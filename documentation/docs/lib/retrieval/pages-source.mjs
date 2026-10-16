// Docs-source adapter: walk the Nextra content tree in _meta.js order and
// normalise each MDX/MD page to a PageRecord ({ source, title, description, url,
// body }). Shared by the retrieval-index and llms-full.txt generators so the
// page-walk lives in one place; a Nextra `_meta` format change is then a
// single-file edit, not one per generator.
//
// Note: this walker only yields pages reachable through `_meta.js` ordering
// (falling back to alphabetical when a directory has no _meta). The per-page
// markdown generator uses a separate flat file-walk on purpose, because it must
// also emit `.md` for pages not listed in any _meta; the two are not
// interchangeable. Only the SITE_URL / SKIP_DIRS constants are shared with it.

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { parseFrontmatter, pageTitle, pageDescription, stripMdx, inlineMdxPartials, frontmatterLines } from './mdx.mjs';
import { openRepo, citationText, pageLinks, linkRoute, routeOf } from '../snippets/repo.mjs';
import { extractCode } from '../snippets/extract.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Content root for every page-walking generator. Defaults to the Nextra `pages/`
// tree; set DOCS_CONTENT_DIR (relative to docs/, or an absolute path) to point the
// generators at another tree. The Nextra 4 move from pages/ to content/ is then an
// env change, not a code edit, and the index can be built against an arbitrary dir.
export const PAGES_DIR = path.resolve(__dirname, '../..', process.env.DOCS_CONTENT_DIR ?? 'content');
// Output root of the text generators (llms-full.txt, docs-index.json, the .md
// mirror). DOCS_PUBLIC_DIR points them elsewhere, so a before/after compare of a
// snippet split can run without touching public/.
export const PUBLIC_DIR = path.resolve(__dirname, '../..', process.env.DOCS_PUBLIC_DIR ?? 'public');
// tsconfig baseUrl: bare specifiers like 'components/...' resolve from here.
export const DOCS_ROOT = path.resolve(__dirname, '../..');
export const SITE_URL = 'https://nym.com/docs';

// Auto-generated / non-content trees, skipped by every generator.
export const SKIP_DIRS = new Set(['api', 'archive', 'playground']);

/**
 * What a generator needs to compose a page that imports snippets (D18): the
 * `@snippets` alias, the extracted code, and the docs link map. NYM_CODEX_ROOT
 * points at another repository root, for a dry run of a split.
 */
export async function loadSnippets(root = process.env.NYM_CODEX_ROOT) {
  const repo = openRepo(root ? path.resolve(root) : undefined);
  // The link map comes from the content tree the generators read (DOCS_CONTENT_DIR included).
  repo.manifest.sites.docs = { ...repo.manifest.sites.docs, pages: path.relative(repo.root, PAGES_DIR) };
  const links = pageLinks(repo, 'docs');
  // With no docs page that imports a snippet, the docs build needs no extraction,
  // so a codex-only citation cannot fail a docs deploy.
  if (links.size === 0) return null;
  const { code, errors } = await extractCode(repo);
  if (errors.length) throw new Error(`snippet code extraction failed:\n${errors.join('\n')}`);
  return { repo, code, links };
}

/** inlineMdxPartials options for one page, with or without snippets. */
export function composeOptions(filePath, snippets) {
  const base = { filePath, root: DOCS_ROOT };
  if (!snippets) return base;
  const page = routeOf(path.relative(PAGES_DIR, filePath));
  return {
    ...base,
    aliases: snippets.repo.aliases,
    citation: (id) => citationText(snippets.repo, id, { code: snippets.code }),
    link: (id) => linkRoute(snippets.links, id, page),
  };
}

/** Ordered child keys for a directory: _meta.js order, else alphabetical. */
function getPageOrder(dir) {
  const metaPath = path.join(dir, '_meta.js');
  if (fs.existsSync(metaPath)) {
    try {
      // Nextra 4 `_meta` files are ESM (`export default { ... }`) with values
      // that are strings OR objects (separators, `type: 'page'`), so JSON.parse
      // will not do. Node treats .js here as CJS (no "type":"module"), so a
      // dynamic import would reject the `export default`; evaluate the object
      // literal instead to read its committed key order.
      const src = fs.readFileSync(metaPath, 'utf-8').replace(/^\s*export\s+default\s*/, '');
      return Object.keys(new Function(`return (${src})`)());
    } catch { /* fall through to alphabetical */ }
  }
  return fs.readdirSync(dir)
    .filter(f => !f.startsWith('_') && !f.startsWith('.'))
    .map(f => f.replace(/\.mdx?$/, ''))
    .filter((v, i, a) => a.indexOf(v) === i)
    .sort();
}

/** Page file path -> canonical docs URL (drop extension and a trailing /index). */
function fileToUrl(filePath) {
  const rel = path.relative(PAGES_DIR, filePath)
    .replace(/\.mdx?$/, '')
    .replace(/\/index$/, '');
  return `${SITE_URL}/${rel}`;
}

/**
 * Recursively collect content pages under `dir` (defaults to PAGES_DIR) in
 * _meta order, as PageRecords. Pages whose body is empty after MDX stripping
 * (pure-JSX or redirect pages) are skipped.
 */
export function collectPages(dir = PAGES_DIR, { source = 'nym-docs', expand, values, snippets } = {}, seen = new Set()) {
  const pages = [];
  for (const key of getPageOrder(dir)) {
    const subDir = path.join(dir, key);
    if (SKIP_DIRS.has(key) && fs.existsSync(subDir) && fs.statSync(subDir).isDirectory()) continue;

    let filePath = null;
    for (const ext of ['.mdx', '.md']) {
      const p = path.join(dir, `${key}${ext}`);
      if (fs.existsSync(p)) { filePath = p; break; }
    }
    if (!filePath && fs.existsSync(subDir) && fs.statSync(subDir).isDirectory()) {
      for (const ext of ['.mdx', '.md']) {
        const p = path.join(subDir, `index${ext}`);
        if (fs.existsSync(p)) { filePath = p; break; }
      }
    }

    // A directory's index.mdx is its page, and its _meta.js lists it again as
    // `index`; one file is one page.
    if (filePath && seen.has(filePath)) filePath = null;
    if (filePath) {
      seen.add(filePath);
      const raw = fs.readFileSync(filePath, 'utf-8');
      const { data, content } = parseFrontmatter(raw);
      // Partials first: their text is part of the page, so it must be present
      // before headings are split and components are expanded.
      const whole = inlineMdxPartials(content, { ...composeOptions(filePath, snippets), lineOffset: frontmatterLines(raw, content) });
      const body = stripMdx(whole, { expand, values });
      if (body.length > 0) {
        pages.push({
          source,
          title: pageTitle(data, content, key),
          description: pageDescription(data),
          url: fileToUrl(filePath),
          body,
        });
      }
    }

    if (fs.existsSync(subDir) && fs.statSync(subDir).isDirectory()) {
      pages.push(...collectPages(subDir, { source, expand, values, snippets }, seen));
    }
  }
  return pages;
}

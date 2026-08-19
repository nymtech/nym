// Docs-source adapter: walk the Nextra `pages/` tree in _meta.json order and
// normalise each MDX/MD page to a PageRecord ({ source, title, description, url,
// body }). Shared by the retrieval-index and llms-full.txt generators so the
// page-walk lives in one place; a Nextra `_meta` format change is then a
// single-file edit, not one per generator.
//
// Note: this walker only yields pages reachable through `_meta.json` ordering
// (falling back to alphabetical when a directory has no _meta). The per-page
// markdown generator uses a separate flat file-walk on purpose, because it must
// also emit `.md` for pages not listed in any _meta; the two are not
// interchangeable. Only the SITE_URL / SKIP_DIRS constants are shared with it.

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { parseFrontmatter, pageTitle, pageDescription, stripMdx, inlineMdxPartials } from './mdx.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export const PAGES_DIR = path.resolve(__dirname, '../../pages');
// tsconfig baseUrl: bare specifiers like 'components/...' resolve from here.
export const DOCS_ROOT = path.resolve(__dirname, '../..');
export const SITE_URL = 'https://nym.com/docs';

// Auto-generated / non-content trees, skipped by every generator.
export const SKIP_DIRS = new Set(['api', 'archive', 'playground']);

/** Ordered child keys for a directory: _meta.json order, else alphabetical. */
function getPageOrder(dir) {
  const metaPath = path.join(dir, '_meta.json');
  if (fs.existsSync(metaPath)) {
    try {
      return Object.keys(JSON.parse(fs.readFileSync(metaPath, 'utf-8')));
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
export function collectPages(dir = PAGES_DIR, { source = 'nym-docs', expand, values } = {}) {
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

    if (filePath) {
      const raw = fs.readFileSync(filePath, 'utf-8');
      const { data, content } = parseFrontmatter(raw);
      // Partials first: their text is part of the page, so it must be present
      // before headings are split and components are expanded.
      const whole = inlineMdxPartials(content, { filePath, root: DOCS_ROOT });
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
      pages.push(...collectPages(subDir, { source, expand, values }));
    }
  }
  return pages;
}

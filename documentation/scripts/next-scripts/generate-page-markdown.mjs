#!/usr/bin/env node

/**
 * Emits one clean Markdown file per docs page under public/, mirroring the page
 * tree, so every page is fetchable as raw markdown:
 *
 *   pages/developers/mcp.mdx      -> public/developers/mcp.md   (/docs/developers/mcp.md)
 *   pages/developers/index.mdx    -> public/developers.md       (/docs/developers.md)
 *   pages/index.mdx               -> public/index.md            (/docs/index.md)
 *
 * This is the keystone of the AI-ready docs surface (see ai-assistant-mcp-plan.md
 * 3.5): once each page has a dereferenceable .md, "copy as markdown" / "open in
 * Claude" buttons and the llms.txt index all just point at these files.
 *
 * Run from repo root or documentation/docs/:
 *   node documentation/scripts/next-scripts/generate-page-markdown.mjs
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { parseFrontmatter, pageTitle, pageDescription, stripMdx } from '../../docs/lib/retrieval/mdx.mjs';
import { loadProjections, loadDocValues } from '../../docs/lib/retrieval/projections.mjs';
// PAGES_DIR / SITE_URL / SKIP_DIRS are shared with the ordered generators. Only
// the walk differs: this generator uses a flat file collector (below) so it also
// emits .md for pages not listed in any _meta.json.
import { PAGES_DIR, SITE_URL, SKIP_DIRS } from '../../docs/lib/retrieval/pages-source.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PUBLIC_DIR = path.resolve(__dirname, '../../docs/public');

/** pages-relative slug: strip extension and a trailing /index; root -> "index". */
function pageSlug(filePath) {
  const rel = path.relative(PAGES_DIR, filePath).replace(/\.mdx?$/, '').replace(/\/index$/, '');
  return rel === '' || rel === 'index' ? 'index' : rel;
}

/** Recursively collect content page files, skipping SKIP_DIRS and _partials. */
function collectFiles(dir) {
  const files = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name.startsWith('_') || entry.name.startsWith('.')) continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (SKIP_DIRS.has(entry.name)) continue;
      files.push(...collectFiles(full));
    } else if (/\.mdx?$/.test(entry.name)) {
      files.push(full);
    }
  }
  return files;
}

console.log(`Scanning ${PAGES_DIR} ...`);
const expand = await loadProjections();
const values = await loadDocValues();
const files = collectFiles(PAGES_DIR);

let written = 0;
const seen = new Map(); // outPath -> source file, to catch silent overwrites
for (const file of files) {
  const raw = fs.readFileSync(file, 'utf-8');
  const { data, content } = parseFrontmatter(raw);
  const body = stripMdx(content, { expand, values });
  if (!body) continue; // skip pages that are pure JSX/redirects with no prose

  const slug = pageSlug(file);
  const title = pageTitle(data, content, slug.split('/').pop());
  const description = pageDescription(data);

  const header = ['---', `title: ${title}`];
  if (description) header.push(`description: ${description}`);
  header.push(`url: ${SITE_URL}/${slug === 'index' ? '' : slug}`, '---', '');

  const outPath = path.join(PUBLIC_DIR, `${slug}.md`);
  // `foo.mdx` and `foo/index.mdx` both slug to `foo`; warn rather than silently
  // clobber (the source duplicate is what needs fixing, e.g. the chain.md/.mdx pair).
  if (seen.has(outPath)) {
    console.warn(`Slug collision: ${file} and ${seen.get(outPath)} both map to ${path.relative(PUBLIC_DIR, outPath)}; keeping the latter.`);
  }
  seen.set(outPath, file);

  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${header.join('\n')}\n${body}\n`);
  written++;
}

console.log(`Wrote ${written} per-page markdown files under ${PUBLIC_DIR}`);

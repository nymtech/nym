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

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PAGES_DIR = path.resolve(__dirname, '../../docs/pages');
const PUBLIC_DIR = path.resolve(__dirname, '../../docs/public');
const SITE_URL = 'https://nym.com/docs';

// Auto-generated / non-content trees, kept out of the markdown export.
const SKIP_DIRS = new Set(['api', 'archive', 'playground']);

// TODO: stripMdx / extractTitle / extractDescription are duplicated across the
// three next-scripts generators. Converge them onto this fence-aware copy when
// fixing the llms-full.txt code-fence bug (generate-llms-txt.mjs strips `import`
// lines inside code fences); dedup rides that fix, verified against llms output.

/** Strip frontmatter, imports and JSX, but never touch fenced code blocks. */
function stripMdx(content) {
  const s = content.replace(/^---[\s\S]*?---\n*/m, ''); // frontmatter
  const out = [];
  let fence = null;

  for (const line of s.split('\n')) {
    const fenceMatch = line.match(/^(\s*)(`{3,}|~{3,})/);
    if (fenceMatch) {
      const marker = fenceMatch[2][0];
      if (fence === null) fence = marker;
      else if (marker === fence) fence = null;
      out.push(line);
      continue;
    }
    if (fence !== null) { out.push(line); continue; } // inside code: verbatim

    if (/^import\s+.*$/.test(line)) continue;
    if (/^\s*<\w[\w.-]*(?:\s[^>]*)?\s*\/>\s*$/.test(line)) continue; // self-closing JSX
    if (/^\s*<\/?\w[\w.-]*(?:\s[^>]*)?\s*>\s*$/.test(line)) continue; // JSX tag line
    out.push(line);
  }
  return out.join('\n').replace(/\n{3,}/g, '\n\n').trim();
}

function extractFrontmatterField(content, field) {
  const re = new RegExp(`^---[\\s\\S]*?${field}:\\s*["']?(.+?)["']?\\s*$`, 'm');
  const m = content.match(re);
  return m ? m[1] : '';
}

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
const files = collectFiles(PAGES_DIR);

let written = 0;
for (const file of files) {
  const raw = fs.readFileSync(file, 'utf-8');
  const body = stripMdx(raw);
  if (!body) continue; // skip pages that are pure JSX/redirects with no prose

  const slug = pageSlug(file);
  const title = extractFrontmatterField(raw, 'title') || slug.split('/').pop().replace(/[-_]/g, ' ');
  const description = extractFrontmatterField(raw, 'description');

  const header = ['---', `title: ${title}`];
  if (description) header.push(`description: ${description}`);
  header.push(`url: ${SITE_URL}/${slug === 'index' ? '' : slug}`, '---', '');

  const outPath = path.join(PUBLIC_DIR, `${slug}.md`);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${header.join('\n')}\n${body}\n`);
  written++;
}

console.log(`Wrote ${written} per-page markdown files under ${PUBLIC_DIR}`);

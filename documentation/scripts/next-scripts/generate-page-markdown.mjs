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

// TODO: the MDX-stripping helpers are duplicated across the three next-scripts
// generators, and the other two still carry two bugs this file fixed:
//   - frontmatter matched with /m (not anchored to head), so a body `---` (e.g. a
//     mermaid `config` header) gets eaten;
//   - `import` lines stripped inside code fences (corrupts llms-full.txt examples).
// Converge generate-index.mjs and generate-llms-txt.mjs onto this file's
// splitFrontmatter + stripJsx, verified against their outputs.

/**
 * Split leading YAML frontmatter from the body. Frontmatter is only valid at the
 * very head of the file, so the pattern is anchored to offset 0 (no /m flag). A
 * page with no frontmatter but a `---` elsewhere (e.g. a mermaid `config` header
 * inside a code fence) is correctly treated as having no frontmatter.
 */
function splitFrontmatter(content) {
  const m = content.match(/^---\n([\s\S]*?)\n---\n?/);
  return m ? { frontmatter: m[1], body: content.slice(m[0].length) } : { frontmatter: '', body: content };
}

/** Read a scalar field from an already-isolated frontmatter block. */
function fmField(frontmatter, field) {
  const m = frontmatter.match(new RegExp(`^${field}:\\s*["']?(.+?)["']?\\s*$`, 'm'));
  return m ? m[1] : '';
}

/** Strip imports and JSX from an MDX body, but never touch fenced code blocks. */
function stripJsx(body) {
  const out = [];
  let fence = null;

  for (const line of body.split('\n')) {
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
const seen = new Map(); // outPath -> source file, to catch silent overwrites
for (const file of files) {
  const raw = fs.readFileSync(file, 'utf-8');
  const { frontmatter, body: rawBody } = splitFrontmatter(raw);
  const body = stripJsx(rawBody);
  if (!body) continue; // skip pages that are pure JSX/redirects with no prose

  const slug = pageSlug(file);
  const title = fmField(frontmatter, 'title') || slug.split('/').pop().replace(/[-_]/g, ' ');
  const description = fmField(frontmatter, 'description');

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

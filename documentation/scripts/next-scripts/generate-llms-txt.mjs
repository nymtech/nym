#!/usr/bin/env node

/**
 * Generates public/llms-full.txt by walking pages/, reading _meta.json
 * for ordering, and concatenating all MDX/MD content as clean Markdown
 * with per-page frontmatter (Next.js llms-full.txt format).
 *
 * Run from repo root or documentation/docs/:
 *   node documentation/scripts/next-scripts/generate-llms-txt.mjs
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PAGES_DIR = path.resolve(__dirname, '../../docs/pages');
const OUTPUT_FILE = path.resolve(__dirname, '../../docs/public/llms-full.txt');
const SITE_URL = 'https://nym.com/docs';

// Directories to skip entirely (auto-generated API reference, archives, etc.)
const SKIP_DIRS = new Set(['api', 'archive', 'playground']);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Read _meta.json for ordered keys; fall back to alphabetical. */
function getPageOrder(dir) {
  const metaPath = path.join(dir, '_meta.json');
  if (fs.existsSync(metaPath)) {
    try {
      const meta = JSON.parse(fs.readFileSync(metaPath, 'utf-8'));
      return Object.keys(meta);
    } catch { /* fall through */ }
  }
  return fs.readdirSync(dir)
    .filter(f => !f.startsWith('_') && !f.startsWith('.'))
    .map(f => f.replace(/\.mdx?$/, ''))
    .filter((v, i, a) => a.indexOf(v) === i)
    .sort();
}

/** Extract title from frontmatter or first H1. */
function extractTitle(content, fallback) {
  const fm = content.match(/^---[\s\S]*?title:\s*["']?(.+?)["']?\s*$/m);
  if (fm) return fm[1];
  const h1 = content.match(/^#\s+(.+)$/m);
  if (h1) return h1[1];
  return fallback.replace(/[-_]/g, ' ');
}

/** Extract description from frontmatter. */
function extractDescription(content) {
  const fm = content.match(/^---[\s\S]*?description:\s*["']?(.+?)["']?\s*$/m);
  return fm ? fm[1] : '';
}

/** Strip frontmatter, imports, and JSX from MDX, leaving clean Markdown. */
function stripMdx(content) {
  let s = content;

  // Frontmatter
  s = s.replace(/^---[\s\S]*?---\n*/m, '');

  // Import statements
  s = s.replace(/^import\s+.*$/gm, '');

  // Self-closing JSX tags: <Component ... />
  s = s.replace(/^\s*<\w[\w.-]*(?:\s[^>]*)?\s*\/>\s*$/gm, '');

  // JSX block tags on their own line: <Callout type="info">, </Callout>, etc.
  // Keep the children — only remove the tag lines themselves.
  s = s.replace(/^\s*<\/?\w[\w.-]*(?:\s[^>]*)?\s*>\s*$/gm, '');

  // Collapse 3+ blank lines → 2
  s = s.replace(/\n{3,}/g, '\n\n');

  return s.trim();
}

/** Convert file path to URL. */
function fileToUrl(filePath) {
  let rel = path.relative(PAGES_DIR, filePath)
    .replace(/\.mdx?$/, '')
    .replace(/\/index$/, '');
  return `${SITE_URL}/${rel}`;
}

// ---------------------------------------------------------------------------
// Recursive page collector
// ---------------------------------------------------------------------------

function collectPages(dir) {
  const pages = [];
  const order = getPageOrder(dir);

  for (const key of order) {
    const subDir = path.join(dir, key);

    // Skip excluded directories
    if (SKIP_DIRS.has(key) && fs.existsSync(subDir) && fs.statSync(subDir).isDirectory()) {
      continue;
    }

    // Find the page file (.mdx preferred over .md)
    let filePath = null;
    for (const ext of ['.mdx', '.md']) {
      const p = path.join(dir, `${key}${ext}`);
      if (fs.existsSync(p)) { filePath = p; break; }
    }

    // Or an index file inside a subdirectory
    if (!filePath && fs.existsSync(subDir) && fs.statSync(subDir).isDirectory()) {
      for (const ext of ['.mdx', '.md']) {
        const p = path.join(subDir, `index${ext}`);
        if (fs.existsSync(p)) { filePath = p; break; }
      }
    }

    if (filePath) {
      const raw = fs.readFileSync(filePath, 'utf-8');
      const title = extractTitle(raw, key);
      const description = extractDescription(raw);
      const body = stripMdx(raw);

      if (body.length > 0) {
        pages.push({ title, description, url: fileToUrl(filePath), body });
      }
    }

    // Recurse into subdirectory
    if (fs.existsSync(subDir) && fs.statSync(subDir).isDirectory()) {
      pages.push(...collectPages(subDir));
    }
  }

  return pages;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

console.log(`Scanning ${PAGES_DIR} ...`);
const pages = collectPages(PAGES_DIR);

const lines = [];

// Global header
lines.push(`# Nym Documentation\n`);
lines.push(`@version: 1.20.4`);
lines.push(`@generated: ${new Date().toISOString().split('T')[0]}`);
lines.push(`@pages: ${pages.length}`);
lines.push(`@source: https://github.com/nymtech/nym/tree/develop/documentation/docs`);
lines.push('');

// Per-page blocks
for (const page of pages) {
  lines.push('---');
  lines.push(`title: ${page.title}`);
  if (page.description) {
    lines.push(`description: ${page.description}`);
  }
  lines.push(`url: ${page.url}`);
  lines.push('---');
  lines.push('');
  lines.push(page.body);
  lines.push('');
}

const output = lines.join('\n');
fs.writeFileSync(OUTPUT_FILE, output);

const sizeKb = (Buffer.byteLength(output, 'utf-8') / 1024).toFixed(0);
console.log(`Wrote ${pages.length} pages to ${OUTPUT_FILE} (${sizeKb} KB)`);

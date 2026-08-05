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
import { PAGES_DIR, collectPages } from '../../docs/lib/retrieval/pages-source.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const OUTPUT_FILE = path.resolve(__dirname, '../../docs/public/llms-full.txt');
// Project the docs version from its single source (docs/package.json) rather than
// hardcoding it here, where it silently went stale.
const DOCS_VERSION = JSON.parse(
  fs.readFileSync(path.resolve(__dirname, '../../docs/package.json'), 'utf-8'),
).version;

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

console.log(`Scanning ${PAGES_DIR} ...`);
const pages = collectPages(PAGES_DIR);

const lines = [];

// Global header
lines.push(`# Nym Documentation\n`);
lines.push(`@version: ${DOCS_VERSION}`);
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

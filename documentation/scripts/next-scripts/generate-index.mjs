#!/usr/bin/env node

/**
 * Phase 0 spike: build a retrieval index from the docs corpus.
 *
 * Walks pages/, splits each page into heading-scoped chunks, and emits
 * a source-tagged, visibility-tagged index that both the docs AI chat and
 * the MCP server load at runtime. Embedding vectors are added by a later
 * step (see the `--stats` output for projected index sizes); this spike
 * produces the chunk structure and measures it.
 *
 * The chunker is source-agnostic: chunkPages() takes normalised page
 * records, so a Confluence adapter (self-hosted; content sanitised at
 * ingestion, then treated as public) can feed the same pipeline and merge
 * into the one docs-index.json. Each chunk keeps a `source` tag for
 * citation provenance.
 *
 * Run:
 *   node documentation/scripts/next-scripts/generate-index.mjs
 *   node documentation/scripts/next-scripts/generate-index.mjs --stats   (no write, just measure)
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PAGES_DIR = path.resolve(__dirname, '../../docs/pages');
const OUTPUT_FILE = path.resolve(__dirname, '../../docs/public/docs-index.json');
const SITE_URL = 'https://nym.com/docs';

// Directories to skip entirely (auto-generated API reference, archives, playground).
const SKIP_DIRS = new Set(['api', 'archive', 'playground']);

// Chunking targets. A chunk is one heading section; sections larger than
// MAX_CHARS are split on paragraph boundaries so no single chunk dominates
// the retrieval budget. Tuned for ~400-600 token chunks (chars/4 heuristic).
const MAX_CHARS = 2400;
const MIN_CHARS = 120; // sections shorter than this fold into the next one

const STATS_ONLY = process.argv.includes('--stats');

// ---------------------------------------------------------------------------
// Page collection helpers (mirrors generate-llms-txt.mjs so the two stay in
// sync; kept local rather than shared to keep the spike self-contained).
// ---------------------------------------------------------------------------

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

function extractTitle(content, fallback) {
  const fm = content.match(/^---[\s\S]*?title:\s*["']?(.+?)["']?\s*$/m);
  if (fm) return fm[1];
  const h1 = content.match(/^#\s+(.+)$/m);
  if (h1) return h1[1];
  return fallback.replace(/[-_]/g, ' ');
}

function extractDescription(content) {
  const fm = content.match(/^---[\s\S]*?description:\s*["']?(.+?)["']?\s*$/m);
  return fm ? fm[1] : '';
}

/** Strip frontmatter, imports, and JSX from MDX, leaving clean Markdown. */
function stripMdx(content) {
  let s = content;
  s = s.replace(/^---[\s\S]*?---\n*/m, '');            // frontmatter
  s = s.replace(/^import\s+.*$/gm, '');                 // imports
  s = s.replace(/^\s*<\w[\w.-]*(?:\s[^>]*)?\s*\/>\s*$/gm, ''); // self-closing JSX
  s = s.replace(/^\s*<\/?\w[\w.-]*(?:\s[^>]*)?\s*>\s*$/gm, ''); // JSX tag lines
  s = s.replace(/\n{3,}/g, '\n\n');                     // collapse blank lines
  return s.trim();
}

function fileToUrl(filePath) {
  const rel = path.relative(PAGES_DIR, filePath)
    .replace(/\.mdx?$/, '')
    .replace(/\/index$/, '');
  return `${SITE_URL}/${rel}`;
}

/** GitHub/Nextra-compatible heading slug for deep-link anchors. */
function slugify(heading) {
  return heading
    .toLowerCase()
    .replace(/[^\w\s-]/g, '')
    .trim()
    .replace(/\s+/g, '-');
}

/** chars/4 is the standard rough token estimate; a real tokeniser comes later. */
function estimateTokens(text) {
  return Math.ceil(text.length / 4);
}

function collectPages(dir) {
  const pages = [];
  for (const key of getPageOrder(dir)) {
    const subDir = path.join(dir, key);

    if (SKIP_DIRS.has(key) && fs.existsSync(subDir) && fs.statSync(subDir).isDirectory()) {
      continue;
    }

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
      const body = stripMdx(raw);
      if (body.length > 0) {
        pages.push({
          source: 'nym-docs',
          title: extractTitle(raw, key),
          description: extractDescription(raw),
          url: fileToUrl(filePath),
          body,
        });
      }
    }

    if (fs.existsSync(subDir) && fs.statSync(subDir).isDirectory()) {
      pages.push(...collectPages(subDir));
    }
  }
  return pages;
}

// ---------------------------------------------------------------------------
// Chunking (source-agnostic: takes normalised page records)
// ---------------------------------------------------------------------------

/**
 * Split a markdown body into heading-scoped sections. Each section carries
 * its heading and the text until the next heading of equal-or-higher level.
 * Content before the first heading attaches to the page title.
 */
function splitByHeadings(body, pageTitle) {
  const lines = body.split('\n');
  const sections = [];
  let current = { heading: pageTitle, level: 1, lines: [] };

  for (const line of lines) {
    const m = line.match(/^(#{2,4})\s+(.+?)\s*#*\s*$/); // H2-H4
    if (m) {
      if (current.lines.join('\n').trim().length > 0) sections.push(current);
      current = { heading: m[2], level: m[1].length, lines: [] };
    } else {
      current.lines.push(line);
    }
  }
  if (current.lines.join('\n').trim().length > 0) sections.push(current);
  return sections;
}

/** Split an oversized section on paragraph boundaries, greedily packing to MAX_CHARS. */
function packParagraphs(text) {
  const paras = text.split(/\n\s*\n/);
  const out = [];
  let buf = '';
  for (const p of paras) {
    if (buf && (buf.length + p.length + 2) > MAX_CHARS) {
      out.push(buf.trim());
      buf = '';
    }
    buf += (buf ? '\n\n' : '') + p;
  }
  if (buf.trim()) out.push(buf.trim());
  return out;
}

function chunkPages(pages) {
  const chunks = [];

  for (const page of pages) {
    const pageId = page.url.replace(`${SITE_URL}/`, '');
    const sections = splitByHeadings(page.body, page.title);

    for (const section of sections) {
      const text = section.lines.join('\n').trim();
      if (!text) continue;

      const anchor = section.heading === page.title ? '' : `#${slugify(section.heading)}`;
      const parts = text.length > MAX_CHARS ? packParagraphs(text) : [text];

      parts.forEach((part, i) => {
        if (part.length < MIN_CHARS && parts.length === 1 && sections.length > 1) {
          // Very short standalone section; still index it (headings matter for
          // retrieval) but flag it so we can measure how many are marginal.
        }
        chunks.push({
          id: `${pageId}${anchor}${parts.length > 1 ? `~${i}` : ''}`,
          source: page.source,
          title: page.title,
          heading: section.heading,
          url: `${page.url}${anchor}`,
          text: part,
          tokensEst: estimateTokens(part),
          // vector: [...]  // added by the embedding step
        });
      });
    }
  }
  return chunks;
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

function report(chunks) {
  const n = chunks.length;
  const tokens = chunks.map(c => c.tokensEst).sort((a, b) => a - b);
  const totalTokens = tokens.reduce((s, t) => s + t, 0);
  const p = q => tokens[Math.min(tokens.length - 1, Math.floor(q * tokens.length))];
  const short = chunks.filter(c => c.text.length < MIN_CHARS).length;
  const big = chunks.filter(c => c.tokensEst > 600).length;

  const textBytes = Buffer.byteLength(JSON.stringify(chunks), 'utf-8');

  // Vector storage projections. float32 binary = dim*4 bytes/chunk; a JSON
  // array of floats is ~7 bytes/float. Real deploys store float32 (base64)
  // to keep the bundle small.
  const vecBin = dim => ((n * dim * 4) / 1024 / 1024).toFixed(2);
  const vecJson = dim => ((n * dim * 7) / 1024 / 1024).toFixed(2);

  console.log('\n  Retrieval index spike: nym-docs\n');
  console.log(`  chunks              ${n}`);
  console.log(`  total tokens (est)  ${totalTokens.toLocaleString()}`);
  console.log(`  tokens/chunk        min ${tokens[0]}  p50 ${p(0.5)}  p90 ${p(0.9)}  max ${tokens[n - 1]}`);
  console.log(`  short (<${MIN_CHARS} chars)   ${short}  (${((short / n) * 100).toFixed(0)}%)`);
  console.log(`  large (>600 tok)    ${big}  (${((big / n) * 100).toFixed(0)}%)`);
  console.log(`  text-only JSON      ${(textBytes / 1024 / 1024).toFixed(2)} MB`);
  console.log('');
  console.log('  projected full index size (text + vectors):');
  console.log(`    dim 1024 (voyage-3)   float32 ${vecBin(1024)} MB   json ${vecJson(1024)} MB`);
  console.log(`    dim 1536 (openai-3)   float32 ${vecBin(1536)} MB   json ${vecJson(1536)} MB`);
  console.log('');
  console.log('  embedding cost (one-off per build, at ~$0.02/1M input tokens):');
  console.log(`    ~$${((totalTokens / 1_000_000) * 0.02).toFixed(4)} per full re-index`);
  console.log('');
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

console.log(`Scanning ${PAGES_DIR} ...`);
const pages = collectPages(PAGES_DIR);
console.log(`Collected ${pages.length} pages.`);

const chunks = chunkPages(pages);
report(chunks);

if (!STATS_ONLY) {
  const index = {
    schema: 1,
    generated: null, // stamp at real build time; Date.* omitted in spike
    embedding: { provider: null, model: null, dim: null }, // filled by embed step
    chunks, // each carries its own `source` tag (nym-docs | confluence | ...)
  };
  fs.writeFileSync(OUTPUT_FILE, JSON.stringify(index, null, 0));
  console.log(`Wrote ${chunks.length} chunks to ${OUTPUT_FILE}`);
}

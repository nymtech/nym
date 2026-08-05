#!/usr/bin/env node

/**
 * Build the docs retrieval index consumed by the AI chat and the MCP server.
 *
 * Walks pages/, normalises each page to a PageRecord, chunks it (shared
 * chunker in docs/lib/retrieval), and, when an embeddings key is present,
 * attaches vectors with a content-hash cache so only changed chunks re-embed.
 * Output: docs/public/docs-index.json.
 *
 * The chunker and embed step are shared with the runtime (chat route, MCP) and
 * unit-tested under docs/lib/retrieval/*.test.ts.
 *
 * Run:
 *   node documentation/scripts/next-scripts/generate-index.mjs           full build
 *   node documentation/scripts/next-scripts/generate-index.mjs --stats   measure only, no write
 *
 * Env:
 *   VOYAGE_API_KEY   when set, chunks are embedded; otherwise a vectorless
 *                    (structure-only) index is written and a warning printed.
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { chunkPages } from '../../docs/lib/retrieval/chunker.mjs';
import { voyageProvider, embedChunks } from '../../docs/lib/retrieval/embed.mjs';
import { parseFrontmatter, pageTitle, pageDescription, stripMdx } from '../../docs/lib/retrieval/mdx.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PAGES_DIR = path.resolve(__dirname, '../../docs/pages');
const OUTPUT_FILE = path.resolve(__dirname, '../../docs/public/docs-index.json');
const CACHE_FILE = path.resolve(__dirname, '../../docs/.cache/embed-cache.json');
const SITE_URL = 'https://nym.com/docs';

const SKIP_DIRS = new Set(['api', 'archive', 'playground']);
const STATS_ONLY = process.argv.includes('--stats');

// ---------------------------------------------------------------------------
// Page collection (docs-source adapter: MDX/MD under pages/ -> PageRecord)
// ---------------------------------------------------------------------------

function getPageOrder(dir) {
  const metaPath = path.join(dir, '_meta.json');
  if (fs.existsSync(metaPath)) {
    try {
      return Object.keys(JSON.parse(fs.readFileSync(metaPath, 'utf-8')));
    } catch { /* fall through */ }
  }
  return fs.readdirSync(dir)
    .filter(f => !f.startsWith('_') && !f.startsWith('.'))
    .map(f => f.replace(/\.mdx?$/, ''))
    .filter((v, i, a) => a.indexOf(v) === i)
    .sort();
}

function fileToUrl(filePath) {
  const rel = path.relative(PAGES_DIR, filePath)
    .replace(/\.mdx?$/, '')
    .replace(/\/index$/, '');
  return `${SITE_URL}/${rel}`;
}

function collectPages(dir) {
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
      const body = stripMdx(content);
      if (body.length > 0) {
        pages.push({
          source: 'nym-docs',
          title: pageTitle(data, content, key),
          description: pageDescription(data),
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
// Embed cache (hash -> vector), persisted so only changed chunks re-embed
// ---------------------------------------------------------------------------

function loadCache() {
  try {
    return new Map(Object.entries(JSON.parse(fs.readFileSync(CACHE_FILE, 'utf-8'))));
  } catch {
    return new Map();
  }
}

function saveCache(cache) {
  fs.mkdirSync(path.dirname(CACHE_FILE), { recursive: true });
  fs.writeFileSync(CACHE_FILE, JSON.stringify(Object.fromEntries(cache)));
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

function report(chunks) {
  const n = chunks.length;
  const tokens = chunks.map(c => c.tokensEst).sort((a, b) => a - b);
  const totalTokens = tokens.reduce((s, t) => s + t, 0);
  const p = q => tokens[Math.min(n - 1, Math.floor(q * n))];
  const big = chunks.filter(c => c.tokensEst > 600).length;

  console.log('\n  Retrieval index: nym-docs\n');
  console.log(`  chunks              ${n}`);
  console.log(`  total tokens (est)  ${totalTokens.toLocaleString()}`);
  console.log(`  tokens/chunk        min ${tokens[0]}  p50 ${p(0.5)}  p90 ${p(0.9)}  max ${tokens[n - 1]}`);
  console.log(`  large (>600 tok)    ${big}  (${((big / n) * 100).toFixed(0)}%)`);
  console.log('');
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

console.log(`Scanning ${PAGES_DIR} ...`);
const pages = collectPages(PAGES_DIR);
console.log(`Collected ${pages.length} pages.`);

const chunks = chunkPages(pages, { siteUrl: SITE_URL });
report(chunks);

if (STATS_ONLY) process.exit(0);

const index = {
  schema: 1,
  generated: new Date().toISOString(),
  embedding: { provider: null, model: null, dim: null },
  chunks, // each carries its own `source` tag (nym-docs | confluence | ...)
};

const apiKey = process.env.VOYAGE_API_KEY;
if (apiKey) {
  const provider = voyageProvider({ apiKey });
  const cache = loadCache();
  const { embedded, cache: updated, stats } = await embedChunks(chunks, provider, cache);
  saveCache(updated);
  index.embedding = { provider: provider.name, model: provider.model, dim: provider.dim };
  index.chunks = embedded;
  console.log(`Embedded ${stats.embedded} new chunk(s), reused ${stats.cached} from cache.`);
} else {
  console.warn('VOYAGE_API_KEY not set: writing a vectorless (structure-only) index. Retrieval needs vectors.');
}

fs.writeFileSync(OUTPUT_FILE, JSON.stringify(index));
const sizeMb = (Buffer.byteLength(JSON.stringify(index), 'utf-8') / 1024 / 1024).toFixed(2);
console.log(`Wrote ${index.chunks.length} chunks to ${OUTPUT_FILE} (${sizeMb} MB)`);

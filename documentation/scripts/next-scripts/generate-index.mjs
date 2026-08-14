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
import { voyageProvider, embedChunks, resolveEmbedKey } from '../../docs/lib/retrieval/embed.mjs';
import { PAGES_DIR, SITE_URL, collectPages } from '../../docs/lib/retrieval/pages-source.mjs';
import { loadProjections, loadDocValues } from '../../docs/lib/retrieval/projections.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const OUTPUT_FILE = path.resolve(__dirname, '../../docs/public/docs-index.json');
// Under node_modules/.cache so Vercel (and CI via actions/cache) persist it
// between deploys; otherwise every deploy re-embeds the whole corpus from scratch.
const CACHE_FILE = path.resolve(__dirname, '../../docs/node_modules/.cache/nym-docs/embed-cache.json');
const STATS_ONLY = process.argv.includes('--stats');

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
const expand = await loadProjections();
const values = await loadDocValues();
const pages = collectPages(PAGES_DIR, { expand, values });
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

const apiKey = resolveEmbedKey('the docs index');
if (apiKey) {
  const provider = voyageProvider({ apiKey });
  const cache = loadCache();
  const { embedded, cache: updated, stats } = await embedChunks(chunks, provider, cache);
  saveCache(updated);
  index.embedding = { provider: provider.name, model: provider.model, dim: provider.dim };
  index.chunks = embedded;
  console.log(`Embedded ${stats.embedded} new chunk(s), reused ${stats.cached} from cache.`);
}

fs.writeFileSync(OUTPUT_FILE, JSON.stringify(index));
const sizeMb = (Buffer.byteLength(JSON.stringify(index), 'utf-8') / 1024 / 1024).toFixed(2);
console.log(`Wrote ${index.chunks.length} chunks to ${OUTPUT_FILE} (${sizeMb} MB)`);

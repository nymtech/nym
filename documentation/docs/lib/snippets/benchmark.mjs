// Retrieval benchmark: does a snippet make a better search unit than a capped
// page chunk?
//
//   VOYAGE_API_KEY=... node lib/snippets/benchmark.mjs [--queries <file>] [--k 6] [--depth 20]
//   node lib/snippets/benchmark.mjs --mock      the plumbing only: random vectors, no key
//   add --misses to print the top k routes of each query with no expected page in the top k,
//   so you can check its labels
//
// The chunker (lib/retrieval/chunker.mjs) cuts a page at its depth 2 to 4
// headings, as split does, and then cuts a section over MAX_CHARS into pieces.
// So on a split page, a section is a snippet, and the two indexes differ only
// where a snippet is long:
// - A: the docs index as the build makes it (collectPages + chunkPages).
// - B: the same, with no size cap on the split pages: one vector per snippet.
//   The pages that are not split keep their chunks from A.
// Both are built here from the current tree with the build's code, model, and
// embed cache, so a unit that equals an A chunk costs no embedding.
//
// For each query in the labelled set (benchmark-queries.json: a query and the
// page routes that answer it), it reports the rank of the first hit on an
// expected page in A and in B, then hit@1, hit@k, and MRR over the first
// `depth` hits. A hit counts on any section of an expected page. The search is
// the MCP server's: a plain cosine sort (lib/retrieval/retrieval.ts), top 6.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { collectPages, loadSnippets, PAGES_DIR, SITE_URL } from '../retrieval/pages-source.mjs';
import { chunkPages } from '../retrieval/chunker.mjs';
import { loadProjections, loadDocValues } from '../retrieval/projections.mjs';
import { voyageProvider, mockProvider, embedChunks, embedQuery } from '../retrieval/embed.mjs';
import { pageFiles, snippetImports, routeOf } from './repo.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
// The build's cache (read only), and this benchmark's own, so the build cache does not grow.
const CACHE_DIR = path.resolve(HERE, '../../node_modules/.cache/nym-docs');
const BUILD_CACHE = path.join(CACHE_DIR, 'embed-cache.json');
const OWN_CACHE = path.join(CACHE_DIR, 'benchmark-cache.json');
const MODEL = { model: 'voyage-3-large', dim: 1024 }; // as generate-index.mjs

const readCache = (file) => {
  try { return Object.entries(JSON.parse(fs.readFileSync(file, 'utf-8'))); } catch { return []; }
};

const cosine = (a, b) => {
  let dot = 0, na = 0, nb = 0;
  for (let i = 0; i < a.length; i++) { dot += a[i] * b[i]; na += a[i] * a[i]; nb += b[i] * b[i]; }
  return dot / (Math.sqrt(na) * Math.sqrt(nb) || 1);
};

/** The page route of an index URL: `https://nym.com/docs/a/b#c` -> `a/b`. */
const routeOfUrl = (url) => url.slice(SITE_URL.length).replace(/^\//, '').split('#')[0].replace(/\/$/, '').replace(/^index$/, '');

/** A and B as unembedded chunk lists, and the routes of the split pages. */
export async function buildIndexes() {
  const snippets = await loadSnippets();
  if (!snippets) throw new Error('no docs page imports a snippet: B would equal A');
  const pages = collectPages(PAGES_DIR, { expand: await loadProjections(), values: await loadDocValues(), snippets });
  const split = new Set();
  for (const rel of pageFiles(PAGES_DIR)) {
    if (snippetImports(fs.readFileSync(path.join(PAGES_DIR, rel), 'utf-8'), snippets.repo.repo).length) split.add(routeOf(rel));
  }
  const isSplit = (page) => split.has(routeOfUrl(page.url));
  const a = chunkPages(pages, { siteUrl: SITE_URL });
  const b = [
    ...chunkPages(pages.filter(isSplit), { siteUrl: SITE_URL, maxChars: Infinity }),
    ...a.filter((c) => !split.has(routeOfUrl(c.url))),
  ];
  return { a, b, split };
}

/** The 1-based rank of the first hit on an expected page among `hits`, or null. */
function rankOf(hits, expect) {
  const i = hits.findIndex((h) => expect.includes(routeOfUrl(h.url)));
  return i === -1 ? null : i + 1;
}

function summary(ranks, k) {
  const n = ranks.length;
  const at = (x) => ranks.filter((r) => r !== null && r <= x).length / n;
  return { hit1: at(1), hitK: at(k), mrr: ranks.reduce((s, r) => s + (r ? 1 / r : 0), 0) / n };
}

const positive = (name, text) => {
  const n = Number(text);
  if (!Number.isInteger(n) || n < 1) throw new Error(`--${name} must be a positive whole number, not ${text}`);
  return n;
};

async function main(argv) {
  const { values: opts } = parseArgs({ args: argv, options: {
    queries: { type: 'string', default: path.join(HERE, 'benchmark-queries.json') },
    k: { type: 'string', default: '6' },
    depth: { type: 'string', default: '20' },
    mock: { type: 'boolean', default: false },
    misses: { type: 'boolean', default: false },
  } });
  const k = positive('k', opts.k);
  const depth = positive('depth', opts.depth);
  if (k > depth) throw new Error(`--k ${k} is more than --depth ${depth}`);
  const queries = JSON.parse(fs.readFileSync(opts.queries, 'utf-8'));
  if (!queries.length) throw new Error(`${opts.queries} holds no query`);

  let provider;
  if (opts.mock) provider = mockProvider({ dim: 8 });
  else if (process.env.VOYAGE_API_KEY) provider = voyageProvider({ apiKey: process.env.VOYAGE_API_KEY, ...MODEL });
  else throw new Error('VOYAGE_API_KEY is not set (or use --mock to test the plumbing)');

  const { a, b, split } = await buildIndexes();
  const cache = opts.mock ? new Map() : new Map([...readCache(BUILD_CACHE), ...readCache(OWN_CACHE)]);
  const embeddedA = await embedChunks(a, provider, cache);
  const embeddedB = await embedChunks(b, provider, cache);
  if (!opts.mock) {
    // Every entry the build cache does not hold, under the keys embedChunks uses.
    const build = new Set(readCache(BUILD_CACHE).map(([key]) => key));
    fs.mkdirSync(CACHE_DIR, { recursive: true });
    fs.writeFileSync(OWN_CACHE, JSON.stringify(Object.fromEntries([...cache].filter(([key]) => !build.has(key)))));
  }
  const textOfA = new Map(a.map((c) => [c.id, c.text]));
  const differ = b.filter((c) => textOfA.get(c.id) !== c.text).length;
  console.error(`A: ${a.length} chunks. B: ${b.length} units over ${split.size} split pages; ${differ} of them are not an A chunk (the long snippets, as one vector each). Embedded now: A ${embeddedA.stats.embedded}, B ${embeddedB.stats.embedded}.`);

  const rows = [];
  for (const q of queries) {
    const v = await embedQuery(q.query, provider);
    const top = (chunks) => chunks.map((c) => ({ url: c.url, score: cosine(v, c.vector) })).sort((x, y) => y.score - x.score).slice(0, depth);
    const hitsA = top(embeddedA.embedded);
    const hitsB = top(embeddedB.embedded);
    rows.push({ query: q.query, expect: q.expect, a: rankOf(hitsA, q.expect), b: rankOf(hitsB, q.expect), hitsA, hitsB });
  }

  const fmt = (r) => (r === null ? `>${depth}` : String(r));
  console.log('| Query | Rank in A (capped chunks) | Rank in B (one vector per snippet) |');
  console.log('|---|---|---|');
  for (const r of rows) console.log(`| ${r.query} | ${fmt(r.a)} | ${fmt(r.b)} |`);
  const pct = (x) => `${Math.round(100 * x)}%`;
  const line = (name, s) => `| ${name} | ${pct(s.hit1)} | ${pct(s.hitK)} | ${s.mrr.toFixed(3)} |`;
  console.log('');
  console.log(`| Index | hit@1 | hit@${k} | MRR@${depth} |`);
  console.log('|---|---|---|---|');
  console.log(line('A, capped chunks', summary(rows.map((r) => r.a), k)));
  console.log(line('B, one vector per snippet', summary(rows.map((r) => r.b), k)));
  console.log(`\n${queries.length} queries${opts.mock ? '; --mock: random vectors, the numbers mean nothing' : ''}.`);

  if (!opts.misses) return;
  const miss = (r) => r === null || r > k;
  const routes = (hits) => hits.slice(0, k).map((h, i) => `${i + 1}. ${routeOfUrl(h.url)}${h.url.includes('#') ? '#' + h.url.split('#')[1] : ''} (${h.score.toFixed(3)})`).join('<br>');
  for (const r of rows.filter((r) => miss(r.a) || miss(r.b))) {
    console.log(`\n### ${r.query}\n\nExpected: ${r.expect.join(', ')}\n`);
    console.log(`| Top ${k} in A | Top ${k} in B |\n|---|---|\n| ${routes(r.hitsA)} | ${routes(r.hitsB)} |`);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main(process.argv.slice(2));
}

#!/usr/bin/env node

/**
 * Builds public/code-index.json: a separate retrieval index over selected source
 * crates/packages, embedded with Voyage's code-tuned model (voyage-code-3). Kept
 * apart from the docs index because vectors from different models are not
 * comparable, and exposed to agents via the MCP `search_code` tool.
 *
 * Scope is the ROOTS list below (repo-relative). Widen it as needed.
 *
 * Run from documentation/docs/:
 *   VOYAGE_API_KEY=xxx node ../scripts/next-scripts/generate-code-index.mjs
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { chunkCodeFile, langOf } from '../../docs/lib/retrieval/code-chunker.mjs';
import { voyageProvider, embedChunks } from '../../docs/lib/retrieval/embed.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(__dirname, '../../..'); // repo root
const OUTPUT_FILE = path.resolve(__dirname, '../../docs/public/code-index.json');
const CACHE_FILE = path.resolve(__dirname, '../../docs/.cache/code-embed-cache.json');
const CODE_MODEL = 'voyage-code-3';

// Curated scope (repo-relative). SDK + wasm + examples + select Sphinx/smolmix crates.
const ROOTS = [
  'sdk/rust',
  'sdk/typescript/packages',
  'sdk/typescript/examples',
  'sdk/ffi',
  'wasm/smolmix',
  'wasm/client',
  'wasm/zknym-lib',
  'common/nymsphinx',
  'common/smol-core',
];

const EXCLUDE = /(^|\/)(node_modules|target|dist|build|out|\.next|pkg|coverage|__pycache__)(\/|$)|\.d\.ts$/;

function walk(dir, acc) {
  let entries;
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return acc; // root may not exist on every checkout
  }
  for (const e of entries) {
    const full = path.join(dir, e.name);
    const rel = path.relative(REPO, full);
    if (EXCLUDE.test(rel)) continue;
    if (e.isDirectory()) walk(full, acc);
    else if (langOf(e.name)) acc.push(full);
  }
  return acc;
}

const files = ROOTS.flatMap((r) => walk(path.join(REPO, r), []));
console.log(`Scanning ${ROOTS.length} roots -> ${files.length} source files ...`);

const chunks = [];
for (const file of files) {
  const rel = path.relative(REPO, file).split(path.sep).join('/');
  const content = fs.readFileSync(file, 'utf-8');
  chunks.push(...chunkCodeFile(content, rel));
}
console.log(`Chunked into ${chunks.length} code chunks.`);

const index = {
  schema: 1,
  generated: new Date().toISOString(),
  embedding: { provider: null, model: null, dim: null },
  chunks,
};

const apiKey = process.env.VOYAGE_API_KEY;
if (apiKey) {
  const provider = voyageProvider({ apiKey, model: CODE_MODEL });
  const cache = fs.existsSync(CACHE_FILE)
    ? new Map(Object.entries(JSON.parse(fs.readFileSync(CACHE_FILE, 'utf-8'))))
    : new Map();
  const saveCache = (c) => {
    fs.mkdirSync(path.dirname(CACHE_FILE), { recursive: true });
    fs.writeFileSync(CACHE_FILE, JSON.stringify(Object.fromEntries(c)));
  };
  // Log progress and persist the cache as we go, so a kill/re-run resumes.
  let logged = 0;
  const { embedded, cache: updated, stats } = await embedChunks(chunks, provider, cache, {
    onProgress: ({ done, total, cache: c }) => {
      if (done - logged >= 200 || done === total) {
        console.log(`  embedded ${done}/${total} chunks ...`);
        logged = done;
        saveCache(c);
      }
    },
  });
  saveCache(updated);
  index.embedding = { provider: provider.name, model: provider.model, dim: provider.dim };
  index.chunks = embedded;
  console.log(`Embedded ${stats.embedded} new chunk(s), reused ${stats.cached} from cache.`);
} else {
  console.warn('VOYAGE_API_KEY not set: writing a vectorless code index. search_code needs vectors.');
}

fs.writeFileSync(OUTPUT_FILE, JSON.stringify(index));
const sizeMb = (Buffer.byteLength(JSON.stringify(index), 'utf-8') / 1024 / 1024).toFixed(2);
console.log(`Wrote ${index.chunks.length} chunks to ${OUTPUT_FILE} (${sizeMb} MB)`);

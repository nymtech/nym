#!/usr/bin/env node

/**
 * Builds public/code-index.json: a separate retrieval index over selected source
 * crates/packages, embedded with Voyage's code-tuned model (voyage-code-3). Kept
 * apart from the docs index because vectors from different models are not
 * comparable, and exposed to agents via the MCP `search_code` tool.
 *
 * Scope is `documentation/indexed-sources.mjs`, the canonical list of source
 * trees the docs are indexed against.
 *
 * Run from documentation/docs/:
 *   VOYAGE_API_KEY=xxx node ../scripts/next-scripts/generate-code-index.mjs
 */

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";
import {
  chunkCodeFile,
  langOf,
} from "../../docs/lib/retrieval/code-chunker.mjs";
import {
  voyageProvider,
  embedChunks,
  resolveEmbedKey,
} from "../../docs/lib/retrieval/embed.mjs";
import { ROOTS } from "../../indexed-sources.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(__dirname, "../../.."); // repo root
const OUTPUT_FILE = path.resolve(
  __dirname,
  "../../docs/public/code-index.json",
);
// Under node_modules/.cache so Vercel (and CI via actions/cache) persist it
// between deploys; otherwise every deploy re-embeds the whole code corpus.
const CACHE_FILE = path.resolve(
  __dirname,
  "../../docs/node_modules/.cache/nym-docs/code-embed-cache.json",
);
const CODE_MODEL = "voyage-code-3";

// The canonical scope lives at the root of documentation/ rather than here, so
// the list that decides what the docs can be held to is not buried in a build
// script. See that file for what adding a root costs.


const EXCLUDE =
  /(^|\/)(node_modules|target|dist|build|out|\.next|pkg|coverage|__pycache__)(\/|$)|\.d\.ts$/;

// No hand-written source approaches this (the largest is ~76KB), but gitignored
// build bundles that live under src/ (e.g. a 30MB rollup `src/worker/worker.js`)
// slip past the directory EXCLUDE, carry near-zero retrieval value, and would
// explode the index (one 30MB file is ~13k chunks). Skip anything above the cap.
const MAX_FILE_BYTES = 256 * 1024;
const oversized = [];

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
    else if (langOf(e.name)) {
      if (fs.statSync(full).size > MAX_FILE_BYTES) {
        oversized.push(rel);
        continue;
      }
      acc.push(full);
    }
  }
  return acc;
}

const files = ROOTS.flatMap((r) => walk(path.join(REPO, r), []));
console.log(
  `Scanning ${ROOTS.length} roots (documentation/indexed-sources.mjs) -> ${files.length} source files ...`,
);
if (oversized.length) {
  console.log(
    `Skipped ${oversized.length} oversized file(s) (>${MAX_FILE_BYTES / 1024}KB, likely generated bundles): ${oversized.join(", ")}`,
  );
}

const chunks = [];
for (const file of files) {
  const rel = path.relative(REPO, file).split(path.sep).join("/");
  const content = fs.readFileSync(file, "utf-8");
  chunks.push(...chunkCodeFile(content, rel));
}
console.log(`Chunked into ${chunks.length} code chunks.`);

const index = {
  schema: 1,
  generated: new Date().toISOString(),
  embedding: { provider: null, model: null, dim: null },
  chunks,
};

const apiKey = resolveEmbedKey("the code index");
if (apiKey) {
  const provider = voyageProvider({ apiKey, model: CODE_MODEL });
  const cache = fs.existsSync(CACHE_FILE)
    ? new Map(Object.entries(JSON.parse(fs.readFileSync(CACHE_FILE, "utf-8"))))
    : new Map();
  const saveCache = (c) => {
    fs.mkdirSync(path.dirname(CACHE_FILE), { recursive: true });
    fs.writeFileSync(CACHE_FILE, JSON.stringify(Object.fromEntries(c)));
  };
  // Log progress and persist the cache as we go, so a kill/re-run resumes.
  let logged = 0;
  const {
    embedded,
    cache: updated,
    stats,
  } = await embedChunks(chunks, provider, cache, {
    onProgress: ({ done, total, cache: c }) => {
      if (done - logged >= 200 || done === total) {
        console.log(`  embedded ${done}/${total} chunks ...`);
        logged = done;
        saveCache(c);
      }
    },
  });
  saveCache(updated);
  index.embedding = {
    provider: provider.name,
    model: provider.model,
    dim: provider.dim,
  };
  index.chunks = embedded;
  console.log(
    `Embedded ${stats.embedded} new chunk(s), reused ${stats.cached} from cache.`,
  );
}

fs.writeFileSync(OUTPUT_FILE, JSON.stringify(index));
const sizeMb = (
  Buffer.byteLength(JSON.stringify(index), "utf-8") /
  1024 /
  1024
).toFixed(2);
console.log(
  `Wrote ${index.chunks.length} chunks to ${OUTPUT_FILE} (${sizeMb} MB)`,
);

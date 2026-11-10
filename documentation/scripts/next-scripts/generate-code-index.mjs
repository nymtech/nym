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
import { chunkCodeFile } from "@nymproject/retrieval-indexer/code-chunker";
import {
  voyageProvider,
  embedChunks,
  resolveEmbedKey,
  loadCache,
  saveCache,
} from "@nymproject/retrieval-indexer/embed";
import {
  assertNoSilentDrops,
  assertRootsYieldChunks,
  collectSourceFiles,
  MAX_FILE_BYTES,
} from "@nymproject/retrieval-indexer/walk";
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
const SHARD = {
  source: "nym-code",
  github: "https://github.com/nymtech/nym/blob/develop",
};

// The canonical scope lives at the root of documentation/ rather than here, so
// the list that decides what the docs can be held to is not buried in a build
// script. See that file for what adding a root costs.
const { files, oversized, dropped } = collectSourceFiles(REPO, ROOTS);
assertNoSilentDrops(dropped);
const accepted = Object.entries(dropped).map(([ext, f]) => `.${ext} ${f.length}`);
if (accepted.length) console.log(`Not chunked, accepted: ${accepted.join(", ")}`);
console.log(
  `Scanning ${ROOTS.length} roots (documentation/indexed-sources.mjs) -> ${files.length} source files ...`,
);
if (oversized.length) {
  console.log(
    `Skipped ${oversized.length} oversized file(s) (>${MAX_FILE_BYTES / 1024}KB, likely generated bundles): ${oversized.join(", ")}`,
  );
}

const chunks = files.flatMap((f) => chunkCodeFile(f.content, f.path, SHARD));
assertRootsYieldChunks(ROOTS, files, chunks.map((c) => c.title));
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
  const cache = loadCache(CACHE_FILE);
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
        saveCache(CACHE_FILE, c);
      }
    },
  });
  saveCache(CACHE_FILE, updated);
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

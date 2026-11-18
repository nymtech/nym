// Writes the nym docs and code shards to the public retrieval store (task 5.1 of
// the add-shared-retrieval-index change; documentation/MCP-STORE.md).
//
// It reads the index files that generate-index.mjs and generate-code-index.mjs
// wrote in this run, and writes their vectors: it embeds nothing itself, so the
// store holds the vectors the generators made. The store skips a chunk whose id
// and content hash it already has.
//
// Run from documentation/docs after both generators, with REQUIRE_EMBEDDINGS set
// so that they fail rather than write files without vectors:
//   RETRIEVAL_STORE_WRITE_URL=... node lib/retrieval/write-store.mts
//
// The two shards are written in two transactions. If the code shard fails after
// the docs shard is written, the store holds the new docs shard beside the old
// code shard until the next run writes both.

import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import pg from 'pg';
import { embedText, type EmbedProvider } from '@nymproject/retrieval-indexer/embed';
import { indexShard, type Modality } from '@nymproject/retrieval-indexer/store';
import type { DocIndex } from '@nymproject/retrieval-indexer/types';
import { PUBLIC_DIR } from './pages-source.mjs';

const REPO = 'nym';
const DIM = 1024;
// The models the MCP route embeds queries with (pages/api/mcp.ts).
const SHARDS: [Modality, string, string][] = [
  ['docs', 'docs-index.json', 'voyage-3-large'],
  ['code', 'code-index.json', 'voyage-code-3'],
];
// A shard that loses more than this share of its rows in one run is refused: a
// lost content directory or a broken walk would otherwise delete that part of
// the public index. ALLOW_SHRINK=1 lets an intended large removal through.
const MAX_SHRINK = 0.2;

/** A provider that answers only from the index file's vectors, and refuses a file of another model. */
export function fileProvider(index: DocIndex, model: string, dim = DIM): EmbedProvider {
  if (!index.embedding?.dim) {
    throw new Error('the index file has no vectors: run the generator with VOYAGE_API_KEY and REQUIRE_EMBEDDINGS set');
  }
  if (index.embedding.model !== model || index.embedding.dim !== dim) {
    throw new Error(`the index file is embedded with ${index.embedding.model} (${index.embedding.dim}), but the store is written with ${model} (${dim})`);
  }
  const known = new Map(index.chunks.filter((c) => c.vector?.length).map((c) => [embedText(c), c.vector]));
  return {
    name: 'voyage',
    model,
    dim,
    async embed(texts) {
      return texts.map((t) => {
        const vector = known.get(t);
        if (!vector) throw new Error(`the index file has no vector for a chunk that starts "${t.slice(0, 80)}"`);
        return vector;
      });
    },
  };
}

/** Refuses a run that would drop more than MAX_SHRINK of a shard's rows. */
export function checkShrink(shard: string, before: number, after: number, allowShrink: boolean): void {
  if (allowShrink || before === 0 || after >= before * (1 - MAX_SHRINK)) return;
  throw new Error(
    `${shard}: this run has ${after} chunks, but the store holds ${before}. Refusing to delete ${before - after} rows; set ALLOW_SHRINK=1 if the removal is intended`,
  );
}

async function main(): Promise<void> {
  const url = process.env.RETRIEVAL_STORE_WRITE_URL;
  if (!url) throw new Error('write-store: RETRIEVAL_STORE_WRITE_URL is not set');
  const allowShrink = process.env.ALLOW_SHRINK === '1';

  // One client, not a pool: a write run is one transaction on one connection.
  const db = new pg.Client({ connectionString: url });
  await db.connect();
  try {
    for (const [modality, file, model] of SHARDS) {
      const index: DocIndex = JSON.parse(readFileSync(path.join(PUBLIC_DIR, file), 'utf-8'));
      const provider = fileProvider(index, model);
      const chunks = index.chunks.map(({ vector: _vector, ...chunk }) => chunk);
      const shard = `${REPO}/${modality}`;
      const { rows } = await db.query<{ n: number }>('select count(*)::int as n from chunks where shard = $1', [shard]);
      checkShrink(shard, rows[0]!.n, chunks.length, allowShrink);

      const stats = await indexShard(db, { repo: REPO, modality, chunks, provider, sourceRev: process.env.GITHUB_SHA ?? null });
      console.log(`${shard} (${model}): ${stats.total} chunks, ${stats.embedded} changed, ${stats.unchanged} unchanged, ${stats.deleted} deleted`);
    }
  } finally {
    await db.end();
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await main();

// Query-time retrieval over the docs index.
//
// Pure TypeScript: imported only by the TS runtime (the chat API route and the
// MCP server), never by the bare-node build script, so it needn't be .mjs.
//
// The source filter is the mechanism behind the "chat sees docs only, MCP sees
// everything" decision: pass sources: ["nym-docs"] for the public chat widget,
// omit it for the MCP.

import type { DocIndex, EmbeddedChunk, SearchHit } from './types';

/** Cosine similarity, guarding against zero-length vectors. */
export function cosineSimilarity(a: number[], b: number[]): number {
  let dot = 0;
  let na = 0;
  let nb = 0;
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i];
    na += a[i] * a[i];
    nb += b[i] * b[i];
  }
  if (na === 0 || nb === 0) return 0;
  return dot / (Math.sqrt(na) * Math.sqrt(nb));
}

export interface SearchOptions {
  topK?: number;
  /** Restrict to these sources; omit for all. e.g. ["nym-docs"] for the chat. */
  sources?: string[];
  /** Drop hits below this cosine score. */
  minScore?: number;
}

/** Drop the (large) vector before returning a chunk to a caller. */
function withoutVector(chunk: EmbeddedChunk): SearchHit['chunk'] {
  const { vector: _vector, ...rest } = chunk;
  return rest;
}

/**
 * Rank the index against a query vector. Linear scan: at ~1400 chunks x 1024
 * dims this is ~1.4M multiply-adds per query, well under a millisecond, so no
 * ANN index is warranted.
 */
export function search(queryVector: number[], index: DocIndex, opts: SearchOptions = {}): SearchHit[] {
  const { topK = 6, sources, minScore = 0 } = opts;

  const pool = sources ? index.chunks.filter((c) => sources.includes(c.source)) : index.chunks;

  return pool
    .map((chunk) => ({ chunk: withoutVector(chunk), score: cosineSimilarity(queryVector, chunk.vector) }))
    .filter((hit) => hit.score >= minScore)
    .sort((a, b) => b.score - a.score)
    .slice(0, topK);
}

/**
 * Fetch a whole section by chunk id or deep-link URL. Oversized sections were
 * split into `id~0`, `id~1`, ... parts sharing one anchor; this rejoins them in
 * order so `get_section` returns the complete section, not one fragment.
 */
export function getSection(index: DocIndex, idOrUrl: string): SearchHit['chunk'] | null {
  const byId = index.chunks.filter((c) => c.id === idOrUrl || c.id.split('~')[0] === idOrUrl);
  const matches = byId.length ? byId : index.chunks.filter((c) => c.url === idOrUrl);
  if (!matches.length) return null;

  const ordered = [...matches].sort((a, b) => {
    const pa = Number(a.id.split('~')[1] ?? 0);
    const pb = Number(b.id.split('~')[1] ?? 0);
    return pa - pb;
  });
  const head = ordered[0];
  return { ...withoutVector(head), id: head.id.split('~')[0], text: ordered.map((c) => c.text).join('\n\n') };
}

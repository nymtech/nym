// Embedding step for the docs index: pluggable provider + content-hash cache.
//
// Plain ESM JS so the bare-node build generator can import it. The provider is
// an interface { name, model, dim, embed(texts, inputType) } so the default
// (Voyage) can be swapped without touching the pipeline. The cache keys vectors
// by SHA-256 of chunk text: on a typical deploy only changed chunks hit the
// provider, which both cuts build latency and decouples the build from the
// embedding API's uptime (an outage reuses cached vectors instead of failing).
//
/** @typedef {import('./types').Chunk} Chunk */
/** @typedef {import('./types').EmbeddedChunk} EmbeddedChunk */

import { createHash } from 'node:crypto';

/** Stable content key for the vector cache. */
export function contentHash(text) {
  return createHash('sha256').update(text).digest('hex');
}

/**
 * Voyage provider (default). voyage-3-large returns unit-normalised 1024-dim
 * vectors and retrieves noticeably better than the older voyage-3. Uses
 * input_type to distinguish document (indexing) from query embeddings, which
 * improves retrieval quality on asymmetric search.
 *
 * NB: index and query MUST use the same model. Changing this default requires a
 * re-index (`generate-index.mjs`); querying a voyage-3 index with voyage-3-large
 * vectors returns garbage.
 *
 * @param {{ apiKey: string, model?: string, dim?: number }} cfg
 */
export function voyageProvider({ apiKey, model = 'voyage-3-large', dim = 1024 }) {
  return {
    name: 'voyage',
    model,
    dim,
    /**
     * @param {string[]} texts
     * @param {'document'|'query'} [inputType]
     * @returns {Promise<number[][]>}
     */
    async embed(texts, inputType = 'document') {
      const res = await fetch('https://api.voyageai.com/v1/embeddings', {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${apiKey}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ input: texts, model, input_type: inputType }),
      });
      if (!res.ok) {
        throw new Error(`voyage embeddings ${res.status}: ${await res.text()}`);
      }
      const json = await res.json();
      return json.data.map((d) => d.embedding);
    },
  };
}

/**
 * Deterministic mock provider for tests (no network). Derives a pseudo-vector
 * from the text hash so the same text always yields the same vector.
 *
 * @param {{ dim?: number }} [cfg]
 */
export function mockProvider({ dim = 8 } = {}) {
  return {
    name: 'mock',
    model: 'mock',
    dim,
    async embed(texts) {
      return texts.map((t) => {
        const h = createHash('sha256').update(t).digest();
        return Array.from({ length: dim }, (_, i) => (h[i % h.length] / 255) * 2 - 1);
      });
    },
  };
}

/**
 * Embed chunks, reusing cached vectors by content hash. Only cache-miss chunks
 * are sent to the provider, in batches.
 *
 * @param {Chunk[]} chunks
 * @param {{ name: string, model: string, dim: number, embed: (texts: string[], inputType?: string) => Promise<number[][]> }} provider
 * @param {Map<string, number[]>} [cache] hash -> vector; mutated and returned
 * @param {{ batchSize?: number }} [opts]
 * @returns {Promise<{ embedded: EmbeddedChunk[], cache: Map<string, number[]>, stats: { total: number, embedded: number, cached: number } }>}
 */
export async function embedChunks(chunks, provider, cache = new Map(), { batchSize = 128 } = {}) {
  const hashes = chunks.map((c) => contentHash(c.text));

  const missIdx = [];
  hashes.forEach((h, i) => {
    if (!cache.has(h)) missIdx.push(i);
  });

  for (let b = 0; b < missIdx.length; b += batchSize) {
    const batch = missIdx.slice(b, b + batchSize);
    const vectors = await provider.embed(batch.map((i) => chunks[i].text));
    batch.forEach((i, j) => cache.set(hashes[i], vectors[j]));
  }

  const embedded = chunks.map((c, i) => ({ ...c, vector: cache.get(hashes[i]) }));
  return {
    embedded,
    cache,
    stats: { total: chunks.length, embedded: missIdx.length, cached: chunks.length - missIdx.length },
  };
}

/** Embed a single search query (input_type=query for providers that support it). */
export async function embedQuery(text, provider) {
  const [vec] = await provider.embed([text], 'query');
  return vec;
}

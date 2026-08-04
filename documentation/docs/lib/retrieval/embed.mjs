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
export async function embedChunks(chunks, provider, cache = new Map(), { batchSize = 128, onProgress } = {}) {
  // Key on model + dim as well as content: vectors from different models live in
  // different spaces, so a model change must invalidate the cache rather than
  // silently reuse the old model's vectors for unchanged chunks.
  const keyPrefix = `${provider.model}:${provider.dim}:`;
  const hashes = chunks.map((c) => keyPrefix + contentHash(c.text));

  const missIdx = [];
  hashes.forEach((h, i) => {
    if (!cache.has(h)) missIdx.push(i);
  });

  // Batch by an estimated token budget (Voyage caps a batch at 120k tokens, and
  // code tokenizes far denser than prose). The estimate only keeps splits rare;
  // embedBatch below is the real safety net.
  const TOKEN_BUDGET = 80_000;
  const estTokens = (t) => Math.ceil(t.length / 1.5);

  // Embed a batch, halving and retrying if the provider still rejects it for too
  // many tokens. Robust to any content density; terminates because a single
  // (size-capped) chunk never exceeds the cap.
  async function embedBatch(texts) {
    try {
      return await provider.embed(texts);
    } catch (e) {
      if (texts.length > 1 && /too many tokens|max allowed tokens/i.test(String(e && e.message))) {
        const mid = Math.ceil(texts.length / 2);
        const head = await embedBatch(texts.slice(0, mid));
        const tail = await embedBatch(texts.slice(mid));
        return [...head, ...tail];
      }
      throw e;
    }
  }

  let b = 0;
  while (b < missIdx.length) {
    const batch = [];
    let tokens = 0;
    while (
      b < missIdx.length &&
      batch.length < batchSize &&
      (batch.length === 0 || tokens + estTokens(chunks[missIdx[b]].text) <= TOKEN_BUDGET)
    ) {
      tokens += estTokens(chunks[missIdx[b]].text);
      batch.push(missIdx[b]);
      b++;
    }
    const vectors = await embedBatch(batch.map((i) => chunks[i].text));
    batch.forEach((i, j) => cache.set(hashes[i], vectors[j]));
    if (onProgress) onProgress({ done: b, total: missIdx.length, cache });
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

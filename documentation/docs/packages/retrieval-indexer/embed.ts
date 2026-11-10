// Embedding step for the docs index: pluggable provider + content-hash cache.
//
// The provider is an interface (`EmbedProvider`) so the default (Voyage) can be
// swapped without touching the pipeline. The cache keys vectors by SHA-256 of
// chunk text: on a typical deploy only changed chunks hit the provider, which
// both cuts build latency and decouples the build from the embedding API's
// uptime (an outage reuses cached vectors instead of failing).

import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

/** What `embedText` reads from a chunk. */
export interface Embeddable {
  title?: string;
  heading?: string;
  text: string;
}

export type InputType = 'document' | 'query';

export interface EmbedProvider {
  name: string;
  model: string;
  dim: number;
  embed(texts: string[], inputType?: InputType): Promise<number[][]>;
}

/** hash -> vector */
export type EmbedCache = Map<string, number[]>;

/**
 * Read a vector cache written by `saveCache`. A missing file is an empty cache;
 * an unreadable one is an error, so a corrupt cache is not silently replaced by a
 * full, paid re-embed.
 */
export function loadCache(file: string): EmbedCache {
  if (!fs.existsSync(file)) return new Map();
  try {
    return new Map(Object.entries(JSON.parse(fs.readFileSync(file, 'utf-8'))));
  } catch (error) {
    throw new Error(`embed cache ${file} is unreadable; delete it to re-embed from scratch`, { cause: error });
  }
}

export function saveCache(file: string, cache: EmbedCache): void {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, JSON.stringify(Object.fromEntries(cache)));
}

export interface EmbedStats {
  total: number;
  embedded: number;
  cached: number;
}

/** Stable content key for the vector cache. */
export function contentHash(text: string): string {
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
 */
export function voyageProvider({ apiKey, model = 'voyage-3-large', dim = 1024 }: { apiKey: string; model?: string; dim?: number }): EmbedProvider {
  return {
    name: 'voyage',
    model,
    dim,
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
      return json.data.map((d: { embedding: number[] }) => d.embedding);
    },
  };
}

/**
 * Resolve the embedding key for an index build.
 *
 * Without a key the index still builds, deploys, and returns HTTP 200, but every
 * search returns nothing. That surfaces much later as bad answers, which points
 * at the model rather than at the deploy that dropped the key. So a missing key
 * is fatal for any build that ships.
 *
 * Everywhere else it is a warning. That covers local work on chunking or page
 * structure, and CI jobs that build only to check the docs still compile. Those
 * have no reason to spend an embedding run, so keying this on CI would fail them
 * for a artifact they never serve.
 *
 * `artifact` names what ends up vectorless in the message. Returns the key, or
 * undefined when building without one.
 */
export function resolveEmbedKey(artifact: string): string | undefined {
  const apiKey = process.env.VOYAGE_API_KEY;
  if (apiKey) return apiKey;

  // Vercel sets VERCEL on every build it serves. REQUIRE_EMBEDDINGS is the opt-in
  // for deploy pipelines elsewhere, so a workflow that publishes cannot quietly
  // lose its key. Plain CI is deliberately not enough.
  if (process.env.VERCEL || process.env.REQUIRE_EMBEDDINGS) {
    console.error(
      `\nVOYAGE_API_KEY is not set.\n` +
        `  Refusing to build ${artifact} without vectors: it would deploy cleanly ` +
        `and then return no results for every query.\n` +
        `  Set VOYAGE_API_KEY in GitHub Actions secrets and in the Vercel project ` +
        `environment (it is needed at build AND at runtime).\n`,
    );
    process.exit(1);
  }

  console.warn(
    `VOYAGE_API_KEY not set: writing ${artifact} without vectors. ` +
      `Retrieval will return nothing until you rebuild with a key.`,
  );
  return undefined;
}

/**
 * The string a chunk is embedded and cached under.
 *
 * Body prose usually assumes its own topic: a section under "Threat actors" that
 * discusses L2 rarely repeats the page title, so a search for the topic misses a
 * chunk that is entirely about it. Prepending the page title and section heading
 * puts that topic in the vector.
 *
 * `chunk.text` stays the body alone, because that is what is displayed and
 * returned to callers. Only the embedded representation carries the headers.
 */
export function embedText(chunk: Embeddable): string {
  const header = [chunk.title, chunk.heading].filter(Boolean);
  // A chunk before the first subheading carries the title as its heading; do not
  // repeat it.
  const unique = header.filter((h, i) => header.indexOf(h) === i);
  return [...unique, chunk.text].join('\n');
}

/**
 * Deterministic mock provider for tests (no network). Derives a pseudo-vector
 * from the text hash so the same text always yields the same vector.
 */
export function mockProvider({ dim = 8 }: { dim?: number } = {}): EmbedProvider {
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
 * are sent to the provider, in batches, with at most `concurrency` batches in
 * flight. `cache` is mutated and returned.
 *
 * The default of one batch at a time suits a rate-limited hosted API. A
 * self-hosted server (text-embeddings-inference takes hundreds of concurrent
 * requests) is round-trip bound when fed serially, so its pipeline raises it.
 */
export async function embedChunks<C extends Embeddable>(
  chunks: C[],
  provider: EmbedProvider,
  cache: EmbedCache = new Map(),
  { batchSize = 128, concurrency = 1, onProgress }: {
    batchSize?: number;
    concurrency?: number;
    onProgress?: (p: { done: number; total: number; cache: EmbedCache }) => void;
  } = {},
): Promise<{ embedded: (C & { vector: number[] })[]; cache: EmbedCache; stats: EmbedStats }> {
  // Key on model + dim as well as content: vectors from different models live in
  // different spaces, so a model change must invalidate the cache rather than
  // silently reuse the old model's vectors for unchanged chunks.
  const keyPrefix = `${provider.model}:${provider.dim}:`;
  const hashes = chunks.map((c) => keyPrefix + contentHash(embedText(c)));

  const missIdx: number[] = [];
  hashes.forEach((h, i) => {
    if (!cache.has(h)) missIdx.push(i);
  });

  // Batch by an estimated token budget (Voyage caps a batch at 120k tokens, and
  // code tokenizes far denser than prose). The estimate only keeps splits rare;
  // embedBatch below is the real safety net.
  const TOKEN_BUDGET = 80_000;
  const estTokens = (t: string) => Math.ceil(t.length / 1.5);

  // Embed a batch, halving and retrying if the provider still rejects it for too
  // many tokens. Robust to any content density; terminates because a single
  // (size-capped) chunk never exceeds the cap.
  async function embedBatch(texts: string[]): Promise<number[][]> {
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

  const batches: number[][] = [];
  let b = 0;
  while (b < missIdx.length) {
    const batch: number[] = [];
    let tokens = 0;
    while (
      b < missIdx.length &&
      batch.length < batchSize &&
      (batch.length === 0 || tokens + estTokens(embedText(chunks[missIdx[b]])) <= TOKEN_BUDGET)
    ) {
      tokens += estTokens(embedText(chunks[missIdx[b]]));
      batch.push(missIdx[b]);
      b++;
    }
    batches.push(batch);
  }

  // Each worker takes the next unclaimed batch. After a failure no worker claims
  // another, so a dead provider is not sent the rest of the run.
  let next = 0;
  let done = 0;
  let failed = false;
  const worker = async () => {
    while (!failed && next < batches.length) {
      const batch = batches[next++];
      try {
        const vectors = await embedBatch(batch.map((i) => embedText(chunks[i])));
        batch.forEach((i, j) => cache.set(hashes[i], vectors[j]));
      } catch (error) {
        failed = true;
        throw error;
      }
      done += batch.length;
      if (onProgress) onProgress({ done, total: missIdx.length, cache });
    }
  };
  await Promise.all(Array.from({ length: Math.min(Math.max(1, concurrency), batches.length) }, worker));

  const embedded = chunks.map((c, i) => ({ ...c, vector: cache.get(hashes[i]) }));
  return {
    embedded,
    cache,
    stats: { total: chunks.length, embedded: missIdx.length, cached: chunks.length - missIdx.length },
  };
}

/** Embed a single search query (input_type=query for providers that support it). */
export async function embedQuery(text: string, provider: EmbedProvider): Promise<number[]> {
  const [vec] = await provider.embed([text], 'query');
  return vec;
}

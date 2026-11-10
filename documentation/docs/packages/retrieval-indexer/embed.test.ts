import { describe, it, expect } from 'vitest';
// @ts-expect-error - plain ESM JS module, no type declarations
import { embedChunks, contentHash, mockProvider, embedText } from './embed.mjs';

describe('embedText', () => {
  it('prepends the page title and section heading to the body', () => {
    expect(embedText({ title: 'Threat actors', heading: 'L2 is the primary adversary', text: 'The destination...' }))
      .toBe('Threat actors\nL2 is the primary adversary\nThe destination...');
  });

  it('does not repeat the title when a chunk sits before the first subheading', () => {
    expect(embedText({ title: 'Overview', heading: 'Overview', text: 'body' })).toBe('Overview\nbody');
  });

  it('leaves the stored body untouched', () => {
    const chunk = { title: 'A', heading: 'B', text: 'body' };
    embedText(chunk);
    expect(chunk.text).toBe('body');
  });

  it('changes the cache key, so a title edit re-embeds that page', () => {
    const before = contentHash(embedText({ title: 'Old', heading: 'H', text: 'body' }));
    const after = contentHash(embedText({ title: 'New', heading: 'H', text: 'body' }));
    expect(before).not.toBe(after);
  });
});

function countingProvider(dim = 4) {
  const base = mockProvider({ dim });
  const state = { calls: 0, texts: 0 };
  return {
    state,
    name: base.name,
    model: base.model,
    dim: base.dim,
    async embed(texts: string[]) {
      state.calls += 1;
      state.texts += texts.length;
      return base.embed(texts);
    },
  };
}

function makeChunks(texts: string[]) {
  return texts.map((t, i) => ({ id: String(i), source: 's', title: 't', heading: 'h', url: 'u', text: t, tokensEst: 1 }));
}

describe('contentHash', () => {
  it('is stable for the same text and differs across texts', () => {
    expect(contentHash('abc')).toBe(contentHash('abc'));
    expect(contentHash('abc')).not.toBe(contentHash('abd'));
  });
});

describe('embedChunks', () => {
  it('embeds every chunk on a cold cache and attaches vectors', async () => {
    const provider = countingProvider(4);
    const { embedded, stats } = await embedChunks(makeChunks(['a', 'b', 'c']), provider);
    expect(stats).toEqual({ total: 3, embedded: 3, cached: 0 });
    expect(embedded.every((c: { vector: number[] }) => c.vector.length === 4)).toBe(true);
  });

  it('reuses cached vectors so unchanged chunks never hit the provider', async () => {
    const chunks = makeChunks(['a', 'b', 'c']);
    const first = await embedChunks(chunks, countingProvider(4));

    const second = countingProvider(4);
    const { stats } = await embedChunks(chunks, second, first.cache);
    expect(stats).toEqual({ total: 3, embedded: 0, cached: 3 });
    expect(second.state.calls).toBe(0); // provider untouched on a warm cache
  });

  it('only re-embeds the chunk whose text changed', async () => {
    const chunks = makeChunks(['a', 'b', 'c']);
    const first = await embedChunks(chunks, countingProvider(4));

    const changed = makeChunks(['a', 'B-CHANGED', 'c']);
    const provider = countingProvider(4);
    const { stats } = await embedChunks(changed, provider, first.cache);
    expect(stats.embedded).toBe(1);
    expect(stats.cached).toBe(2);
    expect(provider.state.texts).toBe(1);
  });

  it('batches provider calls', async () => {
    const provider = countingProvider(4);
    await embedChunks(makeChunks(['a', 'b', 'c', 'd', 'e']), provider, new Map(), { batchSize: 2 });
    expect(provider.state.calls).toBe(3); // ceil(5 / 2)
  });

  it('is deterministic: same text yields the same vector', async () => {
    const p = countingProvider(4);
    const [x] = await p.embed(['same']);
    const [y] = await p.embed(['same']);
    expect(x).toEqual(y);
  });
});

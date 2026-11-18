import { describe, it, expect } from 'vitest';
import { embedText } from '@nymproject/retrieval-indexer/embed';
import type { DocIndex, EmbeddedChunk } from '@nymproject/retrieval-indexer/types';
import { fileProvider } from './write-store.mts';

const chunk = (id: string, text: string, vector: number[]): EmbeddedChunk => ({ id, source: 'nym-docs', title: 'T', heading: id, url: `u#${id}`, text, tokensEst: 1, vector });
const index = (chunks: EmbeddedChunk[], model = 'voyage-3-large', dim = 2): DocIndex => ({ schema: 1, generated: null, embedding: { provider: 'voyage', model, dim }, chunks });

describe('fileProvider', () => {
  const a = chunk('a', 'alpha', [1, 0]);
  const b = chunk('b', 'beta', [0, 1]);

  it('answers each text with the vector the file holds for it, in the order asked', async () => {
    const provider = fileProvider(index([a, b]), 'voyage-3-large', 2);
    expect(await provider.embed([embedText(b), embedText(a)])).toEqual([[0, 1], [1, 0]]);
    expect({ model: provider.model, dim: provider.dim }).toEqual({ model: 'voyage-3-large', dim: 2 });
  });

  it('names the chunk when the file has no vector for it', async () => {
    const noVector = chunk('c', 'gamma', []);
    const provider = fileProvider(index([a, noVector]), 'voyage-3-large', 2);
    await expect(provider.embed([embedText(noVector), embedText(a)])).rejects.toThrow(/no vector for a chunk that starts/);
  });

  it('refuses a file of another model or another dimension', () => {
    expect(() => fileProvider(index([a], 'voyage-code-3'), 'voyage-3-large', 2)).toThrow(/embedded with voyage-code-3 \(2\)/);
    expect(() => fileProvider(index([a], 'voyage-3-large', 3), 'voyage-3-large', 2)).toThrow(/embedded with voyage-3-large \(3\)/);
  });

  it('refuses a file with no vectors', () => {
    const vectorless = { schema: 1, generated: null, embedding: { provider: null, model: null, dim: null }, chunks: [] } as unknown as DocIndex;
    expect(() => fileProvider(vectorless, 'voyage-3-large', 2)).toThrow(/has no vectors/);
  });
});

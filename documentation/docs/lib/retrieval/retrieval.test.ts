import { describe, it, expect } from 'vitest';
import { cosineSimilarity, search } from './retrieval';
import type { DocIndex, EmbeddedChunk } from './types';

describe('cosineSimilarity', () => {
  it('is 1 for identical direction, 0 for orthogonal', () => {
    expect(cosineSimilarity([1, 0], [2, 0])).toBeCloseTo(1);
    expect(cosineSimilarity([1, 0], [0, 1])).toBeCloseTo(0);
  });

  it('is 0 for a zero vector rather than NaN', () => {
    expect(cosineSimilarity([0, 0], [1, 1])).toBe(0);
  });
});

function chunk(id: string, source: string, vector: number[]): EmbeddedChunk {
  return { id, source, title: id, heading: id, url: `https://x/${id}`, text: id, tokensEst: 1, vector };
}

function index(chunks: EmbeddedChunk[]): DocIndex {
  return { schema: 1, generated: null, embedding: { provider: 'mock', model: 'mock', dim: 2 }, chunks };
}

describe('search', () => {
  const idx = index([
    chunk('docs-a', 'nym-docs', [1, 0]),
    chunk('docs-b', 'nym-docs', [0.8, 0.2]),
    chunk('conf-a', 'confluence', [1, 0]),
    chunk('docs-c', 'nym-docs', [0, 1]),
  ]);

  it('ranks by cosine and honours topK', () => {
    const hits = search([1, 0], idx, { topK: 2 });
    expect(hits).toHaveLength(2);
    expect(hits[0].score).toBeGreaterThanOrEqual(hits[1].score);
  });

  it('restricts to the requested sources (the chat/MCP boundary)', () => {
    const hits = search([1, 0], idx, { sources: ['nym-docs'] });
    expect(hits.every((h) => h.chunk.source === 'nym-docs')).toBe(true);
    expect(hits.some((h) => h.chunk.source === 'confluence')).toBe(false);
  });

  it('drops hits below minScore', () => {
    const hits = search([1, 0], idx, { minScore: 0.99 });
    expect(hits.every((h) => h.score >= 0.99)).toBe(true);
    expect(hits.some((h) => h.chunk.id === 'docs-c')).toBe(false); // orthogonal
  });

  it('strips the vector from returned chunks', () => {
    const [hit] = search([1, 0], idx, { topK: 1 });
    expect('vector' in hit.chunk).toBe(false);
  });
});

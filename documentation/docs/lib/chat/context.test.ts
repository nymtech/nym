import { describe, it, expect } from 'vitest';
import { buildContext } from './context';
import type { DocIndex, EmbeddedChunk } from '../retrieval/types';

function chunk(id: string, source: string, vector: number[], text: string): EmbeddedChunk {
  return { id, source, title: 'Page', heading: id, url: `https://nym.com/docs/${id}`, text, tokensEst: 1, vector };
}

const index: DocIndex = {
  schema: 1,
  generated: null,
  embedding: { provider: 'mock', model: 'mock', dim: 2 },
  chunks: [
    chunk('install', 'nym-docs', [1, 0], 'run npm i'),
    chunk('secret', 'confluence', [1, 0], 'internal note'),
    chunk('other', 'nym-docs', [0, 1], 'unrelated'),
  ],
};

describe('buildContext', () => {
  it('defaults to docs-only: never includes Confluence in the chat context', () => {
    const { context, citations } = buildContext([1, 0], index, { topK: 6 });
    expect(context).not.toContain('internal note');
    expect(citations.every((c) => !c.url.includes('secret'))).toBe(true);
  });

  it('numbers context and citations consistently from 1', () => {
    const { context, citations } = buildContext([1, 0], index, { topK: 2 });
    expect(context).toMatch(/^\[1\] /);
    expect(citations[0].n).toBe(1);
    expect(citations[0].url).toBe('https://nym.com/docs/install');
  });

  it('reports how many chunks were retrieved', () => {
    expect(buildContext([1, 0], index, { topK: 1 }).hitCount).toBe(1);
  });

  it('can be widened to other sources explicitly (e.g. for the MCP)', () => {
    const { context } = buildContext([1, 0], index, { topK: 6, sources: ['nym-docs', 'confluence'] });
    expect(context).toContain('internal note');
  });
});

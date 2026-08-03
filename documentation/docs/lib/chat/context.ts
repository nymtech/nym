// Retrieval-to-context assembly for the docs chat. Pure TypeScript, no AI-SDK
// dependency, so it builds and is unit-tested here; the streaming route in
// scaffold/chat-route.ts calls this then hands the context to the model.
//
// Default source filter is ["nym-docs"]: the public chat sees documentation
// only, never Confluence (the MCP omits the filter to see everything).

import type { DocIndex } from '../retrieval/types';
import { search } from '../retrieval/retrieval';

export interface Citation {
  n: number;
  title: string;
  heading: string;
  url: string;
}

export interface ChatContext {
  context: string;
  citations: Citation[];
  hitCount: number;
}

export interface BuildContextOptions {
  topK?: number;
  sources?: string[];
  minScore?: number;
}

/** Retrieve top-k chunks and format them as numbered context + citation list. */
export function buildContext(queryVector: number[], index: DocIndex, opts: BuildContextOptions = {}): ChatContext {
  const { topK = 6, sources = ['nym-docs'], minScore = 0 } = opts;

  const hits = search(queryVector, index, { topK, sources, minScore });

  const citations = hits.map((h, i) => ({
    n: i + 1,
    title: h.chunk.title,
    heading: h.chunk.heading,
    url: h.chunk.url,
  }));

  const context = hits
    .map((h, i) => `[${i + 1}] ${h.chunk.title} - ${h.chunk.heading}\n${h.chunk.url}\n${h.chunk.text}`)
    .join('\n\n');

  return { context, citations, hitCount: hits.length };
}

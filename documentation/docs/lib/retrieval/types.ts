// Shared types for the docs retrieval index (chat + MCP).
//
// The index is built once per docs deploy (see scripts/next-scripts/
// generate-index.mjs) and loaded at runtime by the chat API route and the
// MCP server. Chunks carry a `source` tag so consumers can filter: the public
// chat widget restricts to source === "nym-docs", the MCP sees everything.

/** A normalised page from any source, before chunking. */
export interface PageRecord {
  source: string; // "nym-docs" | "confluence" | ...
  title: string;
  description?: string;
  url: string; // canonical page URL (no anchor)
  body: string; // clean Markdown
}

/** One retrievable unit: a heading-scoped slice of a page. */
export interface Chunk {
  id: string; // stable, unique: `${pagePath}${#anchor}${~part}`
  source: string;
  title: string; // page title
  heading: string; // section heading
  url: string; // deep link, page URL + #anchor
  text: string;
  tokensEst: number;
}

/** A chunk with its embedding vector attached (post embed step). */
export interface EmbeddedChunk extends Chunk {
  vector: number[];
}

/** The on-disk index artifact: public/docs-index.json. */
export interface DocIndex {
  schema: number;
  generated: string | null; // ISO timestamp, stamped at build
  embedding: {
    provider: string | null;
    model: string | null;
    dim: number | null;
  };
  chunks: EmbeddedChunk[];
}

/** A retrieval result: the matched chunk (vector stripped) plus its score. */
export interface SearchHit {
  chunk: Chunk;
  score: number;
}

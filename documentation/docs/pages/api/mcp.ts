// MCP server for AI coding agents, served at https://nym.com/docs/api/mcp (the
// /docs basePath applies). It is a pages-router API route rather than a separate
// service so there is one deployment to operate, and so it can read the retrieval
// index the docs build already produces.
//
// Two constraints shape the code below.
//
// Stateless. Streamable HTTP supports sessions, but a serverless invocation
// shares nothing with the next one, so there is nowhere to keep a session. Each
// request builds its own Server and transport and throws them away.
//
// The transport is Web-Standard and Next's pages router is Node-shaped, so the
// SDK bridges the two and is handed Next's wrapped `res`. That works, but it is
// the seam most likely to break on a Next or SDK upgrade. If it does, the fallback
// is hosting the transport on a pristine http.ServerResponse from createServer,
// as a standalone Node process.
//
// The endpoint is public and unauthenticated, and every search costs a paid
// embedding call, so it wants a rate limiter before it carries real traffic.

import type { NextApiRequest, NextApiResponse } from 'next';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { buildMcpServer } from '../../lib/mcp/build-server';
import { createTools, type McpTool } from '../../lib/mcp/tools';
import type { DocIndex } from '../../lib/retrieval/types';
// allowJs resolves the plain-ESM embed module; types are inferred from the .mjs.
import { voyageProvider, embedQuery } from '../../lib/retrieval/embed.mjs';

// Heavy, immutable state: loaded once per serverless instance (cold start).
const INDEX_PATH = path.join(process.cwd(), 'public/docs-index.json');
let index: DocIndex;
try {
  index = JSON.parse(readFileSync(INDEX_PATH, 'utf-8'));
} catch (e) {
  // A legible failure beats an opaque module-eval crash. The usual cause is the
  // index not being built (run the docs build) or not being traced into this
  // lambda on Vercel (next.config.js outputFileTracingIncludes for /api/mcp).
  throw new Error(`MCP route: cannot load ${INDEX_PATH}: ${(e as Error).message}. Build the docs so generate-index.mjs writes it, and ensure it is traced into this function.`);
}
// A misconfigured server that still answers tools/list is worse than one that
// refuses to start: an agent connects, lists nine tools, and only discovers the
// problem as an opaque 401 inside its first search. Fail at cold start instead,
// naming the variable.
if (!process.env.VOYAGE_API_KEY) {
  throw new Error(
    'MCP route: VOYAGE_API_KEY is not set, so search_docs and search_code cannot ' +
      'embed a query. Set it in the Vercel project environment (it is needed at ' +
      'runtime as well as at build).',
  );
}
if (!index.embedding?.dim) {
  throw new Error(
    'MCP route: public/docs-index.json has no vectors, so it was built without ' +
      'VOYAGE_API_KEY. Every search would return nothing. Rebuild the docs with the key set.',
  );
}

const provider = voyageProvider({ apiKey: process.env.VOYAGE_API_KEY });

// Optional code index (built with voyage-code-3). If present, wire up search_code
// with a matching code embedder. Absence is fine; the tool just is not exposed.
const CODE_INDEX_PATH = path.join(process.cwd(), 'public/code-index.json');
let codeIndex: DocIndex | undefined;
try {
  codeIndex = JSON.parse(readFileSync(CODE_INDEX_PATH, 'utf-8'));
} catch {
  codeIndex = undefined;
}
const codeProvider = voyageProvider({ apiKey: process.env.VOYAGE_API_KEY, model: 'voyage-code-3' });

const tools: McpTool[] = createTools({
  index,
  embedQuery: (q: string) => embedQuery(q, provider),
  codeIndex,
  embedCode: codeIndex ? (q: string) => embedQuery(q, codeProvider) : undefined,
});

export default async function handler(req: NextApiRequest, res: NextApiResponse) {
  if (req.method !== 'POST') {
    // Streamable HTTP GET/DELETE are session operations; stateless mode has none.
    res.setHeader('Allow', 'POST');
    res.status(405).json({ jsonrpc: '2.0', error: { code: -32000, message: 'Method not allowed' }, id: null });
    return;
  }

  // Fresh Server + transport per request: stateless, no cross-request state.
  const transport = new StreamableHTTPServerTransport({ sessionIdGenerator: undefined });
  res.on('close', () => transport.close());
  await buildMcpServer(tools).connect(transport);
  await transport.handleRequest(req, res, req.body);
}

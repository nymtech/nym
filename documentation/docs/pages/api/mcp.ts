// MCP server for AI coding agents, at https://nym.com/docs/api/mcp (the /docs
// basePath applies). A pages-router API route rather than a separate service:
// one deployment to operate, and it can read the index the docs build writes.
//
// Stateless. Streamable HTTP has sessions, but serverless invocations share no
// memory, so each request builds its own Server and transport and discards them.
//
// The SDK transport is Web-Standard; the pages router is Node-shaped. The SDK
// bridges them and takes Next's wrapped `res`. A Next or SDK upgrade breaks here
// first. Fallback is a standalone Node process, where the transport gets a real
// http.ServerResponse from createServer.
//
// Public, unauthenticated, and every search costs an embedding call. Needs a
// rate limiter before real traffic.

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
// Fail at cold start, naming the variable. Otherwise an agent connects, gets a
// full tool list, and hits an opaque 401 inside its first search.
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
// The docs index throws when vectorless, because search is the route's core
// function. The code index is optional, so the fail-closed choice is the
// opposite: drop it, so search_code is simply not exposed, rather than kill the
// docs and live tools over an optional file. A deploy that meant to serve code
// search still fails loudly at check-mcp-server.sh's index-coverage group.
if (codeIndex && !codeIndex.embedding?.dim) {
  console.warn(
    'MCP route: public/code-index.json has no vectors (built without VOYAGE_API_KEY), ' +
      'so search_code will not be exposed. Rebuild with the key set to enable code search.',
  );
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

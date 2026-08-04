// MCP server, mounted as a pages-router API route so it ships with the docs app
// on Vercel (one deployment to operate, not a second service). Clients point
// their agent at https://nym.com/docs/api/mcp (the /docs basePath applies).
//
// Needs:  @modelcontextprotocol/sdk installed.
// Env:    VOYAGE_API_KEY (query embedding); build public/docs-index.json first.
//
// VERIFIED WORKING against SDK 1.30.0 (dev server, curl over Streamable HTTP):
//   - tools/list returns all tools; tools/call round-trips (validate_sdk_config
//     and the live network tools both confirmed).
//   - The transport bridges Node <-> Web Standard via @hono/node-server; it
//     accepts Next's wrapped pages-router `res` fine (this was the main risk).
//     If a future Next/SDK bump breaks that, scaffold/standalone.ts is the
//     fallback (pristine http.ServerResponse from createServer).
//   - Stateless mode (sessionIdGenerator: undefined) suits serverless: a fresh
//     Server + transport per request keeps no cross-invocation state.
//
// STILL TO CONFIRM under load: per-request lifecycle under concurrent Vercel
// invocations; rate-limiting/abuse policy on the public endpoint (plan D3).

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
const provider = voyageProvider({ apiKey: process.env.VOYAGE_API_KEY });
const tools: McpTool[] = createTools({ index, embedQuery: (q: string) => embedQuery(q, provider) });

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

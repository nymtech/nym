// SCAFFOLD - RECOMMENDED deployment. Move to `pages/api/mcp.ts`. Mounts the MCP
// server as a pages-router API route, so it ships with the existing docs app on
// Vercel (one deployment to operate, not a second service). Clients point their
// agent at https://nym.com/docs/api/mcp (the /docs basePath applies).
//
// Needs:  pnpm add @modelcontextprotocol/sdk
// Env:    VOYAGE_API_KEY (query embedding); build public/docs-index.json first.
//
// VERIFIED against SDK 1.30.0 types:
//   - handleRequest(req, res, parsedBody) is typed for Node IncomingMessage/
//     ServerResponse, so a pages-router handler is the right shape.
//   - Stateless mode (sessionIdGenerator: undefined) suits serverless: no
//     per-session state kept between invocations.
//
// VERIFY ON INSTALL (cannot be exercised in-sandbox):
//   - THE SHARP RISK: the 1.30.0 transport bridges Node <-> Web Standard via
//     @hono/node-server internally (NOT direct res writes like the chat route's
//     pipeDataStreamToResponse). Next's pages-router `res` is a WRAPPED
//     ServerResponse, and the hono node-adapter may not accept the wrapper. If it
//     chokes, fall back to standalone.ts, which gets a pristine http.ServerResponse
//     from createServer. This is the most likely place the move fails.
//   - per-request Server + transport lifecycle under concurrent Vercel
//     invocations (a fresh pair per request avoids shared state).
//   - Next's body parsing hands MCP's JSON POST to req.body; passed as parsedBody.

import type { NextApiRequest, NextApiResponse } from 'next';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { buildMcpServer } from './build-server';
import { createTools, type McpTool } from '../tools';
import type { DocIndex } from '../../retrieval/types';
// @ts-expect-error - plain ESM JS module, no type declarations
import { voyageProvider, embedQuery } from '../../retrieval/embed.mjs';

// Heavy, immutable state: loaded once per serverless instance (cold start).
const index: DocIndex = JSON.parse(readFileSync(path.join(process.cwd(), 'public/docs-index.json'), 'utf-8'));
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

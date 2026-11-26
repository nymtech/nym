// MCP server for AI coding agents, at https://nym.com/docs/api/mcp (the /docs
// basePath applies). A pages-router API route rather than a separate service:
// one deployment to operate. It searches the retrieval store that producer CI
// writes (documentation/MCP-SERVER.md).
//
// Stateless. Streamable HTTP has sessions, but serverless invocations share no
// memory, so each request builds its own Server and transport and discards them.
//
// The SDK transport is Web-Standard; the pages router is Node-shaped. The SDK
// bridges them and takes Next's wrapped `res`. A Next or SDK upgrade breaks here
// first. Fallback is a standalone Node process, where the transport gets a real
// http.ServerResponse from createServer.
//
// Public, unauthenticated, and every search costs an embedding call.

import type { NextApiRequest, NextApiResponse } from 'next';
import pg from 'pg';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { buildMcpServer } from '../../lib/mcp/build-server';
import { createTools, type McpTool } from '../../lib/mcp/tools';
import { retryDroppedConnection, storeSource, type RetrievalSource } from '../../lib/retrieval/retrieval-source';
// allowJs resolves the plain-ESM embed module; types are inferred from the .mjs.
import { voyageProvider, embedQuery } from '@nymproject/retrieval-indexer/embed';
import { storeConnection } from '@nymproject/retrieval-indexer/store';

// Fail at cold start, naming the variable. Otherwise an agent connects, gets a
// full tool list, and hits an opaque 401 inside its first search.
if (!process.env.VOYAGE_API_KEY) {
  throw new Error(
    'MCP route: VOYAGE_API_KEY is not set, so search_docs and search_code cannot ' +
      'embed a query. Set it in the Vercel project environment.',
  );
}
const provider = voyageProvider({ apiKey: process.env.VOYAGE_API_KEY });
const codeProvider = voyageProvider({ apiKey: process.env.VOYAGE_API_KEY, model: 'voyage-code-3' });

function storeSourceFromEnv(): RetrievalSource {
  const url = process.env.RETRIEVAL_STORE_READ_URL;
  if (!url) {
    throw new Error('MCP route: RETRIEVAL_STORE_READ_URL is not set. Set it to the reader connection string (documentation/MCP-SERVER.md).');
  }
  // A pool, not one client, so a dropped connection (a server restart, an idle
  // timeout) is replaced on the next query without a redeploy. Reads are single
  // statements, so pooling is safe here; only the write path needs one connection
  // for a whole transaction.
  //
  // The timeouts turn an unreachable or slow store into an error the agent can
  // read. statement_timeout stops a slow query on the server; query_timeout, above
  // it, catches a store that stops answering.
  const pool = new pg.Pool({ ...storeConnection(url, process.env.RETRIEVAL_STORE_CA_CERT), max: 2, idleTimeoutMillis: 30_000, connectionTimeoutMillis: 5_000, statement_timeout: 5_000, query_timeout: 8_000 });
  // An idle connection that dies emits 'error' on the pool. With no listener,
  // Node treats it as unhandled and the instance exits.
  pool.on('error', (e) => console.error(`MCP route: retrieval store connection error: ${e.message}`));
  return storeSource(retryDroppedConnection(pool), { docs: provider.model, code: codeProvider.model });
}

const retrieval = storeSourceFromEnv();

const tools: McpTool[] = createTools({
  retrieval,
  embedQuery: (q: string) => embedQuery(q, provider),
  embedCode: (q: string) => embedQuery(q, codeProvider),
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

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
// Public, unauthenticated, and every search costs an embedding call. There is
// no rate limiter: a spend alert on the Voyage account bounds the cost (design Q2
// of the add-shared-retrieval-index change).

import type { NextApiRequest, NextApiResponse } from 'next';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import pg from 'pg';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { buildMcpServer } from '../../lib/mcp/build-server';
import { createTools, type McpTool } from '../../lib/mcp/tools';
import { indexSource, retryDroppedConnection, storeSource, type RetrievalSource } from '../../lib/retrieval/retrieval-source';
import { storeConnection } from '../../lib/retrieval/store-tls.mjs';
import type { DocIndex } from '@nymproject/retrieval-indexer/types';
// allowJs resolves the plain-ESM embed module; types are inferred from the .mjs.
import { voyageProvider, embedQuery } from '@nymproject/retrieval-indexer/embed';

// Fail at cold start, naming the variable. Otherwise an agent connects, gets a
// full tool list, and hits an opaque 401 inside its first search.
if (!process.env.VOYAGE_API_KEY) {
  throw new Error(
    'MCP route: VOYAGE_API_KEY is not set, so search_docs and search_code cannot ' +
      'embed a query. Set it in the Vercel project environment (it is needed at ' +
      'runtime as well as at build).',
  );
}
const provider = voyageProvider({ apiKey: process.env.VOYAGE_API_KEY });
const codeProvider = voyageProvider({ apiKey: process.env.VOYAGE_API_KEY, model: 'voyage-code-3' });

// The bundled index is large and immutable, so it is read once per serverless
// instance, at cold start.
function bundleSource(): RetrievalSource {
  const indexPath = path.join(process.cwd(), 'public/docs-index.json');
  let index: DocIndex;
  try {
    index = JSON.parse(readFileSync(indexPath, 'utf-8'));
  } catch (e) {
    // A legible failure beats an opaque module-eval crash. The usual cause is the
    // index not being built (run the docs build) or not being traced into this
    // lambda on Vercel (next.config.js outputFileTracingIncludes for /api/mcp).
    throw new Error(`MCP route: cannot load ${indexPath}: ${(e as Error).message}. Build the docs so generate-index.mjs writes it, and ensure it is traced into this function.`);
  }
  if (!index.embedding?.dim) {
    throw new Error(
      'MCP route: public/docs-index.json has no vectors, so it was built without ' +
        'VOYAGE_API_KEY. Every search would return nothing. Rebuild the docs with the key set.',
    );
  }

  // Optional code index (built with voyage-code-3). Absence is fine; search_code
  // is then not exposed.
  const codeIndexPath = path.join(process.cwd(), 'public/code-index.json');
  let codeIndex: DocIndex | undefined;
  try {
    codeIndex = JSON.parse(readFileSync(codeIndexPath, 'utf-8'));
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
  return indexSource(index, codeIndex);
}

function storeSourceFromEnv(): RetrievalSource {
  const url = process.env.RETRIEVAL_STORE_READ_URL;
  if (!url) {
    throw new Error('MCP route: MCP_RETRIEVAL_SOURCE is "store", but RETRIEVAL_STORE_READ_URL is not set. Set it to the reader connection string (documentation/MCP-STORE-SETUP.md).');
  }
  // A pool, not one client, so a dropped connection (a server restart, an idle
  // timeout) is replaced on the next query without a redeploy. Reads are single
  // statements, so pooling is safe here; only the write path needs one connection
  // for a whole transaction.
  //
  // The timeouts make an unreachable store an error the agent can read, not a
  // request that hangs until the function times out. statement_timeout makes the
  // server stop a slow query; query_timeout is above it, so a slow query gets the
  // server's error, and only a store that stops answering hits the client limit.
  const pool = new pg.Pool({ ...storeConnection(url, process.env.RETRIEVAL_STORE_CA_CERT), max: 2, idleTimeoutMillis: 30_000, connectionTimeoutMillis: 5_000, statement_timeout: 5_000, query_timeout: 8_000 });
  // An idle connection that dies emits 'error' on the pool. With no listener,
  // Node treats it as unhandled and the instance exits.
  pool.on('error', (e) => console.error(`MCP route: retrieval store connection error: ${e.message}`));
  return storeSource(retryDroppedConnection(pool), { docs: provider.model, code: codeProvider.model });
}

// MCP_RETRIEVAL_SOURCE picks the index: "bundle" (the default) reads the files
// the docs build traces into this function; "store" reads the public Postgres
// store (task 5.2 of the add-shared-retrieval-index change, documentation/MCP-STORE.md).
const SOURCE = process.env.MCP_RETRIEVAL_SOURCE || 'bundle';
if (SOURCE !== 'bundle' && SOURCE !== 'store') {
  throw new Error(`MCP route: MCP_RETRIEVAL_SOURCE is "${SOURCE}"; use "bundle" or "store".`);
}
const retrieval = SOURCE === 'store' ? storeSourceFromEnv() : bundleSource();

const tools: McpTool[] = createTools({
  retrieval,
  embedQuery: (q: string) => embedQuery(q, provider),
  embedCode: retrieval.searchCode ? (q: string) => embedQuery(q, codeProvider) : undefined,
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

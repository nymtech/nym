// SCAFFOLD - ALTERNATIVE deployment. A standalone Node HTTP server, for running
// the MCP endpoint as its own deployable/package (Phase 2) or for local
// `list tools` testing without booting the whole Next app. Prefer mcp-route.ts
// (ships with the docs app) unless you specifically want a separate service.
//
// Needs:  pnpm add @modelcontextprotocol/sdk
// Run:    VOYAGE_API_KEY=... tsx lib/mcp/scaffold/standalone.ts   (needs tsx)
// Env:    DOCS_INDEX_PATH (default ../../public/docs-index.json), PORT (8787)
//
// VERIFY ON INSTALL: same transport lifecycle note as mcp-route.ts.

import { readFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { buildMcpServer } from './build-server';
import { createTools, type McpTool } from '../tools';
import type { DocIndex } from '../../retrieval/types';
// @ts-expect-error - plain ESM JS module, no type declarations
import { voyageProvider, embedQuery } from '../../retrieval/embed.mjs';

const INDEX_PATH = process.env.DOCS_INDEX_PATH ?? new URL('../../../public/docs-index.json', import.meta.url).pathname;
const PORT = Number(process.env.PORT ?? 8787);

const index: DocIndex = JSON.parse(readFileSync(INDEX_PATH, 'utf-8'));
const provider = voyageProvider({ apiKey: process.env.VOYAGE_API_KEY });
const tools: McpTool[] = createTools({ index, embedQuery: (q: string) => embedQuery(q, provider) });

const http = createServer(async (req, res) => {
  if (req.method === 'POST' && req.url === '/mcp') {
    const transport = new StreamableHTTPServerTransport({ sessionIdGenerator: undefined });
    res.on('close', () => transport.close());
    await buildMcpServer(tools).connect(transport);
    await transport.handleRequest(req, res);
    return;
  }
  res.writeHead(404).end('Not found');
});

http.listen(PORT, () => console.log(`nym-docs MCP on http://localhost:${PORT}/mcp (${tools.length} tools)`));

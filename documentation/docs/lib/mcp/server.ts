// MCP server transport shell (Streamable HTTP). This is the ONLY unverified
// piece: it needs `@modelcontextprotocol/sdk` installed, which the sandbox
// cannot do, so it is not exercised by the test suite. The tool logic it wires
// (createTools) is fully tested in tools.test.ts.
//
// VERIFY ON INSTALL against the SDK version you pin:
//   - package + import paths below (@modelcontextprotocol/sdk/server/*)
//   - the stateless StreamableHTTPServerTransport wiring (per-request Server)
//   - that ListTools accepts JSON Schema `inputSchema` verbatim (it should;
//     that is why this uses the low-level Server, not McpServer + Zod)
//
// Deployment: this becomes its own package/deployable (Phase 2). It loads the
// build-time docs-index.json and needs VOYAGE_API_KEY for query embedding.

import { readFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { ListToolsRequestSchema, CallToolRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import { createTools, type McpTool } from './tools';
import type { DocIndex } from '../retrieval/types';
// @ts-expect-error - plain ESM JS module, no type declarations
import { voyageProvider, embedQuery } from '../retrieval/embed.mjs';

const INDEX_PATH = process.env.DOCS_INDEX_PATH ?? new URL('../../public/docs-index.json', import.meta.url).pathname;
const PORT = Number(process.env.PORT ?? 8787);

// Load the index and build the query embedder once at startup.
const index: DocIndex = JSON.parse(readFileSync(INDEX_PATH, 'utf-8'));
const provider = voyageProvider({ apiKey: process.env.VOYAGE_API_KEY });
const tools: McpTool[] = createTools({ index, embedQuery: (q: string) => embedQuery(q, provider) });

/** Build a Server bound to the tool registry. Low-level API: JSON Schema in,
 *  { content } out, matching the shapes createTools already produces. */
function buildServer(): Server {
  const server = new Server({ name: 'nym-docs-mcp', version: '0.1.0' }, { capabilities: { tools: {} } });

  server.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: tools.map(({ name, description, inputSchema }) => ({ name, description, inputSchema })),
  }));

  server.setRequestHandler(CallToolRequestSchema, async (req) => {
    const tool = tools.find((t) => t.name === req.params.name);
    if (!tool) return { content: [{ type: 'text', text: `Unknown tool: ${req.params.name}` }], isError: true };
    return tool.handler(req.params.arguments ?? {});
  });

  return server;
}

// Stateless Streamable HTTP: a fresh Server + transport per request. Suitable
// for a single public endpoint with no per-session state.
const http = createServer(async (req, res) => {
  if (req.method === 'POST' && req.url === '/mcp') {
    const transport = new StreamableHTTPServerTransport({ sessionIdGenerator: undefined });
    res.on('close', () => transport.close());
    await buildServer().connect(transport);
    await transport.handleRequest(req, res);
    return;
  }
  res.writeHead(404).end('Not found');
});

http.listen(PORT, () => console.log(`nym-docs MCP on http://localhost:${PORT}/mcp (${tools.length} tools)`));

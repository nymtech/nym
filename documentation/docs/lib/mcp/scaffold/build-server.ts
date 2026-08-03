// MCP server core: wire the tested tool registry onto an SDK Server. Shared by
// both host options (pages-router route, standalone HTTP). Lives under scaffold/
// so its @modelcontextprotocol/sdk import is excluded from `next build` and the
// test suite until you install the SDK.
//
// Needs:  pnpm add @modelcontextprotocol/sdk   (no zod, no mcp-handler)
//
// VERIFIED against @modelcontextprotocol/sdk 1.30.0 type definitions:
//   - The low-level `Server` + setRequestHandler(ListToolsRequestSchema /
//     CallToolRequestSchema) is used deliberately. Its ListTools result carries
//     `inputSchema` as raw JSON Schema, so the JSON Schema in tools.ts passes
//     through verbatim. That is why this needs NO zod: zod is only pulled by the
//     high-level McpServer.registerTool convenience, which we do not use.
//   - Import paths server/index.js, types.js resolve in 1.30.0.
//
// VERIFY ON INSTALL (cannot be exercised in-sandbox):
//   - the two request handlers still return the ServerResult shape the pinned
//     SDK expects (content[] / isError passthrough).

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { ListToolsRequestSchema, CallToolRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import type { McpTool } from '../tools';

/** Build an SDK Server bound to a tool registry. Callers own the transport. */
export function buildMcpServer(tools: McpTool[]): Server {
  const server = new Server({ name: 'nym-docs-mcp', version: '0.1.0' }, { capabilities: { tools: {} } });

  server.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: tools.map(({ name, description, inputSchema }) => ({ name, description, inputSchema })),
  }));

  server.setRequestHandler(CallToolRequestSchema, async (req) => {
    const tool = tools.find((t) => t.name === req.params.name);
    if (!tool) return { content: [{ type: 'text', text: `Unknown tool: ${req.params.name}` }], isError: true };
    // handler already returns { content, isError? }; safe() inside tools.ts turns
    // upstream/network failures into isError results rather than throwing.
    return tool.handler(req.params.arguments ?? {});
  });

  return server;
}

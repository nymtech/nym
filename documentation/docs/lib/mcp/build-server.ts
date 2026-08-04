// MCP server core: wire the tested tool registry onto an SDK Server. Shared by
// both host options: the live pages-router route (pages/api/mcp.ts) and the
// standalone HTTP server (scaffold/standalone.ts). Needs @modelcontextprotocol/sdk
// installed.
//
// The low-level `Server` + setRequestHandler(ListToolsRequestSchema /
// CallToolRequestSchema) is used deliberately: its ListTools result carries
// `inputSchema` as raw JSON Schema, so the JSON Schema in tools.ts passes through
// verbatim. That is why this needs NO zod (zod is only pulled by the high-level
// McpServer.registerTool convenience, which we do not use).
//
// Verified working against SDK 1.30.0: tools/list and tools/call both round-trip
// through the pages-router route over Streamable HTTP (SSE reply path).

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { ListToolsRequestSchema, CallToolRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import type { CallToolResult, ListToolsResult, Tool } from '@modelcontextprotocol/sdk/types.js';
import type { McpTool } from './tools';

/** Build an SDK Server bound to a tool registry. Callers own the transport. */
export function buildMcpServer(tools: McpTool[]): Server {
  const server = new Server({ name: 'nym-docs-mcp', version: '0.1.0' }, { capabilities: { tools: {} } });

  // The handlers are annotated with the SDK's own result types. Without that,
  // the low-level Server types the return as the broad ServerResult union and
  // our hand-rolled shapes don't resolve to the right member. inputSchema in
  // tools.ts is plain JSON Schema (kept SDK-free for the tests), so it is
  // downcast to the SDK's Tool.inputSchema here at the boundary.
  server.setRequestHandler(ListToolsRequestSchema, async (): Promise<ListToolsResult> => ({
    tools: tools.map(({ name, description, inputSchema }) => ({
      name,
      description,
      inputSchema: inputSchema as Tool['inputSchema'],
    })),
  }));

  server.setRequestHandler(CallToolRequestSchema, async (req): Promise<CallToolResult> => {
    const tool = tools.find((t) => t.name === req.params.name);
    if (!tool) return { content: [{ type: 'text', text: `Unknown tool: ${req.params.name}` }], isError: true };
    // handler already returns { content, isError? }; safe() inside tools.ts turns
    // upstream/network failures into isError results rather than throwing.
    return tool.handler(req.params.arguments ?? {});
  });

  return server;
}

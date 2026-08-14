// MCP tool registry: the logic layer, framework-agnostic.
//
// Each tool is a plain { name, description, inputSchema, handler } object whose
// handler returns an MCP-style result ({ content: [{ type: "text", text }] }).
// This keeps the entire tool behaviour testable without the MCP SDK; server.ts
// is a thin transport shell that just registers these against a Server.
//
// Dependencies are injected (index, query embedder, network client) so tests
// run hermetically with fakes and no docs-index.json or live network.

import type { DocIndex } from '../retrieval/types';
import { search, getSection } from '../retrieval/retrieval';
import * as nymApi from '../nym-api/client';
import { unymToNym } from '../nym-api/client';
import { validateSetupMixTunnelOpts } from './validate-config';

export interface McpToolResult {
  content: Array<{ type: 'text'; text: string }>;
  isError?: boolean;
  // MCP tool results may carry additional protocol fields (_meta,
  // structuredContent). The index signature both reflects that and lets this
  // assign to the SDK's CallToolResult, which declares one.
  [key: string]: unknown;
}

export interface McpTool {
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
  handler: (args: Record<string, any>) => Promise<McpToolResult>;
}

/** Only the network-calling client functions are injectable; pure helpers are imported. */
type NymClient = Pick<
  typeof nymApi,
  'getGateway' | 'listGateways' | 'getNetworkSummary' | 'getCirculatingSupply' | 'getChainStatus'
>;

export interface ToolDeps {
  index: DocIndex;
  embedQuery: (query: string) => Promise<number[]>;
  nym?: NymClient;
  // Optional code index (separate model). When both are present, search_code is
  // exposed. embedCode must use the same model the code index was built with.
  codeIndex?: DocIndex;
  embedCode?: (query: string) => Promise<number[]>;
}

const text = (s: string): McpToolResult => ({ content: [{ type: 'text', text: s }] });

/** Wrap a live handler so an upstream/network failure returns an error result
 *  the agent can read, rather than throwing and killing the request. */
function safe(fn: (args: any) => Promise<McpToolResult>) {
  return async (args: any): Promise<McpToolResult> => {
    try {
      return await fn(args);
    } catch (e) {
      return { content: [{ type: 'text', text: `Tool error: ${(e as Error).message}` }], isError: true };
    }
  };
}

// The endpoint is public and every search spends an embedding call, so an
// unbounded query is a way to run up someone else's bill. Generous enough that a
// pasted stack trace or error block still works. Rejected rather than truncated:
// a silently shortened query embeds to a different vector and returns confidently
// wrong neighbours, which is worse than an error the agent can read and act on.
const MAX_QUERY_CHARS = 4000;

function checkedQuery(query: string): string {
  if (typeof query !== 'string' || !query.trim()) {
    throw new Error('query must be a non-empty string.');
  }
  if (query.length > MAX_QUERY_CHARS) {
    throw new Error(
      `query is ${query.length} characters; the limit is ${MAX_QUERY_CHARS}. ` +
        'Search with the specific phrase or symbol you are looking for rather than a whole file.',
    );
  }
  return query;
}

export function createTools(deps: ToolDeps): McpTool[] {
  const nym = deps.nym ?? nymApi;

  return [
    {
      name: 'search_docs',
      description:
        'Search the Nym developer documentation and internal knowledge base for relevant sections. Returns ranked excerpts with deep-link URLs. Use this first for any "how do I / what is" question about Nym.',
      inputSchema: {
        type: 'object',
        properties: {
          query: { type: 'string', description: 'Natural-language search query' },
          topK: { type: 'number', description: 'Max results (default 6)' },
        },
        required: ['query'],
      },
      handler: safe(async ({ query, topK = 6 }) => {
        const vec = await deps.embedQuery(checkedQuery(query));
        const hits = search(vec, deps.index, { topK }); // MCP sees all sources
        if (!hits.length) return text(`No documentation matched "${query}".`);
        return text(
          hits
            .map((h) => `## ${h.chunk.title} - ${h.chunk.heading}\n${h.chunk.url}\n\n${h.chunk.text}`)
            .join('\n\n---\n\n'),
        );
      }),
    },
    {
      name: 'get_section',
      description:
        'Fetch a full documentation section by its id or deep-link URL (as returned by search_docs). Use when a search excerpt is truncated and you need the whole section.',
      inputSchema: {
        type: 'object',
        properties: { ref: { type: 'string', description: 'Chunk id or deep-link URL' } },
        required: ['ref'],
      },
      handler: safe(async ({ ref }) => {
        const s = getSection(deps.index, ref);
        return s ? text(`# ${s.title} - ${s.heading}\n${s.url}\n\n${s.text}`) : text(`No section found for "${ref}".`);
      }),
    },
    {
      name: 'network_summary',
      description:
        'Live counts of bonded nym-nodes from the Node Status API: total, gateways, mixnodes, entry and exit. The counts are independent and do not sum to the total.',
      inputSchema: { type: 'object', properties: {} },
      handler: safe(async () => {
        const s = await nym.getNetworkSummary();
        const g = s.gateways.bonded;
        // Reported flat, with no figure nested inside another. The previous
        // phrasing read `603 bonded (80 entry, 100 exit)`, which asserts a
        // breakdown the upstream numbers do not support: they are independent
        // counts and do not sum to the total.
        return text(
          `Bonded nym-nodes, as reported by the Node Status API: ` +
            `${s.total_nodes} total, ${g.count} gateways, ${s.mixnodes.bonded.count} mixnodes, ` +
            `${g.entry} entry, ${g.exit} exit. ` +
            `These are independent counts rather than a breakdown of the total: they overlap ` +
            `and do not sum. Do not derive figures from them by arithmetic.`,
        );
      }),
    },
    {
      name: 'circulating_supply',
      description: 'Live circulating and total NYM token supply.',
      inputSchema: { type: 'object', properties: {} },
      handler: safe(async () => {
        const s = await nym.getCirculatingSupply();
        return text(
          `Circulating: ${unymToNym(s.circulating_supply).toLocaleString()} NYM of ${unymToNym(s.total_supply).toLocaleString()} total.`,
        );
      }),
    },
    {
      name: 'chain_status',
      description: 'Live Nyx chain connection status as seen by the NymAPI.',
      inputSchema: { type: 'object', properties: {} },
      handler: safe(async () => {
        const s = await nym.getChainStatus();
        return text(`Connected nyxd: ${s.connected_nyxd}. Status: ${JSON.stringify(s.status)}`);
      }),
    },
    {
      name: 'list_gateways',
      description: 'List bonded gateways (paginated) with performance scores.',
      inputSchema: {
        type: 'object',
        properties: {
          page: { type: 'number', description: 'Zero-based page (default 0)' },
          size: { type: 'number', description: 'Page size (default 20)' },
        },
      },
      handler: safe(async ({ page = 0, size = 20 }) => {
        const p = await nym.listGateways(page, size);
        const rows = p.items.map((g) => `- ${g.gateway_identity_key}  perf=${g.performance} routing=${g.routing_score}`).join('\n');
        return text(`${p.total} gateways total. Page ${p.page} (${p.items.length}):\n${rows}`);
      }),
    },
    {
      name: 'get_gateway',
      description: 'Live health and scores for a single gateway by its identity key.',
      inputSchema: {
        type: 'object',
        properties: { identity: { type: 'string', description: 'Gateway identity key' } },
        required: ['identity'],
      },
      handler: safe(async ({ identity }) => {
        const g = await nym.getGateway(identity);
        return text(
          `Gateway ${g.gateway_identity_key}: bonded=${g.bonded}, performance=${g.performance}, routing_score=${g.routing_score}, config_score=${g.config_score}.`,
        );
      }),
    },
    {
      name: 'validate_sdk_config',
      description:
        "Validate a mix-tunnel / mix-fetch setup config (SetupMixTunnelOpts) before writing it into code. Flags field type mismatches (errors) and unknown or typo'd keys (warnings), and notes privacy tradeoffs. The field list is a snapshot of one SDK version, so unknown keys warn rather than fail.",
      inputSchema: {
        type: 'object',
        properties: {
          config: { type: 'object', description: 'The SetupMixTunnelOpts object to check' },
        },
        required: ['config'],
      },
      handler: safe(async ({ config }) => {
        const r = validateSetupMixTunnelOpts(config);
        const lines = [r.valid ? 'Config is valid.' : 'Config has errors.'];
        if (r.errors.length) lines.push('\nErrors:\n' + r.errors.map((e) => `- ${e}`).join('\n'));
        if (r.warnings.length) lines.push('\nWarnings:\n' + r.warnings.map((w) => `- ${w}`).join('\n'));
        return text(lines.join('\n'));
      }),
    },
    // Code search is only available when a code index + matching embedder were
    // injected (they use a different, code-tuned model than the docs index).
    ...(deps.codeIndex && deps.embedCode
      ? [
          {
            name: 'search_code',
            description:
              'Search the Nym source code (SDK crates/packages, wasm, and the Sphinx/smolmix internals) for relevant functions, types and examples. Returns ranked snippets with GitHub deep-links. Use for "how is X implemented / show me the code" questions the prose docs do not answer.',
            inputSchema: {
              type: 'object',
              properties: {
                query: { type: 'string', description: 'Natural-language or code-shaped query' },
                topK: { type: 'number', description: 'Max results (default 6)' },
              },
              required: ['query'],
            },
            handler: safe(async ({ query, topK = 6 }) => {
              const vec = await deps.embedCode!(checkedQuery(query));
              const hits = search(vec, deps.codeIndex!, { topK });
              if (!hits.length) return text(`No source code matched "${query}".`);
              return text(
                hits
                  .map((h) => `## ${h.chunk.title}${h.chunk.heading ? `: ${h.chunk.heading}` : ''}\n${h.chunk.url}\n\n\`\`\`\n${h.chunk.text}\n\`\`\``)
                  .join('\n\n---\n\n'),
              );
            }),
          } as McpTool,
        ]
      : []),
  ];
}

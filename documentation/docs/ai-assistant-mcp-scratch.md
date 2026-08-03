# AI assistant + MCP: worklog & test runbook

Working scratchpad for the docs AI chat + developer MCP server. Design lives in
`ai-assistant-mcp-plan.md`; this file tracks what's been built and how you test
it. Branch: `max/docs-ai-assistant-mcp` (off `max/docs-threat-model-overhaul`).

## What's built (chronological)

1. **Phase 0 spike** - `scripts/next-scripts/generate-index.mjs`. Chunks the docs
   corpus; proved 1394 chunks / ~5.5MB index / ~$0.006 per re-index.
2. **Phase 0.1 retrieval core** - `lib/retrieval/`:
   - `chunker.mjs` - heading-scoped, fence-aware, `-1/-2` anchor de-dup, hard-cap
     splitter, skips generated fig-spec noise.
   - `retrieval.ts` - cosine search with a source filter, plus `getSection`.
   - `embed.mjs` - pluggable provider (Voyage default), content-hash cache so only
     changed chunks re-embed.
   - Generator now imports the shared chunker; fixed a `stripMdx` bug that deleted
     `import` lines inside code examples.
3. **Live Nym API client** - `lib/nym-api/client.ts` (+ `live-check.mjs`). Supply,
   chain status, network summary, gateway list + by-id. Endpoints verified against
   production.
4. **MCP tool layer** - `lib/mcp/tools.ts`. 7 tools (search_docs, get_section, 5
   live) with error-wrapping. `server.ts` is the Streamable HTTP transport shell
   (needs the SDK; unverified scaffold).
5. **Chat logic** - `lib/chat/`: `context.ts` (retrieval -> context + citations,
   docs-only by default), `prompt.ts` (system prompt + guardrails). Route + widget
   are scaffold under `lib/chat/scaffold/` (need the AI SDK; unverified).
6. **Docs bug fix** - corrected stale API example paths in `pages/apis/nym-api.mdx`
   and `ns-api.mdx` (`/v1/mixnodes/active` -> `/v1/nym-nodes/rewarded-set`;
   `/api/v1/gateways` -> `/v2/gateways`).

Test coverage: 42 new tests across `lib/**`, all passing. Scaffold files
(`lib/**/scaffold`, `lib/mcp/server.ts`) are excluded from `next build` (tsconfig
exclude) so the branch keeps building without the extra deps.

## How to test (when you have time)

All commands run from `documentation/docs/`.

### 1. Unit tests (no network, no keys)
```
node_modules/.bin/vitest run lib/
```
Expect 42+ passing. This covers the chunker, retrieval, embed cache, Nym client
(hermetic), MCP tool logic, and chat context/prompt.

### 2. Live Nym APIs (network, no keys)
```
node lib/nym-api/live-check.mjs
```
Hits the real supply / summary / gateway endpoints. Confirms the MCP live tools
still map to production. Fails loudly if an endpoint moved.

### 3. Build a real index (needs Voyage key)
```
VOYAGE_API_KEY=xxx node ../scripts/next-scripts/generate-index.mjs
```
Writes `public/docs-index.json` (~5.5MB, gitignored) with vectors + an embed cache
under `.cache/`. Re-run: only changed chunks re-embed. Without the key it writes a
vectorless index and warns.

### 4. Chat route + widget (needs AI SDK + keys)
```
pnpm add ai @ai-sdk/anthropic @ai-sdk/react
```
- Move `lib/chat/scaffold/chat-route.ts` -> `pages/api/chat.ts`.
- Move `lib/chat/scaffold/ChatWidget.tsx` -> `components/ChatWidget.tsx`, and
  mount `<ChatWidget />` in `pages/_app.tsx` inside the ThemeProvider.
- Set `ANTHROPIC_API_KEY`, `VOYAGE_API_KEY`; optional `CHAT_MODEL`
  (default `claude-haiku-4-5`; set `claude-opus-4-8` for quality).
- `pnpm run dev`, open the docs, click "Ask AI".
- VERIFY: the pages-router streaming call (`pipeDataStreamToResponse`) matches the
  AI SDK version you installed; v5 renamed helpers.

### 5. MCP server (needs SDK + built index)
```
pnpm add @modelcontextprotocol/sdk
```
- Set `VOYAGE_API_KEY`, run `lib/mcp/server.ts` (via tsx or after a build step).
- VERIFY the SDK import paths + Streamable HTTP wiring against the installed
  version (see the header in `server.ts`).
- Point a client at `http://localhost:8787/mcp` and list tools.

## Open decisions (see plan D1-D4)
- D1 embeddings provider (defaulted to Voyage dim 1024).
- D2 generation model + budget (defaulted to Haiku 4.5; abuse protection TODO).
- D3 MCP hosting (subdomain vs /docs/mcp) + rate limiting.
- D4 Confluence sanitise step + re-index cadence.

## Not started
- Phase 1 chat wired live (needs deps + keys).
- Confluence adapter.
- Rate limiting / abuse protection / feedback capture on the chat.
- `validate_sdk_config` MCP tool.

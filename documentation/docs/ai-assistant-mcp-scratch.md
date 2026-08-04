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
4. **MCP tool layer** - `lib/mcp/tools.ts`. 8 tools (search_docs, get_section, 5
   live, validate_sdk_config) with error-wrapping. `validate-config.ts` checks a
   SetupMixTunnelOpts object: field types (errors), unknown/typo'd keys (warnings,
   never errors, since the field list is a one-version snapshot), and privacy
   tradeoffs. The Streamable HTTP transport is under `lib/mcp/scaffold/` (needs
   the SDK; excluded from build + tests): `build-server.ts` (reusable core),
   `mcp-route.ts` (pages-router API route, recommended), `standalone.ts` (separate
   Node server, alternative). Verified against @modelcontextprotocol/sdk 1.30.0
   types: the low-level `Server` passes our JSON-Schema `inputSchema` through
   verbatim, so it needs only the SDK (no zod, no mcp-handler).
5. **Chat logic** - `lib/chat/`: `context.ts` (retrieval -> context + citations,
   docs-only by default), `prompt.ts` (system prompt + guardrails). Route + widget
   are scaffold under `lib/chat/scaffold/` (need the AI SDK; unverified).
6. **Docs bug fix** - corrected stale API example paths in `pages/apis/nym-api.mdx`
   and `ns-api.mdx` (`/v1/mixnodes/active` -> `/v1/nym-nodes/rewarded-set`;
   `/api/v1/gateways` -> `/v2/gateways`).
7. **MCP live route** - `pages/api/mcp.ts` (Streamable HTTP) + core in
   `lib/mcp/build-server.ts`. Validated live: `tools/list` returns 8 tools,
   `tools/call` round-trips (validate + live network tools). Only `@modelcontextprotocol/sdk`
   (no zod). `validate_sdk_config` tool added (`validate-config.ts`).
8. **Deploy wiring (2c)** - `build` now runs `generate-index.mjs` +
   `generate-page-markdown.mjs` before `next build`; `next.config.js` traces
   `docs-index.json` into the `/api/mcp` lambda. Needs `VOYAGE_API_KEY` in Vercel
   + one deploy to confirm the traced read (not verifiable from the sandbox).
9. **Per-page markdown export** - `scripts/next-scripts/generate-page-markdown.mjs`
   emits `public/<path>.md` per page (fence-aware strip). Verified: 190 files, 0
   stray top-level imports, code fences preserved. Output gitignored.

Test coverage: 65 tests across `lib/**`, all passing (includes the privacy-model
tests from the base branch). Every SDK-coupled file now lives under
`lib/**/scaffold`, which tsconfig excludes from `next build`, so the branch keeps
building without the extra deps. (Earlier `lib/mcp/server.ts` was NOT under
scaffold/ and would have broken the build; it was split into the three scaffold
files above.)

## How to test (when you have time)

All commands run from `documentation/docs/`.

### 1. Unit tests (no network, no keys)
```
node_modules/.bin/vitest run lib/
```
Expect 65 passing. This covers the chunker, retrieval, embed cache, Nym client
(hermetic), MCP tool logic (incl. validate_sdk_config), and chat context/prompt.

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

### 4. Chat route + widget (needs AI SDK + keys) - scaffold rewritten for v7

The scaffold under `lib/chat/scaffold/` is now written for **ai@7 + @ai-sdk/react@4**
(verified against the installed type defs), not the old v4 API. To wire it live:
```
pnpm add ai @ai-sdk/anthropic @ai-sdk/react
```
- Move `lib/chat/scaffold/chat-route.ts` -> `pages/api/chat.ts`. Fix its relative
  imports the same way `pages/api/mcp.ts` did (`../../lib/chat/...`, `../../lib/retrieval/...`).
- Move `lib/chat/scaffold/ChatWidget.tsx` -> `components/ChatWidget.tsx`, and
  mount `<ChatWidget />` in `pages/_app.tsx` inside the ThemeProvider.
- `/api/chat` is already in `next.config.js` `outputFileTracingIncludes` (reads the
  same index).
- Set `ANTHROPIC_API_KEY`, `VOYAGE_API_KEY`; optional `CHAT_MODEL`
  (default `claude-haiku-4-5`; set `claude-opus-4-8` for quality).
- `pnpm run dev`, open the docs, click "Ask AI".
- Untestable in the sandbox (needs deps + build). Verify: streaming renders via
  `pipeUIMessageStreamToResponse` + `useChat`; message text comes from
  `message.parts`. Citations-as-links are a follow-up (v7 transport hides response
  headers; the model cites `[n]` inline for now).

### 5. MCP server - VALIDATED WORKING (2026-08-04)

Just the SDK: the low-level `Server` takes JSON Schema, so no zod / mcp-handler.
`build-server.ts` is the reusable core (now in `lib/mcp/`); `pages/api/mcp.ts` is
the live route; `scaffold/standalone.ts` is the separate-deploy alternative.

Setup that was run (dev server, one dep):
```
pnpm add @modelcontextprotocol/sdk
mkdir -p pages/api               # mv will NOT create the parent dir
# route imports reach into lib/ from pages/api/ (../../lib/mcp/...); fixed on move
pnpm run dev
```
Smoke tests (note the dual Accept header, else 406):
```
curl -sS -X POST http://localhost:3000/docs/api/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```
Results:
- `tools/list` -> all 8 tools with correct JSON Schema. The `@hono/node-server`
  wrapped-`res` risk did NOT materialise; the transport works as a pages route.
- `tools/call validate_sdk_config` -> error + typo-warning + privacy note.
- `tools/call network_summary` -> live counts from production.
- Reply arrives as SSE framing (`event: message` / `data:`), the Streamable HTTP
  streaming path.
- Not yet tested: `search_docs` / `get_section` (need `VOYAGE_API_KEY` + a
  vectored index). Standalone alt (separate deploy): `tsx lib/mcp/scaffold/standalone.ts`.
- Still open: rate-limiting/abuse policy on the public endpoint (plan D3).

## Open decisions (see plan D1-D4)
- D1 embeddings provider (defaulted to Voyage dim 1024).
- D2 generation model + budget (defaulted to Haiku 4.5; abuse protection TODO).
- D3 MCP hosting (subdomain vs /docs/mcp) + rate limiting.
- D4 Confluence sanitise step + re-index cadence.

## Not started
- Phase 1 chat wired live (needs deps + keys).
- Confluence adapter.
- Rate limiting / abuse protection / feedback capture on the chat.

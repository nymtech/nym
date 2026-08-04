# Docs AI assistant + developer MCP server: plan

Owner: max. Last updated 2026-08-04.

## Current status (2026-08-04)

The MCP server is built and validated end-to-end; the chat is still scaffold.

| Piece | State |
|---|---|
| Retrieval core (chunker, embed cache, cosine search) | built, tested |
| Live Nym API client (5 tools) | built, tested + verified against production |
| MCP tool layer (8 tools, incl. `validate_sdk_config`) | built, tested |
| MCP transport + route (`pages/api/mcp.ts`, Streamable HTTP) | built, validated live (`tools/list` + `tools/call` over curl) |
| Index build wired into deploy | build-wired + lambda-traced; **needs one Vercel deploy + `VOYAGE_API_KEY` to close (see 3.1)** |
| Per-page markdown export (AI-ready keystone) | built + verified (fence-aware; see 3.5) |
| Chat backend + widget | **built + validated locally** (AI SDK v7, right-sidebar drawer, real streamed answers with citations) |
| Code retrieval index + `search_code` | machinery built + tested (voyage-code-3; see 3.6); needs a keyed build |
| Confluence adapter | not started |

65 tests passing. The MCP server compiles into `next build` and serves 8 tools
over Streamable HTTP. The build now generates `docs-index.json` and traces it into
the `/api/mcp` lambda (3.1); the remaining unknown is Vercel-runtime-only (does the
traced lambda read the file, and is `VOYAGE_API_KEY` set), which a single deploy
resolves. The one thing that cannot be closed from the sandbox is the endpoint
serving in production.

---

Two features, one shared backend:

1. An **AI chat assistant** embedded in the docs site (nym.com/docs).
2. An **MCP server** developers point their agents (Claude Code, Cursor, Claude
   Desktop) at, exposing structured docs retrieval **plus live Nym tools**, and
   additionally fed **private, self-hosted content (Confluence)**.

The core realisation: both are *retrieval over the same corpus*. We already emit
that corpus as a build artifact (`public/llms-full.txt`). This plan turns that
into a shared, source-tagged retrieval index that both consumers load.

---

## 1. Architecture

One build-time index, multiple sources, one public artifact, shared by all
consumers. Confluence content is **sanitised at ingestion** and then treated as
public, so there is no trust boundary to enforce downstream.

```
  SOURCES (adapters, normalised to page records)
    pages/**/*.mdx  ──┐   (source: nym-docs)
    Confluence API  ──┤   (source: confluence) [you host + sanitise, then public]
                      │
                 chunkPages()  ── heading-scoped chunks + deep-link URLs
                      │
             embed step (Voyage / OpenAI)
                      │
               docs-index.json   (single public artifact)
                      │
      ┌───────────────┼──────────────────┐
  ┌───────────┐  ┌──────────┐   ┌──────────────────┐
  │ /api/chat │  │   MCP    │   │  (future clients) │
  │ (widget)  │  │ docs+live│   │                   │
  └───────────┘  └──────────┘   └──────────────────┘
```

### Sanitise at ingestion, not partition at runtime

The Confluence content is fine to be public *after a bit of sanitisation*
(stripping internal hostnames, credentials, names, half-finished notes, etc.).
So the boundary is a **one-time ingestion step**, human-reviewable, rather than a
runtime `visibility` filter enforced by every consumer:

- The Confluence adapter fetches, **sanitises**, and normalises pages, then feeds
  them into the same `chunkPages()` pipeline as the docs.
- Everything lands in one `docs-index.json`. No separate private index, no authed
  internal MCP, no credential-boundary machinery.

We keep a `source` tag on each chunk (for citation provenance: a Confluence-
sourced answer links back to Confluence, a docs answer to the docs page), but
drop the `visibility` field. If genuinely-private content ever appears later, the
artifact-level partition pattern (separate index file, credential-free public
build) is the way to add it, but we do not build for that now.

---

## 2. Spike results (Phase 0)

Script: `documentation/scripts/next-scripts/generate-index.mjs`
Run: `node documentation/scripts/next-scripts/generate-index.mjs [--stats]`

Initial spike run on the `pages/` corpus (public docs only); post-hardening
numbers are noted under the findings below:

| Metric | Value |
|---|---|
| Pages collected | 189 |
| Chunks | 1394 |
| Total tokens (est, chars/4) | ~310,000 |
| Tokens/chunk | min 1, p50 155, p90 524, max 6553 |
| Short chunks (<120 chars) | 99 (7%) |
| Large chunks (>600 tok) | 10 (1%) |
| Text-only JSON | 1.58 MB |
| Full index, dim 1024 float32 | ~5.45 MB |
| Full index, dim 1536 float32 | ~8.17 MB |
| Embedding cost per full re-index | ~$0.006 |

Conclusions:

- Index fits in a serverless bundle; loads into memory on cold start. **No
  vector DB needed.**
- Re-index on every deploy is free in dollar terms. The real cost is build
  latency (one batched embedding call per corpus), not money.
- Pure-vector top-k retrieval is enough to start; add BM25 hybrid later if
  recall on exact terms (crate names, CLI flags) proves weak.

The spike table above is the *initial* Phase 0 run. After the chunker hardening
below, the current index is **1392 chunks, p50 160 / max 600 tokens/chunk, 0
chunks over budget** (from the 2026-08-04 build). The findings the spike surfaced
are all resolved in `lib/retrieval/chunker.mjs`:

1. **Unsplit giant chunks** (RESOLVED). The two 6k-token chunks were the
   auto-generated Fig completion specs under `developers/clients/*/commands`.
   Fixed two ways: a hard char-cap fallback (`hardSplit`) that breaks mid-block,
   and a skip list for the generated fig-spec pages (low retrieval value). Max
   chunk is now 600 tokens; 0 oversized.
2. **Duplicate-heading anchor collisions** (RESOLVED). CLI pages repeat
   "Options" / "Usage" per subcommand, so `slugify` collided and deep links
   pointed at the first match. Fixed with per-page slug dedup (`-1`, `-2`
   suffixes), matching Nextra, so `get_section(url)` and chat citations resolve
   to the right chunk.
3. **`##` inside fenced code mis-parsed as a heading** (RESOLVED). Both
   `stripMdx` and `splitByHeadings` are now fence-aware and never split inside a
   fence, so a `## comment` in a bash example no longer creates a false boundary.
4. **Short/marginal chunks.** 7% of chunks are <120 chars (heading with little
   body). Harmless but noisy; consider folding into the following section.
5. **Token estimate is chars/4.** Fine for sizing; the real embed step should
   use the provider's tokeniser for accurate batching and cost.
6. **`generated` and `Date.*` are stubbed** in the spike (sandbox constraint).
   The real build stamps an ISO timestamp.

---

## 3. Components

### 3.1 Index generator (extend the spike)

- Reuses `generate-llms-txt.mjs` page-walking (`_meta.json` order, MDX strip,
  URL derivation). Adds heading-scoped chunking with deep-link anchors.
- `chunkPages()` is **source-agnostic**: it takes normalised page records
  `{ source, title, description, url, body }`. Any adapter that produces these
  feeds the same pipeline.
- Add the embed step: batch chunks to the embedding API, attach `vector`, write
  `{ schema, generated, embedding:{provider,model,dim}, chunks }` to a single
  `docs-index.json`.
- **Build wiring: DONE.** `generate-index.mjs` now runs in the `build` script
  (before `next build`); a missing `VOYAGE_API_KEY` degrades to a vectorless index
  rather than failing the build. `docs-index.json` stays gitignored (rebuilt per
  deploy). Because the route reads it at runtime and Vercel does not put `public/`
  in the serverless filesystem by default, `next.config.js` traces it into the
  lambda via `outputFileTracingIncludes: { "/api/mcp": ["./public/docs-index.json"] }`.
- **Still to close (Vercel-only, cannot verify from the sandbox):** set
  `VOYAGE_API_KEY` in Vercel (else the deployed index is vectorless and
  `search_docs` returns nothing), and confirm on one deploy that the traced lambda
  actually reads the file. `next dev`/`next start` cannot derisk this: both have
  `public/` on local disk regardless of tracing.
- Confluence is merged in by its own adapter (see 3.2), which may run on a
  separate cadence since Confluence changes independently of docs deploys.
- **Cache vectors by chunk content-hash; only changed chunks re-embed.** This
  drops re-embed latency to near-zero on typical deploys, and, critically,
  decouples the docs build from the embedding API's uptime: a provider outage
  reuses cached vectors instead of failing the build.

### 3.2 Confluence adapter (you host + feed)

- Confluence Cloud/Data Center REST API. Fetch spaces/pages, take body in
  `storage` (XHTML) format, convert to Markdown (turndown or similar).
- **Sanitise** each page (strip internal hostnames, credentials, names,
  half-finished notes) so the content is safe to publish. This is the one place
  the public/private judgement is made; keep it simple and reviewable.
- Emit page records `{ source:'confluence', title, url:<confluence deep link>,
  body }` into `chunkPages()`. Merged into the same `docs-index.json`.
- Auth to Confluence via API token, held in your infra (never in the repo).
- Cadence: scheduled re-index (cron) with CQL `lastModified` for incremental
  updates rather than full re-crawl.

### 3.3 Chat backend + widget

Status: scaffold only. Route + widget live under `lib/chat/scaffold/` (tsconfig-
excluded); the retrieval, context-building and prompt logic they use is built and
tested. Wiring live needs `pnpm add ai @ai-sdk/anthropic @ai-sdk/react` + keys.

- **Use the Vercel AI SDK** (`ai` + `@ai-sdk/anthropic`, MIT), not hand-rolled
  SSE. `useChat` on the client handles streaming message state; `streamText` on
  the server handles the token stream. There is no free drop-in Nextra AI-chat
  plugin because the valuable half (retrieval + inference over your corpus and
  key) is inherently yours to run and cost; the AI SDK gives the free half.
- `pages/api/chat.ts` in this app. Same-origin with the widget (both under
  nym.com/docs) so **no CSP change** is needed. API key stays server-side.
  Nextra 2 is the **pages router**, so streaming uses the AI SDK's Node-response
  path (`pipeDataStreamToResponse`) rather than an app-router Web `Response`.
- Retrieval: `embedQuery` -> cosine top-k (~6), `sources: ["nym-docs"]` (chat
  sees docs only). Runs on the tested `lib/retrieval` core.
- Generation: Claude via `streamText`. Model + budget = decision D2.
- Widget: React component injected via `theme.config.tsx` (Nextra slot);
  floating "Ask AI" button + panel wired to `useChat`. Cites source pages with
  deep links from the retrieved chunks' `url` fields.
- Guardrails from day one: per-IP rate limit, on-topic/anti-jailbreak system
  prompt, thumbs up/down feedback logged for quality measurement.

### 3.4 MCP server

- Transport: **Streamable HTTP** (single endpoint, POST -> JSON or SSE reply).
  This is the current standard; a developer pastes a URL, no npm install.
  Optional stdio variant later for fully-local use.
- Tools:

  | Tool | Tier | Backed by | Status |
  |---|---|---|---|
  | `search_docs(query)` | retrieval | index (docs + sanitised Confluence) | core built (`lib/retrieval`) |
  | `get_section(id \| url)` | retrieval | exact section + deep link | core built |
  | `get_gateway(identity)` | live | `GET /v2/gateways/{id}` (node-status) | client built + verified |
  | `list_gateways(page,size)` | live | `GET /v2/gateways/skinny` (node-status) | client built + verified |
  | `network_summary` | live | `GET /v2/summary` (node-status) | client built + verified |
  | `circulating_supply` | live | `GET /v1/circulating-supply` (nym-api) | client built + verified |
  | `chain_status` | live | `GET /v1/network/chain-status` (nym-api) | client built + verified |
  | `validate_sdk_config` | static | field-type + typo + privacy-tradeoff rules | built (`validate-config.ts`) |

- Live tools are what make this worth building: an agent can already read
  `llms.txt`; it cannot ask "is gateway X in the active set right now?".
- Client lives in `lib/nym-api/client.ts` (hermetic tests) with a manual
  `live-check.mjs` (verified against production). Two bases, verified from the
  OpenAPI specs: NymAPI `https://validator.nymtech.net/api` (`/v1/...`), Node
  Status API `https://mainnet-node-status-api.nymtech.cc` (`/v2/...`).
- **Docs bug found:** `pages/apis/nym-api.mdx` and `ns-api.mdx` show stale
  example paths (`/api/v1/gateways`, `/api/v1/mixnodes/active`) that now 404;
  the live routes are `/v2/gateways` and `/v1/nym-nodes/*`. Worth fixing
  separately, and a neat illustration of why live tools beat static docs.

### 3.5 AI-ready docs surface (programmatic access)

Make the docs consumable by agents and LLMs, not just humans. This is the suite
Mintlify-based sites (e.g. LangChain) expose; we already have the hard part (the
MCP server with live tools) and are missing the cheap parts. The layers compound:

| Layer | What it is | Status |
|---|---|---|
| `llms.txt` + `llms-full.txt` | Index + full-text dump for agents to discover / ingest | Both generated (`public/llms.txt`, `public/llms-full.txt`); not advertised |
| Per-page raw markdown (`<path>.md`) | Every page fetchable as clean markdown | **built + verified** (`generate-page-markdown.mjs`, fence-aware, in `build`) |
| "Copy as Markdown" / "Open in Claude / ChatGPT" | Per-page buttons that deep-link to the `.md` | Not built (skipped for now; needs a browser to verify) |
| MCP server | Docs search + live network tools | Built (3.4) |
| Ask AI | In-page assistant | Planned (3.3) |

**Keystone: per-page `.md` is the primitive the rest lean on.** `llms.txt` is
only an index; it is useful because each entry dereferences to a clean per-page
markdown file (LangChain's `llms.txt` entries are `.md` URLs). We emit the index,
but our pages are HTML/MDX, so there is nothing to dereference to. Build per-page
markdown first; then "copy as markdown" and "open in Claude" are just links to
`<path>.md`, and `llms.txt` becomes followable.

Build:

- **Per-page `.md` export (build step): DONE.** `generate-page-markdown.mjs` walks
  `pages/`, strips MDX/JSX fence-aware, and emits `public/<path>.md` per page
  (`developers/index.mdx` -> `developers.md`; pure-component landing pages with no
  prose are skipped). Wired into `build`; output gitignored. `nym.com/docs/developers/mcp.md`
  then serves raw markdown. Verified: 190 files, 0 stray top-level imports, code
  fences preserved.
- **Theme buttons (not built).** A small component wired via `theme.config.tsx`
  adding "Copy as Markdown" and "Open in Claude / ChatGPT" to each page, pointing
  at the emitted `.md`. Skipped for now: needs a browser to verify.
- **Discovery callout (not built).** Advertise `llms.txt` + the MCP server where
  agents and devs look (the developers front door, and/or a short "AI & agents"
  note). The new `developers/mcp` reference page is the seed of this.

Follow-up (own change, not bundled here): `generate-index.mjs` and
`generate-llms-txt.mjs` still carry two MDX-stripping bugs that `generate-page-markdown.mjs`
now fixes: (1) frontmatter matched with `/m` rather than anchored to the file
head, so a body `---` (e.g. a mermaid `config` header) is eaten on pages without
real frontmatter; (2) `import` lines stripped inside code fences, corrupting code
examples in `llms-full.txt` and the index. Converge both onto this file's
`splitFrontmatter` + `stripJsx`, verified against their outputs. (Surfaced by the
fresh review; the per-page exporter's copies are already fixed.)

### 3.6 Code retrieval index (search_code)

A second retrieval index over selected source code, so agents can search what the
code actually does (the antidote to prose rot). Kept separate from the docs index
because vectors from different embedding models are not comparable.

- **Model:** `voyage-code-3` (code-tuned), its own `public/code-index.json` and
  query embedder; the docs index stays on `voyage-3-large`.
- **Chunker:** `lib/retrieval/code-chunker.mjs` splits source at top-level item
  boundaries (fn / struct / impl / class / export) with a hard char-cap fallback,
  tags `source: nym-code`, and builds GitHub deep links with line numbers.
- **Scope:** the `ROOTS` list in `generate-code-index.mjs` (sdk/, wasm/, examples,
  `common/nymsphinx`, `common/smol-core`); 348 files -> 6023 chunks.
- **Tool:** `search_code` (MCP), exposed only when the code index + a matching
  embedder are wired. Chat stays docs-only.
- **Size:** ~80 MB vectored (JSON floats); int8 quantization (~4x) is the lever if
  the lambda bundle or cold-start parse becomes a concern.

---

## 4. Phasing

| Phase | Deliverable | Status |
|---|---|---|
| 0 | Spike: chunker + real corpus stats | done |
| 0.1 | Hardened chunker + source-filtered retrieval + cached embed step, wired into generator | **done, 22 tests** |
| 2a | MCP tool-logic layer: 9 tools (`search_docs`, `get_section`, `search_code`, 5 live, `validate_sdk_config`) + live Nym client | **done, 71 tests total + live-check** |
| 2b | MCP transport + route (`pages/api/mcp.ts`, Streamable HTTP); core in `lib/mcp/build-server.ts` | **done + validated live** (`tools/list` + `tools/call` over curl; `next build` green) |
| 2c | Index built at deploy + traced into the lambda | build wiring + trace **done**; needs `VOYAGE_API_KEY` in Vercel + one deploy to close (see 3.1) |
| 1 | Chat: `/api/chat` + widget on AI SDK v7 (right-sidebar drawer, model display, MCP reminder) | **done + validated locally** (real streamed answers with citations); staging pending |
| 4 | Confluence adapter (fetch + sanitise) merged into `docs-index.json` | not started; you host |
| 6 | Code retrieval index (`voyage-code-3`) + `search_code` tool (see 3.6) | machinery **done, 71 tests**; needs a keyed build + deploy to go live |
| 5 | AI-ready docs surface: per-page `.md` export (**done**), then copy/open-in-assistant buttons + `llms.txt` discovery callout | keystone `.md` export done (3.5); buttons + callout remain |
| later | hybrid BM25 retrieval; feedback capture; rate-limiting on `/api/mcp` | backlog |

---

## 5. Open decisions

- **D1 (embeddings provider).** Voyage (Anthropic-recommended, dim 1024) vs
  OpenAI `text-embedding-3-small` (dim 1536) vs self-hosted. Drives a second API
  dependency and per-build/query cost. Both must match at index and query time.
- **D2 (generation model + budget).** Haiku 4.5 (cost/latency) vs Sonnet
  (quality). Needs a per-query cost ceiling + abuse protection before public.
- **D3 (MCP hosting).** DECIDED + validated: pages-router API route in the docs
  app (`pages/api/mcp.ts`, public URL `https://nym.com/docs/api/mcp`), shipping
  with the existing Vercel deployment (one service). Uses only
  `@modelcontextprotocol/sdk` (low-level `Server` takes our JSON Schema, so no
  zod); `mcp-handler` was rejected because it forces app-router + `server@2` +
  `zod@4` onto a pages-router, zod-free app. The `@hono/node-server` transport
  accepts Next's wrapped `res` (the main risk, confirmed working). Standalone Node
  server kept as the alternative for a separate `mcp.nym.com` deployable. Still
  open under D3: rate-limiting / abuse policy before the endpoint is public. The
  fresh review confirmed the amplification vector: unauthenticated POSTs can drive
  paid Voyage spend (via `search_docs`) and Nym-API load (via the live tools).
- **D4 (Confluence sanitisation + cadence).** What the sanitise step strips and
  who reviews it; full re-crawl vs incremental (CQL lastModified); job frequency.

## 6. Notes

- Corpus artifacts already exist: `public/llms.txt` (11 KB index),
  `public/llms-full.txt` (1.3 MB, regenerated every build).
- Docs app is its own Vercel deployment serving `nym.com/docs` (marketing site
  is separate); server build (`next build`, no static export), so this app *can*
  serve API routes. **Verify before Phase 1:** confirmed only that the app can
  serve functions, not that a POST+SSE to `nym.com/docs/api/chat` actually
  reaches them through whatever fronts `nym.com`. A proxy tuned for GET page
  requests can silently break streaming POSTs. Discriminating check: send a
  streaming SSE POST through the real front door and confirm it lands and
  streams. Until then treat same-origin chat as unverified.
- Model IDs and exact pricing to be pulled from the `claude-api` reference at
  implementation time, not hardcoded from memory here.

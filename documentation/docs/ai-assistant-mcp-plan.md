# Docs AI assistant + developer MCP server: plan

Status: draft / planning. Owner: max. Last updated by spike run.

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

Measured on the current `pages/` corpus (public docs only):

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

### Findings the production version must handle

1. **Unsplit giant chunks.** The two largest chunks (6553, 6125 tokens) are the
   auto-generated Fig completion specs under `developers/clients/*/commands`.
   They are single unbroken code blocks, so the paragraph splitter cannot break
   them. Fix: a hard char-cap fallback that splits mid-block, or skip/truncate
   generated CLI completion output (low retrieval value). Currently just 2
   chunks, but they blow the per-chunk token budget.
2. **Duplicate-heading anchor collisions.** `slugify("Options")` -> `options`
   every time, and CLI command pages repeat "Options" / "Usage" / "Example
   output" per subcommand. So `id`/anchor collides across chunks and the deep
   link points to the *first* match while the chunk text may be the third. This
   breaks the deep-link value prop (`get_section(url)` and chat citations depend
   on unique anchors). Fix: disambiguate repeats the way Nextra does (`-1`, `-2`
   suffixes).
3. **`##` inside fenced code blocks is mis-parsed as a heading.** `stripMdx`
   doesn't strip fenced code and `splitByHeadings` matches `^#{2,4}` on any
   line, so a `## section` comment in a bash example creates a false chunk
   boundary. Fix: track fenced-code state, never split inside a fence. (Does not
   affect the sizing verdict; total tokens/size hold regardless of boundaries.)
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
- Wire into `build` after `generate-llms-txt`. Confluence is merged in by its own
  adapter (see 3.2), which may run on a separate cadence since Confluence changes
  independently of docs deploys.
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

  | Tool | Tier | Backed by |
  |---|---|---|
  | `search_docs(query)` | retrieval | index (docs + sanitised Confluence) |
  | `get_section(id \| url)` | retrieval | exact section + deep link |
  | `get_node_status(identity)` | live | ns-api / node-status-api |
  | `list_gateways` / `resolve_gateway` | live | nym-api topology |
  | `network_status` (epoch, topology) | live | nym-api |
  | `validate_sdk_config` (later) | live | static rules + version checks |

- Live tools are what make this worth building: an agent can already read
  `llms.txt`; it cannot ask "is gateway X in the active set right now?".

---

## 4. Phasing

| Phase | Deliverable | Notes |
|---|---|---|
| 0 (done) | Spike: chunker + real corpus stats | this doc |
| 0.1 | Harden chunker (giant-chunk fallback), add embed step, wire into build | produces `docs-index.json` with vectors |
| 1 | Chat: `/api/chat` + widget, vector retrieval + Claude + SSE | same-origin, highest visible value |
| 2 | MCP (Streamable HTTP): `search_docs`, `get_section` | reuses Phase 0.1 index |
| 3 | Live Nym tools in MCP | independent surface |
| 4 | Confluence adapter (fetch + sanitise) merged into `docs-index.json` | you host; no separate index |

---

## 5. Open decisions

- **D1 (embeddings provider).** Voyage (Anthropic-recommended, dim 1024) vs
  OpenAI `text-embedding-3-small` (dim 1536) vs self-hosted. Drives a second API
  dependency and per-build/query cost. Both must match at index and query time.
- **D2 (generation model + budget).** Haiku 4.5 (cost/latency) vs Sonnet
  (quality). Needs a per-query cost ceiling + abuse protection before public.
- **D3 (MCP hosting).** Subdomain (`mcp.nym.com`) vs `/docs/mcp` route;
  open + rate-limited (single public endpoint now that there's no private tier).
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

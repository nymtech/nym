# AI assistant, MCP server and machine-readability

The docs are built to be consumed by AI agents and LLMs, not only read. This file
is the contributor reference for that machinery: the retrieval pipeline, how the
MCP route and chat backend serve from it, where the keys go, what is tunable, and
how to test a deployment.

Read this before changing anything under `docs/lib/retrieval/`, `docs/lib/mcp/`,
`docs/pages/api/`, or `scripts/next-scripts/generate-*`.

Two pages cover the same ground for readers rather than contributors, and they
stay published: [`/docs/developers/mcp`](docs/pages/developers/mcp.mdx) is the
tool catalogue and client setup, and
[`/docs/use-with-ai`](docs/pages/use-with-ai.mdx) is the consumer-facing guide.

## What ships

- **Ask AI**, an in-docs chat in the right-hand sidebar that answers from the
  documentation with citations, powered by retrieval plus Claude.
- **An MCP server** at `/docs/api/mcp` (Streamable HTTP). Point a coding agent at
  it for docs search (`search_docs`, `get_section`), source-code search
  (`search_code`), live network tools (`network_summary`, `list_gateways`,
  `circulating_supply`, `chain_status`, `get_gateway`), and `validate_sdk_config`.
- **Per-page Markdown.** Append `.md` to any docs URL, or use the page's Copy
  button, to fetch it as clean Markdown.
- **`llms.txt` and `llms-full.txt`**, generated during the build.

The [MCP server](docs/pages/developers/mcp.mdx) and the in-docs **Ask AI** assistant are two consumers of one thing: a semantic index built at deploy time and shipped as a static file. There is no vector database and no separate service to operate. This page explains that pipeline, for anyone extending it or judging what its answers are worth.

## The shape

Everything is built during the docs deploy and read from disk at runtime.

```text
  pages/**/*.mdx                curated source files
        │                       (sdk/, wasm/, examples, nymsphinx, smol-core)
        │                             │
   chunkPages()                 chunkCode()          heading- and boundary-scoped
        │                             │              chunks, each with a deep link
        └──────────┬──────────────────┘
                   │
            embed (Voyage)                           content-hash cache:
                   │                                 only changed chunks re-embed
        ┌──────────┴──────────┐
  docs-index.json       code-index.json              static artifacts in public/
        │                     │
        ├─────────────┬───────┘
        │             │
   /api/chat      /api/mcp                           both load the index at
   (Ask AI)       (agents)                           cold start, search in memory
```

The `build` script runs the generators before `next build`, in this order: `generate-llms-txt.mjs`, `generate-index.mjs`, `generate-code-index.mjs`, `generate-page-markdown.mjs`.

## Building the index

### Chunking

Pages are split into **heading-scoped chunks** rather than fixed-size windows, so a chunk is a section a reader would recognise and can be cited by its own anchor. Four things the chunker (`lib/retrieval/chunker.mjs`) has to get right:

- **Fence awareness.** A `## comment` inside a bash example is not a heading. Both the MDX strip and the heading split track fence state, so code blocks never create false section boundaries.
- **Anchor de-duplication.** CLI pages repeat "Options" and "Usage" once per subcommand, so naive slugs collide and every deep link points at the first match. Slugs are de-duplicated per page with `-1` / `-2` suffixes, matching Nextra, so `get_section` and chat citations resolve to the section actually quoted.
- **A hard character cap.** Auto-generated Fig completion specs produced 6,000-token chunks that would dominate any retrieval. A `hardSplit` fallback breaks oversized sections mid-block, and the generated fig-spec pages are skipped outright as low retrieval value.
- **Deep-link URLs.** Each chunk carries the URL of its own section, which is what makes citations clickable rather than page-level.

### Embedding

Chunks are embedded with Voyage. The docs index uses `voyage-3-large` (1024-dimensional, unit-normalised); the code index uses `voyage-code-3`, which is trained for source rather than prose.

Vectors are cached by **content hash**, so a rebuild only pays for chunks whose text actually changed. Two consequences worth knowing: an edit to one page does not re-embed the corpus, and an embedding-provider outage reuses cached vectors instead of failing the build.

> [!WARNING]
> The query embedding and the index embedding must come from the same model.
> Changing the model in `embed.mjs` requires a full re-index, because querying a
> `voyage-3` index with `voyage-3-large` vectors returns meaningless neighbours
> rather than an error.


### What the code index covers

The code index covers the crates the documentation makes claims about: the SDKs
and wasm packages, Sphinx, the core clients, the exit services (IPR and Network
Requester), the gateway protocol, credentials, and `nym-node`. The list is
`ROOTS` in `scripts/next-scripts/generate-code-index.mjs`. A path outside it
cannot be cited by `search_code` and cannot be checked against the prose, so
that list is the boundary of what the docs can be held to. Both index files are
traced into the `/api/mcp` lambda and parsed at every cold start, so widen it
because the docs describe something, not because it exists.

### Embedding cost and caching

Vectors are cached by chunk content hash
(`lib/retrieval/embed.mjs`), so a rebuild only embeds chunks whose text actually
changed. The cache key includes the model and dimension, so switching embedding
models invalidates it rather than silently mixing two vector spaces.

The cache lives at `node_modules/.cache/nym-docs/`. Vercel restores that path
between builds automatically; GitHub runners start clean, so `cd-docs.yml` restores
it with `actions/cache`. Either way a full cold rebuild is cheap in money (the docs
index is roughly 300k tokens) and the thing worth avoiding is the build time.

Anything that changes chunk *boundaries* rather than page content, such as
`MAX_CHARS` or the chunker's splitting logic, invalidates effectively every entry.
Expect a slow first build after chunker work.

### Why no vector database

Both indexes are small enough to ship with the app and load into memory on cold start, so retrieval is a cosine scan over an in-memory array rather than a network call:

| Index | Chunks | Dimensions | Model | On disk |
|---|---|---|---|---|
| `docs-index.json` | 1,427 | 1024 | `voyage-3-large` | 19.7 MB |
| `code-index.json` | 6,861 | 1024 | `voyage-code-3` | 94.6 MB |

Almost all of that is the vectors, and most of the vector bytes are serialisation overhead rather than information. Per chunk, the 1,024 numbers occupy 12,697 bytes written as JSON text, against 688 bytes for the id, URL, headings and the chunk text together. So the vectors are 95% of the file, and the text they were derived from is 5%.

A float32 is 4 bytes. Written into JSON it becomes a string such as `-0.017815014`, which costs about 12. That is a 3.1x penalty for holding binary data in a text format. It is paid again at load: the parser converts a decimal string to a double 1,024 times per chunk.

Two cheap levers follow, both well before anything as heavy as a database. Base64-encoding the raw float32 bytes is lossless and cuts the vectors to 5,464 bytes per chunk. Quantising each number to a single signed byte cuts them to 1,024. Measured against this corpus, that returns an identical top ten, with scores shifted by about 0.002.

Re-embedding the whole corpus costs well under a cent, so the real price of a rebuild is build latency rather than money.

A vector database would add a service to operate, a network hop per query, and a consistency problem between the deployed docs and the indexed docs. At this corpus size it buys none of that back. The constraint to watch is the serverless bundle: both files are traced into the MCP function, so roughly 114 MB of the platform's limit is spent before any code, and every cold start pays to parse them.

## Serving

### The MCP route

`pages/api/mcp.ts` mounts the server as a pages-router API route, so it deploys with the docs app rather than as a second service. It is **stateless**: `sessionIdGenerator` is undefined, and a fresh `Server` and transport are constructed per request, which is what suits serverless invocations that share nothing.

The index is loaded once per instance at module scope, not per request, so the cost lands on cold start. On Vercel, `public/` is not automatically available to a lambda, so `next.config.js` traces the artifacts in explicitly:

```js
outputFileTracingIncludes: {
  "/api/mcp": ["./public/docs-index.json", "./public/code-index.json"],
  "/api/chat": ["./public/docs-index.json"],
}
```

The code index is optional. If `public/code-index.json` is absent the route still starts and simply does not expose `search_code`, which keeps a docs-only build working.

### The chat backend

`Ask AI` follows the same retrieval path: the question is embedded, the nearest chunks are fetched from the same index, and those sections are passed to Claude as context with instructions to answer only from them and to cite with inline `[n]` markers. The citation numbers map back to the chunks' deep links, which is why answers point at sections rather than pages.

Because generation is constrained to retrieved context, the assistant reports that something is not covered instead of filling the gap from model priors. That is a deliberate trade: it will decline questions the docs genuinely do not answer.

Those `[n]` markers carry a second job. The widget builds its source list from them, so the model decides which retrieved sections are worth showing. See [which sources an answer lists](#which-sources-an-answer-lists).

## Keys

| Variable | Needed at | Read by | Used for |
|---|---|---|---|
| `VOYAGE_API_KEY` | build **and** runtime | our code, explicitly | Embedding chunks during the build, and embedding each query at call time |
| `ANTHROPIC_API_KEY` | runtime only | the Anthropic provider, implicitly | Generation for the chat assistant |
| `CHAT_MODEL` | runtime, optional | our code | Which model answers in the chat widget |
| `CHAT_MIN_SCORE` | runtime, optional | our code | Similarity floor that keeps distant chunks out of the prompt (default `0.2`) |

`VOYAGE_API_KEY` is needed in **both** places for a reason worth stating: the build embeds the corpus, and every query must be embedded at request time with the same model, or the vectors are not comparable. `ANTHROPIC_API_KEY` never appears in our source because the provider reads it from the environment itself, so it is easy to forget when provisioning.

The MCP server needs only the Voyage key. It returns retrieved sections to the calling agent and leaves generation to that agent's own model, which is why it costs nothing per call beyond one query embedding.


**Where each key must be set.** The two are needed at different stages, so they do
not go in the same place. Setting only one of them fails in a way that looks
unrelated to the key, so check both before debugging anything else.

| Key | GitHub Actions | Vercel project env | Why |
|-----|----------------|--------------------|-----|
| `VOYAGE_API_KEY` | **required** | **required** | The build embeds the corpus, and `/api/chat` and `/api/mcp` embed each incoming query at request time. Both stages call Voyage, so it is needed in both places. |
| `ANTHROPIC_API_KEY` | not needed | **required** | Generation happens only at runtime. Nothing in the build calls Claude. |

`ANTHROPIC_API_KEY` never appears in our source: the `@ai-sdk/anthropic` provider
reads it from the environment itself, so grepping for it finds nothing and it is
easy to forget when provisioning.

Each missing key is made to announce itself rather than degrade quietly:

| Missing | What happens |
|---------|--------------|
| `VOYAGE_API_KEY` at build | The index generators **exit non-zero and fail the build** for anything that ships: Vercel (which sets `VERCEL`), and `cd-docs.yml` (which sets `REQUIRE_EMBEDDINGS`). Everywhere else they warn and write a vectorless index, which is what local work on chunking and the check-only CI builds want. `CI` alone is deliberately not a trigger: `ci-docs.yml` builds to prove the docs compile and has no reason to spend an embedding run. |
| `VOYAGE_API_KEY` at runtime | `/api/chat` refuses with `503` and names the variable; `/api/mcp` throws at cold start with the same message. |
| `ANTHROPIC_API_KEY` | `/api/chat` refuses with `503`. The MCP server is unaffected and stays fully functional: it hands retrieved sections to the calling agent and lets that agent's own model generate. |
| A vectorless index reaching production | Both routes detect it (`embedding.dim` is null) and refuse, rather than serving `200` while every search returns nothing. |

**Health check.** `GET /docs/api/chat` reports whether a deployment is wired up,
so verifying staging is one request rather than asking a question and reading the
tea leaves. It returns `200` with `ok: true` when healthy, `503` with a `problems`
array when not:

```bash
curl -s https://<deployment>/docs/api/chat | jq
# { "model": "claude-sonnet-5", "name": "Claude Sonnet 5", "ok": true, "chunks": 1427 }
```

Keys live in GitHub Actions and Vercel secrets, never in the repo. `VOYAGE_API_KEY`
is already wired into `.github/workflows/cd-docs.yml`.

## Which sources an answer lists

Retrieval returns a fixed number of nearest chunks and passes all of them to the model. The model reads them, answers, and cites the ones it used as `[n]`. The chat widget builds its source list from those markers. A section that is retrieved but not cited is not listed.

This puts the relevance decision with the model. An earlier design put it in a similarity floor (`CHAT_MIN_SCORE`), which discarded chunks below a fixed score. That does not work, and the measurement is worth recording.

Cosine similarity measures the distance between two vectors. It was being read as a measure of whether a question is about Nym. The two come apart when the query is short, because a short query carries little for the embedding to place. Scored against this corpus:

| Query | Best score |
|---|---|
| "Who is L2 and why does it matter?" (five best hits all correct) | 0.504 |
| "What is the capital of France?" | 0.523 |
| "What is a SURB?" | 0.593 |

The off-topic question outscores the correct one. No floor admits the first and rejects the second, so every value tried refused real questions or admitted nonsense. Raising the floor also trimmed sources from answers that worked.

`CHAT_MIN_SCORE` remains, as a cost guard. It keeps clearly distant chunks out of the prompt. It does not decide whether a question can be answered.

Two consequences follow. The model must cite reliably, so the system prompt requires a marker on every factual claim; a weaker instruction empties the source list on good answers. And a question the docs do not cover still reaches the model with context attached, which the model declines to use. The refusal comes from the prompt, not from empty retrieval.

One failure mode comes with the design. An answer cut short by the output-token limit loses the sources it had not yet cited, because the markers never arrive. The list stays accurate for what was written, and a truncated answer is the larger problem to fix.

To watch retrieval quality over time, `scripts/next-scripts/check-retrieval-scores.mjs` prints the scores for on-topic and off-topic queries. Read the gap between the groups, not the absolute numbers.

Exact-term recall is the known weakness. Embeddings handle rare tokens poorly, so crate names, CLI flags and error strings rank worse than prose. A lexical index (BM25) alongside the vector search would address that.


## Tunables
Everything below has a working default; none of it is required to run. Runtime
values can be changed in Vercel without a redeploy of the code.

| Variable | Default | Effect |
|----------|---------|--------|
| `CHAT_MODEL` | `claude-sonnet-5` | Which model answers in the widget. The tier matters here: the job is synthesising an answer from retrieved sections and citing them, and Haiku was visibly weaker at it. |
| `CHAT_MIN_SCORE` | `0.2` | Cosine-similarity floor for retrieval. A cost guard that keeps distant chunks out of the prompt. It does **not** decide whether a question is answerable, and raising it to try is a known dead end, see below. |

**Do not raise `CHAT_MIN_SCORE` to filter out irrelevant questions.** It was
built for that and cannot do it. Cosine similarity measures distance between
vectors, not whether a question is about Nym, and the two come apart on short
queries. Measured against this corpus, "Who is L2 and why does it matter?" tops
out at `0.504` with its five best hits all correct, while "What is the capital of
France?" reaches `0.523`. The off-topic question scores higher, so no floor
separates them.

Which sections an answer lists is decided by the model instead. It cites what it
used as `[n]`, and the widget lists only those. A question the docs do not cover
gets a refusal that cites nothing, so no sources appear under it.

The scoring script still earns its place, for watching retrieval quality when the
corpus, the chunking or the embedded text changes:

```bash
# from documentation/docs, needs a built index
VOYAGE_API_KEY=xxx node ../scripts/next-scripts/check-retrieval-scores.mjs
```

It scores a set of on-topic and off-topic questions and prints both groups. Read
the gap between them, not the absolute numbers: a gap that narrows means
retrieval got worse. Pass your own questions as arguments to check a specific
case.

Compile-time constants, changed in source rather than the environment:

| Constant | Where | Default | Effect |
|----------|-------|---------|--------|
| `MAX_MESSAGES` | `pages/api/chat.ts` | `40` | Longest accepted conversation. A bound on what one caller can push through the paid calls. |
| `MAX_TOTAL_CHARS` | `pages/api/chat.ts` | `32_000` | Largest accepted request body, counted over text parts. |
| `topK` | `pages/api/chat.ts` (`buildContext` call) | `10` | How many chunks are retrieved per question. All of them reach the model; it cites the ones it uses, and only those are listed as sources. |
| `MAX_CHARS` | `lib/retrieval/chunker.mjs` | `2400` | Hard cap on a chunk. Changing it changes chunk boundaries, so it needs a full re-index. |
| `INPUT_MAX_HEIGHT` | `components/ChatWidget.tsx` | `140` | How far the chat textarea grows before it scrolls internally. |

<!-- prettier-ignore -->
> [!IMPORTANT]
> The query embedding and the index embedding must come from the same model.
> Changing the provider or model in `lib/retrieval/embed.mjs` requires a full
> re-index: querying a `voyage-3` index with `voyage-3-large` vectors returns
> meaningless neighbours rather than an error.

## Machine-readable surfaces

The same build emits three static surfaces that need no key at all:

- **Per-page Markdown.** Append `.md` to any docs URL for a clean Markdown version of that page.
- **`llms.txt`** for page discovery, and **`llms-full.txt`** for the whole corpus in one file. Both follow [Cloudflare's approach](https://developers.cloudflare.com/style-guide/how-we-docs/ai-consumability/).

These are generated build artifacts and are gitignored, along with both index files. See [Use these docs with AI](docs/pages/use-with-ai.mdx) for how to consume them.

Locally at `http://localhost:3000/docs/llms.txt` and `.../llms-full.txt`; in production at
[nym.com/docs/llms.txt](https://nym.com/docs/llms.txt) and
[nym.com/docs/llms-full.txt](https://nym.com/docs/llms-full.txt).

## Testing
All commands run from `documentation/docs/`.

**Unit tests** cover the chunkers, retrieval, the embed cache, the Nym API client,
the MCP tool layer including `validate_sdk_config`, and the chat context and
prompt. No network, no keys:

```bash
pnpm test
```

**Live Nym API check** confirms the five network tools still match production. It
hits the real endpoints and fails loudly if one has moved, which is the usual way
a tool silently starts returning nothing:

```bash
node lib/nym-api/live-check.mjs
```

**The endpoint itself** speaks plain JSON-RPC, so `curl` exercises it without an
agent. Note the dual `Accept` header: Streamable HTTP replies as SSE, and the
server returns `406` without both types.

```bash
# List the tools
curl -sS -X POST http://localhost:3000/docs/api/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'

# Call one that needs no key
curl -sS -X POST http://localhost:3000/docs/api/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"network_summary","arguments":{}}}'
```

Replies arrive as an SSE frame (`event: message` / `data: {...}`) wrapping the
JSON-RPC result, not as a bare JSON body.

What each layer needs:

| Tool group | Needs |
|------------|-------|
| `validate_sdk_config` | nothing; pure logic |
| `network_summary`, `circulating_supply`, `chain_status`, `list_gateways`, `get_gateway` | network access to the Nym APIs |
| `search_docs`, `get_section` | `VOYAGE_API_KEY` **and** a vectored `public/docs-index.json` |
| `search_code` | the above plus `public/code-index.json`; the tool is simply not exposed when that file is absent |

Against a deployment, swap the host for the deployment URL. If `tools/list`
answers but `search_docs` fails, that is the Voyage key or a vectorless index
rather than the transport; `GET /docs/api/chat` on the same deployment reports
which.

**All of the above at once, against a deployment.**
`scripts/check-mcp-server.sh` runs 36 checks over HTTP: the tool list, transport
negotiation, retrieval and a `get_section` round-trip, code search, index
coverage, config validation, every live network tool, argument-schema rejection,
error shapes, and the chat health endpoint. It covers what unit tests cannot,
namely that the index was traced into the lambda, that the keys exist in *that*
environment, and that the Nym API still returns the field names the tools read.

The **index coverage** group asserts that `search_code` can cite each root in
`ROOTS`, matching on the returned file path rather than on the search term, since
prose elsewhere in the corpus mentions all of these by name. A failure there
normally means the deployed index predates a `ROOTS` change, not that the tool
broke; rebuild with `VOYAGE_API_KEY` set and redeploy.

```bash
# from documentation/
./scripts/check-mcp-server.sh https://nym.com
./scripts/check-mcp-server.sh https://docs-nextra-git-my-branch.vercel.app "$BYPASS"
```

It exits non-zero on any failure, so it can gate a deploy step. Run it after
anything that touches retrieval, the tool registry or the build pipeline.

`$BYPASS` is only needed for Vercel **preview** deployments, which sit behind
Deployment Protection; without it every request is answered with an HTML login
page and every check fails. Production needs no token. Take the value from
Project Settings, Deployment Protection, Protection Bypass for Automation, and
keep it out of shell history:

```bash
read -rs BYPASS && export BYPASS
```


## Keeping docs honest against the code
The docs assert facts the source can settle: constant values, sizes, types, API
signatures, endpoint paths. The goal is that those never drift from the code. The
principle here is: **generate or project the fact from its single source, and
lean on existing tools, rather than shipping bespoke drift-checkers.** A checker keeps
two copies and diffs them; generation keeps one copy and makes drift impossible.

Where each fact class gets its guard:

| Fact class | Guard (existing tool) |
|------------|-----------------------|
| Wasm-boundary types (e.g. `TunnelState`) | **tsify** generates the TS type from the Rust serde in the wasm build |
| Plain shared Rust types | **ts-rs**, as `common/types` does (host `cargo test` export) |
| SDK API reference | **TypeDoc**, regenerated from the SDK source (a regen-and-diff CI gate keeps it fresh) |
| REST endpoints | the `apis/*.mdx` pages embed the **live OpenAPI spec** via Redoc |
| Links (incl. source links) | **lychee** (`ci-docs-linkcheck.yml`) |

`TunnelState` is the worked example of generation. Because it crosses the wasm
boundary, its TS type is generated by [tsify](https://github.com/madonoharu/tsify)
(the generator the crate already uses for `SetupOpts`): `state.rs` derives `Tsify`
under `#[cfg_attr(target_arch = "wasm32", ...)]` (so tsify stays out of any host
build) and `getTunnelState` returns the typed `TunnelState`, so the wasm build emits
the correct `.d.ts`:

```ts
type TunnelState =
  | { state: "connecting" } | { state: "ready" }
  | { state: "shutting_down" } | { state: "shutdown" }
  | { state: "failed"; reason: FailureReason };
```

That removes the drift at the source (it also corrected `reason`, which the SDK type
had as `string` where the runtime emits a `FailureReason` object). The SDK keeps its
copy of the type inline for now (to avoid a dependency on the unpublished
`smolmix-wasm`); a later step can bridge the generated `.d.ts` to the SDK directly so
even that copy is a projection.

For hand-written prose that states a fact (e.g. "a Sphinx packet is 2413 bytes"),
there is no generator, prose cannot be projected. We rely on careful authoring, which
a survey of the docs found holds up well: the drift concentrated in hand-typed
constants and stale generated docs, not in careful prose.


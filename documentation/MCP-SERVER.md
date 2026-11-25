# MCP server and machine-readability

The docs are built to be consumed by AI agents and LLMs, not only read. This file
is the contributor reference for that machinery: the retrieval pipeline, how the
MCP route serves from it, where the key goes, what is tunable, and how to test a
deployment.

Production reads the index from the public Postgres store (`MCP_RETRIEVAL_SOURCE=store`), which [`MCP-STORE.md`](MCP-STORE.md) describes. This file covers the index build, the embedding, the tools and the checks, which both sources share, and the bundle source. The route reads the bundle when `MCP_RETRIEVAL_SOURCE` is unset, so the bundle is also the rollback.

Read this before changing anything under `docs/lib/retrieval/`, `docs/lib/mcp/`,
`docs/pages/api/`, or `scripts/next-scripts/generate-*`.

Two pages cover the same ground for readers rather than contributors:
[`/docs/use-with-ai/mcp`](docs/content/use-with-ai/mcp.mdx) is the
tool catalogue and client setup, and
[`/docs/use-with-ai`](docs/content/use-with-ai/index.mdx) is the consumer-facing guide.

## What ships

- **An MCP server** at `/docs/api/mcp` (Streamable HTTP). Point a coding agent at
  it for docs search (`search_docs`, `get_section`), source-code search
  (`search_code`), live network tools (`network_summary`, `list_gateways`,
  `circulating_supply`, `chain_status`, `get_gateway`), and `validate_sdk_config`.
- **Per-page Markdown.** Append `.md` to any docs URL, or use the page's Copy
  button, to fetch it as clean Markdown.
- **`llms.txt` and `llms-full.txt`**, generated during the build.

The [MCP server](docs/content/use-with-ai/mcp.mdx) serves one thing: a semantic index of the docs and the code, which producer CI writes to a Postgres store and the route searches. This page explains that pipeline, for anyone extending it or judging what its answers are worth.

## The shape

Everything is built during the docs deploy and read from disk at runtime.

```text
  pages/**/*.mdx                curated source files
        │                       (sdk/, wasm/, examples, crates,
        |                         nymsphinx, smol-core, etc)
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
              /api/mcp                              reads the store (see
              (agents)                              MCP-STORE.md), or the bundle
```

The `build` script runs the generators before `next build`, in this order: `generate-llms-txt.mjs`, `generate-index.mjs`, `generate-code-index.mjs`, `generate-page-markdown.mjs`.

## Where the code is

All paths are under `documentation/`. The chunkers, the embedder, the embed cache, the store schema and the store's read and write functions are in the shared package [`@nymproject/retrieval-indexer`](https://github.com/nymtech/retrieval-indexer). The files below are the nym side: the page walk, the nym-specific content rules, the route and the tools.

### Build time

| File | What it does | Test |
|---|---|---|
| `indexed-sources.mjs` | `ROOTS`: the source trees that the code index reads. `search_code` can cite only a path inside this list | `docs/lib/retrieval/index-coverage.test.ts`: each root has a `path` probe in `mcp-probes/retrieval.json` |
| `scripts/next-scripts/generate-index.mjs` | Builds `public/docs-index.json`: walks the pages, chunks them with the package chunker, embeds the changed chunks | `docs/lib/retrieval/coverage.test.ts` |
| `scripts/next-scripts/generate-code-index.mjs` | Builds `public/code-index.json` from `ROOTS` with the package's tree-sitter chunker | `index-coverage.test.ts`, and the build fails on a root with no chunks or a dropped language |
| `scripts/next-scripts/generate-page-markdown.mjs`, `generate-llms-txt.mjs` | The `.md` copy of each page and `llms-full.txt`, from the same page walk | `pages-source.test.ts` (`absoluteLinks`) |
| `docs/lib/retrieval/pages-source.mjs` | The page walk: reads the content tree in `_meta.js` order, inlines the snippet imports, strips MDX, makes root links absolute, and gives one page record for each page | `pages-source.test.ts`, `coverage.test.ts` |
| `docs/lib/retrieval/projections.mjs` | Turns component-rendered content (the threat-model actors, vectors and scenarios) into text, so the index sees what a reader sees | `projections.test.ts` |
| `docs/lib/retrieval/ts-data.mjs` | Loads a TypeScript data module (`lib/privacy-model/*.ts`) from a plain Node build script | `projections.test.ts` |
| `docs/lib/retrieval/embed-key.mts` | The nym rule for the Voyage key: which builds must have it and fail without it | |
| `docs/lib/retrieval/indexer.mts` | Re-exports the package, so the scripts outside `documentation/docs` can import it | |
| `docs/lib/retrieval/write-store.mts` | Writes the `nym/docs` and `nym/code` shards to the store from the two index files. It embeds nothing: it uses the vectors in the files | `write-store.test.ts` (`fileProvider`) |

### Request time

| File | What it does | Test |
|---|---|---|
| `docs/pages/api/mcp.ts` | The route: reads the environment, chooses the source (`MCP_RETRIEVAL_SOURCE`), makes the store pool, builds the server for each request | `scripts/check-mcp-server.sh` against a deployment |
| `docs/lib/mcp/build-server.ts` | Registers the tools on an MCP SDK `Server` | `tools.test.ts` |
| `docs/lib/mcp/tools.ts` | The tool registry: the name, description, input schema and handler of each of the nine tools, with no SDK dependency | `tools.test.ts` |
| `docs/lib/mcp/validate-args.ts` | Checks the arguments of each call against the tool's JSON Schema before the handler runs | `validate-args.test.ts` |
| `docs/lib/mcp/validate-config.ts` | `validate_sdk_config`: checks a `SetupMixTunnelOpts` object | `validate-config.test.ts` |
| `docs/lib/retrieval/retrieval-source.ts` | The one interface the tools read: the bundle source (`indexSource`) or the store source (`storeSource`), and the retry of a dropped store connection | `retrieval-source.test.ts` |
| `docs/lib/retrieval/retrieval.ts` | The bundle source's search: cosine similarity over the in-memory index, and `getSection` | `retrieval.test.ts` |
| `docs/lib/nym-api/client.ts` | The client for the live tools (Nym API and Node Status API) | `client.test.ts`; `live-check.mjs` checks the real APIs by hand |

The unit tests run with `pnpm test` in `documentation/docs`, and `ci-docs-test` runs them on each push to the docs. They use no network and no store. `check-mcp-server.sh` and `scripts/next-scripts/check-retrieval-scores.mjs` need a deployment or a Voyage key.

## Building the index

### Chunking

Pages are split into **heading-scoped chunks** rather than fixed-size windows, so a chunk is a section a reader would recognise and can be cited by its own anchor. Four things the chunker (`@nymproject/retrieval-indexer`, `chunker`) has to get right:

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
`ROOTS` in `indexed-sources.mjs` (imported by `generate-code-index.mjs`). A path outside it
cannot be cited by `search_code` and cannot be checked against the prose, so
that list is the boundary of what the docs can be held to. Both index files are
traced into the `/api/mcp` lambda and parsed at every cold start, so widen it
because the docs describe something, not because it exists.

### Embedding cost and caching

Vectors are cached by chunk content hash
(`@nymproject/retrieval-indexer`, `embed`), so a rebuild only embeds chunks whose text actually
changed. The cache key includes the model and dimension, so switching embedding
models invalidates it rather than silently mixing two vector spaces.

The cache lives at `node_modules/.cache/nym-docs/`. Vercel restores that path
between builds automatically; GitHub runners start clean, so `cd-docs.yml` restores
it with `actions/cache`. Either way a full cold rebuild is cheap in money (the docs
index is roughly 300k tokens) and the thing worth avoiding is the build time.

Anything that changes chunk *boundaries* rather than page content, such as
`MAX_CHARS` or the chunker's splitting logic, invalidates effectively every entry.
Expect a slow first build after chunker work.

### The size of the bundled index

In bundle mode, the route loads both indexes into memory at cold start, and retrieval is a cosine scan over an in-memory array:

| Index | Chunks | Dimensions | Model | On disk |
|---|---|---|---|---|
| `docs-index.json` | 1,427 | 1024 | `voyage-3-large` | 19.7 MB |
| `code-index.json` | 7,761 | 1024 | `voyage-code-3` | ~103 MB |

The `code-index.json` size is derived, not freshly measured: it scales the earlier 7,108-chunk / 94 MB measurement to the current chunk count. Both index files are gitignored, so neither is checked in to measure directly.

Almost all of that is the vectors, and most of the vector bytes are serialisation overhead rather than information. Per chunk, the 1,024 numbers occupy 12,697 bytes written as JSON text, against 688 bytes for the id, URL, headings and the chunk text together. So the vectors are 95% of the file, and the text they were derived from is 5%.

A float32 is 4 bytes. Written into JSON it becomes a string such as `-0.017815014`, which costs about 12. That is a 3.1x penalty for holding binary data in a text format. It is paid again at load: the parser converts a decimal string to a double 1,024 times per chunk.

Two cheap levers follow. Base64-encoding the raw float32 bytes is lossless and cuts the vectors to 5,464 bytes per chunk. Quantising each number to a single signed byte cuts them to 1,024. Measured against this corpus, that returns an identical top ten, with scores shifted by about 0.002.

Re-embedding the whole corpus costs well under a cent, so the real price of a rebuild is build latency rather than money.

Both files are traced into the MCP function, so roughly 120 MB of the platform's limit is spent before any code, and every cold start pays to parse them. [`MCP-STORE.md`](MCP-STORE.md) explains why the index is in a store.

## Serving

### The MCP route

`pages/api/mcp.ts` mounts the server as a pages-router API route, so it deploys with the docs app rather than as a second service. It is **stateless**: `sessionIdGenerator` is undefined, and a fresh `Server` and transport are constructed per request, which is what suits serverless invocations that share nothing.

In bundle mode, the index is loaded once per instance at module scope, not per request, so the cost lands on cold start. On Vercel, `public/` is not automatically available to a lambda, so `next.config.js` traces the artifacts in explicitly:

```js
outputFileTracingIncludes: {
  "/api/mcp": ["./public/docs-index.json", "./public/code-index.json"],
}
```

In bundle mode, the code index is optional. If `public/code-index.json` is absent, or present but built without vectors, the route still starts and simply does not expose `search_code`, which keeps a docs-only build working.


## The key

One key, `VOYAGE_API_KEY`, needed at **build and runtime**. The build embeds the
corpus; the server embeds each incoming query at call time. Both stages call
Voyage, and a query must be embedded with the same model as the index or the
vectors are not comparable, so it goes in both places.

| Where | Why |
|---|---|
| GitHub Actions secret | `cd-docs.yml` builds the index |
| Vercel project env (Preview and Production) | `/api/mcp` embeds each query |

Nothing generates text, so there is no model key. The server returns retrieved
sections and leaves generation to the calling agent, which is why a call costs one
embedding and nothing else.

A missing key announces itself rather than degrading quietly:

| Missing | What happens |
|---------|--------------|
| At build | The index generators **exit non-zero and fail the build** for anything that ships: Vercel (which sets `VERCEL`), and `cd-docs.yml` (which sets `REQUIRE_EMBEDDINGS`). Everywhere else they warn and write a vectorless index, which is what local work on chunking and the check-only CI builds want. `CI` alone is deliberately not a trigger: `ci-docs.yml` builds to prove the docs compile and has no reason to spend an embedding run. |
| At runtime | `/api/mcp` throws at cold start, naming the variable. An agent otherwise connects, gets a full tool list, and hits an opaque 401 inside its first search. |
| A vectorless index reaching production | The route detects it (`embedding.dim` is null). The docs index refuses at cold start; a vectorless code index is dropped instead, so `search_code` is not exposed rather than the route serving `200` with every code search empty. |

**Health check.** `tools/list` against a deployment is the one-request answer to
"is this wired up", and `scripts/check-mcp-server.sh` runs it with 49 more.

Keys live in GitHub Actions and Vercel secrets, never in the repo.

## Tunables

The server applies **no similarity floor**: `search_docs` returns its top `topK`
and leaves relevance to the calling agent, which knows the user's actual task and
can query again. That is deliberate, and it is worth knowing why no floor was
added.

A floor cannot do the job it looks like it can. Cosine similarity measures
distance between vectors, not whether a question is about Nym, and the two come
apart on short queries. Measured against this corpus, "Who is L2 and why does it
matter?" tops out at `0.504` with its five best hits all correct, while "What is
the capital of France?" reaches `0.523`. The off-topic question scores higher, so
no threshold separates them.

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
| `topK` | `lib/mcp/tools.ts` | `6` | Default results per `search_docs` / `search_code` call; the caller can override it per call. |
| `MAX_CHARS` | `@nymproject/retrieval-indexer`, `chunker` | `2400` | Hard cap on a chunk. Changing it changes chunk boundaries, so it needs a full re-index. |

<!-- prettier-ignore -->
> [!IMPORTANT]
> The query embedding and the index embedding must come from the same model.
> Changing the provider or model in `@nymproject/retrieval-indexer` (`embed`) requires a full
> re-index: querying a `voyage-3` index with `voyage-3-large` vectors returns
> meaningless neighbours rather than an error.

## Machine-readable surfaces

The same build emits three static surfaces that need no key at all:

- **Per-page Markdown.** Append `.md` to any docs URL for a clean Markdown version of that page.
- **`llms.txt`** for page discovery, and **`llms-full.txt`** for the whole corpus in one file. Both follow [Cloudflare's approach](https://developers.cloudflare.com/style-guide/how-we-docs/ai-consumability/).

These are generated build artifacts and are gitignored, along with both index files. See [Use these docs with AI](docs/content/use-with-ai/index.mdx) for how to consume them.

Locally at `http://localhost:3000/docs/llms.txt` and `.../llms-full.txt`; in production at
[nym.com/docs/llms.txt](https://nym.com/docs/llms.txt) and
[nym.com/docs/llms-full.txt](https://nym.com/docs/llms-full.txt).

## Testing
All commands run from `documentation/docs/`.

**Unit tests** cover the chunkers, retrieval, the embed cache, the Nym API client,
and the MCP tool layer including `validate_sdk_config`. No network, no keys:

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
| `search_docs`, `get_section` | `VOYAGE_API_KEY` **and** the store (`RETRIEVAL_STORE_READ_URL`, `RETRIEVAL_STORE_CA_CERT`), or in bundle mode a vectored `public/docs-index.json` |
| `search_code` | the above plus a code shard in the store, or in bundle mode a vectored `public/code-index.json`. In store mode the tool is always listed and returns an error with no code shard; in bundle mode it is not exposed when that file is absent or vectorless |

Against a deployment, swap the host for the deployment URL. If `tools/list`
answers but `search_docs` fails, that is the Voyage key or a vectorless index
rather than the transport. The route throws at cold start when the key is
missing, so `tools/list` failing outright points at the key while `tools/list`
succeeding and `search_docs` failing points at the index.

**All of the above at once, against a deployment.**
`scripts/check-mcp-server.sh` runs 50 checks over HTTP: the tool list, transport
negotiation, retrieval and a `get_section` round-trip, code search, index
coverage, config validation, every live network tool, argument-schema rejection,
and error shapes. The checks are data in two probe files: `mcp-probes/server.json`
(the tool list, config validation, the live tools, the error cases) and
`mcp-probes/retrieval.json` (what the store serves). The runner is
`retrieval-mcp-check` from `@nymproject/retrieval-indexer`, whose README gives
the format. It covers what unit tests cannot,
namely that the route reaches its index, that the keys exist in *that*
environment, and that the Nym API still returns the field names the tools read.

The **index coverage** group asserts that `search_code` can cite each root in
`ROOTS`, matching on the returned file path rather than on the search term, since
prose elsewhere in the corpus mentions all of these by name. A failure there
normally means the deployed index predates a `ROOTS` change, not that the tool
broke: the next store write after the `ROOTS` change adds it. Each root has a
`path` probe in `mcp-probes/retrieval.json`, and `lib/retrieval/index-coverage.test.ts`
fails at PR time if a root in `ROOTS` has no probe, so adding a root means adding
its probe. Prefer a symbol that only one crate defines over a description of what
the crate does: a description competes with every crate that does something
similar, and it stops discriminating as the index grows.

Three places run the probe files. `cd-retrieval-store` runs `retrieval.json`
against production after each store write, so a write that leaves a root
unsearchable turns that run red. nym-vpn-client does the same with its own
probe file, `.github/indexer/mcp-probes.json`, after each release write. The
`mcp-checks` workflow of `nymtech/retrieval-indexer` runs every producer's
probe files against nym.com each day. A failure there opens a tracking issue
and a pass closes it, and a Zulip direct message goes out when the checks start
failing and on each passing run. None of these blocks another repository. A
person runs `check-mcp-server.sh` against a preview.

```bash
# from documentation/
./scripts/check-mcp-server.sh https://nym.com
./scripts/check-mcp-server.sh https://<preview-host> "$BYPASS"
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

That removes the drift at the source. The SDK keeps its
copy of the type inline for now (to avoid a dependency on the unpublished
`smolmix-wasm`); a later step can bridge the generated `.d.ts` to the SDK directly so
even that copy is a projection.

For hand-written prose that states a fact (e.g. "a Sphinx packet is 2413 bytes"),
there is no generator, prose cannot be projected. We rely on careful authoring; drift
concentrates in hand-typed constants and stale generated docs rather than in prose.

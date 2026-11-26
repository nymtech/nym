# MCP server and machine-readability

The docs are built to be read by AI agents as well as by people. This file is the contributor reference for that machinery: what goes into the retrieval index, how producers write it to the public store, how the MCP route answers a call, where the key goes, and how to test a deployment.

Read it before you change anything under `docs/lib/retrieval/`, `docs/lib/mcp/`, `docs/pages/api/`, `scripts/next-scripts/generate-*`, `mcp-probes/` or `indexed-sources.mjs`.

Two pages cover the same ground for readers: [`/docs/use-with-ai/mcp`](docs/content/use-with-ai/mcp.mdx) is the tool catalogue and the client setup, and [`/docs/use-with-ai`](docs/content/use-with-ai/index.mdx) is the guide for consumers. To run a store on your machine, follow [`MCP-STORE-LOCAL.md`](MCP-STORE-LOCAL.md).

## What ships

- **An MCP server** at `/docs/api/mcp` (Streamable HTTP), with docs search (`search_docs`, `get_section`), source-code search (`search_code`), live network tools (`network_summary`, `list_gateways`, `circulating_supply`, `chain_status`, `get_gateway`), and `validate_sdk_config`.
- **Per-page Markdown.** Append `.md` to a docs URL, or use the page's Copy button, to get the page as Markdown.
- **`llms.txt` and `llms-full.txt`**, which the docs build makes.

## Architecture

The index is in a Postgres store with pgvector. Each producer repository writes its own shards from its own CI. The route searches every shard of a modality with one SQL query.

```text
nym CI (push to develop)                 nym-vpn-client CI (published release)
cd-retrieval-store.yml                   cd-retrieval-store.yml
  @nymproject/retrieval-indexer            @nymproject/retrieval-indexer
  chunk; embed only changed chunks         chunk; embed only changed chunks
  (Voyage, input_type "document")          (Voyage, input_type "document")
     │ writer role                              │ writer role
     │ nym/docs, nym/code                       │ nym-vpn-client/code,
     │ links at develop                         │ nym-vpn-client/docs,
     │                                          │ links at the release tag
     ▼                                          ▼
┌──────────────────────────────────────────────────────────────┐
│ public store: Postgres + pgvector                            │
│ shards: one row for each (repository, modality), with model  │
│ chunks: text, URL, content hash, vector                      │
│ one model for each modality: docs voyage-3-large,            │
│                              code voyage-code-3              │
└──────────────────────────────┬───────────────────────────────┘
                               │ TLS, reader role
                               ▼
/docs/api/mcp on Vercel
  embed the query (Voyage, input_type "query")
  SQL: cosine over every shard of the modality, top K
                               ▲
                     agent (MCP client)
```

The shared package [`@nymproject/retrieval-indexer`](https://github.com/nymtech/retrieval-indexer) holds the chunkers, the embedder and its cache, the store schema, the write transaction and the search queries. This repository holds the nym side: the page walk, the nym content rules, the nym producer workflow, the route and the tools. The docs build does not embed and holds no index.

The reasons for a store, and not an index in the function bundle:

1. **More than one repository produces the index.** A bundle holds only what its own build made. With a store, each repository writes its own shard at its own cadence, and the route reads all of them.
2. **Size.** The nym and nym-vpn-client shards are about 22,000 chunks. As JSON in the bundle, that is about 300 MB, which is more than Vercel allows for a function. The store holds them in about 150 MB, with the vectors as binary, and the function holds no vectors.
3. **Roles and transactions.** Producers write with the writer role, and the route reads with a role that cannot change a row. A run replaces a shard in one transaction, so a reader never sees half a shard.

The cost: one more service, one network hop for each search, and a PR preview that answers from the develop index, not from the PR's content.

## A call, end to end

```text
agent ─ POST /docs/api/mcp  tools/call search_docs {query, topK}
  │
  ▼
pages/api/mcp.ts          module scope, once per instance: check VOYAGE_API_KEY,
                          make the pg pool, createTools()
  per request:            a new Server and transport (stateless)
  │
  ▼
lib/mcp/build-server.ts   the SDK Server routes tools/call to the handler
lib/mcp/validate-args.ts  the arguments against the tool's JSON Schema
lib/mcp/tools.ts          query length (1 to 4000 characters), topK (1 to 20, default 6)
  │ embedQuery(query) ───────────────► Voyage, voyage-3-large, input_type "query"
  │ ◄─────────────────────────────── one vector of 1024 numbers
  ▼
lib/retrieval/retrieval-source.ts   storeSource.searchDocs(vector, topK)
  │                                  retryDroppedConnection: one retry on a dropped socket
  ▼
package search(db, 'docs', vector, {topK, model})
  1. select model, dim from shards where modality = 'docs'
     refuse when there is no shard, or the model is not the route's model
  2. SEARCH_SQL: 1 - cosine distance, less 0.05 for a test chunk,
     over the chunks of every docs shard, top K
  │
  ▼
lib/mcp/tools.ts          for each hit: "## <page title> - <heading>", the URL, the text
  │
  ▼
agent
```

`search_code` takes the same path with `voyage-code-3` and the code modality. `get_section` embeds nothing: it reads the docs chunks by id (and the `id~0`, `id~1` parts of a split section, in order), or by URL when no id matches, and joins the parts into one section.

An agent usually calls `search_docs`, then `get_section` on a hit whose full text it needs, and `search_code` for a question about the implementation. A hit holds the text of the inlined snippets and code, so the agent sees the same text as a reader of the page.

### The route

`pages/api/mcp.ts` is a pages-router API route, so it deploys with the docs app. It is **stateless**: `sessionIdGenerator` is undefined, and each request gets a new `Server` and transport, because serverless instances share no memory. It answers `GET` and `DELETE` with `405`, because those are session operations.

The store connection:

| Setting | Value | Why |
|---|---|---|
| `max` | 2 | A small pool for each instance; Vercel runs many instances |
| `connectionTimeoutMillis` | 5 s | An unreachable store fails the call with an error the agent can read |
| `statement_timeout` | 5 s | The server stops a slow query |
| `query_timeout` | 8 s | The client stops waiting on a store that stops answering |
| `idleTimeoutMillis` | 30 s | Idle connections close |
| `retryDroppedConnection` | one retry | A frozen instance runs no timers, so its pool can hold a socket that the pooler closed. The first query after the thaw fails on it. Timeouts are not retried |

An idle connection that dies emits `error` on the pool. The route logs it, because with no listener Node exits the instance.

The route needs `VOYAGE_API_KEY`, `RETRIEVAL_STORE_READ_URL` (the reader connection string) and `RETRIEVAL_STORE_CA_CERT` (the store's CA certificate) in the Vercel project environment. `storeConnection` from the package turns the URL and the CA into the `pg` options. With a CA, the URL must not hold `sslmode` or `sslrootcert`.

## What goes into the index

### Docs: pages, with their snippets inlined

```text
documentation/docs/content/**/*.mdx
  │   import X from '@snippets/...'
  ▼
pages-source.mjs  collectPages
  inlineMdxPartials, for each page:
    @snippets import         ─► the text of the snippet .mdx, in place
                                (.nym-codex/snippets/)
    <Snippet id="...#item"/> ─► a code fence with the code item, which
                                extractCode reads from the source tree
    <Snippet id="..."/>      ─► a mermaid fence, for a diagram (.mmd)
    ](snippet:<id>)          ─► ](/docs/<route>#<anchor>)
  stripMdx: components become text, or are dropped
  projections.mjs: the threat-model actors, vectors and scenarios become text
  │   one PageRecord for each page: title, url, body (plain Markdown)
  ▼
chunkPages (package chunker)
  cuts at the depth 2 to 4 headings; splits a section longer than 2,400 characters
  │   chunk: id, title, heading, url (page + #anchor), text
  ▼
generate-index.mjs ─► public/docs-index.json ─► write-store.mts ─► nym/docs
```

The MCP sees a snippet only as text inside a page section. A snippet has no row of its own, and a hit is always a page section with the page URL and the heading anchor. The snippet records (`lib/snippets/records.mts`) feed the codex push, the `ci-codex-snippets` checks and `benchmark.mts`; the MCP does not read them. `generate-page-markdown.mjs` inlines the snippets in the same way for the Markdown copy of each page.

The chunker must get four things right:

- **Fences.** A `## comment` inside a bash example is not a heading. The MDX strip and the heading split both track fences.
- **Anchor de-duplication.** CLI pages repeat "Options" and "Usage" once for each subcommand. The chunker adds `-1`, `-2` suffixes to repeated slugs, as Nextra does, so each deep link opens the section that the chunk quotes.
- **A hard character cap.** `MAX_CHARS` (2,400) splits a long section into `id~0`, `id~1` parts. The generated Fig completion pages are skipped.
- **Deep links.** Each chunk has the URL of its own section, so a citation opens the section, not the page.

### Code: the source tree

```text
documentation/indexed-sources.mjs  ROOTS
  ▼
collectSourceFiles: skips build output, dependencies and files over the size cap
chunkCodeFile:      tree-sitter for Rust, TypeScript, JavaScript, Go and Python
                    (one chunk for each top-level item); a line rule for Kotlin and Swift
assertNoSilentDrops, assertRootsYieldChunks: the run fails on an unknown
                    language or on a root that gives no chunks
  ▼
generate-code-index.mjs ─► public/code-index.json ─► write-store.mts ─► nym/code
```

`ROOTS` covers the crates that the docs make claims about: the SDKs and wasm packages, Sphinx, the core clients, the exit services, the gateway protocol, credentials and `nym-node`. `search_code` cannot cite a path outside `ROOTS`, and the prose cannot be checked against it. So add a root because the docs describe it, not because it exists. Each root needs a `path` probe in `mcp-probes/retrieval.json` (see "Testing").

## Embedding

```text
                     Nym side                       │  third party: Voyage AI
                                                    │  POST https://api.voyageai.com/v1/embeddings
WRITE (cd-retrieval-store.yml, GitHub runner)       │
  for each chunk:                                   │
    key = model + dimension + sha256(embed text)    │
    key in node_modules/.cache/nym-docs/ ?          │
      yes ─► reuse the vector                       │
             (actions/cache keeps it between runs)  │
      no  ─► request ───────────────────────────────┼─► sends: the chunk texts, the model,
               batches of about 80,000 tokens;      │          input_type "document"
               retry on 429, 5xx, dropped           │   docs: voyage-3-large
               connections                          │   code: voyage-code-3
             vectors ◄──────────────────────────────┼── returns: 1024 numbers for each text
  the store skips a chunk whose hash is unchanged   │
                                                    │
REQUEST (/docs/api/mcp on Vercel)                   │
  search_docs / search_code ─► request ─────────────┼─► sends: the agent's query text,
                                                    │          the model of that modality,
                                                    │          input_type "query"
             query vector ◄─────────────────────────┼── returns: 1024 numbers
  SQL in the store: cosine over the modality, top K │
```

The embed text of a chunk is its title, heading and text. It holds no git ref, so a new commit re-embeds only the chunks whose text changed. Anything that moves chunk boundaries (`MAX_CHARS`, the splitting rules) changes most hashes, so expect a long first run after chunker work. The cache key holds the model and the dimension, so a model change does not mix two vector spaces.

Two kinds of text leave the Nym side: each changed chunk of the public docs and code at write time, and the query text of each search. The live tools call the Nym API directly and use no index.

<!-- prettier-ignore -->
> [!IMPORTANT]
> The query and the index must use the same model, because vectors from two models are not comparable. Each search names the route's model, and the package refuses rows of another model with an error. To change a model, read "Failure modes".

## The store

### Who writes and who reads

| Role | Used by | Can do |
|---|---|---|
| owner | the `store-migrate` workflow of `nymtech/retrieval-indexer`, which a person runs | Create the extension and the tables, and apply migrations |
| writer | producer CI: nym on pushes to develop, nym-vpn-client on each published `nym-vpn-v*` release | Read and change rows in `chunks` and `shards` |
| reader | `/api/mcp` on Vercel, production and preview | Read `chunks` and `shards` only |

nym writes from develop, because Vercel builds nym.com/docs from develop. A run from another branch would put unmerged content in the public index. nym-vpn-client writes its newest published release, with links at the tag, because users run releases and a tag link does not move. Its workflow runs on a published release and by dispatch with a tag, and writes only when the tag is the newest release.

One writer role serves all producers. A role for each producer could not be limited to its own shard without row-level security. The store holds only public content, so the reader credential discloses nothing that is not already public. The writer credential can change what agents read, so only the producer repositories' CI holds it.

### The nym write

`cd-retrieval-store.yml` runs on each push to develop when the repository variable `RETRIEVAL_STORE_ENABLED` is `1`:

1. Restore the embedding cache.
2. `generate-index.mjs` and `generate-code-index.mjs` write `public/docs-index.json` and `public/code-index.json` with vectors. `REQUIRE_EMBEDDINGS=1` makes them fail instead of writing files without vectors.
3. Save the cache, also when a generator fails partway, so a failed run keeps the embeddings it paid for.
4. Check that the commit is still develop's head, so a rerun of an old run does not write old content over new content.
5. `write-store.mts` writes the `nym/docs` and `nym/code` shards with the vectors from the two files. It embeds nothing itself.
6. Run `mcp-probes/retrieval.json` against production, so a write that leaves a root unsearchable fails the run.

Each secret is set only on the step that uses it, so it is not in the environment of `pnpm i` or of the third-party actions. The workflow's `concurrency` group queues a second push behind the first.

### The data model

The package defines the schema, as node-pg-migrate SQL files in its `migrations/` folder.

- `shards` has one row for each (repository, modality): the model, the vector dimension, the source revision and the time of the last run.
- `chunks` has one row for each chunk: the text, the deep link, the shard, a content hash and the vector.

A run (`indexShards`) writes one or more shards of one modality in one transaction. It embeds only the chunks whose content hash changed, deletes the chunks that left each shard, and leaves other shards alone. A store-wide advisory lock serialises the transactions. A run embeds before it takes the lock, and then checks that no other run changed its shard meanwhile. If one did, the run fails with "another run changed it while this one was embedding; run again". A shrink guard refuses a run that would delete a large part of a shard, unless the run allows it. All shards of a modality must use one model.

A chunk that gets a new id is embedded again, even with the same text, because the store compares hashes by id.

### Size and query cost

A 1024-dimension vector takes about 4 KB, and the vectors are most of the store. Search is a sequential scan with no vector index: a `search_code` query reads every code vector. This is fast while the store stays in the database's memory, so the instance needs enough RAM to cache the whole store. A vector index (HNSW) is worth adding at about 100,000 chunks.

## Where the code is

All paths are under `documentation/`.

### Write side

| File | What it does | Test |
|---|---|---|
| `indexed-sources.mjs` | `ROOTS`: the source trees that the code index reads | `docs/lib/retrieval/index-coverage.test.ts`: each root has a `path` probe, and each probe is under a root |
| `scripts/next-scripts/generate-index.mjs` | Writes `public/docs-index.json`: walks the pages, chunks them, embeds the changed chunks | `docs/lib/retrieval/coverage.test.ts` |
| `scripts/next-scripts/generate-code-index.mjs` | Writes `public/code-index.json` from `ROOTS` with the tree-sitter chunker | `index-coverage.test.ts`; the run fails on a root with no chunks or an unknown language |
| `docs/lib/retrieval/pages-source.mjs` | The page walk: reads the content tree in `_meta.js` order, inlines the snippets, strips MDX, makes root links absolute, gives one record for each page | `pages-source.test.ts`, `coverage.test.ts` |
| `docs/lib/retrieval/projections.mjs` | Turns component-rendered content (the threat-model actors, vectors and scenarios) into text | `projections.test.ts` |
| `docs/lib/retrieval/ts-data.mjs` | Loads a TypeScript data module (`lib/privacy-model/*.ts`) from a plain Node script | `projections.test.ts` |
| `docs/lib/retrieval/embed-key.mts` | The nym rule for the Voyage key: a run with `REQUIRE_EMBEDDINGS` or on Vercel fails without it; elsewhere the generators warn and write no vectors | |
| `docs/lib/retrieval/indexer.mts` | Re-exports the package for the scripts outside `documentation/docs` | |
| `docs/lib/retrieval/write-store.mts` | Writes the `nym/docs` and `nym/code` shards from the two index files | `write-store.test.ts` |
| `../.github/workflows/cd-retrieval-store.yml` | The nym producer (see "The nym write") | Its post-write check |

### Request side

| File | What it does | Test |
|---|---|---|
| `docs/pages/api/mcp.ts` | The route: reads the environment, makes the store pool, builds the server for each request | `scripts/check-mcp-server.sh` against a deployment |
| `docs/lib/mcp/build-server.ts` | Registers the tools on an MCP SDK `Server` | `tools.test.ts` |
| `docs/lib/mcp/tools.ts` | The tool registry: the name, description, input schema and handler of each tool, with no SDK dependency | `tools.test.ts` |
| `docs/lib/mcp/validate-args.ts` | Checks the arguments of each call against the tool's JSON Schema | `validate-args.test.ts` |
| `docs/lib/mcp/validate-config.ts` | `validate_sdk_config`: checks a `SetupMixTunnelOpts` object | `validate-config.test.ts` |
| `docs/lib/retrieval/retrieval-source.ts` | `RetrievalSource`, the interface the tools read; `storeSource`; `retryDroppedConnection` | `retrieval-source.test.ts` |
| `docs/lib/nym-api/client.ts` | The client for the live tools (Nym API and Node Status API) | `client.test.ts`; `live-check.mjs` checks the real APIs |

### Checks

| File | What it does |
|---|---|
| `mcp-probes/server.json` | Probes for the tool list, config validation, the live tools and the error cases |
| `mcp-probes/retrieval.json` | Probes for what the store serves: docs retrieval, a `get_section` round trip, code search, and a `path` probe for each root |
| `scripts/check-mcp-server.sh` | Runs both probe files against a deployment with `retrieval-mcp-check` from the package |
| `scripts/next-scripts/check-retrieval-scores.mjs` | Prints the scores of on-topic and off-topic queries against a local `docs-index.json` |
| `docs/lib/snippets/benchmark.mts` | hit@k and MRR of a labelled query set |

The unit tests run with `pnpm test` in `documentation/docs`, and `ci-docs-test` runs them on each push to the docs. They use no network and no store.

## The key

One key, `VOYAGE_API_KEY`, in two places:

| Where | Why |
|---|---|
| GitHub Actions secret of each producer repository | The write embeds the changed chunks |
| Vercel project environment (Preview and Production) | `/api/mcp` embeds each query |

The docs build does not need it. Nothing generates text, so there is no model key: the server returns sections and leaves generation to the agent, so a call costs one embedding.

| Missing | What happens |
|---|---|
| In the write | The generators fail, because the workflow sets `REQUIRE_EMBEDDINGS` |
| In the route | `/api/mcp` throws at cold start and names the variable. Otherwise an agent connects, gets the tool list, and gets an opaque 401 inside its first search |

Keys are in GitHub Actions and Vercel secrets, never in the repository.

## Tunables

The server applies **no similarity floor**. `search_docs` returns its top `topK` and leaves relevance to the agent, which knows the task and can query again. A floor cannot separate on-topic from off-topic queries: cosine similarity measures the distance between vectors, not whether a question is about Nym. On this corpus, "Who is L2 and why does it matter?" tops out at `0.504` with its five best hits correct, and "What is the capital of France?" reaches `0.523`.

To watch retrieval quality when the corpus, the chunking or the embed text changes, build a local index and run the scoring script:

```bash
# from documentation/docs
read -rs VOYAGE_API_KEY && export VOYAGE_API_KEY
node ../scripts/next-scripts/generate-index.mjs
node ../scripts/next-scripts/check-retrieval-scores.mjs
```

It prints the scores of on-topic and off-topic queries. Read the gap between the two groups: a gap that narrows means retrieval got worse. Pass your own queries as arguments.

| Constant | Where | Default | Effect |
|---|---|---|---|
| `topK` | `lib/mcp/tools.ts` | `6` | Results for each `search_docs` and `search_code` call, at most 20; the caller can set it |
| `MAX_CHARS` | the package chunker | `2400` | The cap on a chunk. A change moves chunk boundaries, so the next write re-embeds most chunks |
| `TEST_SCORE_PENALTY` | the package search | `0.05` | Subtracted from the score of a test chunk |

## Machine-readable surfaces

The docs build makes three static surfaces that need no key:

- **Per-page Markdown.** Append `.md` to a docs URL.
- **`llms.txt`** for page discovery, and **`llms-full.txt`** for the whole corpus in one file. Both follow [Cloudflare's approach](https://developers.cloudflare.com/style-guide/how-we-docs/ai-consumability/).

They are build output and are gitignored. See [Use these docs with AI](docs/content/use-with-ai/index.mdx). Locally they are at `http://localhost:3000/docs/llms.txt` and `.../llms-full.txt`; in production at [nym.com/docs/llms.txt](https://nym.com/docs/llms.txt) and [nym.com/docs/llms-full.txt](https://nym.com/docs/llms-full.txt).

## Testing

All commands run from `documentation/docs/`.

**Unit tests** cover the page walk, the projections, the store source, the store write, the Nym API client and the tool layer. No network, no keys:

```bash
pnpm test
```

**Live Nym API check.** It calls the real endpoints and fails if one has moved, which is how a live tool usually starts to return nothing:

```bash
node lib/nym-api/live-check.mjs
```

**The endpoint** speaks JSON-RPC, so `curl` can call it. Send both `Accept` types: Streamable HTTP replies as SSE, and the server returns `406` without both.

```bash
# List the tools
curl -sS -X POST http://localhost:3000/docs/api/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'

# Call a live tool
curl -sS -X POST http://localhost:3000/docs/api/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"network_summary","arguments":{}}}'
```

The reply is an SSE frame (`event: message` / `data: {...}`) around the JSON-RPC result.

The route starts only when `VOYAGE_API_KEY` and `RETRIEVAL_STORE_READ_URL` are set, so every tool needs them. To run it locally, follow [`MCP-STORE-LOCAL.md`](MCP-STORE-LOCAL.md). Each tool group also needs:

| Tool group | Also needs |
|---|---|
| `validate_sdk_config` | Nothing |
| `network_summary`, `circulating_supply`, `chain_status`, `list_gateways`, `get_gateway` | Network access to the Nym APIs |
| `search_docs`, `get_section` | A reachable store with a docs shard (`RETRIEVAL_STORE_CA_CERT` for the public store) |
| `search_code` | A code shard. With no code shard, the tool returns an error |

If `tools/list` fails, the route did not start: read the cold-start error, which names the missing variable. If `tools/list` answers and `search_docs` fails, read the tool error: it names the store failure or the model mismatch.

**All of the above against a deployment.** `scripts/check-mcp-server.sh` runs the tool list, transport negotiation, retrieval, a `get_section` round trip, code search, index coverage, config validation, each live tool, argument rejection and error shapes. The checks are data in the two probe files, and the runner is `retrieval-mcp-check` from the package, whose README gives the format. It tests what unit tests cannot: that the route reaches the store, that the keys exist in that environment, and that the Nym API still returns the fields the tools read.

```bash
# from documentation/
./scripts/check-mcp-server.sh https://nym.com
./scripts/check-mcp-server.sh https://<preview-host> "$BYPASS"
```

It exits non-zero on a failure. Run it after a change to retrieval, the tool registry or the route. `$BYPASS` is only for Vercel preview deployments, which are behind Deployment Protection: without it, each request gets an HTML login page. Take the value from Project Settings, Deployment Protection, Protection Bypass for Automation, and keep it out of the shell history:

```bash
read -rs BYPASS && export BYPASS
```

The **index coverage** probes check that `search_code` can cite each root in `ROOTS`. They match the returned file path, not the search term, because the prose names all the roots. A failure usually means the store has not been written since a `ROOTS` change. `index-coverage.test.ts` fails at PR time when a root has no probe. Prefer a symbol that only one crate defines to a description of what the crate does: a description competes with each crate that does something similar.

Three places run the probe files, and none blocks another repository:

| Where | When | What |
|---|---|---|
| `cd-retrieval-store.yml` in nym | After each write | `retrieval.json` against production |
| `cd-retrieval-store.yml` in nym-vpn-client | After each release write | Its `.github/indexer/mcp-probes.json` |
| `mcp-checks` in `nymtech/retrieval-indexer` | Each day, and by dispatch | The probe files of each producer against nym.com. A failure opens a tracking issue and a pass closes it. A Zulip direct message goes out when the checks start to fail, and on each passing run |

## Failure modes

| Failure | Effect | What to do |
|---|---|---|
| The store is unreachable or slow | `search_docs`, `search_code` and `get_section` return a tool error. The live tools keep working | Each query fails on its own limit: 5 s to connect, 5 s `statement_timeout` (`canceling statement due to statement timeout`), 8 s `query_timeout` (`Query read timeout`). A tool call that runs two queries can take twice as long. When both pooled connections are busy, a third call waits up to 5 s |
| No shard for a modality, or a shard with another model | The search of that modality returns a tool error, not an empty result | The error names the modality, and for another model, both models |
| A producer stops writing | Its shard ages, and the answers drift from the source | `staleShards` in the package reports the age of each shard. A write that leaves a root unsearchable fails the producer's run at its post-write check |
| Two runs write the same shard at once | One fails and must run again | The CI `concurrency` group prevents it |
| The model of a modality changes | Every shard of the modality needs new vectors. One producer cannot switch alone: the package rejects its run while other shards keep the old model | Turn writes off in each producer of the modality (`RETRIEVAL_STORE_ENABLED=0`). Write all shards of the modality by hand in one `indexShards` call with the new model. Do not delete the shards first: the delete cascades to their chunks. Change the model in `pages/api/mcp.ts` and `write-store.mts` in one PR and deploy it. Searches of that modality return an error between the write and the deploy. Then turn writes on again |
| The store is lost | The index is gone until the producers write again | Create the database, the roles and the schema (`bootstrap/init.sql` and the `store-migrate` workflow in [`nymtech/retrieval-indexer`](https://github.com/nymtech/retrieval-indexer)), then run each producer once. The cost is one full embedding run |

## Keeping docs honest against the code

The docs state facts that the source can settle: constant values, sizes, types, API signatures, endpoint paths. Those must not drift from the code. The rule: **generate or project the fact from its single source with an existing tool, instead of a custom drift checker.** A checker keeps two copies and compares them; generation keeps one copy.

| Fact class | Guard (existing tool) |
|---|---|
| Wasm-boundary types (for example `TunnelState`) | **tsify** generates the TS type from the Rust serde in the wasm build |
| Plain shared Rust types | **ts-rs**, as `common/types` does (host `cargo test` export) |
| SDK API reference | **TypeDoc**, generated again from the SDK source (a regenerate-and-diff CI gate keeps it current) |
| REST endpoints | the `apis/*.mdx` pages embed the **live OpenAPI spec** with Redoc |
| Links (also source links) | **lychee** (`ci-docs-linkcheck.yml`) |

`TunnelState` is the worked example. It crosses the wasm boundary, so [tsify](https://github.com/madonoharu/tsify) (which the crate already uses for `SetupOpts`) generates its TS type: `state.rs` derives `Tsify` under `#[cfg_attr(target_arch = "wasm32", ...)]`, so tsify stays out of host builds, and `getTunnelState` returns the typed `TunnelState`, so the wasm build emits the correct `.d.ts`:

```ts
type TunnelState =
  | { state: "connecting" } | { state: "ready" }
  | { state: "shutting_down" } | { state: "shutdown" }
  | { state: "failed"; reason: FailureReason };
```

The SDK keeps its own copy of the type for now, to avoid a dependency on the unpublished `smolmix-wasm`. A later step can bridge the generated `.d.ts` to the SDK.

Prose that states a fact (for example "a Sphinx packet is 2413 bytes") has no generator. Careful authoring is the guard. Drift collects in hand-typed constants and in generated docs that nobody generated again.

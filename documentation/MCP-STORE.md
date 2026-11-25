# The public retrieval store

This page explains the Postgres store that holds the MCP server's index, and the files in the function bundle that it replaces. It also explains what the index holds, how embedding works, how an agent uses the tools, and why the index needs a database. It is for contributors who change the retrieval pipeline or the `/api/mcp` route, and for whoever runs the store.

Status: production reads the store (`MCP_RETRIEVAL_SOURCE=store`), and nym CI writes it on each push to develop (`cd-retrieval-store.yml`, with `RETRIEVAL_STORE_ENABLED=1`). The docs build still makes and bundles the index files, so removing `MCP_RETRIEVAL_SOURCE` and redeploying returns the route to the bundle. [`MCP-SERVER.md`](MCP-SERVER.md) describes the bundle path. The public store is created with `bootstrap/init.sql` and the `store-migrate` workflow in [`nymtech/retrieval-indexer`](https://github.com/nymtech/retrieval-indexer). To run a store on your machine, follow [`MCP-STORE-LOCAL.md`](MCP-STORE-LOCAL.md).

## What goes into the index today

Each docs build runs two generators. `generate-index.mjs` makes the docs index from the pages, and `generate-code-index.mjs` makes the code index from the source.

### Docs: pages, with their snippets inlined

```
documentation/docs/content/**/*.mdx               211 pages; 194 import snippets
  │   import X from '@snippets/...'
  ▼
pages-source.mjs  collectPages
  inlineMdxPartials, for each page:
    @snippets import         ─► the text of the snippet .mdx, in place
                                (.nym-codex/snippets/, 1,172 files)
    <Snippet id="...#item"/> ─► a code fence with the code item, which
                                extractCode reads from the source tree
    <Snippet id="..."/>      ─► a mermaid fence, for a diagram (.mmd, 11 files)
    ](snippet:<id>)          ─► ](/docs/<route>#<anchor>)
  stripMdx: components become text, or are dropped
  │   one PageRecord for each page: title, url, body (plain Markdown)
  ▼
chunkPages (package chunker)
  cuts at the depth 2 to 4 headings; splits a section longer than 2,400 characters
  │   chunk: id, title, heading, url (page + #anchor), text
  ▼
public/docs-index.json                            2,039 chunks
```

So the MCP sees a snippet only as text inside a page section. A snippet has no row of its own, and a hit is always a page section with the page URL and the heading anchor. The snippet records (`lib/snippets/records.mts`: `prose.jsonl`, `code.jsonl` and the others) feed the codex push, the `ci-codex-snippets` checks and `benchmark.mts`. The MCP does not read them. `generate-page-markdown.mjs` inlines the snippets in the same way for the Markdown copy of each page (`/docs/<route>.md`), so an agent that fetches a page directly also sees the snippet text.

A planned change: each snippet that has a record becomes one chunk, and the chunker keeps only the text that no record covers.

### Code: the source tree

```
documentation/indexed-sources.mjs  ROOTS
  ▼
collectSourceFiles: skips build output, dependencies and files over the size cap
chunkCodeFile:      tree-sitter for Rust, TypeScript, JavaScript, Go and Python
                    (one chunk for each top-level item); a line rule for Kotlin and Swift
assertNoSilentDrops, assertRootsYieldChunks: the build fails on an unknown
                    language or on a root that gives no chunks
  ▼
public/code-index.json                            7,788 chunks, GitHub links at develop
```

## How embedding works today

```
                     Nym side                       │  third party: Voyage AI
                                                    │  POST https://api.voyageai.com/v1/embeddings
                                                    │  (hosted service; its region is not stated)
BUILD TIME (Vercel build of the docs site)          │
  for each chunk:                                   │
    key = model + dimension + sha256(embed text)    │
    key in node_modules/.cache/nym-docs/            │
           *embed-cache.json ?                      │
      yes ─► reuse the vector                       │
             (Vercel keeps the cache between builds)│
      no  ─► request ───────────────────────────────┼─► sends: the chunk texts, the model,
               batches of about 80,000 tokens;      │          input_type "document"
               retry on 429, 5xx, dropped           │   docs: voyage-3-large
               connections                          │   code: voyage-code-3
             vectors ◄──────────────────────────────┼── returns: one vector of 1024
                                                    │            numbers for each text
  vectors go into the JSON as decimal text          │
  (about 12 bytes for each number)                  │
  no VOYAGE_API_KEY on Vercel ─► the build fails    │
                                                    │
REQUEST TIME (/docs/api/mcp, a Vercel function      │
  in the project's region; Vercel's default is      │
  iad1, Washington, D.C.)                           │
  search_docs / search_code ─► request ─────────────┼─► sends: the agent's query text,
                                                    │          the model of that index,
                                                    │          input_type "query"
             query vector ◄─────────────────────────┼── returns: one vector of 1024 numbers
  cosine similarity with every chunk vector         │
  in memory (vectors are unit length;               │
  a test chunk loses 0.05)                          │
  top K: default 6, at most 20 ─► the agent         │
```

The index and the query must use the same model, because vectors from two models are not comparable. The same `VOYAGE_API_KEY` serves the build and the route. So two kinds of text leave the Nym side: every changed chunk of the public docs and code at build time, and the query text of every search. Both are public or come from the agent; no private content is indexed on this side. The private side has its own embedding service and never calls Voyage (see "The private side"). The live tools (`network_summary`, `list_gateways` and the others) call the Nym API directly and use no index.

## How an agent uses the tools today

```
agent ──search_docs {query, topK}──► /docs/api/mcp
        ◄── for each hit: "## <page title> - <heading>", the URL, the chunk text
agent ──get_section {ref: chunk id or URL}──► the whole section of that chunk
agent ──search_code {query, topK}──► code chunks, each with a GitHub link at develop
```

An agent usually calls `search_docs`, then `get_section` on a hit whose text it needs in full, and `search_code` for a question about the implementation. A hit on a split page holds the inlined snippet text and the inlined code, so the agent gets the same text that a reader sees on the page.

## How the MCP reads today

The index files are not in git: `documentation/docs/.gitignore` excludes `public/docs-index.json` and `public/code-index.json`.

```
Vercel docs build (production builds from develop)
  generate-index.mjs, generate-code-index.mjs (above)
  next.config.js traces both files into the /api/mcp function

/api/mcp, at cold start: readFileSync both files, parse them into memory
/api/mcp, per request:   embed the query with Voyage, scan the arrays in memory
```

## Why the MCP needs a database

The bundle works for one repository. The plan needs more than one producer and a private side, and the bundle cannot serve either.

Measured on a local build of 2026-11-18, with the four shards that the local store holds:

| Content | Chunks | Size in the bundle (JSON) |
|---|---|---|
| nym docs | 2,039 | 28.1 MB |
| nym code | 7,788 | 107.6 MB |
| nym-vpn-client code | 11,954 | about 165 MB (estimated from the nym code size for each chunk) |
| nym-vpn-client docs | 238 | about 3 MB (estimated) |
| Total | 22,019 | about 304 MB |

The local store holds the same four shards in 143 MB, with the vectors as binary.

The reasons, most important first:

1. **More than one producer writes the index.** A bundle holds only the index that its own build made. For `nym-vpn-client` to be searchable, its chunks must be in the nym docs build, so the nym build must check out, chunk and embed another repository. A store lets each repository write its own shard from its own CI, at its own time, and the route reads all of them.
2. **Private content needs a store of its own.** The public/private boundary is physical: the public store holds no private row. Private content cannot go in a bundle that a public deployment serves. The private store runs the same schema and package on private infrastructure.
3. **The bundle does not grow with the corpus.** With the vpn shards, the JSON is about 304 MB, which is more than the 250 MB that Vercel allows for an uncompressed function. In memory, each number is an 8-byte double, so 22,019 vectors take about 180 MB of heap on each warm instance, and each cold start parses all of it again.
4. **A docs build stops being an embedding run.** Today each build needs the Voyage key and embeds the changed chunks. A PR preview builds and serves its own index.

The cheaper fixes solve some of this, but not all of it:

| Fix | What it solves | What it leaves |
|---|---|---|
| Vectors as base64 float32 (5.4 KB for each chunk) or int8 (1 KB) in the JSON (see `MCP-SERVER.md`) | The bundle size, for a time | Reasons 1, 2 and 4 |
| One JSON file for each shard in object storage, which the route downloads | Reason 1 in part | Each cold start downloads and parses every shard. A writer has no transaction, no role of its own and no skip of unchanged rows |
| Postgres with pgvector | All four | A service to run, and a network hop for each query |

Postgres also gives three properties that the plan depends on:

- **Roles.** Each producer writes with the writer role. The route reads with a role that cannot change a row.
- **Transactions.** A run replaces a shard in one transaction, so a reader never sees half a shard.
- **The scan runs next to the data.** The function holds no vectors: it sends one query vector and gets back K rows.

The cost: one more service (Exoscale Postgres), one network hop for each search, and a PR preview answers from the develop index, not from the PR's content.

## How the MCP reads after the cutover

```
nym CI (push to develop)                 nym-vpn-client CI (push to develop)
cd-retrieval-store.yml                   cd-retrieval-store.yml
  @nymproject/retrieval-indexer            @nymproject/retrieval-indexer
  chunk; embed only changed chunks         chunk; embed only changed chunks
  (Voyage, input_type "document")          (Voyage, input_type "document")
     │ writer role                              │ writer role
     │ nym/docs, nym/code                       │ nym-vpn-client/code,
     │                                          │ nym-vpn-client/docs
     ▼                                          ▼
┌──────────────────────────────────────────────────────────────┐
│ public store: Postgres + pgvector (Exoscale)                 │
│ shards: one row for each (repository, modality), with model  │
│ chunks: text, URL, content hash, vector                      │
│ one model for each modality: docs voyage-3-large,            │
│                              code voyage-code-3              │
└──────────────────────────────┬───────────────────────────────┘
                               │ TLS, reader role
                               ▼
/docs/api/mcp on Vercel   (MCP_RETRIEVAL_SOURCE=store)
  embed the query (Voyage, input_type "query")
  SQL: cosine over every shard of the modality, top K
                               ▲
                     agent (MCP client)
```

The route uses the package's read side: `search` and `getSection` from `@nymproject/retrieval-indexer/search`. These load no parser. A `search` call runs two queries: one reads the model of the modality, one scans its rows. `getSection` runs one, or two when it falls back from the chunk id to the URL. The query embedding does not change.

The route reads the store when `MCP_RETRIEVAL_SOURCE` is `store`, with the reader connection string in `RETRIEVAL_STORE_READ_URL` and the store's CA certificate in `RETRIEVAL_STORE_CA_CERT`. When the variable is unset, empty, or `bundle`, it reads the bundled files as before. Each search names the model the route embedded its query with, and the package refuses rows of another model, because such a query would return confident, wrong neighbours. In store mode `search_code` is always listed; with no code shard, it returns an error.

## The private side (planned)

A second store holds the private content. The public side never touches it.

```
public repositories (nym, nym-vpn-client)        private repositories
        │                                                │
        │ CI, as above                                   │
        ▼                                                │
  public store ──► public MCP (/docs/api/mcp, Vercel)    │
                                                         │
private infrastructure                                   │
┌────────────────────────────────────────────────────────┼──────┐
│ private pipeline: the same indexer package             │      │
│   reads the private repositories ◄─────────────────────┘      │
│   and the public ones                                         │
│   embeds with the private model, through the                  │
│   embedding service (self-hosted, next to Postgres)           │
│   writes every shard: the private store is a superset         │
│        ▼                                                      │
│ private store: Postgres + pgvector                            │
│        ▼                                                      │
│ private MCP (Go): search_docs, search_code                    │
│   embeds the query with the same embedding service            │
│   per-person tokens, stored hashed, with an expiry            │
└───────────────────────────────────────────────────────────────┘
```

Each MCP reads one store. The private MCP needs neither the public store nor Voyage, because its store holds the public content too. No credential for the private store or the private repositories leaves the private infrastructure. The model of the private store is an open decision.

## What changes for contributors

| Area | Before | After |
|---|---|---|
| Docs build | Embeds the changed chunks, and needs `VOYAGE_API_KEY` | Does not embed. The two `outputFileTracingIncludes` entries go |
| `/api/mcp` | Reads two JSON files from the bundle | Reads the store. It still needs `VOYAGE_API_KEY` at runtime to embed queries |
| When the index changes | On each deployment | On each producer run that writes. A PR preview answers from the develop index, not from the PR's content |
| Other repositories | Not indexed | Each writes its own shard |

The last step of the cutover removes the bundled files and the embedding from the docs build. Until then, the bundle is the rollback.

## Who writes and who reads

| Role | Used by | Can do |
|---|---|---|
| owner | the `store-migrate` workflow of `nymtech/retrieval-indexer`, which a person runs | Create the extension and the tables, and apply later migrations |
| writer | producer CI: nym on pushes to develop, and later `nym-vpn-client` | Read and change rows in `chunks` and `shards` |
| reader | `/api/mcp` on Vercel, production and preview | Read `chunks` and `shards` only |

The public store runs on our own Postgres server on Exoscale. One writer role serves all producers: a role for each producer could not be limited to its own shard without row-level security, which the design dropped.

Only pushes to develop write, because Vercel builds the production docs from develop. A run from another branch would put unmerged content in the public index.

Two write runs on one shard must not overlap. A run embeds before it takes the store lock, and when it takes the lock it checks that no other run changed its shard in the meantime. If one did, the run fails with "another run changed it while this one was embedding; run again". The workflow's `concurrency` group queues a second run behind the first, so two quick pushes to develop do not fail a job.

The store holds only public content, so the reader credential discloses nothing that is not already public. The writer credential can change what agents read, so it goes only to the CI of the producer repositories. The private store is a separate database on Nym infrastructure, and no credential for it reaches Vercel or a public repository's CI.

## The data model

The package defines the schema, as node-pg-migrate SQL files in its `migrations/` folder (from `v0.3.0`).

- `shards` has one row for each (repository, modality): its model, the vector dimension, the source revision, and the time of the last run.
- `chunks` has one row for each chunk: its text, the deep link, the shard it belongs to, a content hash, and the vector.

A run (`indexShards`) writes one or more shards of one modality in one transaction. It embeds only the chunks whose content hash changed, deletes the chunks that left each shard, and leaves the other shards alone. A store-wide advisory lock serialises the transactions. All shards of a modality must use the same model, and the package rejects a run that would mix two models in one modality.

A chunk that gets a new id is embedded again, even if its text is the same, because the store compares hashes by id.

## Size and query cost

Measured on 2026-11-13: the chunks of a local nym build (2026-11-07) loaded into PGlite with pgvector, with one 1024-dimension vector for each chunk:

| Content | Chunks | Size |
|---|---|---|
| nym docs | 2,020 | 13.6 MB |
| nym docs and code | 9,582 | 63.7 MB |

The vectors take most of the space: about 4 KB for each 1024-dimension vector. At 20,000 chunks the store is about 130 MB.

Search is a sequential scan with no vector index. A `search_code` query reads every code vector, about 31 MB at 7,800 code chunks. This is fast when the data stays in the database's memory, so the instance needs enough RAM to cache the whole store. A vector index (HNSW) is worth adding only at about 100,000 chunks.

## Failure modes

| Failure | Effect | Detection |
|---|---|---|
| The store is unreachable or slow | `search_docs`, `search_code` and `get_section` return a tool error the agent can read. The live network tools keep working | Each query fails on its own limit: 5 s to connect, the route's 5 s `statement_timeout` for a slow query (`canceling statement due to statement timeout`), and 8 s for a store that stops answering (`Query read timeout`). A tool call that runs two queries can take twice as long. When both pooled connections of an instance are busy, a third call waits up to 5 s for one. A query on a connection that was dropped while the instance was frozen is retried once |
| The store has no shard for a modality, or a shard with another model | The search of that modality returns a tool error, not an empty result | The error text names the modality, and for another model, both models |
| A producer stops writing | Its shard ages, and the answers drift from the source | `staleShards` in the package reports the age of each shard. No job runs it yet |
| Two runs write the same shard at once | One fails and must run again | The CI `concurrency` group prevents it |
| The model of a modality changes | Every shard of the modality needs new vectors. One producer cannot switch alone: the package rejects its run while other shards keep the old model | Run all producers of the modality in one `indexShards` call, one run for each shard, so that the shards change model in one transaction. Do not delete the shards first: the delete cascades to their chunks, and the modality has no rows until the producers commit. The route embeds queries with the model set in `pages/api/mcp.ts`, and each search refuses rows of another model, so change the model there and in `write-store.mts` in the same PR. From the deploy until the producers have written the new vectors, searches of that modality return an error. The develop push that deploys the route also starts the nym producer, so the gap is the time between the two. The code modality has producers in two repositories (nym and nym-vpn-client), and each CI writes only its own shard, so their runs cannot share one transaction. So turn writes off in both (`RETRIEVAL_STORE_ENABLED=0`), write both code shards by hand in one `indexShards` call with the new model, deploy the route change, and turn writes on again |
| The store is lost | The index is gone until the producers run again | Create the database, the roles and the schema first (`bootstrap/init.sql` and the `store-migrate` workflow in [`nymtech/retrieval-indexer`](https://github.com/nymtech/retrieval-indexer)), then run each producer once. The cost is one full embedding run |


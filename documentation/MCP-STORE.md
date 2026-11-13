# The public retrieval store

This page explains the planned move of the MCP server's index from files in the function bundle to a Postgres store. It is for contributors who change the retrieval pipeline or the `/api/mcp` route, and for whoever runs the store.

Status (2026-11-13): planned, not built. The work is the OpenSpec change `add-shared-retrieval-index` (tasks 4.1 and 5.0 to 5.4). Until task 5.3 flips the route, [`MCP-SERVER.md`](MCP-SERVER.md) describes the system that runs. To create the store, follow [`MCP-STORE-SETUP.md`](MCP-STORE-SETUP.md).

## How the MCP reads today

The index files are not in git: `documentation/docs/.gitignore` excludes `public/docs-index.json` and `public/code-index.json`. Each docs build makes them.

```
Vercel docs build (production builds from develop)
  generate-index.mjs, generate-code-index.mjs
    embed the changed chunks with Voyage (VOYAGE_API_KEY in the Vercel env;
    a content-hash cache skips the unchanged ones)
    write public/docs-index.json and public/code-index.json, vectors inside
  next.config.js traces both files into the /api/mcp function

/api/mcp, at cold start: readFileSync both files, parse them into memory
/api/mcp, per request:   embed the query with Voyage, scan the arrays in memory
```

`MCP-SERVER.md` puts the two files at about 120 MB of the function bundle: the docs index measured, the code index estimated from an earlier measurement.

## Why the index moves

- The bundle carries about 120 MB before any code, and each cold start parses it.
- One deployment can serve only the index that its own build made. Other repositories (`nym-vpn-client` first) cannot add their docs and code to it without being part of the nym docs build.
- Every docs build is also an embedding run.

A store separates the index from the deployment. Each repository writes its own part of the index from its own CI, and the MCP route reads all of it.

## How the MCP reads after the cutover

```
producer CI (nym on pushes to develop; later nym-vpn-client)
  chunk and embed with @nymproject/retrieval-indexer
  write one shard per (repository, modality) to the public store   writer role

public Postgres + pgvector
  tables: shards, chunks

/api/mcp, per request:
  embed the query with Voyage (unchanged)
  SQL over TLS, through a pooler                                   reader role
```

The route is to use the package's read side: `search`, `getSection`, and `modalities` from `@nymproject/retrieval-indexer/search`. These load no parser. A `search` call runs two queries: one reads the model of the modality, one scans its rows. The query embedding does not change.

## What changes for contributors

| Area | Before | After |
|---|---|---|
| Docs build | Embeds the changed chunks, and needs `VOYAGE_API_KEY` | Does not embed (task 5.3). The two `outputFileTracingIncludes` entries go |
| `/api/mcp` | Reads two JSON files from the bundle | Reads the store. It still needs `VOYAGE_API_KEY` at runtime to embed queries |
| When the index changes | On each deployment | On each producer run that writes. A PR preview answers from the develop index, not from the PR's content |
| Other repositories | Not indexed | Each writes its own shard |

The cutover goes in steps, so the live route never depends on an untested path. Task 5.0 checks that the package reproduces the live results. Task 5.1 writes to the store and keeps the bundled files. Task 5.2 reads the store behind a flag on a preview. Task 5.3 flips the flag and removes the bundled files.

## Who writes and who reads

| Role | Used by | Can do |
|---|---|---|
| owner | a person, once | Create the extension and the tables (`ensureSchema`) |
| writer | producer CI: nym on pushes to develop, and later `nym-vpn-client` | Read and change rows in `chunks` and `shards` |
| reader | `/api/mcp` on Vercel, production and preview | Read `chunks` and `shards` only |

The public store runs on our own Postgres server on Exoscale. One writer role serves all producers: a role for each producer could not be limited to its own shard without row-level security, which the design dropped.

Only pushes to develop are to write, because Vercel builds the production docs from develop. A PR run that wrote would put unmerged content in the public index, so the plan for task 5.1 is that PR runs build the chunks and do not write.

Two write runs on one shard must not overlap. A run embeds before it takes the store lock, and when it takes the lock it checks that no other run changed its shard in the meantime. If one did, the run fails with "another run changed it while this one was embedding; run again". So two quick pushes to develop would fail one job. The write job needs a CI `concurrency` group, so that a second run waits for the first (task 5.1).

The store holds only public content, so the reader credential discloses nothing that is not already public. The writer credential can change what agents read, so it goes only to the CI of the producer repositories. The private store is a separate database on Nym infrastructure (design decisions 1 and 2), and no credential for it reaches Vercel or a public repository's CI.

## The data model

The package defines the schema (`SCHEMA` in `store.ts`).

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

Search is a sequential scan with no vector index (design decision 5). A `search_code` query reads every code vector, about 31 MB today. This is fast when the data stays in the database's memory, so the instance needs enough RAM to cache the whole store. A vector index (HNSW) is worth adding only at about 100,000 chunks.

## Failure modes

| Failure | Effect | Detection |
|---|---|---|
| The store is unreachable | `search_docs`, `search_code` and `get_section` fail | The route should return an error, not an empty result. Task 5.2 has to decide this |
| A producer stops writing | Its shard ages, and the answers drift from the source | `staleShards` in the package reports the age of each shard. No job runs it yet |
| Two runs write the same shard at once | One fails and must run again | The CI `concurrency` group prevents it |
| The model of a modality changes | Every shard of the modality must be embedded again. One producer cannot switch alone: the package rejects its run while other shards keep the old model | Delete the shards of the modality (owner), then run each producer |
| The store is lost | The index is gone until the producers run again | Run each producer once. The cost is one full embedding run |


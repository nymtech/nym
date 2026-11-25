# Add Shared Retrieval Index

## Why

The docs MCP server at `nym.com/docs/api/mcp` works, but its index is baked into the docs deploy artefact. Two JSON files (`docs-index.json`, `code-index.json`) are generated during the Next.js build, traced into the `/api/mcp` lambda, and parsed at every cold start. At current chunk counts that is roughly 120 MB of the serverless bundle before any code runs (MCP-SERVER.md's measured file sizes scaled to today's chunk counts; the doc's own 114 MB figure is stale), and the number grows with every page and every widened code root.

The current setup also blocks two things we now want:

- **Indexing other repos.** `nym-vpn-client` (40 Rust crates, four platform apps) is not indexed at all, and the index build only sees the repo it runs in.
- **A store that grows with the corpus.** Each producer must write its own shard from its own CI, and the route must read all of them, without the bundle size limit.

## What Changes

- Extract the retrieval pipeline (chunkers, the tree-sitter item finder, the generic MDX handling, embedder, code walk, upsert) from `documentation/docs/lib/retrieval/` into one TypeScript indexer package that every producer imports, nym included. The seams for this (injectable `siteUrl`, `source`, `expand`, `values`) already exist; the remaining nym-specific values are module constants. The nym page collector stays in nym, because it joins the docs content with the codex snippets (design decision 4).
- Replace the bundled JSON files and the build-time embed caches with a **public Postgres + pgvector store**. Chunks are upserted by id with a content hash, so unchanged chunks are never re-embedded. Retrieval stays a brute-force cosine scan; at 10-30k chunks no ANN index is needed.
- The public store holds no private row. Private content is in a separate store, outside this change.
- Move index production to a push model: each public producer repo (`nym`, `nym-vpn-client`) runs the shared indexer in its own CI and upserts its shard to the public store.
- Embedding provider decided: Voyage for the public store (`design.md` Q1).
- Fix the stale claims and two small guard gaps found in `documentation/MCP-SERVER.md` and the MCP route during research (see Impact).
- Later stage: a NymVPN docs site (reader-facing) on the same Nextra machinery as the current docs, fed by the public `nym-vpn-client` crate and platform READMEs (design decision 9).

## Capabilities

### New Capabilities

- `docs-shared-index`: multi-repo retrieval index in a public Postgres + pgvector store that holds no private row, produced by a shared indexer package run from each producer's CI.
- `docs-site-surface`: a browser-facing docs surface for its readers, a sibling projection of the same content sources as the index (not built by querying a store). Serves a single visibility fail-closed, gated by deployment protection, with content surfaces verified as projections of their source. Content sources are a later stage (stage 7); this capability fixes only the surface's invariants.

### Modified Capabilities

<!-- none: the public MCP endpoint URL and tool surface are unchanged; this change replaces the storage and build pipeline behind them. The new capability's delta spec is in specs/docs-shared-index/. -->

## Impact

- `documentation/docs/lib/retrieval/`, `documentation/docs/lib/mcp/`, `documentation/scripts/next-scripts/generate-index.mjs`, `generate-code-index.mjs`: refactored into / consuming the shared indexer package. The `/api/mcp` route stops reading bundled files and queries the public store.
- `documentation/docs/next.config.js`: `outputFileTracingIncludes` for the index files removed; the docs deploy artefact shrinks by ~120 MB.
- `.github/workflows/cd-docs.yml` and a new workflow in `nym-vpn-client`: run the indexer, need database credentials as secrets.
- New infrastructure: a public Postgres + pgvector store (may be managed).
- Runtime behaviour of the public MCP server is unchanged for clients: same URL, same tools, same result shapes. Cold starts get cheaper; each query gains one database round trip.
- Documentation fixes riding along: `MCP-SERVER.md` stale `ROOTS` location (line 83), stale sizing figures, the vectorless-index guard that only covers the docs index, and `search_code` being exposed when a vectorless code index parses.

## Non-Goals

- No chat assistant. The `/api/chat` scaffold stays unwired.
- No OAuth or identity-provider build-out.
- No ANN index, no quantisation work. Both become unnecessary once the index leaves the lambda bundle.
- No per-audience tagging and no per-audience index scoping. The spike (E2) resolved the corpus-scope question (design Q3) toward a single broad public index over the nym product code (plus docs and `nym-vpn-client`) with corpus hygiene, not narrowing and not tagging: operators are public-only and need the breadth, and it does not degrade developers or reviewers once non-product noise is excluded.

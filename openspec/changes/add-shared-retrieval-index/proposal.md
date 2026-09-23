# Add Shared Retrieval Index

## Why

The docs MCP server at `nym.com/docs/api/mcp` works, but its index is baked into the docs deploy artefact. Two JSON files (`docs-index.json`, `code-index.json`) are generated during the Next.js build, traced into the `/api/mcp` lambda, and parsed at every cold start. At current chunk counts that is roughly 120 MB of the serverless bundle before any code runs (MCP-SERVER.md's measured file sizes scaled to today's chunk counts; the doc's own 114 MB figure is stale), and the number grows with every page and every widened code root.

The current setup also blocks three things we now want:

- **Indexing other repos.** `nym-vpn-client` (40 Rust crates, four platform apps) is not indexed at all, and the index build only sees the repo it runs in.
- **A private tier.** The `nym-vpn-api` backend source lives in the private `websites` repo (`www/vpn-api`). We want it searchable by our own tools, but nothing about it can appear in a public index. The current pipeline publishes every indexed page four ways (rendered page, raw `.md`, `llms-full.txt`, `docs-index.json`), so there is no safe place for private content in it.
- **Provider independence.** Voyage AI trains on API content by default; opt-out is a dashboard toggle and is prospective only. Sending private backend source to Voyage is not acceptable, and continuing to depend on that toggle for public content is worth questioning too.

## What Changes

- Extract the retrieval pipeline (page collector, chunkers, embedder, upsert) from `documentation/docs/lib/retrieval/` into a shared indexer package that any repo's CI can run. The seams for this (injectable `siteUrl`, `source`, `expand`, `values`) already exist; the remaining nym-specific values are module constants.
- Replace the bundled JSON files and the build-time embed caches with **two physically separate Postgres + pgvector stores**: a public store and a private store. Chunks are upserted by id with a content hash, so unchanged chunks are never re-embedded. Retrieval stays a brute-force cosine scan; at 10-30k chunks no ANN index is needed.
- Split public and private by **physical store separation**, not a role inside one table. The public store holds only public rows, so no query through the public MCP can return private content, because the data is not there. The private store, on separate infrastructure, holds the private content beside a re-embedded copy of the public content, so the access-protected MCP answers over both from one store in one embedding space. Only the private store must be self-hosted; the public store holds world-readable content and may be managed.
- Move public index production to a push model: each public producer repo (`nym`, `nym-vpn-client`) runs the shared indexer in its own CI and upserts its public shard to the public store. The private store is filled by a pipeline on the private infrastructure that indexes the private `websites` source and the public repos with the private model, so private repo access and the private store's write credential never leave that infrastructure.
- Embedding provider decided (Option B): Voyage for the public store, a self-hosted model (Qwen3-Embedding-0.6B or similar) for the private store. The two paths and the rationale are recorded in `design.md` Q1.
- Fix the stale claims and two small guard gaps found in `documentation/MCP-SERVER.md` and the MCP route during research (see Impact).
- Later stage: a NymVPN docs site (reader-facing) on the same Nextra machinery as the current docs, fed by the public `nym-vpn-client` crate and platform READMEs. Its API reference (the vpn-api openspec specs and Redoc over their `openapi.json`, both in the private `websites` repo) is private-visibility content on the access-protected deployment, not the public site (design decision 9).

## Capabilities

### New Capabilities

- `docs-shared-index`: multi-repo retrieval index in two physically separate Postgres + pgvector stores (a public store and a self-hosted private store), with public/private separation enforced by which store holds the rows, produced by a shared indexer package run from each producer's CI and from a private-infrastructure pipeline.
- `docs-site-surface`: a browser-facing docs surface for its readers, a sibling projection of the same content sources as the index (not built by querying a store). Serves a single visibility fail-closed, gated by deployment protection, with content surfaces verified as projections of their source. Content sources are a later stage (stage 7); this capability fixes only the surface's invariants.

### Modified Capabilities

<!-- none: the public MCP endpoint URL and tool surface are unchanged; this change replaces the storage and build pipeline behind them. The new capability's delta spec is in specs/docs-shared-index/. -->

## Impact

- `documentation/docs/lib/retrieval/`, `documentation/docs/lib/mcp/`, `documentation/scripts/next-scripts/generate-index.mjs`, `generate-code-index.mjs`: refactored into / consuming the shared indexer package. The `/api/mcp` route stops reading bundled files and queries the public store.
- `documentation/docs/next.config.js`: `outputFileTracingIncludes` for the index files removed; the docs deploy artefact shrinks by ~120 MB.
- `.github/workflows/cd-docs.yml` and new workflows in `nym-vpn-client` and `websites`: run the indexer, need database credentials as secrets. `VOYAGE_API_KEY` is removed or scoped down depending on the embedding decision.
- New infrastructure: a public Postgres + pgvector store (may be managed) and a self-hosted private Postgres + pgvector store on Nym servers, an embedding inference service (required for the private store, and for the public store too under Option A), the private-infrastructure indexer that fills the private store, and the private MCP deployment.
- Runtime behaviour of the public MCP server is unchanged for clients: same URL, same tools, same result shapes. Cold starts get cheaper; each query gains one database round trip.
- Documentation fixes riding along: `MCP-SERVER.md` stale `ROOTS` location (line 83), stale sizing figures, the vectorless-index guard that only covers the docs index, and `search_code` being exposed when a vectorless code index parses.

## Non-Goals

- No chat assistant. The `/api/chat` scaffold stays unwired.
- No OAuth or identity-provider build-out. The private MCP (used by staff and external reviewers alike) authenticates with per-person expiring tokens checked by the service itself; OAuth 2.1 is the upgrade path if per-user identity demands it. The docs site can still use platform access protection, since it carries no private index content.
- No ANN index, no quantisation work. Both become unnecessary once the index leaves the lambda bundle.
- No per-audience tagging and no per-audience index scoping. The spike (E2) resolved the corpus-scope question (design Q3) toward a single broad public index over the nym product code (plus docs and `nym-vpn-client`) with corpus hygiene, not narrowing and not tagging: operators are public-only and need the breadth, and it does not degrade developers or reviewers once non-product noise is excluded. See `shared-index-spike/EXPERIMENTS.md`.

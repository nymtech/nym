# Add Shared Retrieval Index

## Why

The docs MCP server at `nym.com/docs/api/mcp` works, but its index is baked into the docs deploy artefact. Two JSON files (`docs-index.json`, `code-index.json`) are generated during the Next.js build, traced into the `/api/mcp` lambda, and parsed at every cold start. At current chunk counts that is roughly 120 MB of the serverless bundle before any code runs (MCP-SERVER.md's measured file sizes scaled to today's chunk counts; the doc's own 114 MB figure is stale), and the number grows with every page and every widened code root.

The current setup also blocks three things we now want:

- **Indexing other repos.** `nym-vpn-client` (40 Rust crates, four platform apps) is not indexed at all, and the index build only sees the repo it runs in.
- **A private tier.** The `nym-vpn-api` backend source lives in the private `websites` repo (`www/vpn-api`). We want it searchable by our own tools, but nothing about it can appear in a public index. The current pipeline publishes every indexed page four ways (rendered page, raw `.md`, `llms-full.txt`, `docs-index.json`), so there is no safe place for private content in it.
- **Provider independence.** Voyage AI trains on API content by default; opt-out is a dashboard toggle and is prospective only. Sending private backend source to Voyage is not acceptable, and continuing to depend on that toggle for public content is worth questioning too.

## What Changes

- Extract the retrieval pipeline (page collector, chunkers, embedder, upsert) from `documentation/docs/lib/retrieval/` into a shared indexer package that any repo's CI can run. The seams for this (injectable `siteUrl`, `source`, `expand`, `values`) already exist; the remaining nym-specific values are module constants.
- Replace the bundled JSON files and the build-time embed caches with a **self-hosted Postgres + pgvector** instance on Nym infrastructure. Chunks are upserted by id with a content hash, so unchanged chunks are never re-embedded. Retrieval stays a brute-force cosine scan; at 10-30k chunks no ANN index is needed.
- Split public and private at the database level. The public MCP server connects with a Postgres role that can only read public rows (enforced by a view or row-level security). A second, access-protected MCP deployment connects with a wider role. A filter bug in application code cannot leak rows the connection cannot read.
- Move index production to a push model: each repo (`nym`, `nym-vpn-client`, `websites`) runs the shared indexer in its own CI and upserts its shard with `{repo, source, visibility}` metadata.
- Decide the embedding provider. Two candidate paths are laid out in `design.md` (Open Questions); the decision is deliberately left open in this proposal.
- Fix the stale claims and two small guard gaps found in `documentation/MCP-SERVER.md` and the MCP route during research (see Impact).
- Later stage: a NymVPN docs site (reader-facing) built on the same Nextra machinery as the current docs, fed by the crate READMEs, the vpn-api openspec specs, and Redoc over the existing `openapi.json` files.

## Capabilities

### New Capabilities

- `docs-shared-index`: multi-repo retrieval index in self-hosted Postgres + pgvector, with public/private partitioning enforced by database roles, produced by a shared indexer package run from each repo's CI.

### Modified Capabilities

<!-- none: the public MCP endpoint URL and tool surface are unchanged; this change replaces the storage and build pipeline behind them. The new capability's delta spec is in specs/docs-shared-index/. -->

## Impact

- `documentation/docs/lib/retrieval/`, `documentation/docs/lib/mcp/`, `documentation/scripts/next-scripts/generate-index.mjs`, `generate-code-index.mjs`: refactored into / consuming the shared indexer package. The `/api/mcp` route stops reading bundled files and queries Postgres.
- `documentation/docs/next.config.js`: `outputFileTracingIncludes` for the index files removed; the docs deploy artefact shrinks by ~120 MB.
- `.github/workflows/cd-docs.yml` and new workflows in `nym-vpn-client` and `websites`: run the indexer, need database credentials as secrets. `VOYAGE_API_KEY` is removed or scoped down depending on the embedding decision.
- New infrastructure on Nym servers: Postgres + pgvector, an embedding inference service (required for any self-hosted model path, at minimum for the private shard), and the private MCP deployment.
- Runtime behaviour of the public MCP server is unchanged for clients: same URL, same tools, same result shapes. Cold starts get cheaper; each query gains one database round trip.
- Documentation fixes riding along: `MCP-SERVER.md` stale `ROOTS` location (line 83), stale sizing figures, the vectorless-index guard that only covers the docs index, and `search_code` being exposed when a vectorless code index parses.

## Non-Goals

- No chat assistant. The `/api/chat` scaffold stays unwired.
- No OAuth or identity-provider build-out. The private MCP (used by staff and external reviewers alike) authenticates with per-person expiring tokens checked by the service itself; OAuth 2.1 is the upgrade path if per-user identity demands it. The docs site can still use platform access protection, since it carries no private index content.
- No ANN index, no quantisation work. Both become unnecessary once the index leaves the lambda bundle.
- No change to what the public index covers beyond adding `nym-vpn-client` public sources.

# Design: Add Shared Retrieval Index

## Context

Current state, verified against the code on 2026-09-08:

- Storage is two gitignored JSON files in `documentation/docs/public/`, traced into the `/api/mcp` lambda via `outputFileTracingIncludes` (`next.config.js:91-93`) and read with `readFileSync` at cold start (`pages/api/mcp.ts:27-30`, `:57-63`). There is no database of any kind. The one Vercel KV mention in the repo (`ai-assistant-mcp-scratch.md:310`) is a deferred rate-limiting idea, not storage.
- Embeddings are Voyage AI: `voyage-3-large` for docs, `voyage-code-3` for code, both 1024-dim. A content-hash cache under `node_modules/.cache/nym-docs/` avoids re-embedding unchanged chunks across builds.
- Retrieval is a linear cosine scan over all chunks (`lib/retrieval/retrieval.ts:44`), topK default 6, no similarity floor. Roughly 1.6k docs chunks and 7.3k code chunks today.
- The code index covers 20 roots listed in `documentation/indexed-sources.mjs`, all nym-repo paths. Nothing from `nym-vpn-client` is indexed.
- The private boundary is clean: `nym-vpn-client` consumes only the `public/v1` surface of the vpn-api (`nym-vpn-core/crates/nym-vpn-api-client/src/routes.rs`). Everything sensitive (private OpenAPI spec, webhook handlers, payment internals, openspec design docs) lives in the `websites` repo under `www/vpn-api` and `packages/vpn-api-common`.
- The docs site has no access control and publishes every page through four output paths (rendered page, raw `.md` mirror, `llms-full.txt`, `docs-index.json`). Private content cannot be safely mixed into it.
- The retrieval code is already seamed for reuse: `chunkPages` takes `siteUrl`, `collectPages` takes `source`/`expand`/`values`, `stripMdx` is fully injectable. The hardcoded remainder is a handful of module constants (`SITE_URL` in `pages-source.mjs:23`, `GITHUB` in `code-chunker.mjs:13`, cache directory name, `ROOTS`).

## Goals / Non-Goals

**Goals:**

- One index, three producer repos (`nym`, `nym-vpn-client`, `websites`), each owning its shard's freshness from its own CI.
- Private content searchable through an access-protected MCP endpoint, with the public/private boundary enforced below application code.
- Docs deploys decoupled from index size.
- An embedding setup whose privacy properties we control, at minimum for private content.

**Non-Goals:**

- Chat assistant, ANN indexing, quantisation, hand-rolled auth (see proposal Non-Goals).
- Merging or ranking results across public and private shards in one response.

## Decisions

1. **Storage is self-hosted Postgres + pgvector on Nym infrastructure.** The corpus (about 9k chunks today, projected 10-30k with the new repos) needs no vector database features for performance; a sequential cosine scan in SQL is milliseconds at this scale. Postgres is chosen for decoupling, multi-writer CI access, and role-based access control. Self-hosting follows from a transitive privacy argument: chunks contain the full source text, so a managed database host would see everything the embedding provider is being cut out of seeing. One table, roughly: `chunks(id, repo, source, visibility, title, heading, url, text, lang, tokens_est, content_hash, model, dim, embedding vector)`. The `embedding` column stays untyped (no fixed dimension) so shards can differ in model and dimension; a typed column and index only matter if an ANN index is ever added.

2. **Public/private is enforced by database roles, not application filters.** The public MCP server's connection role has SELECT only on a view (or RLS policy) restricted to `visibility = 'public'`. The private deployment's role reads everything. Fail-closed: application bugs cannot widen what the connection can read. The two deployments also keep separate secrets, so leaking the public server's credentials discloses nothing non-public.

3. **Index production moves to a push model.** Each repo runs the shared indexer package in its own CI and upserts its shard, keyed by chunk id with a content hash computed over the embed text plus the model and dimension, so unchanged chunks skip embedding and a model change re-embeds everything (the same property the current embed cache keys have). This replaces both the build-time index generation and the embed cache files. The alternative (one pipeline pulling all repos) would need a read token for the private `websites` repo inside the nym docs CI and couples three repos to one deploy cadence.

4. **The indexer becomes a package, not a rewrite.** Extract `pages-source.mjs`, `chunker.mjs`, `code-chunker.mjs`, `mdx.mjs`, `ts-data.mjs`, `embed.mjs` plus a new upsert module. Parametrise the four hardcoded constants (site URL, GitHub deep-link base, roots list, cache name). The nym-specific projections (`projections.mjs`) stay in the nym repo and are passed in through the existing `expand` seam.

5. **Retrieval stays brute force; each shard is queried in its own embedding space.** Indexes may use different models (public vs private). Queries are embedded per shard with that shard's recorded model. If results are ever combined across shards, they are fused by rank, never by raw cosine score, since score distributions differ between models.

6. **Runtime query embedding becomes a service concern.** Today each query is embedded by calling Voyage from the lambda. Any self-hosted model path requires an always-on embedding endpoint (text-embeddings-inference or ONNX runtime on CPU; the candidate models answer single queries in tens of milliseconds). This service sits on Nym infra next to Postgres. The private MCP deployment can be co-located there too.

7. **The public MCP server stays on Vercel.** It remains the `/api/mcp` lambda deployed with the docs site, same URL and deploy story as today; only its data source changes, from bundled files to Postgres on Nym infra over TLS with the public read role. Query embedding from the lambda is an HTTPS call either way: to Voyage (Option B) or to the self-hosted embedding endpoint (Option A). Vercel functions have no stable egress IPs on standard plans, so the database endpoint is internet-reachable: a pgbouncer pooler with a TLS-only listener and scram or client-certificate auth, which also absorbs lambda connection fan-out. If an internet-facing pooler is unacceptable, the fallback is a thin HTTPS query service in front of Postgres.

8. **The private MCP is a superset server with per-person expiring tokens.** It runs on Nym infra next to Postgres and the embedder, serves public and private rows through the same tool names, and is the single endpoint internal people and external reviewers configure; nobody mounts both servers. Access is a per-person opaque token, stored hashed in Postgres with an expiry, checked by middleware; external reviewers get tokens scoped to their engagement window, and revocation is a row update, no redeploy. Vercel Deployment Protection was rejected for this endpoint on the transitive privacy argument: it would route private chunk text through Vercel functions. OAuth 2.1 (which MCP clients support) is the upgrade path if per-user identity or token sprawl demands it, not the starting point. If content ever appears that some reviewers must not see, that is a third visibility tier and a third database role, added then, not now.

9. **The NymVPN docs site is a later stage of this change, not a separate one.** It reuses the Nextra 2 setup, which is already a self-contained pnpm root. Initial content splits by repo and visibility: the crate and platform READMEs in `nym-vpn-client` are public and seed a public surface; the vpn-api openspec specs and `openapi.json` are not in `nym-vpn-client` at all but in the private `websites` repo (`www/vpn-api`, `packages/vpn-api-common`), so any API reference derived from them is private-visibility content and belongs on the access-protected deployment, not the public site (per the `docs-site-surface` single-visibility rule). Reader access via deployment protection, where the deployment is restricted. The site's private variant, if any, follows the same fail-closed rule: a separate deployment, never filtered pages in a public one.

## Open Questions

**Q1: Embedding provider.** Two candidate paths; the choice is the main thing this proposal asks reviewers to weigh in on.

| | Option A: full self-host, one model | Option B: hybrid |
|---|---|---|
| Models | Qwen3-Embedding-0.6B (Apache 2.0) for prose and code | Voyage (`voyage-3-large` / `voyage-code-3`) for public shards; self-hosted Qwen3-0.6B for the private shard |
| Third parties | None | Voyage/MongoDB for public content; nothing private leaves our infra |
| Serving | One embedding service on Nym infra, CPU-viable via ONNX/TEI | Same service, plus the Voyage API dependency and key management |
| Quality | Below the top API models on prose; small gap at this corpus size; must be benchmarked on real queries before cutover | Public shard keeps today's retrieval quality exactly; private shard quality same as Option A |
| Migration | Re-embed everything once (~10-30k chunks, one batch job) | Public shards unchanged; only new private/vpn-client shards embedded |
| Ongoing risk | Retrieval quality regression if the benchmark is skipped | Depends on Voyage's train-by-default opt-out toggle staying set and honoured; two models and two quality profiles to maintain |
| Failure mode | Embedding service down means all search down | Voyage outage takes public search down; private unaffected |

Either way, two actions are independent of the choice: verify the Voyage organisation opt-out toggle is set today, and build a small retrieval benchmark (real queries against the current index as baseline) so any model change is measured, not guessed. If Option A's code retrieval disappoints, `bge-code-v1` (Apache 2.0, above `voyage-code-3` on CoIR) is the quality upgrade at the cost of preferring a GPU.

Load-bearing external claims for reviewers to check: Voyage's train-by-default is in their ToS (voyageai.com/tos, updated 2026-05-27, opt-out prospective only); the `bge-code-v1` CoIR figures are from its model card (huggingface.co/BAAI/bge-code-v1).

**Q2: Rate limiting.** The public endpoint is unauthenticated and every query costs an embedding call. This was already flagged in the code before this change. Postgres does not answer it; a limiter in front of both MCP deployments still needs choosing.

## Risks / Trade-offs

- **New operational surface.** Postgres, an embedding service, and a private MCP deployment are always-on services we now run. Mitigation: all three are small, co-located, and stateless apart from Postgres; the index is fully reproducible from the repos, so backup requirements are modest.
- **Retrieval quality regression on model change (Option A).** Mitigation: the benchmark in Q1 is a gate, not an afterthought; cutover only after it passes.
- **Secrets spread.** Three repos' CI gain database write credentials. Mitigation: per-repo roles restricted to that repo's shard rows via RLS `WITH CHECK` policies.
- **CI ordering.** The `websites` repo must never write `visibility = 'public'` rows. Mitigation: an RLS `WITH CHECK` policy per writer role (or a trigger inspecting the session user) rejects rows outside that repo's permitted visibility; indexer configuration is not trusted. A plain table CHECK constraint cannot do this, since it applies to every writer equally.
- **The vectorless-index failure mode moves.** Today a build without an API key ships a vectorless index; the route half-guards it. In the new design, a failed embed leaves stale-but-valid rows instead, which degrades silently to outdated results. Mitigation: index freshness is exposed (last-successful-run timestamp per shard) and checked by a scheduled verification job independent of any producer's CI, because a producer failure never triggers a docs deploy, so deploy-time checks alone would miss it.

## Migration Plan

1. Doc fixes and small guards land first (independent of everything else).
2. Indexer package extracted; nym CI dual-writes: files as today, plus Postgres.
3. Benchmark built; embedding decision (Q1) made against it.
4. MCP route reads Postgres behind a flag; verified with `check-mcp-server.sh`; flag flipped; file generation and tracing removed.
5. `nym-vpn-client` and `websites` CI wired up; private MCP deployment stood up.
6. NymVPN docs site stage begins.

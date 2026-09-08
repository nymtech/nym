# Tasks: Add Shared Retrieval Index

## 1. Ride-along fixes

- [ ] 1.1 Fix `MCP-SERVER.md:83` `ROOTS` location (points at `generate-code-index.mjs`; lives in `indexed-sources.mjs`) and the matching comment in `check-mcp-server.sh:216`
- [ ] 1.2 Refresh stale sizing figures in `MCP-SERVER.md` (chunk counts, ~122 MB bundle estimate)
- [ ] 1.3 Extend the vectorless-index cold-start guard in `pages/api/mcp.ts` to the code index, and only expose `search_code` when the code index is vectored
- [ ] 1.4 Correct `ai-assistant-mcp-scratch.md` claims about `/api/chat` (route does not exist, is not traced)
- [ ] 1.5 Verify the Voyage organisation opt-out toggle is set (dashboard action, record the date)

## 2. Proposal review

- [ ] 2.1 Share proposal with colleagues; collect positions on Open Question Q1 (embedding path)
- [ ] 2.2 Pick the Q1 direction and resolve Q2 in design.md; the benchmark (4.3) gates the final Q1 cutover

## 3. Shared indexer package

- [ ] 3.1 Extract `pages-source` / `chunker` / `code-chunker` / `mdx` / `ts-data` / `embed` into a package; parametrise site URL, deep-link base, roots, cache name
- [ ] 3.2 Add the Postgres upsert module (content-hash skip keyed on embed text + model + dim, per-shard delete of stale rows, last-run timestamp per shard; upsert and delete commit in one transaction per run)
- [ ] 3.3 Keep nym projections in-repo, passed through the `expand` seam
- [ ] 3.4 Port the existing retrieval tests; add an upsert round-trip test
- [ ] 3.5 Extend the chunker to every language present in producer roots: add `langOf` entries and boundary rules for Kotlin and Swift (the `nym-vpn-client` mobile apps); emit a per-run report of source-like extensions that produced no chunks, so an unhandled language cannot pass unnoticed

## 4. Infrastructure

- [ ] 4.1 Provision Postgres + pgvector on Nym infra; schema, per-repo writer roles, public read view/RLS, private read role
- [ ] 4.2 Stand up the embedding service (scope depends on Q1: private shard at minimum)
- [ ] 4.3 Build the retrieval benchmark against the current index as baseline

## 5. Cutover (nym repo)

- [ ] 5.1 nym CI dual-writes: bundled files plus Postgres
- [ ] 5.2 `/api/mcp` reads Postgres behind a flag; run `check-mcp-server.sh` against a preview
- [ ] 5.3 Flip the flag; remove index generation from the build and the `outputFileTracingIncludes` entries; rewrite `MCP-SERVER.md` for the new architecture

## 6. New shards

- [ ] 6.1 `nym-vpn-client` CI: indexer over crates and docs, public shard, covering Rust, TypeScript, Kotlin, and Swift (verify the Android and Apple apps produce chunks, not zero)
- [ ] 6.2 `websites` CI: indexer over `www/vpn-api` + `packages/vpn-api-common`, private shard (role-enforced visibility)
- [ ] 6.3 Private MCP deployment on Nym infra: token table (hashed, expiry), auth middleware, token issue/revoke runbook covering external reviewer engagements; verify public endpoint cannot cite private chunks and an expired token is refused

## 7. NymVPN docs site (later stage)

- [ ] 7.1 Nextra skeleton in `nym-vpn-client` from the existing self-contained `docs/` pattern
- [ ] 7.2 Seed the public surface from `nym-vpn-client` crate and platform READMEs; seed the private surface's API reference from the `websites` repo (vpn-api openspec specs, Redoc over its `openapi.json`) as private-visibility content, never on the public deployment
- [ ] 7.3 Reader access via deployment protection (restricted deployments); wire the site's pages into its public shard

## Out of scope

- Chat assistant, ANN indexing, quantisation, custom auth (see proposal Non-Goals)

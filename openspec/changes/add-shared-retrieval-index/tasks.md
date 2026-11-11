# Tasks: Add Shared Retrieval Index

## 1. Ride-along fixes

- [x] 1.1 Fix `MCP-SERVER.md:83` `ROOTS` location (points at `generate-code-index.mjs`; lives in `indexed-sources.mjs`) and the matching comment in `check-mcp-server.sh:216`
- [x] 1.2 Refresh stale sizing figures in `MCP-SERVER.md` (chunk counts, ~120 MB bundle estimate)
- [x] 1.3 Extend the vectorless-index cold-start guard in `pages/api/mcp.ts` to the code index, and only expose `search_code` when the code index is vectored
- [x] 1.4 Correct `ai-assistant-mcp-scratch.md` claims about `/api/chat` (route does not exist, is not traced). The file is an untracked local scratch; corrected in place 2026-09-23
- [ ] 1.5 Verify the Voyage organisation opt-out toggle is set (dashboard action, record the date)

## 2. Proposal review

- [ ] 2.1 Share proposal with colleagues; collect positions on Open Question Q1 (embedding path)
- [x] 2.2 Q1 direction decided (Option B: Voyage public, self-hosted private); Q2 still open in design.md; the benchmark (4.3) gates the private-model choice and cutover
- [x] 2.3 Spike (Q3): corpus scope by audience. Sample plausible queries per audience (SDK/app developers, node operators, external reviewers) against the current index; measure which `ROOTS` entries are actually cited and at what embedding cost; propose a per-repo scoped roots list. Offline-runnable against the current index; gates the roots list in 3.1 and the shards in 6.1

## 3. Shared indexer package

- [x] 3.1 Extract the package modules in the decision 4 table (`code-chunker`, `items`, `chunker`, the generic half of `mdx`, `embed`, `types`, the code walk) into `@nymproject/retrieval-indexer` as TypeScript; parametrise site URL, deep-link base, roots, cache name. nym consumers, including the Next-compiled `pages/api/mcp.ts` and `lib/archetypes/inline.mts`, import the package; the vectorless docs and code indexes stay byte-identical (timestamp aside) across the move. Done in nym: the one-version pin across producers is task 3.6. Version the package and pin one version across all producers (the public CIs and the private-infra pipeline), so both pipelines chunk a given source identically and a shared id means the same text
- [x] 3.2 Add the Postgres upsert module (content-hash skip keyed on embed text + model + dim, per-shard delete of stale rows, last-run timestamp per shard; upsert and delete commit in one transaction per run). Done: `store.ts` in the package (`indexShard`, `indexShards`, `SCHEMA`), verified by `src/store.test.ts` on PGlite with pgvector (10 tests, 2026-11-10); the first run against a real store is task 5.1
- [x] 3.3 Keep nym projections in-repo, passed through the `expand` seam
- [x] 3.4 Port the existing retrieval tests; add an upsert round-trip test
- [ ] 3.5 Code chunker in the package uses tree-sitter (design decision 10), covering every language present in producer roots through its grammars; the docs chunker stays heading-based. (The spike already proved coverage and the gate with a line-regex chunker for Rust, TS/JS, Kotlin, Swift, Go, and Python; the package generalises that to tree-sitter, which also reaches C/C++ and multiline signatures the line regex cannot.)
  - [ ] 3.5.1 Extend `items()` (already tree-sitter, Rust only) with grammars for the other languages present (TS/JS, Kotlin, Swift, Go, Python), and build the code chunker on its item boundaries and names; port the spike's chunker fixture tests to the tree-sitter output. Each grammar package must ship a prebuilt `.wasm` that the pinned `web-tree-sitter` can load (ABI match), as `tree-sitter-rust` does. Keep the Rust output shape the codex snippet extractor depends on. Done for Rust, TS/JS, Go, and Python (the boundary diff over both repos is in shared-index-spike/CHUNKER.md); Kotlin and Swift keep the line rule until their grammars have a `.wasm` (task 6.1)
  - [ ] 3.5.2 Verify the Android and Apple apps (Kotlin, Swift) produce chunks, not zero, and a search returns app hits
  - [x] 3.5.3 Freeze the accept-drop set in one place (C-family, Objective-C, proto: all generated bindings, a cgo shim, and an FFI example in these repos)
  - [x] 3.5.4 Make the drop report a fail-closed build gate: a source-like extension neither chunked nor on the accept-drop set fails CI, so a new language cannot silently index to zero
  - [x] 3.5.5 Demote test-path chunks at rank time (a score penalty, not exclusion), so a test module cannot outrank the production code it exercises. Evidence: 0a query 1, where a speedtest tool's `mod tests` outranked the canonical `FragmentPreparer` send path (shared-index-spike/EXPERIMENTS.md 0a). Built: the package flags test chunks (`test: true`; a test directory, a test file name, or a Rust `mod tests` block) and search subtracts `TEST_SCORE_PENALTY`. Calibrated on the 0a query: 0.05 sits on a plateau from 0.05 to 0.12, so it stays at 0.05

- [ ] 3.6 Publish the package: move the `shared-index` repository to the nymtech org (or make it public) so nym's CI and Vercel can fetch the pinned git dependency; later choose a registry (npm under `@nymproject` or GitHub Packages) to replace the pin. The `tsc` emit is in place (`prepare`). Gates 6.1 and 6.2, whose producers are other repos

## 4. Infrastructure

- [ ] 4.1 Provision the two stores: a public Postgres + pgvector (may be managed, since it holds only public content) and a self-hosted private Postgres + pgvector on Nym infra. Per-store schema; a read-only credential on the public store for the public MCP; the private store reachable only from the private infrastructure
- [ ] 4.2 Stand up the embedding service (scope depends on Q1: private shard at minimum)
- [ ] 4.3 Build the retrieval benchmark against the current index as baseline
- [ ] 4.4 Make the indexer embed concurrently: replace the one-batch-at-a-time loop in `embedChunks` with a bounded in-flight pool, so a self-hosted re-index is not serialised on request round-trips (the spike measured ~7 chunks/s serialised on CPU; this is the throughput lever for a 20k+ chunk re-index against a self-hosted model). See design Q1; verify against the embedding service (4.2). Built: `embedChunks` takes `concurrency` (default 1, so the Voyage request pattern is unchanged); left: pick the self-hosted value against 4.2

## 5. Cutover (nym repo)

- [ ] 5.1 nym CI dual-writes: bundled files plus the public store
- [ ] 5.2 `/api/mcp` reads the public store behind a flag; run `check-mcp-server.sh` against a preview
- [ ] 5.3 Flip the flag; remove index generation from the build and the `outputFileTracingIncludes` entries; rewrite `MCP-SERVER.md` for the new architecture

## 6. New shards

- [ ] 6.1 `nym-vpn-client` CI: indexer over crates and docs, public shard, covering Rust, TypeScript, Kotlin, and Swift (verify the Android and Apple apps produce chunks, not zero)
- [ ] 6.2 Private-infrastructure indexer: run the shared indexer over the private `websites` source (`www/vpn-api` + `packages/vpn-api-common`) and over the public repos, embed all with the private model, and upsert the private store. Private repo access and the private store's write credential stay on this infrastructure; nothing here writes the public store
- [ ] 6.3 Private MCP deployment on Nym infra, reading the private store: token table (hashed, expiry), auth middleware, token issue/revoke runbook covering external reviewer engagements; verify the public store contains no private rows and the public endpoint cannot cite private chunks, and that an expired token is refused

## 7. NymVPN docs site (later stage)

- [ ] 7.1 Nextra skeleton in `nym-vpn-client` from the existing self-contained `docs/` pattern
- [ ] 7.2 Seed the public surface from `nym-vpn-client` crate and platform READMEs; seed the private surface's API reference from the `websites` repo (vpn-api openspec specs, Redoc over its `openapi.json`) as private-visibility content, never on the public deployment
- [ ] 7.3 Reader access via deployment protection (restricted deployments); wire the site's pages into its public shard

## Out of scope

- Chat assistant, ANN indexing, quantisation, custom auth (see proposal Non-Goals)

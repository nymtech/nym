# Tasks: Add Shared Retrieval Index

## 1. Ride-along fixes

- [x] 1.1 Fix `MCP-SERVER.md:83` `ROOTS` location (points at `generate-code-index.mjs`; lives in `indexed-sources.mjs`) and the matching comment in `check-mcp-server.sh:216`
- [x] 1.2 Refresh stale sizing figures in `MCP-SERVER.md` (chunk counts, ~120 MB bundle estimate)
- [x] 1.3 Extend the vectorless-index cold-start guard in `pages/api/mcp.ts` to the code index, and only expose `search_code` when the code index is vectored
- [x] 1.4 Correct `ai-assistant-mcp-scratch.md` claims about `/api/chat` (route does not exist, is not traced). The file is an untracked local scratch; corrected in place 2026-09-23
- [ ] 1.5 Verify the Voyage organisation opt-out toggle is set (dashboard action, record the date)

## 2. Proposal review

- [ ] 2.1 Share proposal with colleagues; collect positions on Open Question Q1 (embedding path)
- [x] 2.2 Q1 decided (Voyage for the public store); the benchmark (4.3) measures any model change
- [x] 2.3 Spike (Q3): corpus scope by audience. Sample plausible queries per audience (SDK/app developers, node operators, external reviewers) against the current index; measure which `ROOTS` entries are actually cited and at what embedding cost; propose a per-repo scoped roots list. Offline-runnable against the current index; gates the roots list in 3.1 and the shards in 6.1

## 3. Shared indexer package

- [x] 3.1 Extract the package modules in the decision 4 table (`code-chunker`, `items`, `chunker`, the generic half of `mdx`, `embed`, `types`, the code walk) into `@nymproject/retrieval-indexer` as TypeScript; parametrise site URL, deep-link base, roots, cache name. nym consumers, including the Next-compiled `pages/api/mcp.ts` and `lib/archetypes/inline.mts`, import the package; the vectorless docs and code indexes stay byte-identical (timestamp aside) across the move. Done in nym: the one-version pin across producers is task 3.6. Version the package and pin one version across all producers, so they chunk a given source identically
- [x] 3.2 Add the Postgres upsert module (content-hash skip keyed on embed text + model + dim, per-shard delete of stale rows, last-run timestamp per shard; upsert and delete commit in one transaction per run). Done: `store.ts` in the package (`indexShard`, `indexShards`, `SCHEMA`), verified by `src/store.test.ts` on PGlite with pgvector (10 tests, 2026-11-10); the first run against a real store is task 5.1
- [x] 3.3 Keep nym projections in-repo, passed through the `expand` seam
- [x] 3.4 Port the existing retrieval tests; add an upsert round-trip test
- [ ] 3.5 Code chunker in the package uses tree-sitter (design decision 10), covering every language present in producer roots through its grammars; the docs chunker stays heading-based. (The spike already proved coverage and the gate with a line-regex chunker for Rust, TS/JS, Kotlin, Swift, Go, and Python; the package generalises that to tree-sitter, which also reaches C/C++ and multiline signatures the line regex cannot.)
  - [ ] 3.5.1 Extend `items()` (already tree-sitter, Rust only) with grammars for the other languages present (TS/JS, Kotlin, Swift, Go, Python), and build the code chunker on its item boundaries and names; port the spike's chunker fixture tests to the tree-sitter output. Each grammar package must ship a prebuilt `.wasm` that the pinned `web-tree-sitter` can load (ABI match), as `tree-sitter-rust` does. Keep the Rust output shape the codex snippet extractor depends on. Done for Rust, TS/JS, Go, and Python; Kotlin and Swift keep the line rule until their grammars have a `.wasm` (task 6.1)
  - [ ] 3.5.2 Verify the Android and Apple apps (Kotlin, Swift) produce chunks, not zero, and a search returns app hits
  - [x] 3.5.3 Freeze the accept-drop set in one place (C-family, Objective-C, proto: all generated bindings, a cgo shim, and an FFI example in these repos)
  - [x] 3.5.4 Make the drop report a fail-closed build gate: a source-like extension neither chunked nor on the accept-drop set fails CI, so a new language cannot silently index to zero
  - [x] 3.5.5 Demote test-path chunks at rank time (a score penalty, not exclusion), so a test module cannot outrank the production code it exercises. Evidence: 0a query 1, where a speedtest tool's `mod tests` outranked the canonical `FragmentPreparer` send path. Built: the package flags test chunks (`test: true`; a test directory, a test file name, or a Rust `mod tests` block) and search subtracts `TEST_SCORE_PENALTY`. Calibrated on the 0a query: 0.05 sits on a plateau from 0.05 to 0.12, so it stays at 0.05

- [x] 3.6 Publish the package: move the `shared-index` repository to the nymtech org (or make it public). The package has its own public repository, `nymtech/retrieval-indexer` (package at the root, no history, tags `v0.1.0` and `v0.1.1`, public as of 2026-11-16). nym pins `github:nymtech/retrieval-indexer#v0.4.0` (no `&path:`), and the lockfile resolves it to commit `c8240572`. `resolveEmbedKey`, which left the package, is in nym's `documentation/docs/lib/retrieval/embed-key.mts`, and both index build scripts (`generate-index.mjs`, `generate-code-index.mjs`) import it. The `allowBuilds` entry in `documentation/docs/pnpm-workspace.yaml` lets the package's `prepare` script build `dist/` with `tsc` at install. Left for later: choose a registry (npm under `@nymproject` or GitHub Packages) to replace the git pin. Unblocks 6.1, whose producer is another repo
- [ ] 3.7 Add a plain Markdown walk to the package for producers with no Nextra tree (the README and `.md` files of `nym-vpn-client`). Exclude `.nym-codex` from it and from the root walk (`add-codex-snippets` task 2.8). Gates the docs shard of 6.1; its code shard need only the code walk, which the package has

## 4. Infrastructure

- [ ] 4.1 Provision the public Postgres + pgvector store (may be managed, since it holds only public content), with a read-only credential for the public MCP
- [x] 4.2 Not needed: queries are embedded with Voyage (design decision 6)
- [ ] 4.3 Build the retrieval benchmark against the current index as baseline. Status 2026-11-13: for docs, `lib/snippets/benchmark.mts` (from `add-codex-snippets`) is built, with a first baseline in codex task 0.2 (hit@6 on 24 queries, not recall@6). Its no-cap variant was first run on 2026-11-18 (24 queries, the tree at `bb6a1be05f`): A, capped chunks, hit@1 75%, hit@6 96%, MRR@20 0.853; B, one vector for each snippet (1,972 units, 43 of them long snippets as one vector), hit@1 79%, hit@6 96%, MRR@20 0.868. Two queries differ, one in each direction, so on this set B is no worse than A and the cap shows no gain. On 61 queries (2026-11-18, the set extended for codex 6.3): A hit@1 80%, hit@6 98%, MRR@20 0.883; B hit@1 82%, hit@6 98%, MRR@20 0.889
- [x] 4.4 Make the indexer embed concurrently: replace the one-batch-at-a-time loop in `embedChunks` with a bounded in-flight pool. Built: `embedChunks` takes `concurrency` (default 1, so the Voyage request pattern is unchanged)

## 5. Cutover (nym repo)

- [ ] 5.0 Gate before 5.1: prove embedding parity with the live indexer. Re-embed the live corpus with the package (same models, voyage-3-large for docs and voyage-code-3 for code; confirm `embedText` is unchanged), then diff top-k against the current `/api/mcp` for the `check-mcp-server.sh` coverage probes. No probe's root may drop out of the top-k. Both sides index the same content commit, so a content change cannot show as a package difference.
- [x] 5.1 nym CI dual-writes: bundled files plus the public store
- [x] 5.2 `/api/mcp` reads the public store behind a flag; run `check-mcp-server.sh` against a preview
- [x] 5.3 Flip the flag; remove index generation from the build and the `outputFileTracingIncludes` entries; rewrite `MCP-SERVER.md` for the new architecture
- [ ] 5.4 Before 5.1, write the nym retrieval modules that stay as TypeScript (`.mts`, run by Node): `pages-source.mjs`, `projections.mjs`, `ts-data.mjs`, and the `generate-*.mjs` producer scripts, under `tsconfig.tools.json` (design decision 4; `add-codex-snippets` D19 and task 6.0)
- [ ] 5.5 After 5.3: move the nym docs shard to snippet records (design decision 12). Before the move, decide the chunk id scheme: snippet ids have a `nym/` prefix and the docs ids today have none, and a part of a split oversized snippet needs a suffix. Also decide the gate metric: codex 6.3 names recall@6, and `benchmark.mts` measures hit@6. The move itself and its gate are `add-codex-snippets` tasks 6.1 and 6.3

## 6. New shards

- [x] 6.1 `nym-vpn-client` CI: indexer over crates and docs, public shard, covering Rust, TypeScript, Kotlin, and Swift (verify the Android and Apple apps produce chunks, not zero)
- [x] 6.2 Out of scope: private content is in a separate store (design decision 8)
- [x] 6.3 Out of scope: private content is in a separate store (design decision 8)

## 7. NymVPN docs site (later stage)

- [ ] 7.1 Nextra skeleton in `nym-vpn-client` from the existing self-contained `docs/` pattern
- [ ] 7.2 Seed the site from the `nym-vpn-client` crate and platform READMEs
- [ ] 7.3 Reader access via deployment protection (restricted deployments); wire the site's pages into its public shard

## Out of scope

- Chat assistant, ANN indexing, quantisation, custom auth (see proposal Non-Goals)

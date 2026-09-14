# Add Code-Index Coverage Gate

## Why

`search_code` can only cite a path under some entry in `ROOTS` (`documentation/indexed-sources.mjs`). A root missing from the index fails silently: no error, just answers that quietly do not mention it. The only guard is the "Index coverage" group in `documentation/scripts/check-mcp-server.sh`, but its probes are a hand-maintained list enumerated separately from `ROOTS`, with nothing keeping the two in sync. So a root can be added to `ROOTS` and indexed while no probe ever checks it, and the safety net against "a root is silently uncited" has a hole the same shape: a root can be silently *unchecked*.

This has already drifted. Mapping the current probes onto `ROOTS`, **8 of the 20 roots have no probe at all** (`sdk/typescript/packages`, `sdk/typescript/examples`, `sdk/ffi`, `wasm/smolmix`, `wasm/client`, `wasm/zknym-lib`, `common/smol-core`, `common/client-libs`). The header of `indexed-sources.mjs` claims the check "asserts that every root here is actually citable"; that is false today.

The shared-index change (`add-shared-retrieval-index`) makes it worse in a second way: it adds `nym-vpn-client` as a producer, but `ROOTS` and the coverage check are single-repo. Without a repo-aware roots structure, the check silently omits an entire repo's citability.

## What Changes

- Move the coverage probes out of the shell script into a data structure keyed by root — `{ root: { label, query } }` — in a sibling file that is NOT traced into the `/api/mcp` lambda, so probe queries never ship to production.
- Add a static completeness check (a unit test) asserting every entry in `ROOTS` is covered by at least one probe. A root with none fails the test, so adding a root forces adding its probe.
- Keep the live citability probe in `check-mcp-server.sh`, now reading the shared probe data: it still *skips* a root absent from the checkout (the existing branch-predates-a-crate behaviour) and *fails* only when a present root's probe returns no hit under it.
- Make roots and probes repo-aware, so a second producer repo's roots are covered rather than silently omitted. The per-repo roots shape is the one produced by the `add-shared-retrieval-index` audience-scope spike (Q3 / task 2.3).

## Capabilities

### New Capabilities

- `code-index-coverage`: a drift-proof, repo-aware guarantee that every configured source root is verified citable through `search_code`, with the set of verified roots derived from the canonical roots list rather than a parallel hand-maintained one.

## Impact

- New `documentation/scripts/index-coverage-probes.mjs` (the probe map, kept out of the lambda trace).
- New vitest test asserting static completeness against `ROOTS`.
- `documentation/scripts/check-mcp-server.sh`: the Index-coverage group reads probes from the data file instead of inline `covers` calls.
- `documentation/indexed-sources.mjs`: the header can once again promise full coverage, because a test now enforces it (reverting the interim "hand-maintained" caveat).
- Consumes the per-repo roots shape from `add-shared-retrieval-index`.

## Non-Goals

- Auto-generating probe queries. Queries are deliberately human-chosen discriminating symbols; generating them from a path reintroduces the ranking decay the probes were written to avoid (documented in `check-mcp-server.sh`). Completeness is enforced; query authorship stays manual.
- Deciding which roots to index. What belongs in the corpus is the `add-shared-retrieval-index` audience-scope question (Q3); this change verifies whatever roots are configured, it does not choose them.

## Dependencies

- Depends on `add-shared-retrieval-index` task 2.3 (the audience-scope spike) and the per-repo roots structure it feeds (task 3.1). The static-completeness and single-repo drift fix can land first against today's flat `ROOTS`; the repo-aware part follows the shared-index roots shape.

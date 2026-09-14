# Tasks: Add Code-Index Coverage Gate

## 1. Single-repo drift fix (lands now)

- [ ] 1.1 Add `documentation/scripts/index-coverage-probes.mjs`: a `{ root: { label, query } }` map, migrated from the inline `covers` calls in `check-mcp-server.sh`
- [ ] 1.2 Add a vitest test asserting every `ROOTS` entry is covered by at least one probe (a probe covers a root when its path fragment is at or under the root); on failure, name the offending root(s)
- [ ] 1.3 Backfill probes for the 8 currently-uncovered roots: `sdk/typescript/packages`, `sdk/typescript/examples`, `sdk/ffi`, `wasm/smolmix`, `wasm/client`, `wasm/zknym-lib`, `common/smol-core`, `common/client-libs`
- [ ] 1.4 Port the `check-mcp-server.sh` Index-coverage group to read probes from the data file; preserve skip-when-absent and fail-when-present-but-uncited
- [ ] 1.5 Revert the interim "hand-maintained" caveat in `indexed-sources.mjs`; the header can promise full coverage again, now that the test enforces it
- [ ] 1.6 Verify: the completeness test fails on a deliberately un-probed root, and passes once all roots (incl. the backfilled 8) have probes; `check-mcp-server.sh` still green against a live deployment

## 2. Repo-aware (after add-shared-retrieval-index roots shape)

- [ ] 2.1 Extend the probe map and both checks to iterate producer repos × roots, using the per-repo roots structure from `add-shared-retrieval-index` (spike 2.3 / task 3.1)
- [ ] 2.2 Verify a second producer repo (`nym-vpn-client`) whose roots have no probes fails the completeness test rather than being silently omitted

## Out of scope

- Deciding which roots to index (`add-shared-retrieval-index` Q3 spike)
- Auto-generating probe queries (see design Non-Goals)

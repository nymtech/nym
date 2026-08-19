# Tasks: add-nym-swizzle

## 1. Crate scaffolding

- [x] 1.1 Create `sdk/rust/nym-swizzle/` with `Cargo.toml` (workspace-inherited metadata, `publish = true`, deps: `rand`, `rand_distr`, `rand_chacha`, `getrandom`, `futures`; `tokio` (time) under `[target.'cfg(not(target_arch = "wasm32"))']` and `wasmtimer` under `[target.'cfg(target_arch = "wasm32")']`, matching `common/http-api-client`; dev-deps: `tokio` (full, for tests/examples), `plotters`, optionally `statrs`). Hard rule: every non-dev dependency compiles for `wasm32-unknown-unknown`.
- [x] 1.2 Add the crate to the root workspace `members`.
- [x] 1.3 Verify `cargo check -p nym-swizzle --target wasm32-unknown-unknown` passes on the empty scaffold before any feature work, so wasm breakage is attributable per-commit.
- [x] 1.4 Crate-level docs: threat model (V2 timing / V3 start-height linking), what stays app-level (transport/destination splitting, range widening beyond the start edge, dedup), wasm-wrapper distribution story, and the unvalidated-tuning caveat.

## 2. Randomness module (`rng`)

- [x] 2.1 Shared randomness configuration: distribution enum (uniform / poisson-process / normal with std dev) + bounds; all sampling generic over `Rng + CryptoRng`.
- [x] 2.2 Sources: `OsRng` default; `from_seed([u8; 32])` via `ChaCha20Rng` (VRF-derived seeds treated as opaque seed material); `with_rng(R)` injection.
- [x] 2.3 Rejection-resampling with bounded retries and uniform in-bounds fallback (debug-logged).
- [x] 2.4 Poisson-process sampler mirroring `sample_poisson_duration` (`Exp(1/mean)`), with a source comment pointing at `common/nymsphinx/src/utils/mod.rs`.

## 3. `delay` primitive

- [x] 3.1 Constructor with min/max bounds + distribution config; builder-style (`Delay::uniform(min, max)`, `Delay::poisson(mean).max(max)`, `Delay::normal(mean, std_dev).bounds(min, max)`).
- [x] 3.2 `run(future)` — sample, sleep, then poll; result passed through; document and uphold the "not polled before its scheduled time" guarantee.
- [x] 3.3 cfg-gated sleep abstraction (tokio native / wasm timer) isolated in one internal function.

## 4. `range` primitive

- [x] 4.1 Chunk-plan generation: sampled chunk sizes, sampled consecutive overlaps in `[min_overlap, max_overlap]`, full-coverage invariant, no spill past end; clamped fractional overlap defaults.
- [x] 4.2 Start-edge obfuscation: `start_jitter(distribution, floor)` (downward only; default magnitude a clamped percentage of the total range) and `snap_start(spacing | checkpoint list)` (deterministic, RNG-free); when both are enabled, jitter is sampled in whole checkpoint intervals so emitted starts stay on-grid.
- [x] 4.3 Pull API: `Iterator<Item = (u64, u64)>` over the randomly permuted plan.
- [x] 4.4 Push API: `for_each` / `for_each_concurrent(n, closure)` async driver, completing when all chunks executed; optional inter-chunk delay composition using the `delay` primitive.

## 5. Examples (`examples/`)

- [x] 5.1 `delay_broadcast.rs` — delay a simulated `broadcast_tx` by a uniform random duration; note in comments: decorrelate from sync milestones, never broadcast on the sync transport (app-level).
- [x] 5.2 `fetch_blocks_overlapping.rs` — decompose a block range into overlapping shuffled chunks and "fetch" them concurrently; show start jitter + checkpoint snapping for a resumed sync.
- [x] 5.3 `poisson_sampling.rs` — Poisson-process delay sampling; print a handful of samples and their mean vs the configured mean.
- [x] 5.4 `seeded_vrf.rs` — seed from fixed bytes (standing in for a VRF output) twice, print both chunk plans, assert and show they are identical; third different seed shown diverging.
- [x] 5.5 (added post-proposal) Live network example `sdk/rust/nym-swizzle-zcash` — fetches Zcash compact blocks from the public `zec.rocks` lightwalletd over gRPC; compares a direct fetch against overlapping chunking (sequential and concurrent, identical seeded plan), verifies complete coverage of the range, and reports wastage. Kept as a separate crate so `nym-swizzle`'s "every non-dev dependency compiles to wasm" guarantee is not weakened by a gRPC/TLS stack.

## 6. Tests

- [x] 6.1 Delay: wrapped future not polled before schedule (poll-counting future); result passthrough; samples within bounds; independent samples per call.
- [x] 6.2 Distributions: rejection-resampling never returns out-of-bounds; no boundary accumulation (statistical smoke test); pathological config terminates via fallback.
- [x] 6.3 Range invariants (property-style over random configs/seeds): union == range, no end spill, consecutive overlaps within bounds, permutation yields every chunk exactly once; small-range degradation.
- [x] 6.4 Start obfuscation: jitter moves start down only and respects floor; snapping is deterministic, collides within an interval, works with explicit checkpoint lists, and consumes no RNG (identical sample stream with/without snapping); jitter+snapping composition emits only on-grid starts (integer checkpoint multiples below the snapped start).
- [x] 6.5 Determinism: same seed ⇒ identical plan + delay sequence; different seed diverges.
- [x] 6.6 Push driver: completes all chunks, respects concurrency bound, applies inter-chunk delays.
- [x] 6.7 Wasm: `cargo check -p nym-swizzle --target wasm32-unknown-unknown` wired into whatever check pipeline the SDK uses; assert `tokio` is absent from the resolved wasm dependency graph (`cargo tree --target wasm32-unknown-unknown`).

## 7. Profiling harness (development-time)

- [x] 7.1 Harness scaffold (`benches/profiling.rs` or feature-gated `examples/profiling.rs`; `plotters` SVG output into `target/swizzle-profiling/`).
- [x] 7.2 Delay suite: per-distribution histograms (~10⁵ samples) overlaid on theoretical density restricted to bounds; moment checks within tolerance; visual proof of no clamp spike.
- [x] 7.3 Chunking suite: chunk-size / overlap / start-jitter histograms across many plans; coverage visualisation; invariant + moment checks.
- [x] 7.4 Seed suite: overlaid renderings of two identically seeded plans (identical) and a differently seeded plan (diverging); exact-equality assertion.
- [x] 7.5 Make the harness fail programmatically on any tolerance/invariant violation (plots are evidence, not the gate); document how to run it in the crate README.

## 8. Validation & review

- [x] 8.1 `openspec validate add-nym-swizzle` passes.
- [x] 8.2 `cargo test -p nym-swizzle`, examples run, profiling harness runs and passes; review generated plots.
- [x] 8.3 Reviewer pass on the privacy semantics: rejection-resampling, snapping determinism, downward-only start extension, unvalidated-tuning caveats. Satisfied by the external review "Baseline hygiene for Zcash light clients: proposal for nym-swizzle" (C. Diaz, 2026-07-27), which reviewed the crate's mechanisms and proposed the Zcash-specific refinements now tracked as a separate change.

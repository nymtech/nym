# Proposal: address-nym-swizzle-pr-review

## Why

The nym-swizzle PR review (jstuczyn and CodeRabbit, 2026-08-03 through 2026-08-07) surfaced a mix of confirmed defects, modernization requests, and documentation gaps across `nym-swizzle` and `nym-swizzle-zcash`. Every verdict below has been triaged against the code and confirmed with the reviewer: some findings are real (a `Delay::bounds` panic on valid input, a `Committed` outcome that never checks requested-range completeness, live tests that CI's expensive-test step would run against a public server), some resolve to clarity rather than behavior (the `limit.max(1)` misreading), and two dependency requests interact and must land together (rand 0.10 and the modern `rand_core` trait).

## What Changes

**`nym-swizzle`:**

- **BREAKING (pre-publication API):** migrate from workspace `rand` 0.8 to the `rand010`/`rand_chacha010` aliases (rand 0.10.2), adding a paired `rand_distr` alias to the workspace; drop the hand-rolled `CryptoRngCore` trait in favour of the modern `rand_core` trait objects (in rand_core ≥0.9 the marker is `CryptoRng: RngCore`, so the boxed source becomes `Box<dyn CryptoRng + Send>` — exact spelling per the rand 0.10 re-exports); wasm randomness moves to the `getrandom04` backend story.
- Fix `Delay::bounds` to validate the supplied pair atomically (today it validates the new max against the old min and panics on valid input).
- Replace the constant-`min` fallback for unbounded-above rejection-resampling: the min-truncated exponential is sampled exactly via the memoryless shift (`min + fresh draw`); the normal fallback becomes a shifted half-normal — no fallback may concentrate mass on a constant (that spike is the fingerprint the crate exists to avoid). Regression tests for `Delay::poisson(..).min(..)`.
- Clarify the `limit.max(1)` lower clamp with comments at both sites (it guards the `futures` panic on a zero concurrency limit; two reviewers misread it as forcing sequential execution) — no API change; the review replies carry the correction.
- Add a results-yielding concurrent driver to `ChunkPlan` (stream of the closures' outputs), alongside the existing fire-and-forget `for_each_concurrent`.
- Switch logging from `log` to `tracing`.

**`nym-swizzle-zcash`:**

- Enforce complete delivery: every wire request must return exactly its half-open range; missing, duplicate, or out-of-range heights resolve a new `SyncError` variant instead of ever reaching `Committed` (closes block-withholding by a censoring server — previously only the verify window was tracked).
- Add a plan-storage slot: a `PlanStore`-style trait (save/load/clear) integrated with scheduling and resumption, and make `serde` a default feature so `BroadcastPlan` is serializable out of the box; the example persists through the trait instead of hand-rolled text lines.
- Gate live tests behind an environment variable instead of `#[ignore]` — this repo's CI runs `cargo test -- --ignored` as its expensive-test step, so ignored network tests would hit a public server on every CI run.
- Derive the live overhead bound from `spacing + VERIFY_LOOKAHEAD` instead of a fixed `3.0` (the fixed bound flakes ~0.4% of runs when boundary widening triggers).
- Example/support hygiene: the lightwalletd client returns a proper error for invalid ranges (a `debug_assert` vanishes in release builds) and names the half-open→inclusive wire conversion explicitly; `ZEC_GAP` is validated loudly against the tip; README snippets become self-contained (visible `TxBroadcaster` impl, explicit storage).

**Documentation and review-thread closure:**

- Scope the `nym-swizzle` spec's "snapping consumes no randomness" scenario to what is actually guaranteed (unchanged RNG state before planning; byte-identical plans when the start is already on-grid), and define jitter/snapping composition for irregular checkpoint lists (jitter in index units, snap last, lower-bound fallback).
- Annotate superseded tasks in `add-nym-swizzle-zcash-crate` (3.2/3.4 → 9.1) and scope task 2.5's invariant wording to valid inputs.
- Add a clarifying sentence to the zcash README distinguishing the quantization spacing (ladder-selected) from the wire-split unit (always `S_FLOOR`) — one CodeRabbit finding conflated them and readers might too.
- Post the agreed review-thread replies: the `limit.max(1)` correction (both threads), the `resume(self)` rationale (plan is `Copy`; consumption signals firing-is-terminal), and the streaming-`BlockSource` deferral (requests are bounded at `S_FLOOR` blocks, so `Vec` buffering is bounded; revisit with librustzcash integration).

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `nym-swizzle`: rejection-resampling fallback distribution for unbounded-above configurations; atomic bounds validation; zero-unrepresentable concurrency limits; a results-yielding concurrent driver; scoped snapping/RNG guarantees and checkpoint-list composition rules.
- `nym-swizzle-zcash`: complete-delivery enforcement before `Committed`; a plan-storage trait slot with serialization on by default; env-gated (not `#[ignore]`d) live tests; derived overhead bound; validated example inputs and error-returning support client. **Note:** this capability's spec currently lives as the delta in the in-flight `add-nym-swizzle-zcash-crate` change; archive that change before this one so the deltas here apply against a synced base spec.

## Impact

- `sdk/rust/nym-swizzle/`: `Cargo.toml` (rand010/rand_chacha010/rand_distr alias, tracing), `src/rng.rs` (trait swap, fallback), `src/delay.rs` (bounds), `src/range.rs` (results adapter, clamp comments), tests.
- `sdk/rust/nym-swizzle-zcash/`: `src/sync.rs` (delivery enforcement), `src/broadcast.rs` (PlanStore), `Cargo.toml` (serde default), `examples/`, `tests/live.rs` (env gating, derived bound), README.
- Root `Cargo.toml`: add the `rand_distr` alias paired with rand 0.10.
- `openspec/specs/nym-swizzle/spec.md` (via delta), `openspec/changes/add-nym-swizzle-zcash-crate/` (task annotations).
- PR #6984 review threads: replies posted for the discussion items; wasm guarantee and CI checks unchanged in scope but re-verified (getrandom 0.4 backend).

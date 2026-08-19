# Design: address-nym-swizzle-pr-review

## Context

PR #6984 received 26 review threads. Triage (confirmed with the human reviewer) sorted them into: real defects, modernization requests, clarity-only findings, documentation gaps, and two rejected/stale findings. This change implements everything accepted. Key verification results that shape the design:

- `limit.max(1)` at `range.rs:400` and `sync.rs:160` is a *lower* clamp (`std::cmp::max`), guarding the `futures` panic on a zero concurrency limit — not a forced-sequential bug. Two reviewers misread it, which is itself the finding.
- `reject_into_bounds` is already retry-bounded with an infinity-aware fallback (CodeRabbit's "infinite loop" claim is stale) — but the unbounded-max fallback returns the constant `min`, a probability spike at a bound, which is exactly the artifact the crate's rejection-resampling requirement exists to prevent.
- `WindowTracker` tracks only verify-window heights: a server omitting heights *inside the requested range* still yields `Committed`. CodeRabbit's finding was auto-marked "addressed" but was not.
- This repo's `Makefile` defines `test-expensive-main: cargo test --workspace -- --ignored` — `#[ignore]` means "expensive", not "never run", so the live suite would hit `zec.rocks` in CI.
- The live overhead test's fixed `3.0` bound is violated (~3.007) in the ~0.4% of runs where the resume point sits within `VERIFY_LOOKAHEAD` of a spacing boundary and widening triggers.
- develop now provides `rand010 = "0.10.2"`, `rand_chacha010 = "=0.10.0"`, and `getrandom04`; there is no rand-0.10-paired `rand_distr` alias yet. In rand_core ≥0.9 the `CryptoRngCore` convenience trait is gone; `CryptoRng` is a subtrait of `RngCore`, so a boxed crypto-grade source is spelled with the marker trait directly.

## Goals / Non-Goals

**Goals:**

- Land every accepted review item in one coherent pass, with regression tests where a defect was found.
- Keep both crates' invariants intact: wasm32 compiles for all non-dev deps, `nym-swizzle` stays chain-agnostic, `nym-swizzle-zcash` stays librustzcash-free with deterministic network-uniform emission.
- Close every review thread: fixes for the accepted items, written corrections for the two misread/stale items, rationale replies for the two deferred items.

**Non-Goals:**

- No streaming `BlockSource` (deferred: requests are bounded at `S_FLOOR = 1152` compact blocks, so `Vec` buffering is bounded at ~1–2 MB; revisit with librustzcash integration).
- No change to `resume(self)` semantics (`BroadcastPlan` is `Copy`; consumption is cost-free and signals firing-is-terminal — documented instead).
- No edits to archived change documents (`openspec/changes/archive/...`); only the live spec is corrected.
- No behavior change to quantization, grid constants, or broadcast profiles.

## Decisions

### D1. rand 0.10 via workspace aliases, `rand_core` traits instead of the hand-rolled one

Move `nym-swizzle` to `rand010`/`rand_chacha010` (already in the workspace) and add the paired `rand_distr` alias (the release line built against rand 0.10; exact version pinned at implementation). Delete the crate's `CryptoRngCore` trait: with rand_core ≥0.9, `CryptoRng: RngCore`, so `RngSource::Custom` boxes the marker trait directly (`Box<dyn CryptoRng + Send>`, spelled via the rand 0.10 re-exports so no direct `rand_core` dependency is needed). API renames (`gen_range` → `random_range`, etc.) follow mechanically. Wasm randomness moves to the `getrandom04` backend; the existing `getrandom_backend="wasm_js"` RUSTFLAGS in `sdk-wasm-lint` is verified against getrandom 0.4's cfg naming and updated if it changed.

*Sequencing note:* the reviewer's `CryptoRngCore` suggestion was written against rand 0.8 (`rand_core` 0.6.4, where the trait exists). Upgrading first changes the right answer — that is why these two items land as one decision.

### D2. `Delay::bounds` validates the pair atomically

`self.max(max).min(min)` checks the new max against the *old* min: `Delay::uniform(100s, 200s).bounds(0s, 20s)` panics on a valid pair. Fix: validate `min <= max` directly, assign both fields, keep the panic message format. Regression test: shrinking both bounds below the previous minimum succeeds.

### D3. No constant-mass fallback for unbounded-above rejection-resampling

When `max` is unbounded and the retry budget exhausts (mass below `min`), the current fallback returns exactly `min` — a deterministic delay, the boundary-spike fingerprint. Replacements:

- **Poisson/exponential:** the min-truncated exponential *is* `min + Exp(mean)` by memorylessness — sample it exactly, no retries needed at all when `max` is infinite and `min > 0`. This is a distribution-correct fast path, not a fallback.
- **Normal:** no memoryless trick exists; on exhaustion fall back to `min + |N(0, std_dev)|` (shifted half-normal) — still a documented distortion in the pathological case, but continuous, with no point mass.

Regression tests: `Delay::poisson(mean).min(m)` terminates, mean ≈ `m + mean`, no spike at `m`; normal-with-min exhaustion draws are non-constant.

### D4. Concurrency limits stay `usize`; the clamp gets a comment

Both `limit.max(1)` findings were misreadings of the lower clamp (`std::cmp::max(limit, 1)`), which guards the `futures` panic on a zero concurrency limit — behavior is correct. `NonZeroUsize` was considered as a type-level fix and rejected: it pushes `NonZeroUsize::new(n).unwrap()` ceremony onto every caller for a non-bug. The remedy is a one-line comment at each clamp site (`// lower clamp: futures panics on a zero concurrency limit`) in `ChunkPlan::for_each_concurrent`, the new results driver (D5), and `nym-swizzle-zcash`'s `fetch_concurrent`, plus the review replies (9.1) carrying the correction. No API change.

### D5. Results-yielding concurrent driver on `ChunkPlan`

Alongside `for_each_concurrent` (fire-and-forget), add a driver whose closure returns `T` and which yields each chunk's output as it completes — surfaced as a `futures::Stream` (`stream_concurrent(limit, f) -> impl Stream<Item = T>`), so callers can collect, short-circuit, or fold without side-channel state (today's example threads results through a `Mutex`). Inter-chunk delay composition behaves as in `for_each_concurrent` (pre-sampled per chunk). The existing driver stays: most traffic-shaping callers genuinely want fire-and-forget.

### D6. Complete-delivery enforcement in the sync driver

After each wire request `[s, e)`, the driver verifies the response delivered exactly the heights `s..e` (contiguity makes this a count-plus-endpoints check, no allocation proportional to range). Any missing, duplicate, or out-of-range height resolves a new `SyncError::IncompleteDelivery { request, missing/extra detail }` — the sync can then never reach `Committed` on a censored response. This extends the existing trust stance (the verify window already guards *history*; this guards the *requested range*). Unit tests: a source omitting interior heights errors; omitting cover-only heights also errors (the emitted range is the contract); the live suite is unaffected (well-behaved server).

### D7. Plan storage becomes a slot; serialization on by default

- New trait in `broadcast`, the third slot alongside `BlockSource`/`TxBroadcaster`:

```rust
pub trait PlanStore {
    type Error;
    async fn save(&mut self, plan: &StoredPlan) -> Result<(), Self::Error>;
    async fn load(&mut self) -> Result<Option<StoredPlan>, Self::Error>;
    async fn clear(&mut self) -> Result<(), Self::Error>;
}
```

  where `StoredPlan` couples the `BroadcastPlan` with the scheduling moment the wallet must already persist (as an opaque caller-supplied timestamp in seconds, since the crate cannot read clocks portably). Scheduling and resumption gain store-integrated helpers (`schedule_into(store)`, `resume_pending(store, broadcaster, build, now)`) so the persist-restore-clear lifecycle is one obvious path; the raw two-phase API remains for wallets with their own storage discipline.
- The `serde` cargo feature becomes a **default** feature (still disableable via `default-features = false` — the wasm guarantee is checked in both configurations). The example's hand-rolled text format is replaced by a `PlanStore` impl over a file using `serde_json`, which is what it was pretending not to need.

*Alternative considered:* postcard/proto per the review suggestion — rejected for the crate itself (the crate defines the data, the wallet owns the encoding); serde derives serve any of those encodings downstream.

### D8. Live tests gate on an environment variable

Replace `#[ignore]` with an explicit env gate: each live test returns early (with an eprintln) unless `NYM_SWIZZLE_ZCASH_LIVE_TESTS` is set (name final at implementation; `ZEC_*` family also acceptable). `cargo test` and `cargo test -- --ignored` are then both network-free; the README's "run the live suite" command becomes `NYM_SWIZZLE_ZCASH_LIVE_TESTS=1 cargo test -p nym-swizzle-zcash`.

### D9. Derived overhead bound in the live suite

Replace the fixed `quant_ratio <= 3.0` with the structural bound the arithmetic actually guarantees: `emitted_len <= (gap + 1) + spacing + VERIFY_LOOKAHEAD`. This is exact (start extension < spacing, plus one-cell widening only when the lookahead rule fires, end tip-capped) and cannot flake.

### D10. Example and support-client hygiene

- `Lightwalletd::block_range` returns `Status::invalid_argument` for `start >= end` (the `debug_assert` compiles out in release); the half-open→inclusive conversion gets a named local (`last_inclusive = end - 1`) so the wire convention is impossible to misread.
- `wallet_sync` validates `ZEC_GAP`: parse failures abort with a message (no silent default), and `gap` is checked against the fetched tip before subtraction.
- README send-path snippet becomes self-contained: a visible two-line `TxBroadcaster` impl and explicit storage via the new `PlanStore`, no free-floating `my_db`/`my_broadcaster`.

### D11. Documentation corrections (live docs only)

- `nym-swizzle` spec, snapping requirement: the no-RNG guarantee is stated as "the RNG state entering chunk planning is unchanged by snapping" with byte-identical plans guaranteed when the start is already on-grid (the unconditional identical-stream scenario overpromises: snapping that moves the start changes the plan length and thus the draw count). Checkpoint-list composition is defined: jitter samples in index units, snapping applies last (greatest checkpoint ≤ jittered start); "whole checkpoint intervals" language is scoped to fixed-spacing grids; below the first checkpoint, the smallest on-grid point within `[floor, true_start]` wins, else the start stays unsnapped — matching the implemented `obfuscated_start`.
- `add-nym-swizzle-zcash-crate/tasks.md`: 3.2/3.4 annotated "(superseded by 9.1)"; 2.5's invariant phrasing scoped to valid inputs (catch-ups above genesis; overhead bound per regime).
- Zcash README: one clarifying sentence distinguishing the ladder-selected quantization spacing from the fixed `S_FLOOR` wire-split unit.

## Risks / Trade-offs

- [rand 0.10 churn touches every sampling call] → mechanical renames covered by the full existing test suite (distribution moment checks, determinism tests) plus the profiling harness; the seeded-reproducibility tests pin that ChaCha20 streams still reproduce (seeds are opaque material; cross-version stream stability of the *same* rand_chacha major is expected — asserted by the existing fixed-seed tests, updated expectations if the 0.3→0.10 stream differs, with a note that persisted seeds don't exist yet so no compatibility break).
- [serde as a default feature widens the default dependency graph] → serde is optional-by-flag still; wasm check runs both with and without default features.
- [Delivery enforcement could reject servers that stream out of order] → the check is order-independent (set/count over the request's range), only completeness and range membership are enforced.
- [`StoredPlan` timestamp is caller-supplied] → same trust stance as `resume(elapsed)` today: a wallet that lies about time degrades only its own anonymity; documented.

## Migration Plan

Single PR-internal migration; no external consumers. Land as one change with the rand migration first (it moves the ground under the RNG-trait and fallback work), then the two crates' fixes, then docs and review-thread replies. Rollback is `git revert` of the change commits.

## Open Questions

- Exact `rand_distr` version pairing with rand 0.10 (pinned when adding the workspace alias).
- Final env-var name for the live-test gate (`NYM_SWIZZLE_ZCASH_LIVE_TESTS` vs `ZEC_LIVE_TESTS`) — cosmetic, decided at implementation.

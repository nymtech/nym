# Tasks: add-nym-swizzle-zcash-crate

## 1. Crate conversion

- [x] 1.1 Convert `sdk/rust/nym-swizzle-zcash/` from the example binary to a library crate: package name `nym-swizzle-zcash`, `publish = true`, workspace-inherited metadata; non-dev deps exactly `nym-swizzle` + `futures`; optional off-by-default `serde` feature; move `tonic`/`tonic-prost`/`prost`/`tokio` (and `serde_json` for the example's persistence) to `[dev-dependencies]`; delete `src/main.rs`.
- [x] 1.2 Move the hand-rolled prost/tonic lightwalletd client to example/test support (shared via `#[path]` include; no `build.rs`, no `protoc`), keeping its half-open→inclusive range conversion and adding nothing that requires a seed.
- [x] 1.3 Update any in-repo references to `nym-swizzle-zcash-example` (CI, docs) to the new package name, and extend the CI wasm check with `cargo check -p nym-swizzle-zcash --target wasm32-unknown-unknown`.
- [x] 1.4 Verify the wasm check passes on the empty-ish scaffold before feature work, so wasm breakage stays attributable per-commit (mirrors the `nym-swizzle` discipline); assert no tokio/gRPC/TLS in the resolved wasm graph (`cargo tree --target wasm32-unknown-unknown`).

## 2. Grid module (pure quantization)

- [x] 2.1 Named public constants: `SHARD = 144`, `S_FLOOR = 1152`, `VERIFY_LOOKAHEAD = 10`, with doc comments giving their derivation (ZIP 318 shard, ~one day, reorg lookahead); no configurability on the standard path (custom floor = fingerprint, say so in docs).
- [x] 2.2 Ladder selection + quantization: `quantize(range, kind, tip)` → emitted `[a', b')` per steps 1–3 (start down, end up, tip cap), returning a struct that retains the requested range for classification.
- [x] 2.3 Verify-lookahead enforcement: for a catch-up with resume point `a`, widen `a'` by one grid cell when `a − a' < VERIFY_LOOKAHEAD`; expose the verify window on the quantized result; `RangeKind::{Scan, Verify}` with `Verify` emitting no separate request.
- [x] 2.4 `classify(height, &quantized)` → `Disposition::{CoverBelow, VerifyWindow, Requested, CoverAbove}`.
- [x] 2.5 Unit tests: worked examples from the review's arithmetic (ladder choice per gap size, floor clamping, tip capping, same-cell collision, boundary widening at exactly 10, off-by-one edges at grid multiples), plus property-style tests (emitted range always contains requested range; `a' ≤ a − 10` for catch-ups above the genesis lookahead; `b' ≥ b` unless tip-capped; emitted length bounded by `len + 2·spacing`, with the near-genesis and one-block edge cases asserted separately — the sub-2× overhead figure applies per practical regime, not to degenerate gaps).

## 3. Sync driver + BlockSource slot

- [x] 3.1 `BlockSource` trait: generic block type, `block_range(start, end) -> Result<Vec<(u64, Block)>, Error>` over half-open ranges; document that the crate does no I/O and boundaries are chosen by the crate.
- [x] 3.2 `SyncSession::fetch`: quantize, build a seedable `nym_swizzle::Range` chunk plan over the emitted range, drive the source (sequential and bounded-concurrent variants mirroring `ChunkPlan`'s push drivers), deliver `(block, Disposition)` to the wallet's sink. *(Superseded by 9.1: the seedable chunk plan was replaced by deterministic ascending grid-cell requests.)*
- [x] 3.3 Verify-window bookkeeping: track which heights of the window have arrived, invoke the wallet's hash-comparison callback when the window is complete, resolve `SyncOutcome::{Committed, ReorgDetected}`; never reorder or serialize chunks to fetch the window early.
- [x] 3.4 Unit tests with an in-memory `BlockSource` fake: chunk union == emitted range (no gaps/spill), shuffled issue order, seeded reproducibility, commit withheld until verify answered, mismatch ⇒ `ReorgDetected`, dispositions correct across all four zones including cover-above dedupe signalling. *(Superseded by 9.1: shuffled-order and seeded-reproducibility tests became determinism/ascending-order tests.)*

## 4. Broadcast scheduler + TxBroadcaster slot

- [x] 4.1 `blocks(n)` conversion + `TARGET_BLOCK_TIME = 75s` constant.
- [x] 4.2 Profiles: `Scheduler::standard()` (exp mean `blocks(144)`, rejection-resampled above `blocks(576)` via `Delay::poisson(..).max(..)`) and `Scheduler::fast()` (mean `blocks(24)`, proportional cap), each documenting its anonymity-set trade-off.
- [x] 4.3 Two-phase API: `schedule()` samples once → `BroadcastPlan` (POD, public primitive fields; optional serde derives behind the feature); `resume(elapsed, broadcaster, build_tx)` sleeps only the remainder, invokes the builder at fire time (context carries the fresh-tip obligation for expiry = tip + 40), then hands the built bytes to the `TxBroadcaster`.
- [x] 4.4 `TxBroadcaster` trait, distinct from `BlockSource`; scheduler accepts only broadcasters, driver only sources; docs state different-session/different-server guidance.
- [x] 4.5 Anchor-age helper `needs_refresh_sync(last_synced_height, tip)` against the ZIP 318 anchor-retention bound, documented with the sync-on-its-own-session + later-timer pattern.
- [x] 4.6 Unit tests: single sampling (plan fields stable across mock restarts, resume never resamples), remainder arithmetic (elapsed ≥ delay fires immediately), builder invoked only after the delay (poll-counting), mock broadcaster receives builder output, profile sample bounds + mean (seeded statistical check), serde feature round-trip (feature-gated test), refresh-decision boundary.

## 5. Example (wallet-developer-facing)

- [x] 5.1 `examples/wallet_sync.rs` (name TBD at implementation): implement `BlockSource` with the tonic client, connect to `ZEC_SERVER` (default `zec.rocks:443`), fetch a real tip, quantize + sync a modest catch-up range, print the emitted grid-aligned requests vs. the naive request so the reader can see the difference on the wire.
- [x] 5.2 Same example, broadcast leg: schedule with a demo-scaled profile, persist the `BroadcastPlan` to a file, drop state, restore, resume, build a dummy transaction at fire time (fresh tip fetched in the builder), deliver to a mock `TxBroadcaster` that prints what it would have sent; no seed or funds involved.
- [x] 5.3 Example rustdoc written for wallet developers: what the lightwalletd would otherwise learn, what changes, zero internal-process references, and pointers to the verification commands.

## 6. Integration tests (opt-in, real chain data)

- [x] 6.1 Test scaffold: `#[ignore]` network tests under `tests/`, sharing the tonic client, `ZEC_SERVER` override, modest block counts, clear failure messages when the server is unreachable.
- [x] 6.2 Verify-window test: fetch a real window, record hashes as simulated wallet state, sync with resume point just above it ⇒ `Committed`; corrupt one stored hash ⇒ `ReorgDetected`; include a resume point within 10 blocks of a grid boundary to exercise the widening rule live.
- [x] 6.3 Grid-alignment test: run a live sync, assert every emitted request's start is a multiple of the chosen `S` and the end is grid-aligned or tip-capped.
- [x] 6.4 Overhead measurement: report cover-block overhead for a daily-incremental-sized range and a long-catch-up range against the ~2× bound, printed so a reader can eyeball the regimes.

## 7. README + docs contract

- [x] 7.1 Rewrite `README.md` for Zcash wallet developers: the two leaks in wallet terms (resume-point chaining, sync-then-send), what the crate does about each, the trait-slot integration story with a short code walkthrough, wasm note, and the unvalidated-tuning caveat carried over honestly.
- [x] 7.2 Tone pass: conversational, upbeat about Nym, no overpromising; every privacy claim paired with a verification hook (`cargo run --example ...` to watch grid alignment, `cargo test -- --ignored` for the live suite, wire-level inspection pointer).
- [x] 7.3 Sweep README, rustdoc, and examples for internal-process references (OpenSpec, Confluence, PR numbers, review-document citations) — none may appear; constants cite public sources only (ZIPs, librustzcash docs).

## 8. Validation

- [x] 8.1 `openspec validate add-nym-swizzle-zcash-crate` passes.
- [x] 8.2 `cargo test -p nym-swizzle-zcash` (unit) green; wasm check green; example runs against the live server; ignored suite run once locally and green.
- [x] 8.3 Confirm every spec scenario maps to a test or a documented manual verification step.

## 9. Reviewer follow-up (2026-08-06)

- [x] 9.1 Drop randomized chunking from the sync driver: the emitted range goes on the wire deterministically — `Quantized::requests()` splits at network-uniform `S_FLOOR`-aligned boundaries, ascending, disjoint, no random sizes/overlap/shuffle/seed; `SyncSession` collapses to free `sync::fetch` / `sync::fetch_concurrent`; docs, example, README, and live tests updated (cover is now the only sync overhead).
- [x] 9.2 Pin the network-wide range convention as half-open (ladder by half-open length), with boundary tests at 1152/2304-rung multiples and doc notes on the divergence an inclusive reading would cause.
- [x] 9.3 Pin `S_FLOOR` to the `SHARD * 2^j` ladder family with a compile-time assertion (divisibility alone misses non-power-of-two multiples); strengthened the ladder unit test to check the power-of-two quotient.

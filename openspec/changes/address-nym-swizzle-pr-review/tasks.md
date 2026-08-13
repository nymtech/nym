# Tasks: address-nym-swizzle-pr-review

## 1. rand 0.10 migration (nym-swizzle) — lands first, moves the ground under 2

- [x] 1.1 Add the rand-0.10-paired `rand_distr` alias to the workspace `Cargo.toml` (pin the exact version at implementation); switch `nym-swizzle` deps to `rand010`, `rand_chacha010`, the new distr alias, and `getrandom04` for the wasm target.
- [x] 1.2 Mechanical API migration (`gen_range` → `random_range`, `SliceRandom`/shuffle paths, `Exp`/`Normal` constructors from the new distr); verify the seeded-determinism tests still pin plan reproduction (adjust fixed-seed expectations if the ChaCha stream construction changed across majors — no persisted seeds exist, so no compatibility break, but note it in the commit).
- [x] 1.3 Replace the hand-rolled `CryptoRngCore` trait with the modern rand_core traits via the rand 0.10 re-exports (`CryptoRng: RngCore` in rand_core ≥0.9; boxed source becomes the marker-trait object with `Send`); update `with_rng` bounds and docs.
- [x] 1.4 Verify wasm: `cargo check -p nym-swizzle --target wasm32-unknown-unknown` and the `sdk-wasm-lint` RUSTFLAGS backend cfg against getrandom 0.4's naming (update the Makefile flag if the cfg changed); assert no getrandom-0.2 left in the wasm graph.
- [x] 1.5 Switch `log` → `tracing` (one call site in `rng.rs`; dep swap in Cargo.toml).

## 2. Delay/RNG fixes (nym-swizzle)

- [x] 2.1 `Delay::bounds` validates the supplied pair atomically (not against previous values); regression test: shrinking both bounds below the previous minimum succeeds; inverted pair still panics.
- [x] 2.2 Min-truncated exponential sampled exactly via the memoryless shift (`min + fresh draw`) when max is unbounded — a correct fast path, not a fallback; normal-with-unbounded-max exhaustion falls back to a shifted half-normal (`min + |N(0, std_dev)|`), never a constant. Update `reject_into_bounds`/`sample_bounded` structure accordingly and the rustdoc on the distortion caveat.
- [x] 2.3 Regression tests: `Delay::poisson(mean).min(m)` terminates, empirical mean ≈ `m + mean`, no point mass at `m`; normal-with-min pathological config draws non-constant samples; profiling harness updated if its expected moments shift.

## 3. Range driver (nym-swizzle)

- [x] 3.1 Add a clarifying comment at each `limit.max(1)` site (both crates): lower clamp guarding the `futures` panic on a zero concurrency limit; no API change.
- [x] 3.2 Add the results-yielding concurrent driver (`stream_concurrent(limit, f) -> impl Stream<Item = T>` or equivalent): one output per chunk, yielded on completion, inter-chunk delay composition matching `for_each_concurrent`; tests for output cardinality, concurrency bound, and delay composition.
- [x] 3.3 Migrate the wallet_sync-era pattern in `nym-swizzle`'s own examples where a `Mutex` threaded results through `for_each_concurrent`, to the new driver (demonstrates the API earning its keep).

## 4. Sync completeness (nym-swizzle-zcash)

- [x] 4.1 Per-request delivery verification in both fetch drivers: response must cover exactly `[start, end)` (order-independent count/endpoints check, no range-proportional allocation); new `SyncError::IncompleteDelivery` naming the request and the deficiency.
- [x] 4.2 Unit tests: source omitting wallet-requested heights errors; source omitting cover-only heights errors; source adding out-of-range or duplicate heights errors; well-behaved fake chain still commits; concurrent variant covered.

## 5. Plan storage (nym-swizzle-zcash)

- [x] 5.1 `StoredPlan` (plan + caller-supplied scheduling moment, seconds) and the `PlanStore` trait (save/load/clear, async, generic error); serde derives on both behind the feature.
- [x] 5.2 Store-integrated helpers: schedule-into-store and resume-pending (load → wait remainder → build at fire time → broadcast → clear; no-op when empty); raw two-phase API unchanged.
- [x] 5.3 Make `serde` a default feature; wasm check runs with and without default features; README feature docs updated.
- [x] 5.4 Unit tests: in-memory store lifecycle (schedule → restart simulation → resume → cleared), no-pending no-op, serde round-trip of `StoredPlan`.

## 6. Example + support client hygiene (nym-swizzle-zcash)

- [x] 6.1 `Lightwalletd::block_range` returns `Status::invalid_argument` for `start >= end` (drop the `debug_assert`); name the inclusive conversion (`last_inclusive = end - 1`) with a comment on lightwalletd's inclusive `BlockRange`.
- [x] 6.2 `wallet_sync`: unparseable env knobs abort with a message (no silent defaults); `ZEC_GAP` checked against the fetched tip before subtraction.
- [x] 6.3 Example persists through a file-backed `PlanStore` impl (serde_json via the default feature), replacing the hand-rolled two-line text format.
- [x] 6.4 README send-path snippet made self-contained: visible minimal `TxBroadcaster` impl, storage shown via `PlanStore`, no free-floating `my_db`/`my_broadcaster`; sync snippet re-checked against the current API.

## 7. Live-test gating and bounds (nym-swizzle-zcash)

- [x] 7.1 Replace `#[ignore]` with an env gate (working name `NYM_SWIZZLE_ZCASH_LIVE_TESTS`; final name at implementation): tests return early with an eprintln when unset; confirm `cargo test -- --ignored` makes no network connections.
- [x] 7.2 Replace the fixed `3.0` overhead assertion with the derived bound `emitted_len <= (gap + 1) + spacing + VERIFY_LOOKAHEAD`; keep the printed per-regime report.
- [x] 7.3 Update README/example pointers to the new invocation (`NYM_SWIZZLE_ZCASH_LIVE_TESTS=1 cargo test -p nym-swizzle-zcash`).

## 8. Documentation corrections

- [x] 8.1 Live `nym-swizzle` spec updated per the delta: scoped snapping/no-RNG scenario, checkpoint-list composition rules (applies at archive; verify wording matches `obfuscated_start` behavior). (delta wording verified against `obfuscated_start`; live-spec sync happens at archive)
- [x] 8.2 `add-nym-swizzle-zcash-crate/tasks.md`: annotate 3.2/3.4 "(superseded by 9.1)"; scope 2.5's invariant wording (catch-ups above genesis; per-regime overhead bound).
- [x] 8.3 Zcash README: one clarifying sentence distinguishing ladder-selected quantization spacing from the fixed `S_FLOOR` wire-split unit.

## 9. Review-thread closure (PR #6984)

- [x] 9.1 Reply to both `limit.max(1)` threads with the correction (lower clamp guarding the futures zero-limit panic) and point at the clarifying comments.
- [x] 9.2 Reply to the `resume(self)` thread: plan is `Copy`, consumption is cost-free and signals firing-is-terminal; doc sentence added.
- [x] 9.3 Reply to the streaming-`BlockSource` thread: deferred with the bounded-buffering rationale (`S_FLOOR` cap ≈ 1–2 MB per request); revisit with librustzcash integration.
- [x] 9.4 Reply to the README "day-sized cells" CodeRabbit thread: rejected as conflating the two grids, pointing at the new clarifying sentence (8.3).
- [ ] 9.5 Resolve the remaining CodeRabbit threads with fix-commit references once landed.

## 10. Validation

- [x] 10.1 `openspec validate address-nym-swizzle-pr-review` passes; archive ordering honoured (`add-nym-swizzle-zcash-crate` archives before this change). (archive ordering is enforced at archive time)
- [x] 10.2 Full sweep: `cargo test -p nym-swizzle -p nym-swizzle-zcash --all-features` and with `--no-default-features` on the zcash crate; clippy all targets both crates; wasm checks (both feature configurations); profiling harness; example run; gated live suite run once locally.
- [x] 10.3 Confirm every review thread in the final triage list maps to a fix commit, a posted reply, or a documented rejection.

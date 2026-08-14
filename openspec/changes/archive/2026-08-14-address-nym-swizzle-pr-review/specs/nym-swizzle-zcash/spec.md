# nym-swizzle-zcash

> Base-spec note: this capability's spec is the delta in the in-flight
> `add-nym-swizzle-zcash-crate` change. Archive that change first so these
> deltas apply against the synced base.

## ADDED Requirements

### Requirement: Complete delivery enforcement

The sync driver SHALL verify that every wire request's response delivers exactly the heights of the requested half-open range: no missing heights, no duplicates, no heights outside the range. Any violation MUST resolve a delivery error — a sync whose responses were censored or padded MUST NOT resolve `Committed`, regardless of whether the verify window itself passed. Order of delivery within a response is not constrained.

#### Scenario: Censoring server cannot reach Committed
- **WHEN** a source's response omits heights inside a request's range (whether wallet-requested or cover)
- **THEN** the sync resolves a delivery error naming the deficient request, and no `Committed` outcome is reported

#### Scenario: Well-behaved server is unaffected
- **WHEN** every response delivers exactly its requested range
- **THEN** the sync completes as before, with no additional network traffic or per-block allocation proportional to the range

### Requirement: Plan storage slot

The crate SHALL define a plan-storage trait as the wallet author's persistence slot: saving, loading, and clearing a stored plan that couples the broadcast plan with the caller-supplied scheduling moment. Scheduling and resumption SHALL offer store-integrated helpers so the persist–restart–resume–clear lifecycle is a single obvious path; the raw two-phase API (schedule, then resume with elapsed time) SHALL remain for wallets with their own storage discipline. Firing a stored broadcast SHALL clear the stored plan through the same slot.

#### Scenario: Store-integrated lifecycle survives restarts
- **WHEN** a wallet schedules through the store helper, the process restarts, and startup calls the resume helper with the current time
- **THEN** the pending plan is loaded from the wallet's store, the remaining delay elapses, the transaction is built at fire time and broadcast, and the store is cleared

#### Scenario: No pending plan is a no-op
- **WHEN** the resume helper runs and the store holds no plan
- **THEN** nothing fires and no error is raised

## MODIFIED Requirements

### Requirement: Decoupled broadcast scheduling with persistable plans

The crate SHALL provide a broadcast scheduler that samples a delay from an exponential distribution with mean 144 blocks, rejection-resampled (never clamped) above 576 blocks (standard profile), or mean 24 blocks with a proportional cap (fast profile). Scheduling SHALL be two-phase: `schedule()` samples the delay exactly once and returns a plain-old-data plan with public primitive fields that the wallet can persist; resuming a plan waits only the remaining delay (the wallet supplies elapsed time) and MUST NOT resample. Serde-derived serialization SHALL be available by default via a `serde` cargo feature that is on by default and can be disabled (`default-features = false`); persistence MUST NOT require it — the plan's fields remain public primitives.

#### Scenario: Delay sampled once, restart-safe
- **WHEN** a wallet schedules a broadcast, persists the plan, restarts, and resumes with the elapsed time
- **THEN** the broadcast fires after the originally sampled delay in total, with no resampling on resume

#### Scenario: Samples respect the profile bounds
- **WHEN** many delays are sampled from the standard profile
- **THEN** every sample is at most 576 blocks' worth of time, and the empirical mean approximates 144 blocks' worth (rejection-resampling, no boundary accumulation)

#### Scenario: Plans serialize out of the box
- **WHEN** a wallet builds the crate with default features
- **THEN** the plan (and the stored form coupling it with the scheduling moment) derives `Serialize`/`Deserialize`, and disabling default features still compiles for `wasm32-unknown-unknown`

### Requirement: Opt-in integration tests on real chain data

The crate SHALL include integration tests that exercise the verify-window rule against real chain data (stored hashes match ⇒ committed; a corrupted stored hash ⇒ reorg detected), assert that every emitted range lands on grid boundaries, and measure quantization overhead for the two practical regimes (daily incremental sync and long catch-up). The live tests MUST be gated behind an explicit environment variable and skip cleanly (with a message) when it is unset — they MUST NOT rely on `#[ignore]`, because this repository's CI runs ignored tests as its expensive-test step. Asserted overhead bounds MUST be derived from the quantization arithmetic (`emitted length <= requested length + spacing + VERIFY_LOOKAHEAD`), not fixed ratios that boundary widening can exceed. Tests MUST keep block counts modest and the server overridable.

#### Scenario: Reorg rule verified on real data
- **WHEN** the live suite runs with the gating environment variable set, against a live lightwalletd
- **THEN** a sync whose stored verify-window hashes match real chain data commits, and the same sync with one corrupted stored hash reports a reorg

#### Scenario: Overhead measured per regime
- **WHEN** the overhead test runs
- **THEN** it reports cover-block overhead separately for a daily-sync-sized range and a long catch-up range, asserting only the derived structural bound

#### Scenario: CI stays network-free
- **WHEN** `cargo test` runs without the gating environment variable — including with `-- --ignored`
- **THEN** no network connection is attempted and the live tests report themselves skipped

### Requirement: Live example with real sync and mocked send

The crate SHALL ship a runnable example that connects to a real public lightwalletd (default `zec.rocks`, overridable) to fetch compact blocks through the quantized, deterministic sync path using a `BlockSource` implementation included with the example, and demonstrates the broadcast path end-to-end — schedule, persist through the plan-storage slot, simulate a restart, resume, build at fire time, and deliver to a mock `TxBroadcaster` — without requiring a wallet seed or funds. Example inputs MUST fail loudly: a malformed or out-of-range knob (e.g. a gap exceeding the tip) aborts with a clear message rather than silently substituting a default. The example's support client MUST return errors for invalid requests rather than relying on debug assertions, and MUST name its half-open-to-inclusive wire conversion explicitly.

#### Scenario: Example fetches real blocks
- **WHEN** the example runs with network access
- **THEN** it fetches a real block range from the configured lightwalletd via the trait slot and reports the emitted (grid-aligned) ranges it requested

#### Scenario: Example demonstrates save-and-resume
- **WHEN** the example schedules a broadcast
- **THEN** it persists the plan via the plan-storage slot, drops all in-memory state, restores, resumes, and the mock broadcaster receives the transaction built at fire time

#### Scenario: Malformed knobs abort loudly
- **WHEN** the example runs with an unparseable or tip-exceeding gap setting
- **THEN** it exits with an explanatory error instead of proceeding with a silent default

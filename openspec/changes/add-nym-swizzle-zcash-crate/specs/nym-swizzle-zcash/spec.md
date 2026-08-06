# nym-swizzle-zcash

## ADDED Requirements

### Requirement: Quantized sync-range emission

The crate SHALL provide a pure quantization function that maps a queued half-open scan range `[a, b)` and the public chain tip to an emitted range `[a', b')` on a network-wide grid: the spacing `S` MUST be the smallest rung of the ladder `S_j = 144 * 2^j` that is `>= b - a`, clamped to a minimum of `S_floor = 1152`; the emitted start MUST be `a' = S * floor(a / S)`; the emitted end MUST be `b' = min(S * ceil(b / S), tip + 1)` (the tip itself is always includable since it is public). The constants (shard size 144, floor 1152) MUST be exposed as named public constants and MUST NOT be configurable knobs on the standard path (a custom floor is a fingerprint). Because the quantization rule is network-wide by construction, the range-reading convention MUST be pinned: ranges are half-open and the ladder is selected by half-open length, with tests fixing the boundary cases at rung multiples (a 1152-block range takes `S = 1152`, a 1153-block range takes `S = 2304`); and `S_floor` MUST be verified (at compile time or by test) to sit on the `144 * 2^j` ladder — shard divisibility alone does not guarantee the nesting.

#### Scenario: Start rounds down and end rounds up to the grid
- **WHEN** a queued range `[a, b)` is quantized with a tip far above `b`
- **THEN** the emitted start is the greatest multiple of `S` that is `<= a`, the emitted end is the least multiple of `S` that is `>= b`, and `S` is the smallest ladder rung `max(1152, 144·2^j) >= b − a`

#### Scenario: Emitted end never exceeds the tip
- **WHEN** rounding the end up to the grid would pass the chain tip
- **THEN** the emitted end is capped at the tip, and a queued range ending at the tip emits an end at the tip

#### Scenario: Identical requests for clients in the same grid cell
- **WHEN** two wallets with different resume points inside the same grid cell quantize their catch-up ranges at the same tip
- **THEN** both produce byte-identical emitted ranges

#### Scenario: Half-open convention pinned at rung boundaries
- **WHEN** a queued range is exactly one rung long (e.g. 1152 or 2304 blocks, half-open)
- **THEN** the spacing equals that rung, one more block selects the next rung, and a wallet exactly 1152 blocks behind the tip (a 1153-block half-open catch-up) takes `S = 2304` — an implementation reading the rule inclusively would diverge here and partition the collision sets

#### Scenario: Floor stays on the ladder family
- **WHEN** the crate is compiled
- **THEN** `S_floor` is asserted to be a power-of-two multiple of the shard, so the doubling ladder keeps nesting with the ZIP 318 anchor grid

### Requirement: Verify-window handling

Quantization SHALL accept a range kind distinguishing normal scan ranges from the reorg-verify range. For a sync with a resume point `a`, the crate MUST NOT emit any separate request for the verify range (the `VERIFY_LOOKAHEAD = 10` blocks below `a`); instead it MUST enforce `a - a' >= VERIFY_LOOKAHEAD` on the emitted catch-up range, widening the start by exactly one grid cell (`a' = a' - S`) when the condition fails. The sync driver MUST run the wallet's hash comparison on the verify-window blocks as their request arrives and MUST NOT report the sync as committable until the comparison passes.

#### Scenario: Resume point too close to a grid boundary widens by one cell
- **WHEN** a resume point lies fewer than 10 blocks above the grid-rounded start
- **THEN** the emitted start moves down by one full grid spacing so the verify window is inside the emitted range

#### Scenario: Commit withheld until verify passes
- **WHEN** all requests of a sync have been fetched but the verify-window hash comparison has not yet been answered
- **THEN** the sync outcome is not reported as committed, and it is reported as committed only after the wallet's comparison confirms the stored hashes match

#### Scenario: Hash mismatch surfaces a reorg
- **WHEN** the wallet's verify-window comparison reports a mismatch
- **THEN** the sync resolves with a reorg-detected outcome so the wallet can rewind and requeue exactly as its SDK does today

### Requirement: Cover-block classification

The crate SHALL classify every delivered block height relative to the quantized range so the wallet applies the correct rule: heights below the requested start (outside the verify window) are cover and MUST be marked for discard without re-scanning or duplicate note insertion; heights in the verify window are marked for hash comparison; heights inside the requested range are marked for normal scanning; heights above the requested end (grid cover) are marked for dedupe-then-scan since they may be new.

#### Scenario: Cover below the requested start is discarded
- **WHEN** a delivered height is below the wallet's true resume point and outside the verify window
- **THEN** it is classified as cover-below, signalling discard without scanning

#### Scenario: Cover above the requested end is scanned after dedupe
- **WHEN** a delivered height is at or above the wallet's queued end but inside the emitted range
- **THEN** it is classified as cover-above, signalling dedupe against scan state and scanning of what is new

### Requirement: Deterministic sync execution with a transport slot

The crate SHALL define a `BlockSource` trait as the wallet author's slot for their own lightwalletd transport, taking a half-open height range and returning height-tagged, otherwise-opaque blocks (the block type is generic; the crate performs no network I/O). The sync driver SHALL put the emitted range on the wire deterministically: split at network-uniform, `S_floor`-aligned boundaries, issued in ascending order, disjoint and gapless, with no random sizes, no overlap, and no shuffling — randomization among wallets that already emit an identical union buys nothing against the lightwalletd adversary, costs bandwidth, and per-wallet variation would itself be a distinguishing dimension. Every block SHALL be delivered to the wallet together with its classification.

#### Scenario: Wallet author plugs in their own transport
- **WHEN** a wallet author implements `BlockSource` for their gRPC client and runs a sync
- **THEN** every network request is made through their implementation, with request boundaries chosen by the crate (quantized range, grid-aligned split)

#### Scenario: Requests cover exactly the emitted range
- **WHEN** a sync executes over an emitted range
- **THEN** the requests are ascending, disjoint `S_floor`-aligned cells (the last shorter only when tip-capped) whose union equals the emitted range — no gaps, no spill, no re-fetched blocks

#### Scenario: Identical wallet states emit identical requests
- **WHEN** two syncs run from the same queued range and tip
- **THEN** they issue byte-identical requests in identical order, with no randomness involved

### Requirement: Decoupled broadcast scheduling with persistable plans

The crate SHALL provide a broadcast scheduler that samples a delay from an exponential distribution with mean 144 blocks, rejection-resampled (never clamped) above 576 blocks (standard profile), or mean 24 blocks with a proportional cap (fast profile). Scheduling SHALL be two-phase: `schedule()` samples the delay exactly once and returns a plain-old-data plan with public primitive fields that the wallet can persist; resuming a plan waits only the remaining delay (the wallet supplies elapsed time) and MUST NOT resample. A `serde` cargo feature (disabled by default) MAY add derive-based serialization; persistence MUST NOT require it.

#### Scenario: Delay sampled once, restart-safe
- **WHEN** a wallet schedules a broadcast, persists the plan, restarts, and resumes with the elapsed time
- **THEN** the broadcast fires after the originally sampled delay in total, with no resampling on resume

#### Scenario: Samples respect the profile bounds
- **WHEN** many delays are sampled from the standard profile
- **THEN** every sample is at most 576 blocks' worth of time, and the empirical mean approximates 144 blocks' worth (rejection-resampling, no boundary accumulation)

### Requirement: Transaction building deferred to fire time

The broadcast API SHALL accept the transaction builder as a callback invoked only after the delay elapses, so expiry is derived from a tip fetched at fire time (expiry = fresh tip + 40). The API MUST make it impossible for the crate itself to use a pre-delay tip for expiry, and documentation MUST warn that stale-tip expiry reveals the last sync height on-chain.

#### Scenario: Builder runs after the delay
- **WHEN** a scheduled broadcast fires
- **THEN** the wallet's build callback executes at fire time and the built transaction is then handed to the broadcaster slot

### Requirement: Broadcast transport slot separate from sync

The crate SHALL define a `TxBroadcaster` trait, distinct from `BlockSource`, as the slot for the wallet author's send transport; the scheduler SHALL accept only a `TxBroadcaster`, and the sync driver only a `BlockSource`, so no crate API ever couples one session object to both roles. Documentation MUST state that sync and broadcast should use different sessions and preferably different servers.

#### Scenario: Send path exercisable with a mock
- **WHEN** a developer implements `TxBroadcaster` with a mock that records the raw transaction instead of sending it
- **THEN** the full schedule–persist–resume–build–broadcast path runs without a funded wallet

### Requirement: Anchor-age refresh decision

The crate SHALL provide a pure helper deciding whether a broadcast may proceed without a preceding sync: given the last-synced height and the current tip, it returns that no refresh is needed while the gap is within the ZIP 318 anchor-retention bound, and that a refresh sync (on its own session, with the broadcast on a later timer) is needed otherwise.

#### Scenario: Recent sync broadcasts without a preceding request
- **WHEN** the last sync is younger than the anchor-retention bound
- **THEN** the helper approves building and sending with no preceding sync request

### Requirement: Block-denominated time conversion

The crate SHALL expose the Zcash target block time as a named constant (75 seconds) and a conversion function from a block count to a `Duration`, so delay parameters are written in blocks as in the protocol literature and no caller hardcodes the block time.

#### Scenario: Delays written in blocks
- **WHEN** a developer configures the standard profile
- **THEN** the parameters read as block counts (mean 144, cap 576) and convert via the exposed constant

### Requirement: Dependency discipline and wasm compatibility

The library's mandatory dependencies SHALL be limited to `nym-swizzle` and `futures` (plus optional `serde` behind the off-by-default feature). It MUST NOT depend on librustzcash or any Zcash consensus/wallet crate (the crate targets upstreaming into librustzcash; a dependency would be circular). Every non-dev dependency MUST compile for `wasm32-unknown-unknown`, and the check MUST be wired into CI alongside `nym-swizzle`'s. Network stacks (tonic, TLS) are confined to dev-dependencies.

#### Scenario: wasm target check passes
- **WHEN** `cargo check -p nym-swizzle-zcash --target wasm32-unknown-unknown` runs in CI
- **THEN** it succeeds, with no tokio or gRPC/TLS stack in the resolved wasm dependency graph

### Requirement: Live example with real sync and mocked send

The crate SHALL ship a runnable example that connects to a real public lightwalletd (default `zec.rocks`, overridable) to fetch compact blocks through the quantized, deterministic sync path using a `BlockSource` implementation included with the example, and demonstrates the broadcast path end-to-end — schedule, persist the plan to disk, simulate a restart, resume, build at fire time, and deliver to a mock `TxBroadcaster` — without requiring a wallet seed or funds.

#### Scenario: Example fetches real blocks
- **WHEN** the example runs with network access
- **THEN** it fetches a real block range from the configured lightwalletd via the trait slot and reports the emitted (grid-aligned) ranges it requested

#### Scenario: Example demonstrates save-and-resume
- **WHEN** the example schedules a broadcast
- **THEN** it persists the plan, drops all in-memory state, restores the plan from disk, resumes, and the mock broadcaster receives the transaction built at fire time

### Requirement: Opt-in integration tests on real chain data

The crate SHALL include integration tests, ignored by default (run explicitly, e.g. `cargo test -- --ignored`), that exercise the verify-window rule against real chain data (stored hashes match ⇒ committed; a corrupted stored hash ⇒ reorg detected), assert that every emitted range lands on grid boundaries, and measure quantization overhead for the two practical regimes (daily incremental sync and long catch-up) against the review's ~2x bound. Tests MUST keep block counts modest and the server overridable.

#### Scenario: Reorg rule verified on real data
- **WHEN** the ignored test suite runs against a live lightwalletd
- **THEN** a sync whose stored verify-window hashes match real chain data commits, and the same sync with one corrupted stored hash reports a reorg

#### Scenario: Overhead measured per regime
- **WHEN** the overhead test runs
- **THEN** it reports cover-block overhead separately for a daily-sync-sized range and a long catch-up range

### Requirement: Documentation written for Zcash wallet developers

The README and example documentation SHALL address Zcash wallet developers as the sole audience: they MUST be standalone, with no references to internal process artifacts (issue trackers, internal documents, proposal/spec files, PR numbers); the tone SHALL be conversational and positive about Nym while avoiding overpromised anonymity claims (unvalidated tuning is labelled as such); and every substantive privacy claim MUST be paired with a concrete way for the reader to verify it themselves (runnable tests, inspecting emitted ranges, wire-level observation).

#### Scenario: Docs stand alone
- **WHEN** a wallet developer reads the README with no access to internal systems
- **THEN** nothing in it refers to or depends on internal process artifacts, and every referenced resource is public

#### Scenario: Claims come with verification hooks
- **WHEN** the README asserts a privacy property (e.g. grid-aligned requests, restart-safe delays)
- **THEN** it points to a command or procedure the reader can run to check the property locally

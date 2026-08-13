# nym-swizzle

## ADDED Requirements

### Requirement: Randomized delay scheduling of async work

The crate SHALL provide a `delay` primitive constructed with minimum and maximum delay bounds (and a distribution configuration, see the distributions requirement) that wraps a caller-supplied future or async closure. On each invocation it MUST sample a fresh delay within `[min, max]`, wait that long, then execute the wrapped future and return its output unchanged. The wrapped future MUST NOT be polled before its scheduled time. Successive invocations of the same configured instance MUST draw independent samples.

#### Scenario: Broadcast is deferred by a sampled delay
- **WHEN** a delay instance configured with `min = 0s, max = 10s` runs `async move { broadcast_tx(...) }`
- **THEN** a delay `d` with `0s <= d <= 10s` is sampled, the future is first polled only after `d` has elapsed, and the broadcast's result is returned to the caller

#### Scenario: No early side effects
- **WHEN** the wrapped future would perform an observable side effect on first poll
- **THEN** that side effect cannot occur before the sampled delay has elapsed

#### Scenario: Independent samples per call
- **WHEN** the same delay instance is invoked repeatedly
- **THEN** each invocation samples a new delay independently rather than reusing the first sample

### Requirement: Configurable delay distributions with rejection-resampling

The delay sampler SHALL support at least three distributions: uniform over `[min, max]`; Poisson-process (exponential inter-arrival times with a configured mean, matching the mixnet's `sample_poisson_duration` construction); and normal with configured mean and standard deviation. For unbounded distributions, samples falling outside `[min, max]` MUST be discarded and redrawn (rejection-resampling) rather than clamped, so no probability spike accumulates at the bounds. A bounded retry count MUST guard against non-terminating resampling for pathological configurations, falling back to a uniform in-bounds draw.

#### Scenario: Exponential samples respect bounds without a boundary spike
- **WHEN** many delays are drawn from a Poisson-process configuration with a maximum bound
- **THEN** all returned delays lie within the bounds and the empirical distribution shows no excess mass at the maximum

#### Scenario: Normal distribution with configured spread
- **WHEN** a normal configuration with mean `m` and standard deviation `s` is sampled many times
- **THEN** the in-bounds samples are consistent with `Normal(m, s)` restricted to the bounds

#### Scenario: Pathological configuration terminates
- **WHEN** a distribution's mass lies almost entirely outside `[min, max]`
- **THEN** sampling still terminates via the bounded-retry fallback and returns an in-bounds value

### Requirement: Range decomposition into overlapping randomized chunks

The crate SHALL provide a `range` primitive that decomposes a requested index range into chunks with randomly sampled sizes, where consecutive chunks (in index order) overlap by a randomly sampled amount within configured `[min_overlap, max_overlap]` bounds (e.g. `1..100` becomes `1..10, 8..12, 11..21, 19..50, 45..75, ...`). The union of all chunks MUST equal the (start-obfuscated) range exactly: full coverage, no spill past the end. Chunk-size bounds and overlap bounds MUST be configurable; when unset, the overlap default SHALL be derived as a clamped fraction of the total range so it degrades sensibly for both very small and very large ranges. Overlapping (duplicated) indexes are expected output; deduplication is the caller's responsibility.

#### Scenario: Full coverage with overlaps
- **WHEN** a range `0..1000` is decomposed
- **THEN** the union of emitted chunks is exactly `0..1000` and every consecutive chunk pair overlaps by an amount within the configured overlap bounds

#### Scenario: Small range still valid
- **WHEN** a range spanning fewer indexes than the default chunk size is decomposed
- **THEN** a valid covering plan is still produced with the defaults clamped to fit

#### Scenario: No spill past the end
- **WHEN** any plan is generated
- **THEN** no chunk extends beyond the requested end index

### Requirement: Randomized execution order with pull and push styles

The chunk plan SHALL be executable in a uniformly random permutation of chunk order, in two styles: a pull-style iterator yielding `(start, end)` pairs until all chunks have been yielded, and a push-style async driver to which the caller hands an async closure over `(start, end)` and which owns execution — sequential or with bounded concurrency — until every chunk has been executed. The push driver SHALL optionally compose with the delay primitive to insert sampled delays between chunk executions.

#### Scenario: Pull iteration visits every chunk once
- **WHEN** the iterator is drained
- **THEN** every chunk in the plan is yielded exactly once, in randomly permuted (not index) order

#### Scenario: Concurrent push execution completes the plan
- **WHEN** the driver runs an async closure with concurrency limit `n`
- **THEN** at most `n` chunk executions are in flight at once and the driver resolves only after all chunks have executed

#### Scenario: Inter-chunk delay composition
- **WHEN** the driver is configured with a delay instance
- **THEN** a freshly sampled delay separates chunk executions

### Requirement: Randomized start overlap (start-height obfuscation by noise)

The range primitive SHALL support extending the start of the requested range downward by a randomly sampled amount drawn from a configured distribution, clamped to a configurable floor (default 0), so the emitted start precedes the true resume point and previously held data is deliberately re-fetched. When no jitter distribution is configured, the default jitter magnitude SHALL be derived as a clamped percentage of the total range. This is the only sanctioned extension outside the caller's requested range; the end is never extended.

#### Scenario: Start moves down, never up
- **WHEN** start jitter is enabled for a range `1000..2000`
- **THEN** the plan's start is `1000 - j` for a sampled `j >= 0` and the plan's end remains `2000`

#### Scenario: Floor is respected
- **WHEN** the sampled jitter would move the start below the configured floor
- **THEN** the start is clamped to the floor

### Requirement: Checkpoint snapping (start-height obfuscation by collision)

The range primitive SHALL support deterministically rounding the plan's start down to a caller-supplied checkpoint grid, given either a fixed spacing or an explicit list of checkpoint indexes. Snapping MUST be a pure function of the true start and the grid: it MUST NOT sample from or advance the random number generator, so that independent clients resuming within the same checkpoint interval emit an identical start. When both jitter and snapping are enabled, jitter MUST be expressed in whole checkpoint intervals — the sampled jitter moves the emitted start down by an integer number of checkpoints — so the emitted start always lies on the grid and the collision property is preserved.

#### Scenario: Clients in the same interval collide
- **WHEN** two independent clients with true starts `1042` and `1097` snap to spacing `100`
- **THEN** both emit start `1000`

#### Scenario: Snapping consumes no randomness
- **WHEN** two identically seeded plans are generated, one with snapping enabled and one without
- **THEN** the sequence of random samples drawn (chunk sizes, overlaps, permutation) is identical in both

#### Scenario: Explicit checkpoint list
- **WHEN** an explicit checkpoint list is supplied instead of a fixed spacing
- **THEN** the start is rounded down to the greatest checkpoint not exceeding it

#### Scenario: Jitter composes in checkpoint units
- **WHEN** both start jitter and snapping (spacing `100`) are enabled for a true start of `1042`
- **THEN** the emitted start is `1000 - 100k` for a sampled integer `k >= 0`, never an off-grid value

### Requirement: Configurable and reproducible randomness

All sampling SHALL require an RNG satisfying `Rng + CryptoRng`. The default source MUST be `OsRng`. The crate SHALL additionally accept a fixed seed (via a seedable ChaCha CSPRNG) such that two runs with the same seed and configuration produce byte-identical chunk plans and identical delay sequences — including seeds derived externally from a VRF, which the crate treats as opaque seed material. The crate SHALL also accept a caller-supplied generic RNG instance.

#### Scenario: Same seed, same plan
- **WHEN** two range plans are generated with identical configuration and the same seed
- **THEN** their chunk lists, execution permutations, and any sampled delays are identical

#### Scenario: Different seed diverges
- **WHEN** the same configuration is run with a different seed
- **THEN** the resulting plan differs

#### Scenario: Default is non-deterministic and cryptographically sourced
- **WHEN** no seed or RNG is supplied
- **THEN** sampling uses `OsRng`

### Requirement: Wasm compilability as a hard dependency constraint

The crate MUST compile for `wasm32-unknown-unknown`, and every non-dev dependency MUST be wasm-compilable, so that a separate `wasm-pack` wrapper crate can distribute it with JavaScript conveniences without modification. Time-based waiting MUST be isolated behind a single cfg-gated internal abstraction: `tokio` time gated to non-wasm targets, `wasmtimer` gated to `wasm32` (the convention used by `common/http-api-client` and `common/client-core`). Randomness on wasm MUST route through `getrandom` such that a wrapper enabling its `js` feature makes `OsRng` functional in the browser. A `wasm32-unknown-unknown` check MUST be part of the crate's test deliverable. Dev-dependencies are exempt from the constraint.

#### Scenario: Wasm build
- **WHEN** `cargo check --target wasm32-unknown-unknown` runs against the crate
- **THEN** it compiles, with `tokio` absent from the resolved wasm dependency graph and `wasmtimer` providing the timer

#### Scenario: Wrappable by wasm-pack
- **WHEN** a separate wrapper crate depends on `nym-swizzle`, enables `getrandom/js`, and is built with `wasm-pack`
- **THEN** the delay and range primitives function in the wasm environment without any change to `nym-swizzle`

#### Scenario: Dependency regression is caught
- **WHEN** a newly added non-dev dependency does not compile for `wasm32-unknown-unknown`
- **THEN** the wasm check in the crate's test deliverable fails

### Requirement: Runnable examples covering the primary use cases

The crate SHALL ship runnable examples demonstrating: (1) delaying a broadcast by a sampled duration, (2) fetching an index range via overlapping shuffled chunks, (3) sampling delays from the Poisson-process distribution, and (4) seeded (VRF-style) deterministic sampling in which two identically seeded runs are shown to produce the same result.

#### Scenario: Examples build and run
- **WHEN** the crate's examples are executed
- **THEN** each of the four examples compiles and runs to completion, and the seeded example prints two identical plans

### Requirement: Development-time profiling harness with plot output

The crate SHALL include a development-time profiling harness (dev-dependencies only) that empirically validates, with rendered plots and backing numeric checks: (a) sampled delays follow each configured distribution within its bounds, with no boundary spike from resampling; (b) chunk sizes, overlaps, and start jitter follow their configured distributions, and generated plans satisfy the coverage invariant; (c) seeded runs are honoured — identically seeded runs produce identical output, rendered for visual confirmation and asserted for exact equality. Each plot MUST be paired with an automated tolerance check (e.g. sample moments vs theoretical moments) so the harness fails programmatically rather than only visually.

#### Scenario: Delay distribution plots
- **WHEN** the profiling harness runs the delay suite
- **THEN** it emits, per distribution, a histogram of sampled delays overlaid on the theoretical density restricted to the bounds, and fails if sample moments deviate beyond tolerance

#### Scenario: Chunk geometry plots
- **WHEN** the profiling harness runs the chunking suite
- **THEN** it emits chunk-size, overlap, and start-jitter histograms plus a coverage visualisation across many plans, and fails if any plan violates the coverage invariant or if geometry moments deviate beyond tolerance

#### Scenario: Determinism proof
- **WHEN** the profiling harness runs the seed suite
- **THEN** it renders two identically seeded plans overlaid (identical) and one differently seeded plan (diverging), and fails unless the identically seeded outputs are exactly equal

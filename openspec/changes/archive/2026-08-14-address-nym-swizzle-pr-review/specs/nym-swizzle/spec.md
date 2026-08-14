# nym-swizzle

## MODIFIED Requirements

### Requirement: Configurable delay distributions with rejection-resampling

The delay sampler SHALL support at least three distributions: uniform over `[min, max]`; Poisson-process (exponential inter-arrival times with a configured mean, matching the mixnet's `sample_poisson_duration` construction); and normal with configured mean and standard deviation. For unbounded distributions, samples falling outside `[min, max]` MUST be discarded and redrawn (rejection-resampling) rather than clamped, so no probability spike accumulates at the bounds. A bounded retry count MUST guard against non-terminating resampling for pathological configurations. No termination fallback may concentrate probability mass on a single constant: for a min-truncated exponential with no upper bound, the sampler SHALL use the exact memoryless form (`min` plus a fresh exponential draw); for other unbounded-above configurations the exhaustion fallback SHALL be a continuous distribution anchored at `min` (e.g. a shifted half-normal); for bounded configurations the uniform in-bounds fallback remains. Reconfiguring both bounds together MUST validate the supplied pair against each other, not against previously configured values.

#### Scenario: Exponential samples respect bounds without a boundary spike
- **WHEN** many delays are drawn from a Poisson-process configuration with a maximum bound
- **THEN** all returned delays lie within the bounds and the empirical distribution shows no excess mass at the maximum

#### Scenario: Normal distribution with configured spread
- **WHEN** a normal configuration with mean `m` and standard deviation `s` is sampled many times
- **THEN** the in-bounds samples are consistent with `Normal(m, s)` restricted to the bounds

#### Scenario: Pathological configuration terminates
- **WHEN** a distribution's mass lies almost entirely outside `[min, max]`
- **THEN** sampling still terminates via the bounded-retry fallback and returns an in-bounds value

#### Scenario: Min-only exponential is exact and spike-free
- **WHEN** a Poisson-process configuration has a minimum bound and no maximum
- **THEN** sampling terminates without retry exhaustion, the empirical mean approximates `min + mean`, and no probability mass concentrates at exactly `min`

#### Scenario: Rebinding both bounds validates the new pair
- **WHEN** an existing sampler with bounds `[100s, 200s]` is reconfigured with the valid pair `[0s, 20s]`
- **THEN** the reconfiguration succeeds; only a pair with `min > max` is rejected

### Requirement: Randomized execution order with pull and push styles

The chunk plan SHALL be executable in a uniformly random permutation of chunk order, in two styles: a pull-style iterator yielding `(start, end)` pairs until all chunks have been yielded, and a push-style async driver to which the caller hands an async closure over `(start, end)` and which owns execution — sequential or with bounded concurrency — until every chunk has been executed. The push driver SHALL optionally compose with the delay primitive to insert sampled delays between chunk executions. A results-yielding form of the concurrent driver SHALL also be provided: given an async closure returning a value, it yields each chunk's output as that chunk completes, so callers can collect or short-circuit without side-channel state.

#### Scenario: Pull iteration visits every chunk once
- **WHEN** the iterator is drained
- **THEN** every chunk in the plan is yielded exactly once, in randomly permuted (not index) order

#### Scenario: Concurrent push execution completes the plan
- **WHEN** the driver runs an async closure with concurrency limit `n`
- **THEN** at most `n` chunk executions are in flight at once and the driver resolves only after all chunks have executed

#### Scenario: Inter-chunk delay composition
- **WHEN** the driver is configured with a delay instance
- **THEN** a freshly sampled delay separates chunk executions

#### Scenario: Results driver yields every chunk's output
- **WHEN** the results-yielding driver runs an async closure returning a value, with bounded concurrency
- **THEN** the caller receives exactly one output per chunk, as each completes, and the outputs correspond one-to-one with the plan's chunks

### Requirement: Checkpoint snapping (start-height obfuscation by collision)

The range primitive SHALL support deterministically rounding the plan's start down to a caller-supplied checkpoint grid, given either a fixed spacing or an explicit list of checkpoint indexes. Snapping MUST be a pure function of the true start and the grid: the random-number-generator state entering chunk planning MUST be identical whether or not snapping is enabled, so that independent clients resuming within the same checkpoint interval emit an identical start. When both jitter and snapping are enabled, the jitter magnitude is sampled in index units and snapping applies last (the emitted start is the greatest checkpoint not exceeding the jittered start), so the emitted start always lies on the grid; for fixed-spacing grids this is equivalent to jitter in whole checkpoint intervals. When the jittered start lies below the first checkpoint of an explicit list (or below the floor's grid cell), the smallest on-grid point within `[floor, true start]` SHALL be used, and if no such point exists the start remains unsnapped.

#### Scenario: Clients in the same interval collide
- **WHEN** two independent clients with true starts `1042` and `1097` snap to spacing `100`
- **THEN** both emit start `1000`

#### Scenario: Snapping consumes no randomness
- **WHEN** two identically seeded plans are generated over a range whose start is already on-grid, one with snapping enabled and one without
- **THEN** the plans are byte-identical (chunk sizes, overlaps, permutation), demonstrating that enabling snapping draws no samples; when snapping moves the start, the RNG state entering chunk planning is still unchanged, though the resulting plans legitimately differ in length

#### Scenario: Explicit checkpoint list
- **WHEN** an explicit checkpoint list is supplied instead of a fixed spacing
- **THEN** the start is rounded down to the greatest checkpoint not exceeding it, and a start below every checkpoint falls back to the smallest on-grid point within `[floor, true start]` or stays unsnapped

#### Scenario: Jitter composes in checkpoint units
- **WHEN** both start jitter and snapping (spacing `100`) are enabled for a true start of `1042`
- **THEN** the emitted start is `1000 - 100k` for a sampled integer `k >= 0`, never an off-grid value

# Design: `nym-swizzle`

## Context

Wallet-class applications leak through traffic shape even when transport anonymity is perfect: broadcast timing correlates with the client's own sync milestones (a V2 timing leak observed at the destination), and index-addressed range requests expose interest and — via the start height acting as a linking key — chain otherwise-unlinkable sessions together (a V3 content leak). The mixnet cannot remove either; the application must shape its own traffic.

The mitigations are generic transforms over "an async action" and "an index range": randomized scheduling, overlapping randomized chunking, randomized start overlap, checkpoint snapping. The repo already contains the seed of one of them — `sample_poisson_duration` in `common/nymsphinx/src/utils/mod.rs:13`, the mixnet's own exponential inter-arrival sampling — but nothing reusable at the application layer. `nym-swizzle` packages these transforms as one small SDK utility crate.

The API stance is inspired by quickcheck-family testing libraries: the caller hands the harness a function; the harness owns inputs, repetition, and order. That inversion is what lets the range driver shuffle, parallelise, and interleave delays without the caller coordinating any of it.

## Goals / Non-Goals

**Goals:**
- One crate, two primitives (`delay`, `range`), one shared randomness configuration.
- Satisfy, out of the box, the private-wallet sync requirements: randomized broadcast timers decorrelated from sync milestones; overlapping shuffled block fetches; randomized start overlap; checkpoint snapping.
- Deterministic reproducibility from a seed (tests, replay, VRF-derived seeds).
- Wasm compatibility from day one — wallets are the stated audience and wallets are often wasm.
- Development-time statistical proof: a profiling harness with plot output demonstrating that delays, chunk geometry, and seeded determinism behave as specified.

**Non-Goals:**
- Transport selection, destination splitting (sync via one server, broadcast via another), or session management — app concerns, documented in examples only.
- Range widening for interest-masking beyond the start edge (fetching `0..500` when `100..110` is needed): the crate cannot know which indexes exist (chain tip, array bounds), so the caller widens; the crate guarantees obfuscated full coverage of whatever range it is given. The one exception is the downward start edge — see Decision 6.
- Deduplication of overlapping results: index-addressed data is idempotent; dedup is the caller's job.
- Built-in VRF evaluation or proofs — see Decision 9.
- Validated tuning numbers for overlap distributions / checkpoint spacing — see Open Questions.

## Decisions

### Decision 1: Standalone crate at `sdk/rust/nym-swizzle` with zero nym dependencies

A sibling of `nym-sdk` (which is a single crate, not a container), added to the root workspace. Dependencies are limited to `rand`, `rand_distr`, `rand_chacha` (already workspace deps), `futures`, and a cfg-gated timer. Zero nym deps keeps it independently publishable, trivially wasm-able, and honest: it is a generic traffic-shaping utility that happens to live in the nym repo. The only tempting nym dep — `sample_poisson_duration` — is 8 lines, mirrored locally (Decision 5).

### Decision 2: Two primitives, one shared randomness configuration

`delay` and `range` are one crate, not two, because they compose: inter-chunk timing jitter in the range driver *is* the delay primitive. Both draw from a single randomness configuration type (distribution choice + bounds + RNG source) so an application configures its obfuscation posture once.

### Decision 3: The wrapped future is not polled until its scheduled time

Stated as a hard API guarantee. Async blocks are lazy, so `s.run(async move { broadcast_tx(...) })` samples a delay, sleeps, *then* polls the future for the first time. This matters because the observable event (the send) is the execution itself; a "delay the result, start the work immediately" semantic would obfuscate nothing. Delay-then-execute, result passed through unchanged.

### Decision 4: Rejection-resampling, never truncation-clamping

Clamping a sampled value to `max` piles probability mass into a spike at exactly `max` — a statistical fingerprint distinguishable from the pure distribution. Out-of-bounds samples are discarded and redrawn instead. A bounded retry count (with fallback to a uniform draw within bounds, logged/debug-asserted) guards against pathological configurations (e.g. normal with mean far outside `[min, max]`) turning into unbounded loops.

### Decision 5: "Poisson" means exponential inter-arrival times, mirroring the mixnet

A Poisson *process* is specified by exponential inter-event durations; the `Poisson` distribution proper yields event counts, not durations. The delay sampler therefore uses `Exp(1/mean)` exactly as `sample_poisson_duration` does — deliberately, because delays drawn from the same distribution family as mixnet cover traffic blend into traffic the adversary already observes. Bounds enforced per Decision 4.

### Decision 6: Start-edge obfuscation is built in; it is the one sanctioned outward extension

Earlier in exploration the stance was "no outward spill; callers widen ranges themselves." The private-wallet requirements revise that for the **start edge specifically**, where extension is downward (earlier indexes always exist, unlike indexes past the tip) and is precisely the mitigation for start-height linking:

- **Randomized overlap** (`start_jitter`): extend the start downward by a sampled amount (any configured distribution), clamped to a configurable floor (default 0), re-fetching data already held. Starts stop being exact pointers to previous ends; cross-session chaining degrades to approximate, deniable joins. Anonymity by *noise*.
- **Checkpoint snapping** (`snap_start`): round the start down to a caller-supplied grid — fixed spacing or an explicit checkpoint list (chains have irregular canonical checkpoints). Every client resuming within the same interval emits an identical start; the collision window grows from one block to the checkpoint spacing. Anonymity by *collision*.

The two compose: when snapping is enabled, jitter is expressed in whole checkpoint intervals (Resolved Question 2), so the emitted start is always on-grid and jitter only moves *which* checkpoint is chosen — noise and collision stack instead of undermining each other.

### Decision 7: Checkpoint snapping is deterministic and consumes no randomness

The entire value of snapping is that independent clients collide on identical starts. Any RNG involvement destroys that. Snapping is a pure function of (true start, grid); it must not advance the RNG state either, so that seeded plans with and without snapping stay comparable.

### Decision 8: Pull and push execution styles

- **Pull**: the chunk plan is generated up-front (sizes and overlaps sampled per configuration, full-coverage invariant checked), randomly permuted, and exposed as `Iterator<Item = (u64, u64)>`. Simple, caller-driven, inherently sequential.
- **Push** (the quickcheck-style layer): an async driver — `for_each` / `for_each_concurrent(n, |start, end| ...)` — that owns execution: shuffled order, bounded concurrency, optional per-chunk delay sampled from the shared configuration. "Randomly sampled until all chunks have been executed" is the driver's completion condition.

Invariants in both styles: union of chunks == the (obfuscated-start) range, no spill past the end; every adjacent pair of chunks (in index order) overlaps by an amount in `[min_overlap, max_overlap]`; execution order is a uniform random permutation; duplicates are expected and the caller dedups.

### Decision 9: Three RNG tiers; VRF support via seed injection, not built-in

1. `OsRng` — default, unpredictable, satisfies the `CryptoRng` bound all sampling requires (resolving for this crate the TODO left open in `nymsphinx/utils`).
2. `ChaCha20Rng::from_seed(seed)` — deterministic reproducibility: same seed ⇒ byte-identical chunk plan and delay sequence. `ChaCha20` is a seedable CSPRNG, not a VRF; determinism-from-seed is what tests and replay actually need.
3. Generic `R: Rng + CryptoRng` injection — bring-your-own. A caller who needs *verifiable* randomness evaluates their VRF (ECVRF etc.) elsewhere and feeds the output in as a seed; the crate stays VRF-agnostic while fully supporting the workflow.

### Decision 10: Wasm compilability is a hard constraint on the dependency tree

The crate does not build a wasm distribution itself, but it MUST be wrappable, unmodified, by a separate `wasm-pack` wrapper crate that adds JavaScript conveniences. That makes "compiles for `wasm32-unknown-unknown`" a dependency-selection rule, not an aspiration:

- **Every non-dev dependency must compile to `wasm32-unknown-unknown`.** Current tree satisfies this: `rand`/`rand_chacha`/`rand_distr` and `futures` are pure; `OsRng` reaches the browser via `getrandom` (workspace dep, 0.2) whose `js` feature the wasm wrapper enables — this crate only needs the target-gated dependency declared.
- **Timer**: `tokio` (time) under `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`; `wasmtimer` under `[target.'cfg(target_arch = "wasm32")'.dependencies]` — the exact convention already used by `common/http-api-client` and `common/client-core`. The sleep call is isolated behind one internal function so the rest of the crate is target-agnostic.
- **Enforcement**: `cargo check --target wasm32-unknown-unknown` is part of the test deliverable, so a dependency regression fails fast rather than surfacing when someone builds the wrapper.
- Dev-dependencies (`plotters`, `statrs`, full `tokio` for tests/examples) are exempt — they never constrain downstream consumers.
- The wasm-pack wrapper crate itself (JS API design, `wasm-bindgen` surface, packaging) is out of scope for this change; this crate's obligation is to be wrappable.

### Decision 11: Overlap defaults are derived fractions with clamps, presented as unvalidated

Defaults for both chunk overlap and start jitter are percentages of the total range (Resolved Question 1). A raw percentage breaks at both extremes (absurd overlaps for `1..1_000_000`, zero for `1..10`), so defaults are clamped derivations, e.g. `default_overlap = clamp(total/20, 1, max_chunk/2)`, with min/max overlap, chunk-size bounds, and jitter distribution all settable. Per the source requirements, wider overlap/spacing buys a larger anonymity set at the cost of re-downloaded data and **no settled numbers exist** — the percentage defaults are conservative and documented as a trade-off knob, not a validated recommendation.

### Decision 12: Development-time profiling harness with plot output

Statistical claims ("delays follow the configured distribution", "chunk geometry follows its distributions", "seeds are honoured") are provable and should be proven, benchmark-style, at development time:

- A `profiling` harness (cargo bench target or feature-gated example; dev-dependencies only — `plotters` for SVG output, optionally `statrs` for test statistics) that draws large samples (~10⁴–10⁵) per configuration.
- **Delays**: empirical histogram per distribution (uniform, Poisson/exponential, normal) overlaid on the theoretical density restricted to `[min, max]`; visually confirms rejection-resampling leaves no boundary spike.
- **Chunking**: histograms of chunk sizes and pairwise overlaps across many generated plans, plus a coverage plot confirming the union invariant; start-jitter distribution histogram; snapping shown as a step plot (many true starts → few emitted starts).
- **Determinism**: two runs from the same seed rendered as overlaid chunk plans (visually identical) *and* asserted byte-equal; a third run from a different seed shown diverging.
- Every plot is backed by an automated numeric check (sample mean/variance within tolerance of theoretical moments; exact equality for determinism) so the harness fails loudly in CI-like use, not just prettily in a browser.

## Risks / Trade-offs

- **Wasteful by design**: overlapping chunks and start overlap multiply server load; a server with aggressive rate limits may punish exactly the clients trying to be private. Mitigation: concurrency bound + delay composition in the push driver; documented.
- **Parallel chunks over one mixnet client** may serialise anyway in the client's internal queues, weakening the "parallel" promise. The crate is transport-agnostic; the example notes this.
- **Rejection-resampling cost** is unbounded in theory; bounded-retry fallback (Decision 4) trades a small distributional distortion in pathological configs for termination.
- **Anonymity-set arithmetic is unquantified** (see Resolved Question 1 — defaults chosen, quantification still open research); shipping defaults could be mistaken for validated parameters. Mitigation: explicit "unvalidated" documentation and the profiling harness making distributions inspectable.

## Resolved Questions

1. **Tuning numbers** — *resolved: ship percentage-of-range defaults.* Default overlap and default start jitter are each derived as a percentage of the total range (clamped per Decision 11). The quantitative anonymity-set arithmetic remains open research (unchanged from the source note), but the crate does not block on it: defaults are sensible percentages, documented as a trade-off knob rather than validated parameters.
2. **Jitter-then-snap composition** — *resolved: yes, jitter in checkpoint units.* When snapping is enabled, start jitter is expressed in whole checkpoint intervals rather than raw indexes: the sampled jitter moves the emitted start down by an integer number of checkpoints. Emitted starts therefore always lie on the grid, and jitter widens *which* checkpoint is chosen instead of smearing starts off-grid (which would destroy the collision property). Without snapping, jitter remains in raw indexes.
3. **Plan materialisation** — *resolved: up-front.* The chunk plan is generated eagerly, enabling the exact-permutation and full-coverage guarantees and byte-identical seeded plans. Lazy/streaming generation for very large ranges is deferred until a concrete need appears.

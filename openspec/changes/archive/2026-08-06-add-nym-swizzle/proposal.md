# Add `nym-swizzle`: traffic-shape obfuscation utilities for app builders

## Why

The mixnet hides *who* is talking; it does not hide *what an application's query pattern says about it*. Wallet-class applications — anything that fetches sequential, index-addressed data (blocks, notes, checkpoints) or broadcasts at meaningful moments — leak through two channels that in-transit mixing cannot remove:

- **Timing correlation (V2).** The destination observes wall-clock arrival. A wallet that broadcasts a transaction immediately after reaching chain tip, or right after a fee query, is trivially correlatable with its own sync activity. The wallet must decorrelate broadcasts itself.
- **Content / index correlation (V3).** A light client that requests exactly blocks `4_120_000..4_120_010` tells the serving node its resume point. Worse, the *start height acts as a linking key across sessions*: today's start is yesterday's end, so successive otherwise-unlinkable sessions chain into one client history.

Every nym-powered wallet or sync client currently has to reinvent these mitigations ad hoc. The mitigations themselves are well understood (randomized delays; overlapping, shuffled range chunking; randomized start overlap; checkpoint snapping) and are pure application-layer transforms with no dependency on nym internals — they belong in one small, reusable, wasm-friendly utility crate in the SDK.

## What Changes

- New crate `sdk/rust/nym-swizzle`, added to the workspace, with **zero nym dependencies** (only `rand`, `rand_distr`, `rand_chacha`, `futures`, and a cfg-gated timer for native/wasm). Independently publishable and usable outside the nym stack.
- **Wasm compilability is a hard constraint, not a goal**: every non-dev dependency MUST compile for `wasm32-unknown-unknown`. The crate does not produce a wasm distribution itself, but it must be wrappable as-is by a separate `wasm-pack` wrapper crate providing JavaScript conveniences. Timer via `wasmtimer` on `wasm32` (existing repo convention, cf. `common/http-api-client`, `common/client-core`); `OsRng` via `getrandom` with the `js` feature enabled by the wrapper.
- **`delay` primitive**: wraps an async closure/future and schedules it after a randomly sampled delay; the wrapped future is guaranteed not to be polled before its scheduled time; the sampled result is returned to the caller. Min/max bounds set at construction.
- **`range` primitive**: decomposes a requested index range (e.g. `0..1000`) into randomly sized, deliberately overlapping chunks that fully cover the range; executable as a pull-style iterator of `(start, end)` pairs in randomly permuted order, or as a push-style async driver that runs chunks concurrently, quickcheck-style (the harness owns execution). Inter-chunk timing composes with the `delay` primitive.
- **Start-edge obfuscation** on `range`, matching private-wallet sync requirements:
  - *Randomized overlap*: extend the start a sampled number of indexes **downward** (before the true resume point), re-fetching data already held, so starts stop being exact pointers to previous ends and session chaining degrades to approximate, deniable joins. Clamped to a configurable floor.
  - *Checkpoint snapping*: round the start down to a caller-supplied checkpoint grid (fixed spacing or explicit list), deterministically and without consuming randomness, so every client resuming within the same checkpoint interval emits an **identical** start (anonymity by collision rather than by noise).
- **Shared randomness configuration** used by both primitives: `OsRng` by default (all sampling behind a `CryptoRng` bound); uniform, Poisson-process (exponential inter-arrival, mirroring the mixnet's `sample_poisson_duration`), and normal (configurable std dev) distributions; bounds enforced by **rejection-resampling**, never truncation-clamping. Seedable via `ChaCha20Rng::from_seed` for reproducible plans (same seed ⇒ identical chunk plan and delays), and generic over caller-supplied `Rng + CryptoRng` so VRF-derived seeds plug in without the crate knowing about VRFs.
- **Runnable examples** shipped with the crate: (1) delaying a broadcast by a random duration, (2) fetching blocks via overlapping shuffled chunks, (3) Poisson-distributed delay sampling, (4) seeded/VRF-style deterministic sampling showing two identically seeded runs produce the same plan.
- **Tests**: unit tests for coverage/overlap/ordering invariants, laziness of the delay wrapper, rejection-resampling bounds, snapping determinism, and seed reproducibility.
- **Profiling harness** (development-time, dev-dependencies only): a benchmark-style harness that empirically proves the statistical claims and emits plots — (a) sampled delays follow the configured distribution within its bounds (empirical histogram overlaid on the theoretical density, per distribution), (b) chunk sizes and overlaps follow their configured distributions and the coverage invariant holds over many generated plans, (c) seeded/VRF-style runs are honoured (two identically seeded runs render to identical plans; visualised overlay plus exact-equality check). Plots are written as SVG artifacts; automated moment checks (mean/variance within tolerance) back the visual output so the harness fails loudly, not just prettily.

## Capabilities

### New Capabilities
- `nym-swizzle`: application-layer traffic-shape obfuscation — randomized delay scheduling of async work, overlapping randomized range chunking with pull/push execution, start-edge obfuscation (randomized start overlap, checkpoint snapping), and configurable/seedable randomness.

### Modified Capabilities
<!-- None. No existing capability's requirements change. -->

## Impact

- **New crate only**: `sdk/rust/nym-swizzle/` plus a `members` entry in the root workspace `Cargo.toml`. No existing crate is modified.
- **Dependencies**: `rand`, `rand_distr`, `rand_chacha`, `getrandom` (all already workspace deps), `futures`; `tokio` (time feature) gated to non-wasm targets and `wasmtimer` gated to `wasm32` (matching `common/http-api-client` / `common/client-core`). Every non-dev dependency must compile to `wasm32-unknown-unknown`; `cargo check --target wasm32-unknown-unknown` is part of the deliverable. Profiling adds `plotters` (and optionally `statrs`) as **dev-dependencies only** — dev-deps do not constrain downstream wasm consumers.
- **Non-goals**: transport/destination splitting (sync via one server, broadcast via another) and range *widening for interest-masking beyond the start edge* remain application decisions — documented in the crate docs and examples, not implemented.
- **Tuning is exposed, not settled**: overlap distributions and checkpoint spacing widen the anonymity set at the cost of re-downloaded data; there are no settled numbers. The crate ships conservative defaults and documents the trade-off; it does not claim validated parameters.

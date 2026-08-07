# nym-swizzle

Application-layer traffic-shape obfuscation for privacy-preserving apps —
primarily wallets, or anything that fetches sequential, index-addressed data
(blocks, notes, checkpoints) or broadcasts at meaningful moments.

A mixnet hides *who* is talking; it does not hide *what your query pattern
says about you*. `nym-swizzle` provides two composable primitives to shape
the pattern itself:

- **`delay`** — schedule an async action after a randomly sampled delay
  (uniform, Poisson-process, or normal, with rejection-resampled bounds). The
  wrapped future is guaranteed not to be polled before its scheduled time.
- **`range`** — decompose an index range into randomly sized, deliberately
  overlapping, shuffled chunks with full-coverage guarantees, plus start-edge
  obfuscation: randomized start overlap (anonymity by noise) and checkpoint
  snapping (anonymity by collision). Push drivers execute the plan with
  bounded concurrency, either fire-and-forget (`for_each_concurrent`) or
  yielding each chunk's result as it completes (`stream_concurrent`).

All sampling is crypto-grade (OS entropy by default) and seedable (ChaCha20)
for reproducible plans — seeds derived from a VRF are treated as opaque seed
material.

```rust
use std::time::Duration;
use nym_swizzle::{Delay, Range, Snap};

// decorrelate a broadcast from the sync milestone that triggered it
let mut s = Delay::uniform(Duration::ZERO, Duration::from_secs(10));
let result = s.run(async move { broadcast_tx(tx).await }).await;

// resume a sync without your start height linking you to yesterday's session
Range::new(resume_height, tip)
    .snap_start(Snap::Spacing(1000))
    .start_jitter(2500)
    .plan()
    .for_each_concurrent(4, |start, end| get_blocks(start, end))
    .await;
```

## Examples

```sh
cargo run -p nym-swizzle --example delay_broadcast
cargo run -p nym-swizzle --example fetch_blocks_overlapping
cargo run -p nym-swizzle --example poisson_sampling
cargo run -p nym-swizzle --example seeded_vrf
```

### Zcash: ready-made policy layer

For Zcash light clients there is a dedicated crate,
[`nym-swizzle-zcash`](../nym-swizzle-zcash), which packages these primitives
into chain-specific policy (quantized sync ranges, decoupled persistable
broadcast scheduling) and ships a live example against a public lightwalletd:

```sh
cargo run --release -p nym-swizzle-zcash --example wallet_sync
```

It is a separate crate so its example's gRPC/TLS stack (dev-dependencies
only) never touches this crate's wasm dependency guarantee.

## Profiling harness

A development-time harness proves the statistical claims (delay
distributions, chunk geometry, seeded determinism) with SVG plots backed by
hard numeric checks:

```sh
cargo run --release -p nym-swizzle --example profiling
# plots land in <workspace>/target/swizzle-profiling/
```

The harness streams 10M samples per delay distribution (and 50k chunk plans /
500k jitter observations) through fixed-size accumulators, holding every
sample moment within 0.5–1% of theory.

## Wasm

Every non-dev dependency compiles for `wasm32-unknown-unknown`; the crate is
designed to be wrapped, unmodified, by a `wasm-pack` wrapper crate with
JavaScript conveniences. Verify with:

```sh
cargo check -p nym-swizzle --target wasm32-unknown-unknown
```

## Caveats

- Overlap widths and checkpoint spacing trade anonymity-set size against
  re-downloaded data; there are **no settled numbers** — the defaults are
  conservative starting points, not validated recommendations.
- Transport concerns (never broadcast over the sync session; destination
  splitting) and deduplication of overlapping results stay the application's
  responsibility.

# nym-swizzle-zcash-example

A live, network-facing example of [`nym-swizzle`](../nym-swizzle): fetching
real Zcash compact blocks from a public `lightwalletd` over gRPC, naively
versus with overlapping chunking, and measuring what the obfuscation costs.

The synthetic examples in `nym-swizzle` show the *shape* of the traffic.
This one answers the question a wallet author actually asks: **what do I pay
for it?**

## The leak this addresses

A light client that asks a `lightwalletd` for exactly the blocks it needs
hands the server a precise statement of its interest. Worse, because each
session's start height is the previous session's end, that start height acts
as a **linking key** that chains otherwise-unlinkable sessions into one
client history.

`nym-swizzle` breaks the request into randomly sized, deliberately
overlapping, randomly ordered chunks, so no single request describes the
client's actual interval.

## Running it

```sh
cargo run --release -p nym-swizzle-zcash-example
```

It talks to a **real public server**. The defaults are deliberately gentle —
1000 blocks, 4 concurrent requests — and the block range is chosen behind the
chain tip so it cannot straddle a reorg mid-run.

| variable | default | meaning |
|---|---|---|
| `ZEC_SERVER` | `https://zec.rocks:443` | lightwalletd endpoint |
| `ZEC_BLOCKS` | `1000` | how many blocks to sync |
| `ZEC_CONCURRENCY` | `4` | in-flight requests for run 3 |
| `ZEC_TRIALS` | `3` | trials per strategy; results are medians |

## What it measures

Three strategies over the *same* block range:

1. **direct** — one `GetBlockRange` call for the whole range (what a naive
   wallet does).
2. **swizzled, sequential** — overlapping chunks, one at a time. Isolates the
   cost of the deliberate redundancy.
3. **swizzled, concurrent** — the same chunks with bounded concurrency. Shows
   how much of that cost wall-clock time recovers.

Every run asserts it received **every** block in the range, so an incomplete
fetch fails the run rather than printing a plausible-looking number.

### Sample output

```
chain tip 3424513; fetching blocks 3422513..3423513 (1000 blocks)

== summary (median of 3 trials) ==
  direct                     0.08s    1.00x baseline, 1 request(s), 0.0% waste
  swizzled, sequential       0.35s    4.26x baseline, 8 request(s), 25.0% waste
  swizzled, 4 concurrent     0.11s    1.33x baseline, 8 request(s), 25.0% waste
```

**Obfuscation costs bandwidth; concurrency is what buys the wall-clock back.**
About 25% extra blocks were transferred, yet the concurrent run finished
within 1.33x of the unobfuscated baseline.

## Why the comparison is trustworthy

Two details do the work here, and both were added after a first version
produced numbers that looked fine but meant little:

- **Both swizzled runs share a seed**, so they execute a byte-identical plan
  and concurrency is the only variable between them. Without this they draw
  different plans (different chunk counts, different wastage) and runs 2 and
  3 simply are not comparable. The example asserts the two plans matched.
- **Timings are medians over several trials, after a discarded warm-up pass**
  that equalises server-side caching across runs. Single samples are far too
  noisy for this claim: the sequential ratio swung between 1.9x and 12.8x
  across consecutive invocations before this was added.

## Why this is a separate crate

`nym-swizzle` guarantees that every one of its non-dev dependencies compiles
to `wasm32-unknown-unknown`, so a `wasm-pack` wrapper can distribute it
unmodified. A gRPC/TLS stack does not compile to wasm. Keeping this example
in its own crate preserves that guarantee, and keeps `cargo test -p
nym-swizzle` fast.

## Implementation notes

- The lightwalletd messages in `src/lightwalletd.rs` are **hand-written
  `prost` structs** rather than code generated from `.proto` files, so there
  is no build script and no `protoc` requirement. `prost` skips unknown
  fields on decode, so declaring a subset of each message stays
  forward-compatible. Field numbers and the service path come from
  lightwalletd's `compact_formats.proto` / `service.proto`.
- lightwalletd's `BlockRange` is **inclusive at both ends**, while
  `nym-swizzle` chunks are half-open `[start, end)`. The conversion happens
  at the wire boundary in `block_range`.

## What this example does *not* do

It hides the shape of the request only. A real wallet must also:

- never broadcast over the sync connection or session,
- decorrelate broadcast timing from sync milestones (see
  `nym_swizzle::Delay`),
- obfuscate its start height so sessions cannot be chained by resume point
  (`Range::start_jitter` / `Range::snap_start`, demonstrated in
  `nym-swizzle`'s `fetch_blocks_overlapping` example),
- consider destination splitting: sync from one server, broadcast through
  another.

Deduplicating the overlapping blocks is the caller's job — compact blocks are
idempotent, so re-fetched heights can simply be dropped.

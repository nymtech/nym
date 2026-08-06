# nym-swizzle-zcash

Baseline privacy hygiene for Zcash light clients: quantized sync ranges and
decoupled, restart-safe broadcast scheduling, packaged as a small library you
plug your own `lightwalletd` client into.

Built by [Nym](https://nym.com) on top of [`nym-swizzle`](../nym-swizzle),
and deliberately tiny: two dependencies (`nym-swizzle`, `futures`), no
network stack, no build script, compiles to `wasm32-unknown-unknown`. You
keep your transport; the crate decides what goes on the wire and when.

## The problem: your lightwalletd is watching

Suppose your users' transport is perfect — IP rotated on every request,
nothing linkable at the network layer. The server still learns plenty from
**what** a wallet asks and **when**:

1. **Resume-point chaining.** A wallet resumes at exactly
   `previous end + 1` and syncs to the tip. Each request start names one
   specific block, and today's start is yesterday's end — the server can
   chain a wallet's sessions into one history by height alone.
2. **Sync-then-send.** Wallets sync right before broadcasting, so the server
   attributes each `SendTransaction` to the sync session seconds earlier —
   and, via chaining, to the wallet's whole history.

Both leaks survive any mixnet, VPN, or Tor circuit, because they live in the
application's query pattern. That's what this crate is for. (Transport
anonymity is still worth having — it's the other half of the story, and the
half [Nym's mixnet](https://nym.com) exists for. This crate works the same
over any transport.)

## What it does

**Quantized sync ranges** (`grid`, `sync`). Instead of requesting exactly
`[resume, tip]`, the wallet requests a range widened to a network-wide grid:
start rounded down, end rounded up, grid spacing scaled to the range and
never below one day of blocks. Every wallet resuming anywhere in the same
grid cell emits **identical boundaries** — anonymity by collision, not by
noise. The range is then fetched as randomly sized, overlapping, shuffled
chunks, so no single request describes anyone's actual interval. The reorg
check (the few blocks below the resume point that wallets re-fetch and
compare) rides *inside* the widened range — a separate ten-block request
just below your resume point would have named it exactly.

**Decoupled broadcasts** (`broadcast`). Sends are delayed by an exponential
draw with a mean of 144 blocks (~3 hours), capped at 576 (~12 hours) — the
same distribution [ZIP 318](https://zips.z.cash/zip-0318) uses for transfer
scheduling, so wallet sends pool with migration traffic. Delays run longer
than a phone keeps a process alive, so the schedule is **plain data you
persist**: sample once, save, restart as many times as the OS likes, resume
the remainder. The transaction is *built* only when the delay fires, so its
expiry (visible on-chain, [ZIP 203](https://zips.z.cash/zip-0203)) derives
from a fresh tip instead of leaking your last sync height. And you usually
don't need to sync first: anchors stay valid for about two days —
`needs_refresh_sync` tells you when you actually do.

## Using it

You implement two traits — deliberately two, because a session should sync
*or* broadcast, never both, and ideally against different servers:

```rust
use nym_swizzle_zcash::{BlockSource, QueuedRange, SyncSession, SyncOutcome};

struct MyClient { /* your gRPC / proxied / tunnelled lightwalletd client */ }

impl BlockSource for MyClient {
    type Block = MyCompactBlock;
    type Error = MyError;
    async fn block_range(&mut self, start: u64, end: u64)
        -> Result<Vec<(u64, MyCompactBlock)>, MyError>
    {
        // one wire request for [start, end); the crate chooses the ranges
    }
}

// the routine catch-up, from your resume point to the tip
let outcome = SyncSession::new()
    .fetch(
        &mut my_client,
        &QueuedRange::catch_up(resume_point, tip),
        tip,
        |height, block, disposition| {
            // disposition says what to do: discard cover, scan the rest;
            // buffer results — commit only on SyncOutcome::Committed
        },
        |height, block| my_db.stored_hash(height) == block.hash(), // reorg check
    )
    .await?;

if outcome == SyncOutcome::ReorgDetected {
    // rewind and requeue, exactly as your SDK does today
}
```

And the send path, in two phases so it survives restarts:

```rust
use nym_swizzle_zcash::{Scheduler, BroadcastPlan, TxBroadcaster, expiry_height};

// phase 1: at "user pressed send" — sample ONCE, persist, forget
let plan = Scheduler::standard().schedule();
my_db.save(("plan", plan.delay_secs, plan.profile), ("scheduled_at", now));

// phase 2: at every wallet startup (and right after phase 1)
let plan = my_db.load_plan();
plan.resume(now - scheduled_at, &mut my_broadcaster, || async {
    let tip = fetch_fresh_tip().await?;          // AFTER the delay, on purpose
    build_tx(spend_request, expiry_height(tip))  // expiry = fresh tip + 40
}).await?;
```

Enable the `serde` cargo feature if you'd rather derive
`Serialize`/`Deserialize` on `BroadcastPlan` than store two integers by
hand.

## What it costs — honestly

Privacy here is bought with bandwidth and patience, and the bill is real:

- **Sync:** a wallet one day behind downloads roughly 2–3× the compact
  blocks it strictly needs (grid cover plus chunk overlap). Compact blocks
  are small; a session's cover tops out around half a megabyte. Cover blocks
  arrive tagged, so discarding them is one branch in your scan loop.
- **Send:** the default profile delays broadcasts by ~3 hours on average,
  up to 12. That's the price of pooling with roughly a hundred comparable
  transactions instead of standing alone. There's a `fast()` profile
  (~30 min mean) when users need it — with a smaller crowd to hide in, and
  the docs say so.

One thing we'd rather flag than have you find out: the tuning of the *chunk*
sizes and overlaps inherits `nym-swizzle`'s stance — conservative defaults,
exposed knobs, **no validated anonymity numbers**. The grid constants and
broadcast parameters, by contrast, follow ZIP 318's published design. And
keep the defaults: a custom grid floor or delay distribution makes your
wallet recognisably different, which is the opposite of hiding.

## Don't take our word for it

Everything above is checkable on your own machine, against a server we don't
run (pick any lightwalletd with `ZEC_SERVER=`):

```sh
# watch the wire: real sync against a public lightwalletd, grid-aligned
# boundaries printed per request, plus the persist/restore broadcast flow
cargo run --release -p nym-swizzle-zcash --example wallet_sync

# the live suite: reorg detection on real chain data (matching hashes
# commit, a corrupted hash trips ReorgDetected), grid alignment of every
# emitted boundary, and measured overhead for both sync regimes
cargo test -p nym-swizzle-zcash -- --ignored

# the pure logic, no network: quantization arithmetic, coverage and
# reproducibility invariants, delay distribution bounds
cargo test -p nym-swizzle-zcash
```

Or skip our tooling entirely: point your wallet's existing client at the
`SyncSession` driver and log what `block_range` gets asked for — every union
of requests should start on a multiple of the printed grid spacing. The
grid, delay, and expiry constants all cite their public sources
([ZIP 318](https://zips.z.cash/zip-0318),
[ZIP 203](https://zips.z.cash/zip-0203),
[librustzcash](https://github.com/zcash/librustzcash),
[lightwalletd](https://github.com/zcash/lightwalletd)) in the rustdoc, so
you can check the arithmetic against the specs rather than against us.

## Scope, and what's deliberately left out

- **No transport.** The crate never opens a connection. The example's
  hand-rolled gRPC client (`examples/support/lightwalletd.rs`, ~150 lines,
  no `protoc` needed) shows one way to fill the slot; yours is probably
  better.
- **No librustzcash dependency.** The types are bare heights and two small
  enums, so the crate drops into any wallet stack without dragging a second
  wallet library along.
- **No real sends in the example.** Broadcasting for real needs a funded
  seed; the example mocks the broadcaster and says so loudly. Everything
  else — blocks, hashes, tips — is live data.
- **Wasm-clean.** `cargo check -p nym-swizzle-zcash --target
  wasm32-unknown-unknown` passes (CI enforces it); the gRPC stack lives in
  dev-dependencies only.

If you're building a wallet and something here doesn't fit how your sync
pipeline actually works, we'd genuinely like to hear about it — the slot
design came from asking wallet developers the same question.

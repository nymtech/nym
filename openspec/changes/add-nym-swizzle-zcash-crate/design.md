# Design: add-nym-swizzle-zcash-crate

## Context

`nym-swizzle` (`sdk/rust/nym-swizzle`) is a chain-agnostic crate: randomized delay scheduling (`Delay`), overlapping randomized range chunking with start-edge obfuscation (`Range`/`ChunkPlan`/`Snap`), seeded/injectable RNG (`rng`). It deliberately refuses to own chain policy: it never extends a range's *end*, has no notion of a chain tip, and its doc example delays are illustrative rather than calibrated.

The external review "Baseline hygiene for Zcash light clients: proposal for nym-swizzle" (C. Diaz, draft v0.2, 2026-07-27) specifies the Zcash policy layer:

- **Mechanism 1 — quantized sync ranges.** For every queued range `[a, b]`: choose spacing `S = max(1152, smallest 144·2^j >= b − a)`; emit `a' = S·floor(a/S)`, `b' = min(S·ceil(b/S), tip)`. Cover blocks below `a` are discarded without re-scanning; cover blocks above `b` are deduped and scanned. `Verify`-tagged ranges emit no separate request — instead enforce `a − a' >= VERIFY_LOOKAHEAD (10)` (widening by one grid cell when violated), run the hash comparison when the verify-window blocks arrive (chunks are shuffled, so not necessarily first), and commit scan results only after it passes.
- **Mechanism 2 — decoupled broadcast scheduling.** Delay ~ exponential, mean 144 blocks, rejection-resampled above 576 (standard); optional fast profile mean 24 blocks. A session syncs or broadcasts, never both. Broadcasting does not require being at the tip (old anchors are accepted; ZIP 318 grid anchors are retained ~2 days) — only schedule a refresh sync when the last sync is older than the anchor bound. Build the transaction *after* the delay elapses with expiry = fresh tip + 40; never derive expiry from a stale tip.

Currently `sdk/rust/nym-swizzle-zcash/` holds a `publish = false` example binary (`nym-swizzle-zcash-example`) that benchmarks direct vs. swizzled fetching against a public lightwalletd, with a minimal hand-rolled `prost`/`tonic` client (no `build.rs`, no `protoc`). It implements neither mechanism.

Constraints from stakeholders:

- The crate is intended for upstreaming into librustzcash, so it must not depend on librustzcash (circular) and must be near-dependency-free in general.
- `nym-swizzle` guarantees every non-dev dependency compiles for `wasm32-unknown-unknown`; the new library must preserve that.
- Broadcast delays run 3–12 h; mobile wallet processes will not survive that, so schedules must be persistable.
- Documentation is aimed at Zcash wallet developers: standalone, no internal-process references, conversational, upbeat about Nym without overpromising, with concrete self-verification hooks for a skeptical audience.

## Goals / Non-Goals

**Goals:**

- A library crate `nym-swizzle-zcash` at `sdk/rust/nym-swizzle-zcash/` implementing Mechanisms 1 and 2 as a thin policy layer composing `nym-swizzle`.
- Trait slots for the wallet author's transport (`BlockSource` for sync, `TxBroadcaster` for send) — the crate performs no I/O itself.
- Persistable, restart-safe broadcast schedules (plain-old-data; delay sampled exactly once).
- A live example against a real public lightwalletd (blocks real, send mocked) demonstrating quantized sync and broadcast save-and-resume.
- Opt-in integration tests on real chain data: verify-window/reorg rule, grid alignment of emitted ranges, and overhead measurement for the daily-sync and long-catch-up regimes.
- Preserve the wasm32 guarantee for the library's non-dev dependency graph.

**Non-Goals:**

- No librustzcash integration (no `ScanRange` conversions, not even feature-gated) — the crate defines its own minimal range/tag types.
- No real transaction building or broadcasting in examples/tests (requires a funded seed); the broadcast path is exercised with a mock sender.
- No transport implementation in the library (no tonic/TLS in non-dev deps); no mixnet integration — transport anonymity is a separate layer.
- No changes to `nym-swizzle`'s API or requirements.
- No scan-queue management: the wallet's SDK (e.g. librustzcash) owns which ranges are queued; this crate transforms and executes them.

## Decisions

### D1. Two separate transport traits, not one

`BlockSource` (sync) and `TxBroadcaster` (send) are distinct traits, and the sync driver and broadcast scheduler each accept only their own. The review's rule "a session syncs or broadcasts, never both; prefer different servers" becomes structural: the API never hands the caller a combined object. A wallet *can* implement both traits on one struct; the docs warn against sharing a connection, but the type split is the primary nudge.

*Alternative considered:* one `Lightwalletd` trait mirroring the full RPC surface — rejected because it invites exactly the session-sharing the review forbids, and forces mock implementers to stub methods they never use.

### D2. `BlockSource` yields `(height, block)` pairs; the block type is generic

The crate's logic needs only heights (classification, verify-window bookkeeping, coverage); block contents are opaque `B` passed through to the wallet's callback. This keeps the crate free of compact-block message definitions and lets authors use their existing types (librustzcash `CompactBlock`, hand-rolled prost structs, anything). The trait is minimal:

```rust
trait BlockSource {
    type Block;
    type Error;
    /// Fetch the half-open height range [start, end). Heights must accompany blocks.
    async fn block_range(&mut self, start: u64, end: u64)
        -> Result<Vec<(u64, Self::Block)>, Self::Error>;
}
```

(Concretely as an `async fn` in trait / return-position-impl-Trait per the crate's MSRV; if the workspace MSRV predates stable AFIT, a boxed-future signature is used instead — same shape.) `Vec` rather than a `Stream` keeps the trait object-safe-adjacent and trivial to implement; requests are bounded (at most `S_FLOOR = 1152` compact blocks each), so buffering one is fine.

*Alternative considered:* requiring a `Height` trait on `B` — more ceremony for implementers with no gain; the pair is simpler.

### D3. Quantization is a pure, separately exposed module (`grid`)

`grid::quantize(range, tip) -> Quantized` implements ladder selection, rounding, tip capping, and the verify-lookahead widening as pure functions on `u64`s, with the review's constants as named public consts (`SHARD = 144`, `S_FLOOR = 1152`, `VERIFY_LOOKAHEAD = 10`). `grid::classify(height, &Quantized) -> Disposition { CoverBelow, VerifyWindow, Requested, CoverAbove }` classifies delivered heights. Pure functions make the review's arithmetic directly unit-testable against worked examples, and wallet authors who want only the math (not the driver) can use it alone. The uniform rule "steps 1–4 apply to every queued range regardless of tag" means the public input is just `RangeKind::{Scan, Verify}` — `Verify` is the only tag with special behavior (no separate request; lookahead enforcement), `FoundNote` needs nothing beyond step 3, which is unconditional here.

Two pins added per reviewer follow-up (2026-08-06), because the rule is network-wide by construction and two divergent-but-correct implementations would partition the collision sets: (a) the range convention is **half-open**, with ladder selection by half-open length, boundary-tested at rung multiples (a 1152-block range takes `S = 1152`, 1153 takes `S = 2304` — so a wallet exactly 1152 behind the tip takes 2304, where an inclusive reading would give 1152); (b) `S_FLOOR` is asserted at compile time to sit on the `SHARD * 2^j` ladder — divisibility by the shard alone would admit a floor like `3 * SHARD` that silently stops nesting.

*Alternative considered:* hiding quantization inside the sync driver — rejected; the pure layer is the most reusable and most verifiable part.

### D4. Sync driver puts the emitted range on the wire deterministically, and withholds commit until the verify window passes

*(Revised per reviewer follow-up, 2026-08-06. The original design executed the emitted range as `nym_swizzle::Range` overlapping shuffled chunks; that was dropped: the point of quantization is that every wallet resuming in the same cell says exactly the same thing, so randomization within that collision set buys nothing against the lightwalletd adversary, the overlap costs real bandwidth — measured ~1.3x on top of quantization's ~2x — and per-wallet variation in chunk sizes/order is itself a distinguishing dimension. As a side effect the sync module no longer uses `nym-swizzle` at all; the dependency remains for `Delay` in the broadcast scheduler.)*

`sync::fetch(source, queued: QueuedRange, tip, sink, check_hash)`:

1. Quantize `[a, b)` → `[a', b')` (with verify widening).
2. Put `[a', b')` on the wire as `Quantized::requests()`: split at network-uniform `S_FLOOR`-aligned boundaries, ascending, disjoint, gapless — one request when the emitted range is a single cell; the split exists for retry practicality on long catch-ups. No random sizes, no overlap, no shuffle, no seeds.
3. Drive `source.block_range` per request (sequential `fetch`, or `fetch_concurrent` with bounded in-flight clones — concurrency changes completion order, never what goes on the wire).
4. Deliver each block to the wallet's sink with its `Disposition`. Verify-window blocks are handed to a wallet callback that compares hashes against stored state and answers match/mismatch — the crate cannot see the wallet's DB.
5. The call resolves to `SyncOutcome::Committed` only after every request has landed *and* the verify check passed; a mismatch resolves to `SyncOutcome::ReorgDetected` (wallet rewinds and requeues, exactly as its SDK does today).

Requests are disjoint, so nothing is fetched twice; `Disposition` tells the wallet which rule to apply to cover (`CoverBelow` ⇒ discard without scanning; `CoverAbove` ⇒ dedupe against scan state, scan what's new).

### D5. Broadcast schedule is plain-old-data, sampled once, resumable

Two-phase API:

```rust
let plan: BroadcastPlan = Scheduler::standard().schedule();  // samples delay ONCE
// BroadcastPlan { delay_secs: u64, elapsed_secs: u64, profile } — public primitive fields
plan.resume(broadcaster, build_tx).await?;                   // waits the remainder, builds, sends
```

- Sampling happens exactly once, at `schedule()`. Resampling on restart would bias toward short delays for frequently-restarted wallets. On resume the wallet passes how much wall-clock has already elapsed (it persisted the schedule moment alongside the plan — the crate cannot read clocks portably on wasm and must not trust a stale in-struct timestamp); `resume` sleeps only the remainder.
- `build_tx` is a closure invoked *after* the delay elapses, receiving a context with the fresh tip (fetched by the wallet in the closure or supplied via a slot) so expiry = fresh tip + 40. The crate documents, and the example demonstrates, never deriving expiry from a pre-delay tip.
- Profiles: `Scheduler::standard()` (exponential mean `blocks(144)`, rejection-resampled above `blocks(576)` — via `nym_swizzle::Delay::poisson(..).max(..)`, which already rejection-resamples rather than clamping) and `Scheduler::fast()` (mean `blocks(24)`, cap `blocks(96)`), with the anonymity trade-off documented.
- The anchor-age decision is a pure helper: `needs_refresh_sync(last_synced_height, tip) -> bool` against the ZIP 318 anchor-retention bound, so wallets know whether to broadcast directly or schedule a refresh sync first (on its own session).
- Serialization: public primitive fields make the struct trivially persistable by hand; a `serde` cargo feature (off by default) adds `Serialize`/`Deserialize` derives. Keeps the mandatory dependency graph at `nym-swizzle` + `futures`.

*Alternative considered:* a single long-lived `send().await` future (the `Delay::run` shape) — rejected: 3–12 h delays do not survive mobile process lifetimes.

### D6. Block-time conversion lives in this crate, not `nym-swizzle`

`pub const TARGET_BLOCK_TIME: Duration = Duration::from_secs(75)` (post-Blossom) and `pub const fn blocks(n: u64) -> Duration`. The conversion is one multiplication — a generic helper in `nym-swizzle` would rename arithmetic while breaking its chain-agnostic stance; the *named constant* is the value, and it is Zcash policy. Usage reads like the review: `Delay::poisson(blocks(144)).max(blocks(576))`.

### D7. Library stays wasm-clean; gRPC lives in dev-dependencies

Non-dev deps: `nym-swizzle`, `futures` (and `serde` only behind the optional feature). The existing hand-rolled prost/tonic lightwalletd client moves to the example (`examples/` support module), shared with the integration tests via `#[path]` include, so neither `build.rs` nor `protoc` is introduced. CI's wasm32 check (added for `nym-swizzle` in c25ec1a5d-era commits) is extended to `cargo check -p nym-swizzle-zcash --target wasm32-unknown-unknown`.

### D8. Live tests are opt-in and gentle

Integration tests hitting `zec.rocks` are `#[ignore]` by default (run with `cargo test -p nym-swizzle-zcash -- --ignored`, server overridable via `ZEC_SERVER`). CI stays deterministic; a skeptical wallet developer gets a one-command way to watch real emitted ranges land on grid boundaries. Tests keep block counts modest, following the current example's etiquette. The overhead-measurement test reports the two regimes the review names (daily incremental, long catch-up) against its <2× bound.

### D9. Documentation contract

README and example rustdoc are written for Zcash wallet developers as the sole audience: no references to OpenSpec, Confluence, PR numbers, or any internal process artifact; conversational, positive about Nym, explicitly not overpromising (the tuning-caveat stance carries over from `nym-swizzle`); every major claim paired with a way to verify it locally (run the invariants/integration tests, check grid alignment of logged requests, inspect the wire). The threat model is explained in wallet terms ("what your lightwalletd can learn about you") with the two leaks from the review as the framing.

## Risks / Trade-offs

- [Public server dependency in example/tests] → opt-in (`#[ignore]`), modest ranges, `ZEC_SERVER` override, clear failure messages; CI never runs them by default.
- [Clock handling across restarts is subtle (wasm has no reliable monotonic-across-restart clock)] → the crate never reads wall clocks for resume; the wallet supplies elapsed time, and the docs show the correct persist-schedule-moment pattern. Worst case (wallet lies about elapsed) degrades that wallet's own anonymity only.
- [`Vec`-per-request buffering in `BlockSource`] → requests are bounded at `S_FLOOR` (1152) compact blocks; documented. Streaming can be added later without breaking the trait (new method with default impl or a v2 trait).
- [Constants may drift from the review as ZIP 318 evolves] → all constants are named, documented with their derivation, and centralized in `grid`/`broadcast`; changing them is a one-line diff with tests asserting the documented values.
- [Verify-window hash check depends on wallet cooperation] → the driver structurally withholds `Committed` until the callback answers; a wallet that answers dishonestly only harms itself. Documented.
- [Crate rename breaks anyone referencing `nym-swizzle-zcash-example`] → it was `publish = false` and referenced only by CI/docs in-repo; those references are updated in this change.

## Migration Plan

1. Replace the binary crate in place (same directory, new package name `nym-swizzle-zcash`, `publish = true`); delete `src/main.rs`, move the lightwalletd client under `examples/`.
2. Update workspace/CI references (`-p nym-swizzle-zcash-example` → `-p nym-swizzle-zcash`, add wasm check).
3. No data or deployment migration — nothing depended on the example binary.

Rollback: revert the directory to the example crate; no external consumers exist yet.

## Open Questions

None blocking. (Resolved during exploration: serializable schedules — yes, POD + optional serde; verify rule tested on real data — yes, opt-in; blocks→duration helper — this crate, not `nym-swizzle`; librustzcash coupling — none, upstreaming goal forbids it.)

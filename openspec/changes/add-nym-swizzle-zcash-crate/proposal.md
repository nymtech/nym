# Proposal: add-nym-swizzle-zcash-crate

## Why

`nym-swizzle` provides chain-agnostic traffic-shape obfuscation primitives, but a Zcash wallet developer still has to derive all the Zcash-specific policy themselves: the quantized sync-range grid, the reorg-verify lookahead rule, ZIP 318-aligned broadcast delays, and fresh-tip expiry. An external privacy review ("Baseline hygiene for Zcash light clients", C. Diaz, 2026-07-27) specified exactly this policy layer; today the repository only ships a benchmark-style example binary (`nym-swizzle-zcash-example`) that demonstrates chunking but implements none of it. Wallet developers need a library they can depend on directly.

## What Changes

- **BREAKING (crate removal/replacement):** the example binary crate `nym-swizzle-zcash-example` at `sdk/rust/nym-swizzle-zcash/` is replaced by a library crate named `nym-swizzle-zcash` in the same location.
- New library implements the two mechanisms from the baseline-hygiene review as a thin, dependency-light policy layer over `nym-swizzle`:
  - **Quantized sync ranges**: grid ladder `S_j = 144·2^j` with floor 1152, start rounded down / end rounded up to the grid, capped at the public tip; the `Verify` lookahead rule (`a − a' >= 10`, else widen by a full grid cell); classification of delivered blocks as cover (discard), verify-window (hash-check before commit), or new (scan).
  - **Decoupled broadcast scheduling**: exponential delay with mean 144 blocks rejection-resampled above 576 (standard profile) or mean 24 blocks (fast profile); delays denominated in blocks with a named `TARGET_BLOCK_TIME` constant (75 s); transaction building deferred to fire time so expiry derives from a fresh tip; schedules are plain-old-data and persistable, surviving process restarts without resampling.
- Transport stays the wallet author's job via trait slots (`nym-swizzle` convention): a `BlockSource` trait for fetching compact blocks and a separate `TxBroadcaster` trait for sending, so sync and broadcast can never share a session object by construction.
- The library keeps `nym-swizzle`'s guarantee that every non-dev dependency compiles for `wasm32-unknown-unknown` (dependencies: `nym-swizzle`, `futures` only). The gRPC stack moves to dev-dependencies.
- A runnable example connects to a real public lightwalletd (`zec.rocks`) to fetch blocks through the quantized sync path (deterministic, grid-aligned requests), and demonstrates broadcast save-and-resume with a mocked sender (no wallet seed or funds required).
- Opt-in integration tests against real Zcash chain data cover the verify-window/reorg rule and measure quantization overhead for the daily-sync and long-catch-up regimes.
- README and examples are written for Zcash wallet developers: standalone (no references to internal process artifacts), conversational and positive about Nym without overpromising, and offering concrete verification hooks (runnable tests, grid-alignment checks) rather than claims.

## Capabilities

### New Capabilities

- `nym-swizzle-zcash`: Zcash-specific privacy policy layer over `nym-swizzle` — quantized sync-range emission with verify-window handling and cover-block classification, persistable decoupled broadcast scheduling, block-denominated delay helpers, and transport trait slots for wallet authors.

### Modified Capabilities

<!-- none: nym-swizzle's requirements are unchanged; this crate composes its public API -->

## Impact

- `sdk/rust/nym-swizzle-zcash/`: example binary replaced by a library crate (`src/main.rs` removed; `src/lib.rs`, modules, `examples/`, `tests/` added); package name changes from `nym-swizzle-zcash-example` to `nym-swizzle-zcash`; `publish` flips to true.
- Root workspace `Cargo.toml`: member path unchanged; no new workspace dependencies for the library (dev-deps: `tonic`, `tonic-prost`, `prost`, `tokio`, `serde`/`serde_json` for the persistence example if the `serde` feature is exercised).
- CI: the wasm32 check added for `nym-swizzle` should also cover `nym-swizzle-zcash` (library only; examples/tests are native).
- No changes to `nym-swizzle` itself.
- Intended future home: upstreaming into librustzcash — hence zero dependency on librustzcash types (avoids a circular dependency) and no mandatory serde.

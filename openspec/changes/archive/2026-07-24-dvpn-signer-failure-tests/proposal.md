# Proposal: dvpn-signer-failure-tests

## Why

The mainnet nym-api endpoint serving aggregated expiration-date signatures hangs
indefinitely (verified empirically: `aggregated-expiration-date-signatures`
never responds while sibling ecash endpoints answer in <0.3s), because the
distributed ecash signers behind it are offline and are not expected to return.
The SDK's provisioning path has no request timeout, so `ensure_ticketbooks`
blocks forever inside the bandwidth controller's `store_ticketbook`, the freshly
issued (and paid-for, in NYM) ticketbook is never persisted, and every retry
re-deposits real funds. Signer flakiness is a permanent property of the
distributed deployment — the SDK must tolerate it, and that tolerance must be
provable in CI without mainnet access (sandbox has no flaky signers, so the
failure mode is otherwise invisible until it burns funds on mainnet).

## What Changes

- Add a `TimeoutFetcher` decorator in `nym-sdk-session` implementing
  `CredentialPublicDataFetcher` + `CredentialFetcher` over the existing
  `NyxdCredentialFetcher`: the three read-only public-data fetches
  (master verification key, coin-index signatures, expiration-date signatures)
  are bounded by a per-call timeout; `fetch_ticketbooks` (deposit + issuance)
  is deliberately NOT timed (a chain deposit must never be aborted mid-flight).
- Wire the decorator into `Session::spawn_controller` so the controller's
  public-data fetches fail fast instead of hanging. Because
  `store_ticketbook` in `nym-bandwidth-controller` is best-effort per step, a
  fast failure means the ticketbook IS persisted and readiness resolves — no
  hang, no lost funds, automatic recovery when signers return.
- Add an outer provisioning timeout to `Session::ensure_ticketbooks` (a
  `tokio::time::timeout` arm alongside the existing cancellation race) mapping
  to a distinct, clearly worded `SessionError` — defense in depth against any
  other stall in the provisioning path.
- Tier 1 tests: deterministic unit tests for the decorator using
  `#[tokio::test(start_paused = true)]` + virtual-clock advance — a hanging
  inner fetcher (`std::future::pending()`) must yield a bounded error; slow
  under/over-threshold and plain-error modes covered.
- Tier 2 tests: end-to-end recovery test in `nym-sdk-session/tests/` driving
  the real `BandwidthController` + `initialise_ephemeral_storage()` with a
  parameterizable mock fetcher (modes: hang / slow / error / partial), plus a
  shared test helper that fabricates a real `IssuedTicketBook` via the
  compact-ecash primitives. Asserts: without the decorator the controller
  wedges and stores nothing (reproduces the mainnet bug); with it the
  ticketbook persists and `wait_for_ticketbooks` resolves; a second
  provisioning run against the same storage makes zero additional
  `fetch_ticketbooks` calls (no re-deposit — the money-safety property).
- No changes to `common/` production code: all fixes live in
  `sdk/rust/nym-sdk-session`; tests exercise `common/` crates through their
  public APIs only.

## Capabilities

### New Capabilities

_None — this hardens an existing capability._

### Modified Capabilities

- `dvpn-session`: adds a signer-failure-tolerance requirement to ticketbook
  issuance — public-data fetches are time-bounded, an issued ticketbook is
  persisted even when global signing data cannot be fetched, provisioning
  surfaces a distinct timeout error instead of hanging, and a persisted
  ticketbook is never re-purchased on retry.

## Impact

- **Code**: `sdk/rust/nym-sdk-session/src/session.rs` (decorator, wiring,
  outer timeout, new `SessionError` variant); new
  `sdk/rust/nym-sdk-session/tests/` integration tests + test-support module.
- **Dependencies**: `nym-sdk-session` gains `async-trait` (already a
  workspace dep) and dev-deps on `nym-credential-storage` (ephemeral store)
  and the compact-ecash crate for the ticketbook fixture.
- **Behavior**: provisioning that previously hung indefinitely on dead
  signers now either succeeds (ticketbook persisted, signatures back-filled
  later) or fails fast with a clear error; retries stop re-depositing NYM.
- **Not affected**: `common/bandwidth-controller`, `common/bandwidth-fetcher`,
  `common/client-libs/validator-client` production code; `smol-dvpn`
  datapath; example CLIs (they benefit transitively).

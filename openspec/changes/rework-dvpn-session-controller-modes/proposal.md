## Why

Review of https://github.com/nymtech/nym/pull/7119 found that `nym-sdk-session` uses
the bandwidth controller the wrong way round. It always runs the controller event loop
and then simulates "provision once, no background deposits" by installing a credential
fetcher, waiting for readiness, and uninstalling the fetcher again. That inverted
lifecycle is what forced the fresh-fetcher-per-install builder, the installed flag
under a mutex, and the retry-by-reinstall path — machinery that exists only to work
around a mode the controller already supports directly. The controller offers two
intended patterns: a running controller with the fetcher installed at construction
(automatic top-up), and a non-running controller whose inline `fetch_ticketbook`
issues on demand and is then torn down (one-shot). The session must use those as
designed, with **no change to `common/bandwidth-controller`** or any other `common/`
crate.

## What Changes

- **Automatic top-up mode** (`SessionConfig::automatic_topups: Some(policy)`): the
  credential fetcher is installed at controller construction and the controller loop
  runs for the session's lifetime. The controller's first sweep tick provisions the
  managed WireGuard types at startup, and the sweep restocks per policy thereafter.
  `ensure_ticketbooks` becomes a readiness wait (`wait_for_ticketbooks`), with a single
  explicit restock-then-wait retry when nothing is stocked or in flight. No fetcher
  install/uninstall at provisioning time.
- **One-shot mode** (`automatic_topups: None`, the default): nothing runs in the
  background. Ticket spending goes through a non-running controller that has **no
  credential fetcher** (only a read-only public-data fetcher for missing signing data),
  so it cannot deposit. `ensure_ticketbooks` spins up a short-lived, non-running
  controller with a fresh fetcher, reads the stock, calls the controller's inline
  `fetch_ticketbook` for each required type the controller itself judges low or
  expiring, verifies the types are now usable, and tears the controller down (drops it
  and cleans up its fetcher). The provisioning controller shares the session's store
  handle.
- **Removed** from `nym-sdk-session`: the `FetcherBuilder` / `NyxdFetcherBuilder`
  abstraction, `OwnedController` with its installed flag, the install/uninstall
  provisioning cycle, and the retry-by-reinstall path, together with their five tests.
- **Kept**: `TimeoutFetcher` (per-call bounds on the read-only signing-data fetches in
  both modes), topology-scoped ticket types (entry only for single-hop, entry + exit for
  two-hop), the `TopologyMismatch` guard, the overall provisioning timeout, and
  cancel-raced serialisation of concurrent `ensure_ticketbooks` calls.
- Public `Session` API (`new`, `ensure_ticketbooks`, `bandwidth_provider`,
  `obtain_wireguard_credential`, `shutdown`, register calls) is unchanged, so the
  `smoldvpn` examples and tests need no code change.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `dvpn-session`: the ticketbook-issuance requirement is respecified as two controller
  modes (running-with-fetcher for automatic top-up; non-running inline issuance for
  one-shot) instead of the fetcher install/uninstall lifecycle, and the opt-in
  automatic-restock requirement is restated so that "no background deposit" in the
  default mode follows from there being no running controller and no credential
  fetcher on the spending path, not from uninstalling a fetcher.
- `bandwidth-type-scoping`: the "one-off restock via the fetcher lifecycle" guidance is
  replaced with the non-running-controller inline-fetch pattern; the managed-set
  semantics of the controller itself are unchanged.

## Impact

- Code: `sdk/rust/nym-sdk-session/src/session.rs` (controller wiring per mode,
  `ensure_ticket_types`, `shutdown`), `sdk/rust/nym-sdk-session/src/fetcher.rs`
  (`TimeoutFetcher` becomes a cloneable shared handle so the fetcher handed to a
  controller can still be cleaned up), `sdk/rust/nym-sdk-session/src/session_tests.rs`
  (replace the lifecycle tests), `sdk/rust/nym-sdk-session/src/lib.rs` (module docs).
- No change to `common/bandwidth-controller`, `common/bandwidth-fetcher`, or
  `common/credential-storage`. Every controller API used (`fetch_ticketbook`,
  `with_credential_fetcher`, `with_credential_public_data_fetcher`, `run`,
  `wait_for_ticketbooks`, `restock_ticketbooks`, the `BandwidthTicketProvider` impl on
  `BandwidthController`, `AvailableTicketbooks::needs_restock` /
  `contains_minimal_tickets`) is already public.
- `smoldvpn`: examples and `tests/live_bringup.rs` keep working unchanged; the
  `smoldvpn-topup` example exercises the one-shot path end to end.
- Specs: `openspec/specs/dvpn-session/spec.md`,
  `openspec/specs/bandwidth-type-scoping/spec.md`.

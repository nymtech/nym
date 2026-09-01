## Why

A default `nym-sdk-session` (no automatic top-ups) cannot provision its WireGuard
ticketbooks: `ensure_ticketbooks` fails immediately with `TicketbooksUnavailable`
("some required ticketbooks are unavailable and none are being fetched"), so every
dVPN bring-up through the default session — including the smoldvpn `live_bringup`
gated integration tests — dies before the first gateway handshake. The session
expresses "provision once, never deposit in the background" with an empty
`managed_ticket_types` set plus an explicit `restock_ticketbooks` call, but the
controller only acts on explicit restock for *managed* types, so with an empty set
the restock is silently skipped and the following `wait_for_ticketbooks` sees the
type as neither stocked nor in flight.

## What Changes

- Adopt the reference `nym-vpn-client` provisioning pattern in `nym-sdk-session`,
  entirely in the crate's *usage* of the bandwidth controller — **no change to
  `nym-bandwidth-controller`**:
  - Scope `managed_ticket_types` to the WireGuard types in **all** modes (drop the
    empty-set default arm).
  - Drive once-off issuance by the fetcher lifecycle: install the credential
    fetcher (which triggers a restock of the managed types), `wait_for_ticketbooks`
    for readiness, then **unset the fetcher** so no background deposits occur.
  - In automatic-top-up mode, leave the fetcher installed so the controller's sweep
    restocks per policy.
- Control "no background restock" via the fetcher's *presence*, not via an empty
  managed set.
- Correct the `bandwidth-type-scoping` spec: an empty managed set does not (and the
  blessed usage does not rely on it to) make explicit `restock_ticketbooks` fetch —
  the managed set scopes the proactive paths, and background restock is governed by
  whether a fetcher is installed.
- Fix the two now-inaccurate comments in `session.rs` describing the empty-set /
  explicit-restock approach.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `dvpn-session`: the once-off ticketbook provisioning path and the opt-in
  automatic-restock behaviour are respecified in terms of the fetcher lifecycle
  (install → await issuance → unset), with `managed_ticket_types` scoped to the
  WireGuard types in every mode.
- `bandwidth-type-scoping`: remove the incorrect guarantee that an empty managed set
  still serves explicit `restock_ticketbooks` requests; clarify that the managed set
  scopes only the proactive restock paths and that fetcher presence governs
  background restock.

## Impact

- Code: `sdk/rust/nym-sdk-session/src/session.rs` — controller config
  (`managed_ticket_types`), fetcher wiring (install-on-provision / unset), and
  `ensure_ticket_types` (replace the `restock_ticketbooks` trigger with the fetcher
  lifecycle); update the two stale comments.
- No change to `common/bandwidth-controller` (controller and its `restock_ticketbooks`
  request are untouched).
- Tests: the smoldvpn `smoldvpn/tests/live_bringup.rs` gated tests should pass
  against sandbox; add a deterministic `nym-sdk-session` test proving a default-mode
  session provisions on demand and makes no background deposit after provisioning.
- Docs/specs: `openspec/specs/dvpn-session/spec.md`,
  `openspec/specs/bandwidth-type-scoping/spec.md`.

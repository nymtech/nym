## Context

`nym-sdk-session` (the dVPN session/provisioning layer the smoldvpn integration
tests drive) provisions WireGuard ticketbooks through an owned
`nym-bandwidth-controller`. Today it builds the controller like this:

- `managed_ticket_types` is set to the WireGuard types only when
  `automatic_topups` is `Some(policy)`; in the default (`None`) case it is
  `Vec::new()` (`session.rs:254`).
- The credential fetcher is installed once at construction, via the builder
  (`BandwidthController::new(..).with_config(..).with_credential_fetcher(fetcher)`),
  which sets the field but does not itself trigger a restock.
- `ensure_ticketbooks` → `ensure_ticket_types` calls `restock_ticketbooks(types)`
  then `wait_for_ticketbooks(types)` (`session.rs:301`).

The controller only ever restocks *managed* types: `check_and_restock` filters every
requested type through `managed_ticket_types.contains()` (`controller.rs:486`), and
the explicit `RestockTicketbooks` request is handled by that same path
(`controller.rs:234`). So in the default configuration the managed set is empty, the
explicit restock is skipped for every type, nothing is put in flight, and the
immediately-following `wait_for_ticketbooks` resolves the type as `Unavailable` and
returns `TicketbooksUnavailable` — surfaced to the caller as
`Issuance("some required ticketbooks are unavailable and none are being fetched")`.
This fails fast (~0.5s, no network) and deterministically for both
`smoldvpn/tests/live_bringup.rs` gated tests.

The reference `nym-vpn-client` (per Simon Wicky) does a one-off restock with the
**fetcher lifecycle**: create/install the fetcher, await ticket issuance, then unset
the fetcher; leaving the fetcher installed is what enables continuous background
issuance. Installing a fetcher triggers `handle_set_credential_fetcher`, which runs
`check_and_restock(managed_ticket_types)` (`controller.rs:258`) — so the install is
the restock trigger — and the sender already exposes `set_credential_fetcher` and
`unset_credential_fetcher` (`requests/sender.rs:84`, `:101`). This pattern works
against the controller exactly as written; the fix is entirely in the session's usage.

## Goals / Non-Goals

**Goals:**
- Make default-mode `ensure_ticketbooks` provision successfully (the smoldvpn gated
  tests bring up single- and two-hop tunnels).
- Adopt the reference fetcher-lifecycle pattern in `nym-sdk-session`.
- Keep the "default session never deposits in the background" guarantee, now enforced
  by fetcher *presence* rather than an empty managed set.
- Correct the `bandwidth-type-scoping` spec so it no longer claims an empty managed
  set still serves explicit `restock_ticketbooks`.

**Non-Goals:**
- No change to `common/bandwidth-controller` (controller logic, the managed-set gate,
  or the `restock_ticketbooks` request all stay as-is).
- No change to the `nym-smoldvpn` datapath crate.
- No change to gateway registration, ticket *spending*, or top-up-over-metadata
  (`smoldvpn/src/topup.rs`).

## Decisions

**D1 — Scope `managed_ticket_types` to the WireGuard types in every mode.**
Drop the empty-vec default arm at `session.rs:254`; both modes use
`wireguard_ticket_types()`. This is what makes installing the fetcher trigger the
WireGuard restock, and it matches the existing `dvpn-session` "Mnemonic-funded
issuance" requirement ("scoping `managed_ticket_types` to the WireGuard types").
*Alternative rejected:* make the explicit `restock_ticketbooks` path bypass the
managed gate in the controller — rejected because the fix must not touch the
controller, and the reference does not rely on that behaviour.

**D2 — Provision against the RUNNING controller (Simon's Scenario B), one pattern
for every provisioning call.** The session spawns `run(self)` for the controller's
whole lifetime, so provisioning always goes through the request sender, never the
owned inline `fetch_ticketbook` (see D6). `ensure_ticket_types` therefore uses
`set_credential_fetcher(fetcher)` (whose install runs
`check_and_restock(managed_ticket_types)` → issuance of the managed WireGuard types,
per D1) followed by `wait_for_ticketbooks(types)`. Hold the constructed fetcher on the
session's owned-controller handle (not installed via the builder) so it can be
installed on demand.

**D3 — Two provisioning shapes, split by mode:**
- **Default / one-shot (`automatic_topups: None`)** — `set_credential_fetcher(fetcher)`
  → `wait_for_ticketbooks(types)` → **`unset_credential_fetcher()` unconditionally,
  even when the wait errors, times out, or is cancelled.** Removing on every exit path
  is required (Simon: "Don't forget to remove it even if there was an error"): a
  fetcher left installed after a failed provision would silently re-enable background
  deposits, defeating the default-mode guarantee. The fetcher never lingers in this
  mode, so no install-state tracking is needed here.
- **Automatic top-ups (`Some(policy)`)** — the fetcher is a long-lived install:
  install-if-absent (tracked by a local flag on the owned-controller handle; safe
  because the session is the controller's single writer and there is no request to
  query install state), `wait_for_ticketbooks(types)`, then leave it installed so the
  controller's sweep restocks per policy. A later provisioning call skips the
  re-install while the flag is set (avoids needless `cancel_and_join()` + `cleanup()`
  churn).

This preserves both `dvpn-session` "Opt-in automatic restock" scenarios and moves the
"no background deposit" guarantee onto fetcher presence.

**D6 — Do NOT restructure to the owned pre-run inline fetch (`fetch_ticketbook`).**
`BandwidthController::fetch_ticketbook` (`controller.rs:454`) fetches a type inline and
gate-free, but takes `&self` and only works before `run(self)` consumes the controller
(Simon's Scenario A; the `issue_ticket_book` ecash CLI uses exactly this and never
runs). The session cannot: it needs the controller running for the session's lifetime
(spending, top-ups), the provisioned types are known only at `ensure_ticketbooks(
two_hop)` / registration time (after `run()`), and `ensure_ticket_types` is also
invoked by the registration paths once the controller is already running. Mixing a
pre-run path and a running path around the `run(self)` ownership boundary would be
fragile, so the session uses the single running-controller pattern (D2/D3) everywhere.

**D4 — Uninstalling after `wait_for_ticketbooks` is safe.** `wait_for_ticketbooks`
resolves only when the required ticket types are stocked *and* their global signing
data is ready (`readiness.rs` evaluates tickets + global data). So by the time the
session unsets the fetcher, everything needed to spend the stocked ticketbooks is
already stored; later spends need no fetcher, and default mode does not restock again.
Only the issuance *trigger* changes; the `wait_for_ticketbooks` semantics — and thus
the signer-fault behaviour in `dvpn-session`'s availability requirement — are
untouched.

**D5 — Preserve the cancellation/timeout envelope.** Keep the existing
`PROVISIONING_TIMEOUT` + cancellation `select!` in `ensure_ticket_types` around the
new install/wait steps, and treat an empty `types` as a no-op before installing (a
fully cache-served registration must not install a fetcher or deposit).

## Risks / Trade-offs

- **[Uninstalling cancels in-flight global-data prefetch]** → `set_credential_fetcher`
  and `unset` both `cancel_and_join()` in-flight fetches. On the success path we unset
  only after `wait_for_ticketbooks` confirms readiness (D4), so nothing load-bearing is
  in flight; future-epoch reconciliation is re-fetched at spend time when a fetcher
  exists.
- **[Fetcher left installed after a failed provision]** → default-mode
  `ensure_ticket_types` MUST unset the fetcher on every exit path (error, timeout,
  cancellation), not just success (D3). Implement with an explicit best-effort
  `unset_credential_fetcher()` after the wait regardless of its outcome (an async
  cleanup can't live in `Drop`), inside the same `PROVISIONING_TIMEOUT`/cancellation
  envelope. A test asserts removal after a forced provisioning failure.
- **[Behaviour change for callers relying on a permanently installed fetcher]** → No
  public API relies on the fetcher being installed before `ensure_ticketbooks`;
  spending pulls from storage. Auto-top-up mode keeps the fetcher installed, matching
  prior behaviour.
- **[Spec correction could look like a controller regression]** → The
  `bandwidth-type-scoping` edit documents the controller's *actual, unchanged*
  behaviour (the managed set gates the explicit path too); no code moves, so there is
  no runtime regression, only a doc/spec correction.

## Migration Plan

Pure in-process behavioural fix in one crate; no data migration, no config change for
callers. Rollback is reverting the `nym-sdk-session` diff. Validation: the previously
failing `smoldvpn/tests/live_bringup.rs` gated tests pass against sandbox, and a new
deterministic `nym-sdk-session` test proves default-mode provisioning succeeds and
makes no background deposit afterward.

## Open Questions

- **Resolved (skip re-install):** in automatic-top-up mode `ensure_ticket_types`
  assumes the fetcher stays installed and skips re-install when already present (local
  flag), avoiding needless `cancel_and_join()` / `cleanup()` churn. In default mode the
  fetcher is always removed after each one-shot, so it is never lingering and is always
  freshly set. See D3.
- **Resolved (owned vs running):** the session provisions against the running
  controller (Scenario B), not the owned pre-run `fetch_ticketbook` (Scenario A). See
  D6.

## Context

`nym-sdk-session` provisions WireGuard ticketbooks and spends tickets through
`nym-bandwidth-controller`. Today (`session.rs`) it always spawns the controller event
loop and drives one-shot provisioning by the fetcher lifecycle: build a fresh fetcher,
`set_credential_fetcher` (whose install handler restocks the managed types),
`wait_for_ticketbooks`, then `unset_credential_fetcher`. Because uninstalling a fetcher
calls its `cleanup` (closing the `NyxdCredentialFetcher` recovery store), every install
needs a new instance, hence the `FetcherBuilder` trait; because two callers must not
both install, there is an `installed` flag under a mutex; because a failed fetch leaves
no retry trigger, auto-top-up mode retries by reinstalling. Review of PR 7119 judged
this usage wrong.

The controller crate already exposes both intended shapes, and this change must use
them without modifying anything under `common/`:

- `BandwidthController::fetch_ticketbook(typ)` fetches and persists one ticketbook
  inline (plus its global signing data, best-effort) and is documented as suitable
  for a non-running controller and one-shot issuance. The `nym-cli` ecash commands and
  `nym-sdk`'s `BandwidthAcquireClient` already use it that way.
- `BandwidthController<St>` implements `BandwidthTicketProvider` directly, with the
  comment "so we can use the BC without making it run on its own if we don't need
  that". Its `close()` closes the credential store.
- `BandwidthController::run` starts a `tokio::time::interval` whose first tick fires
  immediately, so a running controller constructed `with_credential_fetcher` restocks
  its managed types at startup and per `topup_interval` thereafter. The tail of `run`
  cancels in-flight fetches, cleans up the fetcher, and closes the store.
- `wait_for_ticketbooks(types)` resolves once every type is stocked (above
  `min_nb_ticket_needed`) and its signing data is present, and fails immediately with
  `TicketbooksUnavailable` if a type is neither stocked nor in flight.
- `AvailableTicketbooks` (`TryFrom<Vec<BasicTicketbookInformation>>`,
  `needs_restock`, `contains_minimal_tickets`) is public, and
  `Storage::get_ticketbooks_info` is on the public `Storage` trait, so stock can be
  assessed without the controller's private helper.

Constraints: `common/bandwidth-controller`, `common/bandwidth-fetcher` and
`common/credential-storage` are frozen for this change. The `Session` public API and
the `smoldvpn` call sites stay as they are. Funds safety (a deposit is never dropped
mid-flight; interrupted issuance is recoverable from the pending-request store) and
signer-failure tolerance (per-call bounds on read-only signing-data fetches; issuance
never timed) are preserved.

## Goals / Non-Goals

**Goals:**
- Automatic top-up: one running controller with the fetcher installed at
  construction; provisioning is a readiness wait.
- One-shot: no background task; issuance via inline `fetch_ticketbook` on a
  short-lived non-running controller that is dropped (and its fetcher cleaned up)
  afterwards; spending via a non-running controller that has no credential fetcher and
  therefore cannot deposit.
- Delete the install/uninstall lifecycle machinery and its tests.
- Keep topology scoping, provisioning timeout, cancellation semantics, and signer
  tolerance.
- Update the `dvpn-session` and `bandwidth-type-scoping` specs to describe the two
  modes.

**Non-Goals:**
- Any change to `common/` crates (including a `shutdown(self)` helper on the
  controller, which was considered and ruled out by the constraint).
- Changing the `Session` public API or the `smoldvpn` examples' flow.
- Making transient fetch failures retryable inside the fetcher.
- De-duplicating the examples' explicit `ensure_ticketbooks` call with the one the
  register paths make for uncached hops (both are cheap in the new design; left as is).

## Decisions

### D1. Mode is fixed at `Session::new` and selects the controller shape

`automatic_topups: Some(policy)` → **Running**: `BandwidthController::new(storage)
.with_config(cfg).with_credential_fetcher(fetcher)` with `cfg` from the policy and
`managed_ticket_types = needed_ticket_types(two_hop)`; `run(shutdown)` is spawned;
the `BandwidthTicketProvider` is the request sender.

`automatic_topups: None` → **OneShot**: no loop. The provider is
`BandwidthController::new(storage).with_credential_public_data_fetcher(
NyxdGlobalDataFetcher::new(nyxd))`, held as `Arc<dyn BandwidthTicketProvider>`. It has
no credential fetcher, so no code path on it can deposit; the public-data fetcher lets a
spend fetch signing data that was missing at issuance time (signer tolerance). The
session keeps what one-shot provisioning needs to build its short-lived controller:
the `Arc<DirectSigningHttpRpcNyxdClient>`, a clone of the credential-store handle, the
`fetcher-requests.db` path, the derived client id, the scoped
`BandwidthControllerConfig`, and a `tokio::sync::Mutex<()>`.

*Alternative considered:* one non-running controller holding the credential fetcher for
the whole session, used for both issuance and spending. Rejected: the fetcher's
recovery store could then only be closed by owning the controller at shutdown, which
the shared `Arc` handed to the tunnel prevents; and it leaves a deposit-capable object
alive for the session's lifetime.

### D2. `ensure_ticket_types` per mode

External provider: no-op (unchanged). Empty `types`: no-op (unchanged).

**Running:** `sender.wait_for_ticketbooks(types)`. If that fails with
`BandwidthControllerError::TicketbooksUnavailable` (nothing stocked and nothing in
flight), send `restock_ticketbooks(types)` once and wait again; any other error, or a
second `Unavailable`, is returned as `SessionError::Issuance`. `Unavailable` arises in
two ways: the startup fetch already failed, or the wait was processed before the startup
tick — the interval's first tick fires at startup but is **not** guaranteed to be polled
ahead of a request that is already queued when the loop starts (observed in the
running-mode test on a current-thread runtime). The fallback covers both, and cannot
double-issue: a restock for a type already in flight is a no-op on the controller side.
The whole call is raced against the cancel token and bounded by `PROVISIONING_TIMEOUT`;
that is safe because deposits run inside the controller task, not in the awaited future.

*Why not always restock first:* the controller documents `restock_ticketbooks` as a
manual safety valve, so it is used only as the recovery step for `Unavailable`.

**OneShot:** acquire the session mutex, racing acquisition against the cancel token
(a queued caller gets `Cancelled` promptly — existing behaviour and test). Then
`tokio::spawn` the provisioning task (D3) and race its `JoinHandle` against the cancel
token and `PROVISIONING_TIMEOUT`. The task is never aborted: a caller that cancels or
times out returns immediately while the task runs to completion in the background,
including teardown, so a deposit in flight is never dropped halfway. The mutex guard is
held across the race and released when the caller returns; a follow-up
`ensure_ticketbooks` that lands while the detached task is still fetching would see
stock as low and fetch again — acceptable for a cancelled/timed-out session, and
avoided in practice because callers cancel the whole session, not one provision.

### D3. One-shot provisioning is a free function over `Storage` + `CredentialFetcher`

```text
provision_once<St: Storage, F: CredentialFetcher + 'static>(
    storage: St, fetcher: F, config: BandwidthControllerConfig, types: Vec<TicketType>,
) -> Result<(), SessionError>
```

1. `BandwidthController::new(storage.clone()).with_config(config.clone())
   .with_credential_fetcher(fetcher)`.
2. Stock: `AvailableTicketbooks::try_from(storage.get_ticketbooks_info().await?)`.
3. For each `typ` in `types`: if `stock.needs_restock(typ, &config)` →
   `controller.fetch_ticketbook(typ).await` (mapped to `SessionError::Issuance`).
   Stop at the first error but still run step 5.
4. On success, re-read stock and require `contains_minimal_tickets(typ, &config)` for
   every requested type, else `Issuance("… not usable after issuance")`. This is the
   one-shot analogue of the readiness gate `wait_for_ticketbooks` gives the running
   mode (minus signing data, which is best-effort per the signer-tolerance
   requirement and fetched at spend time by the provider's public-data fetcher).
5. Teardown: drop the controller and call `cleanup` on the fetcher through a retained
   handle. A non-running controller has no in-flight work, so dropping it is complete;
   the fetcher's `cleanup` closes its pending-request recovery store. `storage` is the
   session's own handle (a clone shares the pool) and is left open.

The controller takes its fetcher by value and never hands it back, so the fetcher must
be a cloneable handle: `TimeoutFetcher` (already the session's decorator over the nyxd
fetcher) now holds its inner fetcher in an `Arc` and implements `Clone`; the controller
gets one clone, `provision_once` keeps the other for `cleanup`. `reset` on a still-shared
handle fails with `FetcherStillShared` rather than silently doing nothing (the controller
never calls `reset`). The production caller builds a fresh nyxd fetcher per call — the
recovery store is closed at teardown; the on-disk store carries any interrupted issuance
forward and the nyxd fetcher recovers pending deposits on its next fetch.

*Rejected first cut:* opening a second `PersistentStorage` handle for the provisioning
controller and tearing it down by driving `run` with an already-cancelled token (which
executes the controller's cleanup tail). It worked, but the store's pool guard waits for
the file's OS handles to be released on close and, with the session's own handle still
open, timed out after 2 s with a warning on every `ensure_ticketbooks` call. Sharing the
handle removes the second pool, the wait, the warning, and the `run` idiom.

Being generic makes the function directly testable with `initialise_ephemeral_storage`
and a recording fetcher, without a builder trait or network.

*Alternatives:* reading stock via a public controller method — none exists
(`get_available_ticketbooks` is private); peeking with
`get_next_usable_ticketbook(typ, 0)` — a fully spent book still matches. Reading the
`Storage` trait directly and using `AvailableTicketbooks` is the public route.

### D4. Threshold: the controller's own `needs_restock`

One-shot fetches a type when `remaining_tickets_long_lasting <= nb_ticket_restock`
(default 20), the same predicate the running controller's sweep uses. So "out of
tickets" in one-shot mode means "at or below the restock threshold or only in books
about to expire", and both modes agree on when a ticketbook is needed. One-shot uses
`BandwidthControllerConfig::default()` thresholds with the scoped managed set;
`RestockPolicy` continues to apply to the running mode only (the `Option` is the mode
switch).

### D5. Shutdown

**Running:** cancel the token and await the loop task (unchanged); `run`'s tail cleans
up the fetcher and closes the store. **OneShot:** `provider.close()` closes the store.
A detached provisioning task still running shares that handle and cleans up its own
fetcher when it finishes.
`Drop` keeps cancelling the token best-effort.

### D6. What is deleted / kept in `session.rs`

Deleted: `FetcherBuilder`, `NyxdFetcherBuilder`, `OwnedController` (and `installed`,
`auto_topup`, `provision`, `set_fetcher`, `set_fetcher_if_absent`, `remove_fetcher`,
`wait_for`), `spawn_controller`'s "do not install the fetcher" branch. Kept:
`needed_ticket_types`, `derive_client_id`, `TimeoutFetcher` wrapping,
`PROVISIONING_TIMEOUT`, `TopologyMismatch` guard, the cancel-raced lock acquisition.
Module docs in `lib.rs` and `session.rs` describe the two modes.

### D7. Tests

Replace the five `OwnedController` tests with tests of `provision_once` over ephemeral
storage and a recording fetcher (one that stores nothing and counts calls, or one that
can be pre-seeded so stock reads as sufficient), plus one running-mode test:

- one-shot skips a type whose stock does not need restock (no fetch, no deposit);
- one-shot fetches once per low type and cleans the fetcher up exactly once
  afterwards (teardown ran), on success and on fetch failure alike;
- one-shot reports a type still unusable after issuance as an error;
- queued one-shot caller behind the held mutex is cancellable (retain);
- running mode: `Unavailable` on first wait triggers exactly one restock request and a
  second wait.

The existing `tests/signer_fault_http.rs` harness drives the fetcher stack directly and
is unaffected. `smoldvpn/tests/live_bringup.rs` (gated) and the `smoldvpn-topup`
example exercise the one-shot path against the sandbox.

## Risks / Trade-offs

- [The fetcher handed to the controller cannot be reached again for `cleanup`] → The
  session hands the controller a clone of its `TimeoutFetcher` handle and cleans up
  through the retained one; the "cleans up exactly once" tests assert it. If the crate
  later exposes a shutdown method, the call becomes a one-line swap.
- [A cancelled or timed-out one-shot caller leaves a detached task depositing] → Same
  property as today (deposits ran in the controller task); funds are never dropped
  mid-flight, the task shares the session's store handle and cleans up its own
  fetcher. Documented on `ensure_ticketbooks`.
- [Running mode's first sweep tick may be processed after an already-queued wait] →
  The `Unavailable` → restock → wait fallback covers it (and the failed-startup-fetch
  case alike); an explicit restock for a type already in flight is a no-op, so no
  double issuance.
- [One-shot readiness does not gate on signing data] → Consistent with the existing
  signer-tolerance requirement: a persisted ticketbook is usable once its signing data
  is fetched at spend time through the provider's public-data fetcher.
- [`needs_restock` may buy a book while a few tickets remain] → Intentional: one-shot
  and running modes share one definition of "needs a ticketbook", and it avoids
  connecting on a book that is about to expire.

## Migration Plan

Single PR on the existing review branch. No data migration: credential store,
recovery store and registration cache formats are untouched. Rollback is a revert.

## Open Questions

None. The one design fork (adding a shutdown helper to the controller crate) was
closed by the constraint that `common/` stays unchanged.

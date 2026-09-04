# bandwidth-type-scoping Specification

## Purpose
TBD - created by archiving change fix-dvpn-review-6953. Update Purpose after archive.
## Requirements
### Requirement: Controller-scoped proactive restock

`BandwidthController` SHALL restrict the ticket types it proactively restocks — the
periodic sweep, the post-spend top-up, and the restock triggered when a credential
fetcher is installed — to the set configured in
`BandwidthControllerConfig::managed_ticket_types`. The default SHALL be every
non-mixnet-exit type (prior behaviour). An empty managed set SHALL disable proactive
restocking entirely. The managed set gates every restock path on a running controller,
including an explicit `restock_ticketbooks` request: a type outside the managed set is
never fetched by those paths, so an empty managed set means no restock path deposits.
A caller that wants a one-off issuance without background restock SHALL use a
**non-running** controller: construct it with the credential fetcher and the wanted
types in the managed set, call the inline `fetch_ticketbook` for each type that needs
it, and tear the controller down (drop it, clean up its fetcher) — rather than running
the event loop and toggling the fetcher, or pairing an empty managed set with an
explicit `restock_ticketbooks` call. A caller that wants continuous restock SHALL run
the event loop with the fetcher installed at construction and let the sweep act on the
managed set. The credential fetcher itself SHALL remain unchanged — it fetches whatever
type it is asked for; deciding what to request is the controller's (or the one-off
caller's) responsibility.

#### Scenario: Sweep restocks only managed types
- **WHEN** the config's managed set is the WireGuard ticket types and the periodic
  restock timer fires with all types low
- **THEN** fetches are spawned only for the WireGuard types, never for mixnet types

#### Scenario: Empty managed set makes no deposits
- **WHEN** the config's managed set is empty and a credential fetcher is installed
- **THEN** no restock path (sweep, post-spend top-up, fetcher-install restock, or an
  explicit `restock_ticketbooks` request) deposits, because every path is scoped to
  the managed set

#### Scenario: One-off issuance via a non-running controller
- **WHEN** a caller constructs a controller with the credential fetcher and the
  WireGuard types in the managed set, does not run its event loop, calls
  `fetch_ticketbook` for each WireGuard type whose stock needs a restock, and then
  tears the controller down
- **THEN** exactly those WireGuard ticketbooks are issued and stored, the fetcher is
  cleaned up, and no further deposit can be made because no loop is running

#### Scenario: Continuous restock via a running controller
- **WHEN** a caller constructs a controller with the credential fetcher installed and
  the WireGuard types in the managed set, and runs its event loop
- **THEN** the startup sweep restocks any low WireGuard type and later sweeps restock
  per the configured thresholds, without the caller installing or uninstalling the
  fetcher

#### Scenario: Post-spend restock respects the managed set
- **WHEN** a ticket of a type outside the managed set is spent
- **THEN** no restock is triggered for that spent type

#### Scenario: Default scoping preserves current behavior
- **WHEN** the controller is built without overriding `managed_ticket_types`
- **THEN** the sweep considers the same type list as before the change


## MODIFIED Requirements

### Requirement: Controller-scoped proactive restock

`BandwidthController` SHALL restrict the ticket types it proactively restocks — the
periodic sweep, the post-spend top-up, and the restock triggered when a credential
fetcher is installed — to the set configured in
`BandwidthControllerConfig::managed_ticket_types`. The default SHALL be every
non-mixnet-exit type (prior behaviour). An empty managed set SHALL disable proactive
restocking entirely. The managed set gates every restock path, including an explicit
`restock_ticketbooks` request: a type outside the managed set is never fetched, so an
empty managed set means no path deposits. A caller that wants a one-off restock
without leaving background restock enabled SHALL therefore control it through the
fetcher lifecycle — install the credential fetcher (with the wanted types in the
managed set) to trigger issuance, wait for readiness, then uninstall the fetcher —
rather than by pairing an empty managed set with an explicit `restock_ticketbooks`
call. The credential fetcher itself SHALL remain unchanged — it fetches whatever type
it is asked for; deciding what to request is the controller's responsibility.

#### Scenario: Sweep restocks only managed types
- **WHEN** the config's managed set is the WireGuard ticket types and the periodic
  restock timer fires with all types low
- **THEN** fetches are spawned only for the WireGuard types, never for mixnet types

#### Scenario: Empty managed set makes no deposits
- **WHEN** the config's managed set is empty and a credential fetcher is installed
- **THEN** no restock path (sweep, post-spend top-up, fetcher-install restock, or an
  explicit `restock_ticketbooks` request) deposits, because every path is scoped to
  the managed set

#### Scenario: One-off restock via the fetcher lifecycle
- **WHEN** the managed set is the WireGuard types, a caller installs the credential
  fetcher, waits for the WireGuard ticketbooks to become ready, then uninstalls the
  fetcher
- **THEN** the WireGuard ticketbooks are issued and stored, and no further deposit is
  made once the fetcher is removed

#### Scenario: Post-spend restock respects the managed set
- **WHEN** a ticket of a type outside the managed set is spent
- **THEN** no restock is triggered for that spent type

#### Scenario: Default scoping preserves current behavior
- **WHEN** the controller is built without overriding `managed_ticket_types`
- **THEN** the sweep considers the same type list as before the change

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
restocking entirely, while still allowing an installed credential fetcher to serve
on-demand and explicit `restock_ticketbooks` requests. The credential fetcher itself
SHALL remain unchanged — it fetches whatever type it is asked for; deciding what to
request is the controller's responsibility.

#### Scenario: Sweep restocks only managed types
- **WHEN** the config's managed set is the WireGuard ticket types and the periodic
  restock timer fires with all types low
- **THEN** fetches are spawned only for the WireGuard types, never for mixnet types

#### Scenario: Empty managed set makes no background deposits
- **WHEN** the config's managed set is empty and a credential fetcher is installed
- **THEN** no proactive path (sweep, post-spend top-up, or fetcher-install restock)
  deposits, though an explicit `restock_ticketbooks` request still works

#### Scenario: Post-spend restock respects the managed set
- **WHEN** a ticket of a type outside the managed set is spent
- **THEN** no restock is triggered for that spent type

#### Scenario: Default scoping preserves current behavior
- **WHEN** the controller is built without overriding `managed_ticket_types`
- **THEN** the sweep considers the same type list as before the change


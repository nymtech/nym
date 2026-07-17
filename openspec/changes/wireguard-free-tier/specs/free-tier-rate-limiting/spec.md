## ADDED Requirements

### Requirement: Shared bidirectional bandwidth pool

The gateway SHALL rate-limit free-tier peers via a single shared traffic-control pool on the `nymwg` interface, shaping both directions (egress to the peer and ingress from the peer). A free peer SHALL be placed in the pool at registration by a per-peer classifier keyed on the peer's IP.

#### Scenario: Free peer is shaped within the shared pool

- **WHEN** free peers are active and generating traffic
- **THEN** their combined throughput is capped by the shared pool in both directions

### Requirement: Bounded aggregate cost

Total free-tier bandwidth SHALL be bounded by the pool ceiling regardless of the number of free peers or tokens, so that abuse can degrade free-tier quality but cannot exceed the configured aggregate cost.

#### Scenario: Many free peers cannot exceed the pool

- **WHEN** the number of active free peers grows large
- **THEN** their aggregate throughput never exceeds the configured pool ceiling

### Requirement: Always-admit with graceful degradation

The gateway SHALL admit free-tier peers rather than rejecting them when the pool is busy; contention SHALL manifest as reduced per-peer throughput, not refused connections.

#### Scenario: New free peer under contention

- **WHEN** a new free peer registers while the pool is saturated
- **THEN** it is admitted and shares the degraded pool rather than being rejected

### Requirement: Rate-limit off-switch without disconnect

The gateway SHALL be able to remove a peer's rate limit without disconnecting it, by removing its pool classifier so its traffic falls to the default unlimited class. This off-switch SHALL be usable by both the walled-garden transition and the paid upgrade.

#### Scenario: Limit removed, session preserved

- **WHEN** a peer's rate limit is removed
- **THEN** its WireGuard session stays up and its traffic is no longer capped by the pool

### Requirement: Free-tier metrics exposure

The gateway SHALL expose the number of active free-tier users (pool members) and the configured pool allowance in mb/s, so a client application can surface current free-tier congestion.

#### Scenario: Metrics reflect current occupancy

- **WHEN** free peers join or leave the pool
- **THEN** the active-free-user metric reflects the change and the pool allowance metric reports the configured capacity

### Requirement: Rate-limit pool covers both address families

Each free peer holds both an IPv4 and an IPv6 tunnel address. The per-peer classifier that places a peer in the shared pool SHALL match both, and the off-switch SHALL remove both, so a peer is never shaped on one family while unshaped on the other.

#### Scenario: Both families are shaped and released together

- **WHEN** a free peer is added to or removed from the pool
- **THEN** its IPv4 and IPv6 traffic are shaped (or released) together

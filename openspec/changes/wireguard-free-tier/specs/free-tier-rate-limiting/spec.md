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

### Requirement: Membership scales to large peer counts

Per-peer pool membership SHALL be a kernel set (an `nftables` named set) matched by a single classifier rule, NOT one rule per peer. The per-packet classification cost and the per-peer add/remove cost SHALL therefore be independent of the number of pooled peers (target: 10k+), and the whitelist exemption SHALL likewise be a single set-matched rule rather than one rule per entry.

#### Scenario: Per-packet cost stays flat as the pool grows

- **WHEN** the number of pooled peers grows large
- **THEN** classification remains a single set lookup per packet rather than a linear chain scan, and adding or removing a peer is a set update rather than an iptables chain rewrite

### Requirement: Always-admit with graceful degradation

The gateway SHALL admit free-tier peers rather than rejecting them when the pool is busy; contention SHALL manifest as reduced per-peer throughput, not refused connections.

#### Scenario: New free peer under contention

- **WHEN** a new free peer registers while the pool is saturated
- **THEN** it is admitted and shares the degraded pool rather than being rejected

### Requirement: Purchase endpoint bypasses the rate-limit pool at full speed

A free-tier peer SHALL reach the purchase-endpoint allowlist at full speed even while its other traffic is confined to the shared rate-limit pool. The gateway SHALL enforce this with a rule that matches the allowlisted destinations and skips classification - leaving them in the unlimited default class - ordered ahead of the per-peer pool classifier. The exemption SHALL cover both shaping directions and both address families, and SHALL reuse the same purchase-endpoint allowlist as the walled garden (a single source of truth). The exemption is static for a free peer and does not toggle on exhaustion: the walled-garden transition changes only the fallthrough for non-allowlisted traffic (rate-limited pool while on trial, dropped once exhausted).

Traffic to the purchase endpoint still counts against the free byte allowance, because metering is measured at the WireGuard interface counter (total peer traffic, not per-destination). The exemption governs throughput, not accounting; this is acceptable because the checkout flow is small and it keeps the free allowance a single number.

#### Scenario: Checkout stays fast under a congested pool

- **WHEN** the shared free-tier pool is saturated and a free peer sends traffic to an allowlisted purchase endpoint
- **THEN** that traffic reaches the endpoint at full speed while the peer's other traffic remains capped by the pool

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

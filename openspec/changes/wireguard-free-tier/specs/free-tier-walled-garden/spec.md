## ADDED Requirements

### Requirement: Exhaustion routes to the walled garden

When a free peer's allowance is exhausted, the gateway SHALL move it into a walled garden instead of removing it. The peer's WireGuard session SHALL remain connected.

#### Scenario: Exhausted peer stays connected

- **WHEN** a free peer reaches its volume or time limit
- **THEN** the peer is not disconnected and enters the walled garden

### Requirement: Full-speed, allowlist-confined garden

A peer in the walled garden SHALL be removed from the rate-limit pool (restored to full speed) and simultaneously confined by a destination allowlist so that only the purchase endpoint is reachable. Full speed is permitted because the allowlist confines egress to the checkout path.

#### Scenario: Only the purchase endpoint is reachable

- **WHEN** a garden peer attempts to reach an address outside the allowlist
- **THEN** the traffic is dropped, while traffic to the allowlisted purchase endpoint succeeds at full speed

### Requirement: Node-managed iptables chain separated from operator rules

The garden SHALL be enforced by an `iptables` chain (`NYM-GARDEN`) whose scaffolding is pre-created by the operator setup script, while the node manages only per-peer membership within that chain (inserting and deleting `-s <peerIP>` rules). The node MUST NOT modify operator-managed base rules.

#### Scenario: Node toggles only its own chain

- **WHEN** the node moves a peer into or out of the garden
- **THEN** it inserts or deletes only that peer's rule in the `NYM-GARDEN` chain and leaves operator-managed rules untouched

### Requirement: Reconcile-on-start, unpersisted, fail-closed

The node SHALL rebuild the garden chain's per-peer contents from its free-tier state on startup, SHALL NOT persist those runtime rules, and SHALL be fail-closed: if the node stops with peers in the garden, those peers remain restricted until the node reconciles.

#### Scenario: Rules rebuilt from state after restart

- **WHEN** the node restarts
- **THEN** it flushes its own garden rules and re-derives them from the free-tier state, leaving no stale per-peer rules

#### Scenario: Crash leaves garden peers restricted

- **WHEN** the node crashes while peers are in the garden
- **THEN** those peers remain confined to the allowlist rather than gaining unrestricted egress

### Requirement: Exit to paid clears garden and rate limit

When a peer that is in the garden or the free pool presents a valid paid ecash credential, the gateway SHALL clear its garden rule, remove its rate limit, and set its free-tier flag off, restoring full unrestricted access. In v1 this occurs via reconnect-to-upgrade.

#### Scenario: Purchase restores full access

- **WHEN** a formerly-free peer presents a valid ecash credential
- **THEN** its garden rule and rate limit are removed and it is treated as a paid peer with unrestricted egress

### Requirement: Returning garden peer's registration reflects restricted access

When a free peer whose allowance is exhausted (in the walled garden) re-registers over the LP transport, the gateway SHALL return the peer's WireGuard configuration together with a restricted / purchase-only status marker, rather than a plain unrestricted completed registration. This lets the client keep a working tunnel to reach the purchase endpoint while surfacing that full access requires purchase. The marker mirrors the existing upgrade-mode flag on the success response; it MUST NOT be conveyed via `RequiresCredential` (the peer already holds a working, if restricted, session that it needs for checkout).

#### Scenario: Re-registration of an exhausted free peer signals restriction

- **WHEN** a free peer whose allowance is exhausted re-registers over the LP transport
- **THEN** the gateway returns the peer's config with a restricted / purchase-only marker set, not an unrestricted completed registration

### Requirement: Walled garden is dual-stack (IPv4 + IPv6)

Each free peer holds both an IPv4 and an IPv6 tunnel address, so the garden SHALL be enforced in both `iptables` and `ip6tables`: the `NYM-GARDEN` chain and its jump scaffolding exist in both, the node inserts/deletes the peer's rule for BOTH its v4 and v6 tunnel address, and the purchase-endpoint allowlist covers the endpoint's v4 and v6 addresses. A peer confined in one family but reachable on the other has an escape route.

#### Scenario: Garden confines both address families

- **WHEN** a peer is moved into the garden
- **THEN** its forwarded IPv4 and IPv6 traffic are both confined to the allowlist (rules present in both `iptables` and `ip6tables`)

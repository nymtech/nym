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

### Requirement: Node-ensured table separated from operator rules

The garden SHALL be enforced by a `forward`-hook chain in the node-owned shared `nftables` table (`inet nym_free_tier`, which also holds the rate-limit pool) that the node ENSURES idempotently at startup, rather than relying on an operator setup script. The chain SHALL sit at a priority ahead of the operator/iptables filter chains, since the garden is `DROP`-based and therefore ordering-sensitive, and it SHALL live in the node's own table so it never modifies operator-managed rules. Per-peer membership is a SET ELEMENT, not a rule, so confining or releasing a peer never touches any chain. Ensuring the scaffolding in the node (not an operator script) keeps the free tier working across reboots and upgrades, where kernel firewall state would otherwise be lost or stale.

#### Scenario: Node toggles only its own set

- **WHEN** the node moves a peer into or out of the garden
- **THEN** it adds or removes only that peer's element in its own set and leaves operator-managed rules untouched

#### Scenario: Scaffolding is present after a reboot without operator action

- **WHEN** the node starts and the garden table is absent (e.g. a reboot cleared the ruleset)
- **THEN** the node creates its table itself, so the garden is enforceable without a separate operator step

### Requirement: Garden membership scales to large peer counts

The set of confined peers and the purchase-endpoint whitelist SHALL each be a kernel set (an `nftables` named set) matched by a single rule, NOT one rule per peer or per whitelist entry. The per-packet garden decision and the per-peer confine/release cost SHALL therefore be independent of the number of confined peers and the whitelist size.

#### Scenario: Per-packet decision stays flat as confinement grows

- **WHEN** many peers are confined to the garden
- **THEN** the garden decision remains a single set lookup per packet rather than a linear chain scan, and confining or releasing a peer is a set update rather than an iptables chain rewrite

### Requirement: Reconcile-before-serve, unpersisted, fail-closed

The node SHALL rebuild the garden chain's per-peer contents from its free-tier state on startup and BEFORE it begins forwarding peer traffic or accepting registrations, so a returning garden peer is confined from its first forwarded packet (there is no window in which it is served unrestricted). It SHALL NOT persist those runtime rules to disk, and SHALL NOT tear them down on shutdown: while the node is stopped the kernel retains the `DROP` rules, so peers in the garden stay restricted until the node returns and reconciles. Fail-closed follows from these two together - reconcile-before-serve on startup, and persist-while-down on shutdown.

#### Scenario: Confinement precedes serving on restart

- **WHEN** the node restarts with peers that were in the garden
- **THEN** it rebuilds their garden rules from state before the datapath forwards their traffic, leaving no stale per-peer rules and never briefly serving them unrestricted

#### Scenario: Crash leaves garden peers restricted

- **WHEN** the node crashes while peers are in the garden
- **THEN** the kernel retains the `DROP` rules and those peers remain confined to the allowlist rather than gaining unrestricted egress

### Requirement: Explicit teardown of node-applied enforcement rules

Because the node deliberately does not remove its enforcement rules on shutdown (persist-while-down, above), it SHALL provide an explicit `nym-node` command that removes ALL free-tier routing state the node applies: the rate-limit pool's tc qdisc / classes and the single shared `nftables` table `inet nym_free_tier` (which holds both the pool and the garden, v4 and v6). The command SHALL be idempotent and tolerant of already-absent state, so it is safe to run after a non-graceful crash, when disabling the free tier, or when decommissioning the node.

#### Scenario: Operator wipes leftover rules after a crash

- **WHEN** the operator runs the teardown command after the node crashed with enforcement rules still applied
- **THEN** all node-applied pool and garden rules are removed in both address families, and running it again is a no-op

### Requirement: Exit to paid clears garden and rate limit

When a peer that is in the garden or the free pool presents a valid paid ecash credential, the gateway SHALL clear its garden rule, remove its rate limit, and set its free-tier flag off, restoring full unrestricted access. In v1 this occurs via reconnect-to-upgrade.

#### Scenario: Purchase restores full access

- **WHEN** a formerly-free peer presents a valid ecash credential
- **THEN** its garden rule and rate limit are removed and it is treated as a paid peer with unrestricted egress

### Requirement: Returning garden peer's registration reflects restricted access

When a free peer whose allowance is exhausted (in the walled garden) re-registers over the LP transport while still WITHIN the claim window (not yet eligible for a fresh trial), the gateway SHALL return the peer's WireGuard configuration together with a restricted / purchase-only status marker, rather than a plain unrestricted completed registration. This lets the client keep a working tunnel to reach the purchase endpoint while surfacing that full access requires purchase. The marker mirrors the existing upgrade-mode flag on the success response; it MUST NOT be conveyed via `RequiresCredential` (the peer already holds a working, if restricted, session that it needs for checkout).

#### Scenario: Re-registration of an exhausted free peer within the window signals restriction

- **WHEN** a free peer whose allowance is exhausted re-registers over LP while still within the claim window
- **THEN** the gateway returns the peer's config with a restricted / purchase-only marker set, not an unrestricted completed registration

### Requirement: A returning peer eligible to re-claim is asked for a credential

Once the claim window has elapsed (`now - granted_at >= claim_window`), a returning free peer is eligible for a fresh trial, and the gateway SHALL require a credential to grant it - respond `RequiresCredential`, not a plain resume of the spent/garden state and not an allowance auto-granted without a token. A fresh allowance is never granted without re-presenting a token, so the VPN-API's per-issuance limits stay meaningful. Whether the client reuses its stored token or must fetch a new one is governed by token `exp` (short `exp` -> a fresh token per trial; long `exp` -> one token re-claims each window).

#### Scenario: Reconnect after the window elapsed requires a fresh claim

- **WHEN** a free peer whose trial is spent reconnects after the claim window has elapsed
- **THEN** the gateway responds `RequiresCredential`, and grants a fresh allowance (new `granted_at`) only on a valid token - it does not auto-connect without one

### Requirement: Walled garden is dual-stack (IPv4 + IPv6)

Each free peer holds both an IPv4 and an IPv6 tunnel address, so the garden SHALL cover both families. In the `inet` table the confine chain matches both `ip` and `ip6`, the node adds/removes BOTH the peer's v4 and v6 tunnel address (into per-family sets), and the purchase-endpoint allowlist covers the endpoint's v4 and v6 addresses. A peer confined in one family but reachable on the other has an escape route.

#### Scenario: Garden confines both address families

- **WHEN** a peer is moved into the garden
- **THEN** its forwarded IPv4 and IPv6 traffic are both confined to the allowlist (rules present in both `iptables` and `ip6tables`)

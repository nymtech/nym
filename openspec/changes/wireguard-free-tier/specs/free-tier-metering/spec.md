## ADDED Requirements

### Requirement: Per-public-key free-tier record

The gateway SHALL maintain a per-public-key free-tier record holding the last-claim timestamp, the session start (for time metering), and an `is_free` flag. The last claim SHALL be stored as an absolute timestamp (not a boolean or bare date) so that the single-claim guard is evaluated by elapsed time at read, and no record ever needs a scheduled reset. This record SHALL be the source of truth for a peer's free-tier status, since the persisted client type does not distinguish free peers.

#### Scenario: Record created on free registration

- **WHEN** a peer is admitted on a free-tier token
- **THEN** a free-tier record is created for its public key with the last-claim timestamp set to now, the session start recorded, and `is_free` true

### Requirement: Rolling single-claim guard

A public key whose last claim was less than the claim window (a network constant, e.g. 24h) ago SHALL NOT receive a fresh allowance on a subsequent registration. The guard is evaluated as `now - last_claim < window` at registration time - never by a scheduled per-record reset - so a token cannot be re-presented to refill the allowance.

#### Scenario: Second claim within the window is refused

- **WHEN** a public key whose last claim is within the claim window registers again with a free-tier token
- **THEN** the gateway does not grant a fresh allowance

#### Scenario: Claim allowed once the window has elapsed

- **WHEN** more than the claim window has elapsed since the public key's last claim
- **THEN** a fresh free allowance may be granted and the last-claim timestamp is updated

### Requirement: Volume metering

Free-tier usage SHALL be metered by bytes using the existing bandwidth accounting, seeded from the network-defaults free allowance constant. When the byte allowance is depleted, the volume limit is considered reached.

#### Scenario: Byte allowance depletes

- **WHEN** a free peer consumes its seeded byte allowance
- **THEN** the volume limit is reached and the exhaustion transition is triggered

### Requirement: Time metering

Free-tier usage SHALL also be metered by elapsed session time against a configured cap, evaluated at the existing bandwidth-flush cadence. Precision finer than that cadence is NOT required.

#### Scenario: Session time cap reached

- **WHEN** a free peer's elapsed session time exceeds the configured cap, as observed at a flush-cadence check
- **THEN** the time limit is reached and the exhaustion transition is triggered

### Requirement: Whichever-first exhaustion

The free allowance SHALL end when either the volume limit or the time limit is reached first, and reaching either limit SHALL trigger the walled-garden transition rather than disconnection.

#### Scenario: Time exhausts before volume

- **WHEN** the session time cap is reached while byte allowance remains
- **THEN** the allowance ends and the peer transitions to the walled garden

### Requirement: Entry-gateway per-IP Sybil filtering

The entry gateway SHALL cap the number of new-user free-tier tokens it honors from a single client source IP within a daily window (configurable, e.g. 5/day), rejecting registrations from an IP over the cap. This is defense-in-depth alongside the VPN-API's per-issuance limits. It applies only on the transport where the gateway observes the client source IP (the LP/dVPN entry path), not over the authenticator-over-mixnet transport, and it does NOT apply to renewal tokens. The source IP may be IPv4 or IPv6; for IPv6 the cap SHALL be applied per-prefix (e.g. `/64`) rather than per-exact-address, since a single `/64` trivially yields enough addresses to defeat per-address limiting.

#### Scenario: Source IP over the daily cap is rejected

- **WHEN** a new-user free-tier token is presented from a source IP that has already reached the daily cap
- **THEN** the gateway rejects the registration and grants no free allowance

#### Scenario: Renewal tokens are exempt from per-IP limiting

- **WHEN** a renewal free-tier token is presented
- **THEN** the per-IP daily cap is not applied

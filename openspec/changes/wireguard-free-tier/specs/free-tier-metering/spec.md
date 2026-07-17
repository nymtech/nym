## ADDED Requirements

### Requirement: Per-public-key free-tier record

The gateway SHALL maintain a per-public-key free-tier record holding a single grant timestamp (`granted_at`) and an `is_free` flag. `granted_at` is an absolute timestamp that drives BOTH nested windows - the session time cap (`now - granted_at >= time_cap`) and the rolling refill guard (`now - granted_at < claim_window`) - so no separate "session start" field is needed and no record ever needs a scheduled reset. The record is keyed by public key and SHALL persist independently of the WireGuard peer - so the single-claim guard survives peer removal (otherwise exhaustion-removal would drop the record and let a re-presented token refill the allowance) and a future mid-trial resume remains possible - and is the source of truth for a peer's free-tier status since the persisted client type does not distinguish free peers.

#### Scenario: Record created on a fresh free claim

- **WHEN** a peer is admitted on a fresh free-tier claim
- **THEN** a free-tier record is created for its public key with `granted_at` set to now and `is_free` true

### Requirement: Rolling single-claim guard

A public key whose `granted_at` is less than the claim window (a network constant, e.g. 24h) ago SHALL NOT receive a FRESH allowance on a subsequent registration. The guard governs new grants only (resumes of an active trial are separate - see below) and is evaluated as `now - granted_at < window` at read - never by a scheduled per-record reset - so a token cannot be re-presented to refill the allowance.

#### Scenario: Fresh claim within the window is refused

- **WHEN** a public key whose spent trial is within the claim window registers again with a free-tier token
- **THEN** the gateway does not grant a fresh allowance

#### Scenario: Claim allowed once the window has elapsed

- **WHEN** more than the claim window has elapsed since `granted_at`
- **THEN** a fresh free allowance may be granted and `granted_at` is updated to now

### Requirement: Reconnecting within the trial resumes the same allowance

Reconnecting while the WireGuard peer still exists SHALL resume the existing allowance - the remaining bytes and remaining time measured from `granted_at`, not a fresh grant - and SHALL NOT require re-presenting the token, nor be blocked by the single-claim guard (which governs new grants only). This is provided by the existing-peer short-circuit at registration: a client disconnect does not remove the connectionless server-side peer, and a restart reconciles peers from storage, so the live bandwidth entry is preserved and simply continued. Time is wall-clock from `granted_at`; disconnecting does not pause it.

If instead the WireGuard peer row was removed (e.g. exhaustion-removal before the walled garden exists, or a transient error), its remaining bytes CANNOT be recovered - a re-registration allocates a new client id and a fresh, empty bandwidth entry - so the gateway SHALL NOT seed a fresh allowance for a record still inside the claim window: doing so would let a peer refill its allowance by reconnecting after exhaustion. Such a reconnect is refused in v1, and the peer becomes eligible again only once the claim window elapses. Routing these into the walled garden, and restoring genuinely-remaining bytes, are deferred (the latter requires persisting remaining bytes into the record).

#### Scenario: Reconnect mid-trial resumes the remaining allowance (peer still present)

- **WHEN** a free peer whose WireGuard peer still exists reconnects while within its trial window with bytes remaining
- **THEN** it resumes with its remaining bytes and time, no fresh allowance is granted, and no token is required

#### Scenario: Reconnect after the peer row was removed is not re-granted

- **WHEN** a free peer whose WireGuard peer row was removed re-presents its token while still inside the claim window
- **THEN** the gateway does not seed a fresh allowance (which would be a reconnect-refill) and the peer becomes eligible again only once the claim window has elapsed

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

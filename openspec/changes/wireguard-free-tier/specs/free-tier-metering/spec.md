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

Reconnecting while still inside the trial window SHALL resume the existing allowance - the remaining bytes and remaining time measured from `granted_at`, not a fresh grant - and SHALL NOT require re-presenting the token, nor be blocked by the single-claim guard (which governs new grants only). Because the record is keyed by public key and outlives the WireGuard peer, a peer idle-reaped mid-trial still resumes on reconnect instead of being forced into a new, guard-blocked claim. Time is wall-clock from `granted_at`; disconnecting does not pause it.

#### Scenario: Reconnect mid-trial resumes the remaining allowance

- **WHEN** a free peer reconnects while still within its trial window with bytes remaining
- **THEN** it resumes with its remaining bytes and time, no fresh allowance is granted, and no token is required

#### Scenario: Idle-reaped peer still resumes within the trial

- **WHEN** a free peer whose WireGuard peer was reaped reconnects within the trial window and re-presents its token
- **THEN** the gateway resumes the existing record's remaining allowance rather than treating it as a new (guard-blocked) claim

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

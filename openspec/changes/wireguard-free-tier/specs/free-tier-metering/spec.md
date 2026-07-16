## ADDED Requirements

### Requirement: Per-public-key free-tier record

The gateway SHALL maintain a per-public-key free-tier record holding the daily-claim marker, the session start (for time metering), and an `is_free` flag. This record SHALL be the source of truth for a peer's free-tier status, since the persisted client type does not distinguish free peers.

#### Scenario: Record created on free registration

- **WHEN** a peer is admitted on a free-tier token
- **THEN** a free-tier record is created for its public key with the daily-claim marker set, the session start recorded, and `is_free` true

### Requirement: Daily single-claim guard

A public key that has already claimed a free allowance within the current daily window SHALL NOT receive a fresh allowance on a subsequent registration. This guard is what prevents re-presenting a token from refilling the allowance.

#### Scenario: Second claim in the same day is refused

- **WHEN** a public key that already claimed today registers again with a free-tier token
- **THEN** the gateway does not grant a fresh allowance

#### Scenario: Claim allowed after the daily window resets

- **WHEN** the daily window has reset since the last claim
- **THEN** a fresh free allowance may be granted

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

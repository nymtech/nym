## MODIFIED Requirements

### Requirement: Root-signed checkpoint datum

The system SHALL define a `SignedCheckpoint` datum that wraps a full `Checkpoint` (`height`, `signed_header`, `validators`, `next_validators`), an advisory `created_at` timestamp, and a single root signature over the checkpoint's canonical signing payload. The datum SHALL be self-authenticating: its trust derives solely from the root signature, so it MAY be transported over any untrusted channel. The datum type and its verification SHALL live in `nym-contract-anchor` alongside `Checkpoint` and the tendermint/light-client types it embeds, reusing `nym-contract-attestation`'s domain-tag signing-payload helpers so that no tendermint dependency is forced into that crate.

#### Scenario: Datum carries the full checkpoint
- **WHEN** a `SignedCheckpoint` is constructed for a checkpoint at height `H`
- **THEN** it embeds the complete `Checkpoint` (signed header plus both validator sets) so a loader can build a `LightClientAnchor` without any additional RPC call

#### Scenario: Trust derives from the root signature
- **WHEN** a `SignedCheckpoint` is obtained from any source
- **THEN** it is accepted only if its root signature verifies, regardless of whether the source was the compiled-in constant, an HTTPS response, or any other channel

#### Scenario: Checkpoint bootstrap is not directory-specific
- **WHEN** a client for a contract other than the directory needs a light-client anchor
- **THEN** it loads and verifies a `SignedCheckpoint` through the same layer, since a checkpoint anchors a chain rather than a contract and is therefore shared across every contract on that chain

## ADDED Requirements

### Requirement: A checkpoint is shared across contracts on one chain

A verified checkpoint anchors a chain, not a contract, so the loader, its ordered provider chain, and any persisted verified head SHALL be reusable by every contract client on that chain rather than being duplicated per contract.

#### Scenario: One checkpoint seeds anchors for two contracts

- **WHEN** a process builds a light-client anchor for the directory contract and another for the geolocation contract on the same chain
- **THEN** both may be seeded from the same loaded `SignedCheckpoint`, with no second load or second root-signature verification required

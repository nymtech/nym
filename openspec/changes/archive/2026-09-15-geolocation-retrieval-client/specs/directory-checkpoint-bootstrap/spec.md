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

### Requirement: Domain-separated, protobuf-committed signing payload

The bytes signed for a checkpoint datum SHALL be `domain_tag || chain_id || height || blake3(proto_encode(checkpoint))`, where the fixed-width wrapper follows the existing `nym-contract-attestation` signing-payload convention (length-prefixed variable fields, `blake3` for the bulk-data commitment as in `subset_hash`) and `proto_encode` is Tendermint's own `Protobuf` encoding of the checkpoint (its native canonical form). The checkpoint SHALL NOT be committed via ad-hoc JSON or a hand-rolled serializer of the nested header/validator-set structures. The domain tag SHALL be distinct from every other root-signed or identity-signed payload in the system (upgrade-mode attestation, snapshot, subset digest, node entry), so a signature produced over a checkpoint payload SHALL NOT be interpretable as a valid signature over any other payload type, and vice versa. Signer and verifier SHALL use the same protobuf encoder so the committed bytes are reproducible.

The tag's own value SHALL NOT change with the crate rename. It is part of the signed bytes, so altering it would invalidate every checkpoint ever minted; its historical spelling is therefore retained deliberately.

#### Scenario: Checkpoint signature does not cross domains
- **WHEN** a root signature is produced over a checkpoint payload
- **THEN** it cannot be interpreted as a valid upgrade-mode attestation signature, and an upgrade-mode attestation signature cannot be interpreted as a valid checkpoint signature

#### Scenario: Tampered checkpoint fails verification
- **WHEN** any field of the embedded checkpoint is altered after signing
- **THEN** the recomputed `sha256(proto_encode(checkpoint))` differs and the root-signature verification fails

#### Scenario: Signer and verifier agree on the committed bytes
- **WHEN** the signer and the loader independently compute the signing payload from the same checkpoint
- **THEN** the protobuf-encoded bytes and resulting payload are identical and the signature verifies

#### Scenario: An existing checkpoint still verifies after the rename
- **WHEN** a checkpoint minted before the crate was renamed is loaded
- **THEN** its root signature still verifies, because the domain tag and the payload encoding are unchanged

## ADDED Requirements

### Requirement: A checkpoint is shared across contracts on one chain

A verified checkpoint anchors a chain, not a contract, so the loader, its ordered provider chain, and any persisted verified head SHALL be reusable by every contract client on that chain rather than being duplicated per contract.

#### Scenario: One checkpoint seeds anchors for two contracts

- **WHEN** a process builds a light-client anchor for the directory contract and another for the geolocation contract on the same chain
- **THEN** both may be seeded from the same loaded `SignedCheckpoint`, with no second load or second root-signature verification required

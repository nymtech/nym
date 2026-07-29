# directory-checkpoint-bootstrap Specification

## Purpose
TBD - created by archiving change directory-checkpoint-bootstrap. Update Purpose after archive.
## Requirements
### Requirement: Root-signed checkpoint datum

The system SHALL define a `SignedCheckpoint` datum that wraps a full `Checkpoint` (`height`, `signed_header`, `validators`, `next_validators`), an advisory `created_at` timestamp, and a single root signature over the checkpoint's canonical signing payload. The datum SHALL be self-authenticating: its trust derives solely from the root signature, so it MAY be transported over any untrusted channel. The datum type and its verification SHALL live in `nym-directory-client` alongside `Checkpoint` and the tendermint/light-client types it embeds, reusing `nym-directory-attestation`'s domain-tag signing-payload helpers so that no tendermint dependency is forced into that crate.

#### Scenario: Datum carries the full checkpoint
- **WHEN** a `SignedCheckpoint` is constructed for a checkpoint at height `H`
- **THEN** it embeds the complete `Checkpoint` (signed header plus both validator sets) so a loader can build a `LightClientAnchor` without any additional RPC call

#### Scenario: Trust derives from the root signature
- **WHEN** a `SignedCheckpoint` is obtained from any source
- **THEN** it is accepted only if its root signature verifies, regardless of whether the source was the compiled-in constant, an HTTPS response, or any other channel

### Requirement: Single hardcoded root key with backward-compatible aliasing

Checkpoint datums SHALL be verified against a single hardcoded root public key per network, sourced from `nym-network-defaults` as a bs58 string and parsed downstream. This key SHALL be the existing upgrade-mode attester key, generalised: the shared network-default constant and env-var name SHALL be renamed to a domain-neutral identifier, and the reader SHALL accept the new canonical env-var name and fall back to the legacy `UPGRADE_MODE_ATTESTER_ED25519_PUBKEY` name so existing environment files keep working. The nym-node upgrade-mode override knobs SHALL remain unchanged.

#### Scenario: Legacy env var still resolves the root key
- **WHEN** only the legacy `UPGRADE_MODE_ATTESTER_ED25519_PUBKEY` env var is set
- **THEN** the checkpoint root key resolves to that value

#### Scenario: New canonical env var takes precedence
- **WHEN** both the new canonical env var and the legacy name are set
- **THEN** the new canonical value is used

#### Scenario: Rotation affects both subsystems
- **WHEN** the root key is rotated
- **THEN** both upgrade-mode attestation and directory checkpoint verification move to the new key, because they share one root

### Requirement: Domain-separated, protobuf-committed signing payload

The bytes signed for a checkpoint datum SHALL be `domain_tag || chain_id || height || blake3(proto_encode(checkpoint))`, where the fixed-width wrapper follows the existing `nym-directory-attestation` signing-payload convention (length-prefixed variable fields, `blake3` for the bulk-data commitment as in `subset_hash`) and `proto_encode` is Tendermint's own `Protobuf` encoding of the checkpoint (its native canonical form). The checkpoint SHALL NOT be committed via ad-hoc JSON or a hand-rolled serializer of the nested header/validator-set structures. The domain tag SHALL be distinct from every other root-signed or identity-signed payload in the system (upgrade-mode attestation, snapshot, subset digest, node entry), so a signature produced over a checkpoint payload SHALL NOT be interpretable as a valid signature over any other payload type, and vice versa. Signer and verifier SHALL use the same protobuf encoder so the committed bytes are reproducible.

#### Scenario: Checkpoint signature does not cross domains
- **WHEN** a root signature is produced over a checkpoint payload
- **THEN** it cannot be interpreted as a valid upgrade-mode attestation signature, and an upgrade-mode attestation signature cannot be interpreted as a valid checkpoint signature

#### Scenario: Tampered checkpoint fails verification
- **WHEN** any field of the embedded checkpoint is altered after signing
- **THEN** the recomputed `sha256(proto_encode(checkpoint))` differs and the root-signature verification fails

#### Scenario: Signer and verifier agree on the committed bytes
- **WHEN** the signer and the loader independently compute the signing payload from the same checkpoint
- **THEN** the protobuf-encoded bytes and resulting payload are identical and the signature verifies

### Requirement: Load-time staleness without an expiry field

The datum SHALL NOT carry an expiry field. A loader SHALL derive staleness from the checkpoint's own signed block time and the compiled trusting period, rejecting the datum when `header_time + trusting_period < now`. The staleness check SHALL read the trusting period from the same single source the `LightClientAnchor` uses, so the two cannot diverge. The `created_at` timestamp SHALL be treated as advisory metadata only and SHALL NOT be used as a validity bound.

#### Scenario: Fresh checkpoint loads
- **WHEN** a datum's checkpoint block time is within `now - trusting_period`
- **THEN** the loader accepts it and constructs the anchor

#### Scenario: Stale checkpoint fails loud at load
- **WHEN** a datum's checkpoint block time is older than `trusting_period` relative to now
- **THEN** the loader returns a typed staleness error before any anchor is constructed

#### Scenario: created_at does not affect validity
- **WHEN** a datum's `created_at` is far in the past but its checkpoint block time is within the trusting period
- **THEN** the datum is still accepted, because `created_at` is advisory only

### Requirement: Ordered checkpoint providers and untrusted sources

The system SHALL expose a `CheckpointProvider` abstraction and, for v1, try providers in strict priority order, using the first that yields a valid (non-stale, and for signed sources signature-verified) checkpoint:
1. **stored** - the locally persisted, previously light-client-verified head (no root signature required; trusted at filesystem-integrity level; staleness-checked);
2. **hardcoded** - the compiled-in signed datum from a dedicated `nym-network-defaults` constant;
3. **HTTPS** - a signed datum fetched from a configurable, env-overridable well-known URL mirroring the upgrade-mode attestation URL pattern.

Signed providers SHALL be treated as availability-only: every signed datum SHALL still be verified against the root key, so a malicious or faulty source cannot inject a trusted checkpoint. DNS, bulletin-board, and node/api-served sources are out of scope for v1.

#### Scenario: Fresh stored head is preferred
- **WHEN** a valid, non-stale stored head is present
- **THEN** the loader uses it without reading the hardcoded constant or making any network call

#### Scenario: Fresh boot falls back to the hardcoded seed
- **WHEN** no stored head is present (or it is stale) and the hardcoded constant is non-stale
- **THEN** the loader uses the hardcoded signed datum after verifying it against the root key

#### Scenario: Aged-out seed falls back to HTTPS
- **WHEN** neither the stored head nor the hardcoded constant is within the trusting period
- **THEN** the loader fetches the HTTPS datum and uses it only after root-signature verification and the staleness check pass

#### Scenario: HTTPS source serving a bad datum is rejected
- **WHEN** the HTTPS provider returns a datum whose root signature does not verify
- **THEN** the loader rejects it and does not construct an anchor from it

### Requirement: Checkpoint loader selects a trusted checkpoint

A loader SHALL try its providers in priority order and return the first candidate checkpoint that is within the trusting period, or a typed error if none qualifies. Root-signature verification SHALL be performed by each signed provider before it yields a candidate (the stored provider carries no signature and is trusted transitively); the loader itself SHALL apply only the staleness check and the ordering. Constructing a `LightClientAnchor` from the returned checkpoint SHALL be the caller's responsibility, not the loader's.

#### Scenario: First non-stale candidate is returned
- **WHEN** a provider yields a candidate whose root signature verifies (for a signed source) and whose block time is within the trusting period
- **THEN** the loader returns that checkpoint to the caller and consults no lower-priority provider

#### Scenario: No valid source yields a typed error
- **WHEN** no provider yields a candidate that passes both its own signature verification and the loader's staleness check
- **THEN** the loader returns a typed error and no checkpoint

### Requirement: Offline checkpoint minting tool

The system SHALL provide a maintainer-only offline tool (a dedicated binary, not the user-facing CLI) that fetches a checkpoint from a trusted RPC, signs it with the root key, and regenerates the compiled-in checkpoint datum for commit. The tool SHALL accept the root private key as an argument that can also be sourced from an environment variable. Before writing, the tool SHALL self-verify the minted checkpoint by advancing it at least one light-client verification hop against the RPC, aborting on failure. Given pinned inputs (including an overridable mint timestamp), the tool's output SHALL be deterministic.

#### Scenario: Minted checkpoint is self-verified before writing
- **WHEN** the tool mints a checkpoint from a given RPC
- **THEN** it advances the minted checkpoint at least one light-client verification hop, and it aborts without writing if that fails

#### Scenario: Key is sourced from the environment in production
- **WHEN** the root private key argument is not passed explicitly but the corresponding environment variable is set
- **THEN** the tool uses the environment-provided key

#### Scenario: Deterministic regeneration
- **WHEN** the tool is run twice with the same RPC response, height, and pinned mint timestamp
- **THEN** it produces the identical signed constant


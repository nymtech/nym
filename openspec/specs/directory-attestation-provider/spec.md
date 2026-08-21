# directory-attestation-provider Specification

## Purpose

Defines how a signer (a nym-api today, a nym-node later) produces the data an `AttestedTrustAnchor` needs: signed, K-of-N-quorum-verifiable directory snapshots at a contract-dictated cadence, served over HTTP, plus a generic mechanism for signing and quorum-verifying arbitrary canonical "subsets" of directory/node data. The producer logic is a signer-agnostic library (`nym-directory-attestation`) shared by producers and the verifying client, so the trust tier is purely the client's choice of which signer set it accepts, not a property of the format. This capability covers the producer side and the shared protocol; the anchor and the retrieval client are separate capabilities.

## Requirements

### Requirement: Contract-dictated snapshot cadence

A producer SHALL read the snapshot interval from the directory contract and produce snapshots only at cadence heights (heights that are exact multiples of the interval), so that independent producers converge on identical snapshot heights without coordination. The producer SHALL NOT invent its own cadence. The interval SHALL be treated as configuration read from on-chain state, not as data committed into the directory digest or the signed snapshot.

#### Scenario: Producer follows the contract interval

- **WHEN** the contract's snapshot interval is `Δ` and a producer observes the chain crossing a height `H` with `H mod Δ == 0`
- **THEN** the producer computes and signs a snapshot for height `H`, and does not produce snapshots at non-cadence heights

#### Scenario: Independent producers converge on the same heights

- **WHEN** two independently operated producers read the same interval `Δ` from the contract
- **THEN** both produce snapshots at the identical set of cadence heights, so a quorum can compare like-for-like

### Requirement: Deterministic production, retained window, and settle-lagged latest

A producer SHALL retain the most recent `N` cadence snapshots (a local configuration value) and SHALL answer an explicit request for any retained cadence height immediately once that height is produced. The producer SHALL advertise as "latest" the greatest cadence height `H` such that the current chain tip is at least `H + settle_lag` blocks, where `settle_lag` is a small local configuration value in blocks. A freshly produced cadence height SHALL be explicitly queryable before it is advertised as latest.

#### Scenario: Explicit height is served before it becomes latest

- **WHEN** a producer has just produced the snapshot for cadence height `H` and the tip has not yet reached `H + settle_lag`
- **THEN** an explicit query for height `H` returns that snapshot, while "latest" still returns the previous cadence height

#### Scenario: Latest advances after the settle lag

- **WHEN** the chain tip reaches `H + settle_lag`
- **THEN** "latest" advances to `H`

#### Scenario: Old snapshots fall out of the window

- **WHEN** more than `N` cadence snapshots have been produced
- **THEN** the producer retains only the most recent `N`, and an explicit query for an evicted height is reported as unavailable

### Requirement: Produce only what was verified

Before signing a snapshot for height `H`, a producer SHALL fetch and verify the whole directory at `H` through a `DirectoryTrustAnchor`-backed retrieval, and SHALL sign only the `app_hash`, digest `accumulator`, and node-identity hash that verification established. The source anchor SHALL be configurable (defaulting to a proven-RPC anchor against the producer's own chain connection, and permitting a light-client anchor), and SHALL NOT be an attested anchor (which would make the producer trust another attestation to make its own).

#### Scenario: A directory that fails verification is not attested

- **WHEN** the producer's source anchor fails to verify the directory at `H` (for example the locally recomputed accumulator does not match)
- **THEN** the producer does not sign or publish a snapshot for `H`

#### Scenario: Operator can choose the source anchor

- **WHEN** an operator configures the producer with a light-client source anchor instead of the default proven-RPC anchor
- **THEN** the producer verifies the directory via that anchor before signing, and the published snapshot format is unchanged

### Requirement: Canonical, replay-resistant attestation payloads

The bytes a producer signs SHALL be produced by the shared canonical encoders in `nym-directory-attestation`, identical to what the verifying client recomputes: the snapshot signing payload binds a domain tag, chain-id, contract, height, `app_hash`, `accumulator`, and `node_identities_hash`; a subset digest binds a distinct domain tag, chain-id, height, subset identifier, and a hash over the subset's canonical bytes. Distinct domain tags SHALL keep snapshot signatures, subset-digest signatures, and node-entry signatures mutually non-interchangeable. A signature SHALL bind chain-id (and, for snapshots, contract) so it cannot be replayed across chains or contract instances.

#### Scenario: Producer and client agree on the bytes

- **WHEN** a producer signs a snapshot (or a subset digest) and a client recomputes the signing payload from the same fields
- **THEN** the byte encodings are identical and the signature verifies

#### Scenario: Signature domains do not cross

- **WHEN** a signature is produced over a subset digest
- **THEN** it cannot be interpreted as a valid snapshot signature or a valid node-entry signature, and vice versa

### Requirement: Generic subset attestation

The library SHALL provide a generic mechanism for attesting canonical subsets of directory/node data, independent of and alongside the fixed `DigestSnapshot`: a `DirectorySubset` trait (a stable subset identifier plus a canonical byte encoding), a small signed `SignedSubsetDigest` committing a hash over those canonical bytes at a height, and an `AttestedSubset<T>` carrying that signed digest together with the subset data itself. Trust in subset data SHALL flow from a K-of-N quorum agreeing on the committed hash, and the data itself SHALL be verified by local recompute against that hash. The `node_identities_hash` carried inside `DigestSnapshot` SHALL remain part of the snapshot and SHALL NOT be moved into this mechanism.

#### Scenario: Subset trusted via quorum on the hash, data fetched once

- **WHEN** a client reaches a quorum of K distinct trusted signers on identical `SignedSubsetDigest` hashes for a subset at height `H`, then fetches a single `AttestedSubset<T>` for that subset from any one source
- **THEN** the client accepts the data only if the locally recomputed hash over the subset's canonical bytes equals both the fetched digest's hash and the quorum-agreed hash

#### Scenario: Tampered subset data fails closed

- **WHEN** a source serves subset data that does not hash to the quorum-agreed value
- **THEN** the recompute check fails and the client rejects the data rather than returning it

#### Scenario: A single signed digest does not confer trust

- **WHEN** only one trusted signer's `SignedSubsetDigest` is available for a subset (including the one embedded in a fetched `AttestedSubset<T>`)
- **THEN** it counts as at most one quorum candidate and, below K distinct signers, the subset is not trusted

### Requirement: Signer-agnostic library, tier decided by the client

The producer core SHALL be signer-agnostic: it signs with whatever ed25519 identity keypair it is given, with no notion of "nym-api" versus "nym-node" baked into the format or the library. The trust tier SHALL be determined entirely by the verifying client's choice of which signer set (and quorum threshold) to accept.

#### Scenario: Same library, different signers

- **WHEN** a nym-api and a nym-node each build and sign a snapshot or subset with their own identity keypair using the same library
- **THEN** the produced structures are identical in shape, and only the `signer` field (and which client-side signer set accepts it) distinguishes their trust tier

### Requirement: Whole-directory serving at retained heights

A producer SHALL be able to serve the whole verified directory (entries and the node-identity mapping) at a retained cadence height, so a client with no chain RPC connection can retrieve and verify it against that height's quorum-attested `accumulator` and `node_identities_hash`. This serving path SHALL use the values already committed by `DigestSnapshot` and SHALL NOT require the generic subset mechanism.

#### Scenario: No-RPC client pulls and verifies the full directory

- **WHEN** a client fetches the full directory at a retained height `H` from a producer and holds the quorum-attested snapshot for `H`
- **THEN** it verifies the entries against the `accumulator` and the node identities against the `node_identities_hash` by local recompute alone, with no chain query

### Requirement: HTTP exposure of produced attestations

A producer SHALL expose its produced data over HTTP: the settle-lagged latest signed snapshot, a signed snapshot at a specific retained height, and the full verified directory at a retained height. Responses SHALL carry the canonical signed structures defined by `nym-directory-attestation` so a client can verify signatures and recompute hashes.

#### Scenario: Client fetches latest and a specific height

- **WHEN** a client requests the latest snapshot and then a snapshot at a specific retained height
- **THEN** the producer returns the corresponding `SignedDigestSnapshot`s, verifiable against the producer's identity key

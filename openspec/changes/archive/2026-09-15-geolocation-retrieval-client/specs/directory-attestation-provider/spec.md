## MODIFIED Requirements

### Requirement: Generic subset attestation

The library SHALL provide a generic mechanism for attesting canonical subsets of directory/node data, independent of and alongside the fixed `DigestSnapshot`: a `DirectorySubset` trait (a stable subset identifier plus a canonical byte encoding), a small signed `SignedSubsetDigest` committing a hash over those canonical bytes at a height, and an `AttestedSubset<T>` carrying that signed digest together with the subset data itself. Trust in subset data SHALL flow from a K-of-N quorum agreeing on the committed hash, and the data itself SHALL be verified by local recompute against that hash. The `node_identities_hash` carried inside `DigestSnapshot` SHALL remain part of the snapshot and SHALL NOT be moved into this mechanism. This mechanism SHALL live in `nym-contract-attestation`, the crate formerly named `nym-directory-attestation`.

#### Scenario: Subset trusted via quorum on the hash, data fetched once

- **WHEN** a client reaches a quorum of K distinct trusted signers on identical `SignedSubsetDigest` hashes for a subset at height `H`, then fetches a single `AttestedSubset<T>` for that subset from any one source
- **THEN** the client accepts the data only if the locally recomputed hash over the subset's canonical bytes equals both the fetched digest's hash and the quorum-agreed hash

#### Scenario: Tampered subset data fails closed

- **WHEN** a source serves subset data that does not hash to the quorum-agreed value
- **THEN** the recompute check fails and the client rejects the data rather than returning it

#### Scenario: A single signed digest does not confer trust

- **WHEN** only one trusted signer's `SignedSubsetDigest` is available for a subset (including the one embedded in a fetched `AttestedSubset<T>`)
- **THEN** it counts as at most one quorum candidate and, below K distinct signers, the subset is not trusted

### Requirement: Whole-directory serving at retained heights

A producer SHALL be able to serve the whole verified contract record set (entries and the node-identity mapping) at a retained cadence height, so a client with no chain RPC connection can retrieve and verify it against that height's quorum-attested `accumulator` and `node_identities_hash`. This serving path SHALL use the values already committed by `DigestSnapshot` and SHALL NOT require the generic subset mechanism. The transfer payload SHALL be generic over the record type, so one type serves directory records and geolocation records rather than one struct per domain.

#### Scenario: No-RPC client pulls and verifies the full directory

- **WHEN** a client fetches the full directory at a retained height `H` from a producer and holds the quorum-attested snapshot for `H`
- **THEN** it verifies the entries against the `accumulator` and the node identities against the `node_identities_hash` by local recompute alone, with no chain query

#### Scenario: The same payload shape carries geolocation records

- **WHEN** a producer serves geolocation records at a retained height
- **THEN** it uses the same generic transfer payload, parameterised by the geolocation record type, with no second bespoke struct and no duplicated identity-map encoding

### Requirement: HTTP exposure of produced attestations

A producer SHALL expose its produced data over HTTP: the settle-lagged latest signed snapshot, a signed snapshot at a specific retained height, and the full verified record set at a retained height. Responses SHALL carry the canonical signed structures defined by `nym-contract-attestation` so a client can verify signatures and recompute hashes. Each attested contract SHALL be served under its own route tree rather than through a domain-parameterised path, so the existing directory routes are not reshaped and each tree's schema stays specific to its record type.

#### Scenario: Client fetches latest and a specific height

- **WHEN** a client requests the latest snapshot and then a snapshot at a specific retained height
- **THEN** the producer returns the corresponding `SignedDigestSnapshot`s, verifiable against the producer's identity key

#### Scenario: Geolocation is served under a parallel tree

- **WHEN** a geolocation producer is added
- **THEN** it exposes its own route tree mirroring the directory's shape, and the directory routes are unchanged

### Requirement: Canonical, replay-resistant attestation payloads

The bytes a producer signs SHALL be produced by the shared canonical encoders in `nym-contract-attestation`, identical to what the verifying client recomputes: the snapshot signing payload binds a domain tag, chain-id, contract, height, `app_hash`, `accumulator`, and `node_identities_hash`; a subset digest binds a distinct domain tag, chain-id, height, subset identifier, and a hash over the subset's canonical bytes. Distinct domain tags SHALL keep snapshot signatures, subset-digest signatures, and node-entry signatures mutually non-interchangeable. A signature SHALL bind chain-id (and, for snapshots, contract) so it cannot be replayed across chains or contract instances.

The signing-payload bytes SHALL NOT change with the crate rename or with the rename of the snapshot's contract field. Both are source-level renames, so a signature produced before them SHALL still verify afterwards, and the domain tag's historical spelling is retained for that reason.

#### Scenario: Producer and client agree on the bytes

- **WHEN** a producer signs a snapshot (or a subset digest) and a client recomputes the signing payload from the same fields
- **THEN** the byte encodings are identical and the signature verifies

#### Scenario: Signature domains do not cross

- **WHEN** a signature is produced over a subset digest
- **THEN** it cannot be interpreted as a valid snapshot signature or a valid node-entry signature, and vice versa

#### Scenario: An existing signature survives the renames

- **WHEN** a snapshot signed before the crate and field renames is verified afterwards
- **THEN** it still verifies, because the signing payload is byte-for-byte unchanged

## ADDED Requirements

### Requirement: The snapshot names its contract domain-neutrally

`DigestSnapshot` SHALL carry the attested contract under a domain-neutral field name rather than a directory-specific one, since the same type attests every contract. No per-domain signing-payload domain tag is required, because the contract address is already bound into the payload and therefore prevents a snapshot for one contract being accepted for another.

#### Scenario: One snapshot type, two contracts

- **WHEN** a producer signs a snapshot for the directory contract and another for the geolocation contract
- **THEN** both are the same type, distinguished by the contract address bound into the signing payload

#### Scenario: Cross-contract replay is rejected

- **WHEN** a verifier expecting a geolocation snapshot is given a validly signed directory snapshot at the same height from a trusted signer
- **THEN** it rejects it on the contract mismatch, before counting it towards quorum

### Requirement: Attested contracts share a retained-height window

Every attested contract served by one producer SHALL use the same snapshot cadence and the same retained-height window, so a single height serves all of them and a consumer joining two contracts' data can pin both to one height.

#### Scenario: A consumer joins two trees at one height

- **WHEN** a consumer holds a directory snapshot at retained height `H` and requests geolocation at `H`
- **THEN** the producer retains and serves that height for both, so the join needs no reconciliation across differing heights

#### Scenario: Cadence divergence is not permitted

- **WHEN** a producer is configured with a snapshot cadence
- **THEN** that cadence and its retained window apply to every contract it attests, rather than being set per contract

## ADDED Requirements

### Requirement: HTTP attestation source

The crate SHALL provide a concrete HTTP `AttestationSource` that fetches signed snapshots from a nym-api producer over HTTP, so `AttestedTrustAnchor` can reach quorum against real producers rather than only a mock. It SHALL fetch the producer's latest signed snapshot and a signed snapshot at a specific height, deserialize them into the canonical `SignedDigestSnapshot`, and report its configured source identity. Transport or decode failures SHALL surface as a typed error rather than being silently treated as an absent snapshot.

#### Scenario: Anchor reaches quorum over HTTP sources

- **WHEN** an `AttestedTrustAnchor` is configured with HTTP attestation sources pointing at K trusted producers that agree on a snapshot
- **THEN** the anchor fetches their signed snapshots over HTTP and reaches quorum exactly as it does with in-memory sources

#### Scenario: Transport failure is typed

- **WHEN** a producer is unreachable or returns undecodable data
- **THEN** the source returns a typed transport error, distinct from a validly-signed absent-snapshot response, and the anchor treats it as a non-answer rather than a false result

### Requirement: Signed subset quorum retrieval

The crate SHALL provide a client path to retrieve a canonical subset under quorum trust: reach a K-of-N quorum of trusted signers on a subset's committed hash (`SignedSubsetDigest`) at a height, then fetch the subset data (`AttestedSubset<T>`) from a single source and accept it only if the locally recomputed hash over its canonical bytes equals both the fetched digest's hash and the quorum-agreed hash. A `SignedSubsetDigest` embedded in a fetched `AttestedSubset<T>` SHALL count as at most one quorum candidate, with no elevated trust for being the data-serving source. Any mismatch SHALL fail closed.

#### Scenario: Subset accepted under quorum with data from one source

- **WHEN** K distinct trusted signers agree on a subset's hash at height `H` and the client fetches the subset data from one source whose canonical bytes recompute to that hash
- **THEN** the client returns the subset data

#### Scenario: Tampered or sub-quorum subset rejected

- **WHEN** the fetched subset data does not recompute to the quorum-agreed hash, or fewer than K distinct trusted signers agree on any single hash
- **THEN** the client returns an error and no subset data

### Requirement: Whole-directory retrieval from a nym-api

The crate SHALL provide a way to fetch a whole directory (entries and the node-identity mapping) from a nym-api producer over HTTP and verify it, with no chain RPC connection, against an `AttestedTrustAnchor`'s quorum-attested `accumulator` and `node_identities_hash` for that height (via the existing offline verification path). This SHALL use the values committed by the snapshot rather than the generic subset mechanism.

#### Scenario: No-RPC client verifies a directory fetched from a nym-api

- **WHEN** a client fetches the full directory at a retained height `H` from a nym-api producer and holds the `AttestedTrustAnchor`'s quorum-attested snapshot for `H`
- **THEN** it verifies the entries against the `accumulator` and the node identities against the `node_identities_hash` by local recompute alone, returning the same `VerifiedDirectory` shape as the RPC-backed path

#### Scenario: Mismatch fails closed

- **WHEN** the directory fetched from the nym-api does not recompute to the attested `accumulator` (or the node identities do not match the attested hash)
- **THEN** the client returns a verification error and no directory

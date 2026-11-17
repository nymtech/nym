# contract-trust-anchor Specification

## Purpose
TBD - created by archiving change geolocation-retrieval-client. Update Purpose after archive.
## Requirements
### Requirement: Domain-neutral trust-anchor trait

The crate SHALL define a `TrustAnchor` trait exposing `trusted_app_hash(height)` and `trusted_digest(height)`, naming no directory-specific or geolocation-specific type in its signature or in the types it returns. A `TrustedDigest` SHALL carry the height and the full `LtHash16` accumulator rather than its 32-byte collapse, so a verifying client compares accumulators directly.

#### Scenario: Trait is usable by an unrelated contract client

- **WHEN** a client for any CosmWasm contract that maintains an LtHash accumulator at a raw storage key depends on this crate
- **THEN** it can implement or consume `TrustAnchor` without depending on `nym-directory-client` or on any directory type

#### Scenario: Accumulator is returned, not its collapse

- **WHEN** an anchor returns a `TrustedDigest` for height `H`
- **THEN** it carries the full `LtHash16` accumulator, so a caller can compare it against a locally recomputed accumulator without first collapsing either side

### Requirement: Digest location is a parameter, not a constant

The trusted-digest fetch SHALL take the contract address and the raw digest storage key as arguments rather than reading either from a compiled-in per-domain constant. The raw key SHALL be reconstructed locally by the verifier and never taken from the response of the RPC serving the proof, so a malicious RPC cannot substitute a different key for the one the proof is checked against.

#### Scenario: Two contracts anchored by one implementation

- **WHEN** the same anchor implementation is constructed once for the directory contract and once for the geolocation contract
- **THEN** each returns a trusted digest for its own contract, with no code change and no per-domain constant consulted

#### Scenario: Key is reconstructed, not echoed

- **WHEN** an RPC returns a proof whose key differs from the one the verifier reconstructed
- **THEN** verification is performed against the locally reconstructed key and fails, rather than adopting the key the RPC supplied

### Requirement: Two-layer ICS23 wasm-store proof verification

The crate SHALL verify a raw contract-storage read against a trusted block `app_hash` by chaining the two proof operations a CosmWasm store produces: an `ics23:iavl` existence or non-existence proof binding the key to the wasm-store root, then an `ics23:simple` proof binding the wasm store to the `app_hash`. It SHALL expose membership, non-membership, and a presence check that discriminates the two by proof shape rather than by whether the read value is empty.

#### Scenario: Valid membership proof verifies

- **WHEN** both proof operations verify and the second chains to the trusted `app_hash`
- **THEN** the read value is accepted as the proven value at that height

#### Scenario: Tampered value is rejected

- **WHEN** the value differs from the one the proof commits
- **THEN** verification returns a typed proof error and no value

#### Scenario: Presence decided by proof shape

- **WHEN** a raw read returns an empty value under a valid membership proof
- **THEN** it is reported as present with an empty value, not as absent

### Requirement: Proven absence of the digest item is the empty accumulator

A contract writes its digest item only on its first entry mutation, so a contract with no entries has no digest item at all. A verified non-existence proof of the digest key SHALL be treated as the empty accumulator, mirroring the contract's own load-time default, rather than as a verification failure.

#### Scenario: Empty contract anchors successfully

- **WHEN** the digest key is proven absent at height `H` against the trusted `app_hash`
- **THEN** the anchor returns a `TrustedDigest` at `H` carrying the empty accumulator

### Requirement: The trusted app hash never comes from the RPC serving the proof

Every single-key verified read SHALL take the `app_hash` it checks against from the anchor's `trusted_app_hash`, never from a header re-fetched from the RPC that served the proof. Otherwise a malicious RPC could supply a self-consistent forged pair of header and proof. The proven anchor's own digest fetch SHALL route through `trusted_app_hash` for the same reason, so there is exactly one trust seam.

#### Scenario: Forged self-consistent pair is rejected

- **WHEN** an RPC serves a proof that verifies against an `app_hash` it also supplies, but not against the anchor's trusted `app_hash` for that height
- **THEN** verification fails

#### Scenario: CometBFT off-by-one is preserved

- **WHEN** an anchor establishes the `app_hash` committing state at height `H`
- **THEN** it reads it from the header at `H + 1`

### Requirement: Extraction is behaviour-preserving

The move of the proof machinery and the anchors out of `nym-directory-client` SHALL preserve behaviour. Every test that existed in `nym-directory-client` before the move SHALL still pass, unedited except for import paths and renamed identifiers.

#### Scenario: Directory tests survive the move

- **WHEN** the extraction is complete and the workspace test suite runs
- **THEN** every pre-existing `nym-directory-client` test passes with no change to its assertions, fixtures, or expected values

### Requirement: Core error taxonomy

The crate SHALL own the error variants that belong to anchoring and proving: proof failures, quorum failures, light-client verification failures, checkpoint failures, and unavailable chain state. Client crates SHALL wrap this error rather than redefining its variants, and no path SHALL return a success value carrying unverified data.

#### Scenario: A client crate surfaces an anchor failure

- **WHEN** a domain client's verified read fails because its anchor could not establish a trusted digest
- **THEN** the domain error wraps the core anchor error, preserving the specific cause rather than collapsing it into a generic failure

#### Scenario: No unverified success path

- **WHEN** any verification step fails
- **THEN** the call returns an error and no data, rather than returning data flagged as unverified

### Requirement: Whole-set verification is shared across contract clients

A contract's record type SHALL expose the leaf its contract folds into the accumulator through one `DigestLeaf` trait in `nym-contracts-common`, implemented in the record type's own common crate with that contract's frozen encoding. The anchor crate SHALL provide, generic over any `DigestLeaf` record type:
- recomputing the accumulator over a record set;
- checking a record set against a trusted accumulator;
- checking attested data (`SnapshotData`) against a trusted snapshot: its height label against the requested height, its node-identity map against the snapshot's `node_identities_hash`, then its records against the snapshot's accumulator;
- reading the mixnet bond set at a height as the `NodeId -> ed25519` identity map, with every page pinned to that height.

Every contract client SHALL use these rather than its own copies. Each check SHALL fail closed with an error and no records.

#### Scenario: A record type gains verification by implementing its leaf
- **WHEN** a contract's record type implements `DigestLeaf`
- **THEN** its record sets can be folded, checked against a trusted accumulator and checked against an attested snapshot with no client-specific verification code

#### Scenario: The fold is order-independent
- **WHEN** the same records are folded in two different orders
- **THEN** both yield the same accumulator, so the order pages arrive in cannot affect verification

#### Scenario: A tampered record set fails closed
- **WHEN** a record set has a record added, withheld or altered relative to the set the trusted accumulator commits
- **THEN** the check returns a digest-mismatch error

#### Scenario: Attested data labelled with another height is rejected
- **WHEN** attested data recomputes to the trusted values for height `H` but is labelled with another height
- **THEN** the check returns a height-mismatch error rather than accepting data stamped with a height the anchor never established

#### Scenario: A substituted identity map is rejected
- **WHEN** attested records recompute correctly but the served identity map does not hash to the snapshot's `node_identities_hash`
- **THEN** the check returns an identities-mismatch error

### Requirement: Every whole-set read path yields one verified record set

The anchor crate SHALL provide each way of reading a contract's whole record set at a height `H`, generic over any `DigestLeaf` record type, and each SHALL yield the same verified record set: the records, the bond set's identity map at `H` and the accumulator they matched, labelled with `H`. The paths are:
- through the chain RPC under any trust anchor: the anchor's trusted digest at `H`, then the records from a read pinned to `H`, run only once the digest is established, then the bond set at `H`, then the check of the records against the digest;
- offline, against a quorum-agreed snapshot the caller already holds: the attested-data check of the served data against that snapshot;
- through an attested anchor's sources (see the next requirement).

A contract client SHALL build its own result from this record set rather than implement a read path of its own.

#### Scenario: The RPC read checks the records against the anchor's digest
- **WHEN** the anchor trusts an accumulator at `H` and the pinned read returns records that recompute to it
- **THEN** the read yields those records with the identity map of the bonds at `H` and the trusted accumulator

#### Scenario: The RPC read rejects records the digest does not commit
- **WHEN** the pinned read returns records that do not recompute to the accumulator the anchor trusts at `H`
- **THEN** the read returns a digest-mismatch error and no records

#### Scenario: Nothing is read when trust cannot be established
- **WHEN** the anchor cannot establish a digest for `H`
- **THEN** the read returns that error without running the pinned read

#### Scenario: The offline read checks served data against a held snapshot
- **WHEN** a caller holds a quorum-agreed snapshot for `H` and data for `H` that recomputes to its accumulator and identity hash
- **THEN** the read yields that data with the snapshot's accumulator, and rejects data that is labelled with another height or carries another identity map

### Requirement: The attested anchor returns verified whole-set data from its sources

`AttestedTrustAnchor` SHALL provide a read of the whole record set at a height `H`, available whenever its sources' record type implements `DigestLeaf`. It SHALL take the quorum-agreed snapshot for `H`, then ask its sources for their data at `H` one at a time, returning the first set that passes the attested-data check. It SHALL skip a source that fails to answer or whose data fails the check, return the last failure when no source passes, and fail when no quorum-agreed snapshot exists for `H`. Only the structural check SHALL decide whether a source's data is accepted: the anchor does not interpret records, and checks a client applies afterwards SHALL NOT cause a source to be skipped.

#### Scenario: The first verifying source is returned
- **WHEN** a quorum agrees on the snapshot for `H` and a source serves data that passes the check
- **THEN** the anchor returns that source's records and identity map, together with the accumulator they verified against

#### Scenario: A source serving bad data is skipped
- **WHEN** the first source serves data that fails the check and the second serves data that passes
- **THEN** the anchor returns the second source's data

#### Scenario: No source verifies
- **WHEN** every source fails to answer or serves data that fails the check
- **THEN** the anchor returns the last failure and no records

#### Scenario: No quorum for the height
- **WHEN** no quorum-agreed snapshot exists for `H`
- **THEN** the anchor returns that error without fetching any data

### Requirement: Moving the existing clients onto the shared verification preserves their behaviour

Moving `nym-directory-client` and `nym-geolocation-client` onto the shared verification, the shared read paths, the shared bond read and the shared height-pinned paging SHALL preserve what they verify and return. Their client types, their RPC reads' signatures, their verified result types and their key helpers SHALL remain; their own verify, offline and attested functions SHALL give way to the shared read paths, each result type being built from the shared verified record set. Every contract client SHALL return the anchor crate's shared verification error rather than an error type of its own, so that each cause of failure, including a failure to establish trust and a contract address that is not configured, has exactly one variant. Every scenario the pre-existing tests in both crates covered SHALL stay covered: a domain property by a test in the client crate, with its expected values unchanged, and a shared check by the anchor crate's test of that check.

#### Scenario: Sibling scenarios stay covered after the move
- **WHEN** the move is complete and the client and anchor crates' test suites run
- **THEN** they pass, and every scenario the clients' pre-existing tests covered is covered by one of them with unchanged expected values

#### Scenario: A failure to establish trust has one variant
- **WHEN** a client cannot establish trust for `H`, on any of its paths
- **THEN** its error is the shared error's anchor variant, carrying the anchor failure

#### Scenario: Consumers compile against the shared error
- **WHEN** nym-api, nym-node-status-api and the checkpoint updater are built against the moved clients
- **THEN** they compile, with nym-api's directory refresher naming the shared error in place of the directory's own and reading the `app_hash` through the client's `anchor()`, and no other source change


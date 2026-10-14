## ADDED Requirements

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

### Requirement: The attested anchor returns verified whole-set data from its sources

`AttestedTrustAnchor` SHALL provide a read of the whole record set at a height `H`, available whenever its sources' record type implements `DigestLeaf`. It SHALL take the quorum-agreed snapshot for `H`, then ask its sources for their data at `H` one at a time, returning the first set that passes the attested-data check. It SHALL skip a source that fails to answer or whose data fails the check, return the last failure when no source passes, and fail when no quorum-agreed snapshot exists for `H`. Only the structural check SHALL decide whether a source's data is accepted: the anchor does not interpret records, and checks a client applies afterwards SHALL NOT cause a source to be skipped.

#### Scenario: The first verifying source is returned
- **WHEN** a quorum agrees on the snapshot for `H` and a source serves data that passes the check
- **THEN** the anchor returns that source's records and identity map, together with the trusted snapshot they verified against

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

Moving `nym-directory-client` and `nym-geolocation-client` onto the shared verification, the shared bond read and the shared height-pinned paging SHALL preserve what they verify and return. Their client types, verified result types, offline verification functions and key helpers SHALL remain. Every contract client SHALL return the anchor crate's shared verification error rather than an error type of its own, so that each cause of failure, including a failure to establish trust and a contract address that is not configured, has exactly one variant. Every pre-existing test in both crates that their signatures can still express SHALL keep its scenario and pass, changed only where it imports a function that moved, calls one that was folded into the shared steps, or asserts an error now spelled as a shared variant.

#### Scenario: Sibling tests survive the move
- **WHEN** the move is complete and both crates' test suites run
- **THEN** every pre-existing test passes with its scenario, fixtures and expected values unchanged, except the directory's check that the offline path rejects a missing node-identities hash, which its signature now rules out

#### Scenario: A failure to establish trust has one variant
- **WHEN** a client cannot establish trust for `H`, on any of its paths
- **THEN** its error is the shared error's anchor variant, carrying the anchor failure

#### Scenario: Consumers compile against the shared error
- **WHEN** nym-api, nym-node-status-api and the checkpoint updater are built against the moved clients
- **THEN** they compile, with nym-api's directory refresher naming the shared error in place of the directory's own and no other source change

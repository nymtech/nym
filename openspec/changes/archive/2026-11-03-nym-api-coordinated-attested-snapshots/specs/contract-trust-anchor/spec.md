## MODIFIED Requirements

### Requirement: Every whole-set read path yields one verified record set

The anchor crate SHALL provide each way of reading a contract's whole record set at a height `H`, generic over any `DigestLeaf` record type, and each SHALL yield the same verified record set: the records, the bond set's identity map at `H` and the accumulator they matched, labelled with `H`. The paths are:
- through the chain RPC under any trust anchor: the anchor's trusted digest at `H`, then the records from a read pinned to `H`, run only once the digest is established, then the check of the records against the digest, with the node-identity map the caller supplies. A contract client supplies the bond set it reads at `H` through its own query client; a producer attesting several contracts supplies the map it read once at `H` for all of them;
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

#### Scenario: The identity map comes from the caller's read
- **WHEN** a producer supplies the identity map it read at `H` to the chain reads of several contracts
- **THEN** each read yields its records with that map, and makes no bond read of its own

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
- **THEN** they compile, naming the shared error in place of a client's own, with no other source change

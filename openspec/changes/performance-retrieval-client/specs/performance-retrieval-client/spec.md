## ADDED Requirements

### Requirement: Verified whole-set performance retrieval

The client SHALL retrieve every entry the performance contract's `LatestPerformancePaged` query returns at a single block height `H` and, before returning anything, fold each entry's leaf into an empty `LtHash16` accumulator and compare the result to the accumulator a `TrustAnchor` trusts for `H`. On a mismatch it MUST return an error and MUST NOT return partial or unverified entries. It SHALL work unchanged under any `TrustAnchor` implementation: proven, light-client or attested. Entries SHALL be folded exactly as served, because the leaf commits the score itself, so verification reads neither the contract's weights nor its per-kind medians, and an entry without a score SHALL verify like any other.

#### Scenario: Successful verified retrieval
- **WHEN** the accumulator recomputed over every entry read at `H` equals the anchor's trusted accumulator for `H`
- **THEN** the client returns the entries keyed by node id, together with `H` and the verified accumulator

#### Scenario: A withheld entry fails closed
- **WHEN** the served set omits one committed entry
- **THEN** the client returns a digest-mismatch error and no entries

#### Scenario: An added entry fails closed
- **WHEN** the served set holds an entry the contract did not commit
- **THEN** the client returns a digest-mismatch error and no entries

#### Scenario: An altered score or source epoch fails closed
- **WHEN** a served entry differs from the committed one only in its epoch, or only in its score, including a served `0.8549` for a committed `0.85`
- **THEN** the client returns a digest-mismatch error and no entries

#### Scenario: An unscoreable entry verifies and is kept
- **WHEN** the committed set includes an entry with `score: None`
- **THEN** the set verifies and that entry is returned with no score

#### Scenario: A contract without submissions verifies as empty
- **WHEN** the digest key is proven absent at `H` and the enumeration at `H` is empty
- **THEN** the client returns an empty verified set, because a proven-absent digest is the empty accumulator

### Requirement: Single-block-height read consistency

The digest read and every page of the enumeration MUST be executed at the same block height `H`. `PagedPerformanceQueryClient::get_all_latest_performance` pages without pinning a height, so the validator client SHALL provide `PinnedPerformanceQueryClient::get_all_latest_performance_at_height`, which requests every page at `H`, follows `start_next_after` until it is `None`, fails when a page's cursor does not advance past the one it was requested with, and fails once the read exceeds the bound shared by the other height-pinned reads. The client SHALL enumerate only through that path.

#### Scenario: Pagination is pinned to one height
- **WHEN** the committed set spans several pages
- **THEN** every page request is issued at `H`, the height the trusted digest was established at

#### Scenario: A concurrent submission does not corrupt the result
- **WHEN** a monitor's submission lands at a height after `H` while pagination is in progress
- **THEN** the returned set still matches the accumulator committed at `H`, rather than failing verification because pages straddled the write

### Requirement: The verified set carries the bond set at the verified height

A verified result SHALL carry the `NodeId -> ed25519 identity` map of the mixnet contract's node bonds at `H`, including bonded nodes without a committed entry. Through the chain RPC it SHALL be read at `H`, with every page pinned to `H`, by the same shared read the directory and geolocation clients use. Offline it SHALL be the map served with the entries, accepted only if it hashes to the attested snapshot's `node_identities_hash`.

#### Scenario: Bonded nodes are carried whether or not they were measured
- **WHEN** nodes 7 and 8 are bonded at `H` and only node 7 has a committed entry
- **THEN** the verified result carries both identities and only node 7's entry

### Requirement: The routing view lists every node bonded at the verified height

The client SHALL derive a routing view from a verified result: every node in the bond set at `H`, each with its committed entry, which carries its score and source epoch, or with no entry when it has never been measured. A committed entry for a node absent from the bond set at `H` SHALL be left out of the view and SHALL remain in the verified result. The view SHALL carry `H`, and SHALL be a pure function of the verified result.

#### Scenario: A measured bonded node carries its newest entry
- **WHEN** node 7 is bonded at `H` and its committed entry has epoch 10 and score 85%
- **THEN** the view holds node 7 with score 85% and source epoch 10

#### Scenario: A bonded node never measured has no entry
- **WHEN** node 8 is bonded at `H` and has no committed entry
- **THEN** the view holds node 8 with no entry

#### Scenario: A committed entry without a bond is left out
- **WHEN** node 9 has a committed entry but is absent from the bond set at `H`
- **THEN** the view omits node 9, and the verified result still holds its entry

#### Scenario: An unscoreable entry stays in the view
- **WHEN** bonded node 7's committed entry has `score: None`
- **THEN** the view holds node 7 with that entry and no score

### Requirement: Staleness in the routing view is unbounded

The routing view SHALL NOT drop, discount or flag an entry for the age of its source epoch. During a measurement outage the network keeps operating on last known values, and unbonded nodes leave the committed set through the mixnet contract's unbond callback, so only a bonded node that stopped being measured can carry an old entry. Each entry carries its source epoch, so a consumer that needs a bound applies its own to the view.

#### Scenario: An old entry is served as is
- **WHEN** bonded node 7's committed entry is from an epoch arbitrarily far behind the current one
- **THEN** the view holds node 7 with that entry and its source epoch, unchanged

### Requirement: Attested retrieval without a chain RPC connection

The crate SHALL verify a served entry set and node-identity map against a quorum-attested `DigestSnapshot` for `H` by local recompute alone, with no chain RPC connection: the entries against the snapshot's accumulator and the identity map against its `node_identities_hash`. A mismatch in either SHALL fail closed. Through an `AttestedTrustAnchor` whose sources serve performance entries, it SHALL obtain the verified data at `H` through the anchor's shared read from its sources, and SHALL build the same verified result as the RPC path from it.

#### Scenario: A no-RPC client verifies a producer's set
- **WHEN** a client holds a quorum-attested snapshot for `H` and fetches the entries and node identities for `H` from a producer
- **THEN** it verifies the entries against the accumulator and the identities against the `node_identities_hash` by local recompute alone, and returns the verified result

#### Scenario: Tampered entries fail closed
- **WHEN** the fetched entries do not recompute to the attested accumulator
- **THEN** the client returns a digest-mismatch error and no entries

#### Scenario: A substituted identity map fails closed
- **WHEN** the fetched entries recompute correctly but the identity map does not hash to the attested `node_identities_hash`
- **THEN** the client returns an identities-mismatch error and no entries

#### Scenario: The attested read returns the same shape as the RPC path
- **WHEN** the attested read through an `AttestedTrustAnchor` verifies the data of a source at `H`
- **THEN** it returns a verified result carrying `H`, the attested accumulator, the entries keyed by node id and the identity map, from which the same routing view is derived

### Requirement: Height alignment with the directory

Every read SHALL take `H` from the caller, and the verified result and the routing view SHALL carry it, so a consumer can join the routing view with the directory and the mixnet bonds at one height rather than joining views taken at different heights.

#### Scenario: One height serves both
- **WHEN** a consumer holds a verified directory at `H`
- **THEN** it can request the verified performance set at the same `H`, and both results carry `H`

### Requirement: Proven single-entry reads are out of scope

A node's committed score depends on the weights in force at its source epoch, which the contract resolves by a range over a sparse map, and ICS23 proves the presence or absence of a key rather than the absence of keys within a range. The crate SHALL NOT offer a proven read of one node's score, and a node's entry SHALL be established only through the whole-set recompute.

#### Scenario: One node's score comes from the whole set
- **WHEN** a caller needs one node's verified score
- **THEN** the whole-set verified read is the only path that provides it

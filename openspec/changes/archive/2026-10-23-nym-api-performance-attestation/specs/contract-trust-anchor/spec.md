## MODIFIED Requirements

### Requirement: Domain-neutral trust-anchor trait

The crate SHALL define two traits, naming no directory-specific or geolocation-specific type in their signatures or in the types they return: `ChainAnchor`, exposing `trusted_app_hash(height)`, which concerns the chain only and names no contract; and `TrustAnchor: ChainAnchor`, exposing `trusted_digest(height)`, the digest of one contract. A `TrustedDigest` SHALL carry the height and the full `LtHash16` accumulator rather than its 32-byte collapse, so a verifying client compares accumulators directly.

#### Scenario: Trait is usable by an unrelated contract client

- **WHEN** a client for any CosmWasm contract that maintains an LtHash accumulator at a raw storage key depends on this crate
- **THEN** it can implement or consume `TrustAnchor` without depending on `nym-directory-client` or on any directory type

#### Scenario: Accumulator is returned, not its collapse

- **WHEN** an anchor returns a `TrustedDigest` for height `H`
- **THEN** it carries the full `LtHash16` accumulator, so a caller can compare it against a locally recomputed accumulator without first collapsing either side

#### Scenario: Every trust anchor is a chain anchor

- **WHEN** code needs only the trusted `app_hash` at a height, such as a single-entry proof or a producer signing the app hash
- **THEN** it can take any `ChainAnchor`, and every `TrustAnchor` qualifies

### Requirement: Digest location is a parameter, not a constant

The trusted-digest fetch SHALL take the contract address and the raw digest storage key as arguments rather than reading either from a compiled-in per-domain constant. The raw key SHALL be reconstructed locally by the verifier and never taken from the response of the RPC serving the proof, so a malicious RPC cannot substitute a different key for the one the proof is checked against.

#### Scenario: Two contracts anchored by one implementation

- **WHEN** a `ContractAnchor` is constructed once for the directory contract and once for the geolocation contract
- **THEN** each returns a trusted digest for its own contract, with no code change and no per-domain constant consulted

#### Scenario: Key is reconstructed, not echoed

- **WHEN** an RPC returns a proof whose key differs from the one the verifier reconstructed
- **THEN** verification is performed against the locally reconstructed key and fails, rather than adopting the key the RPC supplied

### Requirement: The trusted app hash never comes from the RPC serving the proof

Every single-key verified read SHALL take the `app_hash` it checks against from the anchor's `trusted_app_hash`, never from a header re-fetched from the RPC that served the proof. Otherwise a malicious RPC could supply a self-consistent forged pair of header and proof. A `ContractAnchor`'s digest fetch SHALL route through its chain anchor's `trusted_app_hash` for the same reason, so there is exactly one trust seam.

#### Scenario: Forged self-consistent pair is rejected

- **WHEN** an RPC serves a proof that verifies against an `app_hash` it also supplies, but not against the anchor's trusted `app_hash` for that height
- **THEN** verification fails

#### Scenario: CometBFT off-by-one is preserved

- **WHEN** an anchor establishes the `app_hash` committing state at height `H`
- **THEN** it reads it from the header at `H + 1`

## ADDED Requirements

### Requirement: One chain anchor serves every contract

The crate SHALL provide its proven and light-client anchors as chain anchors (`ProvenChainAnchor`, `LightClientAnchor`), which take no contract address or digest key, and a `ContractAnchor` that pairs any chain anchor with one contract's address and bare digest item key and implements `TrustAnchor` by proving that contract's digest item against the chain anchor's `app_hash`. One chain anchor instance SHALL be shareable by the contract anchors of several contracts. `AttestedTrustAnchor` SHALL implement both traits itself, since its `app_hash` and digest come from one contract's quorum-agreed snapshot.

#### Scenario: Two contracts share one chain anchor

- **WHEN** a caller builds contract anchors for the directory and performance contracts over one shared chain anchor and reads both at height `H`
- **THEN** both digests are proven against the `app_hash` the shared chain anchor trusts at `H`

#### Scenario: A chain anchor needs no contract

- **WHEN** a caller constructs a `ProvenChainAnchor` or a `LightClientAnchor`
- **THEN** it supplies no contract address and no digest key, and the anchor can serve any contract

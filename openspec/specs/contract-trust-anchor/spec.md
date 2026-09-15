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

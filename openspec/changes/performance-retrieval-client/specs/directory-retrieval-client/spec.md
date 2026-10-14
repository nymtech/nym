## MODIFIED Requirements

### Requirement: Fail closed on missing chain state
When the RPC cannot supply the block header / `app_hash`, or the retained state needed to prove the digest at `H`, the client MUST return a typed error and MUST NOT return unverified entries as if verified. The error variants covering anchoring and proving SHALL be owned by `nym-contract-anchor`, and the client SHALL return the anchor crate's shared verification error, which wraps them, rather than defining an error type of its own.

#### Scenario: State at H is unavailable
- **WHEN** the required state or header for `H` is pruned or otherwise unavailable from the RPC
- **THEN** the client returns a typed error and returns no unverified data

#### Scenario: Anchor failures keep their specific cause
- **WHEN** a verified read fails inside the anchor rather than inside directory-specific logic
- **THEN** the shared verification error wraps the core anchor error, preserving which anchoring step failed

### Requirement: Whole-directory retrieval without a chain RPC connection
When the configured anchor's trusted snapshot carries a node-identities hash (today, only `AttestedTrustAnchor`), the crate SHALL provide a way to verify a whole-directory fetch - entries and node identities alike - using only locally recomputed hashes, with no `CosmWasmClient` / chain RPC connection required. That path SHALL be the anchor crate's shared offline read, which takes the whole quorum-agreed snapshot, with a `VerifiedDirectory` built from its result. The existing RPC-backed `DirectoryClient::verified_directory` path SHALL remain available for all anchors and fetch exactly as before.

#### Scenario: Directory verified from data sourced without any chain connection
- **WHEN** a caller supplies directory entries and a node-identity mapping obtained from any source (not a chain RPC connection) alongside an `AttestedTrustAnchor`'s trusted snapshot for height `H`
- **THEN** the crate verifies both the entries (against the accumulator) and the node-identity mapping (against the node-identities hash) by local recompute alone, and returns the same `VerifiedDirectory` shape as the RPC-backed path

#### Scenario: Decoupled verification requires a node-identities hash
- **WHEN** a caller holds a trusted snapshot that carries no node-identities hash (e.g. from `ProvenTrustAnchor`, `LightClientAnchor`)
- **THEN** the decoupled verification path cannot be invoked, because it takes a quorum-agreed snapshot, which always carries the hash, so authorship verification is never skipped silently

#### Scenario: Existing RPC-backed retrieval is unaffected
- **WHEN** `DirectoryClient::verified_directory` is called as before, with any anchor
- **THEN** it fetches entries and node identities via the client's own chain connection exactly as it did previously, reporting failures through the shared verification error

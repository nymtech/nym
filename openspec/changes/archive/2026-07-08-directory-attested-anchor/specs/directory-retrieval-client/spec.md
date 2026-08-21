## ADDED Requirements

### Requirement: Attested anchor for keyless bootstrap
The crate SHALL provide `AttestedTrustAnchor` as a `DirectoryTrustAnchor` implementation that establishes the trusted `app_hash`, directory digest, and node-identity binding from a K-of-N quorum of configured nym-api identity keys, requiring no root key and no light-client checkpoint. It SHALL ship with a small, overridable default trust root. Deployments that cannot yet provision a light-client checkpoint MAY use it; `ProvenTrustAnchor` and `LightClientAnchor` remain available and unchanged.

#### Scenario: AttestedTrustAnchor satisfies DirectoryTrustAnchor
- **WHEN** `DirectoryClient` is constructed with an `AttestedTrustAnchor`
- **THEN** `verified_directory` and `verified_node_entry` / `verified_curated_entry` behave identically to the other anchors, with the sole difference that `trusted_app_hash` and `trusted_digest` are sourced from a signed-snapshot quorum instead of an RPC header or a light-client verification

#### Scenario: Whole-directory recompute still guards the attested digest
- **WHEN** `verified_directory(H)` runs against an `AttestedTrustAnchor` and the locally recomputed accumulator over the fetched entries does not equal the quorum-attested accumulator
- **THEN** the client returns a `DigestMismatch` error and no entries, so a false attested digest fails closed rather than being accepted

#### Scenario: Single-entry reads remain ICS23-proven
- **WHEN** `verified_node_entry` or `verified_curated_entry` is called against an `AttestedTrustAnchor`
- **THEN** the entry is still verified by an ICS23 membership proof against the quorum-attested `app_hash`, preserving the trustless per-entry path

### Requirement: Whole-directory retrieval without a chain RPC connection
When the configured anchor's trusted snapshot carries a node-identities hash (today, only `AttestedTrustAnchor`), the crate SHALL provide a way to verify a whole-directory fetch - entries and node identities alike - using only locally recomputed hashes, with no `CosmWasmClient` / chain RPC connection required. The existing RPC-backed `DirectoryClient::verified_directory` path SHALL remain available and behaviorally unchanged for all anchors.

#### Scenario: Directory verified from data sourced without any chain connection
- **WHEN** a caller supplies directory entries and a node-identity mapping obtained from any source (not a chain RPC connection) alongside an `AttestedTrustAnchor`'s trusted snapshot for height `H`
- **THEN** the crate verifies both the entries (against the accumulator) and the node-identity mapping (against the node-identities hash) by local recompute alone, and returns the same `VerifiedDirectory` shape as the RPC-backed path

#### Scenario: Decoupled verification fails closed without a node-identities hash
- **WHEN** the decoupled verification path is used with an anchor whose trusted snapshot does not carry a node-identities hash (e.g. `ProvenTrustAnchor`, `LightClientAnchor`)
- **THEN** it returns an error rather than skipping authorship verification silently

#### Scenario: Existing RPC-backed retrieval is unaffected
- **WHEN** `DirectoryClient::verified_directory` is called as before, with any anchor
- **THEN** it fetches entries and node identities via the client's own chain connection exactly as it did previously, with no observable behavior change

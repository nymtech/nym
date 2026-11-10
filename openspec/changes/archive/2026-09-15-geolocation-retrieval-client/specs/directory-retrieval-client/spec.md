## MODIFIED Requirements

### Requirement: Pluggable trust anchor
The trusted digest SHALL be produced by a trust-anchor abstraction, and the retrieval and verification core SHALL be independent of which anchor produced it, so alternative anchors (a nym-api quorum, a full light client) can be added later without changing the verify core. That abstraction SHALL be the domain-neutral `TrustAnchor` trait provided by `nym-contract-anchor` (formerly `DirectoryTrustAnchor`, defined in this crate), and `nym-directory-client` SHALL re-export it so its own public surface does not move for existing callers.

#### Scenario: Verify core is anchor-independent
- **WHEN** the verify core is given any anchor that yields a trusted digest at `H`
- **THEN** retrieval, recomputation, and comparison proceed identically regardless of anchor implementation

#### Scenario: Existing callers are unaffected by the move
- **WHEN** a caller that named `nym_directory_client::DirectoryTrustAnchor` is updated only for the rename to `TrustAnchor`
- **THEN** it compiles against the re-export without adding a direct dependency on `nym-contract-anchor`

### Requirement: Light-client anchor for production use
When compiled with the `light-client` feature on `nym-contract-anchor`, that crate SHALL provide `LightClientAnchor` as a `TrustAnchor` implementation that verifies block headers via the Tendermint light-client protocol before returning `trusted_app_hash`. Production deployments SHOULD use `LightClientAnchor` instead of `ProvenTrustAnchor`, which remains available for local-dev and test contexts. The checkpoint that seeds the anchor SHALL be obtained from the checkpoint-bootstrap layer (a root-signed datum from a hardcoded or well-known source, verified against the root key), rather than requiring the caller to supply a checkpoint out-of-band.

#### Scenario: LightClientAnchor satisfies TrustAnchor
- **WHEN** `DirectoryClient` is constructed with a `LightClientAnchor`
- **THEN** `verified_directory` and `verified_node_entry`/`verified_curated_entry` behave identically to the `ProvenTrustAnchor` path, with the sole difference that `trusted_app_hash` additionally verifies validator-set signatures before returning

#### Scenario: Production anchor is bootstrapped from a root-signed checkpoint
- **WHEN** a production client constructs a `LightClientAnchor`
- **THEN** the seed checkpoint is loaded and verified via the checkpoint-bootstrap layer, so no manually supplied checkpoint is required

#### Scenario: ProvenTrustAnchor remains available
- **WHEN** `nym-contract-anchor` is compiled without the `light-client` feature
- **THEN** `ProvenTrustAnchor` is available and `LightClientAnchor` is not

### Requirement: Attested anchor for keyless bootstrap
`nym-contract-anchor` SHALL provide `AttestedTrustAnchor` as a `TrustAnchor` implementation that establishes the trusted `app_hash`, contract digest, and node-identity binding from a K-of-N quorum of configured nym-api identity keys, requiring no root key and no light-client checkpoint. It SHALL ship with a small, overridable default trust root. Deployments that cannot yet provision a light-client checkpoint MAY use it; `ProvenTrustAnchor` and `LightClientAnchor` remain available and unchanged.

#### Scenario: AttestedTrustAnchor satisfies TrustAnchor
- **WHEN** `DirectoryClient` is constructed with an `AttestedTrustAnchor`
- **THEN** `verified_directory` and `verified_node_entry` / `verified_curated_entry` behave identically to the other anchors, with the sole difference that `trusted_app_hash` and `trusted_digest` are sourced from a signed-snapshot quorum instead of an RPC header or a light-client verification

#### Scenario: Whole-directory recompute still guards the attested digest
- **WHEN** `verified_directory(H)` runs against an `AttestedTrustAnchor` and the locally recomputed accumulator over the fetched entries does not equal the quorum-attested accumulator
- **THEN** the client returns a `DigestMismatch` error and no entries, so a false attested digest fails closed rather than being accepted

#### Scenario: Single-entry reads remain ICS23-proven
- **WHEN** `verified_node_entry` or `verified_curated_entry` is called against an `AttestedTrustAnchor`
- **THEN** the entry is still verified by an ICS23 membership proof against the quorum-attested `app_hash`, preserving the trustless per-entry path

### Requirement: Fail closed on missing chain state
When the RPC cannot supply the block header / `app_hash`, or the retained state needed to prove the digest at `H`, the client MUST return a typed error and MUST NOT return unverified entries as if verified. The error variants covering anchoring and proving SHALL be owned by `nym-contract-anchor` and wrapped by this crate's `DirectoryClientError`, rather than defined twice.

#### Scenario: State at H is unavailable
- **WHEN** the required state or header for `H` is pruned or otherwise unavailable from the RPC
- **THEN** the client returns a typed error and returns no unverified data

#### Scenario: Anchor failures keep their specific cause
- **WHEN** a verified read fails inside the anchor rather than inside directory-specific logic
- **THEN** `DirectoryClientError` wraps the core anchor error, preserving which anchoring step failed

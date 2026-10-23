## MODIFIED Requirements

### Requirement: Light-client anchor for production use
When compiled with the `light-client` feature on `nym-contract-anchor`, that crate SHALL provide `LightClientAnchor` as a `ChainAnchor` implementation that verifies block headers via the Tendermint light-client protocol before returning `trusted_app_hash`; a `ContractAnchor` over it is the `TrustAnchor` a client takes. Production deployments SHOULD use a light-client chain anchor instead of `ProvenChainAnchor`, which remains available for local-dev and test contexts. The checkpoint that seeds the anchor SHALL be obtained from the checkpoint-bootstrap layer (a root-signed datum from a hardcoded or well-known source, verified against the root key), rather than requiring the caller to supply a checkpoint out-of-band.

#### Scenario: A light-client contract anchor satisfies TrustAnchor
- **WHEN** `DirectoryClient` is constructed with a `ContractAnchor` over a `LightClientAnchor`
- **THEN** `verified_directory` and `verified_node_entry`/`verified_curated_entry` behave identically to the proven path, with the sole difference that `trusted_app_hash` additionally verifies validator-set signatures before returning

#### Scenario: Production anchor is bootstrapped from a root-signed checkpoint
- **WHEN** a production client constructs a `LightClientAnchor`
- **THEN** the seed checkpoint is loaded and verified via the checkpoint-bootstrap layer, so no manually supplied checkpoint is required

#### Scenario: ProvenChainAnchor remains available
- **WHEN** `nym-contract-anchor` is compiled without the `light-client` feature
- **THEN** `ProvenChainAnchor` is available and `LightClientAnchor` is not

### Requirement: Attested anchor for keyless bootstrap
`nym-contract-anchor` SHALL provide `AttestedTrustAnchor` as a `TrustAnchor` implementation that establishes the trusted `app_hash`, contract digest, and node-identity binding from a K-of-N quorum of configured nym-api identity keys, requiring no root key and no light-client checkpoint. It SHALL ship with a small, overridable default trust root. Deployments that cannot yet provision a light-client checkpoint MAY use it; contract anchors over `ProvenChainAnchor` and `LightClientAnchor` remain available and unchanged.

#### Scenario: AttestedTrustAnchor satisfies TrustAnchor
- **WHEN** `DirectoryClient` is constructed with an `AttestedTrustAnchor`
- **THEN** `verified_directory` and `verified_node_entry` / `verified_curated_entry` behave identically to the other anchors, with the sole difference that `trusted_app_hash` and `trusted_digest` are sourced from a signed-snapshot quorum instead of an RPC header or a light-client verification

#### Scenario: Whole-directory recompute still guards the attested digest
- **WHEN** `verified_directory(H)` runs against an `AttestedTrustAnchor` and the locally recomputed accumulator over the fetched entries does not equal the quorum-attested accumulator
- **THEN** the client returns a `DigestMismatch` error and no entries, so a false attested digest fails closed rather than being accepted

#### Scenario: Single-entry reads remain ICS23-proven
- **WHEN** `verified_node_entry` or `verified_curated_entry` is called against an `AttestedTrustAnchor`
- **THEN** the entry is still verified by an ICS23 membership proof against the quorum-attested `app_hash`, preserving the trustless per-entry path

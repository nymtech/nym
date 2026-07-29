## MODIFIED Requirements

### Requirement: Light-client anchor for production use
When compiled with the `light-client` feature, the crate SHALL provide `LightClientAnchor` as a `DirectoryTrustAnchor` implementation that verifies block headers via the Tendermint light-client protocol before returning `trusted_app_hash`. Production deployments SHOULD use `LightClientAnchor` instead of `ProvenTrustAnchor`, which remains available for local-dev and test contexts. The checkpoint that seeds the anchor SHALL be obtained from the checkpoint-bootstrap layer (a root-signed datum from a hardcoded or well-known source, verified against the root key), rather than requiring the caller to supply a checkpoint out-of-band.

#### Scenario: LightClientAnchor satisfies DirectoryTrustAnchor
- **WHEN** `DirectoryClient` is constructed with a `LightClientAnchor`
- **THEN** `verified_directory` and `verified_node_entry`/`verified_curated_entry` behave identically to the `ProvenTrustAnchor` path, with the sole difference that `trusted_app_hash` additionally verifies validator-set signatures before returning

#### Scenario: Production anchor is bootstrapped from a root-signed checkpoint
- **WHEN** a production client constructs a `LightClientAnchor`
- **THEN** the seed checkpoint is loaded and verified via the checkpoint-bootstrap layer, so no manually supplied checkpoint is required

#### Scenario: ProvenTrustAnchor remains available
- **WHEN** `nym-directory-client` is compiled without the `light-client` feature
- **THEN** `ProvenTrustAnchor` is available and `LightClientAnchor` is not

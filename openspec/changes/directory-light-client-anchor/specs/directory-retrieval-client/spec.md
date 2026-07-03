## ADDED Requirements

### Requirement: Light-client anchor for production use
When compiled with the `light-client` feature, the crate SHALL provide `LightClientAnchor` as a `DirectoryTrustAnchor` implementation that verifies block headers via the Tendermint light-client protocol before returning `trusted_app_hash`. Production deployments SHOULD use `LightClientAnchor` instead of `ProvenTrustAnchor`, which remains available for local-dev and test contexts.

#### Scenario: LightClientAnchor satisfies DirectoryTrustAnchor
- **WHEN** `DirectoryClient` is constructed with a `LightClientAnchor`
- **THEN** `verified_directory` and `verified_node_entry`/`verified_curated_entry` behave identically to the `ProvenTrustAnchor` path, with the sole difference that `trusted_app_hash` additionally verifies validator-set signatures before returning

#### Scenario: ProvenTrustAnchor remains available
- **WHEN** `nym-directory-client` is compiled without the `light-client` feature
- **THEN** `ProvenTrustAnchor` is available and `LightClientAnchor` is not

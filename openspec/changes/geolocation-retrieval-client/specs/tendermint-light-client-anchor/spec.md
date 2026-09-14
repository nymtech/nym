## MODIFIED Requirements

### Requirement: Feature-gated compilation
`LightClientAnchor` SHALL be compiled only when the `light-client` feature is enabled on `nym-contract-anchor`, the crate it now lives in. `ProvenTrustAnchor` and all other `nym-contract-anchor` functionality SHALL remain available without the feature. `nym-directory-client` SHALL forward the feature so that enabling `light-client` on it continues to make `LightClientAnchor` available to its callers.

#### Scenario: Crate builds without the feature
- **WHEN** `nym-contract-anchor` is compiled without `features = ["light-client"]`
- **THEN** the crate compiles and `ProvenTrustAnchor` is available; `LightClientAnchor` is not

#### Scenario: Crate builds with the feature
- **WHEN** `nym-contract-anchor` is compiled with `features = ["light-client"]`
- **THEN** `LightClientAnchor` is available alongside `ProvenTrustAnchor`

#### Scenario: Directory crate forwards the feature
- **WHEN** `nym-directory-client` is compiled with `features = ["light-client"]`
- **THEN** `LightClientAnchor` is available to its callers exactly as before the extraction

### Requirement: Verified-head persistence via a checkpoint store

`LightClientAnchor` SHALL support an optional, injected `CheckpointStore` that persists its light-client-verified head. When a store is supplied, the anchor SHALL write its advanced trusted head to the store so a subsequent process's stored provider (see the `directory-checkpoint-bootstrap` capability) can reseed from it. The persisted head SHALL NOT require its own root signature, because it was produced by verifying forward from a root-anchored seed; it is trusted at local-filesystem-integrity level. Reading and selecting the persisted head as the anchor's base checkpoint is the responsibility of the loader's ordered provider chain, not of the anchor. The store SHALL be a collaborator of the light-client anchor only and SHALL NOT be a method on the shared `TrustAnchor` trait.

#### Scenario: Store is not part of the shared trait

- **WHEN** a caller holds a value behind the `TrustAnchor` trait
- **THEN** no checkpoint-store method is reachable through it, since persistence is a light-client concern rather than a property of every anchor

## ADDED Requirements

### Requirement: The anchored contract is a construction parameter

`LightClientAnchor` SHALL take the contract it anchors as a construction parameter with a domain-neutral name, so one implementation serves the directory contract, the geolocation contract, and any later contract that maintains an LtHash accumulator at a raw storage key.

#### Scenario: One anchor type, two contracts

- **WHEN** a process constructs one `LightClientAnchor` for the directory contract and another for the geolocation contract
- **THEN** both are the same type, differing only in the contract address and digest storage key supplied at construction

#### Scenario: Below-head and bisection behaviour is unchanged

- **WHEN** an anchor constructed for a non-directory contract is asked for heights out of order, including at or below its current head
- **THEN** it walks and bisects exactly as the directory-anchored instance does, since the contract parameter affects only which digest is read

## ADDED Requirements

### Requirement: Verified-head persistence via a checkpoint store

`LightClientAnchor` SHALL support an optional, injected `CheckpointStore` that persists its light-client-verified head. When a store is supplied, the anchor SHALL write its advanced trusted head to the store so a subsequent process's stored provider (see the `directory-checkpoint-bootstrap` capability) can reseed from it. The persisted head SHALL NOT require its own root signature, because it was produced by verifying forward from a root-anchored seed; it is trusted at local-filesystem-integrity level. Reading and selecting the persisted head as the anchor's base checkpoint is the responsibility of the loader's ordered provider chain, not of the anchor. The store SHALL be a collaborator of the light-client anchor only and SHALL NOT be a method on the shared `DirectoryTrustAnchor` trait.

#### Scenario: Advanced head is persisted
- **WHEN** the anchor advances its trusted head and a store is present
- **THEN** the new head is written to the store so a later process can reseed from it

#### Scenario: Anchor without a store still functions
- **WHEN** no store is supplied
- **THEN** the anchor verifies and advances normally and simply does not persist its head

#### Scenario: Persisted head needs no root signature
- **WHEN** a stored head written by a prior process is later used to seed a new anchor
- **THEN** it is accepted on the basis of local-filesystem trust plus a staleness check, without requiring a root signature, because it was verified forward from a root-anchored seed

### Requirement: Production trusting period below the unbonding period

The production trusting period SHALL be 18 days, defined in a single location and reused by both the anchor's verification options and the checkpoint loader's staleness check so the two cannot diverge. The trusting period SHALL remain strictly below the chain's unbonding period with a safety margin; on nyx (21-day unbonding) 18 days leaves a 3-day margin. If the chain's unbonding period is shortened below this margin, the trusting period MUST be revised.

#### Scenario: Trusting period is below unbonding with margin
- **WHEN** the anchor and loader are compiled for nyx
- **THEN** both use an 18-day trusting period, which is below the 21-day unbonding period

#### Scenario: Anchor and loader share one trusting period
- **WHEN** the loader derives checkpoint staleness and the anchor verifies headers
- **THEN** both use the same trusting-period value from a single source

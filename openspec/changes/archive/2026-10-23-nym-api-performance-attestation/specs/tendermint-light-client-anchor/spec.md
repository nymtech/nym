## MODIFIED Requirements

### Requirement: Trusted checkpoint at construction
`LightClientAnchor` SHALL be constructed from a caller-supplied trusted checkpoint (`Checkpoint { height, signed_header, validators, next_validators }`) that represents a block the caller has verified through an out-of-band channel (e.g., a genesis-pinned block or an operator-attested recent block). The anchor SHALL NOT fetch or self-bootstrap the checkpoint from the RPC it is given at construction.

#### Scenario: Valid checkpoint initialises anchor
- **WHEN** a caller constructs `LightClientAnchor::new(client, checkpoint, options)`
- **THEN** the anchor stores the checkpoint's validator set as the initial trusted state and begins sequential verification from `checkpoint.height`

#### Scenario: No RPC call is made during construction
- **WHEN** `LightClientAnchor::new` is called
- **THEN** it does not make any network call; all RPC interaction is deferred to the first `trusted_app_hash` call

### Requirement: Trusted digest via anchor
A `ContractAnchor` over a `LightClientAnchor` SHALL establish the trusted digest at `H` by calling the light client's `trusted_app_hash(H)` and then verifying an ICS23 membership or non-membership proof for the contract's digest storage key against that `app_hash`; a proven-absent item (a contract with no digest yet) SHALL resolve to the empty accumulator. This is identical in structure to a `ContractAnchor` over a `ProvenChainAnchor` except the `app_hash` is header-verified.

#### Scenario: trusted_digest succeeds when header verifies
- **WHEN** the light client's `trusted_app_hash(H)` succeeds and the ICS23 proof of the digest item verifies against it
- **THEN** the contract anchor's `trusted_digest(H)` returns the proven `TrustedDigest` value

#### Scenario: trusted_digest fails when header does not verify
- **WHEN** the light client's `trusted_app_hash(H)` fails (header not adequately signed)
- **THEN** the contract anchor's `trusted_digest(H)` propagates the error

### Requirement: Feature-gated compilation
`LightClientAnchor` SHALL be compiled only when the `light-client` feature is enabled on `nym-contract-anchor`, the crate it now lives in. `ProvenChainAnchor`, `ContractAnchor` and all other `nym-contract-anchor` functionality SHALL remain available without the feature. `nym-directory-client` SHALL forward the feature so that enabling `light-client` on it continues to make `LightClientAnchor` available to its callers.

#### Scenario: Crate builds without the feature
- **WHEN** `nym-contract-anchor` is compiled without `features = ["light-client"]`
- **THEN** the crate compiles and `ProvenChainAnchor` is available; `LightClientAnchor` is not

#### Scenario: Crate builds with the feature
- **WHEN** `nym-contract-anchor` is compiled with `features = ["light-client"]`
- **THEN** `LightClientAnchor` is available alongside `ProvenChainAnchor`

#### Scenario: Directory crate forwards the feature
- **WHEN** `nym-directory-client` is compiled with `features = ["light-client"]`
- **THEN** `LightClientAnchor` is available to its callers exactly as before the extraction

### Requirement: Verified-head persistence via a checkpoint store

`LightClientAnchor` SHALL support an optional, injected `CheckpointStore` that persists its light-client-verified head. When a store is supplied, the anchor SHALL write its advanced trusted head to the store so a subsequent process's stored provider (see the `directory-checkpoint-bootstrap` capability) can reseed from it. The persisted head SHALL NOT require its own root signature, because it was produced by verifying forward from a root-anchored seed; it is trusted at local-filesystem-integrity level. Reading and selecting the persisted head as the anchor's base checkpoint is the responsibility of the loader's ordered provider chain, not of the anchor. The store SHALL be a collaborator of the light-client anchor only and SHALL NOT be a method on the shared `ChainAnchor` or `TrustAnchor` traits.

#### Scenario: Store is not part of the shared trait

- **WHEN** a caller holds a value behind the `ChainAnchor` or `TrustAnchor` trait
- **THEN** no checkpoint-store method is reachable through it, since persistence is a light-client concern rather than a property of every anchor

#### Scenario: Advanced head is persisted
- **WHEN** the anchor advances its trusted head and a store is present
- **THEN** the new head is written to the store so a later process can reseed from it

#### Scenario: Anchor without a store still functions
- **WHEN** no store is supplied
- **THEN** the anchor verifies and advances normally and simply does not persist its head

#### Scenario: Persisted head needs no root signature
- **WHEN** a stored head written by a prior process is later used to seed a new anchor
- **THEN** it is accepted on the basis of local-filesystem trust plus a staleness check, without requiring a root signature, because it was verified forward from a root-anchored seed

## REMOVED Requirements

### Requirement: The anchored contract is a construction parameter

**Reason**: `LightClientAnchor` is now a contract-free `ChainAnchor`: it takes no contract address or digest key, and a contract's digest is read through a `ContractAnchor` over it, so one light client serves every contract.

**Migration**: Construct `LightClientAnchor::new(client, checkpoint, options)` once and wrap it per contract in `ContractAnchor::new(chain, client, contract, digest_key)`, sharing it (for example behind an `Arc`) between contracts.

## ADDED Requirements

### Requirement: One light client serves every contract

`LightClientAnchor` SHALL take no contract address or digest key, so a single instance verifies headers for every contract a process reads through it. Its verified-`app_hash` cache SHALL serve every contract anchor sharing it, so a height verified for one contract is not re-verified for another.

#### Scenario: Two contracts share one light client

- **WHEN** a process wraps one `LightClientAnchor` in contract anchors for the directory and performance contracts and reads both at height `H`
- **THEN** the header at `H + 1` is verified once, and the second contract's read is served from the cache

#### Scenario: Below-head and bisection behaviour is unchanged

- **WHEN** the shared light client is asked for heights out of order, including at or below its current head
- **THEN** it walks and bisects exactly as an unshared instance does, since no contract affects header verification

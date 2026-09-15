## MODIFIED Requirements

### Requirement: Available in the default build
`AttestedTrustAnchor` SHALL compile in the default `nym-contract-anchor` build without any feature flag (it introduces no heavy dependency), and SHALL also compile when the `light-client` feature is enabled. `nym-directory-client` SHALL re-export it, so callers that reached it through that crate continue to compile.

#### Scenario: Present without any feature
- **WHEN** `nym-contract-anchor` is compiled with no extra features
- **THEN** `AttestedTrustAnchor` is available

#### Scenario: Reachable through the directory crate
- **WHEN** a caller names `AttestedTrustAnchor` through `nym-directory-client`
- **THEN** it resolves via the re-export without a direct dependency on `nym-contract-anchor`

### Requirement: Whole-directory verification requires no chain RPC connection
Given a quorum-agreed snapshot for height `H`, contract entries and the `NodeId -> ed25519 identity` mapping fetched from ANY source SHALL be verifiable by local hash recomputation alone (against the attested `accumulator` and `node_identities_hash` respectively), with no chain RPC connection required by the verifying party. This SHALL fail closed: a mismatch in either recomputed hash SHALL be treated as a verification failure, not partial success. This property SHALL hold for any contract the anchor is constructed for, not only the directory contract.

#### Scenario: Geolocation verified without a chain connection
- **WHEN** a client holds a quorum-agreed geolocation snapshot for `H` and fetches the geolocation record set and node identities from a producer
- **THEN** both are verifiable by local recompute alone, with no chain RPC connection

#### Scenario: Mismatch in either hash fails closed
- **WHEN** either the recomputed accumulator or the recomputed node-identities hash differs from the attested value
- **THEN** the verification fails entirely rather than returning the half that matched

## ADDED Requirements

### Requirement: The attested contract is a construction parameter

`AttestedTrustAnchor` SHALL take the contract it anchors as a construction parameter with a domain-neutral name. Cross-contract replay is already prevented because the contract address is bound into the snapshot signing payload, so one snapshot type and one anchor type serve both contracts without a per-domain signing-payload domain tag.

#### Scenario: A directory snapshot cannot anchor geolocation

- **WHEN** an anchor constructed for the geolocation contract is offered a validly signed snapshot naming the directory contract
- **THEN** it rejects the snapshot, because the contract bound into the signed payload does not match the one the anchor was constructed for

#### Scenario: Quorum rules are unchanged by the parameterisation

- **WHEN** an anchor constructed for any contract counts signatures towards quorum
- **THEN** it requires K distinct trusted signers agreeing on identical snapshot values, exactly as before

### Requirement: One trust root serves every attested contract

The compiled-in default trust root SHALL be a single list of nym-api attestation sources used for every attested contract, rather than one list per contract, because the same nym-apis sign every contract's snapshots from the same base URLs. The default quorum SHALL remain a function of that list's size rather than a separately hardcoded number.

#### Scenario: Growing the signer set moves every contract's quorum

- **WHEN** a third nym-api is added to the default source list
- **THEN** the default quorum moves from 2-of-2 to 2-of-3 for the directory and for geolocation alike, with no code change

#### Scenario: Sources are addressed per contract, not duplicated

- **WHEN** a consumer constructs anchors for two contracts
- **THEN** both draw their trusted signers from the same source list, differing only in which route each queries

# directory-attested-anchor Specification

## Purpose

Defines the requirements for `AttestedTrustAnchor`, a `DirectoryTrustAnchor` implementation that establishes the trusted block `app_hash`, directory digest, and node-identity binding from a K-of-N quorum of configured nym-api (Tier-1) identity keys. It replaces both the honest-RPC assumption of `ProvenTrustAnchor` and the checkpoint requirement of `LightClientAnchor` with a different trust root: a snapshot is accepted only when at least K distinct configured signers sign identical `(height, app_hash, accumulator, node_identities_hash)` values. It ships with a small, overridable default trust root, requires no root key and no light-client checkpoint, and lets whole-directory retrieval proceed with no direct chain RPC connection at all.

## Requirements

### Requirement: Quorum trust root configured out-of-band, with a default
`AttestedTrustAnchor` SHALL be constructed with a set of trusted nym-api ed25519 identity keys, a quorum threshold `K`, the expected chain-id, and the directory contract address. It SHALL ship with a small, hardcoded default trust root (Nym-SA-owned identity keys and a default `K`) usable with no caller-supplied configuration, and SHALL allow any caller to override that default with its own signer set and threshold. It SHALL NOT require a root key or a light-client checkpoint, and SHALL NOT fetch or self-bootstrap its trusted signer set from any untrusted source. Construction SHALL reject a configuration where `K` is zero or `K` exceeds the number of trusted signers.

#### Scenario: Valid configuration initialises the anchor
- **WHEN** a caller constructs `AttestedTrustAnchor::new(sources, trusted_signers, quorum, chain_id, contract)` with `1 <= quorum <= trusted_signers.len()`
- **THEN** the anchor stores the signer set, threshold, chain-id, and contract, and makes no network call during construction

#### Scenario: Default anchor requires no caller-supplied signer set
- **WHEN** a caller constructs the anchor via its default-anchor constructor, supplying only sources, chain-id, and contract
- **THEN** the anchor uses the compiled-in default `trusted_signers` and quorum threshold

#### Scenario: A caller can override the default trust root
- **WHEN** a caller constructs `AttestedTrustAnchor::new(...)` with its own `trusted_signers` and `quorum`, distinct from the compiled-in default
- **THEN** the anchor uses only the caller-supplied set, and the default is not consulted

#### Scenario: Degenerate quorum is rejected
- **WHEN** `new` is called with `quorum == 0` or `quorum > trusted_signers.len()`
- **THEN** it returns an `InvalidQuorumConfig` error and no anchor is constructed

### Requirement: Canonical, replay-resistant attestation format
The bytes a nym-api signs SHALL be produced by a shared canonical encoder (`digest_snapshot_signing_payload`) that binds a domain-separation tag, the chain-id, the contract address, the height, the `app_hash`, the digest `accumulator`, and a hash over the current `NodeId -> ed25519 identity` mapping (`node_identities_hash`), using length-prefixed framing so adjacent variable-length fields cannot be confused. The domain tag SHALL differ from the node-entry signing payload so a snapshot signature can never be interpreted as a node-entry signature. The producer and the client SHALL use the identical encoder for both the signing payload and the `node_identities_hash` itself.

#### Scenario: Payload is deterministic and field-sensitive
- **WHEN** the encoder is called twice with the same inputs
- **THEN** it returns identical bytes, and any change to chain-id, contract, height, app_hash, accumulator, or node_identities_hash produces different bytes

#### Scenario: Node-identity hash is deterministic and order-independent
- **WHEN** the node-identity hash encoder is called twice with the same `(NodeId, identity)` pairs presented in different iteration order
- **THEN** it returns identical bytes, and any change to the set of pairs produces a different hash

#### Scenario: Cross-chain or cross-contract replay is rejected
- **WHEN** a validly signed snapshot carries a chain-id or contract address other than the ones the anchor was configured with
- **THEN** the anchor treats that attestation as invalid and does not count it toward quorum

### Requirement: Quorum verification of a snapshot
`trusted_app_hash(H)` and `trusted_digest(H)` SHALL accept a snapshot only when at least `K` DISTINCT trusted signers produce valid signatures over identical `(height, app_hash, accumulator, node_identities_hash)` values. An attestation is valid only if its signer is in the configured trusted set, its chain-id and contract match, and its ed25519 signature verifies over the canonical payload. Distinctness SHALL be by signer key, so a repeated signer counts once. If no set of values reaches `K` distinct valid signers, the anchor SHALL return a `QuorumNotReached` error and MUST NOT return an `app_hash`, digest, or node-identities hash.

#### Scenario: K agreeing distinct signers are accepted
- **WHEN** at least `K` distinct trusted signers return valid attestations over the same `(height, app_hash, accumulator, node_identities_hash)`
- **THEN** the anchor accepts that snapshot and caches it

#### Scenario: Fewer than K agreeing signers are rejected
- **WHEN** fewer than `K` distinct trusted signers agree on the same values
- **THEN** the anchor returns `QuorumNotReached { needed, agreed }` and returns no trusted value

#### Scenario: A repeated signer counts once
- **WHEN** the same trusted signer's attestation is presented multiple times
- **THEN** it contributes at most one toward the quorum count

#### Scenario: Untrusted or invalid attestations are ignored
- **WHEN** an attestation is signed by a key not in the trusted set, carries an invalid signature, or binds a mismatched chain-id or contract
- **THEN** it is excluded from the quorum count rather than causing the whole verification to error

#### Scenario: Disagreeing signers do not form a quorum
- **WHEN** trusted signers return valid attestations but split across different `(app_hash, accumulator, node_identities_hash)` values so that no single value reaches `K`
- **THEN** the anchor returns `QuorumNotReached` and returns no trusted value

### Requirement: app_hash, digest, and node-identity hash from the same attestation
`trusted_app_hash(H)`, `trusted_digest(H)`, and the node-identities hash accessor SHALL return values drawn from the SAME quorum-agreed attestation for `H`, so they cannot disagree. `trusted_digest(H)` SHALL return the attested accumulator directly and SHALL NOT require an ICS23 proof, because the quorum attests the digest itself. The node-identities hash SHALL likewise be returned without any additional proof.

#### Scenario: Digest is returned without an ICS23 proof
- **WHEN** a quorum-agreed snapshot for `H` exists
- **THEN** `trusted_digest(H)` returns its `accumulator` without performing any store-membership proof

#### Scenario: app_hash, digest, and node-identity hash are drawn from one snapshot
- **WHEN** `trusted_app_hash(H)`, `trusted_digest(H)`, and the node-identities hash accessor are all called for the same `H`
- **THEN** all three are served from the one cached snapshot for `H`, so they cannot disagree

### Requirement: Latest snapshot discovery and retained-window heights
The anchor SHALL support discovering the quorum-agreed LATEST snapshot (`refresh` / `latest_snapshot_height`) by querying each source's latest attestation and reaching quorum. For a specific height `H`, `trusted_app_hash(H)` / `trusted_digest(H)` SHALL serve a cached snapshot for `H` if present, and otherwise fetch a per-height attestation from the quorum. A height the quorum cannot attest (outside the producers' retained window) SHALL return an error rather than any unverified value.

#### Scenario: Latest agreed snapshot is discovered and pinned
- **WHEN** `latest_snapshot_height()` (or `refresh()`) is called and a quorum agrees on a latest snapshot
- **THEN** the anchor caches it and returns its height, which the caller uses to drive `verified_directory`

#### Scenario: A recent past height within the window is verified
- **WHEN** `trusted_app_hash(H)` is called for a height `H` older than the latest but still served by the quorum
- **THEN** the anchor fetches and quorum-verifies the snapshot at `H` and returns its `app_hash`

#### Scenario: A height outside the retained window is rejected
- **WHEN** `trusted_app_hash(H)` is called for a height no quorum snapshot exists for
- **THEN** the anchor returns `NoQuorumSnapshotForHeight(H)` and no value

### Requirement: In-memory cache of quorum-agreed snapshots
Quorum-agreed snapshots SHALL be cached in memory within the anchor instance, keyed by height. Repeated calls for the same height within one process lifetime SHALL be served from cache without re-querying the sources.

#### Scenario: Repeated query uses the cache
- **WHEN** `trusted_app_hash(H)` is called twice for the same `H` in one session
- **THEN** the sources are queried only once; the second call returns from cache

### Requirement: Transport abstraction
Attestations SHALL be fetched through an `AttestationSource` abstraction (fetch latest / fetch at a height), so the anchor is independent of any particular transport and can be exercised with a mock source. The concrete HTTP transport and the nym-api producer endpoint are out of scope for this capability.

#### Scenario: Anchor operates against any AttestationSource
- **WHEN** the anchor is constructed with sources implementing `AttestationSource`
- **THEN** all attestation fetching goes through that trait, and a test mock can drive every path

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

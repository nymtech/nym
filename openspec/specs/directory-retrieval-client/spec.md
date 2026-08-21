# directory-retrieval-client Specification

## Purpose

TBD - created by archiving change directory-retrieval-client. Update Purpose after archive.

## Requirements

### Requirement: Verifiable whole-directory retrieval
The client SHALL retrieve the complete set of directory entries and, before returning them, verify that the set equals the entry set committed by the on-chain LtHash digest at a single block height. On any verification failure it MUST return an error and MUST NOT return partial or unverified entries as if verified.

#### Scenario: Successful verified retrieval
- **WHEN** the client reads the digest and all entries at height `H` and the locally recomputed digest equals the proven digest
- **THEN** it returns the entries together with `H` and the verified digest

#### Scenario: Tampered entry is rejected
- **WHEN** any returned entry differs from what the digest at `H` commits (so the recomputed digest does not match)
- **THEN** the client returns a verification error and no entries

### Requirement: Single-block-height read consistency
Every read that feeds one verification - the digest proof and every paginated `AllEntries` page - MUST be executed at one fixed block height `H`. The client MUST NOT combine entries read at different heights into a single verification.

#### Scenario: Pagination is pinned to one height
- **WHEN** `AllEntries` spans multiple pages
- **THEN** every page request is issued at `height = H`, the same height as the digest proof

#### Scenario: A concurrent write does not corrupt the result
- **WHEN** a write is committed at a height later than `H` while pagination is in progress
- **THEN** because all pages are read at `H`, the returned set still matches the digest committed at `H`

### Requirement: Digest integrity proof against the app hash
In proven mode the trusted digest SHALL be established by an ICS23 membership proof of the raw digest storage item, verified against the block `app_hash` for height `H` (read from `header[H+1]`). A proof that does not verify against that `app_hash` MUST cause rejection.

#### Scenario: Valid proof yields a trusted digest
- **WHEN** the ICS23 proof of the digest item verifies against the `app_hash` for `H`
- **THEN** the proven digest value is treated as trusted for `H`

#### Scenario: Forged or non-verifying proof is rejected
- **WHEN** the ICS23 proof does not verify against the `app_hash` for `H`
- **THEN** the client returns a verification error and does not treat the digest as trusted

#### Scenario: Wrong-height app hash is rejected
- **WHEN** the proof is checked against an `app_hash` from a height other than the one committing `H`
- **THEN** verification fails

### Requirement: Local digest recomputation
The client SHALL recompute the digest locally from the retrieved entries using the same canonical leaf encoding the contract uses, and SHALL accept the set only if the recomputed 32-byte digest exactly equals the proven digest.

#### Scenario: Recomputation matches the proven digest
- **WHEN** the LtHash recomputed over all retrieved entries equals the proven digest
- **THEN** the set is accepted as complete and untampered

#### Scenario: Recomputation differs from the proven digest
- **WHEN** the recomputed digest differs from the proven digest
- **THEN** the set is rejected

### Requirement: Pluggable trust anchor
The trusted digest SHALL be produced by a trust-anchor abstraction, and the retrieval and verification core SHALL be independent of which anchor produced it, so alternative anchors (a nym-api quorum, a full light client) can be added later without changing the verify core.

#### Scenario: Verify core is anchor-independent
- **WHEN** the verify core is given any anchor that yields a trusted digest at `H`
- **THEN** retrieval, recomputation, and comparison proceed identically regardless of anchor implementation

### Requirement: Node entry signature verification
For each node entry the client SHALL verify the stored ed25519 signature over `node_signing_payload(node_id, label, sequence, data)` against the node's identity key obtained from the mixnet bond. An entry whose signature does not verify MUST be surfaced as unauthenticated and MUST NOT be presented as node-authored.

#### Scenario: Valid node signature
- **WHEN** a node entry's signature verifies against the bonded node's identity key
- **THEN** the entry is marked as verified node-authored

#### Scenario: Invalid node signature
- **WHEN** a node entry's signature does not verify
- **THEN** the entry is flagged as unauthenticated rather than accepted as node-authored

#### Scenario: Curated entries are not signature-checked
- **WHEN** an entry is a curated entry
- **THEN** no per-entry node signature is required or checked (its authority is the contract admin)

### Requirement: Trust-tier classification
The client SHALL classify each verified entry by trust tier: node self-authored (signature-verified against the bonded node's identity key) versus admin-curated (authored by the contract admin, with no per-entry signature).

#### Scenario: Entries are labeled by tier
- **WHEN** the returned set contains both node and curated entries
- **THEN** each entry is labeled with its trust tier

### Requirement: Single-entry verified read
The client SHALL support retrieving and verifying a single entry - a node entry `(node_id, label)` or a curated entry `(key)` - via an ICS23 proof of that entry's raw storage key against the `app_hash` at `H` (obtained from the trust anchor, not re-fetched from the RPC serving the proof), decoding the value with the entry value codec. Presence versus absence MUST be decided by the proof (a membership proof versus a non-existence proof), not by whether the read value is empty. A node entry's signature is additionally checked against the bonded node's identity key; a curated entry carries no per-entry signature (its authority is the contract admin), so a verified membership proof is itself the authentication.

#### Scenario: Present node entry is proven
- **WHEN** the membership proof for a node entry's raw key verifies against the `app_hash` for `H`
- **THEN** the decoded entry is returned, with its signature-verification status against the bonded node's identity key

#### Scenario: Present curated entry is proven
- **WHEN** the membership proof for a curated entry's raw key verifies against the `app_hash` for `H`
- **THEN** the decoded curated payload is returned (no per-entry signature is required)

#### Scenario: Entry not present
- **WHEN** the entry does not exist at `H`
- **THEN** a verified non-existence proof causes the client to report it as absent, distinct from a verification failure

### Requirement: Partial publication is not tamper
The client SHALL verify over exactly the entry set the digest commits and MUST NOT treat bonded nodes that have not published entries as a verification failure.

#### Scenario: Some bonded nodes have not published
- **WHEN** only a subset of bonded nodes has entries in the directory at `H`
- **THEN** verification over that committed subset still succeeds

### Requirement: Fail closed on missing chain state
When the RPC cannot supply the block header / `app_hash`, or the retained state needed to prove the digest at `H`, the client MUST return a typed error and MUST NOT return unverified entries as if verified.

#### Scenario: State at H is unavailable
- **WHEN** the required state or header for `H` is pruned or otherwise unavailable from the RPC
- **THEN** the client returns a typed error and returns no unverified data

### Requirement: Light-client anchor for production use
When compiled with the `light-client` feature, the crate SHALL provide `LightClientAnchor` as a `DirectoryTrustAnchor` implementation that verifies block headers via the Tendermint light-client protocol before returning `trusted_app_hash`. Production deployments SHOULD use `LightClientAnchor` instead of `ProvenTrustAnchor`, which remains available for local-dev and test contexts.

#### Scenario: LightClientAnchor satisfies DirectoryTrustAnchor
- **WHEN** `DirectoryClient` is constructed with a `LightClientAnchor`
- **THEN** `verified_directory` and `verified_node_entry`/`verified_curated_entry` behave identically to the `ProvenTrustAnchor` path, with the sole difference that `trusted_app_hash` additionally verifies validator-set signatures before returning

#### Scenario: ProvenTrustAnchor remains available
- **WHEN** `nym-directory-client` is compiled without the `light-client` feature
- **THEN** `ProvenTrustAnchor` is available and `LightClientAnchor` is not

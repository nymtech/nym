# Verifiable retrieval of geolocation data

## Why

The geolocation contract commits its full entry set to an LtHash accumulator at a raw, ICS23-provable storage key, and was designed from the start so a client could verify what it reads. Nothing reads it that way yet: task 7.2 of the archived `verifiable-node-geolocation` change shipped without end-to-end client verification because `common/nym-directory-client` was not on develop at the time. It is now, along with its proven, light-client and attested trust anchors, so the blocker is gone.

Two consumers are waiting on it. nym-api is to serve attested geolocation to clients the way it already serves the directory, and node-status-api is to replace its own independent, metered ipinfo sweep with reads from the contract. Both need a verifying client first, and neither can be built sensibly until the trust anchors it rests on stop being directory-shaped.

## What Changes

- Extract the domain-neutral half of `nym-directory-client` into a new `common/nym-contract-anchor` crate: the ICS23 wasm-store proof verification, the trust-anchor trait and `TrustedDigest`, and the proven, light-client, checkpoint-bootstrap and attested anchors. Everything becomes generic over the contract address and the digest storage key rather than reaching for the directory's constants.
- **BREAKING** (source only, all consumers in-tree): `DirectoryTrustAnchor` becomes `TrustAnchor`, and `DirectoryClientError` splits into a core `AnchorError` plus per-client error types. `nym-directory-client` re-exports the moved items so its own public surface does not move.
- **BREAKING** (wire): rename `common/directory-attestation` to `common/nym-contract-attestation`, its `DigestSnapshot::directory_contract` field to `contract`, and `DirectorySnapshotData` to a generic `SnapshotData<R>`. `DigestSnapshot` is otherwise reusable unchanged for both contracts, because the contract address is already bound into the signing payload and so prevents cross-contract replay on its own.
- **BREAKING** (source): rename `nym_network_defaults::mainnet::DIRECTORY_ATTESTATION_SOURCES` to `CONTRACT_ATTESTATION_SOURCES`. The same nym-apis sign both contracts' snapshots from the same base URLs, so this is one trust root, not two.
- Add `common/nym-geolocation-client`: a whole-set verified read that recomputes the accumulator over every `GeolocationRecord::digest_leaf()` and compares it to a trusted digest, plus the attested source and HTTP transport, plus a pluggable resolution policy with a documented default.
- Fix a pagination gap the verified read depends on: `get_all_geolocation_records` uses `collect_paged!`, which pins no height, so paginated reads interleave writes and would produce a false digest mismatch.

Single-entry proven reads are deliberately out of scope. The geolocation entries key is `(subject_class, subject_id, source)`, so answering "where is node 42" is a prefix scan, and ICS23 proves membership and non-membership of a key rather than completeness over a range. Only the whole-set recompute can establish completeness for a subject, and a single-entry read could therefore only ever answer for a key the caller already knows in full.

## Capabilities

### New Capabilities

- `contract-trust-anchor`: the domain-neutral trust-anchor trait, `TrustedDigest`, ICS23 wasm-store membership, non-membership and presence verification, and the proven anchor, all generic over a contract address and a digest storage key.
- `geolocation-retrieval-client`: verified whole-set retrieval of geolocation records, per-entry-class verification semantics, and the resolution policy seam.

### Modified Capabilities

- `directory-retrieval-client`: the proof and trust-anchor requirements move out to `contract-trust-anchor`; the crate keeps its directory-specific client, verify core, keys, subsets and fetcher, and re-exports the moved items.
- `tendermint-light-client-anchor`: the anchor and its `light-client` feature gate move to `nym-contract-anchor`, and it becomes generic over the contract it anchors.
- `directory-attested-anchor`: moves to `nym-contract-anchor` and becomes generic over the contract, with the trust root and quorum rules unchanged.
- `directory-checkpoint-bootstrap`: the loader, provider chain and checkpoint store move to `nym-contract-anchor`.
- `directory-attestation-provider`: the crate rename, the `contract` field rename, the generic `SnapshotData<R>`, and the requirement that a geolocation producer reuse the directory's snapshot cadence and retained-height window so one height serves both.

## Impact

**New crates**: `common/nym-contract-anchor`, `common/nym-geolocation-client`.

**Renamed crate**: `common/directory-attestation` to `common/nym-contract-attestation`.

**Modified**: `common/nym-directory-client` loses roughly 1230 code lines to the extraction and keeps roughly 810. `common/client-libs/validator-client` gains height-pinned geolocation record pagination. `nym-network-defaults` gains the constant rename. `nym-api` names the renamed types in `src/directory/`, a mechanical fixup.

**Not in this change**: the nym-api geolocation producer, which will consume `nym-geolocation-client` the way `DirectoryDataProvider` consumes `nym-directory-client` today, and the node-status-api migration, which has its own deltas against `node-status-api-monitoring` and `node-status-api-http` and its own decisions recorded in `docs/geolocation/node-status-api-migration.md`.

**Risk concentrated in the extraction**: it is intended to be behaviour-preserving, and the guard is that every existing `nym-directory-client` test passes unchanged after the move. A test that needs editing means the move was not mechanical, which is worth discovering before anything is built on top of it.

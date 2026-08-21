## Why

The `directory-attested-anchor` change defined the *format* of a signed directory snapshot (`DigestSnapshot`, `digest_snapshot_signing_payload`, `node_identities_hash`) and the *consumer* (`AttestedTrustAnchor`, a K-of-N quorum `DirectoryTrustAnchor`), but explicitly deferred three things in its Non-Goals: the nym-api *producer* that computes and signs snapshots, a concrete HTTP `AttestationSource` for the client to fetch them, and any generalization beyond the whole-directory digest. Until those land, `AttestedTrustAnchor` can only talk to a mock - there is no real nym-api to reach quorum against - so attested mode is not deployable.

This change makes nym-apis actually produce the data the attested anchor needs, over HTTP, and adds generic scaffolding for future signed "subsets" of directory/node data (so a later change can publish, say, a stable keys+addresses view without re-solving the attestation protocol). It introduces a contract-dictated snapshot cadence so independent nym-apis converge on identical snapshot heights (a prerequisite for quorum), and lets a nym-api serve the whole verified directory so the prior change's no-RPC retrieval path (`verify_directory_offline`) finally has a real server to pull from.

Because nym-nodes will eventually expose the same shape of data at a lower trust tier, the producer logic is a signer-agnostic library (`nym-directory-attestation`) imported by nym-api now and reusable by nym-node later; the trust tier is entirely the client's choice of which signer set it accepts, not a property of the format or the library.

## What Changes

- **New crate `nym-directory-attestation`** (common): the shared attestation protocol. `DigestSnapshot` / `SignedDigestSnapshot` / `digest_snapshot_signing_payload` / `node_identities_hash` move here from `nym-directory-client` (they were `pub(crate)`; placement was deliberately punted in the prior change's D3). Adds the generic subset scaffolding - a `DirectorySubset` trait plus `SubsetDigest` / `SignedSubsetDigest` / `AttestedSubset<T>` - and a signer-agnostic producer core that builds and signs snapshots and subsets from pre-fetched inputs. Deliberately light on dependencies (no `nym-validator-client`), so both a producer (nym-api, later nym-node) and the verifying client can depend on it.
- **`nym-directory-contract` gains a snapshot cadence parameter**: `snapshot_interval` (in blocks) as a plain contract storage item, set at instantiate and mutable only by the admin, with a query to read it. It is deliberately NOT folded into the LtHash digest nor into the signed snapshot. Breaking change, acceptable since the contract is not yet deployed.
- **`nym-directory-client`** depends on the new crate (re-exporting the moved types so its public surface is unchanged), and gains: a concrete HTTP `AttestationSource` (`#4`), the client-side subset quorum-and-verify path, and a helper to fetch + verify a whole directory from a nym-api (feeding the existing `verify_directory_offline`). Adds the attestation-transport error variant the prior change deferred - there is finally a real call site.
- **`nym-api` becomes a producer**: it reads the interval from the contract and, at each cadence height, fetches and verifies the whole directory through a configurable `DirectoryTrustAnchor` (default `ProvenTrustAnchor` against its own RPC), computes and signs the `DigestSnapshot`, retains the last few cadence heights (config), and serves them over HTTP - the settle-lagged `latest`, a specific retained height, and the full verified directory at a retained height.

## Capabilities

### New Capabilities

- `directory-attestation-provider`: nym-apis (and, later, other signers) produce K-of-N-quorum-verifiable signed directory snapshots at a contract-dictated cadence, plus a generic mechanism for signing and quorum-verifying arbitrary canonical "subsets" of directory/node data. Signer-agnostic (a library), so the trust tier is the client's choice of signer set.

### Modified Capabilities

- `directory-contract`: gains an admin-managed `snapshot_interval` parameter (blocks between attestation snapshots), read by producers so independent nym-apis converge on identical snapshot heights.
- `directory-retrieval-client`: gains a concrete HTTP `AttestationSource` so `AttestedTrustAnchor` works against real nym-apis, a client-side path to quorum-verify and then fetch a signed subset, and a way to pull + verify a whole directory from a nym-api with no chain RPC connection.

## Impact

- **New**: `common/directory-attestation/` (package `nym-directory-attestation`). Light deps: `nym-crypto`, `serde`, `cosmrs`/`tendermint` (for `AppHash`/`Height`), `nym-lthash`, `blake3`, `nym-mixnet-contract-common` (for `NodeId`). No `nym-validator-client`.
- **`common/nym-directory-client/`**: `DigestSnapshot`/`SignedDigestSnapshot`/`digest_snapshot_signing_payload`/`node_identities_hash` removed and re-exported from the new crate; new `HttpAttestationSource`, subset quorum/verify helpers, whole-directory-from-nym-api fetch; new `AttestationTransport` error variant. `recompute_accumulator` / `verify_directory` / `verify_directory_offline` / `AttestedTrustAnchor` stay.
- **`common/cosmwasm-smart-contracts/directory-contract/`** and the contract crate: new `snapshot_interval` storage item, `SnapshotInterval` query, instantiate field, admin `UpdateSnapshotInterval` handler, migration; downstream instantiate wiring (`network-defaults`, contract-generator, localnet orchestrator, wallet if it constructs the instantiate msg).
- **`nym-api/`**: new producer module (config for retention count + settle-lag, a periodic producer task, retained-height store, the configurable source anchor), HTTP routes + `AppState` wiring + openapi; depends on `nym-directory-attestation` (and `nym-directory-client` for the source anchor / directory fetch).
- Consumers of `nym-directory-client` see no source change from the type move (re-exports preserve paths); `ProvenTrustAnchor` / `LightClientAnchor` / the `DirectoryTrustAnchor` trait are unchanged.

## Non-Goals

- **A concrete production subset** (keys+addresses, per-role slices, etc.). The subset mechanism is generic and exercised only against a dummy test subset here; the real subset's stable, quorum-reproducible projection is decided and wired in a follow-up. (Note the keys+addresses example raised during design would need a purpose-built struct anyway: `SkimmedNodeV1` carries nym-api-computed `performance`/`role` fields that differ across independent apis and would break byte-identical quorum.)
- **nym-node as a producer.** The library is signer-agnostic and ready; wiring nym-node's lower trust tier is a follow-up.
- **Contract-configured retention window / settle-lag.** Only the snapshot interval is on-chain (it is the one value that must be identical across apis); retention count and settle-lag are nym-api config for now, promotable to the contract later if convergence needs tightening.
- **Folding `node_identities_hash` into the generic subset mechanism.** It stays grandfathered inside `DigestSnapshot`; only *new* data rides the subset path.
- **Persisting produced snapshots across nym-api restarts.** Snapshots are recomputed from chain state after a restart.
- **The checkpoint / root-key bootstrap** (roadmap steps 1b/1c), still postponed until a root key exists.

## Why

The geolocation contract commits every node's measured, self-declared and overridden locations, together with the agent whitelist, to a digest, and `nym-geolocation-client` verifies that committed set at a height (`geolocation-retrieval-client`), which node-status-api already serves from on mainnet. But nothing produces attested snapshots of it, so a client without a chain connection has nothing to verify against, and the client's HTTP source has returned "not implemented" since #7156. With the directory and performance producers now sharing one snapshot mechanism in nym-api, geolocation is the third user of it, and the last contract whose data has to be attested before every fact about a node can be served to clients at one verified height.

## What Changes

- **Every nym-api produces attested geolocation snapshots.** At the directory contract's snapshot cadence, it reads the geolocation contract's whole committed set at the boundary height through `GeolocationClient` under the chain anchor nym-api shares across its attested contracts, signs a `DigestSnapshot` for it, retains the newest few, and serves them under `/v1/geolocation` (`/snapshot/latest`, `/snapshot/{height}`, `/entries/{height}/records`), mirroring `/v1/performance`. The records are served exactly as committed, the whitelisted-agent entries among them, since the contract folds both into one digest. The identity map at a height is the directory's, so clients take it from `/v1/directory/entries/{height}/node-identities`. The geolocation contract address is assumed present: a missing one fails startup, as the directory's and performance's do. Nothing beside the records is read or held: every geolocation payload is inside the digest leaf.
- **A raw verified read on the geolocation client.** `GeolocationClient` gains a read returning the verified record set at a height exactly as read, with the accumulator and the identity map, which the producer re-serves; the existing grouped read is built on it. The sibling clients are unchanged.
- **The HTTP attestation source for geolocation is implemented.** `NymApiGeolocationSource` fetches from the `/v1/geolocation` routes through new `NymApiClientExt` methods beside the directory's and performance's, assembling the snapshot data from every records page and the directory tree's identity map at the same height, so an `AttestedTrustAnchor` can reach quorum over nym-api producers. Its doc comment stops naming a route shape the deployed trees do not use.
- **Sandbox network defaults carry the deployed addresses.** The sandbox constants for the directory and geolocation contracts are filled from `envs/sandbox.env`, which is the deployed truth; the performance one stays empty until that contract reaches sandbox.
- **The merged node-status-api geolocation change is archived.** `node-status-api-geolocation-from-contract` merged in #7168 without being archived, so its deltas to `node-status-api-http` and `node-status-api-monitoring` were never applied to the main specs. This change archives it as its first task.
- **The sibling specs stop enumerating contracts.** `directory-attestation-provider` and `nym-api-startup` say "every attested contract" where they list the directory and performance today, so a further contract does not touch them again.

Not in this change: no resolved or per-node geolocation view (a later change adds `/v3/nym-nodes/geolocation/{node_id}` routes, default policy applied, with the full records on request); no contract change; no localnet wiring, where neither the directory nor the geolocation contract is deployed and nym-api already cannot start on this stack.

## Capabilities

### New Capabilities

- `geolocation-attestation-provider`: nym-api's producer of attested geolocation snapshots: always on, the directory's cadence and retained window, signing only what `GeolocationClient` verified with the whitelist entries in the record set, the `/v1/geolocation` route tree, and the persisted snapshot cache.

### Modified Capabilities

- `geolocation-retrieval-client`: a new requirement for the raw verified read a producer re-serves, and a new requirement for the HTTP attestation source over nym-api's `/v1/geolocation` routes, taking the identity map from the directory tree.
- `directory-attestation-provider`: the produce-only-what-was-verified requirement and the whole-directory serving requirement say "every attested contract" instead of naming the directory and performance.
- `nym-api-startup`: the startup requirement says every attested contract's snapshot producer runs its startup production beside the startup caches, instead of naming the directory's and performance's.

## Impact

**Code:**
- `nym-api/src/geolocation/`: the snapshot source over `GeolocationClient`, the `/v1/geolocation` handlers, mounted in `support/http/router.rs`; `ContractSnapshots` gains the geolocation state; `support/cli/run.rs` starts the producer beside the other two, persisting to its own cache file. New dependency on `nym-geolocation-client`.
- `nym-api/nym-api-requests`: the `/v1/geolocation` records response; the geolocation common crate with its `utoipa` feature.
- `common/cosmwasm-smart-contracts/geolocation-contract`: `ToSchema` derives for `GeolocationRecord` and the types inside it, behind the existing `utoipa` feature.
- `common/nym-geolocation-client`: the raw verified read; `http.rs` implemented with the tests the performance source has.
- `common/client-libs/validator-client`: `NymApiClientExt` geolocation methods beside the performance ones.
- `common/network-defaults`: the sandbox directory and geolocation constants.
- `docs/geolocation/README.md`: the producer in the implementation map and the status.
- `openspec/`: `node-status-api-geolocation-from-contract` archived, its two deltas applied to the main specs.

**APIs:** three new routes under `/v1/geolocation`. No contract, message or database change. The sibling route trees are unchanged.

**Deployment:** every nym-api reads the geolocation contract from this release on. The mainnet constant already carries its address and node-status-api already serves from the contract, so no operator action is needed there; sandbox nym-apis get the address from the filled constant or, as today, from their env. Localnet is unaffected in practice: its nym-api already fails startup for the missing directory address.

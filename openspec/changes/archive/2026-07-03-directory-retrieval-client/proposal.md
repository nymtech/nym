## Why

The `directory-contract` stores signed node/curated config on chain and maintains an O(1) LtHash integrity digest, but nothing reads or verifies it yet. Consumers still assemble the network view by pulling each node's HTTP API, which stays slow, fragile, and unverifiable until a client can retrieve the whole directory from chain and cryptographically confirm it is complete and untampered.

## What Changes

- Introduce a new `nym-directory-client` library (`common/directory-client`) that retrieves and verifies the directory from chain. Payload-agnostic: it verifies structure (digest, proofs, signatures), not the meaning of the opaque `data` blobs.
- **Proven whole-directory read** (the deliverable): obtain the on-chain 32-byte LtHash digest with an ICS23 membership proof of the raw digest `Item` verified against the block `app_hash`, fetch all entries via the (unproven) `AllEntries` smart query, recompute the LtHash locally, and reject unless the recomputed digest equals the proven one. One proof plus a local recompute authenticates the entire set, because the digest commits every entry.
- **Single-height consistency**: the digest proof and every paginated `AllEntries` page MUST be read at one fixed block height `H`. `AllEntries` is paginated across multiple queries, so reading at "latest" would interleave entries from before and after a concurrent write and recombine them into a set that matches no single committed digest - a false mismatch (or a masked real one). All reads pin `height = H`.
- **Node-signature verification**: verify each node entry's ed25519 signature over `node_signing_payload(node_id, label, sequence, data)` against the node's identity key (cross-queried from the mixnet bond), and classify each entry by tier (node self-authored vs admin-curated).
- **Trust-anchor abstraction**: a `DirectoryTrustAnchor` seam that yields a trusted digest at a height. The proven implementation derives it from the ICS23 proof against an `app_hash` fetched from a configured RPC (phase 1a). The attested mode (a nym-api quorum) and a full tendermint light client are future implementations behind the same seam.
- **Single-node verified read**: an IAVL membership proof for one entry's raw storage key, decoded via the entry value codec.

## Capabilities

### New Capabilities
- `directory-retrieval-client`: a verifiable retrieval client for the directory contract - whole-directory and single-node reads whose integrity is checked against the on-chain LtHash digest (proven mode: ICS23 proof against `app_hash`), with node ed25519 signature verification, trust-tier classification, and a pluggable trust anchor.

### Modified Capabilities
None. The `directory-contract` capability is consumed unchanged (no contract requirement changes). nym-api / nym-node integration and the producer (node push) are separate future changes with no OpenSpec spec today.

## Impact

- New crate `common/directory-client` (`nym-directory-client`). Reuses (no new authoring): `nym-directory-contract-common` (entry types + `digest_leaf` + `node_signing_payload`), `nym-lthash` (`LtHash16`), `nym-validator-client` (`DirectoryQueryClient` / `get_all_directory_entries` / `get_digest`), `nym-mixnet-contract-common` (`MixnetContractQuerier` for identity keys), `nym-crypto` (ed25519 verify), `ics23 0.12` (`verify_membership`, `host-functions` feature), and `tendermint-rpc` `abci_query(..., prove=true)`.
- New build in the client crate: an ICS23 two-layer store-proof wiring (IAVL key -> wasm-store root, then simple-merkle wasm-store -> `app_hash`, with the `app_hash` at `header[H+1]` off-by-one) and a wasm raw-key builder (`0x03 || len-prefix(canonical addr) || contract_key`).
- New shared work in `nym-validator-client`: (a) a proof-carrying raw-store query - `abci_query(..., prove=true)` for a raw key at `H`, surfacing the value + `ProofOps` + response height (the plumbing exists but `prove=true` is unused and not exposed); and (b) a height-pinned smart query - `query_contract_smart` currently targets latest, so a height-parameterised variant is needed to pin every `AllEntries` page to `H`.
- Operational: the proven path needs an RPC that retained state at the proof height (archival or a recent height). No contract or on-chain changes.

## Out of scope (future changes)

- A full tendermint light client (pinned checkpoint + validator-set verification) replacing the trusted-RPC `app_hash` (phase 1b), behind the same trust-anchor seam.
- Attested mode (trust a K-of-N nym-api quorum), which additionally needs nym-api to serve a signed digest snapshot (phase 2).
- The producer side (nodes pushing entries) and its label/payload taxonomy.
- HTTP-pull fallback for nodes that have not published yet: acting on unpublished nodes means knowing the exact content they would publish and how to map each node's HTTP shape onto a directory entry (the per-label payload taxonomy, still undecided), so it is a consumer-integration concern, not part of this payload-agnostic verifier.
- nym-api / node-status / client integration that consumes this library.

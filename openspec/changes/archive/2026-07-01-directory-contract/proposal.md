## Why

nym-nodes today publish their configuration (identity and sphinx public keys, IP addresses, advertised roles, and similar metadata) only over a per-node HTTP API. Every consumer (nym-api, the node-status API, potentially clients) must reach out to each node individually to assemble a network view. That is slow, fragile (a transiently unreachable node drops out of the view), and unverifiable (no proof the returned data was not tampered with in transit). We want a single, chain-anchored, verifiable source: nodes (and a small set of curated entities such as nym-apis) push signed data to a contract, and consumers read and cryptographically verify it.

## What Changes

- Introduce a new CosmWasm `directory-contract` that stores opaque, deterministically-encoded bytes under two key-classes, each in its own store:
  - **node entries** keyed `(node_id, label)`, authored by the node itself - authorized by an ed25519 signature from the node's identity key (fetched from the mixnet bond) bound to a per-node monotonic sequence;
  - **curated entries** keyed by a single admin-chosen path `String` (no label/suffix structure - the admin picks a sensible path), managed by the contract admin (governance) - e.g. nym-api identity keys and endpoints - so off-chain aggregators are authenticatable on-chain.
- Invert distribution from PULL to PUSH. Writes are authorized by the signature, so **anyone may relay** the transaction (the tx sender is unchecked).
- Maintain a single **global incremental multiset digest** (LtHash16, ~2 KB) over all entries in contract state (each leaf commits the entry's key and value - for node entries including the `signature` and `sequence`, so authorship is committed, not just content), so a consumer can verify it holds the complete, untampered directory at a given height with a single chain proof.
- Manage an **admin-mutable label whitelist** with a per-label maximum byte size (hard ceiling 128 KiB). `RemoveLabel` is non-destructive (blocks new writes only; never cascade-deletes).
- Lifecycle: writes require the node **bonded and not unbonding**; on unbond the mixnet contract notifies the directory via a **hard callback** that deletes that node's entries (bounded; not reply-on-error, so a directory fault fails the unbond - the directory contract is a required part of every deployment).
- Encoding: entry values use a compact hand-rolled raw-bytes codec (`to_bytes`/`try_from_bytes`), not JSON/base64, while keys are handled by cw-storage-plus (two per-class stores, the `StoredDeposits` pattern). The digest hashes a separate canonical leaf (class tag + key + committed value), not the stored value bytes, so it stays independent of storage layout. The opaque payload inside `data` is a consumer concern - `prost` (derive macro, no `.proto`/`protoc`) with `BTreeMap` map fields is recommended there for deterministic, forward-compatible payloads.
- Persist the ed25519 signature **and the sequence** alongside each node entry and commit both to the digest, so each entry is self-authenticating and the whole directory is auditable from current state alone (not merely an offline-verification bonus).
- Out of scope for this change (separate future work): the retrieval/verification client (light-client bootstrap, root key, paranoid vs normal routes, cross-check). There is **no merkle tree** and **no per-node digest** in v1.

## Capabilities

### New Capabilities
- `directory-contract`: a signed, push-based key-value directory of node-published data (keyed `(node_id, label)`) and admin-curated data (keyed by a path string) over opaque deterministic bytes, with identity-key write authorization, per-node replay protection, an admin label whitelist with per-label sizes, self-cleaning lifecycle via a mixnet unbond callback, and a global on-chain integrity digest enabling verifiable whole-directory retrieval.

### Modified Capabilities
None as OpenSpec specs - no `mixnet-contract` capability spec exists. The mixnet contract does require a code change (a directory address in state, a queued migration, and a hard unbond sub-message); this is captured under Impact and in tasks, and the directory side of that callback is a requirement within `directory-contract`.

## Impact

- New crates mirroring the node-families layout: `contracts/directory` (the contract) and `common/cosmwasm-smart-contracts/directory-contract` (shared message/type definitions).
- **Mixnet contract code change** (no existing spec): add `directory_contract_address` to `State` + `InstantiateMsg` + a queued migration; in the unbond handler emit a hard `OnNymNodeUnbond { node_id }` sub-message (`.add_message`, not reply-on-error - a directory fault fails the unbond). The directory exposes an `OnNymNodeUnbond` handler gated by an `UnauthorisedMixnetCallback` check (sender must be the mixnet contract).
- Cross-queries the mixnet contract (`MixnetContractQuerier`) for node existence (bonded and not unbonding) and the node's ed25519 identity key (stored base58; decode to the raw 32 bytes that `ed25519_verify` expects).
- New dependencies: the in-house `nym-lthash` crate at `common/lthash` (a generic multiset hash; `blake3` XOF via `digest 0.10`, `sha3 0.10` as a dev-only test XOF) for the digest, and `prost` (derive-only). Feasibility confirmed by building a contract that computes a blake3-backed LtHash and passing `cosmwasm-check`. Stays on the workspace's digest 0.10 (no workspace-wide upgrade).
- Downstream (separate changes): nym-node gains a push client; nym-api and the node-status API gain a read path; the verification client also cross-references the coconut-DKG contract for the core nym-api signer set.

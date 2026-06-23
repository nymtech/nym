## Why

nym-nodes today publish their configuration (identity and sphinx public keys, IP addresses, advertised roles, and similar metadata) only over a per-node HTTP API. Every consumer (nym-api, the node-status API, potentially clients) must reach out to each node individually to assemble a network view. That is slow, fragile (a transiently unreachable node drops out of the view), and unverifiable (no proof the returned data was not tampered with in transit). We want a single, chain-anchored, verifiable source: nodes (and a small set of curated entities such as nym-apis) push signed data to a contract, and consumers read and cryptographically verify it.

## What Changes

- Introduce a new CosmWasm `directory-contract` that stores opaque, deterministically-encoded bytes in a key-value map under two key-classes:
  - **node entries** keyed `(node_id, label)`, authored by the node itself - authorized by an ed25519 signature from the node's identity key (fetched from the mixnet bond) bound to a per-node monotonic sequence;
  - **curated entries** keyed `(curated_id, label)`, managed by the contract admin (governance) - e.g. nym-api identity keys and endpoints - so off-chain aggregators are authenticatable on-chain.
- Invert distribution from PULL to PUSH. Writes are authorized by the signature, so **anyone may relay** the transaction (the tx sender is unchecked).
- Maintain a single **global incremental multiset digest** (LtHash16, ~2 KB) over all entries in contract state (`leaf = canonical(key, value)`), so a consumer can verify it holds the complete, untampered directory at a given height with a single chain proof.
- Manage an **admin-mutable label whitelist** with a per-label maximum byte size (hard ceiling 128 KiB). `RemoveLabel` is non-destructive (blocks new writes only; never cascade-deletes).
- Lifecycle: writes require the node **bonded and not unbonding**; on unbond the mixnet contract notifies the directory via a **best-effort callback** that deletes that node's entries (bounded; reply-on-error so a directory fault cannot block unbonding).
- Deterministic encoding: `prost` (derive macro, no `.proto`/`protoc`) with `BTreeMap` map fields; the digest hashes the stored bytes.
- Persist the ed25519 signature alongside each node entry (enables chain-free offline authorship verification).
- Out of scope for this change (separate future work): the retrieval/verification client (light-client bootstrap, root key, paranoid vs normal routes, cross-check). There is **no merkle tree** and **no per-node digest** in v1.

## Capabilities

### New Capabilities
- `directory-contract`: a signed, push-based key-value directory of node-published and admin-curated data, keyed by `(id, label)` over opaque deterministic bytes, with identity-key write authorization, per-node replay protection, an admin label whitelist with per-label sizes, self-cleaning lifecycle via a mixnet unbond callback, and a global on-chain integrity digest enabling verifiable whole-directory retrieval.

### Modified Capabilities
None as OpenSpec specs - no `mixnet-contract` capability spec exists. The mixnet contract does require a code change (a directory address in state, a queued migration, and a best-effort unbond sub-message); this is captured under Impact and in tasks, and the directory side of that callback is a requirement within `directory-contract`.

## Impact

- New crates mirroring the node-families layout: `contracts/directory` (the contract) and `common/cosmwasm-smart-contracts/directory-contract` (shared message/type definitions).
- **Mixnet contract code change** (no existing spec): add `directory_contract_address` to `State` + `InstantiateMsg` + a queued migration; in the unbond handler emit a best-effort `OnNymNodeUnbond { node_id }` sub-message (reply-on-error, non-fatal). The directory exposes an `OnNymNodeUnbond` handler gated by an `UnauthorisedMixnetCallback` check (sender must be the mixnet contract).
- Cross-queries the mixnet contract (`MixnetContractQuerier`) for node existence (bonded and not unbonding) and the node's ed25519 identity key (stored base58; decode to the raw 32 bytes that `ed25519_verify` expects).
- New dependencies validated against the contracts workspace MSRV (rustc 1.86) and the cosmwasm VM: `lthash-rs` + `sha3` (SHAKE XOF) for the digest, and `prost` (derive-only). Feasibility was confirmed by building a contract that uses LtHash and passing `cosmwasm-check`.
- Downstream (separate changes): nym-node gains a push client; nym-api and the node-status API gain a read path; the verification client also cross-references the coconut-DKG contract for the core nym-api signer set.

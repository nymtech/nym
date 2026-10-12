## Why

The network-monitor-v3 orchestrator is about to submit liveness, stress and config scores to the performance contract every epoch, and two consumers need more from the contract than it offers today. Rewarding anchors its 24-epoch fallback at the requested epoch, so any outage longer than a day (orchestrator down, monitor account out of fees, every kind dropped by a coverage guard) resolves every node to no score and zeroes the network's rewards. Routing has no verifiable source at all: clients pick nodes using performance served by an untrusted topology server, which can deflate competitors below the clients' minimum-performance cutoffs or inflate its own nodes to capture traffic, and nothing lets a client check that the set of scores it received is complete and authentic. The directory and geolocation contracts already solve that second problem with an LtHash digest, a bulk enumeration and a mixnet unbond callback; this change gives the performance contract the same pattern and anchors the fallback where it stops punishing global outages. The contract is not deployed anywhere, so its side needs no migration.

## What Changes

- An LtHash16 digest over one entry per node: its newest bundle, as node id, source epoch and the score under that epoch's weights. Only the latest entry is committed, never history, and no per-kind medians, so adding a measurement kind leaves the commitment format unchanged. The entry is the bundle the existing last-known-epoch pointer names, so no new per-submission write is needed. The accumulator is stored raw at a fixed key for ICS23 proofs, with a smart query for its 32-byte collapse.
- A frozen, exact, fixed-length binary leaf encoding for that entry, defined in the common crate so the contract and every verifier derive identical bytes.
- Submissions keep the digest in step, loading and saving the accumulator once per transaction: the first report of a node in an epoch replaces its previous entry, and a further monitor's report updates the current one. An entry added under earlier weights is subtracted under those same weights.
- An `OnNymNodeUnbond { node_id }` handler, callable only by the stored mixnet contract address. It subtracts the node's entry and deletes its pointer, so the pointer map holds exactly the nodes in the routing view. Bundles stay, and rewarding and audits keep resolving the node's data.
- A `LatestPerformancePaged` bulk query that walks the pointers and returns exactly the committed entries, which is what a client folds to verify the digest at a pinned height.
- **BREAKING** `RewardingInputs` (and so `RewardingScore`) anchors its fallback at R, the newest epoch at or before the requested one that holds any bundle, instead of at the requested epoch. Normal operation is unchanged (R is the requested epoch). A node unmeasured while others are measured still loses its fallback after 24 epochs, while a global outage of any length freezes every node at its last value until data resumes. A node without a pointer, never measured or unbonded, resolves through the same bounded walk that audits use.
- **BREAKING** The admin removals `RemoveNodeMeasurements` and `RemoveEpochMeasurements` are dropped, together with `RemoveEpochMeasurementsResponse`, `EPOCH_PERFORMANCE_PURGE_LIMIT` and their validator-client methods. Bundles become append-only, so every pointer names an existing bundle and audits of past epochs always reproduce what rewarding resolved.
- **BREAKING** (mixnet contract) `ContractState` gains `performance_contract_address: Option<Addr>`, set through a new required `MigrateMsg.performance_contract_address`. `try_remove_nym_node` sends a fourth `OnNymNodeUnbond`, beside node families, geolocation and directory.
- The validator client gains `on_nym_node_unbond`, `get_latest_performance_paged`, `get_all_latest_performance` and `get_digest`, as its exhaustiveness tests require.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `performance-contract`: adds the latest-entry digest, its leaf encoding and raw storage, digest maintenance on submission, the mixnet unbond callback, and the bulk latest-entry and digest queries; re-anchors the `RewardingInputs` fallback at R and routes nodes without a pointer through its walk; makes the unbond callback the only writer that deletes a pointer; removes the admin removals; and updates the storage layout, response surface, error variants, paging limits and validator-client surface to match. The mixnet contract has no capability spec, so its address field, migration and callback are specified from the performance contract's side.

## Impact

**Code**: `common/cosmwasm-smart-contracts/nym-performance-contract` (entry type and leaf encoding, messages, errors, storage-key constants), `contracts/performance` (digest storage, submission maintenance, unbond handler, R-anchored resolution, new queries, removal of the admin removals, harness and tests, schema), `common/cosmwasm-smart-contracts/mixnet-contract` (`ContractState`, `MigrateMsg`), `contracts/mixnet` (migration, unbond fan-out, getter, test sibling wiring, schema), `common/client-libs/validator-client` (performance query and signing clients), `tools/internal/localnet-orchestrator` (its mixnet migrate message gains the new field), and `common/cosmwasm-smart-contracts/contracts-common` (a shared `DigestItem`, `DigestResponse` and `CorruptDigestState`, which the directory and geolocation contracts and their common crates move onto, with no change to their stored bytes or messages).

**Cost**: a submission transaction pays roughly 72k gas for loading and saving the 2 KB accumulator, plus about 1.5k gas per node for loading the old bundle and hashing, about 15% on top of today's submission cost. A `RewardingInputs` miss costs one extra iterator step to find R. A node without a pointer costs a walk of up to 25 loads, which usually stops at the first.

**Storage**: with no admin removals the bundle store is append-only, about 0.55 GB a year raw at 1000 nodes, 3 kinds and 3 monitors. Retention, if ever needed, comes back as a later code migration designed against the digest.

**Deployment**: the performance contract is instantiated and the mixnet migration that sets its address runs together, so no node can unbond in between and leave a dead entry in the digest.

**Follow-ups, each its own change**: the attested retrieval client and routing view library that verify this digest (`performance-retrieval-client`), the orchestrator's submission path (`network-monitor-contract-submission`), and the nym-api consumer (`nym-api-performance-contract-consumer`).

## Context

The performance contract (#7213) stores one bundle per `(epoch, node)` under `pr`, holding every monitor's value per kind (liveness, stress, config) as sorted integer percents. A per-node `last-known-epoch` pointer is written when a bundle is created, as a max. Weights live in a sparse map keyed by the epoch they take effect from; `UpdateWeights` always writes at `current + 1`, so the weights of an epoch are fixed before that epoch can hold data. Submissions are accepted only for the current mixnet epoch while it is in progress. `RewardingInputs { X, node }` resolves the bundle at X, else the pointer's bundle if it lies within 24 epochs of X, else (for audits, when data exists after X) the newest bundle in `[X - 24, X - 1]`, and scores it under the weights at X. Two admin removals, `RemoveNodeMeasurements` and `RemoveEpochMeasurements`, delete bundles without touching the pointer. The contract is not deployed anywhere: `mainnet::PERFORMANCE_CONTRACT_ADDRESS` is empty.

This change is the first of four stacked PRs that get network-monitor-v3 measurements into the contract and serve them verifiably. It is reviewed on its own and merged together with the others: the attested retrieval client and routing view library (PR2), the orchestrator's submission path (PR3) and the nym-api consumer (PR4). PR3 will submit, every epoch, only measured values for nodes with a config score and at least one of liveness or stress, dropping a kind for everyone when its coverage is below a threshold, so a broken measurement becomes a missing one and the contract fallback serves.

Two consumers need more than the contract offers. Rewarding: the lookback is anchored at X, so any outage longer than 24 epochs (orchestrator down, monitor account out of fees, every kind dropped by the coverage guard, the contract rejecting submissions) resolves every node to no score, which is zero rewards and an empty rewarded set. Routing: clients choose nodes from performance served by an untrusted topology server (nym-api today, nym-nodes later). Such a server can deflate competitors below a client's minimum-performance cutoff (`client-core/src/client/topology_control/nym_api_provider.rs`, `client-core/src/init/helpers.rs`) or inflate its own nodes to capture traffic, and nothing lets a client check the scores it was given.

The directory and geolocation contracts already carry the full pattern: an LtHash16 accumulator (`common/lthash`, 1024 x u16 = 2 KB, BLAKE3 XOF) stored raw at `digest_state` (`load_digest` / `save_digest`, copied into each contract's `storage.rs`), a paged enumeration a client folds to recompute it, an `OnNymNodeUnbond` handler fed by the mixnet contract's `try_remove_nym_node`, and the client-side `nym-contract-anchor` / `nym-contract-attestation` machinery that PR2 reuses.

## Goals / Non-Goals

**Goals:**

- At a pinned height H, a client can verify the authenticity of every node's latest score, the completeness of the set, and that both form one consistent snapshot with the directory at H.
- Keep the routing view to one entry per node, maintained with no new storage beyond the accumulator and a bounded cost per transaction.
- Keep rewarding alive through a global outage of any length while a node that alone stops being measured still loses its fallback after 24 epochs.
- Make every audit of a past epoch reproduce what rewarding resolved, with bounded reads, however long an outage lasted.

**Non-Goals:**

- **Accuracy, freshness and availability.** Monitors are trusted; the client checks the time of H itself; serving the last snapshot while the chain is unreachable is the calling binary's concern.
- **The staleness policy and bond filtering.** The routing view library (PR2) tags each entry with its source epoch, filters by mixnet bonds at H, and leaves staleness unbounded. The contract commits the data, not the policy.
- **History in the digest.** Routing needs the latest value per node; history stays available through the existing queries.
- **Retention.** Bundles become append-only (Decision 7).
- **The retrieval client, the orchestrator submission and the nym-api consumer.** PR2, PR3 and PR4.

## Decisions

### 1. The digest covers one entry per node: its newest bundle

The accumulator commits, for each node with a pointer `p`, the entry `LatestPerformance { node_id, epoch_id: p, score }`. A client checking routing data needs the current value per node and nothing older, and committing history would make the committed set, and with it every client's full pull, grow without bound. The source epoch is part of the entry, so a client applies its own staleness policy and a resubmission yielding an identical score in a new epoch still changes the leaf.

### 2. The entry commits only the score, under the weights of its own epoch

The leaf carries the score computed under `weights_at(p)` and no per-kind medians. Every routing consumer uses one number per node: the clients' minimum-performance cutoffs, the routing view library, and nym-api's routing figure. More measurement kinds are planned, and committing the medians would freeze the set of kinds into the wire format, so each added kind would take a new tag and a re-fold of the whole accumulator. With the score alone, a new kind only changes how scores are computed. The medians stay readable, unproven, through `NodePerformance`, `EpochPerformancePaged` and `RewardingInputs`.

Committing the score rather than leaving verifiers to compute it means verifiers never prove weights. They would otherwise have to prove the weights in force at `p`, and the weights map is sparse and resolved by a descending range: an ICS23 point proof can show an entry exists but not that no entry lies between it and `p`. The committed score is stable, because `UpdateWeights` writes at `current + 1` and bundles exist only for epochs up to the current one, so the weights of an epoch that holds data never change after the fact.

The contract recomputes an old entry's score from its stored bundle when subtracting it, so a code change to the score computation must leave the score of every stored bundle unchanged. A new weighted kind satisfies this: it is absent from every older bundle, and `Weights::score` skips absent kinds. A new required gate like config does not, nor does a change to the renormalisation, the config multiplier, the rounding or the median rule. Such a change must be selected by a version carried in the weights entry, which is already keyed by epoch, so that earlier epochs keep the old formula, or ship with a migration that re-folds the accumulator over every pointer. A score that read anything beyond the bundle and its epoch's weights, such as stake or another contract's state, would break the digest without any code change. The doc on `Weights::score` lists these, and `score_outputs_are_frozen` pins sample outputs so that an accidental change fails a test.

Rejected: committing the medians beside the score. It would let a client verify per-kind values and weight them itself, but no consumer does, and it ties the format to the set of kinds.

### 3. The leaf encoding is exact and fixed-length

The leaf is 26 bytes: a tag byte `1`, `node_id` and `epoch_id` as big-endian `u32` (the byte order both sibling contracts use for node ids), then the score as a presence byte followed by the big-endian `u128` atomics of its `Decimal`. Every field is fixed-width, so no length prefixes are needed, and the presence byte keeps an absent score apart from a present zero. It lives on `LatestPerformance::digest_leaf` in the common crate, so the contract and PR2's verifier share one implementation.

A one-byte integer percent was considered, since every score the contract produces is a whole percent. It was rejected because it is lossy as a commitment: a verifier folding a served `0.8549` would derive the same leaf as for the committed `0.85` unless it also checked that every value is a whole percent. With exact atomics any deviation fails verification and the verifier needs no extra rule. The leaf is hashed through the BLAKE3 XOF either way, so its length costs nothing measurable.

The tag versions the format. The accumulator holds a single entry class, so nothing within it needs separating, and leaves need not be distinguishable from another contract's, because each contract has its own accumulator and a proof binds to one contract address and key.

### 4. The pointer map is the routing view's index, and unbonding deletes from it

Bundles are keyed epoch-first, so a node's newest bundle can only be found cheaply through its pointer. The mixnet contract does record unbonded nodes, but its `UnbondedNymNode` holds the identity key, owner and unbonding height, not which epoch holds the node's newest bundle. The pointer already moves exactly when a node's entry does, so the routing view needs no new per-submission write: the pointer map is its index, and the committed set is every node in it.

The unbond callback deletes the node's pointer. After that, the only reads that care about an unbonded node's newest bundle are rewarding for the epoch in which it began unbonding and audits, and both are served by the bounded walk `RewardingInputs` already uses (Decision 9). `LastKnownEpoch` and `NodePerformancePaged` treat the node as never measured, while its bundles stay readable per epoch through `NodePerformance`, `NodeMeasurements` and the epoch and full-history pages.

Rejected alternatives:

- **An `unbonded` marker set beside the pointers.** The bulk query would visit every node ever measured, forever, and check each against the set, so pages would fill with skipped nodes and grow more expensive with every unbond.
- **Moving the pointer into an `unbonded-last-known-epoch` map.** It serves only the rare reads above, which the walk serves equally.
- **A separate `latest` map written per submission.** A second copy of what the bundle holds, paid once per node per epoch.

### 5. Submissions update the digest per bundle, loading and saving it once per transaction

`Submit` and `BatchSubmit` load the accumulator once, update it per bundle written, and save it once. For each bundle at `(X, node)`:

- a further monitor's report merges into an existing bundle: subtract the leaf of the bundle as stored, add the leaf of the merged bundle;
- the first report of the node in X creates the bundle: subtract the node's previous entry if it has a pointer (that bundle always exists), add the new leaf, and advance the pointer as today.

The previous entry must be subtracted with the score it was added with, so it needs the weights at its own epoch E. `weights_at(X)` is loaded once per transaction and returns `effective_from` F along with the weights. Since no weights entry lies in `(F, X]`, any `E >= F` shares them and needs no read. Only `E < F`, which happens on a node's first submission after an `UpdateWeights` took effect, reads `weights_at(E)`. That read is not cached: it costs a few hundred gas per such node, a few hundred thousand gas once per weights update at about 1000 nodes, against about 12M gas every epoch, so a cache would buy nothing worth its mutable state.

Writing the bundle and maintaining the digest are separate steps. The storage-level insert writes the bundle and reports the node's newest bundle it replaced, which is the bundle as stored on a merge and the bundle the pointer named on a create, together with the bundle written; the contract-level step subtracts the replaced entry and adds the written one. A single `Submit` is a batch of one that turns a skipped node into `NodeNotBonded`, so the batch path is the only one that touches the accumulator.

An unbonded node never reaches this step: the mixnet contract reports an unbonding node as not bonded, and node ids are never reused, so its submission fails or is skipped before the insert.

Cost, estimated: about 72k gas per transaction for the 2 KB load and save, plus about 1.5k per node for the old bundle load and two leaf hashes, roughly 15% on top of today's submission. At about 1000 nodes that is about 12k gas per node and 12M per epoch overall, about 0.3 NYM per epoch at 0.025 unym/gas.

### 6. The unbond callback subtracts the entry and deletes the pointer

The mixnet contract's `try_remove_nym_node` already sends `OnNymNodeUnbond { node_id }` to node families, geolocation and directory. It gains a fourth, to `performance_contract_address` when set. The handler requires the sender to be the stored mixnet address (`UnauthorisedMixnetCallback { sender }` otherwise, as in the siblings). If the node has a pointer `p`, it subtracts the leaf of the entry at `p`, with its score under `weights_at(p)`, and deletes the pointer. A node with no pointer was never measured, and the handler writes nothing. Bundles and weights stay.

The callback fires exactly once per node: a second `UnbondNymNode` fails at `ensure_bonded` with `NodeIsUnbonding`. It fires in the transaction in which the mixnet starts reporting the node as not bonded, so no submission for the node can land after its entry left the digest. Removal is permanent for the same reason.

It is a plain message, not a reply-on-error sub-message, so a failing handler reverts the unbond. That is the trade-off the three existing callbacks already make, and the handler is bounded: one pointer read, one bundle load, one weights lookup, and the digest load and save. It returns an empty response, like the contract's other handlers that have nothing to report.

### 7. The admin removals are dropped and bundles become append-only

`RemoveNodeMeasurements` and `RemoveEpochMeasurements` were kept by #7213 as the only retention backstop for an append-only store. With a digest over each node's latest bundle they cost more than they give:

- they are the one way a pointer can name a missing bundle, a state that the digest maintenance, the bulk query and `RewardingInputs` would each have to handle;
- removing recent epochs moves R (Decision 9), so audits of later epochs would no longer reproduce what rewarding resolved;
- they were never usable as a bad-data escape hatch. Removing a bundle strips every monitor's values for the node, which leaves rewarding nothing to resolve, and the remedy for a misbehaving monitor is `RetireNetworkMonitor`.

Dropping them makes every pointer name an existing bundle, gives the bulk query nothing to skip, keeps the `RewardingInputs` pointer branch to one load, and makes audits unconditionally reproducible. `RemoveEpochMeasurementsResponse`, `EPOCH_PERFORMANCE_PURGE_LIMIT` and three validator-client methods go with them; nothing else calls them. The cost is storage: #7213 sized the store at about 0.55 GB a year raw at 1000 nodes, 3 kinds and 3 monitors. If that ever needs trimming, retention comes back as a new message in a later code migration, designed against the digest: for example, only epochs well behind current and never a bundle that a pointer names.

### 8. The bulk query walks the pointer map

`LatestPerformancePaged { start_after, limit }` ranges over `last-known-epoch` in ascending node order and returns every node it visits, each with its score under the weights at its own epoch, computed by the same scoring the digest maintenance uses: the latest weights cover every entry from the epoch they took effect, and only older entries read their own. Every pointer names an existing bundle and the map holds no unbonded node, so nothing is skipped and the cursor follows the convention of the epoch pages: `start_next_after` is the last node returned, or `None` for an empty page. Limits are 100 by default and 200 at most, like the other performance pages. Callers pin every page and the digest read to one height H.

The entry type `LatestPerformance` is separate from `HistoricalPerformance`: it carries exactly the fields the leaf commits, with no medians that a verifier might trust unproven, and the frozen leaf encoding, and it means "the committed latest entry" rather than a history row. A `Digest {}` smart query returns the 32-byte collapse of the stored accumulator, mirroring both sibling contracts, as an unproven convenience for comparing digests. The proven read is the raw key.

### 9. The rewarding fallback is anchored to R, and a node without a pointer walks

R is the newest epoch `<= X` holding a bundle for any node: one descending iterator step over the `pr` keys from `(X, u32::MAX)`, over state that already exists. The source for `(X, node)` is the bundle at X if present, otherwise the node's newest bundle in `[R - 24, R]` below X:

- with a pointer at or below X, the pointer's bundle if it lies at or after `R - 24`, one load as today;
- with a pointer above X (an audit) or no pointer at all (never measured, or deleted by the unbond callback), a walk down from `min(R, X - 1)` to `R - 24`, newest first.

`MAX_FALLBACK_LOOKBACK_EPOCHS` stays 24, and weights still come from X.

The effect: normal operation is identical to today, since R = X whenever anyone was measured at X. A node left unmeasured while others are measured gets nothing after 24 epochs. A global outage of any length (orchestrator down, account out of fees, the coverage guard dropping every kind, the contract rejecting submissions) freezes every node at its last value until data resumes, instead of zeroing the network's rewards. A partial epoch followed by an outage is fine, because the window around R still contains the last complete epoch.

The walk serves an unbonded node exactly as its pointer would have. The pointer named the node's newest bundle; when it lies below X it also lies at or below R, and with no removals it still exists. So the walk finds that bundle when it lies within the lookback, and nothing when it does not, which is precisely the pointer rule. That covers the node's last epoch: it began unbonding during X, the mixnet still rewards it for X (`try_reward_node` checks `still_bonded()`, a non-zero operator stake, and the stake only goes when the `UnbondNymNode` event runs during reconciliation, after rewarding), and if no monitor had reported it in X yet, the walk usually stops at the first load, X - 1. Audits of past epochs are reproducible for the same reason, however long an outage lasted, because the walk is bounded by R rather than by X.

Rejected: raising the cap, which only moves the cliff; and a per-epoch "complete" flag set by the orchestrator, which adds a protocol between the orchestrator and the contract for no gain over R.

The cost is one iterator step on a miss at X, plus, for a node without a pointer, up to 25 loads. nym-api reads off-chain and pays nothing. Once the mixnet contract calls `RewardingScore` directly, it would pay the walk only for never-measured nodes in the rewarded set and for nodes that began unbonding in X before being measured in X.

### 10. The mixnet learns the address only through its migration

`ContractState` gains `performance_contract_address: Option<Addr>`, `None` at instantiation. `MigrateMsg` gains a required `performance_contract_address: String`, applied by a new `introduce_performance_contract` inside `if !skip_state_updates`, exactly like `directory_contract_address` and `geolocation_contract_address`. A required field means nobody can migrate the mixnet contract and forget it; environments that skip state updates (the localnet orchestrator) pass a placeholder, as they already do for the other two.

It does not go into `InstantiateMsg`. The mixnet contract's instantiate is close to the wasm 100-locals limit, and the performance contract needs the mixnet address at its own instantiation, so in any environment the mixnet contract exists first and is wired afterwards, which is what the migration does. `cw_serde` (cosmwasm-schema 2.x) does not deny unknown fields, so binaries that deserialise the mixnet `ContractState` without the new field, such as nym-api's contract cache, keep working.

The migration ships with the performance deployment, so no node can unbond between the two and leave a dead entry in the digest.

### 11. The raw digest storage is shared by all three digest contracts

The directory and geolocation contracts each carry a private copy of `load_digest` / `save_digest`, a `DigestResponse` and a `CorruptDigestState` variant, and a third copy would follow here. Their rules (a raw key used verbatim, the full accumulator rather than its collapse, a length check on load) are load-bearing for every verifier, so they move into one `DigestItem` in `nym-contracts-common`, used like a `cw-storage-plus` `Item`: each contract holds a `digest: DigestItem` field built from its own `DIGEST_STATE` constant. `DigestResponse` and the `CorruptDigestState` error type sit beside it outside any feature, so the contract common crates re-export them without pulling in the hashing dependency; `DigestItem` itself is behind an `lthash` feature that only the contracts enable. Each contract's error enum wraps the shared error in a `CorruptDigestState` variant through thiserror's `#[from]`, with a transparent message, so the on-chain error string is unchanged.

The directory and geolocation contracts move onto it in this change. Their stored bytes, messages and query responses are unchanged, so their deployed instances are unaffected until their next code upgrade.

### 12. Testing

The core property test checks, after every step of an operation sequence (single and batch submissions by several monitors across epochs, merges, `UpdateWeights`, unbond callbacks), that folding every entry from `LatestPerformancePaged` into an empty accumulator equals the stored accumulator. The harness gets a helper that performs exactly that check and unwraps.

The unbond callback is tested at `App` level through a real `UnbondNymNode` on the mixnet contract, because deps-level handler tests never run sub-messages. The performance harness registers its own address with the mixnet contract through `MixnetContractSiblings`, as the directory harness does. The mixnet harness's `unbond_nymnode` advances the epoch, so tests that need the unbonding state within the same epoch execute `UnbondNymNode` directly.

## Risks / Trade-offs

- [A failing handler blocks every unbond] -> The handler is small and bounded (Decision 6), the same trade-off the three existing callbacks make, and it is covered by an `App`-level test through the real mixnet contract.
- [A node that unbonds while the mixnet contract lacks the address keeps a dead entry forever] -> The mixnet migration ships with the performance instantiation. Nothing exists before it, and unbonding nodes are rejected on submission, so nodes already unbonding at deployment never get an entry.
- [The bundle store grows without bound, about 0.55 GB a year raw] -> Accepted for now; retention can return through a code migration designed against the digest (Decision 7).
- [An unbonded node's per-node history query returns nothing] -> Its bundles stay readable per epoch and through the epoch and full-history pages; only the per-node convenience is lost.
- [Never-measured nodes resolve through a walk of up to 25 loads] -> Free off-chain. On-chain it matters only once the mixnet contract reads `RewardingScore` directly, and then only for never-measured nodes in the rewarded set.
- [The leaf encoding is a frozen wire format] -> The tag byte versions it, and it commits no per-kind values, so a new measurement kind leaves it unchanged (Decision 2). Nothing is deployed, so the format is settled before any data exists.
- [A change to the score computation would desynchronise the digest] -> Subtraction recomputes an old entry's score, so such a change must keep every stored bundle's score, as an added weighted kind does, be selected by a version in the weights entry, or re-fold the accumulator. A frozen-output test makes an accidental change fail (Decision 2).
- [Submission gas rises about 15%] -> Accepted: about 0.3 NYM per epoch for the whole network at about 1000 nodes.
- [Entries without a score are committed] -> They are part of the routing view and the client decides what to do with them. PR3 never submits a bundle that cannot score.
- [A client mixing pages from different heights sees an inconsistent set] -> The library pins every page and the digest read to one height (PR2).
- [LtHash wraps after 2^16 identical leaves] -> Unreachable: each leaf carries its node id and each node has at most one.
- [A global outage freezes every value indefinitely] -> Intended: the network keeps operating on last known values. PR3 alerts on any incomplete epoch, so an outage is noticed rather than silently frozen.

## Migration Plan

1. Performance contract: not deployed, so there is no state migration. Instantiate it on mainnet with its initial weights and the mixnet contract address.
2. Mixnet contract: upgrade the code and run `migrate` with `performance_contract_address` set to the new contract (plus the directory and geolocation addresses its `MigrateMsg` already requires), in the same step as the instantiation.
3. Rollback: migrating the mixnet contract back to a build without the field is safe, since unknown fields are ignored. The performance contract then keeps working without callbacks, and nodes that unbond meanwhile keep their entries until the callback is restored and a fix-up is decided.

## Open Questions

None for this change.

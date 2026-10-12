## ADDED Requirements

### Requirement: The digest commits to every node's latest performance entry

The contract SHALL maintain an LtHash16 accumulator (`nym-lthash`) that, after every successful execute message, equals the sum of one leaf per node in the committed set and nothing else. The committed set SHALL be every node with an entry `p` in `last-known-epoch`, and its entry SHALL be `LatestPerformance { node_id, epoch_id: p, medians, score }`, where `medians` are the per-kind medians of the bundle at `(p, node)` and `score` is their score under `WeightsAt { epoch_id: p }` per the score requirement, `None` when the score is. Bundles are never removed, so that bundle always exists. Older bundles of a node MUST NOT contribute to the digest, and a node the mixnet contract has called back for has no pointer and so is not in the committed set. Only submissions and the unbond callback SHALL change the committed set.

#### Scenario: Recomputing the committed entries reproduces the digest
- **WHEN** any sequence of single and batch submissions by several monitors across several epochs, `UpdateWeights` and unbond callbacks executes
- **THEN** after every step, folding the leaf of every entry that `LatestPerformancePaged` returns, paged to the end, into an empty accumulator yields exactly the stored accumulator

#### Scenario: Older bundles are not committed
- **WHEN** node 7 is the only node and has bundles at epochs 9 and 10
- **THEN** the stored accumulator equals the empty accumulator with only the leaf of node 7's epoch 10 entry added

#### Scenario: An unscoreable bundle is committed without a score
- **WHEN** node 7's only bundle holds `Liveness: [90%]` and no `Config`
- **THEN** its committed entry carries the liveness median and `score: None`

### Requirement: The leaf encoding is fixed, exact and shared with verifiers

`LatestPerformance::digest_leaf` in the common crate SHALL be the one leaf encoding, used by the contract and by every verifier. The leaf SHALL be exactly 77 bytes: the tag byte `1`; `node_id` as 4 bytes big-endian; `epoch_id` as 4 bytes big-endian; then the liveness median, the stress median, the config median and the score, in that order, each as a presence byte (`0` absent, `1` present) followed by the 16 bytes big-endian of the `u128` atomics of its `Decimal` value, all zero when absent. The encoding MUST be lossless, so that a served entry differing from the committed one in any field, by any amount, yields a different leaf. It is a frozen wire format: changing it is a breaking migration that re-folds the whole accumulator and takes a new tag byte. The accumulator holds a single entry class, so the tag versions the format rather than separating classes, and leaves need not be distinguishable from another contract's, because each contract has its own accumulator and a proof binds to one contract address and key.

#### Scenario: The layout is as documented
- **WHEN** the leaf of `{ node_id: 7, epoch_id: 10, medians: { Liveness: 100%, Config: 50% }, score: 50% }` is computed
- **THEN** it is the 77 bytes `01`, `00000007`, `0000000a`, `01` followed by the atomics of `1`, `00` followed by 16 zero bytes, `01` followed by the atomics of `0.5`, and `01` followed by the atomics of `0.5`

#### Scenario: Every field is committed
- **WHEN** two entries differ only in the node id, only in the epoch, only in one median, or only in the score
- **THEN** their leaves differ

#### Scenario: An absent value differs from a present zero
- **WHEN** two entries differ only in that one has no stress median and the other a stress median of `0%`
- **THEN** their leaves differ

#### Scenario: A served value cannot drift within a rounding slack
- **WHEN** two entries differ only in that one carries a liveness median of `0.85` and the other `0.8549`
- **THEN** their leaves differ

### Requirement: The accumulator is stored raw at a fixed key and touched at most once per transaction

The contract SHALL persist the full accumulator, `DIGEST_LEN` (2048) raw bytes, at the storage key `digest_state`, the common crate constant `storage_keys::DIGEST_STATE`, written with `store.set` directly rather than through a `cw-storage-plus` item, so that the proven key is exactly those bytes under the contract's prefix. The key MUST never change. An absent key SHALL read as the empty accumulator, and a stored value of any other length MUST fail with `CorruptDigestState`. `QueryMsg::Digest {}` SHALL return `DigestResponse { digest: Binary }` holding the 32-byte collapse `LtHash::out()` of the stored accumulator, as an unproven convenience; the collapse MUST NOT be persisted. Every `Submit`, `BatchSubmit` and `OnNymNodeUnbond` MUST load the accumulator at most once and save it at most once.

#### Scenario: The raw key holds the accumulator and the query its collapse
- **WHEN** a submission has landed and a raw read of `digest_state` and the `Digest {}` query are performed at the same height
- **THEN** the raw read returns 2048 bytes and the query returns the BLAKE3 collapse of exactly those bytes

#### Scenario: A fresh contract has the empty digest
- **WHEN** nothing has been submitted since instantiation
- **THEN** `Digest {}` returns the collapse of the empty accumulator

#### Scenario: Batching does not change the digest
- **WHEN** the same nodes are submitted as one `BatchSubmit` and, against identical starting state, as one `Submit` per node
- **THEN** the resulting accumulators are identical

### Requirement: Submissions keep the digest in step with each node's latest entry

For every bundle a submission writes at `(X, node)`, the contract MUST update the accumulator as follows. When the bundle already existed, because another monitor reported the node in `X` first, it MUST subtract the leaf of the entry for the bundle as stored and add the leaf for the merged bundle. When the submission creates the bundle, it MUST subtract the node's previous entry if the node has an entry `p` in `last-known-epoch`, and add the leaf for the new bundle, while the pointer advances to `X` as before. A previous entry at epoch `p` MUST be subtracted with its score recomputed under `WeightsAt { epoch_id: p }`, the weights it was added with. When `p` is at or after the `effective_from` of `WeightsAt { epoch_id: X }`, those are the weights already loaded for `X`; otherwise `WeightsAt { epoch_id: p }` is read, at most once per distinct epoch per transaction. A node the mixnet contract has called back for never reaches this step, because the mixnet contract reports it as not bonded.

#### Scenario: A node's first bundle adds its entry
- **WHEN** node 7 has never been measured and an authorised monitor submits it for epoch 10
- **THEN** the leaf of its epoch 10 entry is added and nothing is subtracted

#### Scenario: A node's next epoch replaces its entry
- **WHEN** node 7 is the only node, has a bundle at epoch 10, and an authorised monitor submits it for epoch 11
- **THEN** the accumulator equals the empty accumulator with only the leaf of node 7's epoch 11 entry added, and the bundle at epoch 10 is still stored

#### Scenario: A second monitor in the same epoch updates the entry
- **WHEN** weights are `{ Liveness: 100% }`, monitor A submits node 7 with `{ Liveness: 90%, Config: 100% }` for epoch 10 and monitor B submits node 7 with `{ Liveness: 80%, Config: 100% }` for epoch 10
- **THEN** the accumulator equals the empty accumulator with only the leaf of `{ node_id: 7, epoch_id: 10, medians: { Liveness: 85%, Config: 100% }, score: 85% }` added

#### Scenario: An entry added under earlier weights is subtracted exactly
- **WHEN** node 7's latest bundle is at epoch 10 under `{ Liveness: 100% }`, the admin sets `{ Liveness: 50%, Stress: 50% }` effective from epoch 11, and node 7 is submitted for epoch 12
- **THEN** the accumulator equals the empty accumulator with only the leaf of node 7's epoch 12 entry, scored under the new weights, added

### Requirement: The mixnet contract's unbond callback removes a node from the digest permanently

`ExecuteMsg::OnNymNodeUnbond { node_id }` MUST fail with `UnauthorisedMixnetCallback { sender }` unless the sender is the stored mixnet contract address. Otherwise, when the node has a pointer `p`, the handler MUST subtract the leaf of its entry at `p` and delete the pointer; when it has none, because it was never measured, the handler MUST change nothing. It MUST NOT remove bundles or touch the weights. `RewardingInputs` and `RewardingScore` therefore answer for the node exactly as before the callback, through the walk their resolution takes for a node without a pointer, so rewarding for the epoch in which the node began unbonding still resolves its data. `LastKnownEpoch` and `NodePerformancePaged` treat the node as never measured, and its bundles stay readable through `NodePerformance`, `NodeMeasurements` and the epoch and full-history pages. The mixnet contract sends the callback once per node, when the node begins unbonding, and from then on reports the node as not bonded; node ids are never reused, so every later submission for the node fails or is skipped as `NodeNotBonded` and it never re-enters the committed set.

The mixnet contract SHALL hold `performance_contract_address: Option<Addr>` in its `ContractState`, `None` at instantiation and set only by its migration from the required `MigrateMsg.performance_contract_address`, validated as an `Addr` and applied unless `unsafe_skip_state_updates` is set. When it is set, `try_remove_nym_node` MUST send `OnNymNodeUnbond { node_id }` to it as a plain message beside the node-families, geolocation and directory callbacks, so that a failing handler reverts the unbond.

#### Scenario: A real unbond removes the node from the digest
- **WHEN** node 7's latest bundle is at epoch 10, the mixnet contract holds the performance contract's address, and node 7's owner executes `UnbondNymNode` on the mixnet contract
- **THEN** the callback runs, `LatestPerformancePaged` omits node 7, the digest equals the recompute, `LastKnownEpoch { 7 }` returns `None`, and node 7's bundles are unchanged

#### Scenario: Rewarding and per-epoch reads of an unbonded node are unchanged
- **WHEN** node 7 has bundles at epochs 9 and 10 and has been called back
- **THEN** `RewardingInputs` and `RewardingScore` return for node 7, at every epoch, exactly what they returned before the callback, `NodePerformance { 10, 7 }` still returns the epoch 10 medians, and `NodePerformancePaged` for node 7 returns an empty page

#### Scenario: Only the mixnet contract may call back
- **WHEN** any address other than the stored mixnet contract sends `OnNymNodeUnbond { node_id: 7 }`
- **THEN** the call fails with `UnauthorisedMixnetCallback { sender }` and nothing changes

#### Scenario: A never-measured node changes nothing
- **WHEN** node 8 has never been measured and the callback arrives for it
- **THEN** the call succeeds, nothing is written, and the accumulator is unchanged

#### Scenario: An unbonded node cannot return
- **WHEN** node 7 has begun unbonding through the mixnet contract, which called back, and an authorised monitor submits it for the current epoch
- **THEN** `Submit` fails with `NodeNotBonded { node_id: 7 }`, `BatchSubmit` lists it in `non_existent_nodes`, and the accumulator is unchanged

#### Scenario: The mixnet migration sets the address
- **WHEN** the mixnet contract is migrated with `performance_contract_address: P`
- **THEN** its `ContractState.performance_contract_address` is `Some(P)`, and a migration with `unsafe_skip_state_updates: true` leaves it unchanged

#### Scenario: Without the address no callback is sent
- **WHEN** the mixnet contract's `performance_contract_address` is `None` and a measured node unbonds
- **THEN** the unbond succeeds and the performance contract's committed set still holds the node

### Requirement: LatestPerformancePaged enumerates exactly the committed entries

`QueryMsg::LatestPerformancePaged { start_after: Option<NodeId>, limit: Option<u32> }` SHALL return `LatestPerformancePagedResponse { performance: Vec<LatestPerformance>, start_next_after: Option<NodeId> }`. It SHALL range over `last-known-epoch` in ascending node order with an exclusive lower bound at `start_after`, and return the entry of every node it visits, each `score` under the weights of the entry's own `epoch_id`. `start_next_after` SHALL be the last node returned, or `None` for an empty page. A caller verifying the digest MUST pin every page and the digest read to one block height.

#### Scenario: Every committed entry is returned once
- **WHEN** nodes 1 to 5 have been measured, node 2 has been called back, and the query pages with `limit: 2`
- **THEN** the pages hold nodes `[1, 3]` with `start_next_after: Some(3)`, then `[4, 5]` with `Some(5)`, then nothing with `None`

#### Scenario: Each entry carries its own source epoch and weights
- **WHEN** node 1's latest bundle is at epoch 10, node 2's at epoch 12, and new weights took effect from epoch 11
- **THEN** node 1's entry has `epoch_id: 10` and a score under the epoch 10 weights, and node 2's has `epoch_id: 12` and a score under the new weights

## MODIFIED Requirements

### Requirement: A per-node last-known-epoch pointer is written when a bundle is created

The contract SHALL keep `Map<NodeId, EpochId>` under the namespace `last-known-epoch`. When a submission creates the bundle for `(epoch, node)`, meaning no bundle existed for that key before the insert, the handler MUST set the pointer for that node to the greater of its stored value and `epoch`. A submission that merges into an existing bundle MUST NOT write the pointer. The unbond callback MUST delete the node's pointer, and nothing else deletes it, so the pointer map holds exactly the nodes in the routing view and every pointer names an existing bundle. `QueryMsg::LastKnownEpoch { node_id }` SHALL return `LastKnownEpochResponse { epoch_id: Option<EpochId> }`, `None` for a node that has never had a bundle or that the mixnet contract has called back for.

#### Scenario: The first monitor to report a node in an epoch advances the pointer
- **WHEN** node 7 has no bundle for epoch 10, monitor A submits it, and then monitor B submits it in the same epoch
- **THEN** after monitor A `LastKnownEpoch { 7 }` is `Some(10)`, and monitor B's submission leaves it at `Some(10)`

#### Scenario: A never-measured node has no pointer
- **WHEN** node 8 has never been submitted
- **THEN** `LastKnownEpoch { 8 }` returns `None`

#### Scenario: The unbond callback deletes the pointer
- **WHEN** node 7's pointer is 10 and the mixnet contract calls back for it
- **THEN** `LastKnownEpoch { 7 }` returns `None` and the bundle at `(10, 7)` is still stored

### Requirement: RewardingInputs resolves a bundle deterministically with per-bundle fallback and scores it with the weights of the requested epoch

`QueryMsg::RewardingInputs { epoch_id: X, node_id }` SHALL resolve a source bundle as follows. The bundle at `(X, node)` is the source if it exists. Otherwise let `R` be the newest epoch not after `X` that holds a bundle for any node, found by one descending step over the bundle keys from `(X, u32::MAX)`, let `floor = R.saturating_sub(MAX_FALLBACK_LOOKBACK_EPOCHS)`, and let `p` be the node's last-known-epoch pointer. The source is `None` if `R` is absent. If `p` is present and `p <= X`, the source is the bundle at `(p, node)` when `p >= floor`, else `None` (the pointer names the newest bundle the node ever had, so nothing older is consulted). Otherwise, when `p` is absent (the node was never measured, or the unbond callback deleted its pointer) or `p > X` (data arrived after `X`), the source is the first existing bundle at the epochs `e` with `floor <= e <= R` and `e < X`, newest first, else `None`. Because bundles are never removed, that walk finds exactly the bundle a deleted pointer would have named whenever it lies within the lookback. `MAX_FALLBACK_LOOKBACK_EPOCHS` SHALL be the contract constant `24`. Anchoring the lookback at `R` rather than `X` means that a node left unmeasured while other nodes are measured loses its fallback 24 epochs after `R` moves past its last bundle, while epochs in which no node was measured at all do not count against the lookback, so a global outage of any length leaves every node at its last bundle until data resumes. The response SHALL be `RewardingInputsResponse { requested_epoch_id: X, source: Option<ResolvedMedians { epoch_id, medians }>, weights: Option<EpochWeights>, score: Option<Percent> }` where `medians` are the per-kind medians of the source bundle, `weights` are `WeightsAt { X }`, and `score` is computed from the source medians and those weights per the score requirement, `None` when either is absent. The resolution MUST depend only on stored bundles, the pointer and the weights map. Bundles are never removed and a bundle for an epoch is frozen once its transition begins, so repeated calls after that point return identical results, and an audit of a past epoch always reproduces what was resolved while that epoch was current.

#### Scenario: An existing bundle is used directly
- **WHEN** node 7 has a bundle at epoch 10
- **THEN** `RewardingInputs { 10, 7 }` returns `source.epoch_id = 10`, its medians, the weights at 10, and the score

#### Scenario: A missing epoch falls back to the latest earlier bundle within the lookback
- **WHEN** node 7 has bundles at epochs 8 and 12 and none at 9, 10 or 11
- **THEN** `RewardingInputs { 10, 7 }` and `RewardingInputs { 11, 7 }` both return `source.epoch_id = 8`

#### Scenario: Data older than the lookback is not used
- **WHEN** node 7's only bundle is at epoch 10 and node 8 has a bundle at every epoch from 10 to 35
- **THEN** `RewardingInputs { 34, 7 }` returns `source.epoch_id = 10` and `RewardingInputs { 35, 7 }` returns `source = None` and `score = None`

#### Scenario: A global outage freezes every node at its last bundle
- **WHEN** nodes 7 and 8 have bundles at epoch 10 and no bundle exists at any later epoch
- **THEN** `RewardingInputs { 100, 7 }` returns `source.epoch_id = 10` and a score under `WeightsAt { 100 }`

#### Scenario: An audit across a long outage reproduces what was resolved at the time
- **WHEN** node 7 has bundles at epochs 10 and 60 and no node has a bundle at any epoch from 11 to 59
- **THEN** `RewardingInputs { 40, 7 }` returns `source.epoch_id = 10`, the same source it resolved while epoch 40 was current

#### Scenario: A node's last epoch resolves after it began unbonding
- **WHEN** node 7's latest bundle is at epoch 9, other nodes are measured in epoch 10, and node 7 begins unbonding during epoch 10 before any monitor has reported it
- **THEN** `RewardingInputs { 10, 7 }` returns `source.epoch_id = 9`, found by the walk now that the callback has deleted the pointer

#### Scenario: Fallback is per bundle, so a kind that stopped applying is not borrowed
- **WHEN** node 7's bundle at epoch 10 holds `{ Liveness, Stress, Config }`, its bundle at epoch 12 holds `{ Liveness, Config }` only, and weights are `{ Liveness: 70%, Stress: 30% }`
- **THEN** `RewardingInputs { 12, 7 }` has medians without `Stress` and a score renormalised to `Liveness` alone

#### Scenario: The score uses the weights of the requested epoch, not the source epoch
- **WHEN** node 7's only bundle is at epoch 10, `{ Liveness: 100% }` is effective from 0 and `{ Liveness: 50%, Stress: 50% }` from 11
- **THEN** `RewardingInputs { 12, 7 }` returns `source.epoch_id = 10`, `weights.effective_from = 11`, and a score computed with the `50%/50%` weights

#### Scenario: Repeated calls agree
- **WHEN** `RewardingInputs { 10, 7 }` is called, the mixnet advances several epochs and further submissions land, and it is called again
- **THEN** both calls return identical responses

### Requirement: RewardingScore returns only the final value rewarding uses, from the same resolution

`QueryMsg::RewardingScore { epoch_id, node_id }` SHALL return `RewardingScoreResponse { score: Option<Percent> }` where `score` is exactly the `score` field that `RewardingInputs { epoch_id, node_id }` returns for the same arguments. It MUST be computed by the same resolution function as `RewardingInputs` and MUST NOT have a code path of its own. It exists so that a consuming contract, the mixnet contract in the second iteration, reads the single value it needs, while the medians, source epoch and weights that explain it remain available to external users through `RewardingInputs`.

#### Scenario: The score-only query agrees with RewardingInputs
- **WHEN** `RewardingScore { X, Y }` and `RewardingInputs { X, Y }` are called with the same arguments in a direct-hit case, a fallback case and a no-data case
- **THEN** `RewardingScore.score` equals `RewardingInputs.score` in every case

#### Scenario: No usable data yields no score
- **WHEN** node `Y` has no bundle at `X`, and its newest bundle at or before `X` lies more than `MAX_FALLBACK_LOOKBACK_EPOCHS` before the newest epoch not after `X` that holds any bundle
- **THEN** `RewardingScore { X, Y }` returns `score: None`

### Requirement: Paged queries apply the existing default and maximum limits

Every paged query SHALL use `limit.unwrap_or(DEFAULT).min(MAX)` with the `retrieval_limits`: node performance 100/200, epoch performance 100/200, epoch measurements 50/100, full historical performance 100/200, latest performance 100/200, network monitors 50/100, and retired network monitors 50/100.

#### Scenario: A limit above the maximum is capped
- **WHEN** `EpochPerformancePaged` is called with `limit: Some(10_000)`
- **THEN** at most 200 entries are returned

#### Scenario: The latest-performance page is capped
- **WHEN** `LatestPerformancePaged` is called with `limit: Some(10_000)` while more than 200 nodes are in the committed set
- **THEN** the page holds 200 entries and `start_next_after` is the 200th node

### Requirement: The validator-client exposes one method per execute and query variant

`PerformanceSigningClient` SHALL expose `update_weights` and `on_nym_node_unbond`, and its `submit_performance` / `batch_submit_performance` SHALL take `NodeSubmission` payloads. `PerformanceQueryClient` SHALL expose `get_rewarding_inputs`, `get_rewarding_score`, `get_last_known_epoch`, `get_weights_at`, `get_current_weights`, `get_latest_performance_paged` and `get_digest`, and `PagedPerformanceQueryClient` SHALL keep a collector per paged query, including `get_all_latest_performance`. The existing `all_execute_variants_are_covered` and `all_query_variants_are_covered` tests MUST enumerate every variant, so a message variant without a client method fails compilation.

#### Scenario: Every message variant has a client method
- **WHEN** the validator-client crate is compiled with its tests
- **THEN** both exhaustiveness tests compile against the new `ExecuteMsg` and `QueryMsg` enums

### Requirement: Public storage layout

The contract's persistent state SHALL consist of exactly: cw2's `contract_info` and the build information item; a cw-controllers `Admin` under `contract-admin`; `Item<EpochId>` under `initial-epoch-id`; `Item<LastSubmission>` under `last-submission`; `Item<Addr>` under `mixnet-contract`; `Item<u32>` under `authorised-count`; `Map<&Addr, NetworkMonitorDetails>` under `authorised`; `Map<&Addr, RetiredNetworkMonitor>` under `retired`; `Map<&Addr, NetworkMonitorSubmissionMetadata>` under `submission-metadata`; `Map<(EpochId, NodeId), EpochNodeMeasurements>` under `pr`; `Map<NodeId, EpochId>` under `last-known-epoch`; `Map<EpochId, Weights>` under `weights`; and the raw accumulator bytes under the unprefixed key `digest_state`. Only the bundle map's namespace is abbreviated, because it is the only one paid once per `(epoch, node)`.

#### Scenario: Namespaces are as declared
- **WHEN** the storage-key constants are read
- **THEN** they equal the names above, the bundle map's is `pr`, and the digest key is `digest_state`

### Requirement: Public event and response-data surface

`BatchSubmit` SHALL set `BatchSubmissionResult` as response data and emit an event `batch_performance_submission` with attributes `accepted_scores` and `non_existent_nodes` (the `Debug` rendering of the list). `UpdateWeights` SHALL emit `weights_update` with `effective_from` and `weights`. `UpdateAdmin` SHALL carry the cw-controllers admin-update attributes. `Submit`, `AuthoriseNetworkMonitor`, `RetireNetworkMonitor` and `OnNymNodeUnbond` SHALL return an empty response.

#### Scenario: Batch submission is observable from the response
- **WHEN** a batch is accepted with two bonded and one non-bonded node
- **THEN** the response data parses as `BatchSubmissionResult { accepted_scores: 2, non_existent_nodes: [..] }` and the event carries `accepted_scores = "2"`

### Requirement: Public error variants

`NymPerformanceContractError` SHALL expose exactly: `FailedMigration { comment }`, `Admin(AdminError)`, `StdErr(StdError)`, `AlreadyAuthorised { address }`, `NotAuthorised { address }`, `StalePerformanceSubmission { epoch_id, node_id, last_epoch_id, last_node_id }`, `UnsortedBatchSubmission`, `NodeNotBonded { node_id }`, `EpochNotCurrent { epoch_id, current_epoch_id }`, `EpochInTransition { epoch_id }`, `EmptyNodeSubmission { node_id }`, `WeightsDoNotSumToOne { total: Decimal }`, `UnauthorisedMixnetCallback { sender: Addr }` and `CorruptDigestState`. Each condition named in this specification MUST surface as its own variant rather than through an opaque catch-all.

#### Scenario: Errors are distinguishable
- **WHEN** a handler rejects a call for any reason listed above
- **THEN** the returned error compares equal to the named variant with the stated fields

## REMOVED Requirements

### Requirement: Admin removals are an escape hatch that does not touch the pointer or the weights

**Reason**: Removals were the store's only retention backstop, but with a digest over each node's latest bundle they cost more than they give. They create the one state in which a pointer names a missing bundle, which every reader of the pointer (the digest maintenance, the bulk query, `RewardingInputs`) would have to handle. Removing recent epochs would also move the fallback anchor R, so audits of later epochs would stop being reproducible. They were never usable as a bad-data escape hatch either: a removal strips every monitor's values for the node, which leaves rewarding nothing to resolve, and the remedy for a misbehaving monitor is `RetireNetworkMonitor`. Bundles are now append-only.

**Migration**: None needed, since the contract is not deployed and nothing outside the contract and the validator client calls `RemoveNodeMeasurements` or `RemoveEpochMeasurements`. Their validator-client methods, `RemoveEpochMeasurementsResponse` and `EPOCH_PERFORMANCE_PURGE_LIMIT` are deleted. If storage growth ever calls for retention, it comes back as a new message in a later code migration, designed against the digest: for example, only epochs well behind current and never a bundle that a pointer names.

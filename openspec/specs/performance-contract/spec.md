# performance-contract Specification

## Purpose

The on-chain per-epoch performance store. Authorised network monitors submit, for the current mixnet epoch only, one bundle per node carrying any subset of the measurement kinds (liveness, stress, config), and bundles are append-only. The contract keeps every monitor's value per kind, serves the per-kind medians and their combined score under the weights in force at each epoch, and resolves deterministically what rewarding used for a node in an epoch, with a fallback to the node's newest earlier bundle bounded below the newest epoch that holds any data, so that a global outage of any length leaves every node at its last value. It also commits each node's latest entry, the score of its newest bundle under that epoch's weights, to an LtHash digest stored raw for ICS23 proofs, which a client verifies by folding the bulk enumeration of those entries; the mixnet contract's unbond callback takes a node out of this routing view for good.

## Requirements

### Requirement: Contract instantiation persists the mixnet contract address, admin, creation epoch, initial monitors and initial weights

The contract SHALL be instantiable via the standard CosmWasm `instantiate` entry point with `InstantiateMsg { mixnet_contract_address: String, authorised_network_monitors: Vec<String>, initial_weights: Weights }`. The handler MUST record the cw2 contract name `crate:nym-performance-contract` and `CARGO_PKG_VERSION`, record build information via `set_build_information!`, validate `mixnet_contract_address` as an `Addr` and persist it, set `info.sender` as the contract admin via `cw-controllers::Admin`, query the mixnet contract for the current absolute epoch and persist it as the creation epoch, persist `LastSubmission { block_height: env.block.height, block_time: env.block.time, data: None }`, initialise the authorised-monitor count to zero, authorise every listed monitor exactly as `AuthoriseNetworkMonitor` would (validating each as an `Addr`), validate `initial_weights` under the weights rules and persist them in the weights map under the creation epoch. It SHALL return `Response::default()`. Any failure MUST persist nothing.

#### Scenario: Valid instantiation persists every item
- **WHEN** `instantiate` is called by `S` with a valid mixnet contract address while the mixnet reports epoch `E`, two monitor addresses, and `initial_weights = { Liveness: 70%, Stress: 30% }`
- **THEN** `S` passes `Admin::assert_admin`, the mixnet address is stored, the creation epoch is `E`, both monitors are authorised with cursors `{ E, 0 }` and the count is 2, `LastSubmission.data` is `None`, and `WeightsAt { epoch_id: E }` returns `EpochWeights { effective_from: E, weights: { Liveness: 70%, Stress: 30% } }`
- **AND** `cw2::get_contract_version` returns the crate's `CARGO_PKG_VERSION`

#### Scenario: Invalid mixnet contract address is rejected
- **WHEN** `instantiate` is called with a `mixnet_contract_address` that fails `Addr::validate`
- **THEN** the call returns an error and no state is persisted

#### Scenario: Invalid initial weights reject instantiation
- **WHEN** `instantiate` is called with `initial_weights = { Liveness: 70% }`
- **THEN** the call fails with `WeightsDoNotSumToOne { total: 0.7 }` and no state is persisted

### Requirement: Migration refreshes build information and forbids version downgrades

The contract SHALL expose a `migrate` entry point taking an empty `MigrateMsg`. It MUST refresh build information via `set_build_information!` and MUST call `cw2::ensure_from_older_version` with the contract name and `CARGO_PKG_VERSION`, rejecting any on-chain version strictly greater than the current one. It SHALL perform no state migration; the queued-migrations module is empty.

#### Scenario: Equal or older on-chain version is accepted
- **WHEN** `migrate` is called against an on-chain cw2 version less than or equal to `CARGO_PKG_VERSION`
- **THEN** the call succeeds and build information is refreshed

#### Scenario: Newer on-chain version is rejected
- **WHEN** `migrate` is called against an on-chain cw2 version strictly greater than `CARGO_PKG_VERSION`
- **THEN** the call returns an error and storage is unchanged

### Requirement: The admin is replaced only by the current admin, and is queryable

`ExecuteMsg::UpdateAdmin { admin }` MUST validate `admin` as an `Addr` and delegate to `cw-controllers::Admin::execute_update_admin`, which rejects any sender other than the current admin with `Admin(AdminError::NotAdmin)` and returns that controller's response attributes. `QueryMsg::Admin {}` SHALL return `cw_controllers::AdminResponse`.

#### Scenario: Only the current admin may update
- **WHEN** a non-admin sends `UpdateAdmin { admin: X }`
- **THEN** the call fails with `Admin(AdminError::NotAdmin {})` and `Admin {}` still returns the previous admin

#### Scenario: The admin hands over
- **WHEN** the current admin sends `UpdateAdmin { admin: X }` with a valid address
- **THEN** `Admin {}` returns `X` and only `X` may subsequently pass `Admin::assert_admin`

#### Scenario: A malformed address is rejected
- **WHEN** the admin sends `UpdateAdmin` with `"definitely-not-valid-account"` or `""`
- **THEN** the call returns an error and the admin is unchanged

### Requirement: The contract depends on the mixnet contract for the epoch clock and node existence

The contract SHALL read the mixnet contract at the stored address through `MixnetContractQuerier` for exactly three facts, each a raw read of the mixnet contract's state rather than a smart query. `query_current_absolute_mixnet_epoch_id` (the interval under `ci`) SHALL be the epoch clock used for the creation epoch, the writable-epoch check, cursor initialisation at authorisation, the effective epoch of a weights update, and `CurrentWeights`. `query_current_mixnet_epoch_status` (the status under `ces`) SHALL say whether the current epoch is in progress, and is used only by the writable-epoch check. `check_node_existence` (the bond under `nn`) SHALL decide whether a node is bonded, and it treats a node in the unbonding state as non-existent. The contract MUST NOT derive epoch identity from block time or height.

#### Scenario: The epoch clock is the mixnet contract's
- **WHEN** the mixnet contract's interval is advanced by seven epochs
- **THEN** a submission for the previous epoch fails with `EpochNotCurrent` and a submission for the new one succeeds

#### Scenario: A transition seals the current epoch
- **WHEN** the mixnet contract's epoch status is `Rewarding`, `ReconcilingEvents` or `RoleAssignment` while it reports epoch 10
- **THEN** a submission for epoch 10 fails with `EpochInTransition { epoch_id: 10 }`, and once the status is `InProgress` again with the interval advanced to 11 a submission for epoch 11 succeeds

#### Scenario: An unbonding node counts as not bonded
- **WHEN** node 7 has begun unbonding and an authorised monitor submits it for the current epoch
- **THEN** `Submit` fails with `NodeNotBonded { node_id: 7 }`

### Requirement: Measurement kinds are a fixed set of named fields split into routing kinds and the config multiplier

The kinds SHALL be `liveness`, `stress` and `config`, expressed as one optional field each on every per-kind type: `Measurements` (a submission's values, `Option<Percent>`), `EpochNodeMeasurements` (the stored bundle, `Option<NodeResults>`) and `KindMedians` (medians, `Option<Percent>`). `Measurements` and `EpochNodeMeasurements` SHALL serialise the fields via `#[serde(rename)]` as `"l"`, `"s"` and `"c"`, MUST omit an absent field from the JSON, and MUST read a missing field back as `None`; `KindMedians` keeps full field names. `liveness` and `stress` SHALL be the routing kinds, the only kinds that carry a weight; `config` SHALL be the multiplier and has no weight field, so it cannot be weighted by construction. There is no enum of kinds, because the contract's JSON codec cannot deserialise an enum used as a map key. Adding a kind SHALL be a new optional field on each per-kind type and MUST NOT require migrating stored bundles or weights.

#### Scenario: Absent kinds are omitted and read back as absent
- **WHEN** a `Measurements` with `liveness = 50%` and `config = 100%` and no `stress` is serialised
- **THEN** the JSON is exactly `{"l":"0.5","c":"1"}`, deserialising it yields `stress = None`, and `{}` deserialises to a value with every kind absent

### Requirement: A submission carries one node and a non-empty map of kinds, and a batch is strictly ordered

The submission payload SHALL be `NodeSubmission { node_id: NodeId, measurements: Measurements }`, serialised with field renames `"n"` and `"m"`. `ExecuteMsg::Submit { epoch, data: NodeSubmission }` SHALL submit one node and `ExecuteMsg::BatchSubmit { epoch, data: Vec<NodeSubmission> }` SHALL submit many. A `NodeSubmission` whose `measurements` has every kind absent MUST be rejected with `EmptyNodeSubmission { node_id }`. A batch MUST be sorted by strictly ascending `node_id` and MUST be rejected as a whole with `UnsortedBatchSubmission` otherwise, including on a duplicate. A batch with an empty `data` vector SHALL succeed with `BatchSubmissionResult::default()` and MUST NOT touch the cursor or the last submission.

#### Scenario: A submission may carry any subset of kinds
- **WHEN** an authorised monitor submits `{ n: 7, m: { Liveness: 95%, Config: 100% } }` for the current epoch and node 7 is bonded
- **THEN** the bundle for `(epoch, 7)` holds `Liveness: [95%]` and `Config: [100%]` and no `Stress` entry

#### Scenario: A submission with every kind absent is rejected
- **WHEN** an authorised monitor submits `{ n: 7, m: {} }`
- **THEN** the call fails with `EmptyNodeSubmission { node_id: 7 }` and the monitor's cursor is unchanged

#### Scenario: A batch out of order is rejected as a whole
- **WHEN** an authorised monitor batch-submits nodes `[3, 5, 4]` for the current epoch
- **THEN** the call fails with `UnsortedBatchSubmission` and no bundle is written for any of the three

#### Scenario: An empty batch is a no-op
- **WHEN** an authorised monitor batch-submits `data: []`
- **THEN** the call succeeds with `accepted_scores: 0`, and the cursor and `LastSubmission` are unchanged

### Requirement: Submissions are validated in a fixed order: sender, epoch, cursor, then per node

`Submit` and `BatchSubmit` MUST check, in this order and stopping at the first failure: that the sender is an authorised monitor (`NotAuthorised { address }`); that `epoch` equals the current mixnet epoch (`EpochNotCurrent { epoch_id, current_epoch_id }`); that the mixnet epoch is in progress (`EpochInTransition { epoch_id }`); that the submission is not stale against the sender's cursor, using the first node of a batch (`StalePerformanceSubmission`); and then, per node in order, that the payload is non-empty and, for a batch, in strictly ascending order, that the node is bonded, and finally the insert. `Submit` for a non-bonded node MUST fail with `NodeNotBonded { node_id }`. `BatchSubmit` MUST skip a non-bonded node, list it in `BatchSubmissionResult { accepted_scores, non_existent_nodes }`, and continue.

#### Scenario: An unauthorised sender is rejected before anything else
- **WHEN** an address that was never authorised, or has been retired, submits for a past epoch with an empty map
- **THEN** the call fails with `NotAuthorised { address }`

#### Scenario: A batch skips non-bonded nodes and reports them
- **WHEN** an authorised monitor batch-submits nodes `[1, 2, 3]` for the current epoch and node 2 is not bonded
- **THEN** bundles are written for 1 and 3, the response data is `BatchSubmissionResult { accepted_scores: 2, non_existent_nodes: [2] }`, and the cursor advances to node 3

### Requirement: Only the current mixnet epoch is writable, and only while it is in progress, so a bundle is frozen once its transition begins

A submission MUST fail with `EpochNotCurrent { epoch_id, current_epoch_id }` when its `epoch` differs from the current mixnet epoch, whether earlier or later, and with `EpochInTransition { epoch_id }` when the epoch is current but the mixnet epoch status is not `InProgress`. Consequently a bundle for epoch `X` MUST be immutable from the moment `BeginEpochTransition` for `X` executes in the mixnet contract, which precedes every `reward_node` for `X`, and the contract SHALL never hold a value for an epoch that was submitted after its transition began.

#### Scenario: A past epoch is rejected
- **WHEN** the mixnet reports epoch 10 and an authorised monitor submits for epoch 9
- **THEN** the call fails with `EpochNotCurrent { epoch_id: 9, current_epoch_id: 10 }` and no state changes

#### Scenario: A future epoch is rejected
- **WHEN** the mixnet reports epoch 10 and an authorised monitor submits for epoch 11
- **THEN** the call fails with `EpochNotCurrent { epoch_id: 11, current_epoch_id: 10 }`

#### Scenario: A submission during the epoch transition is rejected
- **WHEN** the mixnet reports epoch 10 with status `Rewarding` and an authorised monitor submits for epoch 10
- **THEN** the call fails with `EpochInTransition { epoch_id: 10 }`, the monitor's cursor is unchanged and no bundle is written

#### Scenario: A bundle is frozen once the epoch advances
- **WHEN** node 7 has a bundle for epoch 10 from one monitor, the mixnet advances to 11, and a second monitor submits node 7 for epoch 10
- **THEN** the second submission fails with `EpochNotCurrent` and the epoch 10 bundle still holds exactly one value per kind

### Requirement: The per-monitor cursor prevents resubmission and out-of-order submission within an epoch

Each authorised monitor SHALL have `NetworkMonitorSubmissionMetadata { last_submitted_epoch_id, last_submitted_node_id }` in the submission-metadata map, initialised at authorisation to `{ current mixnet epoch, 0 }`. A submission MUST fail with `StalePerformanceSubmission { epoch_id, node_id, last_epoch_id, last_node_id }` when its epoch is earlier than `last_submitted_epoch_id`, or when its epoch equals it and its first `node_id` is not greater than `last_submitted_node_id`. After a successful submission the cursor MUST be set to the epoch and the last node in the call, whether or not that node was bonded. Because a node is submitted once per epoch with all of its kinds together, the cursor needs no kind dimension and a monitor contributes at most one value per `(node, kind)` per epoch.

#### Scenario: The same node cannot be submitted twice in an epoch
- **WHEN** an authorised monitor submits node 7 for the current epoch and then submits node 7 again for the same epoch with different kinds
- **THEN** the second call fails with `StalePerformanceSubmission { epoch_id, node_id: 7, last_epoch_id: epoch_id, last_node_id: 7 }`

#### Scenario: Nodes must ascend within an epoch
- **WHEN** an authorised monitor submits node 9 and then node 4 for the same epoch
- **THEN** the second call fails with `StalePerformanceSubmission { .., node_id: 4, last_node_id: 9 }`

#### Scenario: A new epoch resets the node ordering
- **WHEN** an authorised monitor submitted node 9 for epoch 10 and the mixnet advances to 11
- **THEN** a submission of node 4 for epoch 11 succeeds and the cursor becomes `{ 11, 4 }`

### Requirement: Storage holds one bundle per epoch and node, merged kind by kind

Bundles SHALL be stored as `Map<(EpochId, NodeId), EpochNodeMeasurements>` under the namespace `pr`, where `EpochNodeMeasurements { liveness, stress, config: Option<NodeResults> }` holds one optional `NodeResults` per kind, serialised as `"l"`, `"s"` and `"c"` with absent kinds omitted. `NodeResults` SHALL keep its values sorted and never empty, and SHALL store each value as an integer percent in `0..=100` (a `u8`) serialised as a JSON array of integers, because the two-decimal rounding already collapses the domain to those 101 values; its Rust API SHALL expose `Percent`, converting with `round_to_two_decimal_places().round_to_integer()` on insert and `Percent::from_percentage_value` on read. Inserting a submission MUST, for each kind present in it, round and convert the value and insert it into that kind's `NodeResults` in sorted position, creating it from the single value when the kind is absent. The median of a kind SHALL be the middle value for an odd count and the two-decimal rounding of the average of the two middle values for an even count, returned as a `Percent`.

#### Scenario: Two monitors reporting different kind subsets merge into one bundle
- **WHEN** monitor A submits node 7 with `{ Liveness: 90%, Config: 100% }` and monitor B submits node 7 with `{ Liveness: 80%, Stress: 70%, Config: 100% }` in the same epoch
- **THEN** the bundle holds `Liveness: [80%, 90%]`, `Stress: [70%]`, `Config: [100%, 100%]`

#### Scenario: Values are stored as integer percents
- **WHEN** monitors submit `Liveness` values `0.93`, `0.95` and `0.97` for one node and epoch
- **THEN** the stored JSON for that kind is `[93,95,97]` and the values read back through the Rust API as `Percent` `93%`, `95%` and `97%`, with `0` and `100` round-tripping likewise

#### Scenario: Values are rounded half-up on insert and kept sorted
- **WHEN** monitors submit `Liveness` values `0.955`, `0.10` and `0.5` for the same node and epoch, in that order
- **THEN** the bundle's `Liveness` results are `[10%, 50%, 96%]`

#### Scenario: Medians follow the existing rule
- **WHEN** a kind holds `[10%, 20%]`, `[10%, 20%, 30%]` and `[0%, 0%, 100%, 100%, 100%, 100%, 100%]` respectively
- **THEN** its medians are `15%`, `20%` and `100%`

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

### Requirement: The last submission is recorded on every successful submission

Every successful non-empty `Submit` or `BatchSubmit` MUST overwrite the `LastSubmission` item with `env.block.height`, `env.block.time` and `LastSubmittedData { sender, epoch_id, data }` where `data` is the last `NodeSubmission` of the call. `QueryMsg::LastSubmittedMeasurement {}` SHALL return it.

#### Scenario: A batch records its last entry
- **WHEN** monitor M batch-submits nodes `[1, 2, 3]` for epoch 10
- **THEN** `LastSubmittedMeasurement` returns the current block height and time, `sender: M`, `epoch_id: 10`, and `data.node_id: 3`

#### Scenario: A fresh contract reports no data
- **WHEN** no submission has been made since instantiation
- **THEN** `LastSubmittedMeasurement` returns the instantiation block height and time with `data: None`

### Requirement: Weights are validated, admin-updated, and take effect from the next epoch

`Weights` SHALL be a struct with one `Percent` field per routing kind, `liveness` and `stress`, each `#[serde(default)]` so that a field absent from a message deserialises as zero. A zero weight means the kind does not contribute. Validation MUST reject a sum of weights not exactly equal to one with `WeightsDoNotSumToOne { total }` where `total` is the `Decimal` sum, all weights zero included. Each weight is in `[0, 1]` by construction of `Percent`. `ExecuteMsg::UpdateWeights { weights }` MUST call `Admin::assert_admin`, validate, query the current mixnet epoch `C`, store the weights under `C + 1` in the weights map overwriting any entry at that key, and emit a `weights_update` event with attributes `effective_from` and `weights` (the JSON rendering).

#### Scenario: An update takes effect from the next epoch
- **WHEN** `{ Liveness: 100% }` is in force, the mixnet reports epoch 10, and the admin executes `UpdateWeights { weights: { Liveness: 70%, Stress: 30% } }`
- **THEN** `WeightsAt { epoch_id: 10 }` returns the `Liveness: 100%` entry and `WeightsAt { epoch_id: 11 }` returns `EpochWeights { effective_from: 11, weights: { Liveness: 70%, Stress: 30% } }`
- **AND** the response carries a `weights_update` event with `effective_from = "11"`

#### Scenario: Two updates in one epoch overwrite
- **WHEN** the admin executes `UpdateWeights` twice while the mixnet reports epoch 10, first `{ Liveness: 60%, Stress: 40% }` then `{ Liveness: 50%, Stress: 50% }`
- **THEN** `WeightsAt { epoch_id: 11 }` returns the `50%/50%` entry and no `60%/40%` entry exists

#### Scenario: Non-admin is rejected
- **WHEN** a non-admin executes `UpdateWeights`
- **THEN** the call fails with `Admin(AdminError::NotAdmin {})` and the weights map is unchanged

#### Scenario: All-zero weights are rejected and a missing field is zero
- **WHEN** the admin executes `UpdateWeights { weights: { liveness: 0%, stress: 0% } }`
- **THEN** the call fails with `WeightsDoNotSumToOne { total: 0 }`
- **AND** a message carrying only `{ liveness: 100% }` deserialises with `stress` zero and is accepted

#### Scenario: Weights that do not sum to one are rejected exactly
- **WHEN** the admin executes `UpdateWeights { weights: { Liveness: 70%, Stress: 20% } }`
- **THEN** the call fails with `WeightsDoNotSumToOne { total: 0.9 }`
- **AND** `{ Liveness: 70%, Stress: 30% }` is accepted, the sum being exactly one in `Decimal`

### Requirement: Weights are resolved per epoch by the latest entry at or before it

`QueryMsg::WeightsAt { epoch_id }` SHALL return `WeightsResponse { weights: Option<EpochWeights> }` where `EpochWeights { effective_from, weights }` is the entry with the greatest key not greater than `epoch_id`, found by a descending range bounded inclusively at `epoch_id` taking one, and `None` when no entry is at or before `epoch_id`. `QueryMsg::CurrentWeights {}` SHALL return the same for the current mixnet epoch.

#### Scenario: Resolution picks the latest entry at or before the epoch
- **WHEN** the weights map holds entries effective from 0 and 11
- **THEN** `WeightsAt { 10 }` returns the entry effective from 0, and `WeightsAt { 11 }` and `WeightsAt { 500 }` return the entry effective from 11

#### Scenario: An epoch before the creation epoch has no weights
- **WHEN** the contract was created at epoch 5
- **THEN** `WeightsAt { 4 }` returns `weights: None`

### Requirement: The score is the renormalised weighted mean of the applied routing kinds times the config median

Given per-kind medians and an `EpochWeights`, the applied set SHALL be the routing kinds whose weight is non-zero and which are present in the medians. The score SHALL be `None` when the applied set is empty or when the medians hold no `Config`. Otherwise the score SHALL be `(sum over applied of weight * median) / (sum over applied of weight)`, multiplied by the `Config` median, computed in `Decimal` and rounded to two decimal places with `round_to_two_decimal_places`. A kind present in the medians whose weight is zero MUST NOT contribute. A weighted kind absent from the medians MUST be renormalised away. This mirrors nym-api's `PerformanceComponents::performance`.

#### Scenario: A single applied kind reproduces its median times config
- **WHEN** medians are `{ Liveness: 80%, Config: 50% }` and weights are `{ Liveness: 70%, Stress: 30% }`
- **THEN** the score is `40%`, because `Stress` is renormalised away and `0.8 * 0.5 = 0.4`

#### Scenario: Two applied kinds use their declared shares
- **WHEN** medians are `{ Liveness: 90%, Stress: 50%, Config: 100% }` and weights are `{ Liveness: 70%, Stress: 30% }`
- **THEN** the score is `78%`

#### Scenario: Config gates every kind
- **WHEN** medians are `{ Liveness: 100%, Stress: 100%, Config: 50% }` and weights are `{ Liveness: 70%, Stress: 30% }`
- **THEN** the score is exactly `50%`

#### Scenario: An unweighted kind does not contribute
- **WHEN** medians are `{ Liveness: 100%, Stress: 0%, Config: 100% }` and weights are `{ Liveness: 100% }`
- **THEN** the score is `100%`

#### Scenario: No applied routing kind yields no score
- **WHEN** medians are `{ Config: 100% }`, or medians are `{ Stress: 90%, Config: 100% }` while weights are `{ Liveness: 100% }`
- **THEN** the score is `None`

#### Scenario: Missing config yields no score
- **WHEN** medians are `{ Liveness: 90% }` and weights are `{ Liveness: 100% }`
- **THEN** the score is `None`

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

### Requirement: Exact-epoch queries return the raw bundle and the per-kind medians without fallback

`QueryMsg::NodeMeasurements { epoch_id, node_id }` SHALL return `NodeMeasurementsResponse { measurements: Option<EpochNodeMeasurements> }`, the stored bundle verbatim, so its values appear as integer percents. `QueryMsg::NodePerformance { epoch_id, node_id }` SHALL return `NodePerformanceResponse { performance: Option<EpochNodePerformance> }` where `EpochNodePerformance { epoch_id, medians, score }` holds the median of each kind present and the score under `WeightsAt { epoch_id }`. Neither query MUST fall back to another epoch.

#### Scenario: Medians are per kind over the monitors that reported that kind
- **WHEN** the bundle at `(10, 7)` is `Liveness: [80%, 90%]`, `Stress: [70%]`, `Config: [100%, 100%]`
- **THEN** `NodePerformance { 10, 7 }` returns medians `{ Liveness: 85%, Stress: 70%, Config: 100% }`

#### Scenario: A missing epoch is absent rather than borrowed
- **WHEN** node 7 has a bundle at epoch 9 and none at 10
- **THEN** `NodePerformance { 10, 7 }` returns `performance: None` and `NodeMeasurements { 10, 7 }` returns `measurements: None`

### Requirement: Node history is bounded by the pointer and omits gaps

`QueryMsg::NodePerformancePaged { node_id, start_after, limit }` SHALL return `NodePerformancePagedResponse { node_id, performance: Vec<EpochNodePerformance>, start_next_after }`. The walk SHALL start at the creation epoch, or `start_after + 1`, and SHALL end at the node's last-known-epoch pointer, returning an empty page with `start_next_after: None` when the pointer is absent or below the start. It MUST visit at most `limit` epochs, MUST include only epochs that have a bundle, and MUST set `start_next_after` to the last epoch visited when that is below the pointer and `None` otherwise.

#### Scenario: Gaps are omitted and the walk stops at the pointer
- **WHEN** node 7 has bundles at epochs 2, 3 and 5, the pointer is 5, and the mixnet reports epoch 40
- **THEN** `NodePerformancePaged { 7, None, None }` returns entries for 2, 3 and 5 only with `start_next_after: None`

#### Scenario: Pagination resumes from the last epoch visited
- **WHEN** node 7 has bundles at epochs 2, 3 and 5 and the query uses `start_after: Some(1), limit: 2`
- **THEN** the page holds 2 and 3 with `start_next_after: Some(3)`, and the page from `start_after: Some(3)` holds 5 with `start_next_after: None`
- **AND** a `limit: 2` page from the very start visits the empty epochs 0 and 1, holds nothing, and reports `start_next_after: Some(1)`, because the limit bounds the epochs visited rather than the entries returned

#### Scenario: Starting past the pointer returns nothing
- **WHEN** node 7's pointer is 5 and the query uses `start_after: Some(5)` or `Some(42)`
- **THEN** the page is empty with `start_next_after: None`

### Requirement: Epoch-wide and full-history pages carry per-node medians and scores

`QueryMsg::EpochMeasurementsPaged { epoch_id, start_after, limit }` SHALL return that epoch's bundles keyed by node in ascending order via a prefix range with an exclusive lower bound, as `EpochMeasurementsPagedResponse { epoch_id, measurements: Vec<NodeMeasurements { node_id, measurements }>, start_next_after }`. `QueryMsg::EpochPerformancePaged` SHALL return the same nodes as `Vec<NodePerformance { node_id, medians, score }>` with every score under `WeightsAt { epoch_id }` resolved once per page. `QueryMsg::FullHistoricalPerformancePaged { start_after: Option<(EpochId, NodeId)>, limit }` SHALL range over the whole bundle map in `(epoch, node)` order returning `Vec<HistoricalPerformance { epoch_id, node_id, medians, score }>`, each score under the weights in force at its own epoch. `start_next_after` SHALL be the last key returned, or `None` for an empty page.

#### Scenario: An epoch page scores every node under that epoch's weights
- **WHEN** epoch 10 holds bundles for nodes 1, 2 and 3 and `{ Liveness: 100% }` is in force at 10
- **THEN** `EpochPerformancePaged { 10, None, None }` returns three entries in node order, each with `score = liveness median * config median`, and `start_next_after: Some(3)`

#### Scenario: An epoch page resumes after the given node
- **WHEN** epoch 10 holds bundles for nodes 1, 2, 3, 5 and 6 and the query uses `start_after: Some(3)`
- **THEN** the page holds nodes 5 and 6 with `start_next_after: Some(6)`

#### Scenario: Full history spans epochs in key order
- **WHEN** bundles exist at `(9, 5)`, `(10, 1)` and `(10, 4)`
- **THEN** `FullHistoricalPerformancePaged { None, None }` returns them in that order with `start_next_after: Some((10, 4))`

### Requirement: Network monitors are authorised and retired by the admin, and are queryable

`ExecuteMsg::AuthoriseNetworkMonitor { address }` MUST validate `address` as an `Addr`, call `Admin::assert_admin`, fail with `AlreadyAuthorised { address }` if already authorised, remove any retired record for it, increment the authorised count, save `NetworkMonitorDetails { address, authorised_by: sender, authorised_at_height: env.block.height }`, and initialise the cursor to `{ current mixnet epoch, 0 }`. `ExecuteMsg::RetireNetworkMonitor { address }` MUST validate the address, call `Admin::assert_admin`, fail if the address is not authorised, remove it from the authorised map, decrement the count, and save `RetiredNetworkMonitor { details, retired_by: sender, retired_at_height }`. `QueryMsg::NetworkMonitor { address }` SHALL return `NetworkMonitorResponse { info: Option<NetworkMonitorInformation { details, current_submission_metadata }> }`, and `NetworkMonitorsPaged` and `RetiredNetworkMonitorsPaged` SHALL page the respective maps by address with `start_next_after` as the last address returned.

#### Scenario: Only the admin may authorise or retire
- **WHEN** a non-admin sends `AuthoriseNetworkMonitor` or `RetireNetworkMonitor`
- **THEN** the call fails with `Admin(AdminError::NotAdmin {})`

#### Scenario: Double authorisation is rejected
- **WHEN** the admin authorises M twice
- **THEN** the second call fails with `AlreadyAuthorised { address: M }` and the count is 1

#### Scenario: A retired monitor can no longer submit and can be re-authorised
- **WHEN** the admin retires M, M submits, and the admin re-authorises M
- **THEN** M's submission fails with `NotAuthorised`, and after re-authorisation M is absent from the retired map, present in the authorised map, and its cursor is `{ current epoch, 0 }`

#### Scenario: A monitor's information includes its cursor
- **WHEN** M has submitted node 9 for epoch 10
- **THEN** `NetworkMonitor { M }` returns its details and `current_submission_metadata = { 10, 9 }`

### Requirement: Paged queries apply the existing default and maximum limits

Every paged query SHALL use `limit.unwrap_or(DEFAULT).min(MAX)` with the `retrieval_limits`: node performance 100/200, epoch performance 100/200, epoch measurements 50/100, full historical performance 100/200, latest performance 100/200, network monitors 50/100, and retired network monitors 50/100.

#### Scenario: A limit above the maximum is capped
- **WHEN** `EpochPerformancePaged` is called with `limit: Some(10_000)`
- **THEN** at most 200 entries are returned

#### Scenario: The latest-performance page is capped
- **WHEN** `LatestPerformancePaged` is called with `limit: Some(10_000)` while more than 200 nodes are in the committed set
- **THEN** the page holds 200 entries and `start_next_after` is the 200th node

### Requirement: The validator-client exposes one method per execute and query variant

`PerformanceSigningClient` SHALL expose `update_weights` and `on_nym_node_unbond`, and its `submit_performance` / `batch_submit_performance` SHALL take `NodeSubmission` payloads. `PerformanceQueryClient` SHALL expose `get_rewarding_inputs`, `get_rewarding_score`, `get_last_known_epoch`, `get_weights_at`, `get_current_weights`, `get_latest_performance_paged` and `get_performance_digest` (named apart from the directory query client's `get_digest`, since a verifier imports both traits), and `PagedPerformanceQueryClient` SHALL keep a collector per paged query, including `get_all_latest_performance`. The existing `all_execute_variants_are_covered` and `all_query_variants_are_covered` tests MUST enumerate every variant, so a message variant without a client method fails compilation.

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

`NymPerformanceContractError` SHALL expose exactly: `FailedMigration { comment }`, `Admin(AdminError)`, `StdErr(StdError)`, `AlreadyAuthorised { address }`, `NotAuthorised { address }`, `StalePerformanceSubmission { epoch_id, node_id, last_epoch_id, last_node_id }`, `UnsortedBatchSubmission`, `NodeNotBonded { node_id }`, `EpochNotCurrent { epoch_id, current_epoch_id }`, `EpochInTransition { epoch_id }`, `EmptyNodeSubmission { node_id }`, `WeightsDoNotSumToOne { total: Decimal }`, `UnauthorisedMixnetCallback { sender: Addr }` and `CorruptDigestState(CorruptDigestState)`, wrapping the error type `nym-contracts-common` shares with the directory and geolocation contracts. Each condition named in this specification MUST surface as its own variant rather than through an opaque catch-all.

#### Scenario: Errors are distinguishable
- **WHEN** a handler rejects a call for any reason listed above
- **THEN** the returned error compares equal to the named variant with the stated fields

### Requirement: The digest commits to every node's latest performance entry

The contract SHALL maintain an LtHash16 accumulator (`nym-lthash`) that, after every successful execute message, equals the sum of one leaf per node in the committed set and nothing else. The committed set SHALL be every node with an entry `p` in `last-known-epoch`, and its entry SHALL be `LatestPerformance { node_id, epoch_id: p, score }`, where `score` is the score of the per-kind medians of the bundle at `(p, node)` under `WeightsAt { epoch_id: p }` per the score requirement, `None` when the score is. The entry MUST NOT commit the medians themselves. Bundles are never removed, so that bundle always exists. Older bundles of a node MUST NOT contribute to the digest, and a node the mixnet contract has called back for has no pointer and so is not in the committed set. Only submissions and the unbond callback SHALL change the committed set.

#### Scenario: Recomputing the committed entries reproduces the digest
- **WHEN** any sequence of single and batch submissions by several monitors across several epochs, `UpdateWeights` and unbond callbacks executes
- **THEN** after every step, folding the leaf of every entry that `LatestPerformancePaged` returns, paged to the end, into an empty accumulator yields exactly the stored accumulator

#### Scenario: Older bundles are not committed
- **WHEN** node 7 is the only node and has bundles at epochs 9 and 10
- **THEN** the stored accumulator equals the empty accumulator with only the leaf of node 7's epoch 10 entry added

#### Scenario: An unscoreable bundle is committed without a score
- **WHEN** node 7's only bundle holds `Liveness: [90%]` and no `Config`
- **THEN** its committed entry carries `score: None`

### Requirement: The leaf encoding is fixed, exact and shared with verifiers

`LatestPerformance::digest_leaf` in the common crate SHALL be the one leaf encoding, used by the contract and by every verifier. The leaf SHALL be exactly 26 bytes: the tag byte `1`; `node_id` as 4 bytes big-endian; `epoch_id` as 4 bytes big-endian; then the score as a presence byte (`0` absent, `1` present) followed by the 16 bytes big-endian of the `u128` atomics of its `Decimal` value, all zero when absent. The encoding MUST be lossless, so that a served entry differing from the committed one in any field, by any amount, yields a different leaf. It is a frozen wire format: changing it is a breaking migration that re-folds the whole accumulator and takes a new tag byte. It commits no per-kind values, so adding or removing a measurement kind leaves it unchanged. Because the contract recomputes an entry's score from its stored bundle when subtracting it, a change to the score computation MUST leave the score of every stored bundle under its epoch's weights unchanged, or re-fold the accumulator. The accumulator holds a single entry class, so the tag versions the format rather than separating classes, and leaves need not be distinguishable from another contract's, because each contract has its own accumulator and a proof binds to one contract address and key.

#### Scenario: The layout is as documented
- **WHEN** the leaves of `{ node_id: 7, epoch_id: 10, score: 50% }` and `{ node_id: 7, epoch_id: 10, score: None }` are computed
- **THEN** the first is the 26 bytes `01`, `00000007`, `0000000a`, `01` followed by the atomics of `0.5`, and the second ends in `00` followed by 16 zero bytes instead

#### Scenario: Every field is committed
- **WHEN** two entries differ only in the node id, only in the epoch, or only in the score
- **THEN** their leaves differ

#### Scenario: An absent score differs from a present zero
- **WHEN** two entries differ only in that one has `score: None` and the other a score of `0%`
- **THEN** their leaves differ

#### Scenario: A served value cannot drift within a rounding slack
- **WHEN** two entries differ only in that one carries a score of `0.85` and the other `0.8549`
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

For every bundle a submission writes at `(X, node)`, the contract MUST update the accumulator as follows. When the bundle already existed, because another monitor reported the node in `X` first, it MUST subtract the leaf of the entry for the bundle as stored and add the leaf for the merged bundle. When the submission creates the bundle, it MUST subtract the node's previous entry if the node has an entry `p` in `last-known-epoch`, and add the leaf for the new bundle, while the pointer advances to `X` as before. A previous entry at epoch `p` MUST be subtracted with its score recomputed under `WeightsAt { epoch_id: p }`, the weights it was added with. When `p` is at or after the `effective_from` of `WeightsAt { epoch_id: X }`, those are the weights already loaded for `X`; otherwise `WeightsAt { epoch_id: p }` is read. A node the mixnet contract has called back for never reaches this step, because the mixnet contract reports it as not bonded.

#### Scenario: A node's first bundle adds its entry
- **WHEN** node 7 has never been measured and an authorised monitor submits it for epoch 10
- **THEN** the leaf of its epoch 10 entry is added and nothing is subtracted

#### Scenario: A node's next epoch replaces its entry
- **WHEN** node 7 is the only node, has a bundle at epoch 10, and an authorised monitor submits it for epoch 11
- **THEN** the accumulator equals the empty accumulator with only the leaf of node 7's epoch 11 entry added, and the bundle at epoch 10 is still stored

#### Scenario: A second monitor in the same epoch updates the entry
- **WHEN** weights are `{ Liveness: 100% }`, monitor A submits node 7 with `{ Liveness: 90%, Config: 100% }` for epoch 10 and monitor B submits node 7 with `{ Liveness: 80%, Config: 100% }` for epoch 10
- **THEN** the accumulator equals the empty accumulator with only the leaf of `{ node_id: 7, epoch_id: 10, score: 85% }` added

#### Scenario: An entry added under earlier weights is subtracted exactly
- **WHEN** node 7's latest bundle is at epoch 10 under `{ Liveness: 100% }`, the admin sets `{ Liveness: 50%, Stress: 50% }` effective from epoch 11, and node 7 is submitted for epoch 12
- **THEN** the accumulator equals the empty accumulator with only the leaf of node 7's epoch 12 entry, scored under the new weights, added

### Requirement: The mixnet contract's unbond callback removes a node from the digest permanently

`ExecuteMsg::OnNymNodeUnbond { node_id }` MUST fail with `UnauthorisedMixnetCallback { sender }` unless the sender is the stored mixnet contract address. Otherwise, when the node has a pointer `p`, the handler MUST subtract the leaf of its entry at `p` and delete the pointer; when it has none, because it was never measured, the handler MUST change nothing. It MUST NOT remove bundles or touch the weights. `RewardingInputs` and `RewardingScore` therefore answer for the node exactly as before the callback, through the walk their resolution takes for a node without a pointer, so rewarding for the epoch in which the node began unbonding still resolves its data. `LastKnownEpoch` and `NodePerformancePaged` treat the node as never measured, and its bundles stay readable through `NodePerformance`, `NodeMeasurements` and the epoch and full-history pages. The mixnet contract sends the callback once per node, when the node begins unbonding, and from then on reports the node as not bonded; node ids are never reused, so every later submission for the node fails or is skipped as `NodeNotBonded` and it never re-enters the committed set.

The mixnet contract SHALL hold `performance_contract_address: Option<Addr>` in its `ContractState`, set at instantiation from the required `InstantiateMsg.performance_contract_address` and by its migration from the required `MigrateMsg.performance_contract_address`, each validated as an `Addr`, the latter applied unless `unsafe_skip_state_updates` is set. When it is set, `try_remove_nym_node` MUST send `OnNymNodeUnbond { node_id }` to it as a plain message beside the node-families, geolocation and directory callbacks, so that a failing handler reverts the unbond.

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

#### Scenario: Instantiation sets the address
- **WHEN** the mixnet contract is instantiated with `performance_contract_address: P`
- **THEN** its `ContractState.performance_contract_address` is `Some(P)`

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

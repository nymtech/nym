# network-monitor-config-score Specification

## Purpose
TBD - created by archiving change network-monitor-config-score. Update Purpose after archive.
## Requirements
### Requirement: Config score is computed by logic shared with nym-api

The orchestrator SHALL compute a node's config score using scoring logic shared with nym-api, such that the same inputs produce the same score in both systems. Neither system MAY carry an independent copy of the formula, the gates, or the token-sufficiency rule.

The value produced in the orchestrator exists to be compared against nym-api's for the same nodes before anything depends on it. That comparison is only interpretable if a discrepancy points to a difference in the data each system read, never to a difference in how they scored it, so the scoring is shared code rather than a reimplementation.

#### Scenario: Identical inputs produce an identical score
- **WHEN** a node's reported version, binary name, terms acceptance, on-chain balance and feegrant status are the same as nym-api reads for it
- **THEN** the orchestrator's config score equals nym-api's score for that node

#### Scenario: A change to the shared logic reaches both systems
- **WHEN** the shared scoring logic changes
- **THEN** both nym-api and the orchestrator score with the change, with no separate orchestrator copy left to update

### Requirement: The config score is a version-freshness score, gated and penalised

The config score SHALL be `penalty ^ (versions_behind ^ penalty_scaling)`, where `versions_behind` is the weighted distance of the node's reported version from the on-chain version history. The score MUST be zero when the node does not run the `nym-node` binary, when the operator has not accepted the terms and conditions, or when the reported version is missing or does not parse as semver. When the node cannot transact on chain the score MUST be multiplied by `(1 - chain_interactions_penalty)`.

This is nym-api's existing definition, relocated rather than revised. The gates are hard zeros because a node running the wrong binary or refusing the terms has misconfigured itself regardless of how fresh its version is, while the chain-interaction penalty is soft because an inability to transact degrades rather than disqualifies.

#### Scenario: A current, compliant, transacting node scores at the top
- **WHEN** a node runs `nym-node`, has accepted the terms, reports the latest version and holds at least the minimum balance
- **THEN** its config score is 1.0, since it is zero versions behind and takes no penalty

#### Scenario: An unaccepted terms and conditions zeroes the score
- **WHEN** a node has not accepted the operator terms and conditions
- **THEN** its config score is 0.0 regardless of its version or chain standing

#### Scenario: A non-nym-node binary zeroes the score
- **WHEN** a node's self-reported binary name is not `nym-node`
- **THEN** its config score is 0.0

#### Scenario: Missing or unparseable version information zeroes the score
- **WHEN** a node's reported version is missing or does not parse as semver
- **THEN** its config score is 0.0

#### Scenario: Being behind reduces the score
- **WHEN** a node reports a version several releases behind the on-chain head
- **THEN** its `versions_behind` is greater than zero and its config score is below 1.0

### Requirement: A node can transact when it has sufficient tokens or a feegrant

The orchestrator SHALL treat a node as able to transact when its on-chain balance is at least the configured minimum OR it is a feegrant grantee, and MUST apply the chain-interaction penalty only when neither holds.

#### Scenario: Sufficient balance avoids the penalty
- **WHEN** a node's on-chain balance is at least the minimum
- **THEN** no chain-interaction penalty is applied, whether or not it holds a feegrant

#### Scenario: A feegrant avoids the penalty despite a low balance
- **WHEN** a node's balance is below the minimum but it is a feegrant grantee
- **THEN** no chain-interaction penalty is applied

#### Scenario: Neither balance nor feegrant applies the penalty
- **WHEN** a node's balance is below the minimum and it holds no feegrant
- **THEN** its score is multiplied by `(1 - chain_interactions_penalty)`

### Requirement: On-chain standing is cached and the token threshold is applied at score time

The orchestrator SHALL cache the RAW on-chain balance and feegrant status of each described node, kept warm by a background sweep on its own interval independent of the describe refresh and with bounded concurrency, so that materialisation reads the cache rather than querying the chain. It MUST derive token sufficiency by comparing the cached balance against the minimum in force at score time, rather than caching a sufficiency decision. It SHOULD spread refresh due-times across nodes so that a population cached together does not all fall due at once.

Per-node balance and feegrant are chain queries that do not scale to the whole fleet at every epoch, so they are cached behind a time-to-live and refreshed by a warm-keeping sweep rather than lazily on the epoch path, which would burst per-node queries at the transition. Caching the raw balance rather than a sufficiency bool keeps the cached fact separate from the policy, so a change to the threshold takes effect against the cached balances without re-querying. Spreading due-times avoids a synchronised refresh of the whole fleet one interval after a cold start. The sweep is driven from the descriptions, so a node that is no longer bonded, having lost its description, is no longer queried.

#### Scenario: A raised threshold takes effect without re-querying
- **WHEN** the minimum balance is raised so that a node's already-cached balance now falls below it
- **THEN** the next epoch's config score reflects the node as unable to transact, using the cached balance and without a new balance query

#### Scenario: Capabilities refresh on their own schedule
- **WHEN** the capability refresh interval elapses for a node
- **THEN** its balance and feegrant status are re-queried independently of the describe sweep

#### Scenario: Refreshes are spread rather than synchronised
- **WHEN** a population of nodes is cached together in one sweep
- **THEN** their next refreshes fall due spread across a window rather than all at the same instant one interval later

#### Scenario: Only described nodes are queried
- **WHEN** the sweep looks for nodes whose standing is missing or due
- **THEN** it considers only nodes that currently have a description, so a bonded node never described and a node no longer bonded are not queried

### Requirement: Config-score inputs are sourced from self-description and the mixnet contract

The orchestrator SHALL read the reported version, binary name, terms acceptance and on-chain address from each node's self-description on its existing refresh sweep, as part of the same complete reading as the rest of the description, and MUST obtain the version history and formula params from the mixnet contract rather than a pinned local copy. A node that cannot report these inputs, or that reports an on-chain address that is empty or does not parse, MUST NOT be described from that reading.

Sourcing the version history and formula params from the contract means the orchestrator's score tracks governance, so it does not drift from nym-api when those params change. The inputs are required like every other part of the description because a description is only ever written from a complete reading. A node always reports the address of its own account, so a missing or malformed one means its operator broke something, and the node takes the same consequence as any other incomplete describe.

#### Scenario: Describe inputs are captured on refresh
- **WHEN** the orchestrator refreshes a bonded node that answers completely
- **THEN** its description holds that node's reported version, binary name, terms acceptance and on-chain address from the same reading as the rest of it

#### Scenario: An invalid on-chain address fails the describe
- **WHEN** a node reports an on-chain address that is empty or does not parse
- **THEN** the reading is rejected as a whole, exactly like any other incomplete describe

#### Scenario: Formula params track the contract
- **WHEN** the mixnet contract's config-score params or version history change
- **THEN** the orchestrator scores against the new values on its next materialisation

### Requirement: Config score is materialised once per epoch as a snapshot

The orchestrator SHALL compute and persist the config score of each described node as an epoch begins, from the description and cached on-chain standing current at that moment, and MUST serve the persisted value thereafter rather than recomputing it per request. A described node whose on-chain standing is not cached yet MUST be left without a config score for that epoch rather than scored from a guess. Config score MUST be filed only under the epoch in progress, never under an epoch being backfilled. Re-running materialisation for an epoch that already has values MUST NOT alter them.

Config score has no sample window to replay, so its value is a snapshot of state as it stands when the epoch opens. Materialising once and serving the stored value keeps a published value stable, matching the probe aggregates. Only described nodes are scored: a node without a description has no inputs to score, whether it never answered completely or is no longer bonded. Leaving a node without a value, whether undescribed, not yet cached, or in a backfilled epoch, means an untrue score is never written and so can never later be submitted.

#### Scenario: Config score is materialised at the transition and served unchanged
- **WHEN** an epoch begins
- **THEN** a config score is computed and stored for each described node whose on-chain standing is cached, and served unchanged for the rest of that epoch

#### Scenario: A node without a description is not scored
- **WHEN** a bonded node has no description as an epoch begins
- **THEN** no config score is stored for it for that epoch

#### Scenario: A node whose standing is not cached yet is deferred
- **WHEN** a described node's on-chain standing has not been cached yet, for example after starting against a discarded database
- **THEN** no config score is stored for it for that epoch, rather than one computed as if it could not transact

#### Scenario: A backfilled epoch gets no config score
- **WHEN** the orchestrator backfills an epoch it missed while down
- **THEN** that epoch gets its probe aggregates but no config scores, since config score has no history to reconstruct and today's state would be untrue of that epoch

#### Scenario: Re-materialisation does not alter a stored config score
- **WHEN** materialisation runs again for an epoch that already has config scores
- **THEN** the stored values are unchanged

### Requirement: Config score has its own decomposed shape and storage

The orchestrator SHALL store config score in its own table keyed `(mixnet_epoch, node_id)`, holding the score together with when the epoch began and the subcomponents that produced it: versions behind, terms acceptance, whether it runs the `nym-node` binary, whether it has sufficient tokens, and whether it is a feegrant grantee. It MUST NOT model config score as a probe test kind, nor force it into the score-and-count shape the probe aggregates share.

A node scores zero for several distinct reasons, and the decomposition is what lets a consumer tell a stale version from unaccepted terms from the wrong binary. That is why config score keeps its own shape rather than being collapsed to a bare number and a sample count.

#### Scenario: The decomposition distinguishes reasons for a zero
- **WHEN** a node scores zero because it has not accepted the terms
- **THEN** the stored row records the zero score and that terms were not accepted, distinguishable from a zero caused by a stale version

#### Scenario: Config score is not a probe test kind
- **WHEN** config score is stored for an epoch
- **THEN** it is not recorded as a stress or liveness aggregate, and the probe aggregate table's kind set is unchanged

### Requirement: Config scores are evicted with the aggregates

The orchestrator SHALL delete a config score once its epoch began longer ago than the aggregate retention, in the same sweep and under the same setting that evicts the aggregates.

Config scores accrue one row per described node per epoch, so without eviction the table grows without bound. They are per-epoch figures served beside the aggregates and consumed with them, so they share the aggregates' retention rather than carrying a setting of their own.

#### Scenario: An old config score is evicted
- **WHEN** a config score's epoch began longer ago than the aggregate retention
- **THEN** the eviction sweep deletes it, while config scores of later epochs are kept

### Requirement: Config score is served as an optional field on the epoch aggregates

The orchestrator SHALL serve config score as an optional field of the per-node epoch aggregates, beside the optional liveness and stress entries, and absent whenever no config score is stored for that node and epoch. A node with a config score but no probe aggregate for the epoch, or with a probe aggregate but no config score, MUST still be represented in the epoch's response.

An absent config score is not a zero: a node that was not described, whose standing was not cached yet, or whose epoch was backfilled has no value, and serving it as zero would put an untrue figure in front of a consumer. The same rule already keeps an unmeasured probe kind absent rather than zero.

#### Scenario: A scored but unprobed node still appears
- **WHEN** a node was config-scored for an epoch but no probe run of any kind fell in the window
- **THEN** it appears in the epoch's aggregates carrying its config score, with the probe fields absent

#### Scenario: A probed but unscored node still appears
- **WHEN** a node has probe aggregates for an epoch but no config score, as in a backfilled epoch
- **THEN** it appears in the epoch's aggregates carrying its probe aggregates, with the config score absent

#### Scenario: A node with no figures is absent from the epoch
- **WHEN** a node has neither a config score nor a probe aggregate for an epoch
- **THEN** it does not appear in that epoch's aggregates, and reading that node for that epoch returns a record with every entry absent

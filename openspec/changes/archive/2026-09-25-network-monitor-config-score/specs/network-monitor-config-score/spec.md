## ADDED Requirements

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

The config score SHALL be `penalty ^ (versions_behind ^ penalty_scaling)`, where `versions_behind` is the weighted distance of the node's reported version from the on-chain version history. The score MUST be zero when the node does not run the `nym-node` binary, when the operator has not accepted the terms and conditions, when no self-description is available, or when the reported version does not parse as semver. When the node cannot transact on chain the score MUST be multiplied by `(1 - chain_interactions_penalty)`.

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
- **WHEN** a node has no self-description, or reports a version that does not parse as semver
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

The orchestrator SHALL cache each node's RAW on-chain balance and feegrant status, kept warm by a background sweep on its own interval independent of the describe refresh and with bounded concurrency, so that materialisation reads the cache rather than querying the chain. It MUST derive token sufficiency by comparing the cached balance against the minimum in force at score time, rather than caching a sufficiency decision. It SHOULD spread refresh due-times across nodes so that a population cached together does not all fall due at once.

Per-node balance and feegrant are chain queries that do not scale to the whole fleet at every epoch, so they are cached behind a time-to-live and refreshed by a warm-keeping sweep rather than lazily on the epoch path, which would burst per-node queries at the transition. Caching the raw balance rather than a sufficiency bool keeps the cached fact separate from the policy, so a change to the threshold takes effect against the cached balances without re-querying. Spreading due-times avoids a synchronised refresh of the whole fleet one interval after a cold start.

#### Scenario: A raised threshold takes effect without re-querying
- **WHEN** the minimum balance is raised so that a node's already-cached balance now falls below it
- **THEN** the next epoch's config score reflects the node as unable to transact, using the cached balance and without a new balance query

#### Scenario: Capabilities refresh on their own schedule
- **WHEN** the capability refresh interval elapses for a node
- **THEN** its balance and feegrant status are re-queried independently of the describe sweep

#### Scenario: Refreshes are spread rather than synchronised
- **WHEN** a population of nodes is cached together in one sweep
- **THEN** their next refreshes fall due spread across a window rather than all at the same instant one interval later

#### Scenario: An empty capability cache degrades rather than blocks
- **WHEN** the orchestrator starts against a discarded database with no cached capabilities
- **THEN** config scores are computed as if nodes cannot transact, recovering as the refresh sweep fills the cache, rather than blocking materialisation

### Requirement: Config-score inputs are sourced from self-description and the mixnet contract

The orchestrator SHALL read the reported version, binary name, terms acceptance and on-chain address from each node's self-description on its existing refresh sweep, and MUST obtain the version history and formula params from the mixnet contract rather than a pinned local copy.

Sourcing the version history and formula params from the contract means the orchestrator's score tracks governance, so it does not drift from nym-api when those params change.

#### Scenario: Describe inputs are captured on refresh
- **WHEN** the orchestrator refreshes a bonded node
- **THEN** it captures that node's reported version, binary name, terms acceptance and on-chain address from the describe response

#### Scenario: Formula params track the contract
- **WHEN** the mixnet contract's config-score params or version history change
- **THEN** the orchestrator scores against the new values on its next materialisation

### Requirement: Config score is materialised once per epoch as a snapshot

The orchestrator SHALL compute and persist each bonded node's config score at the epoch transition, from the node state and cached on-chain standing current at that moment, and MUST serve the persisted value thereafter rather than recomputing it per request. Re-running materialisation for an epoch that already has values MUST NOT alter them.

Config score has no sample window to replay, so its value is a snapshot of state as it stands when the epoch opens. Materialising once and serving the stored value keeps a published value stable, matching the probe aggregates.

#### Scenario: Config score is materialised at the transition and served unchanged
- **WHEN** an epoch begins
- **THEN** a config score is computed and stored for each bonded node and served unchanged for the rest of that epoch

#### Scenario: A backfilled epoch is computed from current state
- **WHEN** the orchestrator backfills an epoch it missed while down
- **THEN** that epoch's config score is computed from current node state, since config score has no history to reconstruct, and the probe aggregates for that epoch are unaffected

#### Scenario: Re-materialisation does not alter a stored config score
- **WHEN** materialisation runs again for an epoch that already has config scores
- **THEN** the stored values are unchanged

### Requirement: Config score has its own decomposed shape and storage

The orchestrator SHALL store config score in its own table keyed `(mixnet_epoch, node_id)`, holding the score together with the subcomponents that produced it: versions behind, terms acceptance, whether it runs the `nym-node` binary, whether a self-description was available, whether it has sufficient tokens, and whether it is a feegrant grantee. It MUST NOT model config score as a probe test kind, nor force it into the score-and-count shape the probe aggregates share.

A node scores zero for several distinct reasons, and the decomposition is what lets a consumer tell a stale version from unaccepted terms from a missing describe. That is why config score keeps its own shape rather than being collapsed to a bare number and a sample count.

#### Scenario: The decomposition distinguishes reasons for a zero
- **WHEN** a node scores zero because it has not accepted the terms
- **THEN** the stored row records the zero score and that terms were not accepted, distinguishable from a zero caused by a stale version

#### Scenario: Config score is not a probe test kind
- **WHEN** config score is stored for an epoch
- **THEN** it is not recorded as a stress or liveness aggregate, and the probe aggregate table's kind set is unchanged

### Requirement: Config score is served as a non-optional field on the epoch aggregates

The orchestrator SHALL serve config score as a non-optional field of the per-node epoch aggregates, present for every node in an epoch's aggregates, while liveness and stress remain optional. A node with a config score but no probe aggregate for the epoch MUST still be represented in the response.

Every bonded node has a config score, because "no self-description" is itself a valid score of zero rather than an absence. Making the field non-optional states that invariant, and it lets config score be the record that probe aggregates attach to, so a node that was scored but not probed still appears.

#### Scenario: Every node carries a config score
- **WHEN** a consumer reads an epoch's aggregates
- **THEN** each node record carries a config score, including a node with no liveness or stress aggregate

#### Scenario: A scored but unprobed node still appears
- **WHEN** a node was config-scored for an epoch but no probe run of any kind fell in the window
- **THEN** it appears in the epoch's aggregates carrying its config score, with the probe fields absent

#### Scenario: An unavailable score is present rather than omitted
- **WHEN** a node has no self-description
- **THEN** it is present in the aggregates with a config score of zero flagged as unavailable, rather than being left out

## Why

Config score is the one performance input still computed only inside nym-api. The `network-monitor-epoch-aggregates` change moved liveness and stress into the orchestrator as per `(node, epoch)` values destined for the `nym-performance-contract`, and deliberately left config score behind as its Open Question 2. This change closes that gap so all three inputs originate in one place and can move to the contract together. As with the parent change it stops short of submission: it produces the value and serves it on the orchestrator's own HTTP API so it can be validated against nym-api's before anything depends on it.

Unlike liveness and stress, config score is not measured by a probe. It is a deterministic function of a node's self-reported configuration and its on-chain standing: how far behind the reported version is, whether the operator accepted the terms and conditions, whether the binary is `nym-node`, and whether the node can transact on chain. The orchestrator sees none of these today, so the work is mostly about sourcing that data, not extending the aggregation path.

## What Changes

- **New:** a shared `nym-config-score` crate holding the pure scoring logic - a `ConfigScoreCalculator` that turns a node's version distance, gate booleans and balance into a score, plus the token-sufficiency predicate. It reuses the version-distance math already in `nym-mixnet-contract-common` and owns only the float scoring the contract crate keeps out for wasm determinism.
- **New:** nym-api's `calculate_config_score` becomes a thin adapter over the shared crate rather than its own implementation, so the orchestrator and nym-api compute an identical score. This is what makes the parallel-run comparison meaningful rather than an approximation.
- **New:** the orchestrator captures the describe-derived inputs it ignores today - reported version, binary name, terms acceptance and the node's on-chain address - on its existing per-node refresh sweep.
- **New:** a per-node chain-capability cache holding the RAW on-chain balance and the feegrant flag, refreshed on its own TTL with bounded concurrency. Storing the raw balance rather than a sufficiency bool lets the token threshold change without re-querying every node, applying the threshold at score time instead.
- **New:** the orchestrator queries the mixnet contract for the version history and formula params that the score depends on, so its output tracks governance rather than a pinned local copy.
- **New:** config score is materialised once per epoch, reusing the epoch-transition materialiser and backfill path, and stored in its OWN table keyed `(mixnet_epoch, node_id)`, decomposed into its subcomponents rather than forced into the score-and-count shape the probe kinds share.
- **New:** config score is served as a third, NON-optional field on `NodeEpochAggregates`. Every bonded node has one, since "no self-description" is itself a valid score of zero, so config score drives the per-node record set that liveness and stress attach to optionally.
- **NOT in scope**, each its own later change: submitting config score to the performance contract, the weighting that folds config score into a single performance figure, and removing config score from nym-api. Both systems compute it in parallel until the comparison says otherwise.

## Capabilities

### New Capabilities

- `network-monitor-config-score`: how the orchestrator computes a per `(node, epoch)` config score matching nym-api's, where it sources the version, terms, binary and chain-standing inputs, how it caches the on-chain data, and how it materialises and serves the value alongside the probe aggregates.

### Modified Capabilities

None. `network-monitor-epoch-performance` already anticipated this sibling: its serving requirement states config score "is the next candidate to move out of nym-api" and that the response admits a further field of its own shape. Adding that field is additive and fulfils the anticipation rather than changing the requirement, so all config-score behaviour lands as ADDED requirements under the new capability and nothing modifies the parent.

## Impact

**Affected code:**

- new crate `common/nym-config-score`, added to the workspace and its shared dependency table
- `nym-api`: `calculate_config_score` and `has_sufficient_tokens` refactored to call the shared crate, its `ConfigScoreV2` becoming a wrapper over the shared outcome
- orchestrator `node_refresher`: describe extraction extended to carry version, binary name, terms acceptance and on-chain address, with matching columns on the `nym_node` record
- orchestrator storage: a new `node_chain_capability` cache table, a new `mixnet_epoch_config_score` table and their queries
- a new chain-capability refresh task, and a per-epoch config-score step added to the existing materialiser
- new mixnet-contract queries for the config-score params and version history
- `nym-network-monitor-orchestrator-requests`: a `ConfigScore` type and a non-optional field on `NodeEpochAggregates`
- `http/api/v1/aggregates`: config score threaded into the aggregates responses, bypassing the score-shaped fold

**Dependencies.** The orchestrator gains per-node `get_balance` and `allowances` queries (bounded and cached behind a TTL) and the config-score params/version-history contract queries, extending the chain dependency the parent change already introduced.

**Sequencing.** This builds on `network-monitor-epoch-performance` - the materialiser, the `NodeEpochAggregates` response and the `/v1/aggregates` endpoint - which is archived and live, so there is no ordering constraint against another unarchived change.

**Downstream.** Nothing consumes the value yet. It is intended to be compared against nym-api's `ConfigScoreV2` for the same nodes while both run, which is the entire reason the scoring logic is shared rather than reimplemented.

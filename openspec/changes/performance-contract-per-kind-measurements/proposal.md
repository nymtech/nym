## Why

The performance contract stores one aggregated value per node per epoch: every authorised network monitor submits a single `Percent`, the contract keeps them sorted and serves the median. That was enough for the first iteration, but what rewarding actually needs is a breakdown - a liveness score, a stress score, a config score - so that a consumer can compute the final performance figure itself, users can see why a node scored what it did, and the mixnet contract can eventually resolve performance directly instead of trusting a nym-api to have combined it correctly. The contract is not deployed anywhere, so the storage layout and message surface can be redesigned without a migration, and doing it now is far cheaper than doing it after the first deployment.

An earlier attempt (PR #6282) added per-kind measurements as an admin-defined string registry keyed `(epoch, node, kind)`. It left the replay-protection cursor unable to express two kinds for one node, repeated the kind string in every storage key, iterated the registry with a point-load per kind, and held no weights. This change supersedes it with a design settled around three requirements that surfaced while reviewing it: the on-chain record must be auditable ("what values were used for rewarding epoch X for node Y" must be answerable at any later time), a missing epoch must fall back cheaply to the last epoch with data, and storage must stay proportional to what is genuinely kept.

## What Changes

- **BREAKING** `MeasurementKind` becomes a closed enum (`Liveness`, `Stress`, `Config`) with a structural split: delivery kinds carry weights, `Config` is the multiplier. No runtime define/retire registry; the enum is the set and the weights map decides what scores.
- **BREAKING** A network monitor submits one `NodeSubmission { node_id, measurements: BTreeMap<MeasurementKind, Percent> }` per node, any subset of kinds. Storage holds one bundle per `(epoch, node)`, each kind keeping every monitor's value sorted for median aggregation. The per-monitor replay cursor is unchanged, because a node is still submitted exactly once per epoch.
- **BREAKING** Submissions are accepted only for the current mixnet epoch. Once the epoch advances a bundle can never change, which is what makes the record reproducible after the fact. The orchestrator's backfill of missed epochs therefore never reaches the chain.
- Weights live on-chain in a sparse map keyed by the epoch they take effect from, written by the admin and applied from the next epoch, with the initial set supplied at instantiation.
- A per-node last-known-epoch pointer, written once when a node's bundle for an epoch is first created.
- A `RewardingInputs { epoch_id, node_id }` query resolving the bundle for that epoch or, failing that, the latest bundle at or before it within a fixed lookback, then combining it with the weights in force at the requested epoch exactly as nym-api's scoring does: a renormalised weighted mean of the delivery kinds that applied, multiplied by the config score, and no score at all when none applied or config is missing. It is deterministic over frozen data, so it is both the rewarding query and the audit query. A score-only projection of the same resolution, `RewardingScore { epoch_id, node_id }`, gives a consuming contract just the value.
- Every medians view (per epoch, per node history, full history) returns the unweighted per-kind medians and, beside them as a separate field, the combined score under the weights in force at that epoch; weights apply across kinds, never across monitors. Node history is bounded above by the pointer and omits gaps; a `LastKnownEpoch { node_id }` query exposes the pointer directly.
- The bundle map's storage namespace is shortened, since it is the only namespace paid once per `(epoch, node)`.
- Stored values are integer percents (`u8`, serialised as JSON integers) rather than `Percent` decimal strings. The existing two-decimal rounding already limits them to 101 values, so nothing is lost and a bundle shrinks by about a quarter; `Percent` remains the type at every boundary.
- The validator-client query and signing traits gain a method per new message variant, which their exhaustiveness tests require.

## Capabilities

### New Capabilities

- `performance-contract`: the on-chain per-epoch performance store. The closed kind set, the per-node submission and bundle layout, the current-epoch-only write rule, the replay cursor, the last-known-epoch pointer, the weights map and its validation, the deterministic fallback resolution and score formula, the full query surface, the admin removal escape hatches, and network-monitor authorisation.

### Modified Capabilities

None. `network-monitors-contract` is the separate three-tier stress-fleet authorisation registry and is untouched. No existing spec describes the performance contract.

## Impact

**Code**: `common/cosmwasm-smart-contracts/nym-performance-contract` (types, messages, errors, constants), `contracts/performance` (storage, transactions, queries, tests, schema), `common/client-libs/validator-client` (`performance_query_client.rs`, `performance_signing_client.rs`), `tools/internal/localnet-orchestrator` (instantiates the contract, so it must supply initial weights), and a minimal edit to `nym-api/src/node_performance` so the root workspace compiles against the new per-epoch page shape. That nym-api edit is compile-preserving only; whether nym-api should adopt `RewardingInputs` for fallback parity is the follow-up's question.

**Supersedes** PR #6282 (`dyn/perf-contract`), which is closed in favour of this change rather than rebased.

**Merge order and follow-ups**: this change is implemented fresh on `develop` and is expected to merge before the network-monitor-v3 work that will populate the contract, which is then rebased on top of it. Two follow-ups are deliberately out of scope, each its own change: the nym-api consumer for the first iteration (reading `RewardingInputs` and relaying the score into rewarding, and its interplay with `use_performance_contract_data`), and the network-monitor-v3 submission path (submitting the current epoch's aggregates, nothing else). The second iteration, in which the mixnet contract queries this contract directly, calls `RewardingScore`, the score-only projection of the same resolution, while `RewardingInputs` returns the complete inputs for external users auditing the number.

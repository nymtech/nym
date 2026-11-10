## Why

Node performance is currently computed inside nym-api, which reads raw monitor results out of its own storage and folds them into a score at reward time. The direction of travel is to move that data into the `nym-performance-contract`, where a per-node, per-epoch, per-kind value becomes readable by rewarding, explorers and anything else, and nym-api stops being the place measurements live. This change is the first step of that move and deliberately stops short of it: the orchestrator learns to produce the per-epoch aggregates and serve them on its own HTTP API, so their numbers can be validated against nym-api's before anything depends on them.

The timing constraint is what shapes the design. A value for epoch E has to exist BEFORE E ends, because rewarding reads it the moment E closes and cannot wait for a submission to land. nym-api today anchors its trailing window at the epoch's END (`map_epoch_id_to_end_timestamp`), so the value for E is only fully determined once E is already over. That is fine for a consumer reading its own database after the fact and unworkable for one reading a contract.

## What Changes

- **New:** the orchestrator computes a per `(node, epoch, test kind)` aggregate from the test runs it already stores, as the mean of the per-run scores over a trailing window anchored at the epoch's START.
- **New:** aggregates are MATERIALISED once, at each epoch transition, rather than recomputed per request. This makes a published value stable against results that arrive late but fall inside an already-anchored window.
- **New:** an HTTP endpoint on the orchestrator serving those aggregates per kind, each carrying the evidence behind it (how many runs contributed, and whether thin evidence reflects the node or the monitor) so a consumer can judge a value rather than just read it.
- **New:** the orchestrator tracks mixnet epochs, which it has no notion of today, by querying the mixnet contract.
- **Modified:** test-result eviction gains a lower bound. Retention shorter than the longest aggregation window would silently compute aggregates over truncated data.
- **NOT in scope**, and each is its own later change: submitting aggregates to the performance contract, the weighting that turns per-kind values into one performance figure, and moving config score out of nym-api.

## Capabilities

### New Capabilities

- `network-monitor-epoch-performance`: how the orchestrator turns stored test runs into per-node, per-epoch, per-kind aggregates, when it computes them, what evidence it records alongside them, and how it serves them.

### Modified Capabilities

None. Eviction's own behaviour is unchanged - it still deletes completed testruns older than `testrun_eviction_age`. What this change adds is a startup validation relating that existing knob to the new aggregation windows, which is a new concern rather than a change to an existing one, so it lives as an ADDED requirement under the new capability.

This is deliberate. The `network-monitor-liveness-tests` change is unarchived and already carries a MODIFIED delta for the eviction requirement and for the orchestrator's configuration defaults. A second unarchived change modifying the same requirements against the same baseline would conflict at archive time, so this change avoids touching them.

## Impact

**Affected code**, all within `nym-network-monitor-v3/nym-network-monitor-orchestrator`:

- storage: a new materialised aggregate table and its queries, alongside a new read over `testrun` / `testrun_measurement` windowed by timestamp
- a new epoch-tracking component backed by the mixnet contract, using the nyxd client the orchestrator already holds for agent authorisation
- a new scheduled task firing at epoch transitions, and backfill on startup for epochs missed while down
- `stale_results_eviction`: retention bounded below by the aggregation window
- `http/api`: the new per-kind aggregate endpoint
- `orchestrator-requests`: the response types

**Dependencies.** The orchestrator gains a hard runtime dependency on reaching the mixnet contract, which it previously needed only when authorising agents. Losing chain access now means epochs cannot be resolved and no aggregate can be materialised.

**Sequencing.** This change assumes the per-kind test model introduced by `network-monitor-liveness-tests`: results keyed by `(node, kind, role)`, per-kind cadences, and the per-interface measurement rows an aggregate is computed from. That change should land first. This one adds no delta against `nym-network-monitor` precisely so the two can be archived in either order without conflicting.

**Downstream.** Nothing consumes the endpoint yet. The values are intended to be compared against nym-api's existing scores while both run, which is why the aggregation method deliberately matches nym-api's `AVG(result)` rather than pooling raw packet counts.

**Operational.** Materialised aggregates are a rebuildable cache rather than durable state, preserving the property that the orchestrator's database can be discarded. The cost of discarding it is that aggregates are computed over a partial window until it refills, which for the stress kind is up to a day.

# network-monitor-epoch-performance Specification

## Purpose
TBD - created by archiving change network-monitor-epoch-aggregates. Update Purpose after archive.
## Requirements
### Requirement: An epoch aggregate is the mean of per-run scores over a window anchored at the epoch's start

For a given `(node, epoch, test kind)` the orchestrator SHALL compute the aggregate as the arithmetic mean of the scores of the completed test runs of that kind for that node whose test timestamp falls in the half-open window `[epoch_start - window, epoch_start)`. It MUST average the per-run scores rather than pooling the underlying packet counts.

Anchoring at the epoch's START rather than its end is what makes the value exist before the epoch closes, which is required because a consumer reads it the moment the epoch ends and cannot wait for it to be produced. The consequence is that the value describes the window PRECEDING the epoch it is filed under, so a node that breaks at the start of an epoch still carries its previous standing through that epoch.

Averaging per-run scores rather than pooling counts matches what nym-api does today (`AVG(result)` over its result rows), which is what makes the two figures comparable while both systems run. The orchestrator does hold the raw per-interface counts and could pool them, but a pooled figure would differ from nym-api's for the same underlying runs and make any discrepancy impossible to attribute.

A run's test timestamp is the moment its result was stored. Timestamps are compared as stored text, which keeps sub-second precision while epoch boundaries are whole seconds, so a run stored within the first second after a bound may be counted in the window ending at that bound rather than the one starting there.

#### Scenario: The window precedes the epoch it is filed under
- **WHEN** an aggregate is computed for epoch E with a window of 6 hours
- **THEN** it covers runs timestamped in `[E_start - 6h, E_start)` and no run timestamped after `E_start` contributes to it

#### Scenario: Each run contributes equally regardless of its packet count
- **WHEN** a node has one run scoring 1.0 over 50 packets and one scoring 0.0 over 3 packets
- **THEN** the aggregate is 0.5, not the pooled delivery ratio of the 53 packets

#### Scenario: A node with no runs in the window produces no aggregate
- **WHEN** no completed run of that kind falls in the window for that node
- **THEN** no aggregate row is written for that `(node, epoch, kind)` and the absence is distinguishable from a score of 0.0

### Requirement: Aggregation windows are configured per kind

The two liveness kinds SHALL share one aggregation window, defaulting to 6 hours, and the stress kind SHALL carry its own, defaulting to 24 hours. The orchestrator MUST apply each kind's window when computing that kind's aggregate.

The defaults differ because the cadences do, by an order of magnitude. At a 15 minute liveness interval a 6 hour window holds roughly 24 runs, which is ample evidence while keeping the figure responsive to a node that has just broken. At a 2 hour stress interval the same window would hold three, so stress needs 24 hours to reach a comparable dozen. The liveness kinds share their window as they share the rest of their scheduling, which is also what keeps their combined figure exact.

No startup validation ties a window to its kind's cadence, deliberately. Such a check could only assert what the configuration is CAPABLE of producing, and cadence is a target rather than a guarantee: a node can be held by another kind's per-node lock, or simply not reached by the sweep, so no relationship between window and interval guarantees that any particular number of runs actually arrived. The sample count recorded with each aggregate is the only ground truth, and how few samples is too few is a judgement for whichever consumer acts on the value rather than one the orchestrator should make on its behalf.

#### Scenario: Each kind uses its own window
- **WHEN** aggregates are materialised for an epoch
- **THEN** the liveness aggregates cover the liveness window and the stress aggregate covers the stress window, independently of each other

#### Scenario: A thin aggregate is published with its count rather than withheld
- **WHEN** only one run of a kind falls in that kind's window for a node
- **THEN** the aggregate is materialised from that single run and its evidence records one sample, leaving the consumer to decide whether that is enough

### Requirement: Aggregates are materialised once per epoch transition and never recomputed

The orchestrator SHALL compute and persist every node's aggregates at the moment an epoch begins, and MUST serve the persisted value thereafter rather than recomputing it per request. Re-running materialisation for an epoch that already has values MUST NOT duplicate or alter them.

Recomputation on read would make a published value unstable: eviction eventually removes the runs behind it, and a result stored around a boundary can land after the value was first read, so the same query answered at two moments could give two answers and a value already handed to a consumer could change underneath it.

#### Scenario: A late-arriving result does not move a published value
- **WHEN** a run whose test timestamp falls inside epoch E's window is stored after E's aggregates were materialised
- **THEN** E's materialised aggregate is unchanged, and the run contributes only to later epochs whose windows still contain it

#### Scenario: Materialisation is idempotent
- **WHEN** materialisation runs twice for the same epoch
- **THEN** the stored aggregates are identical to those written by the first run

### Requirement: Epochs missed while the orchestrator was down are backfilled where the data still supports it

On startup the orchestrator SHALL materialise any epoch that has begun since its last materialised epoch, provided that epoch's window is still covered by retained test runs. An epoch whose window has aged out of retention MUST be left without aggregates rather than computed from truncated data.

An orchestrator that has never materialised anything MUST take the same path, materialising the epoch already in progress, whose start is known and already past. There is deliberately no separate first-deployment behaviour, so the recovery path is exercised on every deploy rather than only during an incident.

A backfilled value may differ from the one that would have been written at the time, because results have arrived since. That is accepted: a value computed from more complete evidence is not worse, and the alternative is a permanent hole.

#### Scenario: A missed epoch is backfilled
- **WHEN** the orchestrator restarts having missed one epoch transition, and retained runs still cover that epoch's window
- **THEN** it materialises that epoch before serving, and the value reflects every run now present in the window

#### Scenario: An epoch beyond retention is left empty
- **WHEN** the orchestrator restarts after an outage longer than the retention period
- **THEN** epochs whose windows are no longer covered are skipped and no aggregate is invented for them

#### Scenario: A first deployment materialises the epoch already in progress
- **WHEN** an orchestrator starts having never materialised an epoch
- **THEN** it materialises the epoch currently in progress by the same backfill path, rather than waiting for the next transition

### Requirement: Every aggregate carries the evidence behind it

Each materialised aggregate SHALL record how many runs contributed to it, and no aggregate MAY exist for a node and kind that returned no run at all, so that an absent value can never be read as a measured one.

The sample count is the only ground truth about how much evidence stands behind a value, since no relationship between a window and a kind's cadence can guarantee that any particular number of runs actually arrived.

A node that was never assigned work and one whose assignments never came back both have no runs, and so both read as absent. The orchestrator does not record which of the two it was per node: either way nothing is published for that node and kind, and how often assignments go unanswered is visible in aggregate through the per-kind lease-expiry metrics.

#### Scenario: A measured node reports its sample count
- **WHEN** eight runs contributed to a node's aggregate
- **THEN** the stored evidence records eight samples

#### Scenario: A node whose assignments never returned has no aggregate
- **WHEN** a node was assigned work during the window but no result was ever stored for it
- **THEN** no aggregate is stored for that node and kind

#### Scenario: An unmeasured node is distinguishable from a measured zero
- **WHEN** a consumer reads an epoch's aggregates
- **THEN** a node with no value is absent from them rather than present with a score of zero

### Requirement: A run that failed counts as a sample rather than being excluded

A completed run carrying a run-level error SHALL be scored the same way as any other and MUST NOT be dropped from the aggregate.

This follows the existing rule that unmeasurable must never score better than measurably broken: a node that refuses a connection is not routable, and excluding such runs would let a node that fails every probe read the same as one that was never probed.

Its score does not need forcing to zero to achieve that. An interface that sent nothing already rates zero, so a run that failed before measuring anything scores zero of its own accord. The only runs a forced zero would touch are those that measured something before aborting - a probe that exceeded its deadline part-way through its load test, say - and for those the measured ratio is the truer statement about the node. Forcing it would also put this figure out of step with the one submitted to the nym-api for the same run, which is the comparison the whole transition rests on.

#### Scenario: An errored run pulls the aggregate down
- **WHEN** a node has three runs scoring 1.0 and one that failed without measuring anything
- **THEN** the aggregate is 0.75, not 1.0

#### Scenario: A run that aborted after measuring keeps what it measured
- **WHEN** a run exceeds its deadline having already had half its packets returned
- **THEN** it contributes 0.5 rather than zero, and the nym-api receives the same 0.5 for that run

### Requirement: One aggregate per node and kind, with both liveness kinds reported as one

The orchestrator SHALL store exactly one aggregate per `(node, epoch, kind)`, averaging across every announced address that kind exercised for that node in the window. It SHALL report the two liveness kinds as a single liveness figure: the mean over both kinds' runs, obtained by weighting each kind's aggregate by its sample count.

Runs rotate through a node's announced addresses, so several run streams exist per node and kind. Collapsing them matches the existing stored-score semantics, and a node whose ipv6 address is broken therefore carries genuine zeros into its aggregate, which is the intended reading rather than a defect.

A dual-role node is tested by both liveness kinds, one per role. Storing them apart keeps every kind's aggregate keyed like its results, while reporting them as one matches nym-api, where both kinds' results land in a single liveness series. The combination is exact because each aggregate carries its sample count and both liveness kinds share one aggregation window.

#### Scenario: A dual-role node yields one liveness figure
- **WHEN** a node had three mixnode liveness runs scoring 1.0 and one gateway liveness run scoring 0.0 within the window
- **THEN** its reported liveness aggregate is 0.75 over four samples, the mean over all four runs rather than the mean of the two kinds' means

#### Scenario: A failing address is reflected rather than hidden
- **WHEN** a node's runs against one announced address consistently score zero while another scores well
- **THEN** the aggregate reflects both, and the shortfall is visible rather than suppressed

### Requirement: Epoch boundaries are resolved from the mixnet contract

The orchestrator SHALL determine epoch identity and timing by querying the mixnet contract, and MUST NOT derive them from local wall-clock arithmetic. When the contract cannot be reached the orchestrator MUST decline to materialise rather than materialise against a guessed boundary.

The contract is the only source that cannot drift from what rewarding will use. A locally derived boundary that disagrees with the chain would file measurements under the wrong epoch, which is unrecoverable once published.

The identifier MUST be named so that it is not confused with the sphinx key rotation id, which the orchestrator already stores under the name `epoch` for an unrelated purpose.

#### Scenario: Epoch timing comes from the chain
- **WHEN** the orchestrator needs to know when an epoch began
- **THEN** it uses interval data obtained from the mixnet contract

#### Scenario: An unreachable chain blocks materialisation rather than guessing
- **WHEN** the mixnet contract cannot be queried at an epoch transition
- **THEN** no aggregate is materialised for that epoch, and it becomes a candidate for backfill once the chain is reachable again

### Requirement: Test-run retention must exceed the longest window

The orchestrator MUST reject at startup any configuration in which `testrun_eviction_age` does not exceed the longest configured aggregation window.

Aggregates are computed from the stored test runs, so a window reaching further back than their retention would compute over silently truncated data and produce a plausible-looking number derived from part of the evidence.

The floor that is enforced is strict: retention greater than the longest window. The excess of retention over that window is the backfill budget - how far a restart can reach back to recover missed epochs - and its sizing is left to the operator, for the same reason a window is not checked against its kind's cadence: the maximum downtime cannot be known at startup, so no fixed margin could guarantee it is "enough".

#### Scenario: Test-run retention shorter than a window is rejected
- **WHEN** `testrun_eviction_age` is configured at or below the longest aggregation window
- **THEN** startup fails with an error naming both values

### Requirement: Aggregates are served per kind on the orchestrator's HTTP API

The orchestrator SHALL expose materialised aggregates over HTTP, keyed by node and epoch, with liveness and stress reported separately alongside their evidence. The response shape MUST admit further entries without a breaking change.

Liveness and stress are kept separate because the weighting that combines them into a single performance figure is deliberately not the orchestrator's concern, and because further entries are expected: config score is the next candidate to move out of nym-api, and it will sit beside liveness and stress rather than being folded into them. Each entry is a named field of its own rather than a value in a uniform map, precisely because that next entry will not share this one's score-and-count shape.

The runs an aggregate was computed from are served per node and kind by the orchestrator's results endpoints, and nym-api holds the same per-run scores, so a value can be checked against its evidence without a surface of its own.

#### Scenario: Each entry is reported with its own value and evidence
- **WHEN** a consumer requests a node's aggregates for an epoch
- **THEN** it receives a separate liveness and stress entry, each carrying its value and sample count

#### Scenario: An entry with no aggregate is absent rather than zero
- **WHEN** a node has a stress aggregate and no liveness aggregate of either kind
- **THEN** the response omits the liveness entry rather than reporting it as 0.0

### Requirement: Materialised aggregates are a rebuildable cache rather than durable state

The materialised aggregate table SHALL be treated as reconstructible. Discarding it MUST NOT prevent the orchestrator from starting, and MUST NOT require a data migration to preserve it. An aggregate SHALL be evicted once its epoch began longer ago than the aggregate retention, 30 days by default, which is independent of `testrun_eviction_age`.

This keeps the property the orchestrator's storage already relies on, that its database can be discarded and rebuilt from the chain and from re-measurement. The cost of discarding it is bounded and known: aggregates are computed over a partial window until the window refills, which is up to six hours for liveness and up to a day for stress, and during that period some nodes will have no aggregate at all.

#### Scenario: Old aggregates are evicted
- **WHEN** an aggregate's epoch began longer ago than the aggregate retention
- **THEN** the eviction sweep removes it, while aggregates of more recent epochs remain

#### Scenario: A discarded database does not block startup
- **WHEN** the orchestrator starts against an empty database
- **THEN** it begins measuring and materialises aggregates as the window fills, rather than failing or blocking

#### Scenario: Aggregates during recovery are thin rather than wrong
- **WHEN** an epoch's window is only partially covered because measurement recently restarted
- **THEN** the aggregate is computed from the runs that exist and its evidence reports the reduced sample count


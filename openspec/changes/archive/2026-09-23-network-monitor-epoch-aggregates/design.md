## Context

Network monitor v3 stores every completed test run locally and submits it to nym-api, which computes node performance at reward time by averaging the runs in a trailing window. That window is anchored at the epoch's END (`map_epoch_id_to_end_timestamp` in `legacy_storage_provider.rs`), and the score is `AVG(result)` over the rows it finds. This works because nym-api reads its own database after the epoch has closed.

The direction of travel is to move that data into the `nym-performance-contract`, where `(node, epoch, kind)` becomes a value any consumer can read, and nym-api stops being where measurements live. The contract's interface takes only a node id, a measurement kind and a score, so nothing explanatory survives the trip.

The binding constraint is timing. A contract value for epoch E must exist BEFORE E ends, because rewarding reads it the moment E closes and cannot wait for a submission. An end-anchored window cannot satisfy that: it is only fully determined once the epoch is already over.

The relevant existing constraints are:

- The orchestrator has NO notion of mixnet epochs. Its only `epoch` is `key_rotation_id` for sphinx keys, which is unrelated. It does hold a nyxd client, used today only to authorise agents on-chain.
- Its database is treated as disposable. The per-kind storage migration reshaped the work tables empty rather than backfilling, on the stated grounds that the registry rebuilds from the contract, leases are orphaned by any restart, and results are a retry buffer already submitted to nym-api.
- Test cadences differ by an order of magnitude: `test_interval` is 2 hours for stress, `liveness_test_interval` is 15 minutes.
- Results are evicted after `testrun_eviction_age`, default 7 days.
- A probe that fails critically is deliberately never submitted, so the lease expires and the node keeps its turn. The orchestrator therefore knows which assignments were made and never returned.

## Goals / Non-Goals

**Goals:**

- Produce a per `(node, epoch, kind)` value that exists before its epoch ends.
- Produce it by a method whose output can be checked against nym-api's existing score for the same runs, so the two can be compared before anything depends on the new one.
- Preserve, at the orchestrator's own boundary, the distinction between a node that measured badly and a node that was not measured, which the contract's interface cannot carry.
- Leave the orchestrator's database disposable.

**Non-Goals:**

- Submitting anything to the performance contract. That is the next change, and it owns the question of how few samples are too few to publish.
- Weighting per-kind values into a single performance figure.
- Moving config score out of nym-api, though the response shape anticipates it.
- Changing what nym-api does today. Both run in parallel until the comparison says otherwise.
- Changing how test runs are scheduled, executed or stored.

## Decisions

### Decision 1: The aggregation window is anchored at the epoch's START

**Choice.** The aggregate for epoch E covers runs timestamped in `[E_start - window, E_start)`.

**Why.** It is the only anchoring that satisfies the timing constraint. The window is fully determined the instant E opens, so by the time E closes the value has been available for a whole epoch.

**Alternative considered.** Anchoring at `E_end`, matching nym-api. Rejected because it reproduces exactly the problem this change exists to solve. Also considered: anchoring at `E_start - grace` to let the window settle before it is read. Rejected as solving with a fudge factor what Decision 4 solves properly, and it would have made the value's meaning depend on an arbitrary constant.

**Consequence.** The value filed under epoch E describes the window PRECEDING E. A node that breaks at E's start carries its previous standing through E and is only marked down from E+1. With a 6 hour window that smear already dominates, so the additional epoch of lag changes little, but it is a real semantic difference from nym-api and must be stated wherever the two are compared.

### Decision 2: The aggregate is the mean of per-run scores, not a pooled delivery ratio

**Choice.** Average the per-run scores. Do not sum the underlying packet counts and divide.

**Why.** Three reasons, the last decisive. It is consistent with every layer below, since a run's own score is already an average over the kind's fixed measurement set. The metric is availability over time, so one observation should weigh the same whether it happened to send 50 packets or 37. And during the transition both systems compute values from the SAME runs: if the orchestrator pools while nym-api averages, the two disagree by construction and no discrepancy can be attributed to a bug rather than to the definition.

**Alternative considered.** Pooling the raw per-interface counts, which the orchestrator holds and nym-api does not. It is the better estimator of a true delivery probability, weighting each observation by its evidence. Rejected for the comparability reason above; it remains available later, once nothing depends on agreeing with nym-api.

**Consequence.** A run truncated by a straggler timeout counts as a full observation. Given a probe that fails critically is never submitted at all, the surviving truncated runs are few, so the practical difference between the two methods is small.

### Decision 3: Windows are configured per kind, with no validation against cadence

**Choice.** Liveness 6 hours, stress 24 hours, each configurable. No startup check relating a window to its kind's test interval.

**Why.** The defaults differ because the cadences differ by an order of magnitude: 6 hours holds roughly 24 liveness runs but only 3 stress runs. A shorter liveness window is worth having because it makes the figure responsive to a node that has just broken, and liveness has the sample density to afford it.

No validation, because a check of the form `window >= N x interval` can only assert what the configuration is CAPABLE of producing. Cadence is a target, not a guarantee: a node can be held by the other kind's per-node lock or simply not reached by the sweep. So the check would guarantee nothing while giving the impression that it did.

**Alternative considered.** A single shared window, rejected as forcing liveness to inherit stress's sluggishness. Also considered and dropped: requiring a window to span at least three test intervals. It permits precisely the configuration this decision rejects, since three stress intervals is six hours.

**Consequence.** Nothing prevents an operator configuring a 10 minute stress window. The sample count recorded with every aggregate is what makes that visible, and the judgement about sufficiency belongs to the consumer.

The defaults are also implicitly sized against an HOURLY epoch, which is what the mixnet contract runs today. A 6 hour window then covers the six epochs preceding the one it is filed under, which is a reasonable smear. Should epochs become daily, a 6 hour window would describe only the final quarter of the preceding day while governing rewards for a full one, and the windows would want to grow roughly in step. That is a configuration change rather than a code change, which is why both are knobs, but it is a coupling to notice rather than discover.

### Decision 4: Aggregates are materialised once at the epoch transition

**Choice.** Compute every node's aggregates when an epoch begins and persist them. Serve the persisted values. Never recompute.

**Why.** Results keep arriving for runs whose timestamps fall inside an already-anchored window, because submission lags measurement. Computing on read would therefore give different answers at different moments, and a value already handed to a consumer could change underneath it. For a figure destined for a contract that is unacceptable.

**Alternative considered.** Computing on read and accepting the drift, which needs no table and no scheduled work. Rejected on the stability argument. Also considered: the `E_start - grace` anchor of Decision 1, which narrows the drift without eliminating it.

**Consequence.** A new table, a task firing at epoch transitions, and an idempotency requirement so a repeated run neither duplicates nor alters. It also makes the endpoint cheap, since serving is a lookup rather than an aggregation, and gives the evidence of Decision 6 somewhere to live. A result arriving after its epoch was materialised is not lost, it simply contributes only to later epochs whose windows still contain it.

### Decision 5: Missed epochs are backfilled where retention still covers their window

**Choice.** On startup, materialise every epoch that has begun since the last materialised one, provided retained runs still cover its window. Skip those that have aged out.

**Why.** An orchestrator restart spanning an epoch transition would otherwise leave a permanent hole, and holes are not recoverable later.

**Alternative considered.** Leaving holes and relying on the consumer to tolerate them, which is simpler and is safe given absence is representable. Rejected because a hole is permanent while a backfill is cheap, and the data is usually still there.

**Consequence.** A backfilled value can differ from the one that would have been written at the time, because results have arrived since. This is accepted: more complete evidence is not worse. It is also a reason to keep sample retention comfortably above the longest window rather than exactly at it.

There is therefore NO first-deployment special case. A freshly deployed orchestrator has no last-materialised epoch, so backfill starts from an empty state and materialises the epoch already in progress, whose start is known and already past. That is the same code path as recovering from downtime across a transition, which is worth having exercised on every deploy rather than only during an incident. It is safe for the same reason recovery is: the value is produced well before the epoch ends and rewarding reads it.

Nothing special is done about nodes that have unbonded. They fall out of the registry on its next refresh and are materialised until then, which is harmless: the performance contract rejects submissions for unbonded nodes anyway, so a stale aggregate for one is wasted work rather than a wrong answer, and adding a bonded check here would duplicate a check that already exists where it matters.

### Decision 6: Aggregation reads a dedicated sample table, written at assignment with a nullable score

**Choice.** Add a narrow table holding one row per assignment: node, test kind, timestamp, and a NULLABLE score filled in when the result arrives. Aggregates are computed from it rather than from `testrun` and `testrun_measurement`, and it carries its own eviction schedule independent of `testrun_eviction_age`.

Alongside each aggregate, store how many runs contributed. Which of three cases produced it - assigned and returned, assigned and never returned, never assigned - stays derivable from this table rather than being copied onto the aggregate: an aggregate exists only for the first case, so the other two are told apart by reading the assignments, and nothing published consumes that distinction until the submission change's minimum-evidence policy does.

**Why.** The contract's interface carries only a score, so a 0.0 meaning "the node was dead" and a 0.0 meaning "we never tested it" are indistinguishable once published. Only the orchestrator can tell them apart, and only if it records assignments rather than just results.

It cannot do that today. `assign_next_testruns` writes `node_test_state.last_tested_ip` at assignment and `last_tested_at` at result, but both are single "last" values rather than a log, and eviction DELETES expired `testrun_in_progress` rows outright, so a lease that expired leaves no trace. Over a window the orchestrator can therefore count results and nothing else. Writing the row at assignment with a null score is what makes the three states derivable: a row with a score returned, a row still null past its lease was assigned and never returned, and no row means never assigned. A critically failed probe is deliberately never submitted so the lease expires and the node keeps its turn, which is exactly the case the null row captures and which is a MONITOR failure rather than a statement about the node.

Two further benefits fall out. The table's own eviction schedule decouples the aggregate's source data from general result retention, so the retention floor this change needs becomes a property of a table it owns rather than a new constraint on an existing knob. And a purpose-built narrow table makes the windowed read cheaper than joining two wider ones per node per kind.

**Alternative considered.** Publishing the value with its sample count alone, and treating zero samples as ambiguous. Simpler, and it needs no new writes, but it discards the only signal that positively identifies a monitor fault - which is the distinction that motivated this whole line of design. Also considered: suppressing thin values, rejected because a node with no published value gets no rewards, converting a monitor problem into a node penalty. Also considered: retaining expired `testrun_in_progress` rows instead of deleting them, which would make that table the log. Rejected because eviction's behaviour is the requirement the unarchived liveness change already modifies, and changing it here would collide (Decision 10).

**Consequence.** One extra write per assignment, and `testrun_in_progress` keeps its existing role as the per-node mutex, gaining only the id of the sample its assignment created. That column is what a returning result is matched back by; `(node, kind, timestamp)` cannot serve, since the result's timestamp is not the assignment's. The link points from the lease to the sample rather than the reverse because of the order the rows come into existence: the sample exists from the moment work is handed out, while the `testrun` a result produces is only inserted once that result arrives. Rows that are still null long past their lease need clearing on the table's own schedule, or they accumulate. The evidence stays at the orchestrator's boundary and does not survive submission, which is correct: the consumer that needs it is the submission policy, and that runs here.

### Decision 7: Epoch identity and timing come from the mixnet contract

**Choice.** Query the mixnet contract for interval data. Do not derive epochs from local wall-clock arithmetic against a remembered anchor.

**Why.** The contract is the only source that cannot drift from what rewarding will use. A locally derived boundary that disagrees with the chain files measurements under the wrong epoch, which is unrecoverable once published.

**Alternative considered.** Caching the interval once and extrapolating locally, which would survive a chain outage. Rejected because a silently wrong epoch is worse than a missing one, and Decision 5's backfill already covers the outage case.

**Consequence.** The orchestrator gains a hard runtime dependency on reaching the chain, where previously it needed it only to authorise agents. A transition that arrives while the chain is unreachable produces no aggregate and becomes a backfill candidate. The identifier is named `mixnet_epoch` wherever it appears, deliberately explicit, because the orchestrator's storage already uses the bare word `epoch` for the unrelated sphinx `key_rotation_id`.

### Decision 8: The materialised table is a rebuildable cache, not durable state

**Choice.** Treat the aggregate table as reconstructible. Discarding it must not block startup and must not require a migration to preserve.

**Why.** It keeps the property the orchestrator's storage already relies on and which the per-kind migration leaned on to avoid writing backfill logic. The authoritative record of anything published is the contract, not this table.

**Alternative considered.** Treating it as durable, backing it up and preserving it across migrations. Rejected as buying little: the values it holds are either already published, in which case the contract has them, or not yet published, in which case they can be recomputed while the window still covers them.

**Consequence.** Losing the database costs a period of degraded aggregates rather than corrupted ones, bounded by the longest window: up to 6 hours for liveness and a day for stress. During that period some nodes have no aggregate at all, because after a wipe every node reads as never-tested and the sweep reaches them in staleness order rather than all at once. That is precisely the situation a last-known-good fallback in the contract exists to absorb, which makes such a fallback a requirement of the submission change rather than an optional nicety.

### Decision 9: One aggregate per node and kind, collapsing role and tested address

**Choice.** Average across every role and every announced address that kind exercised for the node in the window.

**Why.** It matches the existing stored-score semantics, in which a dual-role node's score already averages its mixnode and gateway runs and its declared role labels the node rather than the measurement. The contract is keyed per node, so the collapse has to happen somewhere regardless.

**Alternative considered.** Retaining per-role values. Rejected because nothing downstream can consume them and the contract could not carry them.

**Consequence.** A node whose ipv6 address is broken carries genuine zeros into its aggregate. That is the intended reading, not a defect, but it means a half-scoring node needs the orchestrator's own per-run surface to diagnose.

### Decision 10: This change adds no delta against the `nym-network-monitor` capability

**Choice.** Everything lands as ADDED requirements under a new `network-monitor-epoch-performance` capability, including the retention floor.

**Why.** The `network-monitor-liveness-tests` change is unarchived and already carries MODIFIED deltas for both the eviction requirement and the orchestrator's configuration defaults. Two unarchived changes modifying the same requirements against the same baseline conflict at archive time. Eviction's own behaviour is genuinely unchanged here, so framing the retention floor as a new concern is accurate as well as convenient.

**Consequence.** The two changes can be archived in either order.

## Risks / Trade-offs

- **The value filed under an epoch describes the window before it**, so a node that breaks at an epoch's start is still paid for that epoch. → Inherent to the timing constraint; the smear from a multi-hour window already dominates the single-epoch lag, and the semantics are stated wherever the figure is compared with nym-api's.
- **Losing the orchestrator database degrades aggregates for up to a day.** → Bounded and non-corrupting; absorbed by a last-known-good fallback in the contract, which the submission change must provide.
- **A hard dependency on the chain at every epoch transition.** → A missed transition is a backfill candidate rather than a permanent hole, and a wrong epoch is worse than a late one.
- **Results arriving after their epoch was materialised never count towards it.** → Accepted as the price of a stable value; they contribute to later epochs whose windows still contain them.
- **An operator can configure a window too short to be meaningful**, since no validation prevents it. → The sample count recorded with every aggregate makes it visible rather than silent.
- **The evidence does not survive submission**, because the contract carries only a score. → The decision that consumes the evidence is the submission policy, which runs in the orchestrator where the evidence is.

## Migration Plan

1. Land the change. It is additive and inert: a new table, a new scheduled task, a new endpoint, and nothing consuming any of them. Existing scheduling, execution, storage and nym-api submission are untouched.
2. Deploy and let the windows fill. The first epoch materialised after deployment computes over a partial window, and reaches full coverage after 6 hours for liveness and 24 for stress.
3. Compare the endpoint's values against nym-api's scores for the same nodes. The two should agree closely, because both average the same per-run scores; the expected divergence is the one Decision 1 introduces, since nym-api anchors at the epoch's end and this anchors at its start.
4. Once the comparison holds, the submission change follows, carrying the contract work: per-kind submission streams, the minimum-evidence policy, and the last-known-good fallback.

Rollback is deleting rows and not calling the endpoint. Nothing else reads the table, and removing the scheduled task leaves the rest of the orchestrator unaffected.

## Open Questions

1. **What minimum evidence justifies submitting a value?** Deliberately deferred to the submission change, since it depends on how the contract treats a missing entry. A floor of three samples was discussed as the likely answer.
2. **Whether config score moves next**, which would make it a third sibling alongside liveness and stress and would require the orchestrator to learn node versions, terms acceptance and chain-interaction capability, none of which it can see today.

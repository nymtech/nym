## Context

The orchestrator hands out work only when an agent asks for it. A node becomes eligible for a kind when it has a description, has that kind's role, holds no in-flight lock (the lock is per node and shared across kinds), and its last run under that kind is older than the kind's staleness age. That single predicate, in `select_candidates`, is the whole of the scheduling model.

Because nothing pushes, throughput is demand-limited: the fleet does as much work as there is work to do, and no more. Achieved throughput therefore says nothing about available capacity, which is precisely the question a cadence change asks. The agent is also a one-shot process today, announcing, taking one assignment and exiting, so "number of agents" really means the rate at which an external scheduler invokes them, and the orchestrator only ever sees that as an arrival rate of work requests.

Of the current 45 series, the ones that look relevant are either global where they need to be per kind, refreshed far too slowly, or structurally wrong. `test_duration_seconds` mixes all three kinds in one histogram. The per-kind in-flight gauges are recounted only by the eviction sweep, roughly every 150 seconds, against a 60 second liveness lease, so a whole wave can open and close between samples. The `bonded_*` gauges count self-described roles among bonded nodes rather than testable nodes, and refresh every 2 hours. The agent gauges count addresses ever seen.

## Goals / Non-Goals

**Goals:**

- Answer, from a scrape, whether a kind has spare capacity and how much, in units that convert directly into a cadence.
- Answer how much the agent fleet would have to grow or could shrink, in a way that still works when the fleet is saturated.
- Make "tests are running behind" an alertable signal rather than something noticed by looking.
- Leave the series count lower than it started while doing all of the above.

**Non-Goals:**

- Per-agent attribution of any kind.
- Measuring agent-side time, concurrency or sleep. The orchestrator sees arrivals, not agents.
- Changing scheduling behaviour. This change observes; it does not alter what gets assigned to whom.

## Decisions

### Capacity is the idle gap, accumulated as a counter

Per kind, the task accumulates elapsed time into `work_pending_seconds_total` on every tick where pending or in-flight work exists. Utilisation at any window is then `rate(work_pending_seconds_total[window])`, headroom is its reciprocal, and the minimum sustainable cadence is `staleness_age_seconds * utilisation`.

*Alternatives considered.* Inferring the ceiling from the fraction of work requests that find work, computable today as `empty_testrun_assignments / agent_testrun_requests`, measures the same quantity but indirectly through the agents: it is global rather than per kind (an empty response means every kind was empty, so it cannot be attributed), and it varies with poll rates and wave sizes. A 0/1 "has pending work" gauge averaged with `avg_over_time` loses every tick between scrapes, which matters because the task ticks faster than Prometheus scrapes and matters more as the cadence shortens. A locally computed ratio gauge fixes the averaging window at emit time, and after a restart an in-memory window reads as zero, which is indistinguishable from a perfectly idle fleet with infinite headroom. The counter has none of these problems: no ticks are lost because it accumulates between scrapes, the window stays a query-time choice so an alert and a dashboard can differ, and `rate()` handles counter resets natively.

Accumulating measured elapsed time rather than a fixed tick constant keeps it correct under drift or a missed tick.

### Why the idle gap is observable at all

The scheduler has no cycle boundary; nodes become due spread across the period according to when they were last tested. But a fleet that drains quickly tests nodes together, and nodes tested together become due together one cadence later, so draining fast is self-clustering. The batched regime therefore emerges exactly when there is spare capacity, which is the case being measured. When capacity is tight the clustering smears out, the queue stops emptying, and the informative signal becomes backlog depth and head lateness instead. The measurement is sharp in the over-provisioned regime and blunt in the under-provisioned one, and the two gauges cover the second case.

### Pending and in-flight together, rather than changing the scheduler

`peek_kind_head` returns nothing both when no node is due and when every due node is locked in flight, because `select_candidates` excludes any node with an in-progress row. Those are opposite capacity readings. Emitting `testruns_pending` and `testruns_in_flight` from the same tick resolves it without touching the assignment path: both zero is genuine idle, pending zero with in-flight positive is saturation, pending positive is backlog.

*Alternative considered.* Changing `peek_kind_head` or the assignment path to distinguish the two cases directly. Rejected as needlessly invasive for an observability requirement, when two gauges read side by side already carry the distinction.

### The task owns its tick

The metrics task does not extend the eviction sweep. The sweep's cadence is derived as half the stress lease, floored at a minute, giving roughly 150 seconds at defaults, which cannot resolve a 60 second liveness lease and would degrade further as the cadence shortens. The tick should be at or below the Prometheus scrape interval so no sample is stale, which suggests a default in the region of 15 seconds.

### Required rate is published; configuration is not

The orchestrator publishes `required_assignment_rate` per kind, computed as testable population over the configured staleness interval over the configured wave cap. It does not publish the staleness interval or the wave cap themselves.

*Rationale.* Telemetry should carry what cannot be recovered from configuration and environment. A cadence or a wave cap is something the deployment already knows, and re-publishing it is the process restating its own input. An earlier draft exported both so that panels could divide them, justified on the grounds that stored history would then describe which cadence each sample belonged to. That argument does not hold up: a configuration change is a deployment, deployments are already recorded, and Grafana annotations exist for marking them. Population, by contrast, is observed and changes continuously, so the quotient is not inferable from configuration either and is the figure a consumer actually wants.

*The divisor must be the configured cap, not the observed mean.* When a kind is over-provisioned its queue drains and waves collapse to a handful of targets, so the mean from the wave-size histogram is depressed exactly when there is the most slack, which would inflate the required rate precisely when it should look comfortable. The observed mean stays useful as a corroborating diagnostic: well below cap means the due population ran dry.

*Consequence.* Asking what a different cadence would require is still answerable, because `testable_population` is published separately: the hypothetical supplies its own interval and cap, and does not need the current ones as series.

### Achieved rates are queried, not published

The rate at which assignments are currently handed out is `rate(testrun_assignments_total{kind}[window])` over the counter the collapse produces, so no series is added for it. Publishing a pre-computed rate would fix the averaging window at emit time, which is the mistake this change already rejected when it chose a pending-time counter over a utilisation ratio.

Required rate stays in assignment units rather than target units for a specific reason: that is the form that converts a shortfall into agent invocations, which is the actionable question. The superficially appealing comparison of achieved against required is not the "are we keeping up" signal it looks like, because in steady state the two necessarily coincide, whatever the cadence. Falling behind shows up as `head_lateness_seconds` climbing and `testruns_pending` not returning to zero, which say it directly.

### Never-tested nodes are counted, not folded into lateness

`head_lateness_seconds` covers only nodes a kind has measured at least once; nodes it has never measured are reported as `never_tested`.

*Rationale.* `select_candidates` orders by `last_tested_at ASC NULLS FIRST`, so a never-tested node is always the head whenever one exists. A live network gains bonded nodes continuously, so folding never-tested into the overdue figure would leave the head permanently in a state with no due time to measure against, and the primary "are we keeping up" signal would be either constantly firing or deliberately blind to its own head. Separated, the overdue figure answers whether the cadence is slipping for nodes under measurement, and the count answers whether new nodes are being picked up at all. Those are different failures: the first means the fleet is undersized, the second can happen with plenty of spare capacity if new nodes are not becoming assignable.

*Reversal.* An earlier draft dropped `never_tested` on the grounds that such nodes already sit inside `testruns_pending`. That reasoning does not survive excluding them from head lateness: without its own series, a backlog of brand-new unmonitored nodes becomes invisible in exactly the signal meant to catch it.

### Oldest outstanding age complements the expired count

`in_flight_expired` is a failure signal: it fires once a lease has already been breached and nodes are locked by abandoned runs. The oldest outstanding dispatch age is the leading indicator of the same condition, and the two read together separate a fleet working comfortably inside its lease from one about to start breaching.

It earns its own series rather than being inferred. The upper tail of `test_duration_seconds{kind}` approximates wave duration, because a wave's targets are probed concurrently and the wave is therefore about as long as its slowest target, but that is an inference from a distribution rather than a measurement of the dispatch actually outstanding. It costs one `MIN(started_at)` per kind per tick, and it matters more as the liveness cadence drops toward 2.5 minutes against a lease that does not move.

### The denominator is testable nodes, with exclusions surfaced separately

`testable_population` counts what the scheduler can actually assign: bonded, described, role-enabled. `excluded_nodes` counts bonded nodes with no usable description.

*Alternatives considered.* Counting only testable nodes and saying nothing about the rest would let a silently growing set of unreachable nodes flatter utilisation, because work that cannot exist never appears as backlog. Counting all bonded nodes in the denominator would be dishonest in the other direction: the fleet cannot test what has no description, so the backlog would never drain and head lateness would alert forever.

### Collapse existing series now

With labels available, 31 of the 45 series are one concept repeated per variant. Renaming them is breaking for anything consuming them by name, and this is the cheapest moment it will ever be: the Grafana dashboards these metrics exist to feed do not exist yet, so nothing downstream breaks. Deferring means doing the same renames after dashboards and alerts have been written against the old names.

A request total stays its own series rather than becoming one value of an `outcome` label, so no query can sum a breakdown together with the total it breaks down.

### One bucket set for the labelled duration histogram

A histogram family carries a single bucket set across all its children, so `test_duration_seconds{kind}` cannot be tuned per kind. The buckets widen to span a liveness target bounded by a 60 second lease and a stress run bounded by a 300 second one. Separate families per kind would allow tuned buckets, at the cost of reintroducing exactly the hand-enumeration this change removes. The shared set is the right trade: the decision this metric informs is whether a wave finishes comfortably inside its lease, which needs resolution in seconds, not a finely tuned tail.

### The agent count is a redundancy signal, not a sizing one

Sizing is answered by required rate against the observed request rate, which never uses a count, and a count cannot say whether to add pods or invoke existing ones more often, since both move the same rate. What a count uniquely provides is detection of silent partial fleet loss. Because work is pulled, losing half the agents produces no observable change: the survivors each take more work, the queue absorbs it, throughput stays flat and backlog stays at zero, until the remainder saturates and the fleet is already behind. It also distinguishes many agents asking occasionally from few asking constantly, identical in request rate and very different in resilience. Both effects get sharper at a tighter cadence, where there is less slack to hide an outage.

The consequence for consumers is that the useful alert is a drop against the series' own recent baseline rather than a comparison against an expected fleet size, which the orchestrator does not know.

The timestamp goes on the existing in-memory agent record, stamped where the work-request handler already resolves the caller. Losing it on restart is honest rather than lossy: every live agent re-requests within a cadence, so the count rebuilds on its own, and persisting an ops signal is not worth a migration.

### Lease release and retention pruning get separate cadences

The eviction task's interval is currently derived as `max(60s, min(testrun_eviction_age, stress_lease_budget) / 2)`, giving 150 seconds at defaults. It is computed from the stress lease and never consults the liveness lease, so an abandoned liveness wave holds its targets for up to 210 seconds against a 60 second lease.

The underlying problem is that one timer drives two jobs with opposite requirements. Releasing an expired lease wants to happen well inside the shortest lease, since every lease still held is a node excluded from assignment. Deleting results past a 7-day retention and aggregates past a 30-day one wants to happen rarely, and will want it more after the cadence change puts roughly 8M rows in the liveness tables. The existing 60 second floor is the retention half of that tension defending itself, and it is why the lease half cannot simply be tightened.

So the derived interval becomes two configured ones, driven as two timers inside the same task: `lease_release_interval`, validated against the shortest configured lease, and `retention_sweep_interval`.

*Alternatives considered.* Running retention on every Nth lease-release tick keeps one timer and one knob, but makes the retention cadence a derived number again, which is what this is moving away from, and N has to be re-derived whenever either interval changes. Splitting into two tasks gives each its own failure mode and independent observability, at the cost of taking the long-lived task set to eight and widening a spec delta that is already renaming that requirement.

This is a scheduling behaviour change inside an otherwise observability-focused change. It is included because `in_flight_expired` would otherwise report a problem this change knowingly declines to fix, immediately before a cadence change that makes it worse.

## Risks / Trade-offs

**The measurement goes blind at saturation.** Once utilisation pins at 1 the queue is always full and drain rate is capped by supply, so you know the fleet is short but not by how much. → `scale_factor`, built from `testable_population`, `staleness_age_seconds` and `wave_size` against the observed request rate, is computed from configuration rather than observation and therefore keeps reporting the shortfall past saturation. Both are needed; neither alone is sufficient.

**Sampled, not integrated.** Pending and in-flight are read on a tick, so a burst shorter than the tick is invisible. → Acceptable at a 15 second tick against a 15 minute cadence and still reasonable at 2.5 minutes. Worth revisiting if the cadence ever approaches the tick, and recorded here so the limit is known rather than discovered.

**A dead task reads as a perfectly idle fleet.** The pending-seconds counter stops growing, so utilisation falls to zero and headroom looks infinite. → `metrics_task_ticks_total` makes it a one-line alert. This is the reason that series exists.

**Renames break any existing consumer.** → Done deliberately and now, with the full mapping listed in the proposal, before the dashboards exist.

**Per-tick query cost.** Several counting queries per tick against the scheduling tables. → Roughly a thousand rows behind the relevant indexes, and SQLite runs in WAL mode so readers never block the assignment path's immediate-mode write transactions. Queries stay split per kind, following the existing `select_candidates` arms, rather than being combined into something cleverer and less readable.

## Migration Plan

No database, API or wire changes. Deployment is a normal release, but the ordering is load-bearing in two directions: `nym-metrics-label-support` must land first, and this must be deployed and scraped long enough to establish a baseline **before** the liveness cadence changes, because Prometheus cannot backfill. A day of history is enough to read utilisation and decide whether the target cadence is affordable.

Rollback is reverting the change; nothing depends on the new series.

## Settled during review

- **Telemetry tick stays 15 seconds.** A tick faster than the scrape costs only query load; a tick slower than the scrape leaves gauges stale between scrapes and coarsens the pending-time counter. 15 seconds is therefore the conservative direction whatever the deployed scrape interval turns out to be, and only needs revisiting if per-tick query cost ever shows up.
- **`agents_seen_recently` takes one configuration knob**, `agent_liveness_window`. The window has to be a few multiples of however often agents are invoked, which only the deployment knows, so the orchestrator cannot derive it. Two fixed windows would avoid the knob at the cost of two series for one concept, which is the pattern this change otherwise removes; deriving it from the shortest cadence would make the number move whenever an unrelated knob moves.
- **The eviction sweep fix is folded in here** rather than deferred to its own change, as two timers in one task. See the decision above.

## Open Questions

None outstanding.

## Why

Liveness cadence is likely to move from 15 minutes to somewhere between 2.5 and 5 minutes. Nothing the orchestrator currently emits can answer whether that fits inside existing agent capacity, or by how much the agent fleet would have to grow. The two gauges that look like they should answer it cannot: `known_agents_total` and `known_agents_announced` are seeded from every agent ever authorised in the contract, are keyed on an announced pod address that changes across restarts, and have no removal path, so they count addresses ever seen rather than agents currently working.

The information needed is already in the orchestrator's own database. The scheduler's eligibility query defines exactly which nodes are due, and the gap between a kind's work draining and becoming due again is its spare capacity. None of it is exposed, because the backlog is only ever evaluated when an agent happens to ask for work, and the per-kind in-flight gauges are recounted only by the eviction sweep on a roughly 150 second cadence against a 60 second liveness lease.

At the same time, 31 of the orchestrator's 45 series are one concept repeated per variant, a consequence of `nym-metrics` having had no labels. With labels available, those collapse to 11. Doing that now is materially cheaper than doing it later: the Grafana dashboards these metrics exist to feed have not been built yet, so renaming series today breaks nothing, whereas renaming them after the dashboards and alerts exist breaks all of them.

Prometheus has no backfill, so this has to be deployed and scraped before the cadence moves, or there is no baseline to compare against.

## What Changes

### New: a metrics task and the capacity series

- A new long-lived **metrics task**, on its own tick, recomputing the per-kind scheduling picture from storage. It does not piggyback the eviction sweep, whose cadence is too coarse to resolve a 60 second lease and gets worse as the cadence shortens.
- New per-kind series, labelled `kind`:
  - `testruns_pending`: nodes due and not in flight. The scheduler's own eligibility predicate counted rather than limited, so it cannot disagree with what an assignment would hand out.
  - `work_pending_seconds_total`: a counter accumulating elapsed time whenever pending or in-flight work exists. `rate()` over it yields utilisation at any window without losing ticks between scrapes.
  - `in_flight_expired`: in-flight rows already past their lease but not yet reaped. Required to read in-flight counts honestly, since expired leases inflate them.
  - `oldest_in_flight_age_seconds`: how long the longest-running assignment has been outstanding. The leading indicator of the condition `in_flight_expired` reports only after the fact, so narrowing headroom against the lease is visible before a lease is breached.
  - `head_lateness_seconds`: how overdue the oldest due node is, counting only nodes the kind has measured at least once. The primary "are we keeping up" signal, and the one that still quantifies severity once utilisation has pinned at 1.
  - `never_tested`: nodes the kind has no result for yet. Separate from head lateness rather than folded into it, because the scheduler orders candidates never-tested first, so a single newly bonded node would otherwise own the head permanently and leave lateness undefined.
  - `testable_population`: the scheduler's own denominator, which the `bonded_*` gauges are not, both because they count self-described roles rather than testable nodes and because they refresh only every 2 hours.
  - `required_assignment_rate`: the assignment rate the kind must sustain to hold its cadence, being the testable population over the configured staleness interval over the configured wave cap. Published as the figure rather than as its three ingredients, so no consumer has to carry configuration values in a query.
- New global series: `excluded_nodes` (bonded with no usable description, so coverage gaps surface instead of flattering utilisation), `agents_seen_recently`, and `metrics_task_ticks_total` (a watchdog, without which a dead task is indistinguishable from an idle fleet).
- A **last-seen timestamp** on the in-memory agent record, stamped where the orchestrator already resolves the calling agent on every work request. In-memory, no migration. `agents_seen_recently` is meaningless without it.

`agents_seen_recently` exists to detect silent partial fleet loss, not to size the fleet. Sizing is answered by required rate against the observed request rate and never uses a count. But because work is pulled, losing half the agents changes nothing observable: the survivors each take more work, the queue absorbs it, throughput stays flat and backlog stays at zero, right up until the remainder saturates and the fleet is already behind. A count is the only signal that shows the loss beforehand, and it also separates many agents asking occasionally from few asking constantly, which are identical in request rate and very different in resilience. The useful alert is a drop against its own recent baseline, which needs no knowledge of the expected fleet size.

### Also: the eviction sweep stops being derived from the stress lease

The sweep's cadence is currently `max(60s, min(testrun_eviction_age, stress_lease_budget) / 2)`, which is 150 seconds at defaults. It is sized from the stress lease and ignores the liveness lease entirely, so an abandoned liveness wave holds its targets for up to 210 seconds against a 60 second lease, as the task's own doc comment notes. At a 2.5 minute cadence the wave rate rises roughly sixfold, multiplying both how often that happens and how much of a shorter cycle it consumes.

The two jobs the sweep performs want opposite cadences: releasing expired leases wants to run well inside the shortest lease, while deleting results and aggregates past a 7-day and 30-day retention has no business running every half minute. That tension is what the existing 60 second floor is defending against, and it worsens after the cadence change, since the liveness tables reach roughly 8M rows at 2.5 minutes. So the single derived interval becomes two configured ones, `lease_release_interval` and `retention_sweep_interval`, driven as two timers inside the one existing eviction task. The task count is unaffected.

- Three new configuration knobs in total: the telemetry tick interval, the agent liveness window, and the split sweep intervals replacing the derived one.

### Modified: existing series collapse onto labels

**BREAKING** for anything currently consuming these series by name. 31 names become 11:

| Today | Becomes |
|---|---|
| 3 `*_assignments` | `testrun_assignments_total{kind}` |
| 3 per-kind `*_testruns_in_progress` plus the `testruns_in_progress` total | `testruns_in_flight{kind}`, with the total as `sum()` in PromQL |
| 3 `*_leases_expired` | `testrun_leases_expired_total{kind}` |
| 2 `*_wave_size` | `liveness_wave_size{kind}` (both already share one bucket set) |
| 3 `performance_submission_*_omitted` | `performance_submission_kind_omitted{kind}` |
| 4 `bonded_*_nym_nodes` | `bonded_nym_nodes{role}` |
| 3 `submitted_results_*` | `submitted_results_total{outcome}` |
| 4 announce dispositions plus `agent_announce_requests` | the request total, plus `agent_announce_outcome_total{outcome}` |
| 3 testrun-request dispositions plus `empty_testrun_assignments`, plus `agent_testrun_requests` | the request total, plus `agent_testrun_request_outcome_total{outcome}` |

A request total stays its own series rather than becoming an `outcome` value, so a breakdown is never summed together with the total it breaks down.

- `test_duration_seconds` gains a `kind` label and widened buckets. Today one unlabelled histogram bucketed for a 40 to 60 second cluster carries both a liveness target bounded by a 60 second lease and a stress run bounded by a 300 second one, so neither distribution is readable. Wave duration against the fixed lease is the input the cadence decision turns on. A histogram family has one bucket set across all its children, so the buckets widen to span both ranges rather than being tuned per kind.
- **REMOVED**: `known_agents_total` and `known_agents_announced`, replaced by `agents_seen_recently`.

Net effect: 45 series today become 23 after the collapse, plus 11 new ones, so 34 in total while gaining the capacity and agent telemetry that motivated the change.

Three per-kind dispatch helpers disappear with the collapse: `in_progress_metric` and `leases_expired_metric` in the storage layer, and the kind match inside the assignment metric emitter.

### Non-goals

- **Per-agent series.** Needs a label whose values do not churn with pod restarts, plus disciplined removal. `agents_seen_recently` gives the fleet count that sizing needs; naming which agent went quiet does not follow from it.
- **Attributing a completed run to the agent that performed it.** Needs the agent carried on `testrun_in_progress` into the result tables, which is a migration.
- **Per-wave duration.** A wave finishes when its last target submits, which the task can only infer by watching rows vanish. Measuring it properly means instrumenting the submission site and grouping by kind and dispatch time.
- **Labelling `test_packets_sent` and `test_packets_received` by direction.** Technically the same shape, but keeping them separate makes rate comparisons read more directly, and nothing here needs it.
- **Changing the liveness cadence.** This change measures; the cadence moves separately, and only once a baseline exists.

## Capabilities

### Modified Capabilities

- `nym-network-monitor`: the long-lived task set grows from six tasks to seven and the eviction task's derived cadence becomes two configured ones; the configuration requirement gains the telemetry tick interval, the agent liveness window and the two sweep intervals; the eviction requirement separates lease release from retention pruning; and a new set of requirements covers the orchestrator's telemetry, which the capability does not describe today at all.

## Impact

- `src/orchestrator/`: a new metrics task module alongside the existing ones, and `prometheus.rs` for the new, relabelled and removed series.
- `src/storage/`: per-kind counting queries for pending, expired-in-flight, testable population and excluded nodes, following the existing `select_candidates` arms rather than combining them, and reusing the existing head-of-queue lookup. Reads only, no schema change. SQLite runs in WAL mode so these never block the assignment path's write transactions. The two per-kind metric-name dispatch helpers are deleted.
- `src/http/state.rs`: a last-seen field on the cached agent record, the collapsed assignment emitter, and removal of the two agent gauges.
- `src/http/api/v1/agent/mod.rs`: stamp last-seen where the caller is already resolved, and emit announce and request dispositions as labelled outcomes.
- `src/submission/mod.rs`: the per-kind omission gauges become one labelled family.
- `src/orchestrator/stale_results_eviction.rs`: the derived `check_interval` is replaced by two configured intervals driven as two timers in the same task, with lease release and retention pruning separated into their own passes.
- `src/cli/run_orchestrator.rs`: the telemetry tick, agent liveness window and two sweep intervals, plus task wiring.
- **Depends on `nym-metrics-label-support`**, which must land first.
- No API, database or wire changes. Deployment order matters: scrape this before the cadence changes.

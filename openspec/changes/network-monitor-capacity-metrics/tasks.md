## 1. Storage reads

- [x] 1.1 Per-kind count of nodes due and not in flight, applying the same predicate as `select_candidates` counted rather than limited, as one static query per existing kind arm rather than a combined query
- [x] 1.2 Per-kind count of in-flight rows whose `expires_at` has already passed
- [x] 1.3 Per-kind count of the testable population: bonded, described, carrying that kind's role
- [x] 1.4 Count of bonded nodes with no usable description
- [x] 1.5 Head-of-queue lookup for head lateness restricted to nodes the kind has measured at least once, deriving the overdue amount from the head's due time; the existing lookup orders never-tested first, so it cannot be reused unchanged
- [x] 1.7 Per-kind count of nodes the kind has no result for, meaning described and role-carrying with no `last_tested_at` under that kind
- [x] 1.6 Per-kind oldest `started_at` among in-flight rows as a `MIN` query, with the age derived against the same `now` the rest of the tick uses, and no row returning nothing rather than a sentinel

## 2. Labelled families and the collapse

- [x] 2.1 Rework `PrometheusMetric` so a variant declares its label names, and registration builds a labelled family where it has any
- [x] 2.2 Collapse the three `*_assignments` counters into one kind-labelled family, removing the kind match in the assignment emitter
- [x] 2.3 Collapse the three per-kind in-progress gauges and the separate total into one kind-labelled family, removing the `in_progress_metric` helper; the total becomes a sum at query time
- [x] 2.4 Collapse the three `*_leases_expired` counters, removing the `expired_leases_metric` helper
- [x] 2.5 Collapse the two wave-size histograms into one kind-labelled family, which needs no bucket change since both already share one set
- [x] 2.6 Collapse the three `performance_submission_*_omitted` gauges into one kind-labelled family
- [x] 2.7 Collapse the four `bonded_*_nym_nodes` gauges onto a role label
- [x] 2.8 Collapse the three `submitted_results_*` counters onto an outcome label
- [x] 2.9 Collapse the announce dispositions (duplicate, contract success, contract failure, malformed addresses) onto an outcome-labelled family, keeping the announce-request total as its own series
- [x] 2.10 Collapse the testrun-request dispositions (unknown agent, not announced, no work assigned) onto an outcome-labelled family, keeping the request total as its own series
- [x] 2.11 Give `test_duration_seconds` a kind label and widen its buckets to span both the liveness lease and the longer stress lease, since a family carries one bucket set across its children
- [x] 2.12 Pre-create every child at `initialise` using `init_label_values`, so each series is exposed zeroed from the first scrape and no first observation reads as a counter reset
- [x] 2.13 Remove `known_agents_total` and `known_agents_announced` along with the cache's gauge-publishing path

## 3. New capacity series

- [x] 3.1 Per-kind pending gauge, fed by 1.1
- [x] 3.2 Per-kind pending-time counter, advanced by measured elapsed time on each tick where pending or in-flight work exists, so a retuned tick interval and a missed tick stay correct
- [x] 3.3 Per-kind expired-in-flight gauge, fed by 1.2
- [x] 3.4 Per-kind head lateness gauge over measured nodes only, fed by 1.5, reporting zero when every measured node is within cadence
- [x] 3.10 Per-kind never-tested gauge, fed by 1.7
- [x] 3.5 Per-kind testable population gauge, fed by 1.3
- [x] 3.6 Per-kind required assignment rate gauge, as testable population over the configured staleness interval over the configured wave cap, taking the cap from configuration rather than from observed wave sizes; the intervals and caps themselves are not published
- [x] 3.7 Excluded-nodes gauge, fed by 1.4
- [x] 3.8 Telemetry tick counter
- [x] 3.9 Per-kind oldest outstanding dispatch age gauge, fed by 1.6, reporting zero when the kind holds nothing in flight so an idle kind is not confused with one holding a very old dispatch

## 4. Agent liveness

- [x] 4.1 Add a last-seen timestamp to the cached agent record (`KnownAgent::last_active_at` already existed, written on announce and read nowhere)
- [x] 4.2 Stamp it in the work-request handler at the point the caller is already resolved from the cache
- [x] 4.3 Publish the count of agents whose last request falls within `agent_liveness_window`

## 5. Eviction sweep split

- [ ] 5.1 Replace the derived `check_interval` with two configured intervals, `lease_release_interval` and `retention_sweep_interval`, removing the `MIN_CHECK_INTERVAL` floor and the stress-lease derivation along with the doc comment that documents its consequence
- [ ] 5.2 Separate the sweep's two duties into their own passes, so releasing expired dispatches no longer runs together with deleting rows past retention
- [ ] 5.3 Drive both passes from one task on two timers, keeping the single blocking pre-serve sweep that releases rows left by a prior crash
- [ ] 5.4 Reject at configuration time a `lease_release_interval` longer than the shortest configured lease budget

## 6. Task and configuration

- [x] 6.1 New telemetry task module alongside the existing long-lived tasks, on its own interval with an immediate first tick, refreshing every series in groups 3 and 4.3 from one pass
- [ ] 6.2 Add `metrics_refresh_interval` (default 15 seconds), `agent_liveness_window` (default 15 minutes), `lease_release_interval` (default 30 seconds) and `retention_sweep_interval` (default 1 hour) to the orchestrator CLI and config
- [x] 6.3 Spawn the telemetry task under the shutdown manager with the other six, and update the startup task-count assertion and logging (there is no runtime task-count assertion or "started N tasks" log in this crate; the count lives only in the capability prose, which the spec delta rewrites)

## 7. Tests

- [x] 7.1 The per-kind due count is drawn from the same set an assignment would lock, exercised by counting and then assigning against one fixture
- [x] 7.2 A kind whose due nodes are all in flight reports a zero pending count with a positive in-flight count, rather than reading as idle
- [x] 7.3 An in-flight row past its lease is counted both as in flight and as expired
- [x] 7.4 The pending-time counter does not advance across ticks where a kind has neither due nor in-flight work, and does advance where it has either
- [x] 7.5 A bonded node with no usable description is absent from every kind's population and is counted among excluded nodes
- [x] 7.6 An agent whose last request predates the window leaves the recently-seen count, and a restarted agent holding a second cache entry on a new address is counted once
- [ ] 7.7 Partial fleet loss moves the agent count while leaving pending counts and head lateness unchanged, which is the case the count exists for
- [ ] 7.8 A labelled family exposes one series per kind under a single `TYPE` line, and every child is present and zeroed before any event is recorded
- [ ] 7.9 An expired dispatch of the shortest-lease kind is released on a lease-release pass without a retention pass having run
- [ ] 7.10 A `lease_release_interval` longer than the shortest lease budget fails configuration construction
- [x] 7.11 A dispatch outstanding but not yet past its deadline raises the oldest outstanding age while the expired count stays zero, and a kind holding nothing in flight reports zero rather than carrying its last value
- [x] 7.12 A newly described, never-tested node raises the never-tested count without disturbing head lateness, which is the case that would otherwise make the primary alert unusable

## 8. Verification

- [ ] 8.1 `cargo test -p nym-network-monitor-orchestrator` green
- [ ] 8.2 `cargo check -p nym-network-monitor-orchestrator --tests` clean, since the collapse touches every per-kind emission site
- [ ] 8.3 Confirm against a running orchestrator, or the initialise path, that the full series set is exposed zeroed before any work is assigned
- [ ] 8.4 `cargo fmt` before committing, so the pre-commit hook does not reformat into the commit

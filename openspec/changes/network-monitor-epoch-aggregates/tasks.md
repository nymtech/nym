## 1. Mixnet epoch awareness

- [x] 1.1 Add an epoch source backed by the mixnet contract, using the nyxd client the orchestrator already holds for agent authorisation, exposing the current epoch id and the start timestamp of any given epoch id
- [x] 1.2 Name the identifier `mixnet_epoch` throughout - columns, types and config - never the bare `epoch`, which `storage/models.rs` already uses for the unrelated sphinx `key_rotation_id`
- [x] 1.3 Answer from the interval reading already held until the epoch it describes is over, and return an error rather than extrapolating from a stale anchor when the contract cannot be reached (Decision 7)
- [x] 1.4 Unit tests: an epoch id maps to the expected start timestamp; an unreachable contract yields an error rather than a guessed boundary

## 2. Sample table

- [x] 2.1 Migration adding the sample table: an id, node id, test kind, assignment timestamp, and a NULLABLE score, plus an index supporting a `(node, kind, timestamp)` range scan
- [x] 2.2 Write a sample row from `assign_next_testruns` as each target is handed out, leaving the score unset. `testrun_in_progress` keeps its existing role as the per-node mutex, gaining only the id of the sample its assignment created
- [x] 2.3 Fill the score in on the result-submission path, matching the result back to its row by id rather than by `(node, kind, timestamp)`, since the result's timestamp is not the assignment's
- [x] 2.4 `storage/manager.rs`: a windowed read returning, per node, the scored samples of one kind in the half-open range `[start, end)`. Read for the whole population in one query rather than per node, since materialising an epoch asks about every node in the registry
- [x] 2.5 The same read carries what the coverage classification is derived from: alongside the scores, how many of that node's assignments never returned, with a node that was never assigned absent from the result entirely. No lease deadline is consulted: the submission path already drops a result whose in-flight row has been reaped, so a sample can only be scored while its lease is live
- [x] 2.6 Tests: an assignment creates an unscored row and its result scores it; an expired lease leaves it unscored; the windowed read excludes a sample exactly at the upper bound and includes one exactly at the lower

## 3. Aggregate storage

- [ ] 3.1 Migration adding the materialised aggregate table keyed `(epoch_id, node_id, test_kind)`, holding the score, the contributing sample count, and the coverage classification
- [ ] 3.2 Add the corresponding types to `storage/models.rs`, keeping the coverage classification a named enum rather than a bare string or integer
- [ ] 3.3 `storage/manager.rs`: idempotent upsert of an aggregate row, a point read for one `(node, epoch)` across kinds, and a read of every aggregate for an epoch
- [ ] 3.4 Tests: a repeated upsert neither duplicates nor alters

## 4. Aggregate computation

- [ ] 4.1 Compute the aggregate as the arithmetic mean of the scored samples for a `(node, kind)`, explicitly NOT pooling the underlying packet counts (Decision 2)
- [ ] 4.2 Collapse every role and every tested address for that node and kind into the single value (Decision 9)
- [ ] 4.3 Count a run carrying a run-level error as a 0.0 sample rather than excluding it, matching the existing rule that unmeasurable must not score better than measurably broken
- [ ] 4.4 Derive the three-way coverage classification from 2.5: assigned and returned, assigned and never returned, never assigned
- [ ] 4.5 Tests: one sample of 1.0 over 50 packets plus one of 0.0 over 3 packets yields 0.5 rather than the pooled ratio; an errored run pulls the mean down; a dual-role node produces one value; each coverage classification is reachable

## 5. Materialisation

- [ ] 5.1 Add per-kind aggregation window configuration with CLI and env wiring in `cli/run_orchestrator.rs`, defaulting to 6 hours for liveness and 24 hours for stress, with NO validation against the kind's test interval (Decision 3)
- [ ] 5.2 Add a task that fires at each epoch transition and materialises aggregates for every node in the registry, for every kind
- [ ] 5.3 Make the task idempotent, so a repeated run for an already-materialised epoch is a no-op rather than a duplicate or an overwrite with different values
- [ ] 5.4 On startup, backfill every epoch begun since the last materialised one whose window is still covered by retained samples, and skip those that are not (Decision 5). A first deployment has no last-materialised epoch and so takes the same path, materialising the epoch already in progress - no separate first-run branch
- [ ] 5.5 Tests: a result arriving after its epoch was materialised does not change that epoch's value but does contribute to a later one; a missed epoch within retention is backfilled; one beyond retention is skipped rather than computed from truncated data

## 6. Sample retention

- [ ] 6.1 Add a sample retention setting with its own default, independent of `testrun_eviction_age`
- [ ] 6.2 Validate at startup that sample retention exceeds the longest configured aggregation window by a margin sufficient for backfill, failing with an error naming both values
- [ ] 6.3 Tests: the shipped defaults start cleanly; a sample retention below the longest window is rejected; changing `testrun_eviction_age` does not affect startup

## 7. HTTP endpoint

- [ ] 7.1 Add the response types to `nym-network-monitor-orchestrator-requests`, with one entry per kind carrying value, sample count and coverage classification, shaped so a further sibling such as config score can be added without a breaking change
- [ ] 7.2 Add the handler and route under `http/api/v1`, following the existing endpoints' auth convention. No pagination: the population is around a thousand nodes and each entry is small
- [ ] 7.3 Omit a kind with no aggregate from the response rather than reporting it as 0.0, so absence stays distinguishable from a measured zero
- [ ] 7.4 Tests: each kind is reported with its own evidence; a node with an aggregate for one kind only omits the other

## 8. Verification

- [ ] 8.1 Deploy and let the windows fill, confirming the first materialised epoch computes over a partial window without error and that coverage reaches full after 6 hours for liveness and 24 for stress
- [ ] 8.2 Compare the endpoint's values against nym-api's scores for a sample of nodes over the same period, confirming they agree within the divergence Decision 1 predicts, since nym-api anchors its window at the epoch's end and this anchors at its start
- [ ] 8.3 Confirm that a node never assigned work in a window is classified as such rather than appearing as a measured zero, and that a node whose assignments expired unscored is classified differently again
- [ ] 8.4 Confirm that discarding the aggregate table does not block startup and that values are rebuilt as the window refills, per Decision 8

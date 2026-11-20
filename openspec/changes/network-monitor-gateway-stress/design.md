## Context

The network monitor has three test kinds. `mixnode_liveness` and `mixnode_stress` are one probe at two volumes: the agent sends sphinx packets through the node back to itself, 50 packets for liveness and 30,000 at 1000 packets/second for stress. `gateway_liveness` is a different probe: the agent registers a client session over the gateway's plain websocket and measures two interfaces in turn, `client_ingest` (forwards pushed in over the session, received on the agent's mixnet listener) and `client_delivery` (final-hop packets sent to the gateway's mix port, received over the session). Gateways have no stress kind.

What shapes this change:

- The mixnode stress probe cannot be pointed at a gateway. A gateway-only node does not process forward hops, which is why the gateway liveness probe avoids them.
- An agent announces one client identity and a gateway accepts one live session per identity, so one agent cannot present many concurrent clients.
- An announced monitor identity gets an unmetered, unpersisted session, so volume costs no bandwidth credential. nym-node needs no change.
- The gateway probe is already driven by a profile: both phases pace from it and each sends the profile's count in full.
- The performance contract's `stress` measurement carries no role, and its score renormalises over the kinds a node's bundle holds. A gateway-only node is scored on liveness and config alone today.
- Kinds are flat by design: one probe against one role, one results table per kind, per-kind SQL written out in full. A fourth kind follows that mould.
- The orchestrator's nym-api result stream no longer exists on this branch, so results reach scoring only through the contract submission.

## Goals / Non-Goals

**Goals:**

- Give gateways the liveness-plus-stress parity mixnodes have, with the two gateway kinds being one sequence at two volumes.
- Change nothing in the performance contract, nym-api or nym-node.
- Start with a pass mark gateways can plausibly meet, and make raising it a deployment change.

**Non-Goals:**

- Stressing a gateway's exit services (IP packet router, network requester, WireGuard), its wss path or the Lewes Protocol data plane.
- Simulating many concurrent clients.
- A separate contract measurement or weight for gateway stress.
- Changing the scheduler's policy, or the mixnode probe (its send loop keeps checking the window between batches).

## Decisions

### 1. Gateway stress is the gateway liveness sequence under a stress profile, one target per assignment

`gateway_stress` establishes one client session, runs the ingress phase and then the egress phase, and reports `client_ingest` and `client_delivery`, exactly as `gateway_liveness` does. Only the profile and the arity differ: one target, as for `mixnode_stress`.

This is the relation the mixnode kinds have, and it reuses the session, the shared listener, the result shape and the scoring rule (the mean of the two interfaces' delivery ratios). The code comments that say a gateway stress test "would not resemble" the liveness probe are wrong under this decision and are rewritten.

Alternatives considered: the mixnode stress probe against the gateway's mix port (impossible, forward hops are dropped); loading only one of the two interfaces (says nothing about the other, and breaks the one-sequence parity); an exit-traffic probe (a different project that needs a far end, and explicitly outside the liveness design's scope).

### 2. No check before the load

The mixnode probe sends one packet and a replay of it before its load and aborts if either does not return. The gateway sequence has no such step and gains none: a phase that returns nothing is measured as a zero and the other phase still runs.

Adding a check to stress alone would make the two gateway kinds two sequences. The cost is a full run spent on a gateway whose session works and whose paths are dead, and no distinction inside a stress result between "forwards nothing" and "drops under load"; the liveness result beside it makes that distinction.

Alternatives considered: a one-packet check per phase that skips only that phase's load; a check that aborts the run as the mixnode probe does (leaves the other interface unmeasured).

### 3. A phase's send window ends the phase, a blocked write included

Each gateway phase stops sending once the profile's send window (`sending_duration`) has elapsed since its first dispatch. The sent count is already forced to the expected count, so what the gateway did not take counts as lost: a gateway that cannot take the load in time is not sustaining it.

The bound has to interrupt a write, not only be checked between batches. Under back-pressure the websocket or Noise write itself blocks, and a check between batches would leave the phase hanging until the run's deadline, at which point the egress phase reports zero without having run. So the phase's send loop is raced against the end of its window. A profile's last dispatch goes out one batch interval before the window ends (29.95 s into a 30 s window at the gateway stress values, 0.8 s into the liveness profile's 1 s), so a gateway that keeps up is never cut.

The bound sits in the phase loop both gateway kinds share. For `gateway_liveness` this is a behaviour change only under back-pressure: 50 forwards that do not go out inside the one-second window are no longer given until the 30 second deadline.

An abandoned write leaves the rest of its batch queued on the session. The egress phase only reads the session, so it is unaffected. The session's close sends that queue first and would block on a gateway that has stopped reading, which can already happen after a deadline today, so the close is bounded by a fixed limit after which the connection is dropped.

Alternatives considered: bounding the stress kind only (a kind-conditional inside the loop, and two sequences again); no bound, relying on the run's deadline (a slow ingress path then zeroes the egress measurement too).

### 4. A phase cut short still counts what arrived

Today a phase that fails part-way returns before its drain, so it reports zero received whatever had arrived, and a run deadline landing mid-phase does the same. With thousands of packets and a window that ends phases on purpose, that would turn "sustained 80% of the rate" into a score of zero.

Every exit from a phase's send step (completed, window ended, send failed) therefore goes on to the straggler wait and the count, scored against the full expected count; a send failure is still recorded as that interface's local error. When the run's deadline cuts a phase off, the packets already in that phase's inbox are counted without waiting further, as the mixnode probe does at its deadline. A phase the deadline left no time for keeps its zeroed measurement.

The cost: a gateway whose session dies under load keeps credit for what it delivered first, and the failure is visible only in the agent's log, since a per-interface error has no field on the wire.

### 5. Gateway stress has its own profile, with lenient provisional values

Five agent knobs, each with an environment variable: `--gateway-stress-target-rate`, `--gateway-stress-sending-duration`, `--gateway-stress-sending-batch-size`, `--gateway-stress-waiting-duration` and `--gateway-stress-per-target-timeout`. The profile is built like the mixnode stress one, a rate held for a duration with the count derived, and applied in full to each phase.

Initial values: 200 packets/second for 30 seconds (6000 packets per phase), batches of 10 (a dispatch every 50 ms, the mixnode stress cadence), a 5 second straggler wait. Gateways have never been tested under load and are expected to be less resilient than mixnodes, so the rate starts at a fifth of the mixnode profile's. It is still four times the liveness rate held thirty times as long. The values are provisional and are raised progressively through deployment configuration as runs against real gateways show what they sustain; the PR description says so.

The existing stress knobs (`--target-rate`, `--sending-duration`, `--sending-batch-size`, `--waiting-duration`, `--stress-per-target-timeout`) are renamed `--mixnode-stress-*`, and their environment variables `NYM_NETWORK_MONITOR_AGENT_MIXNODE_STRESS_*`, so that beside the gateway stress knobs neither can be read as the other's. The old names are dropped, not aliased: the deployment scripts are updated in the same redeploy.

Alternatives considered: sharing the mixnode stress profile (no new knobs, but one pass mark for two very different paths); the mixnode values as the starting point (no evidence gateways meet them, and the result is reward-affecting); keeping the existing names (ambiguous beside `--gateway-stress-*`); keeping the old names working (clap aliases a flag but not an environment variable, so each old variable would need a hidden argument of its own).

### 6. Its own per-target deadline, 3 minutes, unvalidated

With both phases bounded, a run takes at most session setup, the egress connection, two send windows and two straggler waits, about 93 seconds at the initial values. The deadline is a backstop for a hang outside those bounds. It gets its own knob because the gateway duration is tunable on its own, and a shared deadline would couple the two stress kinds' tuning. 3 minutes leaves about 90 seconds over the bounded run and 2 minutes under the 5 minute stress lease. As with the existing deadlines, nothing validates it against the profile or the lease.

### 7. Scheduled on the stress cadence, behind its own switch

`gateway_stress` uses `test_interval` (2 hours) and `test_timeout` (5 minutes) with a wave size of one, and is declared after `mixnode_stress`, so ties resolve as liveness, mixnode stress, gateway stress. `--gateway-stress-enabled` takes an explicit value and defaults to on, like the liveness switch; `Config::schedule` returns nothing for the kind while it is off.

Sharing the cadence mirrors the liveness family and keeps a dual-role node's two stress kinds at comparable sample counts when they are combined. The shared lease has to cover the slower run, which is the gateway one. The asymmetry is accepted: mixnode stress still has no switch.

Alternatives considered: its own interval and lease (two more flags, and the more frequently tested kind would then dominate a dual-role node's combined stress); no switch (the kind could not be turned off without a build).

### 8. The kind is plumbed the way the existing three are

- Wire types: `TestKind::GatewayStress`, `TestRunAssignment::GatewayStress(Box<GatewayProbeTarget>)`, `RunMeasurements::GatewayStress { client_ingest, client_delivery }`, and `latest_gateway_stress` on the node read type. The kind still derives from the result's variant, so a gateway stress result for a node dispatched for gateway liveness is rejected although the two carry the same interfaces.
- Storage: one migration inserts the `gateway_stress` row into `test_kind` and creates `gateway_stress_testrun` with the columns and indexes of `gateway_liveness_testrun`. The work-state, in-flight and aggregate tables are keyed by kind already and are untouched. One row struct serves both gateway tables, as `MixnodeTestRunRow` serves both mixnode tables. The per-table query functions are written out in full for the new table, per the repository's readable-SQL convention.
- Scheduling predicates and coverage eligibility treat the two gateway kinds alike: the node must report the entry-gateway role.
- Metrics are labelled by kind and pick the new kind up from the enum; it has no wave-size series, being single-target.

### 9. Reported as the contract's existing `stress`

The orchestrator already folds the two liveness kinds into one liveness figure by sample count. The stress arm of that fold overwrites today, because only one stress kind exists; it becomes the same combine. Both stress kinds use the stress aggregation window, which is what keeps the combination exact. The coverage guard is per test kind, so `gateway_stress` gets its own figure: a broken gateway stress probe is omitted on its own, leaving gateway-only nodes with liveness and dual-role nodes with their mixnode stress.

Nothing changes in the contract (its `stress` measurement has no role and absent kinds are renormalised away) or in nym-api (its annotation maps the stress median with no role check).

The cost is one weight and one median for both roles, and a step in every gateway's committed score in the epoch gateway stress first clears the coverage guard. The guard holds the kind back until half the gateways have a reachable aggregate, then admits it for all of them at once.

Alternative considered: a separate contract measurement with its own weight. It would change four contract types, the weights validation and score loop, the schema and the served medians, and need a migration and a weights update, to buy separate weighting nobody has asked for.

### 10. The gateway probe takes the kind

The probe resolves its profile, its deadline and its result variant from the kind it is constructed for, where today all three are fixed to gateway liveness. A gateway stress assignment runs as a gateway wave of one, as a mixnode stress assignment runs as a mixnet wave of one. The session's receive timeout comes from the run's own profile. The probe and wave types are renamed so they no longer say "liveness". A manual `test-gateway-stress` command joins the three existing ones, sharing the gateway command's module.

### 11. What is tested

Orchestrator behaviour is covered against the in-memory database as the existing kinds are: per-kind table isolation, eligibility, the switch and tie order, the stress fold, coverage and the submission plan. On the agent, a gateway stress run is checked to report under its own kind.

The phase rules of Decisions 3 and 4 are not unit-tested: they live on the run, which needs a live client session, and the repository allows neither a live socket in a unit test nor a test-only seam on production code. The one run-level test covers a session that cannot be established. The untested scenarios are listed in task 7.2, not forced with a new seam. No test pins a default to its literal.

## Risks / Trade-offs

- [The pass mark is a judgement, not a measurement] → lenient initial values, every one tunable by environment, and task 7.3 measures real gateways with the manual command to decide each increase. It is not a gate on the first deploy.
- [Reward-affecting step in gateway scores when the kind clears the coverage guard] → small for a healthy gateway at the initial values; called out in the PR description.
- [The kind takes every agent request until each gateway has been tested once] → a consequence of the scheduler's never-tested rule, accepted as it was for the liveness kinds. At single-target runs of about 70 seconds it lasts longer than a liveness first pass, and it is short against the 6 hour liveness window.
- [Renamed mixnode stress knobs] → a deployment still setting an old variable gets the mixnode stress defaults without an error. The deployment scripts are switched in the same redeploy, and the PR description lists each rename.
- [Agent and orchestrator must match] → an agent built before this change cannot parse a gateway stress assignment and would leave its target locked for the lease. The two are redeployed together; `--gateway-stress-enabled false` is the fallback.
- [One rate serves both interfaces] → the websocket ingest loop and the mix-port delivery path probably have different ceilings, so the rate has to suit the slower one. Per-phase rates are not introduced.
- [The egress phase follows the ingress phase] → a gateway still working through an ingress backlog is measured on delivery under that load. Not designed around: a gateway in that state is not sustaining the profile.
- [Gateway liveness changes under back-pressure and on a mid-phase failure] → intended, and stated in the spec delta and the PR description.
- [A write abandoned at the window] → the egress phase only reads the session, so it is unaffected; the close, which would otherwise wait on a gateway that stopped reading, is bounded by a fixed limit (Decision 3).
- [Two scenario titles are renamed] → `openspec archive` treats a renamed scenario as a dropped one and aborts, so the main specs are synced by hand and the change is archived with `--skip-specs` (task 7.5).

## Migration Plan

1. Deploy the orchestrator and the agents together, with the agents' deployment scripts switched to the renamed mixnode stress variables. The migration is additive: one `test_kind` row and one new table.
2. Gateway stress starts at once (the switch defaults to on) and is offered on every request until its first pass completes. It enters the contract submission in the first epoch in which half the gateways have a reachable aggregate.
3. Raise the profile's values through the agents' environment as measurements allow.

Rollback: set `--gateway-stress-enabled false` to stop assigning the kind; its aggregates age out of the stress window and the coverage guard then omits it. Rolling the orchestrator binary back past the migration means discarding its database, which is a rebuildable cache.

## Open Questions

- How fast, and to what values, the profile is raised. Decided from the measurements of task 7.3, outside this change.

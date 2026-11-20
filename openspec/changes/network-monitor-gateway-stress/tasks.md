## 1. Wire types

- [x] 1.1 In the orchestrator requests crate, add `TestKind::GatewayStress` (declared last), `TestRunAssignment::GatewayStress(Box<GatewayProbeTarget>)` and `RunMeasurements::GatewayStress { client_ingest, client_delivery }`, with their `kind()` and `all()` arms
- [x] 1.2 Add `latest_gateway_stress` to `NymNodeWithTestRuns`, and update the doc comments in the crate that describe three kinds or a gateway probed only for liveness

## 2. Agent: phase rules shared by both gateway kinds

- [x] 2.1 End a gateway phase's sending at the profile's send window, interrupting a blocked write; the sent count stays the expected count
- [x] 2.2 Make every exit from a phase's send step (completed, window ended, send failed) continue into the straggler wait and the count, keeping a send failure as the interface's local error
- [x] 2.3 On the run's deadline, count the packets already arrived for the phase in flight, without waiting further: each phase runs under its own `timeout_at` against one deadline instant, arrivals collect in a `received` field on the run (as `ProbeRun::received` does), and `per_target_timeout` returns a `Duration` rather than an `Option`
- [x] 2.4 Bound the session's close by a fixed limit (a constant, 5s), after which the connection is dropped (design Decision 3)
- [x] 2.5 No unit tests: the phase rules live on the run, which needs a live session (design Decision 11). Note the scenarios of "A gateway phase stops sending when its send window ends and counts what arrived" for 7.2

## 3. Agent: gateway stress

- [x] 3.1 Config: add the gateway stress profile and its per-target timeout to `NodeTesterConfig`, give `profile_for` and `per_target_timeout` their `GatewayStress` arms, and take the session's receive timeout from the profile of the kind being run
- [x] 3.2 CLI: add the five `--gateway-stress-*` flags and environment variables with the initial values (200 packets/second, 30s, batches of 10, 5s wait, 3m timeout), and rename the five existing stress flags and their environment variables to `--mixnode-stress-*` / `NYM_NETWORK_MONITOR_AGENT_MIXNODE_STRESS_*` (code fields to match), with no aliases and no check for the old names
- [x] 3.3 Make the gateway probe and its wave take the kind (profile, deadline, result variant); add `ProbeMeasurements::GatewayStress` with its `unmeasured` and wire-conversion arms and its arm in the mixnet probe's refusal of gateway kinds; rename the types so they no longer say liveness; rewrite the comments claiming a gateway stress test would not resemble the liveness probe
- [x] 3.4 Dispatch `TestRunAssignment::GatewayStress` as a gateway wave of one
- [x] 3.5 Add the manual `test-gateway-stress` command beside `test-gateway-liveness`
- [x] 3.6 Unit test: a gateway stress run that cannot establish its session reports two zeroed measurements under the gateway stress variant (profile and deadline selection are not tested: done by defaults they would pin literals)

## 4. Orchestrator storage

- [x] 4.1 Migration `07_gateway_stress.sql`: insert `gateway_stress` into `test_kind` and create `gateway_stress_testrun` with the columns and indexes of `gateway_liveness_testrun`
- [x] 4.2 Storage `TestKind::GatewayStress` (declared last) with its conversions to and from the wire kind; one row struct for both gateway tables with `into_gateway_stress`; the `DescribedNodeRoles::is_tested_by` arm
- [x] 4.3 The per-table functions for `gateway_stress_testrun` (insert, by id, page, per-node page, in window, eviction) and their arms in each per-kind dispatch
- [x] 4.4 Scheduling predicates: the `select_candidates` arm, the due-count, never-tested-count and measured-head queries, and the facade arms that count candidates, lateness and the testable population
- [x] 4.5 Tests: every figure of a gateway stress run round-trips; per-kind table isolation and eviction cover the new table; a gateway stress result for a node dispatched for gateway liveness is rejected; a gateway-only node is a candidate for the gateway kinds only

## 5. Orchestrator scheduling, config and HTTP

- [x] 5.1 Add `--mixnode-stress-enabled` and `--gateway-stress-enabled` (explicit values, default true, each with its environment variable) and carry them into `Config`; `Config::schedule` gives gateway stress the stress interval and lease with a wave of one, and gives each stress kind nothing while its switch is off
- [x] 5.2 `AggregationWindows::for_kind`: gateway stress uses the stress window
- [x] 5.3 HTTP state: build the single-target gateway stress assignment, give the assignment metrics their arm, and serve `latest_gateway_stress` on the node read
- [x] 5.4 Tests: a disabled stress kind (either one) is never chosen while the other kinds still are; a cold start reaches gateway stress after mixnode stress; a gateway stress assignment carries one target

## 6. Aggregation and submission

- [ ] 6.1 `ReportedAggregates::add_row`: combine the two stress kinds by sample count, as the liveness kinds are
- [ ] 6.2 `Coverage`: one figure per test kind (`mixnode_stress` in place of `stress`, plus `gateway_stress`), with `measure` and `for_kind` to match
- [ ] 6.3 Tests: a dual-role node's stress combines both kinds' runs; a gateway-only node's gateway stress is submitted as stress; with gateway stress omitted, a gateway-only node is submitted with liveness alone and a dual-role node with its mixnode stress; a gateway does not count against mixnode stress coverage

## 7. Verification and close-out

- [ ] 7.1 `cargo fmt`; `cargo check --all-targets` and the tests of the agent, the requests crate (also with `--features openapi`) and the orchestrator
- [ ] 7.2 Read the diff against every scenario of the three delta specs, and record in design Decision 11 which scenarios are left untested and why
- [ ] 7.3 Run `test-gateway-stress` against real gateways and record the rate each sustains, as the input for raising the profile. Not a gate on the first deploy
- [ ] 7.4 PR description: the new kind, the gateway liveness behaviour change (send window, partial credit), the lenient initial values with the note that they will be raised progressively, the reward effect on gateways, the two new orchestrator switches `--mixnode-stress-enabled` and `--gateway-stress-enabled`, that orchestrator and agents deploy together, and a section of its own on the renamed mixnode stress flags and environment variables: a table of every old name to its new one, and the warning that a deployment script still setting an old variable silently gets the default
- [ ] 7.5 Sync the main specs by hand (two scenario titles are renamed, so `openspec archive` would abort), check `git diff openspec/specs` line by line, and archive the change in the PR with `--skip-specs`

## 1. Source research (reverse-engineering inputs)

- [x] 1.1 Map the `network-monitors` contract and its common types: the three-tier storage (`contract_admin`, `authorised_orchestrators`, `authorised_agents`), the execute handlers (`transactions.rs`) and their authorisation checks, the query surface (`queries.rs`), key-shape validation, the socket-keyed `AgentStorageKey` ordering, and the build-info-only migration.
- [x] 1.2 Map the orchestrator (`nym-network-monitor-v3/nym-network-monitor-orchestrator`): startup gating (`orchestrator/mod.rs`), the four-task set and cadences, the node refresher (mixnet-contract bonds plus self-description), queue-less staleness-ordered assignment (`storage/manager.rs`), the two bearer-token HTTP surfaces, the result submitter (destination endpoint, signing, watermark), stale/old-data eviction, and the four-table SQLite model.
- [x] 1.3 Map the agent (`nym-network-monitor-v3/nym-network-monitor-agent`) and the shared `nym-network-monitor-orchestrator-requests` protocol: the one-shot lifecycle and CLI subcommands, bearer-only auth, the two-hop self-loop probe sequence (`agent/tester.rs`), the fixed-rate load test with dedup-by-id, `reuse_header`, the `packets_sent = expected` override, and the result struct.
- [x] 1.4 Map the nym-api ingest (`nym-api/src/nym_nodes/handlers/v3.rs`, `support/http/state/network_monitors.rs`): the six-step staleness/authorisation/replay/signature/watermark/insert flow, `NetworkMonitorsCache` population from the contract, `LastNMSubmissions`, per-entry drop-not-fail, and the `nym_node_stress_testing_result` table dedupe.
- [x] 1.5 Map the payload types (`StressTestBatchSubmission = SignedMessage<StressTestBatchSubmissionContent>`, `StressTestResult`) and confirm the signature covers the JSON body.
- [x] 1.6 Map the nym-node authorisation-propagation path: the one-time `get_all_network_monitor_agents` load, the nyxd websocket event subscription (`nyxd_watcher/network_monitor_agents.rs`), the canonical-IP-keyed routing set and Noise map, and the replay/bloomfilter bypass gate (`mixnet/handler.rs`). Confirm there is no separate "extra initiator IPs" allowlist and no periodic reconciliation.
- [x] 1.7 Trace the downstream consumer surface (stress-test aggregation, the performance provider, the composite performance gating flags, rewarding) and confirm the mixnodes-only restriction across the orchestrator, the created test type, and the nym-api ingest.
- [x] 1.8 Spot-verify the load-bearing surprising claims directly against source (the `packets_sent = expected` override and its in-source rationale; the nym-api six-step order; the contract admin-at-instantiation and shape-only key validation; the node-side event-driven propagation and IP-only bypass gate).

## 2. Author artifacts

- [x] 2.1 Write `proposal.md` (Why / What Changes / Capabilities / Impact).
- [x] 2.2 Write `design.md` (Context / Goals-NonGoals / Decisions 1-13 / Risks-Trade-offs / Future direction (non-normative) / Open Questions).
- [x] 2.3 Write `specs/network-monitors-contract/spec.md` delta (`## ADDED Requirements`): registry/instantiation, orchestrator authorise/revoke + cascade, identity-key self-announcement, agent authorise, agent revoke/revoke-all, revoked-orchestrator loss of authority, query surface + ordering, migration.
- [x] 2.4 Write `specs/nym-network-monitor/spec.md` delta (`## ADDED Requirements`): orchestrator gating, task set, node refresher, assignment, on-chain agent authorisation, node-side propagation, HTTP surfaces, signed submission, eviction, storage; agent lifecycle, bearer auth, probe mechanics, load test, `reuse_header`, back-pressure penalty, result shape; mixnodes-only seam; nym-api ingest, per-entry validation, payload shape; consumer catalog; configuration surface.

## 3. Validate via openspec tooling

- [x] 3.1 Run `openspec validate network-monitor-v3-spec --strict` and confirm it reports valid. **Valid (2026-07-23).**
- [x] 3.2 Run `openspec show network-monitor-v3-spec` and review the rendered output for readability, section ordering, and requirement/scenario counts across both capability deltas. **31 requirements / 68 scenarios (8/19 contract + 23/49 fleet).**

## 4. Reviewer pass (before archiving)

- [x] 4.1 Confirm the `proposal.md` "Why" and the limitation notes (bearer-only agent auth, source-IP-only replay bypass, no periodic node reconciliation, orchestrator restart re-announce, cascade-delete gas ceiling) match operational understanding. **Reviewed 2026-07-23.**
- [x] 4.2 Walk `design.md` Decisions 1-13 and the eight Open Questions; confirm each rationale matches team reasoning, especially Decision 2 (node-side propagation / replay bypass), Decision 7 (back-pressure penalty), and Decision 10 (signed monotonic submission). **Resolved 2026-07-23**: all eight walked through with the maintainer (see design.md "Resolved Questions"). Q1 keep mixnodes-only; Q2 back-pressure penalty intended; Q3 bearer auth accepted; Q4 the v2 locust crate is not in use (spec calls it unused/legacy); Q5 admin is the Nymtech SA multisig. Q6/Q7/Q8 documented as current behaviour AND open follow-on changes (periodic node reconciliation; identity-keyed replay bypass - established during review to need no wire-format change since `XKpsk3` already possession-authenticates the agent static key; persisted replay watermark - backstopped today by the DB primary-key dedupe).
- [x] 4.3 Walk both spec deltas requirement by requirement; for each disagreement decide whether the spec is wrong (edit the spec) or the implementation is wrong (open a follow-on change). In particular ratify or reject the current-state facts: the `packets_sent = expected` penalty, the IP-only bypass gate, and the missing node-side reconciliation. **Reviewed 2026-07-23**: maintainer ratified all current-state facts; the IP-only bypass and missing reconciliation are captured as follow-on changes, not spec edits.

## 5. Archive the change

- [ ] 5.1 Once reviewed and accepted, run `openspec archive network-monitor-v3-spec` to promote both deltas into `openspec/specs/network-monitors-contract/spec.md` and `openspec/specs/nym-network-monitor/spec.md` as the canonical specs.

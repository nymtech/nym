## 1. Source research (reverse-engineering inputs)

- [x] 1.1 Read the full `nym-api/src/network_monitor/` module (`mod.rs`, `monitor/mod.rs`, `preparer.rs`, `sender.rs`, `receiver.rs`, `processor.rs`, `summary_producer.rs`, `gateway_client_handle.rs`, `gateways_reader.rs`, `test_route/mod.rs`, `test_packet.rs`).
- [x] 1.2 Trace the startup/gating path in `nym-api/src/support/cli/run.rs` (`network_monitor.enabled`, `has_performance_data`, `HistoricalUptimeUpdater`).
- [x] 1.3 Extract the configuration surface and defaults from `nym-api/src/support/config/mod.rs` (`NetworkMonitorDebug`).
- [x] 1.4 Trace the persistence entry points in `nym-api/src/support/storage/` (`insert_monitor_run_results`, `insert_monitor_run_report`, `MonitorRunScore`).
- [x] 1.5 Read the shared test primitives in `common/node-tester-utils/` (`TestMessage`, `TestableNode`/`NodeType`, `NodeTester` topology substitution, `TestPacketProcessor`) and `NodeResult` in `common/types/src/monitoring.rs`.
- [x] 1.6 Verify the accumulated route-node `blacklist` is not consumed by `PacketPreparer::prepare_test_routes` (confirmed write-only; recorded as a current-state gap).
- [x] 1.7 Trace every nym-api consumer of the persisted score (performance provider -> node annotations; route-selection feedback loop; historical-uptime updater; `EpochAdvancer` rewarding + rewarded-set `performance^20` weighting; read-only HTTP endpoints) and the provider source-selection gate, plus the two external signed-submission ingestion paths. Verified via grep that `min_mixnode_reliability` / `min_gateway_reliability` / `minimum_interval_monitor_threshold` have no readers (accuracy fix to the config requirement).

## 2. Author artifacts

- [x] 2.1 Write `proposal.md` (Why / What Changes / Capabilities / Impact).
- [x] 2.2 Write `design.md` (Context / Goals-NonGoals / Decisions / Risks / Open Questions).
- [x] 2.3 Write `specs/network-monitor/spec.md` delta (`## ADDED Requirements`, requirements + scenarios covering lifecycle, readiness gate, run loop/nonce isolation, route selection, route verification + unwired blacklist, route composition, per-node packet construction + node isolation + static keys, sending, receiving/matching/acks, connection lifecycle, scoring, reporting, persistence, config surface).
- [x] 2.4 Add the consumer-surface requirement ("The persisted reliability score is consumed by a defined set of nym-api subsystems") + provider source-selection gate; add the "Downstream data flow" catalog (Decision 12) and Open Question 6 to `design.md`; fix the inaccurate `min_*_reliability` statement in the config requirement (they are inert / no readers).

## 3. Validate via openspec tooling

- [x] 3.1 Run `openspec validate network-monitor-spec` and confirm it reports valid.
- [x] 3.2 Run `openspec show network-monitor-spec` and review the rendered output for readability and section ordering (15 requirements / 51 scenarios parsed after the consumer-surface requirement was added).

## 4. Reviewer pass (before archiving)

- [x] 4.1 Confirm the `proposal.md` "Why" and the limitation notes (static keys, ignored acks, unwired blacklist, eager receiver spawn) match operational understanding. **Reviewed 2026-07-23**: maintainer confirmed the documents.
- [x] 4.2 Walk `design.md` Decisions 1-12 and the open questions; confirm each rationale matches team reasoning, especially Decision 4 (blacklist) and Decision 6 (static keys). **Resolved 2026-07-23**: all six open questions walked through with the maintainer and resolved as document/keep (no code changes, no follow-on changes) - see design.md "Resolved Questions". Q1 gained the credentials/ticket-usage rationale (Decision 6); Q5 sharpened to "abandoned/vestigial dead code" (Decision 4 + the spec's route-verification requirement); Q6 kept documented as inert; Q4 kept as a future idea; Q2 is accepted tech-debt; Q3 is moot given Q5.
- [x] 4.3 Walk `specs/network-monitor/spec.md` requirement by requirement; for each disagreement decide whether the spec is wrong (edit the spec) or the implementation is wrong (open a follow-on change). In particular ratify or reject the current-state gap in "Candidate routes are verified all-or-nothing" (unwired blacklist / overlapping routes). **Reviewed 2026-07-23**: maintainer accepted the spec, including the vestigial-blacklist current-state gap. Adversarial bug review surfaced separate candidate issues (readiness-gate math, mixnode-traffic funnel through one route gateway, NaN-on-empty-run) - noted for follow-on changes, not blockers to ratifying current behaviour.

## 5. Archive the change

- [x] 5.1 Once reviewed and accepted, run `openspec archive network-monitor-spec` to promote `specs/network-monitor/spec.md` into `openspec/specs/network-monitor/spec.md` as the canonical spec. **Archived 2026-07-23.**

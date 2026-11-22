## MODIFIED Requirements

### Requirement: The persisted reliability score is consumed by a defined set of nym-api subsystems

The reliability score persisted by a monitor run SHALL form the monitor's output contract for the rest of nym-api. The score MUST be consumable by the subsystems enumerated below; the detailed behaviour of each consumer is specified by its own capability, and this requirement fixes only WHICH nym-api subsystems read the score and FOR WHAT. Whether the monitor's persisted score is the *effective* performance source is gated by the mode: when `use_performance_contract_data` is `false` the `LegacyStoragePerformanceProvider` MUST read the monitor's per-node reliability (averaged over the last 24 hours) and convert it into the routing component of each node's annotation; when it is `true` nym-api computes no figure of its own, so the monitor's persisted reliability MUST NOT be read to compute annotations, and rewarding and rewarded-set selection take each node's figure from the performance contract instead, as `nym-network-monitor` specifies (the monitor-run tables remain directly readable by the status/uptime HTTP endpoints regardless).

The consumers are:

- **Node annotations.** The averaged reliability MUST feed `NodeAnnotationV2.detailed_performance.performance_score` via the node-status cache refresher (legacy mode only).
- **Route-selection feedback loop.** The monitor's own `PacketPreparer` MUST read that `performance_score` back as the candidate-selection weight (see the performance-weighted sampling requirement), so a node's past score biases its future testing. In contract mode there are no annotations, so every candidate takes the preparer's default weight and the loop has no effect.
- **Historical uptime.** The `HistoricalUptimeUpdater` MUST aggregate the last 24 hours of per-run statuses into per-day historical-uptime rows, and MUST run only when the network monitor is enabled.
- **Rewarding and rewarded-set selection.** In legacy mode the `EpochAdvancer` MUST derive each node's rewarding performance from its annotation and use it both as the reward-amount performance in each `RewardNode` message and as the rewarded-set selection weight (`saturation * performance^20`), where a performance of 0 yields a selection weight of 0 (de-facto exclusion). This path MUST run only when rewarding is enabled and `has_performance_data` is `true`.
- **Read-only HTTP endpoints.** nym-api MUST expose monitor-derived data over HTTP, including per-node uptime history and core-status counts, per-node detailed test results, per-run monitor reports (specific run and latest run), and per-node annotation / performance / uptime-history endpoints; node listings additionally embed each node's recent performance.

#### Scenario: Legacy provider feeds node annotations from the persisted reliability
- **WHEN** `use_performance_contract_data` is `false` and a node has a persisted 24-hour reliability
- **THEN** that reliability becomes the routing component of the node's `performance_score` in its `NodeAnnotationV2`

#### Scenario: Contract source disables monitor-derived annotations
- **WHEN** `use_performance_contract_data` is `true`
- **THEN** the monitor's persisted reliability is not read to compute any annotation, and rewarding and rewarded-set selection take each node's figure from the performance contract

#### Scenario: Score feeds the route-selection feedback loop
- **WHEN** the monitor prepares candidate routes on a later run
- **THEN** it weights node selection by the `performance_score` derived from earlier persisted scores

#### Scenario: Historical uptime aggregates persisted statuses
- **WHEN** the network monitor is enabled and a day's per-run statuses exist
- **THEN** the `HistoricalUptimeUpdater` writes per-day historical-uptime rows aggregated from them

#### Scenario: Performance drives reward amount and rewarded-set membership
- **WHEN** rewarding is enabled, `has_performance_data` is `true` and `use_performance_contract_data` is `false`
- **THEN** each node's performance is used both as its reward-amount performance and as its rewarded-set selection weight, and a node whose performance is 0 has selection weight 0

#### Scenario: Monitor-derived data is exposed over HTTP
- **WHEN** a client queries the relevant status or nym-node endpoints
- **THEN** monitor-derived reliability, uptime history, and monitor-run reports are returned

# network-monitor Specification

## Purpose
TBD - created by archiving change network-monitor-spec. Update Purpose after archive.
## Requirements
### Requirement: The network monitor runs only when explicitly enabled and spawns three cooperating tasks

The network monitor SHALL start only when `config.network_monitor.enabled` is `true`. When enabled, `network_monitor::start::<SphinxMessageReceiver>` MUST build the subsystem and spawn exactly three long-lived tokio tasks under the process `ShutdownManager`: the `BandwidthController` (`run`), the `PacketReceiver` (`run`), and the `Monitor` (`run`). When enabled, the `HistoricalUptimeUpdater` MUST also be started so that persisted scores are rolled into uptime history. When disabled, none of these tasks run and the monitor produces no scores.

Reward eligibility is gated by `has_performance_data = config.network_monitor.enabled || config.performance_provider.use_performance_contract_data`; the network monitor is one of two possible performance-data sources and its being disabled does not by itself disable rewarding when the performance contract is in use.

The `ReceivedProcessor` builds its receive future at construction; `start_receiving` MUST `take()` that future and spawn it exactly once. Calling `start_receiving` a second time MUST panic. This eager-construction lifecycle is a known caveat flagged in-source for future refactor.

#### Scenario: Monitoring enabled spawns the full task set
- **WHEN** nym-api starts with `config.network_monitor.enabled = true`
- **THEN** the `BandwidthController`, `PacketReceiver`, and `Monitor` tasks are spawned under the shutdown manager
- **AND** the `HistoricalUptimeUpdater` is also started
- **AND** `has_performance_data` is `true`

#### Scenario: Monitoring disabled runs no monitor tasks
- **WHEN** nym-api starts with `config.network_monitor.enabled = false`
- **THEN** no monitor, receiver, or bandwidth-controller task for the network monitor is spawned and no monitor runs are produced
- **AND** `has_performance_data` is still `true` if `performance_provider.use_performance_contract_data` is set

#### Scenario: Receiver task is armed exactly once
- **WHEN** `Monitor::run` calls `start_receiving`
- **THEN** the receive task is spawned and begins consuming forwarded gateway messages
- **AND** a second call to `start_receiving` panics

### Requirement: The monitor waits for caches and a minimal topology before the first run

Before entering the run loop, the monitor SHALL block until the mixnet contract cache and the self-described-nodes cache report initial values, and then until the described topology contains at least `minimum_test_routes` entry-capable gateways and enough mixing nodes that `mixnodes_count * 3 >= minimum_test_routes`. Node roles MUST be determined from each node's self-described `declared_role` (`mixnode` for mixing capability, `entry` for gateway capability). While the minimal topology is not yet online the monitor MUST re-check after a fixed 30-second backoff and MUST NOT start a test run.

#### Scenario: Caches not yet initialised blocks the first run
- **WHEN** the mixnet contract cache or described-nodes cache has not produced initial values
- **THEN** the monitor waits and does not begin any test run

#### Scenario: Insufficient topology retries after backoff
- **WHEN** the described topology has fewer than `minimum_test_routes` gateways or insufficient mixing nodes
- **THEN** the monitor logs that the minimal topology is not online and re-checks after 30 seconds

#### Scenario: Minimal topology satisfied starts the loop
- **WHEN** at least `minimum_test_routes` gateways and `mixnodes_count * 3 >= minimum_test_routes` mixing nodes are online
- **THEN** the monitor proceeds to its periodic run loop

### Requirement: Test runs execute on a fixed interval and are isolated by a monotonic per-run nonce

The monitor SHALL run `test_run` on a `tokio` interval of `run_interval`. The run loop MUST use a biased `select!` so that a cancelled shutdown token takes priority over both a new interval tick and an in-flight test run. Each `test_run` MUST be identified by `test_nonce`, which starts at 1 and MUST be incremented by exactly 1 after every `test_run` regardless of whether the run succeeded or aborted. Route-verification packets MUST use the reserved nonce `ROUTE_TESTING_TEST_NONCE = 0`, which is distinct from any main-run nonce.

#### Scenario: Interval tick triggers one test run
- **WHEN** a `run_interval` tick fires and no shutdown is pending
- **THEN** exactly one `test_run` executes and afterwards `test_nonce` is incremented by 1

#### Scenario: Shutdown preempts the loop and any in-flight run
- **WHEN** the shutdown token is cancelled
- **THEN** the run loop breaks, preempting a running `test_run` if one is in progress

#### Scenario: Distinct runs use distinct nonces
- **WHEN** two consecutive main test runs execute
- **THEN** they stamp their packets with consecutive, distinct `test_nonce` values, and route verification within each run uses the reserved nonce `0`

### Requirement: Candidate test routes are selected by performance-weighted random sampling across layers

`PacketPreparer::prepare_test_routes(n)` SHALL draw candidate routes from the described-nodes cache using performance weights from the node-status cache. Each candidate node's selection weight MUST be its `detailed_performance.performance_score`, defaulting to `0.5` when the node has no annotation. Mixing nodes MUST be assigned to the three legacy layers by random assignment; if any of the three layers ends up empty, the preparer MUST fall back to `naive_rearrange`, which round-robins all mixing nodes across the three layers. The preparer MUST select up to `n` nodes from each layer and up to `n` gateways via `choose_multiple_weighted`, then zip them index-wise into `TestRoute`s, producing as many routes as the smallest per-layer/gateway count allows. If any layer is missing or no nodes/gateways are available, it MUST return `None`.

#### Scenario: Selection favours higher-performing nodes
- **WHEN** candidate routes are prepared and nodes have differing performance scores
- **THEN** higher-scored nodes are more likely to be chosen, and a node lacking an annotation is weighted `0.5`

#### Scenario: Empty layer triggers naive round-robin fallback
- **WHEN** random layer assignment leaves one of the three layers empty (for example on a small testnet)
- **THEN** the preparer round-robins all mixing nodes across the three layers instead

#### Scenario: No available nodes yields no candidates
- **WHEN** a layer has no parseable nodes or there are no gateways
- **THEN** `prepare_test_routes` returns `None`

### Requirement: Candidate routes are verified all-or-nothing before use; a run aborts if too few verify

Before scoring the network, the monitor SHALL verify candidate routes by sending `route_test_packets` self-loop packets (under the reserved nonce `0`) through each candidate, waiting `packet_delivery_timeout`, dropping the gateway clients, and draining the received packets. A candidate route MUST be treated as "working" if and only if the number of its packets received equals `route_test_packets` exactly; a route that forwarded fewer (including zero) MUST be rejected. The monitor MUST repeatedly build candidate batches of `remaining * 2` routes and verify them, accumulating working routes until it has `test_routes`, giving up after `test_routes * 10` attempts (or when no further candidates can be generated). If at least `minimum_test_routes` working routes were found it MUST proceed; otherwise it MUST abort the run with an error and persist nothing.

Nodes on a confirmed-working route MUST be inserted into an in-memory blacklist. In the current implementation this blacklist is **vestigial / abandoned dead code**: it is built and populated but never consumed - it is never passed to `PacketPreparer::prepare_test_routes`, so it has zero runtime effect, candidate selection does not exclude previously-used nodes, and verified routes MAY overlap on shared nodes. This is documented as abandoned behaviour, not an active feature.

#### Scenario: Route delivering all its packets is accepted
- **WHEN** a candidate route returns exactly `route_test_packets` packets within `packet_delivery_timeout`
- **THEN** the route is marked working and used for network testing

#### Scenario: Route delivering fewer packets is rejected
- **WHEN** a candidate route returns fewer than `route_test_packets` packets (including zero)
- **THEN** the route is rejected and not used

#### Scenario: Insufficient working routes aborts the run
- **WHEN** after `test_routes * 10` attempts fewer than `minimum_test_routes` routes are working
- **THEN** the run is aborted with an error and no monitor run, statuses, or report are persisted

#### Scenario: Vestigial blacklist has no effect on selection
- **WHEN** multiple working routes are verified in one run
- **THEN** their nodes are recorded in the blacklist, but because the blacklist is never consumed (abandoned dead code) it has no effect and the verified routes may still share nodes

### Requirement: A test route is a self-contained 3-mix + 1-gateway topology

A `TestRoute` SHALL wrap a `NymTopology` containing exactly one layer-1 mixnode, one layer-2 mixnode, one layer-3 mixnode, and one entry gateway, plus a fake single-entry `EpochRewardedSet` (with `epoch_id = EpochId::MAX`) that assigns each node to its role. The route MUST carry a random `u64` id and the current key-rotation id from the contract cache. The gateway's client-facing websocket address MUST be derived from the gateway's self-described entry address (`ws_entry_address(false)`), and the gateway identity MUST be the gateway's ed25519 key.

#### Scenario: Route exposes one node per role plus a gateway
- **WHEN** a `TestRoute` is constructed from three mixnodes and one gateway
- **THEN** it exposes exactly one node for each of layer 1, layer 2, layer 3, and the entry gateway role, and a random route id

#### Scenario: Gateway client address is derived from the gateway description
- **WHEN** the route's gateway client address is requested
- **THEN** it returns the gateway's self-described websocket entry address

### Requirement: Per-node test packets isolate one node by topology substitution and loop back to the monitor

For each main run, `PacketPreparer::prepare_test_packets` SHALL build `per_node_test_packets` sphinx packets for every parseable mixnode and every parseable gateway, per verified route. To isolate the node under test, the packet's route topology MUST substitute that node into the relevant role: a mixnode is placed into its randomly assigned layer (`testable_mix_topology`), and a gateway is placed into both the entry and exit gateway roles (`testable_gateway_topology`). Every packet's recipient MUST be the monitor itself, reached through the route's entry gateway for mixnode tests and through the tested gateway for gateway tests (`create_packet_sender`). Packets MUST be grouped by the entry gateway's identity into `GatewayPackets`. Each test message MUST be JSON-serialised and MUST fit within a single `RegularPacket`; a message that does not fit MUST fail with `TestMessageTooLong`.

The monitor SHALL use a single static key set (one ed25519 identity keypair, one x25519 encryption keypair, one ack key) generated at startup as the sender/recipient identity for every gateway and every run. This reuse of well-known keys is a documented security limitation (a malicious gateway can fingerprint monitor traffic).

#### Scenario: Mixnode packet traverses the route with the tested mix substituted
- **WHEN** a test packet is built for a mixnode under test on a given verified route
- **THEN** the packet path enters and exits through the route's gateway and passes through the tested mixnode in its assigned layer alongside the route's other two mixnodes, returning to the monitor

#### Scenario: Gateway packet ingresses and egresses through the tested gateway
- **WHEN** a test packet is built for a gateway under test
- **THEN** the packet enters and leaves the mixnet through the tested gateway, with the route's three mixnodes in between, returning to the monitor

#### Scenario: Packets are grouped by entry gateway
- **WHEN** packets for many nodes across several routes are prepared
- **THEN** they are bucketed into one `GatewayPackets` per entry gateway identity

#### Scenario: Static monitor keys are reused across gateways and runs
- **WHEN** the monitor builds packets for different gateways or across successive runs
- **THEN** it uses the same static identity, encryption, and ack keys (a documented fingerprinting limitation)

### Requirement: Packets are delivered to gateways via authenticated, rate-limited gateway clients

`PacketSender::send_packets` SHALL, for each `GatewayPackets`, create a `GatewayClient`, authenticate (bounded by `gateway_connection_timeout`), claim initial bandwidth (bounded by `gateway_bandwidth_claim_timeout`), and start listening for mixnet messages. On successful startup the shared symmetric key MUST be cached (`gateways_key_cache`) for reuse on later runs; on any startup failure the cached key for that gateway MUST be removed and that gateway MUST be dropped from the run. Before sending, the client's remaining bandwidth MUST be above its configured threshold. Sending MUST be rate-limited to `gateway_sending_rate`: if the packet count is at or below the rate the packets MAY be sent as a single batch, otherwise they MUST be sent in 50-millisecond chunks sized to the rate; the overall send MUST be bounded by roughly three times the estimated send time. The number of gateway clients worked concurrently MUST be bounded by `max_concurrent_gateway_clients` (where `0` means unlimited). On a successful send the receiver MUST be notified of the new connection so returning packets can be read.

#### Scenario: Authenticated gateway receives its packets within the rate limit
- **WHEN** a gateway authenticates, claims bandwidth, and has sufficient remaining bandwidth
- **THEN** its packets are sent (batched or rate-limited into 50 ms chunks) and the receiver is told the connection is live

#### Scenario: A gateway that fails startup or sending is dropped
- **WHEN** authentication, bandwidth claim, or sending to a gateway times out or errors
- **THEN** that gateway is dropped from the run, its cached key removed, and every node whose packets routed through it loses those packets for the run

#### Scenario: Concurrency is bounded
- **WHEN** there are more gateways than `max_concurrent_gateway_clients` and the limit is non-zero
- **THEN** at most that many gateway clients are worked at once, the rest queued

#### Scenario: Shared key is cached and reused
- **WHEN** a gateway startup succeeds
- **THEN** its shared symmetric key is cached and reused to skip re-authentication on the next run

### Requirement: Returned packets are collected by a single receiver task and filtered by the armed nonce

The `PacketReceiver` SHALL own a `GatewaysReader` that multiplexes every live gateway's mixnet-message and acknowledgement streams (keyed by gateway identity), adding and removing per-gateway receivers as `GatewayClientUpdate::New` / `Disconnect` events arrive, and forward received `GatewayMessages` to the `ReceivedProcessor`. The `ReceivedProcessor` MUST decrypt each data packet and: reject it as "received outside a test run" if the armed nonce is the idle sentinel `u64::MAX`; reject it as a non-matching nonce if the packet's embedded `test_nonce` differs from the armed nonce; otherwise collect it. A collected packet's node attribution MUST come from the `tested_node` field embedded in the decrypted `TestMessage`. `return_received` MUST atomically drain the collected packets and reset the armed nonce to `u64::MAX`. Acknowledgements MUST be decoded but MUST NOT affect scoring (they are logged and dropped).

#### Scenario: Packet with the armed nonce is collected
- **WHEN** a decrypted packet's `test_nonce` equals the currently armed nonce
- **THEN** it is added to the received set and attributed to its embedded `tested_node`

#### Scenario: Packet with a mismatched nonce is dropped
- **WHEN** a decrypted packet's `test_nonce` differs from the armed nonce (for example a stale packet from a previous run)
- **THEN** it is rejected and not counted

#### Scenario: Packet received while idle is dropped
- **WHEN** a data packet arrives while the armed nonce is `u64::MAX`
- **THEN** it is rejected as received outside a test run

#### Scenario: Acknowledgements are ignored
- **WHEN** a returning acknowledgement is decoded
- **THEN** it is logged and discarded without affecting any score

#### Scenario: Draining resets to the idle state
- **WHEN** `return_received` is called at the end of a run
- **THEN** it returns all collected packets and resets the armed nonce to `u64::MAX`

### Requirement: Gateway connections are kept alive across the delivery wait and disconnected on drop

The monitor SHALL retain the live `GatewayClientHandle`s returned by the send step across the `packet_delivery_timeout` sleep, so that returning packets can still be received, and MUST drop them only after the wait. Dropping a `GatewayClientHandle` MUST send a `GatewayClientUpdate::Disconnect` so the `PacketReceiver` removes that gateway's receivers.

#### Scenario: Handles outlive the send so returns are captured
- **WHEN** all packets have been sent and the monitor is waiting `packet_delivery_timeout`
- **THEN** the gateway client handles remain alive and returning packets continue to be received

#### Scenario: Dropping a handle disconnects the gateway
- **WHEN** the delivery wait ends and the handles are dropped
- **THEN** each drop emits a disconnect update and the receiver stops reading that gateway

### Requirement: Node reliability is the delivery ratio over verified routes

`SummaryProducer::produce_summary` SHALL compute, for each node under test, `reliability = round(received / (verified_route_count * per_node_test_packets) * 100)` as a `u8`, where `verified_route_count` is the number of routes actually verified for the run. The result set MUST be seeded with every tested mixnode and gateway at zero received, so an unreachable node scores 0. Each result MUST be a `NodeResult { node_id, identity (base58), reliability }` and MUST be bucketed as a mixnode or gateway result according to the node's `NodeType`. Per-route performance MUST be computed as `received_for_route / ((tested_mixnode_count + tested_gateway_count) * per_node_test_packets) * 100`. The overall `network_reliability` MUST be `total_received / total_sent * 100`.

#### Scenario: Node returning all its packets scores 100
- **WHEN** a node returns `verified_route_count * per_node_test_packets` packets
- **THEN** its reliability is 100

#### Scenario: Node returning nothing scores 0
- **WHEN** a tested node returns no packets
- **THEN** it is still present in the results with reliability 0

#### Scenario: Denominator uses the achieved route count
- **WHEN** a run verified fewer than `test_routes` routes (down to `minimum_test_routes`)
- **THEN** the reliability denominator uses the achieved verified-route count, so a node reachable on every verified route still scores 100

#### Scenario: Results are split by node type
- **WHEN** summaries are produced
- **THEN** mixnode results and gateway results are returned in separate collections

### Requirement: Each run produces a human-readable report bucketed by reliability threshold

Each successful run SHALL produce a `TestReport` carrying `network_reliability`, `total_sent`, `total_received`, and score-to-count histograms for mixnodes and gateways. The monitor MUST log a `DisplayTestReport` that buckets nodes into exceptional (`>= 95`), fine (`80`-`95`), poor (`60`-`80`), unreliable (`1`-`60`), and unroutable (`< 1`), for mixnodes and gateways separately, and lists the per-route reliabilities. The report is always logged.

#### Scenario: Report is logged for every completed run
- **WHEN** a run completes scoring
- **THEN** a `DisplayTestReport` with overall reliability, per-route reliabilities, and per-bucket counts is logged

#### Scenario: Nodes are bucketed by reliability threshold
- **WHEN** the display report is rendered
- **THEN** each node is counted in exactly one of the exceptional / fine / poor / unreliable / unroutable buckets based on its reliability against the thresholds 95 / 80 / 60 / 1

### Requirement: Each completed run is persisted as a monitor-run record with per-node statuses and score histograms

On a completed run the monitor SHALL persist results to `NymApiStorage` in two steps. `insert_monitor_run_results` MUST create a monitor-run row stamped with the current unix timestamp, submit the mixnode statuses and gateway statuses, and record each test route used. `insert_monitor_run_report` MUST store the run's `network_reliability`, `total_sent`, and `total_received`, plus one `MonitorRunScore` row per distinct rounded score for each node class (`typ` of `"mixnode"` or `"gateway"`, with the count of nodes at that score). Persistence errors MUST be logged and MUST NOT crash the monitor; the run loop continues. A run that aborted for insufficient routes MUST persist nothing.

#### Scenario: Completed run persists the full record
- **WHEN** a run finishes scoring
- **THEN** a monitor-run row, per-node mixnode and gateway statuses, the test routes used, the run report, and the per-score `MonitorRunScore` rows are all persisted

#### Scenario: Persistence failure is non-fatal
- **WHEN** a database write fails during persistence
- **THEN** the error is logged and the monitor continues to its next run

#### Scenario: Aborted run persists nothing
- **WHEN** a run aborts because fewer than `minimum_test_routes` routes verified
- **THEN** no monitor-run row, statuses, routes, report, or scores are written

### Requirement: The persisted reliability score is consumed by a defined set of nym-api subsystems

The reliability score persisted by a monitor run SHALL form the monitor's output contract for the rest of nym-api. The score MUST be consumable by the subsystems enumerated below; the detailed behaviour of each consumer is specified by its own capability, and this requirement fixes only WHICH nym-api subsystems read the score and FOR WHAT. Whether the monitor's persisted score is the *effective* performance source is gated by provider selection: when `use_performance_contract_data` is `false` the `LegacyStoragePerformanceProvider` MUST read the monitor's per-node reliability (averaged over the last 24 hours) and convert it into the routing component of each node's annotation; when it is `true` the performance-contract cache MUST be the source instead and the monitor's persisted reliability MUST NOT be read to compute annotations (the monitor-run tables remain directly readable by the status/uptime HTTP endpoints regardless).

The consumers are:

- **Node annotations.** The averaged reliability MUST feed `NodeAnnotationV2.detailed_performance.performance_score` via the node-status cache refresher (legacy-provider path only).
- **Route-selection feedback loop.** The monitor's own `PacketPreparer` MUST read that `performance_score` back as the candidate-selection weight (see the performance-weighted sampling requirement), so a node's past score biases its future testing.
- **Historical uptime.** The `HistoricalUptimeUpdater` MUST aggregate the last 24 hours of per-run statuses into per-day historical-uptime rows, and MUST run only when the network monitor is enabled.
- **Rewarding and rewarded-set selection.** The `EpochAdvancer` MUST derive each node's rewarding performance from its annotation and use it both as the reward-amount performance in each `RewardNode` message and as the rewarded-set selection weight (`saturation * performance^20`), where a performance of 0 yields a selection weight of 0 (de-facto exclusion). This path MUST run only when rewarding is enabled and `has_performance_data` is `true`.
- **Read-only HTTP endpoints.** nym-api MUST expose monitor-derived data over HTTP, including per-node uptime history and core-status counts, per-node detailed test results, per-run monitor reports (specific run and latest run), and per-node annotation / performance / uptime-history endpoints; node listings additionally embed each node's recent performance.

#### Scenario: Legacy provider feeds node annotations from the persisted reliability
- **WHEN** `use_performance_contract_data` is `false` and a node has a persisted 24-hour reliability
- **THEN** that reliability becomes the routing component of the node's `performance_score` in its `NodeAnnotationV2`

#### Scenario: Contract source disables monitor-derived annotations
- **WHEN** `use_performance_contract_data` is `true`
- **THEN** node annotations are derived from the performance-contract cache and the monitor's persisted reliability is not read to compute them

#### Scenario: Score feeds the route-selection feedback loop
- **WHEN** the monitor prepares candidate routes on a later run
- **THEN** it weights node selection by the `performance_score` derived from earlier persisted scores

#### Scenario: Historical uptime aggregates persisted statuses
- **WHEN** the network monitor is enabled and a day's per-run statuses exist
- **THEN** the `HistoricalUptimeUpdater` writes per-day historical-uptime rows aggregated from them

#### Scenario: Performance drives reward amount and rewarded-set membership
- **WHEN** rewarding is enabled and `has_performance_data` is `true`
- **THEN** each node's performance is used both as its reward-amount performance and as its rewarded-set selection weight, and a node whose performance is 0 has selection weight 0

#### Scenario: Monitor-derived data is exposed over HTTP
- **WHEN** a client queries the relevant status or nym-node endpoints
- **THEN** monitor-derived reliability, uptime history, and monitor-run reports are returned

### Requirement: The monitor's behaviour is governed by a configuration surface with defined defaults

The monitor's behaviour SHALL be controlled by `NetworkMonitorDebug` with the following defaults: `run_interval` = 15 minutes, `packet_delivery_timeout` = 20 seconds, `test_routes` = 3, `minimum_test_routes` = 1, `route_test_packets` = 1000, `per_node_test_packets` = 3, `gateway_sending_rate` = 200 packets/second, `max_concurrent_gateway_clients` = 50, `gateway_response_timeout` = 5 minutes, `gateway_connection_timeout` = 15 seconds, `gateway_bandwidth_claim_timeout` = 2 minutes, and `disabled_credentials_mode` = `true`. The top-level `enabled` flag MUST default to `false`. `disabled_credentials_mode` being `true` means the monitor claims gateway bandwidth without presenting credentials; it is the inverse of the operator-facing `monitor_credentials_mode` flag. The `min_mixnode_reliability` (default 50), `min_gateway_reliability` (default 20), and the rewarding-side `minimum_interval_monitor_threshold` (default 60) also live on the configuration surface; despite an in-source comment describing reliability-based blacklisting and rewarded-set exclusion, these thresholds currently have NO readers anywhere in nym-api and MUST NOT be assumed to gate probing, blacklisting, or selection. De-facto exclusion of low-performing nodes arises only from the rewarded-set selection weight (`saturation * performance^20`), where a performance of 0 produces a selection weight of 0.

#### Scenario: Defaults match the documented values
- **WHEN** a nym-api config is created without overriding monitor debug values
- **THEN** the effective values are those listed above and `enabled` is `false`

#### Scenario: Credentials mode defaults to disabled
- **WHEN** the operator does not set `monitor_credentials_mode`
- **THEN** `disabled_credentials_mode` is `true` and the monitor claims bandwidth without presenting credentials

#### Scenario: Reliability-threshold knobs are currently inert
- **WHEN** `min_mixnode_reliability`, `min_gateway_reliability`, or `minimum_interval_monitor_threshold` is set to any value
- **THEN** no probing, blacklisting, or rewarded-set decision changes, because no code path reads these fields


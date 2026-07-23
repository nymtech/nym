## Context

The network monitor is the active-probing half of nym-api's performance-data pipeline. It lives in `nym-api/src/network_monitor/` and runs as three long-lived tokio tasks (a `BandwidthController`, a `PacketReceiver`, and the `Monitor` loop) spawned from `network_monitor::start` when `config.network_monitor.enabled` is true (`run.rs:359`). On a fixed interval (default 15 minutes) it constructs a handful of verified test routes through the live mixnet, sends a small number of known sphinx packets aimed at every mixnode and gateway such that they loop back to itself, waits a fixed delivery timeout, and scores each node by the fraction of its packets that returned. The scores are written to `NymApiStorage` as a "monitor run" and consumed downstream by historical-uptime tracking and reward eligibility.

The subsystem integrates with several nym-api caches (the mixnet contract cache for the rewarded set and key-rotation id, the self-described-nodes cache for node roles and connection details, the node-status cache for performance weights), with `nym-gateway-client` for the actual mixnet ingress/egress, with the `nym-bandwidth-controller` for claiming gateway bandwidth, and with the shared `common/node-tester-utils` crate for constructing and recovering test packets. It has shipped, is in active use, and is documented only as source plus inline comments. This document captures the architectural choices behind the implementation as it stands today; no behaviour change is proposed.

## Goals / Non-Goals

**Goals:**
- Capture what a reliability score actually measures: a fraction of self-addressed loop packets returned within a fixed timeout, over a small, redundantly-verified set of routes, isolating one node per measurement via topology substitution.
- Document the run-isolation model (monotonic per-run nonce, reserved route-testing nonce, idle sentinel) that makes concurrent/stale packets safe to ignore.
- Document the route lifecycle: performance-weighted candidate selection, all-or-nothing verification, and node blacklisting to prevent route overlap.
- Document the gateway-client send lifecycle (authenticate, claim bandwidth, rate-limited batching, RAII disconnect-on-drop) and the receive/match lifecycle.
- Document the scoring and persistence surface (`NodeResult`, `TestReport`, monitor-run rows and score histograms) that downstream consumers depend on.
- Record the configuration knobs and their defaults, since they directly change what a score means.
- Record the known limitations as deliberate current-state facts, not latent bugs.

**Non-Goals:**
- The *internal behaviour* of the downstream consumers of the scores (historical-uptime aggregation, `EpochAdvancer` reward maths, the node-status cache refresher, the HTTP handlers, the performance contract). These are separate capabilities. This document does, however, catalog the *consumer surface* - which nym-api subsystems read the score and for what - in "Decision 12: Downstream data flow", because that surface is what the user needs to reason about a score's blast radius.
- The `nym-gateway-client`, `nym-bandwidth-controller`, and sphinx/`node-tester-utils` internals beyond the surface the monitor relies on.
- The self-describe / mixnet-contract / node-status cache internals that supply the monitor's inputs.
- Any redesign of the flagged limitations (ephemeral keys, receiver-task lifecycle, ack handling). These are acknowledged as future work; the spec captures the current surface.

## Decisions

### Decision 1: Active loop-back probing with self-addressed sphinx packets

**Choice.** Reliability is measured by injecting real sphinx packets into the live mixnet whose final recipient is the monitor itself (via the route's entry gateway). A returned packet proves the entire path forwarded correctly; a missing packet after a fixed timeout is counted as a loss.

**Why.** End-to-end delivery through the real network is the only signal that captures the property that matters for routing (can traffic actually traverse this node right now), as opposed to passive metrics like a node being bonded or self-reporting healthy. Self-addressing avoids needing any cooperating receiver other than the monitor.

**Alternative considered.** Passive telemetry / self-reported health. Rejected because it does not prove forwarding and is trivially gameable.

**Consequence.** A score is a delivery-ratio over a fixed window; it conflates the node under test with the rest of its route and with transient network conditions. Redundancy across multiple routes (Decision 5) and route pre-verification (Decision 4) exist to reduce that conflation.

### Decision 2: Run isolation is nonce-based, not connection-based

**Choice.** Each test run stamps every packet's payload (`NymApiTestMessageExt { route_id, test_nonce }`) with a monotonically increasing `test_nonce` (starting at 1, incremented per run in `Monitor::test_run`). The shared `ReceivedProcessor` holds a single `AtomicU64` armed nonce; on receipt it decrypts the packet and rejects it unless its embedded `test_nonce` equals the armed value. Two reserved values carry special meaning: `ROUTE_TESTING_TEST_NONCE = 0` is used for route-verification packets, and `u64::MAX` is the "no run in progress" sentinel (any data packet received while armed to `u64::MAX` is dropped as `ReceivedOutsideTestRun`). `return_received` atomically drains the collected packets and resets the armed nonce back to `u64::MAX`.

**Why.** Gateway connections are kept open across the delivery-wait window and packets arrive asynchronously; a stale packet from the previous run (or a route-verification packet arriving during the main run, or vice versa) must not be counted. A payload-embedded nonce lets a single always-on receiver task filter correctly without tearing down and rebuilding connection state between runs.

**Alternative considered.** Per-run receiver instances or per-connection demultiplexing. Rejected as heavier and still racy against in-flight packets.

**Consequence.** Correctness depends on the armed nonce being set (`set_new_test_nonce` / `set_route_test_nonce`) before packets are sent and drained (`return_received`) after the wait. Packets that arrive after the drain are dropped. The nonce is a `u64`, so wraparound is not a practical concern.

### Decision 3: A single node is isolated by substituting it into a pre-verified route

**Choice.** Every test packet is built on top of a verified `TestRoute` (3 mixnodes, one per layer, plus one entry gateway). To test a specific node, `node-tester-utils` clones the route topology and replaces the occupant of the node's role: `testable_mix_topology(layer, node)` swaps the tested mixnode into its (randomly assigned) layer, and `testable_gateway_topology(node)` swaps the tested gateway into both the entry and exit gateway roles. Mixnode packets therefore traverse `route-gateway -> (two route mixes + tested mix) -> route-gateway -> self`; gateway packets ingress and egress through the tested gateway with the route's three mixes in between.

**Why.** A packet must traverse a known-good path so that a loss can be attributed to the one substituted node rather than to an unknown broken hop. Pre-verifying the route (Decision 4) and then substituting one node at a time gives that attribution.

**Alternative considered.** Random full-path selection per node. Rejected because a loss could then be caused by any hop, destroying attribution.

**Consequence.** The score for a node is only as trustworthy as the surrounding route. A node that is fine but happens to be tested only against a marginal route can score low; multi-route redundancy mitigates this. Mixnodes are assigned a random layer per run for both selection and testing, so a node is exercised in whatever layer it lands in.

### Decision 4: All-or-nothing route verification, with a vestigial (abandoned) node blacklist

**Choice.** Before a route is used, the monitor sends `route_test_packets` (default 1000) self-loop packets through it under the reserved route nonce and marks it "working" only if *all* of them return. It aims for `test_routes` (default 3) working routes, retrying candidate batches (each `remaining * 2` candidates) up to `test_routes * 10` attempts, and settles for at least `minimum_test_routes` (default 1) if it cannot reach the target. Nodes belonging to a confirmed-working route are inserted into a `blacklist` set via `blacklist_route_nodes`.

**Why.** Using a route to judge other nodes is only sound if the route itself is reliable; a strict threshold keeps marginal routes out. The blacklist was intended to spread the test routes across distinct nodes so the network is probed through diverse paths.

**Alternative considered.** A softer threshold (for example 95% of route packets). Rejected for the first cut in favour of a simple, conservative "fully working" bar; an outlier-removal refinement is sketched but not implemented (see Resolved Questions, Q4).

**Consequence.** On a lossy network the monitor may burn attempts discarding nearly-good routes and fall back to `minimum_test_routes`, making scores hinge on very few paths. If fewer than `minimum_test_routes` can be built, the run is aborted with an error and no scores are written. IMPORTANT - the `blacklist` is **abandoned dead code**, not merely unwired: `Monitor::prepare_test_routes` builds it and populates it (via `blacklist_route_nodes`) on every working route, but nothing ever reads it. It is never passed to `PacketPreparer::prepare_test_routes(n)`, whose signature takes only `n` and applies no exclusion; the local set is populated and then dropped when the function returns, with zero runtime effect. (The preparer's doc comment describing blacklist-honoring selection is stale/aspirational, and the in-source asymmetry note - that a working gateway paired with a broken mixnode should not be blacklisted, to avoid discarding scarce good gateways under the gateway-to-mixnode imbalance - is doubly moot since the set is unused.) Consequently candidate selection does NOT exclude nodes from earlier working routes and verified routes CAN overlap on shared nodes. This is a half-built "spread routes across distinct nodes" feature that was never finished; it is documented here as abandoned/vestigial (resolution recorded in Resolved Questions).

### Decision 5: Reliability = received / (verified-route-count * packets-per-node)

**Choice.** For each node under test, `SummaryProducer::produce_summary` computes `reliability = round(received / (test_routes.len() * per_node_test_packets) * 100)` as a `u8`, where `test_routes.len()` is the number of routes actually verified this run and `per_node_test_packets` defaults to 3. The result set is seeded with every tested node at zero, so a node that returns nothing scores 0. Per-route performance is computed separately as `received-for-route / ((mix+gateway count) * per_node_test_packets) * 100`. Overall `network_reliability` is `total_received / total_sent * 100`.

**Why.** Each node is probed once per verified route, so the expected count scales with the route count; dividing by the actual verified-route count keeps the score a true 0-100 delivery ratio regardless of how many routes were built. Seeding at zero ensures unreachable nodes are recorded rather than omitted.

**Alternative considered.** Dividing by the target `test_routes` rather than the achieved count. Rejected because a run that only reached `minimum_test_routes` would then cap every node's score below 100.

**Consequence.** With `minimum_test_routes = 1` a node's score can rest on a single route/gateway path and be noisy. The score has no latency or partial-credit dimension - it is purely delivery ratio within the timeout window.

### Decision 6: One static monitor key set, reused for all gateways and all runs

**Choice.** At build time the monitor generates a single ed25519 identity keypair, x25519 encryption keypair, and ack key, and uses them as the sender/recipient identity for every gateway connection and every run.

**Why.** Beyond the recipient identity needing to be stable for loop-back addressing to resolve back to the monitor, this was a deliberate cost optimisation from when the tester was assumed to require bandwidth **credentials** to probe the network. A stable client identity lets the monitor reuse cached gateway shared keys and its claimed bandwidth allowance across runs; rotating the identity per run would force a fresh registration handshake with every gateway each run and burn tickets on re-crediting, severely increasing ticket usage. So static keys were the economical choice under a credentialed tester. Note that `disabled_credentials_mode` now defaults to `true` (the monitor probes without presenting credentials), so that original constraint may no longer bind - revisiting key rotation is therefore possible future work, but is not planned.

**Alternative considered.** Fresh ephemeral keys per run or per gateway. Explicitly flagged in-source (`preparer.rs`, `mod.rs::build`) as the desirable future state. It is more invasive than it looks: rotation would have to update the pubkeys in `PacketPreparer`, rebuild the `ReceivedProcessor`'s decryption key, swap the immutable `PacketSender.local_identity`, and flush `gateways_key_cache` (else the monitor would try to authenticate with shared keys derived from the retired identity). Key rotation is also only a partial mitigation, since test traffic is still detectable by pattern.

**Consequence.** A malicious gateway can fingerprint the monitor's well-known keys and selectively forward test traffic while dropping real traffic, inflating its score. This is a known limitation, documented as a public-surface fact so it is not mistaken for an accident. See Resolved Questions, Q1.

### Decision 7: Performance-weighted, layer-randomised node selection with a small-network fallback

**Choice.** Candidate route nodes are drawn with `choose_multiple_weighted` where the weight is each node's `detailed_performance.performance_score` from the node-status cache, defaulting to `0.5` when a node has no annotation. Mixing nodes are bucketed into the three legacy layers by random assignment; if any layer ends up empty (small localnets/testnets) the monitor falls back to `naive_rearrange`, which round-robins all mixing nodes across the three layers.

**Why.** Weighting toward historically-good nodes builds working routes faster; random layer assignment avoids fixed layer roles for nym-nodes that can serve any layer; the fallback keeps the monitor functional on tiny networks where random assignment can starve a layer.

**Consequence.** Route construction is non-deterministic across runs, and on a fresh network with no history every node has weight 0.5 (uniform). Selection reads from live caches, so the monitor first waits for the caches and a minimal topology to be online.

### Decision 8: Gateway clients are RAII handles kept alive across the delivery wait

**Choice.** Each gateway connection is a `GatewayClientHandle` that, on `Drop`, sends a `GatewayClientUpdate::Disconnect` so the `PacketReceiver` stops reading that gateway. The send path (`send_packets`) returns the live handles to the `Monitor`, which holds them across the `packet_delivery_timeout` sleep and only drops them (starting background disconnects) after the wait, so return packets can still be received. Successful shared symmetric keys are cached (`gateways_key_cache`) and reused on the next run; a failed startup removes the cached key. Concurrency is bounded by `max_concurrent_gateway_clients` (default 50; 0 means unlimited) via a custom `ForEachConcurrentClientUse` combinator.

**Why.** Receiving is asynchronous and continues after sending finishes, so connections must outlive the send. Caching keys avoids re-running the authentication handshake every run.

**Consequence.** A gateway whose client fails to authenticate, claim bandwidth, or send (each guarded by its own timeout) is simply dropped from the run; every node whose packets were routed through that gateway loses those packets and scores lower. Bandwidth is claimed per run; `disabled_credentials_mode` (Decision 10) governs whether real credentials are presented.

### Decision 9: Acknowledgements are constructed and received but ignored

**Choice.** Test packets carry ack keys and the receiver decodes returning acks, but ack handling is a no-op (logged at trace and dropped). The ack channel exists only so the gateway client does not error.

**Why.** The monitor scores on returned data packets, not acks; wiring acks into scoring (for example as a latency/RTT signal) was deferred.

**Consequence.** No latency dimension is captured. Documented so the presence of ack plumbing is not read as an active feature.

### Decision 10: `disabled_credentials_mode` defaults to true

**Choice.** `NetworkMonitorDebug::disabled_credentials_mode` defaults to `true`, meaning the monitor attempts to claim gateway bandwidth without presenting bandwidth credentials. It is inverted from the operator-facing `monitor_credentials_mode` flag during config assembly.

**Why.** The monitor is operated by the same entity as the gateways in the standard deployment, so requiring paid credentials for probing traffic is unnecessary by default.

**Consequence.** Gateways must accept the monitor's uncredentialed bandwidth claims for probing to work in the default configuration.

### Decision 11: Test messages are JSON-serialised and must fit a single sphinx packet

**Choice.** `TestMessage` (carrying `tested_node`, `msg_id`, `total_msgs`, and the flattened `NymApiTestMessageExt`) is serialised with `serde_json`; construction fails with `TestMessageTooLong` if the padded message does not fit in one `RegularPacket`. Received nodes are matched back purely by the `tested_node` field (`encoded_identity`, `node_id`, `type`) embedded in the decrypted payload.

**Why.** Test messages are small, so JSON's overhead is negligible and its debuggability is worth more than bincode compactness; keeping each test to a single packet keeps the delivery signal one-packet-one-outcome.

**Consequence.** Node matching trusts the self-describing payload rather than any transport-level identity, which is why the nonce filter (Decision 2) is required to keep the payload trustworthy within a run.

### Decision 12: Downstream data flow - where the score goes inside nym-api

**Choice.** This is a catalog, not a mechanism choice: it fixes the consumer surface of the persisted score so its blast radius is legible. The monitor writes per-node reliability into the legacy `mixnode_status` / `gateway_status` tables plus the `monitor_run` / `monitor_run_report` / `monitor_run_score` tables. Within nym-api the score fans out as follows.

- **Effective-source gate (read this first).** There are two mutually-exclusive performance sources selected at startup (`run.rs`). When `use_performance_contract_data` is `false`, the `LegacyStoragePerformanceProvider` (`node_performance/provider/legacy_storage_provider.rs`) reads `get_average_node_reliability_in_the_last_24hrs` (`support/storage/mod.rs`) and turns it into a `RoutingScore`. When it is `true`, the performance-contract cache is the source and the monitor's persisted reliability is NOT read to build annotations. Either way, `has_performance_data = network_monitor.enabled || use_performance_contract_data`.
- **Node annotations.** The `NodeStatusCacheRefresher` (`node_status_api/cache/refresher.rs`) folds the routing score (plus config and stress-test components) into `NodeAnnotationV2.detailed_performance.performance_score`. This annotation is the single fan-out point for the reward and route-selection consumers below.
- **Route-selection feedback loop.** The monitor's own `PacketPreparer` (`network_monitor/monitor/preparer.rs`) reads `node_annotations()` and uses `performance_score` (default 0.5 when absent) as the candidate-selection weight - a node's past score biases its future testing. Covered normatively by the performance-weighted-sampling requirement.
- **Historical uptime.** The `HistoricalUptimeUpdater` (`node_status_api/uptime_updater.rs`) reads the last 24 hours of `mixnode_status` / `gateway_status` reports directly (independent of the provider gate) and writes per-day `*_historical_uptime` rows; it is started only when `network_monitor.enabled`.
- **Rewarding + rewarded-set selection.** The `EpochAdvancer` (`epoch_operations/`) reads the annotation via `to_rewarding_performance()` and uses it (a) as the `performance` in each `RewardNode` message and (b) as the rewarded-set selection weight `saturation * performance^20` (`rewarded_set_assignment.rs`); `performance == 0` gives weight 0, the only de-facto exclusion. Runs only when `rewarding.enabled && has_performance_data`.
- **HTTP read endpoints.** Representative routes: deprecated `/v1/status/{mixnode,gateway}/{id}/history` and `/core-status-count` (mounted only when the monitor is enabled); `/v1/status/{mixnodes,gateways}/unstable/{id}/test-results`; `/v1/status/network-monitor/unstable/run/{id|latest}/details`; `/v1/nym-nodes/{annotation,performance,historical-performance,performance-history,uptime-history}/{id}`; and skimmed/semi-skimmed node listings that embed `last_24h_performance`.

**Inert threshold knobs.** `min_mixnode_reliability` (50), `min_gateway_reliability` (20), and the rewarding-side `minimum_interval_monitor_threshold` (60) exist only in config definitions/defaults and the config template. A repo-wide search finds no readers. The config comment claiming sub-threshold nodes are blacklisted and excluded from the rewarded set does not correspond to any code path in the current implementation.

**Two related-but-distinct ingestion paths (data-model context, not consumers).** The internal monitor is one of three writers of "monitoring results", and they use different tables:
- Internal monitor -> `submit_mixnode_statuses` / `submit_gateway_statuses` -> legacy `mixnode_status` / `gateway_status` (+ `monitor_run*`).
- External signed `MonitorMessage` (POST `/v1/status/submit-{node,gateway}-monitoring-results`, `node_status_api/handlers/without_monitor.rs`) -> verified against the **hardcoded** `NETWORK_MONITORS` allow-set (`common/types/src/monitoring.rs`) -> `submit_mixnode_statuses_v2` / `submit_gateway_statuses_v2` -> the **`*_v2`** tables.
- v3 stress-testing (POST `/v3/nym-nodes/stress-testing/batch-submit`, `nym_nodes/handlers/v3.rs`) -> **chain-backed** `NetworkMonitorsCache.is_authorised` allow-set + `LastNMSubmissions` replay guard -> `insert_nym_node_stress_testing_results` (separate stress table; feeds the provider's stress-score component of `performance_score`).

**Why document it here.** A reader asking "what breaks if the monitor scores a node 0?" needs the fan-out (annotations -> reward weight `^20` -> possible rewarded-set drop, plus the self-reinforcing route-selection loop) in one place. Keeping only the producing side in the spec left that question unanswered and led to a false claim about the inert threshold knobs.

**Consequence.** The consumer subsystems keep their own capabilities; this catalog is the index. The provider gate means "the monitor feeds rewards" is only true in the legacy-storage configuration; under the performance contract the monitor still feeds historical uptime and the status HTTP endpoints but not annotations/rewards.

## Risks / Trade-offs

- **Reused static keys → gateway fingerprinting.** A malicious gateway can recognise the monitor and cheat its own score. → Mitigation (future): per-run ephemeral keys (Resolved Questions, Q1). Currently unmitigated by design - accepted as a documented risk.
- **All-or-nothing route verification is strict.** A single dropped packet out of 1000 discards a route, wasting attempts on a lossy network. → Mitigation: up to `test_routes * 10` attempts and a `minimum_test_routes` floor; run aborts cleanly if the floor is not met.
- **Low `minimum_test_routes` → noisy scores.** With the default floor of 1, a node's reliability can hinge on a single path. → Mitigation: operators can raise `test_routes` / `minimum_test_routes`; the score denominator uses the achieved route count so it stays a valid ratio.
- **Per-gateway failure zeroes many nodes.** If a gateway client fails startup or send, every node routed through it loses packets for that run. → Mitigation: retries across runs; transient dips are absorbed by downstream historical-uptime averaging (out of scope here).
- **No latency signal.** Acks are ignored, so a slow-but-working node is indistinguishable from a fast one. → Accepted trade-off.
- **Eager receiver-task spawn.** `ReceivedProcessor` builds its receive future at construction and `start_receiving` `take()`s it, panicking if called twice; flagged in-source as needing refactor. → Mitigation: single controlled startup sequence.

## Resolved Questions

All questions were walked through with the maintainer on 2026-07-23. Every one resolved as document/keep: no behaviour changes and no follow-on changes are opened by this spec. None remain open.

1. **Ephemeral keys per run to defeat fingerprinting? (Decision 6.)** RESOLVED - accept as a known, documented risk; do not change. The static identity was a deliberate cost optimisation from when the tester was assumed to require bandwidth credentials: a stable identity reuses cached gateway shared keys and bandwidth allowance across runs, whereas rotating per run would burn tickets on re-registration. Recorded in Decision 6. `disabled_credentials_mode` now defaults to `true`, so the original constraint may no longer bind; revisiting rotation is possible future work but is not planned here.
2. **Refactor the `ReceivedProcessor` construct-then-`take()` lifecycle? (In-source TODO.)** RESOLVED - accepted internal tech-debt. It has no behaviour or spec impact; a candidate for a trivial standalone code cleanup, not tracked as an OpenSpec change.
3. **Should auth failure blacklist only the gateway, not its mixnodes? (Decision 4.)** RESOLVED - moot. It depends on the route-node blacklist, which is abandoned dead code (Question 5), so the asymmetry has no observable effect. Nothing to decide until/unless the blacklist is revived.
4. **Adopt or drop the sketched outlier-removal step (`ALLOWED_RELIABILITY_DEVIATION`)?** RESOLVED - keep as a documented future idea. Marginal value, because all-or-nothing route pre-verification already keeps bad routes out of scoring. The commented-out note in `summary_producer.rs` is retained; no work planned.
5. **Wire the route-node `blacklist`, or remove the dead code? (Decision 4.)** RESOLVED - keep documented as abandoned/vestigial dead code (built, never consumed, zero effect; verified routes can overlap). No code change now; removal is a possible future cleanup. Captured in Decision 4.
6. **Wire the inert `min_mixnode_reliability` / `min_gateway_reliability` / `minimum_interval_monitor_threshold` knobs, or remove them? (Decision 12.)** RESOLVED - keep in config, documented as inert. They have no readers; a hard reliability floor would be a reward-economics policy change requiring product sign-off, and today's soft de-weighting of low performers already comes from the `performance^20` selection weight. No change; the misleading in-source comment is recorded as not matching code (Decision 12).

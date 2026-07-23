## Why

The network monitor (`nym-api/src/network_monitor/`) is nym-api's active liveness prober: on a fixed interval it injects known test packets into the live mixnet, addressed so they loop back to itself, and turns the fraction that return into a per-node reliability score (0-100). Those scores are persisted per monitor run and feed historical uptime and, downstream, reward eligibility (`has_performance_data` gates `EpochAdvancer` in `run.rs`). It is one of two performance-data sources for the whole network (the other being the performance contract).

The subsystem has shipped, is live, and exists only as Rust source plus scattered `TODO`/`SAFETY` comments. Several behaviours are load-bearing but non-obvious from reading any single file, and get re-derived from scratch whenever monitor output looks wrong during triage:

- **Run isolation is nonce-based, not connection-based.** Every run stamps its packets with a monotonically increasing `test_nonce`; the receiver rejects any decrypted packet whose embedded nonce does not match the currently-armed value, and `u64::MAX` is the sentinel for "no run in progress." Route-verification packets reuse a fixed reserved nonce `ROUTE_TESTING_TEST_NONCE = 0`. This is how late packets from a previous run are silently discarded rather than mis-attributed.
- **A single node is isolated by topology substitution.** Test packets always traverse a verified 3-mix + 1-gateway "test route," but for the node under test that node is swapped into its role in the route topology, so a mixnode measurement is `route-gateway -> (2 route mixes + tested mix) -> route-gateway -> self`, and a gateway measurement ingresses and egresses through the tested gateway. The score isolates one node at a time even though a full route is involved.
- **Route verification is all-or-nothing.** A candidate route is declared "working" only if *all* `route_test_packets` (default 1000) return; otherwise it is discarded. Nodes on a working route are accumulated into a blacklist that was *intended* to keep subsequent routes from overlapping - but the blacklist is currently write-only (never passed to the candidate selector), so verified routes can in fact still share nodes. This is a real, easy-to-miss current-state gap.
- **Reused static monitor keys are a known security limitation.** The monitor generates one ed25519/x25519/ack key set at startup and reuses it for every gateway and every run, flagged in-source as something malicious gateways could fingerprint. This is current, intended-for-now behaviour, not a bug to be silently fixed.
- **Acknowledgements are received but ignored**, and `disabled_credentials_mode` defaults to `true` (the monitor claims bandwidth without presenting credentials by default). Each of these is a deliberate current-state fact that reads like an oversight without the rationale.

Capturing the spec now - while the behaviour and its rationale are fresh - is materially cheaper than reconstructing it from `git blame` later, and gives reward-eligibility discussions a normative reference for exactly what a reliability score measures.

## What Changes

- Introduce a new capability spec `network-monitor` covering the subsystem end to end: startup gating and task spawning, the cache/topology readiness gate, the periodic run loop, test-route generation and verification, per-node test-packet construction (including the topology-substitution isolation trick and self-addressed loop recipients), the gateway-client send path (authentication, bandwidth claim, rate-limited batching, RAII disconnect-on-drop), the receive-and-match path (nonce filtering, ack handling), reliability scoring, report generation, and persistence to `NymApiStorage`.
- Document the **run-isolation model**: monotonic `test_nonce`, the reserved route-testing nonce `0`, the `u64::MAX` idle sentinel, and the atomic drain-and-reset on `return_received`.
- Document the **scoring semantics**: `reliability = round(received / (num_test_routes * per_node_test_packets) * 100)` clamped into a `u8`, why the denominator scales with route count, and how per-route performance is computed separately.
- Document the **configuration surface and defaults** (`NetworkMonitorDebug`) since these knobs directly change what a score means (route count, packets per node, timeouts, sending rate, concurrency), and correct the record on the `min_mixnode_reliability` / `min_gateway_reliability` / `minimum_interval_monitor_threshold` knobs, which are defined but currently have no readers (an in-source comment implies otherwise).
- Document the **consumer surface within nym-api** - where the persisted score goes: the performance provider feeding node annotations (`performance_score`), the monitor's own route-selection feedback loop, the historical-uptime updater, rewarding and rewarded-set selection (`saturation * performance^20`), and the read-only HTTP status / nym-node endpoints - together with the provider source-selection gate (monitor-storage vs performance-contract) that decides whether the monitor's score is the effective source, plus data-model context on the two external signed-submission ingestion paths (legacy `MonitorMessage` -> `*_v2` tables; v3 stress-testing -> chain-backed allow-set) that produce similar `NodeResult` data through different tables.
- Document, as public-surface facts, the **known limitations**: reused static keys, ignored acks, the write-only/unwired route-node blacklist (verified routes can overlap), and the `ReceivedProcessor` startup-lifecycle caveat (it spawns its receive future eagerly, flagged in-source as needing refactor).
- No code changes, no migrations, no new dependencies. This is a documentation-only deliverable that ratifies the current implementation as the baseline. Any behaviour that a reviewer judges to be an actual bug (rather than intended) becomes a follow-on change, not an edit to this spec.

## Capabilities

### New Capabilities
- `network-monitor`: The nym-api active reliability-probing subsystem - lifecycle, test-route selection and verification, per-node test-packet construction, send/receive over gateway clients, reliability scoring, reporting, persistence, configuration, and known limitations.

### Modified Capabilities
<!-- None. No existing capability's requirements change. -->

## Impact

- **Documentation only.** No runtime behaviour changes.
- **Code described** (treated as the normative baseline): `nym-api/src/network_monitor/**` (`mod.rs`, `monitor/mod.rs`, `preparer.rs`, `sender.rs`, `receiver.rs`, `processor.rs`, `summary_producer.rs`, `gateway_client_handle.rs`, `gateways_reader.rs`, `test_route/mod.rs`, `test_packet.rs`), the startup gate in `nym-api/src/support/cli/run.rs`, the config surface in `nym-api/src/support/config/mod.rs`, the persistence entry points in `nym-api/src/support/storage/`, the shared test primitives in `common/node-tester-utils/`, and the `NodeResult` / `MonitorMessage` types in `common/types/src/monitoring.rs`.
- **Consumer code catalogued** (behaviour owned by other capabilities, referenced here for the output surface): `nym-api/src/node_performance/provider/`, `nym-api/src/node_status_api/cache/`, `nym-api/src/node_status_api/uptime_updater.rs`, `nym-api/src/epoch_operations/`, `nym-api/src/node_status_api/handlers/`, `nym-api/src/nym_nodes/handlers/`, and `nym-api/src/unstable_routes/`.
- **No migrations. No new dependencies.**

## Why

The nym-api-internal v1 network monitor scores a node's "liveness" by looping packets through a route of five nodes (entry gateway, three mixnodes, exit gateway) and attributing the whole route's delivery ratio to whichever node was substituted in. Its own design record predicted the consequence we now observe in production: "a node that is fine but happens to be tested only against a marginal route can score low", and with `minimum_test_routes = 1` a score "can rest on a single route/gateway path and be noisy". Each node also receives only `per_node_test_packets` (3) packets per route, so a mixnode's reliability is quantised to ninths.

Network monitor v3 already has the machinery to fix this: chain-authorised agents that probe a single node directly and attribute the result to that node alone. Its design record reserves exactly this work, naming "single-hop LIVENESS checks for mixnodes AND gateways, in order to REPLACE the nym-api-internal v1 network monitor" as the known roadmap, and the current implementation already carries the seams (a gateway test type in the data model, gateway-capable nodes retained in the registry, `is_mixnode` / `was_reachable` on every result, and a node-type-agnostic authorisation model). This change builds the first half of that: liveness as a second test kind alongside stress testing, scored per node with no other node in the path.

## What Changes

- **A new `liveness` test kind** alongside the existing stress test, assigned by the orchestrator and executed by the agent. For a mixnode it is the existing two-hop self-loop probe at low volume (order 100 packets rather than 30000). For a gateway it is a new two-phase test.
- **Gateway liveness is one indivisible test with two phases** over a single client session: an ingress phase (agent as gateway client, packet forwarded out to the agent acting as a mixnode) and an egress phase (agent as mixnode, packet delivered as a final hop back to the agent's client session). Both phases MUST be performed by the same agent in the same run; a phase that produces no signal scores zero rather than being excluded from the average.
- **Gateway client sessions are established over `ws://<announced-ip>:<clients_ws_port>` only**, ignoring announced hostnames and wss entries. This keeps the tested source IP truthful, removes DNS and certificate handling from the probe, and costs nothing in authenticity because the registration handshake already authenticates the gateway's ed25519 identity. Testing the wss ingress path is explicitly out of scope and recorded as a future test kind.
- **BREAKING (nym-node):** final-hop packets originating from an authorised network-monitor agent are currently dropped outright ("unsupported network monitor final hop packets"). They MUST instead be processed and delivered to a live client session, and MUST NOT fall back to disk storage. Gateway egress testing is impossible without this, and the no-disk rule keeps monitor traffic from accruing undeliverable rows on every gateway.
- **BREAKING (nym-node):** a client websocket session opened from an authorised network-monitor agent IP MUST be treated as an ephemeral monitor session: unmetered, and writing nothing to gateway storage. The agent presents no ecash ticketbook.
- **The agent tests a wave of targets concurrently** rather than one target per invocation, turning the assignment lease bound from the sum of per-target worst cases into the maximum of them, and making a full-network liveness sweep viable at v1's cadence. This requires a shared ingress listener, a multi-target Noise view, and per-target attribution of returned packets.
- **Per-test-kind scheduling in the orchestrator**: per-kind staleness gates, per-kind address rotation cursors, per-kind lease budgets materialised as an `expires_at` on each in-progress row, and a cooldown that keeps a liveness test from measuring a node still recovering from a stress test. The existing single-in-flight-test-per-node mutex is retained unchanged and now spans kinds.
- **Per-kind result submission** to nym-api, with a separate watermark per kind and a per-signer replay high-water mark that cannot be shared between kinds.
- **Liveness enters node performance as a third component with weight zero** (shadow mode) alongside the v1 routing score and the v3 stress score, plus a divergence gauge comparing v3 liveness against v1 routing, bucketed by whether the gateway announces a wss entry so that expected divergence is separable from unexpected divergence.

## Capabilities

### New Capabilities

None. Liveness testing is performed by the same actors, under the same chain authorisation, through the same assignment and submission lifecycle as stress testing, so it belongs in the existing capability rather than duplicating that context.

### Modified Capabilities

- `nym-network-monitor`: adds the liveness test kind and its per-node and per-gateway probe mechanics; amends testrun assignment to be keyed by (node, test kind) with per-kind staleness, rotation and lease budgets; amends the agent lifecycle from one target per invocation to a concurrently-executed wave; amends the node-side gating requirement to permit monitor final-hop delivery without disk fallback and to define the ephemeral unmetered monitor client session; amends the orchestrator storage requirement for the per-kind schema; amends result submission for per-kind watermarks; amends the downstream consumer surface for the shadow-weighted liveness component; and amends the configuration surface for the liveness knobs.

## Impact

- **nym-node** (`src/node/mixnet/handler.rs`): final-hop handling for network-monitor packets. **BREAKING** for gateway egress testing, which cannot work on un-upgraded nodes. Un-upgraded gateways will score zero on the egress phase, which is one reason liveness ships at weight zero.
- **gateway** (`src/node/client_handling/websocket/connection_handler/{fresh,authenticated}.rs`) and **common/credential-verification** (`bandwidth_storage_manager.rs`): ephemeral, unmetered monitor client sessions. Touches the `ClientDetails` / `BandwidthStorageManager` seam because the storage-assigned `client_id` has no meaning for a session that persists nothing.
- **nym-network-monitor-orchestrator**: new migration (per-kind test state, per-kind watermarks, in-progress leases, gateway ws entry address, per-signal result rows), assignment query, node refresher, result submitter, prometheus surface.
- **nym-network-monitor-agent**: liveness probe profile, wave concurrency with a shared listener and multi-target Noise view, gateway client session and the two-phase gateway probe, ed25519 client identity (derived, not provisioned).
- **nym-network-monitor-orchestrator-requests**: test-kind-tagged assignments and results carrying per-signal breakdowns.
- **nym-api**: liveness ingest endpoint with its own per-signer replay high-water mark, storage for liveness results, and a shadow-weighted performance component plus the divergence gauge.
- **No contract change.** The `network-monitors` contract cannot be redeployed, and nothing here requires it: liveness reuses the existing agent authorisation, and the gateway session exemption keys on the already-propagated authorised agent IPs.

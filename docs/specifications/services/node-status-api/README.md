# Node Status API

The Node Status API serves information about individual `nym-nodes` in the Mixnet: the role each node operates in, statistics about each node, the services it offers such as Network Requesters, and summaries of the state of the Mixnet. Developers who build applications such as explorers or analytics interfaces for the Mixnet should run their own instance of the API. More instances give a more robust network of downstream services and spread the load of API calls across more endpoints.

The node status API is the network's aggregation and read service. It scrapes the nym-api directory and the chain on a timed cycle, stores what it finds in PostgreSQL, and serves that store to the explorer, to NymVPN clients through the dVPN directory routes, and to operators. It also runs the network's active gateway testing: it queues probe and ports-check test runs, hands them to external `nym-node-status-agent` workers over an authenticated internal API, and pays for those tests with a private buffer of ecash ticketbooks it issues itself.

## Why it exists

Nobody can read "is this gateway reachable, and from where" off the chain. Bond state and self-description tell a client what a node claims about itself, not whether it answers a connection. The service exists to close that gap: it holds the only aggregated, continuously refreshed view of directory state, geolocation, packet counters and live probe results, and it is the only component that runs active tests against gateways from outside the chain's view.

A second problem sits behind the first. Active testing needs an ecash ticket to spend against the gateway under test, the same as any client. The service cannot ask a human to keep it stocked with tickets, so it deposits on `nyxd` and collects threshold-signed ticketbooks for itself, on the same schedule as its probes.

## Technical details

- [Design and rationale](../../technical-details/node-status-api/README.md): the process model, the four subsystems, and how they share the database.
- [Monitor cycle](../../technical-details/node-status-api/monitor-cycle.md): the ordered ingestion cycle, the description and packet-stats scrapers, and the metrics scraper.
- [HTTP surface](../../technical-details/node-status-api/http-surface.md): the public route table, the response contract, caching, and the dVPN directory pipeline.
- [Persistence](../../technical-details/node-status-api/persistence.md): the PostgreSQL schema, column ownership, and retention.
- [Test-runs](../../technical-details/node-status-api/testruns.md): the queuer, the authenticated agent protocol, and submission handling.
- [Ticketbook issuance and geodata](../../technical-details/node-status-api/ticketbook-and-geodata.md): the ecash ticketbook buffer that funds test runs, and the current in-process geolocation lookup.

## Code

- `nym-node-status-api/nym-node-status-api`: the service binary. `src/main.rs` wires the pool, the caches and every background worker. `src/monitor`, `src/node_scraper`, `src/metrics_scraper` are the ingestion workers. `src/http` is the axum server. `src/testruns` is the queuer and the internal agent API. `src/ticketbook_manager` is the ecash buffer.
- `nym-node-status-api/nym-node-status-agent`: the CLI worker that polls the internal test-run API, runs `nym-gateway-probe` in process, and submits signed results. The probe library is specified in [Gateway Probe](../../tools/gateway-probe/README.md).
- `nym-node-status-api/nym-node-status-client`: the shared signed-request client the agent uses to talk to the internal API (`src/lib.rs`, `src/auth.rs`, `src/models.rs`).
- `nym-node-status-api/nym-node-status-ui`: the Next.js explorer and operator frontend that consumes the public HTTP surface. It is a consumer, not part of this node.
- `common/credential-proxy` (crate `nym-credential-proxy-lib`): the deposit, quorum-check and ecash-state code the ticketbook manager embeds directly. See the note under [Ticketbook issuance and geodata](../../technical-details/node-status-api/ticketbook-and-geodata.md#relationship-to-nym-credential-proxy).

## Used by

- NymVPN clients and the VPN API, through the unauthenticated `/dvpn/v1/directory/*` routes.
- The explorer and operator frontend, `nym-node-status-ui`, through the `/v2/*` and `/explorer/v3/*` routes.
- `nym-node-status-agent` instances, through the authenticated `/internal/testruns*` routes.
- Node operators and dashboards, through `/v2/status/*`.

## Cross-links

- Performance contract (`contracts/performance`): the on-chain performance score the service reads and republishes as `performance`, `uptime` and the dVPN `performance`/`performance_v2` fields. The service does not write scores; it only reads and reshapes what nym-api and the performance contract already produced.
- Network monitors contract (`contracts/network-monitors`): a separate authorisation registry for network monitors. It does not gate the node status API's own probes; the two are independent testing paths that both feed into node health.
- Geolocation contract (`contracts/geolocation`): the service runs its own IP-to-location lookup against `ipinfo.io` (see [Ticketbook issuance and geodata](../../technical-details/node-status-api/ticketbook-and-geodata.md)) and does not read the geolocation contract.

## Why

Mixnodes are tested for liveness and under load; gateways are tested for liveness only. A gateway that passes fifty packets can still fall over under sustained client traffic, and nothing in its score shows it: the performance contract scores a gateway on liveness alone, with the stress share renormalised away. The contract's `stress` measurement is already role-blind and the nym-api result stream is gone, so gateways can start reporting stress without touching the contract or nym-api.

## What Changes

- A fourth test kind, `gateway_stress`: the gateway liveness test (one client session, the ingress phase then the egress phase) run under a stress profile of its own, one target per assignment.
- Both gateway kinds: a phase stops sending when its profile's send window ends, even while a write is blocked, and the packets the gateway did not take count as lost. A phase cut short (by the window, a failed send or the run's deadline) still counts what arrived, against the full expected count. This changes gateway liveness under back-pressure and on a mid-phase failure, where today a phase is unbounded and reports zero received.
- Agent: five `--gateway-stress-*` knobs (target rate, sending duration, batch size, straggler wait, per-target timeout) with matching environment variables, and a manual `test-gateway-stress` command. The initial values are deliberately lenient (200 packets/second for 30 seconds per phase) and are meant to be raised progressively. The existing stress knobs and their environment variables are renamed `mixnode-stress-*`, without aliases.
- Orchestrator: `gateway_stress` is scheduled on the stress interval and lease it shares with `mixnode_stress`, behind its own `--gateway-stress-enabled` switch (on by default). One migration adds the kind and its results table.
- Aggregation and submission: both stress kinds share the stress window and are reported as one stress figure, combined by sample count as the liveness kinds are. The coverage guard measures gateway stress on its own. A gateway's stress reaches the performance contract as the existing `stress` measurement.
- **BREAKING** (agent and orchestrator wire types): an agent built before this change cannot parse a gateway stress assignment, so the two are deployed together.
- **BREAKING** (agent configuration): the five mixnode stress flags and environment variables are renamed, so the agents' deployment scripts change in the same redeploy.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `nym-network-monitor`: the set of kinds and which role each assigns; the new gateway stress test, its profile, switch and results table; the send window and partial-credit rules of a gateway phase.
- `network-monitor-epoch-performance`: the two stress kinds share one aggregation window and are reported as one stress figure.
- `network-monitor-contract-submission`: coverage is measured per test kind over four kinds, and a node's submitted stress combines the stress kinds the guard kept.

## Impact

- `nym-network-monitor-v3`: the agent (gateway probe, profiles, CLI), the orchestrator requests crate (assignment and result variants, read types) and the orchestrator (storage and migration, scheduling, HTTP state, aggregation, submission plan, metrics labels).
- No change to the performance contract, nym-api or nym-node.
- Scoring: once gateway stress clears the coverage guard, every gateway's committed score carries the stress weight, so this is reward-affecting. The lenient initial profile keeps that step small for a healthy gateway.
- Deployment: orchestrator and agents are redeployed together. The new kind takes every agent request until each gateway has been tested once.

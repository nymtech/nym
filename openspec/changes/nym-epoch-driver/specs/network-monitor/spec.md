## REMOVED Requirements

### Requirement: The network monitor runs only when explicitly enabled and spawns three cooperating tasks
**Reason**: nym-api no longer measures the network. The v1 monitor is removed with every path that fed nym-api's own performance figure; measurement is the NMv3 orchestrator's, submitted to the performance contract.
**Migration**: Remove `[network_monitor]` from existing configurations (it is ignored if left). The orchestrator and the performance contract carry every measurement nym-api serves.

### Requirement: The monitor waits for caches and a minimal topology before the first run
**Reason**: The v1 monitor is removed.
**Migration**: None.

### Requirement: Test runs execute on a fixed interval and are isolated by a monotonic per-run nonce
**Reason**: The v1 monitor is removed.
**Migration**: None.

### Requirement: Candidate test routes are selected by performance-weighted random sampling across layers
**Reason**: The v1 monitor is removed.
**Migration**: None.

### Requirement: Candidate routes are verified all-or-nothing before use; a run aborts if too few verify
**Reason**: The v1 monitor is removed.
**Migration**: None.

### Requirement: A test route is a self-contained 3-mix + 1-gateway topology
**Reason**: The v1 monitor is removed.
**Migration**: None.

### Requirement: Per-node test packets isolate one node by topology substitution and loop back to the monitor
**Reason**: The v1 monitor is removed.
**Migration**: None.

### Requirement: Packets are delivered to gateways via authenticated, rate-limited gateway clients
**Reason**: The v1 monitor is removed, with its credentials store.
**Migration**: None.

### Requirement: Returned packets are collected by a single receiver task and filtered by the armed nonce
**Reason**: The v1 monitor is removed.
**Migration**: None.

### Requirement: Gateway connections are kept alive across the delivery wait and disconnected on drop
**Reason**: The v1 monitor is removed.
**Migration**: None.

### Requirement: Node reliability is the delivery ratio over verified routes
**Reason**: The v1 monitor is removed; a node's figure is the performance contract's.
**Migration**: Read a node's figure from the performance contract or from nym-api's attested performance snapshots.

### Requirement: Each run produces a human-readable report bucketed by reliability threshold
**Reason**: The v1 monitor is removed.
**Migration**: None.

### Requirement: Each completed run is persisted as a monitor-run record with per-node statuses and score histograms
**Reason**: The v1 monitor is removed; nothing writes a run any more. Its tables stay, frozen, behind the deprecated read routes.
**Migration**: None; the routes over these records stay mounted and deprecated, as `nym-network-monitor` specifies.

### Requirement: The persisted reliability score is consumed by a defined set of nym-api subsystems
**Reason**: No consumer remains: the legacy annotation path, the historical uptime updater, legacy rewarding and the route-selection feedback loop are all removed, and nym-api builds its annotations from the attested performance snapshots alone. Every monitor-era GET route stays mounted, deprecated, over the frozen tables, as `nym-network-monitor` now specifies.
**Migration**: Consumers of per-node history use the frozen routes until a replacement exists; the replacement is a follow-up outside nym-api.

### Requirement: The monitor's behaviour is governed by a configuration surface with defined defaults
**Reason**: The v1 monitor is removed with its configuration section and the `--enable-monitor` and `--monitor-credentials-mode` flags of `init` and `run`.
**Migration**: Remove `[network_monitor]` from existing configurations; it is ignored if left. Do not pass `--enable-monitor` or `--monitor-credentials-mode` to `init` or `run`.

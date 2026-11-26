## MODIFIED Requirements

### Requirement: Aggregates are served per kind on the orchestrator's HTTP API

The orchestrator SHALL expose materialised aggregates over HTTP, keyed by node and epoch, with one-hop liveness and stress reported separately alongside their evidence. The one-hop liveness entry is the liveness aggregate of the node's one-hop runs and is named so because it is not scaled by five-hop liveness: `network-monitor-five-hop-liveness` serves five-hop liveness beside it as an entry of its own, and the product of the two exists only in what is submitted. The response shape MUST admit further entries without a breaking change.

Liveness and stress are kept separate because the weighting that combines them into a single performance figure is deliberately not the orchestrator's concern: the performance contract applies its own weights. Each entry is a named field of its own rather than a value in a uniform map, because entries do not all share the score-and-count shape: config score, served beside them, is a decomposition of its own, and five-hop liveness is a bare value.

The runs an aggregate was computed from are served per node and kind by the orchestrator's results endpoints, and nym-api holds the same per-run scores, so a value can be checked against its evidence without a surface of its own. Five-hop liveness carries no count, since whatever a scoring strategy counts on the way is its own; the test messages it was computed from are served per epoch by the route session export of `network-monitor-five-hop-liveness`, for as long as the sessions are kept.

#### Scenario: Each entry is reported with its own value and evidence
- **WHEN** a consumer requests a node's aggregates for an epoch
- **THEN** it receives a separate one-hop liveness and stress entry, each carrying its value and sample count

#### Scenario: An entry with no aggregate is absent rather than zero
- **WHEN** a node has a stress aggregate and no liveness aggregate of either kind
- **THEN** the response omits the one-hop liveness entry rather than reporting it as 0.0

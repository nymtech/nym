## MODIFIED Requirements

### Requirement: Aggregation windows are configured per kind

The two liveness kinds SHALL share one aggregation window, defaulting to 6 hours, and the two stress kinds SHALL share another, defaulting to 24 hours. The orchestrator MUST apply each kind's window when computing that kind's aggregate.

The defaults differ because the cadences do, by an order of magnitude. At a 15 minute liveness interval a 6 hour window holds roughly 24 runs, which is ample evidence while keeping the figure responsive to a node that has just broken. At a 2 hour stress interval the same window would hold three, so stress needs 24 hours to reach a comparable dozen. The kinds of each family share their window as they share the rest of their scheduling, which is also what keeps each family's combined figure exact.

No startup validation ties a window to its kind's cadence, deliberately. Such a check could only assert what the configuration is CAPABLE of producing, and cadence is a target rather than a guarantee: a node can be held by another kind's per-node lock, or simply not reached by the sweep, so no relationship between window and interval guarantees that any particular number of runs actually arrived. The sample count recorded with each aggregate is the only ground truth, and how few samples is too few is a judgement for whichever consumer acts on the value rather than one the orchestrator should make on its behalf.

#### Scenario: Each kind uses its own window
- **WHEN** aggregates are materialised for an epoch
- **THEN** the liveness aggregates cover the liveness window and the stress aggregates cover the stress window, independently of each other

#### Scenario: A thin aggregate is published with its count rather than withheld
- **WHEN** only one run of a kind falls in that kind's window for a node
- **THEN** the aggregate is materialised from that single run and its evidence records one sample, leaving the consumer to decide whether that is enough

### Requirement: One aggregate per node and kind, with both liveness kinds reported as one

The orchestrator SHALL store exactly one aggregate per `(node, epoch, kind)`, averaging across every announced address that kind exercised for that node in the window. It SHALL report the two liveness kinds as a single liveness figure: the mean over both kinds' runs, obtained by weighting each kind's aggregate by its sample count. It SHALL report the two stress kinds the same way, as a single stress figure.

Runs rotate through a node's announced addresses, so several run streams exist per node and kind. Collapsing them matches the existing stored-score semantics, and a node whose ipv6 address is broken therefore carries genuine zeros into its aggregate, which is the intended reading rather than a defect.

A dual-role node is tested by both kinds of each family, one per role. Storing them apart keeps every kind's aggregate keyed like its results, while reporting each family as one matches the performance contract, which holds one liveness and one stress measurement per node whatever its roles. The combination is exact because each aggregate carries its sample count and the kinds of a family share one aggregation window.

#### Scenario: A dual-role node yields one liveness figure
- **WHEN** a node had three mixnode liveness runs scoring 1.0 and one gateway liveness run scoring 0.0 within the window
- **THEN** its reported liveness aggregate is 0.75 over four samples, the mean over all four runs rather than the mean of the two kinds' means

#### Scenario: A failing address is reflected rather than hidden
- **WHEN** a node's runs against one announced address consistently score zero while another scores well
- **THEN** the aggregate reflects both, and the shortfall is visible rather than suppressed

#### Scenario: A dual-role node yields one stress figure
- **WHEN** a node had three mixnode stress runs scoring 1.0 and one gateway stress run scoring 0.0 within the window
- **THEN** its reported stress aggregate is 0.75 over four samples, the mean over all four runs rather than the mean of the two kinds' means

#### Scenario: A gateway-only node's stress figure is its gateway stress
- **WHEN** a gateway-only node had gateway stress runs within the window
- **THEN** its reported stress aggregate is the mean of those runs, with their count as its samples

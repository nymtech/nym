## MODIFIED Requirements

### Requirement: A node is submitted with measured values only, with a config score and at least one of liveness or stress

Each submitted node SHALL carry only the figures the orchestrator materialised for that epoch: its config score, its liveness (the liveness test kinds the coverage guard kept, combined as served, then multiplied by the node's five-hop liveness, which `network-monitor-five-hop-liveness` defines and which is 1 for a node the route test has never measured), and its stress, each converted to a percent. A figure the orchestrator does not hold MUST be omitted, never submitted as zero. A node SHALL be submitted only with a config score and at least one of liveness or stress.

An aggregate exists only where runs exist, and a run that failed already scores zero, so an absent figure means unmeasured rather than broken. The contract uses a node's bundle at the epoch whenever one exists, even one it cannot score, so a bundle without a config score, or with nothing but a config score, would also block the fallback to that node's last scored epoch. The five-hop liveness only scales a one-hop liveness the node already has: it is never submitted on its own, so a node with no liveness aggregate is submitted exactly as it would be without the route test.

#### Scenario: An unmeasured kind is omitted
- **WHEN** a node has a config score and a stress aggregate but no liveness aggregate
- **THEN** it is submitted with its config score and stress and no liveness value

#### Scenario: A measured zero is submitted
- **WHEN** a node has a config score and a liveness aggregate of 0.0
- **THEN** it is submitted with liveness 0%

#### Scenario: A mixnode's liveness carries its five-hop liveness
- **WHEN** a mixnode has a config score, a liveness aggregate of 1.0 and a five-hop liveness of 0.9 for the epoch
- **THEN** it is submitted with liveness 90%

#### Scenario: Five-hop liveness is not submitted on its own
- **WHEN** a mixnode has a config score, a stress aggregate, a five-hop liveness of 0.9 and no liveness aggregate
- **THEN** it is submitted with its config score and stress and no liveness value

#### Scenario: A node without a config score is not submitted
- **WHEN** a node has liveness and stress aggregates but no config score for the epoch
- **THEN** it is not submitted

#### Scenario: A config score alone is not submitted
- **WHEN** a node has a config score and no liveness or stress aggregate
- **THEN** it is not submitted


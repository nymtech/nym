## MODIFIED Requirements

### Requirement: A node is submitted with measured values only, with a config score and at least one of liveness or stress

Each submitted node SHALL carry only the figures the orchestrator materialised for that epoch: its config score, its liveness (the liveness test kinds the coverage guard kept, combined as served), and its stress (the stress test kinds the coverage guard kept, combined the same way), each converted to a percent. A figure the orchestrator does not hold MUST be omitted, never submitted as zero. A node SHALL be submitted only with a config score and at least one of liveness or stress.

An aggregate exists only where runs exist, and a run that failed already scores zero, so an absent figure means unmeasured rather than broken. The contract uses a node's bundle at the epoch whenever one exists, even one it cannot score, so a bundle without a config score, or with nothing but a config score, would also block the fallback to that node's last scored epoch. The contract's stress measurement carries no role, so a gateway's stress reaches it as the same measurement a mixnode's does.

#### Scenario: An unmeasured kind is omitted
- **WHEN** a node has a config score and a stress aggregate but no liveness aggregate
- **THEN** it is submitted with its config score and stress and no liveness value

#### Scenario: A measured zero is submitted
- **WHEN** a node has a config score and a liveness aggregate of 0.0
- **THEN** it is submitted with liveness 0%

#### Scenario: A node without a config score is not submitted
- **WHEN** a node has liveness and stress aggregates but no config score for the epoch
- **THEN** it is not submitted

#### Scenario: A config score alone is not submitted
- **WHEN** a node has a config score and no liveness or stress aggregate
- **THEN** it is not submitted

#### Scenario: A gateway's stress is submitted as stress
- **WHEN** a gateway-only node has a config score, a gateway-liveness aggregate and a gateway-stress aggregate, and the guard kept both test kinds
- **THEN** it is submitted with liveness, stress and config, its stress being its gateway-stress aggregate

### Requirement: A test kind without enough reachable coverage is omitted for every node

For each test kind (mixnode liveness, gateway liveness, mixnode stress, gateway stress), the orchestrator SHALL compute coverage as the share of eligible nodes with a reachable aggregate of that test kind for the epoch. A node is eligible when it is described and declares the role the scheduler assigns that test kind to (mixnode liveness and mixnode stress: mixnode; gateway liveness and gateway stress: gateway). An aggregate is reachable when it scores above zero. A test kind whose coverage is below the configured threshold, 0.5 by default, MUST be omitted from every node's submission for the epoch, before the liveness test kinds are combined and before the stress test kinds are. The rule on what a node carries then applies to the remaining values, so when every test kind is omitted nothing is submitted for the epoch. A threshold outside `[0, 1]` MUST be rejected at startup.

A fault on the measuring side records failures, which score zero and look like a network-wide outage. Omitting the test kind turns broken measurements into missing ones: the contract renormalises over the kinds still present, and with nothing submitted it falls back to each node's last scored epoch. Coverage is computed per test kind because the mixnode and gateway probes fail independently, and a broken gateway probe could otherwise hide behind the mixnodes' combined figure, for liveness and for stress alike. The default matches the threshold below which nym-api drops a component. Eligibility follows the scheduler, so a node a test kind never tests cannot count against it. A test kind that has only just been enabled has few aggregates, so the same rule keeps it out of the submission until at least the threshold share of its nodes has been reached.

#### Scenario: A test kind below the threshold is omitted for every node
- **WHEN** 30% of the nodes eligible for mixnode stress have a mixnode-stress aggregate above zero
- **THEN** mixnode stress is omitted, and no node's submitted stress for the epoch includes mixnode-stress runs

#### Scenario: A broken gateway probe does not reach gateways as zeros
- **WHEN** every gateway's gateway-liveness aggregate is 0.0 while most mixnodes' mixnode-liveness aggregates are above zero
- **THEN** gateway liveness is omitted, and no node's submitted liveness includes gateway-liveness runs

#### Scenario: A dual-role node keeps the liveness that was not omitted
- **WHEN** gateway liveness is omitted and a dual-role node has a config score and aggregates of both liveness test kinds
- **THEN** it is submitted with a liveness value computed from its mixnode-liveness runs alone

#### Scenario: A node left without a measured kind is not submitted
- **WHEN** gateway liveness is omitted and a gateway-only node has a config score and a gateway-liveness aggregate
- **THEN** that node is not submitted

#### Scenario: Every test kind omitted means nothing is submitted
- **WHEN** all four test kinds fall below the threshold
- **THEN** nothing is submitted for the epoch

#### Scenario: A gateway does not count against the mixnode kinds' coverage
- **WHEN** a described node declares only the gateway role
- **THEN** it is not eligible for mixnode stress or mixnode liveness, and its missing aggregates of those test kinds do not lower their coverage

#### Scenario: A measured zero is not reachable
- **WHEN** an eligible node's mixnode-liveness aggregate is 0.0
- **THEN** it counts as eligible but not reachable for mixnode-liveness coverage

#### Scenario: A threshold outside the unit interval is rejected
- **WHEN** the coverage threshold is configured as 1.5
- **THEN** startup fails with an error naming the value

#### Scenario: A broken gateway stress probe does not reach gateways as zeros
- **WHEN** every gateway's gateway-stress aggregate is 0.0 while most mixnodes' mixnode-stress aggregates are above zero
- **THEN** gateway stress is omitted: a gateway-only node is submitted with its liveness and no stress, and a dual-role node's stress is computed from its mixnode-stress runs alone

#### Scenario: A newly enabled test kind is omitted until it has coverage
- **WHEN** gateway stress was enabled so recently that fewer than half the gateways have a gateway-stress aggregate above zero
- **THEN** gateway stress is omitted for that epoch and gateways are submitted as they were before it was enabled

### Requirement: An incomplete epoch and a guard trip are visible, and a missing node's reason is logged

The orchestrator SHALL expose, while submission is on, the newest epoch with nothing left to send, so that an epoch left incomplete is visible as that figure no longer advancing, and SHALL expose, per test kind, whether the coverage guard omitted it for that epoch. A node that fails to be described SHALL be logged at `info` with its node id and the step that failed.

An incomplete epoch has to alert rather than fall back silently to last-known values, and a guard trip means the measuring side is broken. During the shadow period, every node nym-api scores but the contract does not has to be matched to a reason before operators are told that describing successfully is a reward requirement; that is a search per node, which a log line with the failing step serves. Completeness is tracked per epoch rather than per chunk, because a retried or restarted pass re-chunks only what is left. It is judged by whether the newest complete epoch keeps advancing, about once an epoch, rather than against the epoch in progress, which only a successful read of the interval reveals: a pass that cannot read the interval, fails before submitting, or finds the epoch in transition leaves the figure where it was all the same.

#### Scenario: An incomplete epoch is visible
- **WHEN** a chunk keeps failing until the epoch's transition begins
- **THEN** the newest completed epoch stays at the one before

#### Scenario: A pass failing before its submission is visible
- **WHEN** the config step fails in every pass of an epoch
- **THEN** the newest completed epoch stays at the one before

#### Scenario: A restart restores completeness from the chain
- **WHEN** the orchestrator restarts after its epoch's submission had fully landed
- **THEN** its first pass finds nothing above the cursor and exposes that epoch as complete

#### Scenario: A guard trip is visible
- **WHEN** the guard omits gateway liveness for an epoch
- **THEN** the orchestrator exposes gateway liveness as omitted for that epoch, and each of the other three test kinds as not omitted

#### Scenario: A describe failure names its step
- **WHEN** a gateway answers every describe read except the client websocket
- **THEN** an `info` log line names the node and the failed client-websocket read

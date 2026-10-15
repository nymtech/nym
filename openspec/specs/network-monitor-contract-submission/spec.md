# network-monitor-contract-submission Specification

## Purpose
The network-monitor orchestrator's submission of each epoch's measurements to the performance contract: when it is switched on and the startup check that the account may submit, which epoch is submitted and in which materialisation pass, what a node's submission carries, the per-test-kind coverage guard that turns a fault on the measuring side into missing data rather than zeros, the ascending chunked submission resumed from the contract's own cursor, and the signals that make an incomplete epoch or a guard trip visible.

## Requirements
### Requirement: Submission is on exactly when enabled in the configuration, and gated on authorisation at startup

The orchestrator SHALL submit to the performance contract exactly when its configuration enables submission, which is off by default. With submission off it MUST run as it does without this capability and submit nothing. With submission on, startup MUST verify, directly after the network-monitors contract authorisation check, that the orchestrator's account is an authorised network monitor in the performance contract (its `NetworkMonitor` query returns an entry for the account), retrying with the same attempt budget and delay as that check, and MUST abort before any task starts once the budget is exhausted.

Submitting is an operator's decision, taken once the orchestrator's account is authorised in the contract and funded, so it is switched explicitly rather than inferred from the network details, which are assumed to carry the performance contract's address. Checking at startup turns a missing authorisation into a failed deploy rather than an epoch of rejected transactions, and retrying covers the admin's authorisation transaction landing after the deploy.

#### Scenario: With submission off nothing is submitted
- **WHEN** the orchestrator's configuration leaves submission off
- **THEN** it starts without checking the performance contract and never submits to it

#### Scenario: An unauthorised account aborts startup
- **WHEN** submission is enabled and the contract still lists no network monitor for the orchestrator's account once the retry budget is exhausted
- **THEN** startup aborts and no task begins

#### Scenario: An authorisation landing within the budget lets startup continue
- **WHEN** the contract lists the orchestrator's account as a network monitor on a retry within the budget
- **THEN** startup continues

### Requirement: Only the epoch in progress is submitted, in the pass that materialised it

The orchestrator SHALL submit only for the epoch in progress, as the last step of the materialisation pass. The epoch MUST be the one named by the interval reading that pass materialised against, and the submission MUST run only when that pass materialised both the aggregates and the config scores without error. An epoch being backfilled MUST NOT be submitted. Before submitting, the pass SHALL read the mixnet's epoch status and MUST NOT submit once the epoch's transition has begun; a pass that skips its submission for that reason has not failed, and a pass that cannot read the status has failed.

The contract accepts measurements only for the current epoch, and seals it once its transition begins, so a past epoch can never be submitted. Taking the epoch from the reading the figures were materialised against means the submitted figures are exactly those filed under that epoch, and requiring both steps to have succeeded means a submission is never built from half of an epoch's figures. A transition can stall for hours, during which every submission for the sealed epoch would fail, so checking the status first keeps a stalled transition from turning into an error every minute. A status that cannot be read is a failure rather than a skip, because a pass that has not failed looks again only at the epoch's end, so one failed read as the epoch begins would lose its submission.

#### Scenario: An epoch is submitted as it begins
- **WHEN** a pass materialises the aggregates and config scores of the epoch that has just begun
- **THEN** the same pass submits them under that epoch's id

#### Scenario: Backfilled epochs are not submitted
- **WHEN** the orchestrator restarts having missed two epoch transitions, and its first pass backfills both missed epochs and materialises the one in progress
- **THEN** only the epoch in progress is submitted

#### Scenario: A failed step prevents the submission
- **WHEN** a pass fails to materialise the config scores
- **THEN** it submits nothing, and the submission waits for a pass in which both steps succeed

#### Scenario: A stalled transition sends nothing and is not a failure
- **WHEN** a pass reads the epoch in progress while its transition has already begun, with part of its submission still unsent
- **THEN** the pass sends nothing, reports no failure, and the epoch stays visible as incomplete

### Requirement: A node is submitted with measured values only, with a config score and at least one of liveness or stress

Each submitted node SHALL carry only the figures the orchestrator materialised for that epoch: its config score, its liveness (the liveness test kinds the coverage guard kept, combined as served), and its stress, each converted to a percent. A figure the orchestrator does not hold MUST be omitted, never submitted as zero. A node SHALL be submitted only with a config score and at least one of liveness or stress.

An aggregate exists only where runs exist, and a run that failed already scores zero, so an absent figure means unmeasured rather than broken. The contract uses a node's bundle at the epoch whenever one exists, even one it cannot score, so a bundle without a config score, or with nothing but a config score, would also block the fallback to that node's last scored epoch.

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

### Requirement: A test kind without enough reachable coverage is omitted for every node

For each test kind (mixnode liveness, gateway liveness, stress), the orchestrator SHALL compute coverage as the share of eligible nodes with a reachable aggregate of that test kind for the epoch. A node is eligible when it is described and declares the role the scheduler assigns that test kind to (mixnode liveness and stress: mixnode; gateway liveness: gateway). An aggregate is reachable when it scores above zero. A test kind whose coverage is below the configured threshold, 0.5 by default, MUST be omitted from every node's submission for the epoch, before the liveness test kinds are combined. The rule on what a node carries then applies to the remaining values, so when every test kind is omitted nothing is submitted for the epoch. A threshold outside `[0, 1]` MUST be rejected at startup.

A fault on the measuring side records failures, which score zero and look like a network-wide outage. Omitting the test kind turns broken measurements into missing ones: the contract renormalises over the kinds still present, and with nothing submitted it falls back to each node's last scored epoch. Coverage is computed per test kind because the mixnode and gateway liveness probes fail independently, and a broken gateway probe could otherwise hide behind the mixnodes' combined liveness. The default matches the threshold below which nym-api drops a component. Eligibility follows the scheduler, so a node a test kind never tests cannot count against it.

#### Scenario: A test kind below the threshold is omitted for every node
- **WHEN** 30% of the eligible nodes have a stress aggregate above zero
- **THEN** no node's submission for the epoch carries stress

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
- **WHEN** all three test kinds fall below the threshold
- **THEN** nothing is submitted for the epoch

#### Scenario: A gateway does not count against stress coverage
- **WHEN** a described node declares only the gateway role
- **THEN** it is not eligible for stress or mixnode liveness, and its missing aggregates of those test kinds do not lower their coverage

#### Scenario: A measured zero is not reachable
- **WHEN** an eligible node's mixnode-liveness aggregate is 0.0
- **THEN** it counts as eligible but not reachable for mixnode-liveness coverage

#### Scenario: A threshold outside the unit interval is rejected
- **WHEN** the coverage threshold is configured as 1.5
- **THEN** startup fails with an error naming the value

### Requirement: Nodes are submitted in ascending order, in chunks, resuming from the contract's cursor

The orchestrator SHALL submit an epoch's nodes in ascending node-id order through `BatchSubmit` transactions of at most the configured batch size, 200 nodes by default, one after another. Before submitting it SHALL read its own submission cursor from the contract. When the cursor is at the epoch being submitted, only nodes above the cursor's node SHALL be submitted. A cursor at an earlier epoch leaves every node to submit. A cursor at a later epoch, or no entry for the account, MUST fail the submission. A chunk that fails MUST end the submission without attempting the chunks after it, leaving them to the next pass, which resumes from the cursor again. When no node is left above the cursor, nothing SHALL be sent.

The contract accepts a strictly ascending `(epoch, node)` stream per monitor and moves the cursor to the last node of each landed batch, including a node it skipped as not bonded. So the cursor alone tells a retried or restarted pass exactly what has landed, including a transaction that landed after the client gave up on it, with no local record that could disagree with the chain. A cursor ahead of the epoch being submitted means this orchestrator's interval reading is behind the chain.

#### Scenario: Chunks go out in ascending order
- **WHEN** 450 nodes are to be submitted with the default batch size
- **THEN** three `BatchSubmit` transactions are sent in order, holding 200, 200 and 50 nodes in ascending node id

#### Scenario: A restart resumes after the cursor
- **WHEN** the orchestrator restarts mid-epoch with the cursor at node 400 of the epoch in progress
- **THEN** it submits only the nodes above 400 for that epoch

#### Scenario: A failed chunk leaves the rest for the next pass
- **WHEN** the second of three chunks fails
- **THEN** the third is not attempted, and the next pass submits from the cursor left by the first

#### Scenario: A completed epoch sends nothing
- **WHEN** the cursor is at the last node of the epoch's plan
- **THEN** no transaction is sent and the epoch counts as completely submitted

#### Scenario: A cursor ahead of the reading fails the submission
- **WHEN** the cursor names a later epoch than the one being submitted
- **THEN** nothing is sent and the submission fails, to be retried with a new reading

### Requirement: An incomplete epoch and a guard trip are visible, and a missing node's reason is logged

The orchestrator SHALL expose, while submission is on, the epoch in progress its last pass read, set as soon as the pass reads the interval and before it materialises anything, and the newest epoch with nothing left to send, so that an epoch left incomplete is visible while it lasts, and SHALL expose, per test kind, whether the coverage guard omitted it for that epoch. A node that fails to be described SHALL be logged at `info` with its node id and the step that failed.

An incomplete epoch has to alert rather than fall back silently to last-known values, and a guard trip means the measuring side is broken. During the shadow period, every node nym-api scores but the contract does not has to be matched to a reason before operators are told that describing successfully is a reward requirement; that is a search per node, which a log line with the failing step serves. Completeness is tracked per epoch rather than per chunk, because a retried or restarted pass re-chunks only what is left. The epoch is exposed as the pass reads it rather than when the submission runs, because the submission runs only after both materialisation steps succeeded and only while the epoch is in progress: a pass that fails before submitting, or first reads the epoch during a stalled transition, must still leave the epoch visible as incomplete.

#### Scenario: An incomplete epoch is visible
- **WHEN** a chunk keeps failing until the epoch's transition begins
- **THEN** the exposed epoch stays ahead of the newest completed epoch

#### Scenario: A pass failing before its submission is visible
- **WHEN** the config step fails in every pass of an epoch
- **THEN** the exposed epoch stays ahead of the newest completed epoch

#### Scenario: A restart restores completeness from the chain
- **WHEN** the orchestrator restarts after its epoch's submission had fully landed
- **THEN** its first pass finds nothing above the cursor and exposes that epoch as complete

#### Scenario: A guard trip is visible
- **WHEN** the guard omits gateway liveness for an epoch
- **THEN** the orchestrator exposes gateway liveness as omitted for that epoch, and mixnode liveness and stress as not omitted

#### Scenario: A describe failure names its step
- **WHEN** a gateway answers every describe read except the client websocket
- **THEN** an `info` log line names the node and the failed client-websocket read

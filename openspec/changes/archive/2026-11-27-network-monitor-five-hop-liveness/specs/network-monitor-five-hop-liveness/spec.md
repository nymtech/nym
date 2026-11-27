## ADDED Requirements

### Requirement: The route test measures active mixnodes with traffic a mixnode cannot tell from a client's

The subsystem SHALL test the mixnodes of the current active set by sending packets along full client routes: from a client session at a gateway, through one mixnode of each of the three layers, back to the same gateway and on to the same client. This is the route a client's loop cover traffic takes, and the score it produces for a mixnode is that mixnode's five-hop liveness.

Every test packet MUST be built by the packet builders the client stack itself uses for loop cover traffic, so that its size, its sphinx format, its framing, the way its per-hop delays are drawn and its SURB-ack are a client's by construction and cannot drift from it. The mixnode of each layer MUST be drawn uniformly at random from that layer, afresh for every packet and independently for the packet and for its SURB-ack, from a cryptographically secure random source that is not seeded from anything predictable. Packets MUST be sent as a Poisson stream at the combined rate of the configured main and cover streams, which default to a client's.

The test measures mixnodes only. A gateway is the door the traffic enters and leaves through: carrying sessions MUST NOT give it a five-hop liveness, and does not affect its liveness. The test does not attempt to be indistinguishable to the gateway it enters through.

The one-hop tests cannot serve this purpose, because their packets arrive from an authorised agent's address and return to it, which a mixnode sees. A packet of this test reaches a mixnode from a gateway or another mixnode and leaves for another mixnode or a gateway, as client traffic does.

#### Scenario: A test packet crosses one mixnode of each layer
- **WHEN** a session sends a test packet
- **THEN** the packet's route is a layer-1, a layer-2 and a layer-3 mixnode of the active set followed by the session's own gateway, and its recipient is the session's own client address

#### Scenario: Routes are drawn afresh for every packet and for its ack
- **WHEN** a session sends two test packets
- **THEN** each packet's three mixnodes and each ack's three mixnodes are drawn independently of one another

#### Scenario: A gateway is not scored by the route test
- **WHEN** five-hop liveness is computed for an epoch in which a gateway carried sessions
- **THEN** carrying them stores no five-hop liveness for that gateway and changes nothing in its submitted liveness

### Requirement: A route session is an ordinary client session without the monitor exemption

Each gateway of a route run SHALL be used through one client session of its own, opened with a client identity generated for that session and never with the agent's announced identity, so that the gateway grants it no monitor exemption. The session MUST register with the gateway and obtain bandwidth the way a client configured without tickets does. The active set MUST come from nym-api as a client learns it, with the client stack's default topology settings, and MUST NOT be taken from the orchestrator. It MUST be read once per route run, when the run starts, and every session of the run MUST use that one reading for every packet it sends, so that the sessions of a run name one epoch and draw from the same layers.

The session SHALL reach its gateway exactly as a default client does. It MUST derive the gateway's websocket endpoint from the run's reading of the topology, through the client stack's own selection of a gateway's listeners with its default settings, and MUST NOT be given an address by the orchestrator. Under those defaults that is the gateway's hostname when it announces one and otherwise its first announced address, on the plain websocket port, with the next of them as a fallback.

Because the client's default topology settings apply, a mixnode whose performance as nym-api reports it is below the client's default minimum is left out of the topology, as it is for clients. A session does not route through it, so that mixnode gets no test message from the session and, if no session of an epoch crosses it, no five-hop liveness for that epoch.

A session SHALL send for a time drawn uniformly at random between two thirds and four thirds of the configured sending duration, wait for packets and acks still in flight until every one is back or the configured straggler time has passed, ask the gateway to forget the client, and close. The draw exists so that a route session has no fixed length for a mixnode to recognise.

A session that cannot connect, register, obtain bandwidth or build routes (the gateway is missing from the client's topology, or a layer is empty) MUST report an error and no records. The other sessions of the run are unaffected. When the run cannot read the active set, no session starts, and every session of the run is submitted with that error.

Working without the exemption is what lets the same test run as a plain client by anyone, and what a later move to real tickets needs.

#### Scenario: A session uses a fresh identity
- **WHEN** an agent runs two sessions, in one run or in two
- **THEN** each registers under an identity of its own, and neither is the agent's announced identity

#### Scenario: A session reaches its gateway as a client does
- **WHEN** a session's gateway announces a hostname
- **THEN** the session connects to that hostname on the gateway's plain client websocket port, and tries the gateway's first announced address if that fails

#### Scenario: A mixnode clients avoid is not routed through
- **WHEN** an active mixnode's performance at nym-api is below the client's default minimum
- **THEN** no test packet or ack of the session crosses it

#### Scenario: A refused bandwidth claim fails the session alone
- **WHEN** one gateway of a run refuses the session's bandwidth claim
- **THEN** that session reports an error and no records, and the run's other sessions send as usual

#### Scenario: The gateway is asked to forget the client
- **WHEN** a session finishes
- **THEN** it asks its gateway to forget the client before closing, so the gateway keeps no keys, bandwidth or stored messages for it

#### Scenario: Session length varies
- **WHEN** the sending duration is configured as 45 seconds
- **THEN** each session sends for a time between 30 and 60 seconds, drawn for that session

#### Scenario: The wait ends once everything is back
- **WHEN** every payload and ack of a session has come back before its straggler time has passed
- **THEN** the session stops waiting at once

### Requirement: A route run is handed out on its own cadence, ahead of every per-node test

A route run SHALL be due when five-hop liveness is enabled, no route run is in flight, and either no run has ever had its sessions submitted or the last such run started at least the configured interval ago. When an agent requests work while a run is due, the orchestrator MUST hand it that run before choosing a test kind.

A run is in flight from the moment it is handed out until its sessions are submitted or its timeout passes. At most one run is in flight. A run whose timeout passes without a submission MUST be due again at once, without waiting for the interval.

The run comes first because it is tied to the clock: a backlog of per-node work, which the scheduler otherwise drains first, would push it past its slot.

#### Scenario: A due run is handed out before an overdue kind
- **WHEN** an agent requests work while a route run is due and a test kind has overdue nodes
- **THEN** the agent receives the route run

#### Scenario: A run that is not due leaves the request to the kinds
- **WHEN** an agent requests work less than the interval after the last submitted run started
- **THEN** it is assigned a test kind as if the route test did not exist

#### Scenario: Only one run is in flight
- **WHEN** a second agent requests work while a route run is in flight
- **THEN** it is not given a route run

#### Scenario: An abandoned run is due again
- **WHEN** a run's timeout passes without its sessions being submitted
- **THEN** the next agent to request work is given a new run

#### Scenario: Disabled five-hop liveness hands nothing out
- **WHEN** five-hop liveness is disabled
- **THEN** no route run is ever handed out

### Requirement: A run's gateways are drawn at random from the gateways fit to carry it

A route run SHALL carry up to the configured number of gateways, drawn uniformly at random and without repetition from the gateways that are fit to carry it.

A gateway is a candidate when all of these hold: it has a description reporting the entry-gateway role; that description says it does not enforce zk-nyms; and it has a gateway liveness run. A gateway whose enforcement flag has not been read, or which has no gateway liveness run, is not a candidate.

The bar a candidate's most recent gateway liveness run must reach SHALL be chosen at each draw, a run's score being the one its aggregates are built from. When at least the configured share of the candidates, 40% by default, scored at least the configured minimum, 1.0 by default, the bar is that minimum. Otherwise the bar is the configured fallback minimum, 0.98 by default. Nothing about the choice is stored between draws, and tests in flight play no part in it.

A gateway is fit when it is a candidate, its most recent gateway liveness run reached the bar, and no test is in flight against it. The orchestrator MUST expose how many gateways were fit at the latest draw.

The share keeps the strict bar for as long as meeting it is a gateway's normal state, and widens the draw when it is not, so that the draw never narrows to a small and therefore predictable set of gateways.

The draw MUST be random rather than by staleness, so that a mixnode cannot predict which gateways will carry test traffic next. When fewer gateways are fit than the configured number, the run carries all of them. When none is fit, no run is handed out and the run stays due.

For each gateway the run MUST carry its node id and its identity key, and nothing else: a session finds what it needs to reach the gateway in the topology it reads, as a client does.

A route run MUST NOT take an in-flight row for its gateways. A per-node test may therefore start on a gateway while a session runs through it.

#### Scenario: Most candidates meet the minimum
- **WHEN** 60% of the candidates scored 1.0 on their most recent gateway liveness run, with the defaults
- **THEN** the bar is 1.0, and a gateway whose most recent run scored 0.99 is not drawn

#### Scenario: Too few candidates meet the minimum
- **WHEN** 30% of the candidates scored 1.0 on their most recent gateway liveness run, with the defaults
- **THEN** the bar is 0.98, and a gateway whose most recent run scored 0.99 may be drawn

#### Scenario: A gateway below the fallback is never drawn
- **WHEN** a gateway's most recent gateway liveness run scored 0.9
- **THEN** it is not among the gateways of any route run, whichever bar is in use

#### Scenario: An enforcing gateway is not drawn
- **WHEN** a gateway's description says it enforces zk-nyms
- **THEN** it is not among the gateways of any route run

#### Scenario: A gateway with no liveness run yet is not drawn
- **WHEN** a gateway has never had a gateway liveness run
- **THEN** it is not among the gateways of any route run

#### Scenario: A gateway under test is not drawn
- **WHEN** a gateway has a test of any kind in flight
- **THEN** it is not among the gateways of a run handed out at that moment

#### Scenario: The draw does not follow staleness
- **WHEN** runs are handed out one after another while more gateways are fit than a run carries
- **THEN** a gateway used by one run may be drawn again by the next, and a gateway never used has no priority

#### Scenario: Few fit gateways still make a run
- **WHEN** one gateway is fit and a run carries up to two
- **THEN** the run carries that one

#### Scenario: No fit gateway means no run
- **WHEN** no gateway is fit
- **THEN** the request is answered as if no run were due, and the run stays due

### Requirement: Every test packet and its ack are recorded in the order sent

For every test packet a session sends, it SHALL record, in the order sent: when it was sent, the three mixnodes of the packet's route, the three mixnodes of its ack's route, whether the packet's payload came back, and whether its ack came back. A payload or ack counts as back only if it arrived before the session's straggler wait ended, and one that arrives more than once counts once.

A returning payload and a returning ack MUST be matched to their record by an identifier the session placed in the packet's encrypted payload and in the ack, which only the session can read. Nothing outside the session may be consulted to learn a route or an outcome: in particular there MUST be no process-wide registry of sent or received packets.

#### Scenario: A delivered packet and ack are recorded as back
- **WHEN** a packet's payload and its ack both return before the straggler wait ends
- **THEN** its record names both routes and marks the payload and the ack as back

#### Scenario: A late arrival is not counted
- **WHEN** a packet's payload returns after the straggler wait has ended
- **THEN** its record marks the payload as not back

#### Scenario: A repeated arrival counts once
- **WHEN** the same payload is pushed to the session twice
- **THEN** its record marks the payload as back, once

### Requirement: A session's records are submitted as one binary batch, and the orchestrator stores it as received

A session's records SHALL be packed into one binary batch that states the gateway's node id and the mixnet epoch of the active set the session used, and then one fixed-width record per packet in the order sent. A record names its mixnodes by their node ids.

The agent SHALL submit a run once, with one entry per gateway of the run: the session's batch, or its error. The orchestrator MUST accept a submission only for the run that is in flight, and MUST reject it, storing nothing, when five-hop liveness is switched off, when the run is unknown, already submitted or past its timeout, when the epoch its batches name has already been scored, when an entry names a gateway the run was not given, when two entries name the same gateway, when a batch cannot be read, or when a batch's header names another gateway than its entry. A rejected run stays in flight until its timeout passes. A submission without an entry for some of the run's gateways is stored as it is.

The orchestrator MUST store each accepted batch exactly as it was received, beside the gateway, the epoch and the record count read from it and whether the discard rule keeps the session, judged against the other sessions of its run as the submission arrives, and MUST NOT expand it into one stored row per packet. Stored sessions are evicted at the age test runs are.

#### Scenario: A run's sessions are stored
- **WHEN** an agent submits a run in flight with two entries
- **THEN** two sessions are stored, each with its batch as received, and the run is no longer in flight

#### Scenario: A late submission is rejected
- **WHEN** a run is submitted after its timeout has passed
- **THEN** the submission is rejected and nothing is stored

#### Scenario: A run whose epoch was scored is rejected
- **WHEN** a run in flight is submitted with batches naming epoch E-1 after the five-hop liveness filed under epoch E was materialised
- **THEN** the submission is rejected and nothing is stored

#### Scenario: An unreadable batch is rejected
- **WHEN** a submitted batch ends partway through a record
- **THEN** the submission is rejected and nothing is stored

#### Scenario: A failed session is stored without records
- **WHEN** a submitted entry carries an error and no batch
- **THEN** a session is stored for that gateway with its error and no records

### Requirement: Records become test messages by two rules about the ack

Before scoring, each record SHALL be turned into test messages, a test message being the time it was sent, the three mixnodes on its path in path order, and whether it came back. The session's gateway is not on a test message as the gateway: a gateway is never blamed for a loss, and a session whose gateway is suspected is discarded whole instead (see the next requirement). A node that is the session's gateway and also sits in a mix layer of the epoch, which a client allows, appears only on the test messages on which it was drawn as a mixnode, and is scored there as one. Two rules apply. An ack is a test message only if its packet reached the gateway, which is known when the payload or the ack came back. And when the ack came back while the payload did not, the three mixnodes of the test packet delivered it, so the packet counts as delivered.

A record therefore yields:

- payload back, ack back: the packet's three mixnodes succeeded, and the ack's three mixnodes succeeded;
- payload back, ack not back: the packet's three mixnodes succeeded, and the ack's three mixnodes failed;
- payload not back, ack back: the packet's three mixnodes succeeded, and the ack's three mixnodes succeeded;
- neither back: the packet's three mixnodes failed, and there is no test message for the ack.

Every test message of a record carries the record's send time. These rules belong to the step that turns records into test messages. A scoring strategy receives test messages and knows nothing of acks or gateways.

#### Scenario: Both back
- **WHEN** a record's payload and ack both came back
- **THEN** it yields a successful test message for the packet's three mixnodes and a successful one for the ack's three mixnodes

#### Scenario: Only the payload back
- **WHEN** a record's payload came back and its ack did not
- **THEN** it yields a successful test message for the packet's three mixnodes and a failed one for the ack's three mixnodes

#### Scenario: Only the ack back
- **WHEN** a record's ack came back and its payload did not
- **THEN** it yields a successful test message for the packet's three mixnodes and a successful one for the ack's three mixnodes

#### Scenario: Neither back
- **WHEN** neither the payload nor the ack of a record came back
- **THEN** it yields a failed test message for the packet's three mixnodes and none for the ack

#### Scenario: The gateway is on a test message only as a mixnode
- **WHEN** a session's records are turned into test messages
- **THEN** every test message names the three mixnodes of its path, and the session's gateway appears only where it was drawn as one of them

### Requirement: A session whose gateway is suspected is discarded before scoring

A gateway is assumed reliable and is not a suspect in any loss. To keep a gateway's faults, or a fault on the measuring side, from being charged to mixnodes, the orchestrator SHALL discard a whole session, before scoring, when either of these holds. The rule is applied when the run is submitted, since a run's sessions arrive together, and its verdict is stored with each session. A session's loss rate is the share of its test messages that were lost.

- The session lost more than half of its test messages.
- The session ran beside another session of the same route run, and its loss rate is higher than that session's by more than the configured margin, 5 percentage points by default. Only the worse of the two is discarded.

A session with no other session of its run to be compared with is subject to the first test only. A discarded session contributes no test message to any epoch. Its batch MUST stay stored, and it is counted among the epoch's discarded sessions. A session stored with an error has no test messages, so it contributes none either, and it too is counted among the epoch's discarded sessions.

The two sessions of a run draw their mixnodes from the same layers at the same moment, so the mixnodes cost both the same loss on average and a clear gap between them belongs to one gateway or to the path to it. Losing more than half would need about a fifth of all mixnodes to be dead, so that too is taken as the gateway's or the measuring side's doing.

Two things this does not catch are accepted: two gateways of one run that are bad by a similar amount, and a gateway whose extra loss stays inside the margin. In both cases the loss is shared among mixnodes.

#### Scenario: A session that lost most of its messages is discarded
- **WHEN** a session lost 60% of its test messages
- **THEN** it is discarded, whatever the other session of its run lost

#### Scenario: The worse of two unequal sessions is discarded
- **WHEN** the two sessions of a run lost 2% and 9% of their test messages
- **THEN** the session that lost 9% is discarded and the other is scored

#### Scenario: Two sessions that agree are both kept
- **WHEN** the two sessions of a run lost 8% and 10% of their test messages
- **THEN** both are scored

#### Scenario: A lone session is judged on its own
- **WHEN** a run has one session with records and it lost 12% of its test messages
- **THEN** it is scored

#### Scenario: A discarded session stays stored
- **WHEN** a session is discarded
- **THEN** its batch is still stored, and its epoch's count of discarded sessions rises by one

### Requirement: Five-hop liveness is computed by a replaceable scoring strategy, initially the fail-sequence rule

Scoring SHALL sit behind one interface with two inputs and one output. The inputs are the test messages, ordered by the time they were sent, each with its send time, the nodes on its path in path order and whether it came back; and, for each node, the score of its most recent one-hop test, when it has one. The output is each node's five-hop liveness, for the nodes on the test messages that the strategy can score. Anything a strategy counts on the way, such as the first strategy's samples, stays inside it.

A strategy MUST NOT know about batches, sessions, epochs or storage, and the orchestrator MUST depend on the interface and not on a particular strategy, so that once another strategy has been worked out it replaces the first without touching anything else. Test messages are ordered by sending time because not every one is received.

The one-hop scores are given so that a strategy can recognise a node that is down or malfunctioning and avoid blaming the other nodes on the path of a message it dropped. How they are to change the blame is not yet specified. The first strategy therefore receives them and MUST NOT let them change its result.

The first strategy is the fail-sequence rule of the performance monitor specification. It keeps, per node, a count of positive samples, a count of negative samples and a fail sequence, all starting at zero. For a test message that came back, every node on it gains one positive sample and its fail sequence returns to zero. For one that did not, the fail sequence of every node on it grows by one, and then the nodes on it with the highest fail sequence share one negative sample equally. A node's score is its positive samples divided by the sum of its positive and negative samples. A node with neither has no score.

#### Scenario: The specification's worked example
- **WHEN** the test messages are, in order: m1 m2 m3 back; m4 m5 m6 back; m1 m5 m6 lost; m4 m5 m3 lost; m1 m2 m6 back; m1 m5 m3 lost (the specification's example with its gateways left out)
- **THEN** the scores are m1 0.857, m2 1, m3 1, m4 1, m5 0.3 and m6 0.857, m5 holding 1 positive and 2.333 negative samples

#### Scenario: A first failure is shared by everyone on the path
- **WHEN** a test message is lost and none of its three mixnodes had failed since its last success
- **THEN** each of the three gains a third of a negative sample

#### Scenario: A repeat offender takes the whole sample
- **WHEN** a test message is lost and one of its mixnodes has a longer fail sequence than the others on it
- **THEN** that mixnode gains one whole negative sample and the others gain none

#### Scenario: The first strategy does not use the one-hop scores
- **WHEN** the same test messages are scored once with one-hop scores for their nodes and once with none
- **THEN** the two results are identical

### Requirement: Five-hop liveness is materialised as an epoch begins, from the sessions of the epoch that just ended

As an epoch begins, in the pass that materialises its aggregates, the orchestrator SHALL compute the five-hop liveness filed under that epoch from the stored sessions whose batch names the epoch that just ended, and from no others. It MUST first leave out the sessions the discard rule did not keep, then unpack the rest into test messages and order them by the time they were sent (the test messages of one record keeping their order, and equal times ordered by gateway node id). It MUST give the strategy, as each mixnode's most recent one-hop score, the score of that mixnode's most recent mixnode liveness run, and none for a mixnode that has no such run. It MUST store, for every mixnode the strategy gave a five-hop liveness, that value, marked as measured. A mixnode the strategy could not score has no measured value: when its newest earlier measured value is recent enough to be carried forward, as the next requirement defines, the carried value is stored under the epoch, marked as carried forward, and otherwise nothing is stored for it. Only a measured value is carried forward.

Each epoch is scored on its own: whatever a strategy keeps between test messages, such as a fail sequence, starts afresh. Within an epoch the test messages of all its runs form one list, and nothing is reset between runs: a fail sequence a mixnode ended one run with is the one it starts the epoch's next run with. The stored values MUST be written once and never recomputed, and a run submitted after its epoch was scored is rejected, so it is never counted. A stored value is deleted once its epoch began longer ago than the aggregate retention, in the sweep that evicts the aggregates. An epoch being backfilled is computed the same way, from whichever of the preceding epoch's sessions are still stored. A mixnode named by a batch whose bond the registry has not recorded yet is stored like any other, and its value applies once a refresh records the bond.

Taking the epoch from the batch rather than from the clock ties a session to the active set it actually used. The measured value, like every aggregate, describes the epoch before the one it is filed under.

#### Scenario: The value filed under an epoch comes from the epoch before
- **WHEN** five-hop liveness is materialised as epoch E begins
- **THEN** it is computed from the sessions whose batch names epoch E-1

#### Scenario: A late batch is not counted
- **WHEN** a session naming epoch E-1 is submitted after the five-hop liveness filed under E was materialised
- **THEN** the submission is rejected, the values filed under E are unchanged, and the session contributes to no epoch

#### Scenario: An epoch with no sessions has no measured five-hop liveness
- **WHEN** no stored session names the epoch that just ended
- **THEN** no measured five-hop liveness is filed under the epoch that begins, only values carried forward

#### Scenario: Fail sequences do not carry across epochs
- **WHEN** a mixnode ended one epoch on a run of failures
- **THEN** its fail sequence starts at zero when the next epoch's sessions are scored

#### Scenario: Fail sequences carry across the runs of an epoch
- **WHEN** a mixnode's last test message in one run of an epoch was lost, and its first test message in the epoch's next run is lost too, on a path whose other mixnodes had not failed since their last success
- **THEN** it takes the whole negative sample of the second, its fail sequence being longer than theirs

#### Scenario: The strategy is given the most recent one-hop scores
- **WHEN** a mixnode on an epoch's test messages has mixnode liveness runs stored
- **THEN** the strategy receives the score of the most recent of them for that mixnode, and receives none for a mixnode with no such run

### Requirement: An epoch without usable sessions is unmeasured, and the session counts show it

There SHALL be no guard over an epoch's scored values. An epoch for which no session is stored, or whose sessions were all discarded, simply has no measured five-hop liveness, and every mixnode's last measured value is carried forward as for any other unmeasured epoch. The orchestrator MUST expose, per epoch, how many sessions it kept and how many it discarded, so that a missing or thin epoch is visible and its cause can be read off.

A guard is not needed because a kept session is never thin: it delivered at least half of its test messages and agrees with the other session of its run, and even the shortest session puts every mixnode on dozens of test messages. A session that fails on the measuring side or at its gateway is discarded before scoring, so it cannot zero the mixnodes it crossed.

#### Scenario: An epoch with every session discarded has no measured five-hop liveness
- **WHEN** every session naming an epoch was discarded
- **THEN** no measured five-hop liveness is filed for it, and the counts show zero sessions kept

#### Scenario: One failed session among several does not spoil the epoch
- **WHEN** one session of an epoch returned nothing and the others returned nearly everything
- **THEN** that session is discarded and five-hop liveness is stored for the epoch from the others

#### Scenario: The counts are served per epoch
- **WHEN** an epoch had twenty-two sessions kept and two discarded
- **THEN** the orchestrator exposes both numbers for that epoch

### Requirement: A mixnode's submitted liveness is its one-hop liveness times its five-hop liveness

The orchestrator SHALL multiply the liveness it is about to submit for a node by that node's five-hop liveness filed under the epoch being submitted, before converting it to a percent. A node's five-hop liveness filed under an epoch is:

- the value measured for it, when there is one;
- otherwise, when the node's newest earlier measured value was filed at most the configured number of epochs before, 48 by default, `1 - e^(-r * d) * (1 - value)`, where `d` is the number of epochs between the two and `r` the configured decay rate, 0.1 by default;
- otherwise none, and its liveness is submitted as it is, as if its five-hop liveness were 1.

The second case carries a mixnode's last measured value forward and lets it fade, with the defaults about half after 7 epochs and a tenth after 24, so that a mixnode that routes badly while active does not return to full weight the moment it leaves the active set. Nothing is tested outside the active set. A node the route test has never measured, which includes every gateway-only node, has none.

The value belongs to the node, not to the role it holds. A node measured as a mixnode keeps its carried-forward value in a later epoch in which it serves as a gateway, and the liveness submitted for it in that epoch is multiplied by it. Leaving the mixnode role is one way of leaving the active set, and it must not restore full weight any faster than the others.

The same value scales the liveness runs the orchestrator posts to nym-api, as `nym-network-monitor` specifies, taken for the newest epoch the orchestrator has materialised, so that nym-api's own liveness figure includes five-hop liveness for as long as that stream exists.

While five-hop liveness is disabled nothing is filed under the epochs that begin, so from the next epoch no node's liveness is multiplied. The values filed under the epoch in progress when it was disabled apply until that epoch ends.

#### Scenario: A measured mixnode
- **WHEN** a mixnode has a one-hop liveness of 1.0 and a five-hop liveness of 0.9 measured and filed under the epoch being submitted
- **THEN** its submitted liveness is 90%

#### Scenario: A mixnode that left the active set
- **WHEN** a mixnode has no value measured for the epoch being submitted, and its newest measured value, 0.6, was filed 7 epochs before, with the default limit and rate
- **THEN** its five-hop liveness is about 0.80

#### Scenario: A carried value follows a node into a gateway role
- **WHEN** a node's newest measured value, 0.6, was filed 3 epochs before the epoch being submitted, with the default limit and rate, and in that epoch it serves as a gateway with a liveness of 1.0
- **THEN** its five-hop liveness is about 0.70 and its submitted liveness is 70%

#### Scenario: An old value no longer counts
- **WHEN** a mixnode's newest measured value was filed 49 epochs before the epoch being submitted, with the default limit
- **THEN** it has no five-hop liveness and its submitted liveness is its one-hop liveness

#### Scenario: A node never measured
- **WHEN** a node has no measured five-hop liveness at all
- **THEN** it has no five-hop liveness and its submitted liveness is its one-hop liveness

#### Scenario: Disabling removes it from the next epoch
- **WHEN** five-hop liveness is disabled while values are stored
- **THEN** from the next epoch every node's submitted liveness is its one-hop liveness

### Requirement: The measured five-hop liveness is served beside the other aggregates

The orchestrator's aggregates API SHALL carry, for a node and an epoch, a five-hop liveness entry of its own holding the stored value. The entry MUST be the value measured and filed under that epoch, not a value carried forward, and MUST be absent when none is filed under it.

#### Scenario: A measured mixnode's value is served
- **WHEN** a consumer requests the aggregates of a mixnode for an epoch that has a five-hop liveness filed under it
- **THEN** the response carries a five-hop liveness entry with the measured value

#### Scenario: An unmeasured node has no entry
- **WHEN** a consumer requests the aggregates of a node with no five-hop liveness filed under that epoch
- **THEN** the response carries no five-hop liveness entry, rather than a carried-forward value or zero

### Requirement: An epoch's route sessions can be exported as test messages

The orchestrator SHALL serve the route sessions of a given mixnet epoch at `GET /v1/results/route-sessions/epoch/{mixnet_epoch}`, on its results surface and behind the `metrics_and_results_token`. The epoch is the one the sessions ran in, which is the epoch before the one their five-hop liveness is filed under. A session is of the epoch its batch names. A session stored with an error has no batch and names no epoch, so it is served with the other sessions of its run: the response carries every session of each run that has a session naming the epoch, and a run whose sessions all failed is in no epoch's response.

The response SHALL list the sessions in the order their runs started, and the sessions of one run by the gateway's node id. For each session it MUST carry the route run it belonged to, the gateway's node id, the session's error if it had one, whether the scoring keeps or discards it, and its test messages in the order sent: for each, the time it was sent, the node ids of the three mixnodes on its path in path order, and whether it came back. The test messages MUST be produced from the stored batch by the step that feeds the scoring, and whether the session is kept MUST be the verdict stored when its run was submitted. A discarded session's test messages are served like any other's. A session stored with an error has none.

The whole epoch SHALL be returned in one response. An epoch with no stored session, one whose sessions have been evicted included, returns an empty list.

An epoch's test messages and each node's one-hop runs, which the results surface already serves, are the whole input of a scoring strategy. Serving them lets a replacement strategy be tried on real data without access to the orchestrator's database.

#### Scenario: An epoch's sessions are exported
- **WHEN** a consumer requests the route sessions of an epoch in which two runs of two sessions each were stored
- **THEN** the response carries four sessions in the order their runs started, each run's two by gateway node id, each with its test messages in the order sent

#### Scenario: A discarded session is exported with its mark
- **WHEN** one session of the requested epoch is discarded by the discard rule
- **THEN** it is served with its test messages and marked as discarded, and the others are marked as kept

#### Scenario: A failed session has no test messages
- **WHEN** a session of a run of the requested epoch was stored with an error and no batch
- **THEN** it is served with its error and no test messages

#### Scenario: The export is what was scored
- **WHEN** the test messages of the kept sessions in the response are merged in send order
- **THEN** they are the test messages the scoring of that epoch was given

#### Scenario: An epoch with no sessions is an empty list
- **WHEN** a consumer requests the route sessions of an epoch for which none is stored
- **THEN** the response is an empty list, not an error

### Requirement: Five-hop liveness is governed by configuration with defined defaults

The orchestrator SHALL be configured with: an enable flag for five-hop liveness, expressible as an explicit value and defaulting to on; the interval between route runs, 5 minutes; the number of gateways a run carries, 2; a run's timeout, 3 minutes; the minimum score a gateway's latest gateway liveness run must reach, 1.0; the share of candidate gateways that must reach that minimum for it to be the bar, 40%; and the fallback minimum that is the bar otherwise, 0.98. Nearly every mainnet gateway's liveness is 1.0, so the minimum starts there. The share and the fallback are for when that stops being true, which is expected once gateway stress joins the filter. It SHALL also be configured with the margin by which a session's loss rate may exceed that of the other session of its run before it is discarded, 5 percentage points; the number of epochs a measured five-hop liveness is carried forward for after it was filed, 48; and the rate per epoch at which a carried value fades, 0.1. A carried value's weight is under one percent after 48 epochs at that rate. A decay rate that is negative or not a finite number MUST be rejected.

The agent SHALL be configured with: the nym-api url its route sessions read the active set from, which is required; a sending duration for a route session, 40 seconds; a straggler wait, 10 seconds; a timeout for a whole route run, 2 minutes, which has to sit inside the orchestrator's run timeout and, as with the other timeouts, is not validated against it; and, for network experiments, the average delays of a client's main and cover streams, whose combined rate a session sends at, and the average per-hop delays of test packets and of their acks. Each of these four defaults to the client stack's own default, read from its configuration rather than copied, so that unchanged they send a default client's traffic. The packet size is the client stack's default and is not configurable.

Every knob MUST be overridable by CLI flag or environment variable.

#### Scenario: Five-hop liveness can be disabled without a new build
- **WHEN** the orchestrator's five-hop liveness flag is set to off
- **THEN** no route run is handed out, no five-hop liveness is materialised, and from the next epoch submitted liveness is the one-hop liveness alone

#### Scenario: An agent without a nym-api url does not start
- **WHEN** the agent is started without a nym-api url
- **THEN** configuration construction fails

### Requirement: The route test can be run by hand without being a network monitor

The agent binary SHALL provide a `test-five-hop-liveness` command that takes a nym-api url, the gateways to use and the agent's route session settings with their defaults, runs one session through each as a route run does, and prints the scores computed with the same rules for test messages and the same scoring strategy the orchestrator uses. The gateways are given by node id, with their identity keys read from nym-api's gateways; when none is given, the command draws a number of them at random among nym-api's gateways whose performance reaches a minimum, 2 gateways at 99% by default. Before scoring, the command discards a suspected session as the orchestrator does, by a margin it takes as a setting of its own, 5 percentage points by default, and it prints each session's loss rate and whether it was kept. The command MUST need no orchestrator, no noise key and no on-chain authorisation.

For experiments, the command SHALL also take which mixnodes to route through: the active set by default, the rewarded set, or every node declaring the mixnode role. Nodes of the active set keep their layer, and any other selected node is placed in layer `node_id % 3 + 1`. A route run handed out by the orchestrator always uses the active set.

#### Scenario: A manual run prints scores
- **WHEN** `test-five-hop-liveness` is run with a nym-api url and two gateways
- **THEN** it runs a session through each and prints each node's five-hop liveness

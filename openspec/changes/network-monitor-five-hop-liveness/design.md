## Context

The network monitor has three test kinds, and every one is a probe against a single node: the agent sends packets through the node and straight back to itself. That isolates the node, so a loss can only be its own, but it also makes the traffic recognisable. A mixnode sees test packets arrive from an authorised agent's address and leave for it; a gateway sees a session opened under an identity the network monitors contract lists. A mixnode can therefore forward every measurement while dropping client packets, and still score perfectly.

The performance monitor specification (Confluence, "NIP-xx: network performance monitor") describes the test that closes this: measurement packets sent along full routes `G-m1-m2-m3-G` through the active set, a per-epoch dataset with one row per message, and a post-processing pass that assigns each lost message to the nodes most likely to have dropped it. This change builds that test into the monitor.

What shapes it:

- The active set is 60 mixnodes, 20 per layer. Clients may enter through any gateway, about 600 of them.
- A default client sends about 55 loop packets a second when idle (a main stream every 20 ms and a cover stream every 200 ms on average), each through one random mixnode per layer and back to its own gateway. Each is a regular-size sphinx packet with an exponential delay per hop, 15 ms on average, and it carries a SURB-ack that the gateway sends back through three more mixnodes. A gateway refuses a regular-size final-hop packet that has no SURB-ack.
- Every packet therefore gives up to two samples per layer: one client sending for a minute puts each mixnode on about 165 test packets and up to 165 acks.
- nym-node defaults to not enforcing zk-nyms, and a client configured without tickets claims free bandwidth from its gateway. Each gateway publishes whether it enforces.
- Results reach scoring by two paths today. The orchestrator posts every run's score to nym-api, which averages them over 24 hours into its own figure, and once per epoch it submits to the performance contract, which holds one whole-percent `liveness` per node per epoch. The epoch driver (#7398) removes the first path. This change lands before it, and before the directory switch (#7423) and gateway stress (#7442).
- Liveness carries no weight on mainnet yet.
- The orchestrator knows nodes from the mixnet contract's bonds and each node's own description. It does not know the active set, the layers, or (outside the materialiser) the epoch.
- The agent is a one-shot job with its own minimal gateway session code, which cannot claim bandwidth and exists to hold the monitor's exempt session.

## Goals / Non-Goals

**Goals:**

- Score active mixnodes on traffic they cannot tell from a client's, and fold that into their submitted liveness.
- Follow the performance monitor specification: its routes, its per-message dataset, its fail-sequence rule, last epoch only for active nodes, and its decay for nodes that have left the active set.
- Work as an ordinary client, without the monitor exemption, so the same test runs by hand against mainnet and can move to tickets later.
- Keep the scoring separate from everything else, so another strategy can replace it.
- Change nothing in the performance contract, nym-api or nym-node.

**Non-Goals:**

- Being indistinguishable to the gateway the traffic enters through. Wanted later; no approach is chosen.
- Scoring gateways by route traffic. They are the door, and keep their one-hop liveness.
- Testing mixnodes outside the active set by route. Nothing routes through them.
- Covering a whole epoch. Forty seconds of sending every five minutes is the starting point.
- Spending tickets. See Open Questions.
- The specification's two-day warm-up for newly bonded nodes.

## Decisions

### 1. The route test scores mixnodes only, and gateways are the door

A session enters and leaves through one gateway, as a client's loop traffic does. The gateway takes no part in the scoring: it is assumed reliable, it is never blamed for a loss, no five-hop liveness is stored for it, and its liveness stays the one-hop one. A session whose gateway is suspected is discarded whole (Decision 20).

A gateway can recognise the session whatever is done on the wire (Decision 2), so a five-hop liveness for it would protect against nothing the one-hop test does not already cover. Treating gateways as doors also frees their selection: a run can use a few good ones rather than having to visit all six hundred.

Because a test message names only its three mixnodes, a node that is one session's gateway and also sits in a mix layer in the same epoch is scored only on the test messages on which it was a mixnode.

Alternative considered: scoring every gateway through sessions of its own. It needs a session per gateway, most of the agent's time spent connecting, and a score built from one session every few hours.

### 2. A route session is an ordinary client, with no monitor exemption

Each session registers under an identity generated for it, never the agent's announced one, so the gateway grants no exemption. It obtains bandwidth as a client configured without tickets does, by claiming it, and asks the gateway to forget it when it is done, so gateways do not accumulate one client record per session (the gateway probe does the same).

This costs the use of gateways that enforce zk-nyms, which the run avoids (Decision 5). It buys three things: the test needs nothing from the network monitors contract, so anyone can run it by hand against mainnet, which is also how it gets tested; it is the starting point for tickets; and it takes one tell away from the gateway, although the agent's source address remains one.

Alternative considered: the agent's existing exempt session. It needs no bandwidth at all, and a packet sent over it is forwarded like a client's. But it ties the test to authorised agents and to an exemption that is meant to go.

### 3. A route run is its own unit of work, not a fifth test kind

A test kind is one probe against one node: a target leased by node id, a result made of interface counters, a score per run, staleness per node, an aggregate that is the mean of run scores. A route run fits none of that. It targets no node, its result is a list of packets, it has no score per run, and what it scores is not what it connects to.

So it gets its own small pieces: a `route_run` row for the cadence and the in-flight state, a `route_session` row per gateway with the batch, a table of five-hop liveness values beside the aggregates, as config score has, and its own entry in the work response. The existing kinds and everything keyed by kind are untouched, the per-kind result stream to nym-api included: it has nothing to post for a route run.

Alternative considered: `TestKind::RouteLiveness`. It would inherit the lease and submission paths for free, and then need an exception in every scheduling predicate, in the per-kind telemetry, in the results read types, in aggregation and in coverage.

### 4. A run every 5 minutes, handed out before any kind, one at a time

A run is due when none is in flight and the last submitted run started at least the interval ago. The first agent to ask gets it, before the choice of a kind. It stays in flight until its sessions are submitted or its timeout (3 minutes) passes, and a run that times out is due again at once.

It goes first because it is tied to the clock, and the scheduler otherwise drains every backlog of never-tested nodes before anything else. The cost is one delayed per-node assignment per run.

Twelve runs an epoch rather than a few long ones, because the specification's author asked for sampling spread evenly over the hour with no gap longer than five minutes, and because a single session would make a mixnode's whole epoch hinge on one stretch of seconds: one that coincides with the monitor's own stress run on that mixnode, or with a network fault at the agent, would then be the entire result.

Alternatives considered: a long-lived client per agent sending for the whole epoch (a hundred times the samples a whole-percent score can use, a different agent lifecycle, heartbeats); letting the run wait its turn among the kinds.

### 5. Two gateways, drawn at random among the fit ones, and not locked

A gateway is a candidate when it reports the entry-gateway role, does not enforce zk-nyms and has a gateway liveness run. A gateway with no gateway liveness run, or whose enforcement flag has not been read, is not a candidate. A candidate is fit when its latest gateway liveness run reached the bar and it has no test in flight.

The bar is chosen at each draw: the configured minimum, 1.0, when at least 40% of the candidates reach it, and otherwise the fallback, 0.98. Nothing is stored between draws. A gateway liveness run is 100 packets, so its score moves in steps of 0.01 and 1.0 means it lost nothing. On mainnet on 2026-11-24, 574 gateways had a liveness of 1.0, 13 were between 0.99 and 1.0, 41 between 0.95 and 0.99, and about 30 below, so today the bar is 1.0 with plenty to draw from.

The share is for when 1.0 stops being a gateway's normal state, which is expected once gateway stress (#7442) joins the filter. A gateway stress run is 12,000 packets and a single lost one takes it below 1.0; of 233 mainnet mixnodes, the nearest measured proxy, 148 had a latest stress run of exactly 1.0 and 192 of at least 0.98. The filter is meant to require the latest gateway stress run as well. That condition is added when #7442 lands, since this change goes in first, and the same rule then covers both runs: no separate minimum has to be chosen for stress. The cost of a share is that just under it hundreds of gateways may still meet the minimum while the draw admits ones that lost a packet or two; a worse gateway costs at most a discarded session (Decision 20). The number of fit gateways at each draw is a gauge, and a draw that uses the fallback is logged.

The draw is random, not by staleness, so that a layer-1 mixnode cannot predict which gateways carry test traffic next. Two side by side mean a gateway that goes bad mid-session costs half a run and not all of it, and each session has another, through the same mixnodes at the same moment, to be compared with. One at a time would rule that comparison out; three would add a majority at half again the traffic.

A run does not take in-flight rows for its gateways. A per-node test can start on a gateway during a session, and the overlap is small: a session adds one client's traffic to the gateway, and the only per-node test of a gateway today is a liveness probe of about a hundred packets. Locking would need either a kind row for something that is not a kind or a change to every kind's candidate query.

A run's score here is the one its aggregates are built from (`CompletedTestRun::score`), which is computed from the stored counters rather than held in a column. The orchestrator therefore reads each gateway's latest gateway liveness run and applies the share, the filter and the draw in code, over a few hundred rows, and the SQL stays plain.

The orchestrator learns enforcement from each gateway's own description. The node refresher reads the gateway's root details, which carry both the enforcement flag and the client websocket port, in place of the websocket read it makes today; `nym-node-requests` has no client call for the root details yet.

Alternatives considered: stalest-first rotation (predictable); letting the agent find out about enforcement by being refused (a wasted slot per run for every enforcing gateway drawn); lowering the minimum by hand when too few gateways reach it (the draw stays narrow until someone notices); a minimum of its own for gateway stress, picked from measured data (a second value to choose and maintain, where the share adapts by itself).

### 6. About 40 seconds per session at a default client's rate, built by the client's own builders

A session sends for a time drawn between two thirds and four thirds of the configured 40 seconds, so there is no fixed length to recognise, then waits for stragglers, up to 10 seconds and no longer once every payload and ack is back. It sends a Poisson stream at the combined rate of a client's two streams, about 55 packets a second by default. The two stream rates and the per-hop delays of packets and of acks are knobs of the agent, for network experiments, and each defaults to the client stack's own default, read from its config types rather than copied. The packet size is the client's default and is not a knob. A value away from the default makes route traffic differ from a client's, so it is for experiments only.

A session is about 2,200 packets, which puts each mixnode on about 110 test packets and up to 110 acks: about 220 samples per mixnode per session, the 200 the specification's author asked for. Per epoch that is twelve runs of two sessions, 40 seconds of sending in every five minutes and about 53,000 packets, so each mixnode is on about 2,640 test packets and up to as many acks. A whole-percent score needs far less; the surplus is what makes a lost session or a bad gateway cheap.

Packets are built by the client's builders, not by a copy of their layout: the loop cover packet builder and the SURB-ack constructor in `nym-sphinx` gain a form that takes the route (and, for the cover packet, its content), and the existing functions become callers that draw a route and pass it on.

A packet is identified by its sequence number in the session. It goes in two places. In the packet it takes the place of the cover marker at the start of the content, which is padded as cover content is and encrypted to the client's own key, so nobody else can read it; the session decrypts a returning payload the way the client's receiver does, from the ephemeral key in front of it. In the SURB-ack it is the five-byte identifier the ack carries, encrypted under the session's ack key and recovered with the stock `recover_identifier`.

The session drives the stock `GatewayClient` directly: `perform_initial_authentication` to register, `claim_initial_bandwidth` with tickets not required, `send_mix_packet` for each packet, `start_listening_for_mixnet_messages` with a `PacketRouter` whose two channels deliver payloads and acks apart, and `send_client_request` for the forget request. It is given its caller's bandwidth controller request sender, so that real tickets can be passed through later; until then callers pass one with no controller behind it, and since tickets are not required the gateway client never asks it. The active set and the gateway's endpoint come from the stock nym-api topology provider (Decision 16). The session does not use the SDK's message pipeline, which chooses routes itself and consumes acks internally. The agent's own session code stays as it is, for the exempt sessions of the gateway kinds.

Alternatives considered: the SDK client sending messages to itself (routes and acks are out of reach without the process-wide fragment registries, which are error prone and are not to be used); the agent's test packet builder (ack-size packets, constant delays, a reused header, none of which a client sends).

### 7. The sender picks the routes, with the ordinary random source

Because the session builds its packets, it knows each route as it sends and writes it into the record. A returning payload or ack is matched by its identifier against the session's own table of packets in flight. There is no seed to recompute a route from and no registry outside the session.

Alternative considered: seeding the route selection so a route could be recomputed later. That only helps when someone else picked the route. It would also make routes predictable to whoever learned the seed.

### 8. Acks are measurements, under two rules

A SURB-ack is a packet crossing three known mixnodes, in the ack size class every hop can see, so it is recorded like the packet that carried it. A node that dropped only acks would otherwise go unscored while costing real clients retransmissions.

Two rules, applied where records become test messages, keep that honest. An ack exists only if its packet reached the gateway, so when neither came back there is no test message for the ack: a failed one would blame three mixnodes for a packet that was never launched. And when the ack came back but the payload did not, the gateway unwrapped the packet and failed to hand it over, so the packet's three mixnodes get a success. The gateway is not scored, so nothing else is recorded for that packet.

An ack's test message carries its packet's send time, although the ack leaves the gateway later by the packet's transit, a few hundred milliseconds. The ordering tolerates that: it moves an ack past at most a few of a mixnode's other test messages, and only a node that starts or stops failing inside that window is blamed differently. Recording when the payload came back would estimate the launch better, for another field per record and a fallback when only the ack returns.

These rules add to the specification, which has no acks. They sit outside the scoring strategy, which sees only test messages.

### 9. Records travel and are stored as one packed batch per session

A batch is an 8-byte header (gateway node id, mixnet epoch) and then 34 bytes per packet, to the end of the batch: the time it was sent in milliseconds, the node ids of the packet's three mixnodes, those of the ack's three, and one byte each for "payload back" and "ack back". Every field is a fixed-width big-endian integer. The encoding is written by hand in the records crate, the header and the record each encoding and decoding themselves.

The agent submits a run once, with every session's batch or error. The orchestrator checks the run, reads the header, and stores the batch as received in `route_session`, with the gateway, epoch and record count beside it. It never expands a batch into a row per packet. A run is about 150 KB; a day is about 43 MB.

The cost is that SQL cannot answer "which packets failed through this node" without decoding. The format carries no version, because the agents and the orchestrator are deployed together; a later change to it deletes the stored sessions in its migration. A submission with an unreadable batch, a gateway the run was not given, or a run that is not in flight is rejected whole.

Alternatives considered: JSON records (several times the size for no gain, and the same decoding step); a row per packet (1.3 million rows a day at this volume, and the row count would follow the packet rate); per-node counts scored by the agent (no order, so no way to re-score, and the rule would live in the agent); counts per route (they drop the order the specification's rule is defined on); naming mixnodes by one-byte positions in layers carried in the header (9 bytes a record, for an indirection and checks on every position); a serialisation library (bincode is unmaintained, and two fixed-width structs need none).

### 10. Scoring is a trait with two inputs, and the first strategy is the specification's rule

Scoring is a trait with one method, in the records crate. It takes the test messages in the order they were sent, each with its send time, the nodes on its path in path order and whether it came back, and each node's most recent one-hop score where there is one. It returns each node's positive samples, negative samples and score. It knows nothing else, and the orchestrator calls it through the trait in one place, so replacing the strategy is replacing one implementation.

This is the interface the specification's author asked for. The postprocessing is expected to change once experiments on real data are done, and the trait is what lets a candidate be run beside the current strategy on the same test messages. The messages are ordered by sending time because not every one is received.

The one-hop scores are there because an unresponsive node is the likely cause of any loss on a route it is on, so a strategy can blame it and spare the others. How exactly is still to be specified by research. The first strategy therefore receives the scores and ignores them. "Most recent" is the score of the mixnode's latest mixnode liveness run, read when the epoch is scored; a mixnode with no such run has none.

The first strategy is the specification's: a test message that came back gives every node on it a positive sample and resets its fail sequence; one that was lost lengthens the fail sequence of every node on it, and the nodes with the longest then share one negative sample. The score is positives over positives plus negatives. The specification's six-message example is its test vector.

Alternative considered: a plain function with the same signature. It substitutes as easily in one call site, but gives no place to hold two strategies side by side or to choose one by configuration.

### 11. The five-hop liveness filed under an epoch comes from the epoch before, and no other

In the pass that materialises an epoch's aggregates, a new step takes the stored sessions whose batch names the epoch that just ended, turns them into test messages, orders them by send time, scores them afresh (nothing a strategy keeps, such as a fail sequence, carries over from the epoch before), and stores each mixnode's result in `mixnet_epoch_five_hop_liveness` under the epoch that is beginning. Stored once, never recomputed, as the aggregates are.

Within the epoch the runs are not scored apart. Their test messages form one list, twelve bursts with gaps of about four minutes, and a fail sequence carries from one run into the next. That is the specification's rule over an epoch's dataset, and a mixnode that is down stays the first suspect across the gap. Resetting at each run was considered and left out for that reason.

Last epoch only is the specification's rule for active nodes: a node is neither punished for past downtime nor carried by past good behaviour. Filing under the next epoch is the one-epoch lag every value the monitor submits already has, because the contract takes submissions only for the epoch in progress.

A session belongs to the epoch its batch names, which is the epoch of the active set it read, not to the clock. Nothing keeps a run away from an epoch's edges. A session that straddles a change keeps sending to the previous set for at most its own length, as a client does until its next topology refresh, and its batch is then too late to be counted.

Alternatives considered: the 6-hour window liveness uses (the specification rules averaging out); scoring an epoch shortly before it ends so the value is filed under itself (races the transition, leaves the tail unmeasured, and breaks "materialised once, at the transition").

### 12. No guard over an epoch; the kept and discarded sessions are counted

There is no epoch-level guard. An earlier draft had one (store nothing when fewer than half of the scored mixnodes are above zero), from when the gateway was a blame suspect and a dead session could zero the mixnodes it crossed. The discard of Decision 20 replaces it: a session that fails on the measuring side or at its gateway never reaches the scoring, and a kept session is never thin, since it delivered at least half of its messages and even the shortest one puts every mixnode on dozens of them.

An epoch with no kept session has no values, and Decision 13 carries the last ones forward. What a guard would have added is a visible signal, so the orchestrator exposes, per epoch, how many sessions it kept and how many it discarded.

Alternative considered: a coverage guard over the active set with a minimum sample count per mixnode. It would trip only in cases that already produce no values.

### 13. The last measured value is carried forward with the specification's decay

For an epoch with no measured value for a node, its five-hop liveness is `1 - e^(-0.1 * d) * (1 - value)`, from the node's newest earlier measured value, `d` epochs before. Beyond 48 epochs, where the weight is under one percent, it is 1. A node never measured has 1.

Without this, a mixnode that looks fine one-hop and routes badly is active for two epochs in every four: its bad five-hop liveness removes it for two epochs, then nothing is on file and it returns at full weight. The decay keeps the last measured value in play for about a day.

The value is the node's, whatever role it holds. A node measured as a mixnode keeps its fading value in an epoch in which it serves as a gateway, and the liveness submitted for it there is multiplied by it: a role change is one way of leaving the active set, and it should not restore full weight any faster than the others.

Nothing new is stored: the lookup reads `mixnet_epoch_five_hop_liveness`, which is retained for 30 days.

### 14. One switch, and five-hop liveness multiplies submitted liveness

`--five-hop-liveness-enabled`, an explicit value defaulting to on, governs everything: off, no run is handed out, nothing is materialised, and five-hop liveness is 1 for every node at once, which is the rollback.

The product is applied in the submission plan, to the liveness as the plan already computes it (both liveness kinds the coverage guard kept, combined), before the conversion to a percent. A node with no one-hop liveness to submit gets none from its five-hop liveness. On the aggregates API the liveness entry is unchanged and a five-hop liveness entry appears beside it, with the measured value and its positive and negative samples, so the two can be read apart.

There is no second switch to compute without counting. Liveness has no weight on mainnet until it is given one, and it is not to be given one before this is in.

### 15. Two crates and a manual command

`nym-network-monitor-route-records` holds the record, the batch format, the rules for test messages, and the scoring trait with its first strategy. It has no I/O, and the agent, the orchestrator and anyone re-scoring a stored batch depend on it.

`nym-network-monitor-route-client` holds the session: the client stack, the sending loop, the table of packets in flight. The agent's route run and its `test-five-hop-liveness` command are thin callers. The crate shares nothing with the agent's one-hop code: a different identity, transport, packet shape and lifecycle.

### 16. Sessions read the active set and the gateway's endpoint from nym-api, and the agent is told where it is

A session uses the client stack's topology provider against nym-api, as a client does, so it routes through exactly what clients route through. It reads the active set once, when it starts, and uses that reading for every packet, so the epoch in the batch's header holds for every record. The orchestrator never learns the active set. The agent takes the nym-api url as a required setting, not from network defaults.

This is the simplest source: the provider is one constructor and one call, and it returns the layers together with the epoch and the key rotation id.

The provider is used with the client's default settings, which leave out a mixnode whose performance at nym-api is below 50. A session therefore does not route through such a mixnode, as clients do not, and it has no five-hop liveness for the epoch: its last value fades as Decision 13 describes. Routing through it anyway would send it traffic no client sends, which it could tell apart. In practice a node that low is not in the active set.

The gateway's endpoint comes from the same reading. The session passes the gateway's entry in the topology to the client's own `SelectedGateway::from_topology_node` with the default selection, which gives `ws://<hostname>:<port>` when the gateway announces a hostname and its first announced address otherwise, with the next of them as the fallback the gateway client tries. So a run carries only each gateway's node id and identity key, and the agent's route run and the manual command resolve a gateway the same way. The session needs the gateway in its topology anyway, for the sphinx key of the last hop. Mixnodes cannot see this leg, so the choice does not bear on what the test measures; it is simply what a client does. The gateway kinds keep dialling `ws://<ip>:<port>` from their assignment, which their exemption needs.

For experiments, the manual command can route through more than the active set: the rewarded set (the layers and standby), or every node declaring the mixnode role, both read through the provider's extended read. Nodes of the active set keep their layer, and every other node is placed in layer `node_id % 3 + 1`, so a node's layer is the same in every session and epoch, no node repeats on a path, and the layers stay roughly even; the session then draws one node per layer as it does for the active set. nym-node forwards to any node it has resolved as a nym-node, and the one-hop tests already send through standby mixnodes, so such routes are expected to work; the manual run is what shows it. Route runs always use the active set: a wider selection would give standby and inactive nodes a five-hop liveness, which the specification rules out.

Alternatives considered: the orchestrator sending each gateway's address and websocket port from its registry (not what a client dials, and a second way of resolving a gateway beside the manual command's); the orchestrator tracking the rewarded set and sending the layers with each run (it would have to follow epoch changes it does not follow today, and the manual command would still need another source); the session reading the chain itself, the rewarded set from the mixnet contract and each node's keys and address from the directory contract, which mainnet nodes publish to. That removes the dependency on nym-api, but no topology provider is built that way: the entries would have to be decoded and the topology assembled by hand. It can replace the nym-api provider later without changing anything else.

### 17. What is tested

- The first scoring strategy: the specification's worked example, the tie rules, and that the one-hop scores do not change its result.
- The rules for test messages: the four outcomes.
- The batch: what is written is what is read; a batch that cannot be read is refused.
- The orchestrator, against the in-memory database as the kinds are: when a run is due and that it goes first; one run in flight; an abandoned run; which gateways are candidates, the bar at, above and below the share, and which gateways are fit; what a submission is rejected for; values from the previous epoch's sessions only; a late batch; a fail sequence carried from one run of an epoch into the next; that the strategy is given the most recent one-hop scores; the discard of suspected sessions and the per-epoch counts; the three cases of a node's five-hop liveness, and a carried value multiplying the liveness of a node now serving as a gateway; the product in the plan; that a posted liveness run is scaled and a posted stress run is not; the export of an epoch's sessions (order, the discarded mark, a failed session, an empty epoch, and that its kept test messages are the scoring's input); the switch.
- The node refresher storing the enforcement flag.

The session itself is not unit-tested. It needs live gateways and mixnodes, and the repository allows neither a live socket in a unit test nor a test-only seam on production code. `test-five-hop-liveness` against mainnet is its check, which Decision 2 is what makes possible. No test pins a default to its literal.

### 18. Liveness runs posted to nym-api are scaled until that stream goes

The product of Decision 14 is formed once per epoch in the contract submission, which nym-api's own figure never sees: nym-api averages the one-hop run scores the orchestrator posts per run. So each liveness run is posted with its score multiplied by the node's five-hop liveness for the newest materialised epoch (measured, carried forward, or 1, as Decision 13 defines). Stress runs are posted as they are.

This puts five-hop liveness into nym-api's figure at once, with no change to nym-api, and rewarding moves to the contract alone shortly after. The costs are accepted for that short life: a posted score is no longer what that run measured, nym-api's 24-hour mean spreads one bad epoch over a day, and the code sits in the result submitter that the epoch driver (#7398) deletes.

Alternatives considered: leaving nym-api's figure one-hop only until #7398; switching nym-api to read the contract, which replaces its whole score and not just liveness.

### 19. Built on develop, ahead of the epoch driver, the directory switch and gateway stress

Updated liveness is wanted before #7398 and #7423 merge, so this change is based on develop and depends on none of the three stacked PRs. Develop has everything it builds on. Gateway stress cannot sensibly go first: it assumes the nym-api result stream is gone, and on develop it would need a nym-api route and scoring for gateway stress that #7398 then deletes.

### 20. A gateway is never blamed, and a session whose gateway is suspected is discarded

The specification counts the gateway among the suspects of a lost message. Here it is not: a test message names its three mixnodes only, so a first failure is shared a third each, and the gateway is assumed reliable, as the specification's author asked in review. The assumption is checked per session, by the orchestrator, before scoring. A session is discarded when it lost more than half of its test messages, or when its loss rate is more than a configured margin (5 percentage points) above that of the other session of its run. A session with no other session to compare with gets the first test only. The batch stays stored, and the session is counted among its epoch's discarded sessions (Decision 12).

The two sessions of a run draw from the same layers at the same moment, so the mixnodes cost both the same loss on average. With about 2,200 packets each, two healthy sessions differ by about 0.3 points by chance at 1% loss and by about 1 point at 10%, so 5 points is well clear of chance. The first test is there because nothing else absorbs a dead session any more: if a fault on the measuring side killed both sessions of a run, 4,400 lost packets would otherwise be shared over 60 mixnodes, about 1.5 points off every active mixnode for that epoch.

Alternative considered: keeping the gateway as a suspect. A dead session's gateway then takes nearly all the blame by itself, with no threshold to set, but a gateway that drops at random passes three quarters of those losses to mixnodes and nothing ever removes its session.

Two consequences. The gateway filter requires gateway liveness only until gateway stress lands (Decision 5). And the three PRs are rebased over this change: #7423 does not overlap; #7398 and #7442 touch many of the same orchestrator and agent files, each side adding or removing beside the other, and their migrations are renumbered after this one.

### 21. An epoch's route sessions can be exported as test messages

The scoring strategy is expected to be replaced once research has experimented on real data (Decision 10), and its input is an epoch's test messages plus each node's one-hop scores. The one-hop runs are already served per node. So the results API gains one route, `GET /v1/results/route-sessions/epoch/{mixnet_epoch}`, behind the token the rest of that surface uses: the sessions that ran in that epoch, in the order their runs started, each with its run, gateway, error, whether the scoring keeps it, and its test messages decoded.

Nothing new is stored. The test messages come from the stored batch through the same step the materialisation uses, and the kept-or-discarded mark from the same rule, both when the request is made. A margin changed since the epoch was scored would therefore show in the marks; the batches are what was measured either way.

The response is the whole epoch, unpaged: 24 sessions and about 100,000 test messages, somewhere under 10 MB of JSON. A consumer wants the epoch whole, and paging by session would only move the merge to the caller. Epochs are available for as long as sessions are kept, 7 days.

Alternatives considered: no HTTP surface, copying the SQLite file and decoding the blobs by hand for each experiment; serving the packed batches undecoded, which would make every consumer implement the record format and the two ack rules.

## Risks / Trade-offs

- [The results token shows every route the monitor used] → after the fact only, for routes drawn at random and never reused. The same token already reads every test run.
- [The export is large] → one epoch is under 10 MB and is built on request from 24 blobs. It is an operator's read behind a token, not a public route.
- [Two gateways of a run that are bad by a similar amount are not suspected] → their loss rates agree, so both sessions are kept and the loss is shared among mixnodes. Rare with a bar of 1.0 on gateways, and one run is a twelfth of an epoch.
- [A gateway's loss inside the margin lands on mixnodes] → up to 5 points of one session's loss, a third to each mixnode on the packet. One session is a twenty-fourth of an epoch's data.
- [The rule gives a randomly dropping node a third of its blame] → when no mixnode on a lost test message has failed since its last success, all three share the sample. Accepted with the specification; the strategy is replaceable (Decision 10) and the batches are kept for 7 days to re-score.
- [Uniform loss on the measuring side lowers every active mixnode evenly] → an agent that drops a share of everything spreads it over all mixnodes through ties: both sessions of a run agree, so neither is discarded, and nothing tells that apart from a real network-wide problem. It costs every mixnode about a third of a point for every point of loss in that run. Visible as every mixnode's five-hop liveness moving together. Accepted while liveness has no weight; comparing a run with the epoch's other runs is the mitigation if the exported data shows it happening.
- [A session can coincide with the monitor's own stress run on a mixnode] → no lease is taken on mixnodes, by design. Twelve runs an epoch dilute one coincidence to a twelfth.
- [An outage is counted twice] → a mixnode down for part of an epoch loses in its one-hop liveness and in its five-hop liveness, and their product carries both.
- [The gateway knows] → the agents' source addresses are on chain. Only mixnodes are the target (Decision 1).
- [Layer 1 can see a quiet gateway start carrying one client's traffic] → the same thing a real client connecting looks like. The random draw removes the advance signal and the varied length the fixed one.
- [Free bandwidth is the gateway's choice] → enforcing gateways are not drawn. If most gateways come to enforce, the test starves and tickets become necessary (Open Questions). On 2026-11-24 all 40 sampled mainnet entry gateways, of 659 declaring the role, reported that they do not enforce.
- [A mixnode clients filter out is not measured] → a session uses the client's default topology settings, so an active mixnode below a nym-api performance of 50 gets no test message and its last value fades (Decision 16). Clients do not route through it either.- [Reward-affecting once liveness has weight] → every active mixnode's liveness is multiplied by a five-hop liveness at or below 1. Called out in the PR description.
- [Sessions at an epoch's edges can be lost] → one that straddles the change, or reads an active set nym-api has not refreshed yet, arrives after its epoch's values were stored. At most one run in twelve.
- [The enforcement flag is self-reported] → a gateway that says it does not enforce and does refuses the claim, and that session reports an error.
- [A description written before the migration has no flag] → the gateway is not drawn until the next refresh, within 2 hours.
- [A traffic knob set away from its default makes route traffic recognisable to mixnodes] → every knob defaults to the client's own value, read from the client's config types, and changing one is for experiments only.
- [The agent gains the client stack] → build time and binary size.
- [Agent and orchestrator must match] → an agent built before this change cannot parse a route run. The two are redeployed together; `--five-hop-liveness-enabled false` is the fallback.
- [Agents need a nym-api url] → an agent without one does not start. The deployment scripts gain it in the same redeploy.
- [A liveness score posted to nym-api is no longer what the run measured] → accepted until #7398 removes the stream (Decision 18). The orchestrator's own stored runs and its read API keep the measured score.
- [nym-api's 24-hour mean dilutes one bad epoch] → a single bad five-hop epoch moves nym-api's figure by about a twenty-fourth of what it moves the contract's.
- [The three stacked PRs conflict with this change when rebased] → mechanical: both sides add or remove beside each other, and the later migrations are renumbered (Decision 19).

## Migration Plan

1. Deploy the orchestrator and the agents together, the agents with a nym-api url. The migration is additive: one nullable column on `nym_node_description` and three tables.
2. Route runs start once gateways are fit, which needs a refresh that read their enforcement flag (within 2 hours) and a gateway liveness run each. The first values are filed in the epoch after the first runs.
3. Liveness stays without weight on mainnet until the decision to give it one is taken. Until then five-hop liveness is visible on the aggregates API and in the submitted liveness without affecting rewards.

Rollback: set `--five-hop-liveness-enabled false`. No run is handed out and five-hop liveness is 1 for every node at once. Rolling the orchestrator binary back past the migration means discarding its database, which is a rebuildable cache.

## Open Questions

- Tickets. Not designed here; the direction is only that the test should move to real tickets later. One cost is already known and has to be weighed then: a ticket is bound to one gateway and buys 200 MB, of which a session uses about 5 MB, and forgetting the client discards the rest. With random gateways and a forgotten client per session, every session needs a ticket of its own: 24 an epoch, 576 a day, about 860 NYM a day at the default deposit of 75 NYM for a book of 50 (the live price is not in the repository). Reusing bandwidth would bring that to about 15 tickets a day by volume, and needs identities kept at a small pool of gateways, which is at odds with both the random draw and forgetting.
- Gateway stress in the gateway filter. Added when gateway stress (#7442) lands (Decisions 5 and 19).
- Stress scores as a scoring input. The specification's author wants the strategy to receive each node's recent stress score beside its liveness score. It is added alongside liveness once stress testing is merged; until then the second input is the most recent one-hop liveness score only (Decision 10).
- Indistinguishable gateway testing. Wanted; no approach chosen.
- The specification's warm-up for newly bonded nodes. Left out of this change.

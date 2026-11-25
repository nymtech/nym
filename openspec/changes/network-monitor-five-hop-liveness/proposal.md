## Why

The network monitor tests every node on its own: an agent sends packets through one node and straight back to itself. A node can recognise that traffic, a mixnode by the agent's source address and a gateway by the agent's announced identity, so a mixnode could forward every measurement while dropping client packets and still score perfectly. Liveness has not been given weight on mainnet yet, and it should not be until this is closed: this change adds a test that sends packets shaped like a default client's through full routes of the active set, which a mixnode cannot tell from client traffic, and folds its result, the mixnode's five-hop liveness, into its submitted liveness.

## What Changes

- A route test beside the existing kinds. Every 5 minutes the orchestrator hands the next agent that asks for work a route run, ahead of any per-node test: two gateways drawn at random among those which do not enforce zk-nyms and whose latest gateway liveness run reached the bar. The bar is 1.0 while at least 40% of gateways reach it, and 0.98 otherwise.
- For each gateway the agent starts an ordinary client: a fresh identity, no monitor exemption, bandwidth claimed the way a client without tickets claims it, and a request to be forgotten when it finishes. It reads the active set from nym-api as any client does, reaches its gateway at the endpoint a default client would use, and sends for about 40 seconds (varied between about 27 and 53) at a default client's rate. Every packet is built by the client's own packet builders and crosses one randomly drawn mixnode of each layer back to the same gateway, as a client's loop traffic does.
- Every packet and its SURB-ack are recorded: the three mixnodes each crossed, and whether the payload and the ack came back. A session's records travel to the orchestrator as one binary batch per gateway, which the orchestrator stores as received.
- As each epoch begins the orchestrator unpacks the previous epoch's batches into test messages (the time sent, the route, and whether it came back) and scores them, giving every active mixnode a five-hop liveness. A gateway is never blamed: a lost message is shared among its three mixnodes only, and a session whose gateway is suspected (it lost more than half of its messages, or clearly more than the session running beside it) is discarded whole before scoring. Scoring sits behind one interface that takes the test messages in the order sent and each node's most recent one-hop score, so another strategy can replace the first once it has been worked out. The first strategy is the fail-sequence rule of the performance monitor specification; it does not use the one-hop scores, since how they should change the blame is not yet specified. An epoch left with no kept session is unmeasured, and the orchestrator exposes how many sessions it kept and discarded for each epoch.
- A mixnode's submitted liveness becomes its one-hop liveness times its five-hop liveness. That is the value measured in the previous epoch; for a mixnode that was not measured, the last measured value, decayed back towards 1 over about a day; for a mixnode never measured, 1. Gateways are only the door for the test and their liveness is unchanged.
- The liveness runs the orchestrator posts to nym-api are multiplied by the same five-hop liveness, so nym-api's own figure includes it at once, for as long as that stream exists.
- The measured five-hop liveness is served beside liveness and stress on the orchestrator's aggregates API.
- An epoch's route sessions can be read from the orchestrator's results API, decoded into test messages and each marked kept or discarded, so a replacement scoring strategy can be tried on real data without the orchestrator's database.
- The node registry also records whether a gateway enforces zk-nyms.
- Two new crates: one with the record format, the rules for test messages and the scoring interface with its first strategy, with no I/O; one with the route client. A manual `test-five-hop-liveness` command runs sessions through named gateways and prints the scores, and needs neither an orchestrator nor a monitor's authorisation.
- The sphinx builders for loop cover packets and SURB-acks gain a form that takes the route, so the packet layout keeps one implementation.
- Orchestrator: `--five-hop-liveness-enabled` (on by default), an interval, the number of gateways per run, a run timeout, a minimum gateway score with the share of gateways that must reach it and a fallback, and a session loss margin. Agent: a sending duration for a route session, a straggler wait and a timeout for a whole run, a nym-api url, and, for network experiments, the two stream rates and the per-hop delays of packets and acks, each defaulting to a client's.
- **BREAKING** (agent and orchestrator wire types): the work response can carry a route run, which an agent built before this change cannot parse, so the two are deployed together.
- **BREAKING** (agent configuration): the agent requires a nym-api url, so the agents' deployment scripts change in the same redeploy.

## Capabilities

### New Capabilities

- `network-monitor-five-hop-liveness`: the route test end to end: how a run is handed out and which gateways it uses, the client session and the shape of its traffic, the records and their batch, the rules that turn records into test messages, the scoring interface and its first strategy, the discard of suspected sessions, the per-epoch five-hop liveness and its session counts, how it enters submitted liveness, its entry on the aggregates API, the export of an epoch's sessions, its configuration and the manual command.

### Modified Capabilities

- `nym-network-monitor`: a due route run is handed out before the choice of a kind; the agent's single assignment may be a route run; the node refresher records whether a gateway enforces zk-nyms; the stored state gains that column and the route run and route session tables; a liveness run posted to nym-api carries the node's five-hop liveness; the agent surface gains the route run submission and the results surface a seventh read.
- `network-monitor-contract-submission`: a node's submitted liveness is multiplied by its five-hop liveness.

## Impact

- `nym-network-monitor-v3`: two new crates, the agent (a route run path, a manual command, configuration), the orchestrator requests crate (the work response, the route run submission, the aggregates read type, the route session export) and the orchestrator (one migration, the node refresher, route run assignment and ingestion, a materialiser step, the submission plan, the aggregates API, the results API, eviction).
- `common/nymsphinx`: route-taking forms of the loop cover and SURB-ack builders. `nym-node-requests`: a client call for a gateway's root details. `nym-topology`: an iterator over the nodes that declare the mixnode role, for the manual command's widest selection.
- New dependencies for the agent: the client stack's gateway client, bandwidth handling and topology provider.
- No change to the performance contract, nym-api or nym-node.
- Scoring: reward-affecting once liveness carries weight. Every active mixnode's liveness is multiplied by a value at or below 1.
- Traffic: about 53,000 packets an epoch through the mixnet, and about 3 GB a day through the gateways used. It costs nothing while free bandwidth can be claimed; see the design for what tickets would cost.
- Deployment: orchestrator and agents are redeployed together, and the agents gain a required nym-api url.
- Ordering: this lands on develop before the epoch driver (#7398), the directory switch (#7423) and gateway stress (#7442), which are rebased over it. The gateway filter gains its gateway stress condition when #7442 lands.

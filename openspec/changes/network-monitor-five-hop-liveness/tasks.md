## 1. Records, test messages and scoring

- [x] 1.1 Create `nym-network-monitor-v3/nym-network-monitor-route-records`, set up like the orchestrator requests crate (a workspace member, not a default member, `publish = false`, depended on by relative path), with no I/O dependencies: the record, the session batch with its header, and their hand-written encoding and decoding (design Decision 9)
- [x] 1.2 The test message (the time it was sent, the three mixnodes on its path in path order, whether it came back; no gateway); the step that turns a session's records into test messages under the two ack rules; a session's loss rate and the rule that discards a suspected session (more than half lost, or worse than the other session of its run by more than the margin; design Decision 20); and the merge of the kept sessions' test messages into send order (equal times by gateway node id)
- [x] 1.3 The scoring trait (test messages in send order and each node's most recent one-hop score in; per-node positive samples, negative samples and score out) and its first strategy, the fail-sequence rule, which receives the one-hop scores and does not use them (design Decision 10)
- [x] 1.4 Tests: the specification's worked example; a first failure shared three ways; a repeat offender taking the whole sample; the same result with and without one-hop scores; the four record outcomes; the discard rule (most messages lost, the worse of two unequal sessions, two sessions that agree, a lone session); the merge's order (send time, then gateway node id, each session's own order kept); a batch read back as written; a batch ending partway through a record, and one with a flag byte other than 0 or 1, refused

## 2. Sphinx builders

- [x] 2.1 In `nym-sphinx`'s acknowledgements crate, a SURB-ack constructor that takes the route; the existing constructor draws a route and calls it
- [x] 2.2 In `nym-sphinx`'s cover crate, a loop cover packet builder that takes the route, the SURB-ack and the content; `generate_loop_cover_packet` draws a route and calls it
- [x] 2.3 The existing tests of both crates and of their callers still pass, with no behaviour change for the client

## 3. Route client

- [x] 3.1 Create `nym-network-monitor-v3/nym-network-monitor-route-client`, set up like the records crate: the session's configuration (nym-api url, sending duration, straggler wait, and the traffic knobs: the main and cover stream delays and the per-hop delays of packets and acks, defaulting to the client config types' defaults; and which mixnodes to route through: the active set by default, the rewarded set or all mixnodes) and the per-session draw of the sending time between two thirds and four thirds of the configured duration
- [x] 3.2 Session setup: a fresh client identity and keys, the active set read once from nym-api through the client stack's topology provider with the client's default settings (its extended read for a wider selection), the three per-layer lists of mixnodes (active nodes keep their layer, any other selected node goes to layer `node_id % 3 + 1`; design Decision 16), the gateway's listeners derived from that topology through the client's own `SelectedGateway::from_topology_node` with the default selection, registration through the stock `GatewayClient` with tickets not required and the caller's bandwidth controller request sender, and the bandwidth claim; every failure here is the session's error
- [ ] 3.3 The sending loop: a Poisson stream at the combined rate of the configured two streams; for each packet, draw the packet's and the ack's mixnodes, one from each per-layer list, with the ordinary random source, build both through the route-taking builders at the client's default size and the configured delays, record them and send
- [ ] 3.4 Receiving: decrypt returning payloads and recover returning acks' identifiers (design Decision 6), match both to the session's table of packets in flight, each counted once; when the straggler wait ends, close the table and produce the session's batch
- [ ] 3.5 Finishing: ask the gateway to forget the client through the gateway client's `send_client_request`, and close; and one function that runs several sessions side by side on one clock under one deadline, each given a clone of the caller's bandwidth controller request sender
- [ ] 3.6 Unit tests for what needs no live session: the bounds of the sending-time draw, and the table of packets in flight for a returned, a late and a repeated arrival

## 4. Wire types

- [ ] 4.1 In the orchestrator requests crate, let the work response carry a test assignment, a route run or nothing: the route run with its id and, per gateway, node id and identity key
- [ ] 4.2 The route run submission (run id and, per gateway, a batch or an error), its route `/v1/agent/submit-route-run-result` and its client call
- [ ] 4.3 The five-hop liveness entry (score, positive samples, negative samples) on `NodeEpochAggregates`
- [ ] 4.4 Update the crate's doc comments that say work is always of a kind
- [ ] 4.5 The read type of an exported route session (its run, the gateway's node id, the start time, its error, whether it is discarded, and its test messages: time sent, three node ids, whether it came back) and the results route `/route-sessions/epoch/{mixnet_epoch}`

## 5. Orchestrator storage and node registry

- [ ] 5.1 Migration `06_five_hop_liveness.sql`: a nullable `enforces_zk_nyms` on `nym_node_description`, and the `route_run`, `route_session` and `mixnet_epoch_five_hop_liveness` tables
- [ ] 5.2 A `nym-node-requests` client call for a gateway's root details; the node refresher reads the client websocket port and the enforcement flag from it and stores the flag with the description
- [ ] 5.3 Storage for runs and sessions: start a run, the run in flight, mark it submitted, the start of the last submitted run, each gateway's latest gateway liveness run with its description (the candidates, the bar, the fit filter and the random draw are then made in code), store sessions as received, the sessions naming an epoch
- [ ] 5.4 Storage for five-hop liveness: insert an epoch's values, read them by epoch and by node and epoch, the newest measured value at or before an epoch within the 48-epoch limit, and each mixnode's most recent mixnode liveness run
- [ ] 5.5 Eviction: runs and sessions go with test runs, five-hop liveness values with aggregates
- [ ] 5.6 Tests: each condition of a candidate and of the fit filter; one run in flight; an abandoned run is due again; sessions read back as stored; the newest-value lookup at, inside and beyond the limit; eviction; the refresher storing the flag, and a description without one

## 6. Orchestrator scheduling, ingestion and config

- [ ] 6.1 Config and CLI: `--five-hop-liveness-enabled` (explicit value, default true), `--five-hop-liveness-interval` (5m), `--five-hop-liveness-gateways-per-run` (2), `--five-hop-liveness-run-timeout` (3m), `--five-hop-liveness-min-gateway-score` (1.0), `--five-hop-liveness-min-gateway-share` (0.4), `--five-hop-liveness-fallback-min-gateway-score` (0.98) and `--five-hop-liveness-session-loss-margin` (0.05), each with its environment variable, carried into a five-hop liveness config of its own
- [ ] 6.2 Assignment: before choosing a kind, hand out a route run when one is due and at least one gateway is fit, and build its payload. The bar is chosen at each draw by the share of candidates that reach the minimum (design Decision 5), with a gauge for the number of fit gateways and a log line when the fallback is used
- [ ] 6.3 The submission handler: accept only the run in flight and the gateways it was given, read each batch's header, store the sessions and mark the run submitted; reject anything else whole
- [ ] 6.4 Tests: a due run goes before an overdue kind; a run not due leaves the request to the kinds; a second request during a run is given a kind; no fit gateway leaves the run due; the bar with the share of candidates at the minimum above, at and below the configured share; a gateway below the fallback never drawn; disabled five-hop liveness hands nothing out; a late, an unknown and a mismatched submission each store nothing

## 7. Materialisation, submission and the read API

- [ ] 7.1 A five-hop liveness step in the materialisation pass: the sessions naming the previous epoch, the discard of suspected sessions, the test messages of the rest in send order, each mixnode's most recent one-hop liveness score, the scoring through the trait, and the results of bonded mixnodes stored under the epoch, written once; the same for backfilled epochs; the number of sessions kept and the number discarded, exposed as two gauges for the epoch just scored
- [ ] 7.2 A node's five-hop liveness for an epoch: the value measured for it, else the newest earlier measured value within the limit decayed, else 1; and 1 for every node while five-hop liveness is disabled
- [ ] 7.3 The submission plan multiplies a node's liveness by its five-hop liveness before the conversion to a percent
- [ ] 7.4 The aggregates API serves the five-hop liveness entry
- [ ] 7.5 The result submitter multiplies each liveness run it posts to nym-api by the node's five-hop liveness for the newest materialised epoch, and posts stress runs unchanged (design Decision 18)
- [ ] 7.6 The results API serves an epoch's route sessions (design Decision 21): the sessions naming the epoch in start order, each decoded into test messages by the records crate and marked kept or discarded by the same rule the materialisation uses, in one unpaged response, with its OpenAPI entry
- [ ] 7.7 Tests: the export of an epoch (start order, a discarded session with its mark, a failed session without test messages, an empty epoch, and its kept test messages merged being the scoring's input); a posted liveness run is scaled and a posted stress run is not; values come from the previous epoch's sessions only; a late batch changes nothing; nothing carries across epochs in the scoring, and a fail sequence carries from one run of an epoch into the next; the strategy is given the most recent one-hop scores; an epoch whose sessions were all discarded stores nothing, while one discarded session among several leaves the rest scored; the kept and discarded counts of an epoch; the three cases of a node's five-hop liveness, a carried value multiplying the liveness of a node now serving as a gateway, and the disabled case; the plan's product, and no liveness from five-hop liveness alone; the read entry present and absent

## 8. Agent

- [ ] 8.1 Config and CLI: a required `--nym-api-url`, `--five-hop-liveness-sending-duration` (40s), `--five-hop-liveness-waiting-duration` (10s), `--five-hop-liveness-run-timeout` (2m), and the four traffic knobs (`--five-hop-liveness-message-sending-average-delay`, `--five-hop-liveness-loop-cover-traffic-average-delay`, `--five-hop-liveness-average-packet-delay`, `--five-hop-liveness-average-ack-delay`), which fall back to the route client's defaults when unset, each with its environment variable
- [ ] 8.2 Dispatch a route run: its sessions through the route client under the run's deadline, always over the active set, with a bandwidth controller request sender that has no controller behind it while tickets are off, submitted once
- [ ] 8.3 The manual `test-five-hop-liveness` command: a nym-api url, gateways, the route session settings of 8.1 and, for experiments, which mixnodes to route through (active set by default, rewarded set, all mixnodes), sessions through the route client, scores printed through the records crate
- [ ] 8.4 Unit test, as the gateway kinds have one: a route run whose sessions cannot be established is submitted with an error for each gateway

## 9. Verification and close-out

- [ ] 9.1 `cargo fmt`; `cargo check --all-targets` and the tests of the two new crates, the two `nym-sphinx` crates, the agent, the requests crate (also with `--features openapi`) and the orchestrator
- [ ] 9.2 Read the diff against every scenario of the three delta specs, and record in design Decision 17 which scenarios are left untested and why
- [ ] 9.3 Run `test-five-hop-liveness` by hand against mainnet through a few gateways and check that payloads and acks return and the printed scores are plausible; then a local orchestrator with one agent, to see a run handed out, submitted, materialised and served
- [ ] 9.4 PR description: what the route test is and why; the reward effect once liveness has weight; the orchestrator's new switch and knobs; the agent's required nym-api url and new knobs, for the deployment scripts; that orchestrator and agents deploy together; that liveness runs posted to nym-api are now scaled by five-hop liveness; that the gateway filter uses gateway liveness only until gateway stress lands; that #7398, #7423 and #7442 are rebased over this; the known limits (a quarter of the blame for a random dropper, uniform loss on the measuring side, the ticket cost once tickets are needed, the one-hop scores not yet used by the scoring)
- [ ] 9.5 Sync the main specs, check `git diff openspec/specs` line by line, and archive the change in the PR

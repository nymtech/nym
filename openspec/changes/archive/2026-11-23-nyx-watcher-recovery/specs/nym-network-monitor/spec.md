## MODIFIED Requirements

### Requirement: Network nodes learn the authorised-agent set from the contract and gate connection, routing, and replay-bypass on it

A Nym node SHALL derive which network-monitor agents may probe it directly from the network-monitors contract, not from any orchestrator or nym-api. A node MUST load the full authorised-agent set at startup with every page pinned to one block height (via `get_all_network_monitor_agents_at_height`; the startup requirement says what a failed load does) and MUST thereafter keep it current in REAL TIME through a nyxd websocket event subscription that starts at the block after that height and dispatches `AuthoriseNetworkMonitor`, `RevokeNetworkMonitor`, and `RevokeAllNetworkMonitors` contract events. A node MUST load the set again, in the same way, whenever the watcher behind that subscription has ended and is started anew (see the supervision requirement). Between those loads this is an event subscription, NOT a periodic contract poll; the node's periodic topology refresher explicitly preserves (does not reload) the agent set.

The node MUST fold the set into THREE shared, lock-free structures: a canonical-IP-keyed routing set (`RoutableNetworkMonitors`), a canonical-IP-keyed noise-key map (`NoiseNetworkView`, in which one IP may host several agents disambiguated by port), and a set of announced monitor ed25519 CLIENT IDENTITIES keyed by that identity rather than by any address. The two IP-keyed structures MUST key on `IpAddr::to_canonical()` at insert AND lookup so that a v4-mapped-IPv6 form matches its canonical IPv4 form. The identity set MUST tolerate the same identity arriving from several agent entries, since an agent authorises one entry per address family and both carry its identity, and MUST tolerate entries carrying no identity at all, which is a validly authorised agent that simply cannot be recognised on the client-session path. There is no separate "extra initiator IPs" allowlist; inbound acceptance is a facet of the noise map. The authorised set MUST gate five behaviours: (1) the Noise responder handshake - an inbound connection from an IP not in the noise map falls back to raw TCP and the agent's handshake fails; (2) packet routing through `NetworkRoutingFilter`, in which a packet originating from an authorised monitor may ONLY be routed to another authorised monitor; (3) most importantly, the sphinx REPLAY / bloomfilter BYPASS - a packet detected as replayed MUST be dropped as a replay UNLESS it originates from an authorised network-monitor agent IP, which is the mechanism that lets the agent's deliberately-replayed probe header (see the `reuse_header` requirement) be processed rather than filtered; (4) FINAL-HOP DELIVERY - a final-hop packet originating from an authorised monitor MUST be processed and delivered to a live client session, and MUST NOT be written to the recipient's on-disk store if no session is live, so that a packet which did not arrive on the socket was definitively not delivered and monitor traffic cannot accrue undeliverable stored messages on every gateway; and (5) CLIENT SESSION METERING - a client websocket session whose registration handshake authenticates an ed25519 identity in the announced-identity set MUST be treated as an ephemeral monitor session: it MUST NOT be metered for bandwidth, MUST NOT require any bandwidth credential, and MUST NOT persist a shared-key, bandwidth, or stored-message entry.

Gates (4) and (5) are what make gateway liveness testing possible; before them a monitor's final-hop packets were dropped outright as unsupported, and a monitor could not open a client session without presenting bandwidth credentials.

Gates (1) through (4) are keyed by SOURCE IP only (not public key); the port is effectively ignored on the agent-as-initiator probe path (it is consulted only when the node dials an agent). Gate (5) is the ONE exception and MUST be keyed on the handshake-verified client identity, with the source IP playing no part, because the client websocket port performs no Noise handshake and so can never be covered by the follow-up that moves the mixnet gates onto the Noise-authenticated static key, and because the exemption it guards is not confined by gate (2): a packet handed to a gateway over a client session is forwarded without the monitor flag and may therefore be routed to any known node, so an IP-keyed exemption inherited by a co-tenant behind a shared host port or a recycled address pool would grant unconfined unmetered transit.

The consequences are: an agent cannot successfully probe a node until it is authorised on-chain AND that authorisation event has been ingested by the node (propagation is bounded by block inclusion plus websocket delivery, on the order of seconds, NOT by any refresh interval; right after a watcher start it can take about 30 seconds longer, while the blocks produced during the load are fetched by height); for gates (1) through (4) the IP the agent actually connects from MUST equal one of the `mixnet_address` IPs recorded on-chain for it (an egress IP that is neither, whether through NAT or a third interface, still breaks all four); an authorised agent with an announced identity obtains unmetered gateway transit for sessions presenting that identity, bounded by the orchestrator's ability to revoke the authorisation; and a node does not need a process restart to recover an agent set its watcher failed to keep current: blocks missed during websocket downtime are fetched by height, and a watcher that cannot advance is replaced after the whole set has been loaded again, so a revocation the watcher could not deliver takes effect at that reload. What a node can still miss for good is a contract message its binary cannot parse, which is logged and skipped and triggers no reload; for gate (5) that means a monitor revoked by such a message keeps its unmetered sessions until the node's next reload or restart. A node can also lose an agent's noise entry until then: an agent on an IP that the topology refresher lists for a nym-node is left out of the noise map, and nothing puts it back once that nym-node leaves, so the agent's handshakes fail.

Because an agent authorises one ipv4 and one ipv6 address, it occupies TWO entries in each node's structures - one per address, both carrying the same noise key - so a probe arriving over either family passes every gate. The node treats those entries independently: it neither knows nor needs to know that they belong to one agent, and revoking one leaves the other authorised.

Intended follow-up (recorded here as a planned change, NOT current behaviour): gate the replay bypass and the final-hop delivery on the agent's Noise-authenticated x25519 static key rather than its source IP. The current `Noise_XKpsk3` handshake already receives and possession-authenticates that key (the message-3 `se` step proves the agent holds the corresponding private key), so this hardening needs no packet-format change and would remove the source-IP spoofing and NAT-fragility of the present gates. The follow-up covers the MIXNET gates only; the client-session exemption is out of its reach and is why gate (5) is identity-keyed from the outset.

#### Scenario: A newly authorised agent is accepted in near real time
- **WHEN** an orchestrator authorises an agent on-chain and the transaction is included in a block
- **THEN** each node's websocket watcher ingests the `AuthoriseNetworkMonitor` event and adds the agent's IP and noise key to its routing set and noise map without waiting for any refresh interval

#### Scenario: An unauthorised agent cannot complete a handshake or have replays accepted
- **WHEN** an agent that the node has not ingested opens a connection and sends replayed packets
- **THEN** the Noise handshake falls back to raw TCP and fails, and any replayed packet is dropped as a replay because it does not come from an authorised agent IP

#### Scenario: Replayed probe traffic from an authorised agent bypasses the bloomfilter
- **WHEN** an authorised agent sends its deliberately-replayed probe header
- **THEN** the node still runs its replay-detection bloomfilter but bypasses the drop because the packet's source IP is in the authorised network-monitor set, and processes the packet

#### Scenario: A monitor's final-hop packet is delivered to its live session
- **WHEN** an authorised agent sends a final-hop packet to a gateway addressed to a client session it currently holds open
- **THEN** the gateway unwraps it and pushes it into that session

#### Scenario: A monitor's final-hop packet is dropped rather than stored
- **WHEN** an authorised agent sends a final-hop packet whose recipient has no live session
- **THEN** the packet is dropped and nothing is written to the on-disk store

#### Scenario: A monitor's client session needs no bandwidth
- **WHEN** an authorised agent opens a client websocket session presenting its announced ed25519 identity and forwards packets
- **THEN** the session is not metered, no credential is required, and no shared-key, bandwidth, or stored-message entry is persisted for it

#### Scenario: An authorised IP alone does not earn the session exemption
- **WHEN** a client opens a websocket session from an authorised agent's IP but presents an identity that is not in the announced-identity set
- **THEN** the session is metered and requires credentials like any other client's

#### Scenario: Revocation stops acceptance after the event is ingested, with no periodic re-sync
- **WHEN** an agent is revoked on-chain and the node ingests the `RevokeNetworkMonitor` event
- **THEN** the node removes it from the routing set and noise map so new handshakes fail, replays are dropped again, and its client sessions are metered like any other
- **AND** the node does not poll the contract in the meantime: a revocation its watcher could not deliver takes effect when that watcher is replaced and the whole set is loaded again

## ADDED Requirements

### Requirement: A node loads the authorised-agent set pinned at one height and starts its watcher at the next block

Whenever a node starts its chain watcher, at boot and after every watcher end alike, it SHALL run one sequence: obtain the current block height N from a chain RPC endpoint; load the full authorised-agent set from the network-monitors contract with EVERY page of the query pinned at N, from that same endpoint; publish the loaded set (see the replacement requirement); and start the watcher with N as the height its state reflects, so that the first block it processes is N + 1.

The endpoints MUST be tried in configured order, `nyxd_urls` followed by `nyxd_fallback_urls`, moving to the next only when the height query or the pinned load failed on the previous one. The height and the load MUST NOT come from different endpoints. The node's signing client, which holds one randomly chosen endpoint for the node's lifetime, MUST NOT be used for this. The loaded set MUST be published before the watcher start is attempted, so that an attempt refreshes the set even when the watcher then cannot be started.

#### Scenario: An authorisation made while the node was loading is not lost
- **WHEN** an agent is authorised in a block above N that was produced before the watcher's subscription delivered its first block
- **THEN** the watcher fetches that block by height and the agent is added

#### Scenario: A backend that has not reached the height is passed over
- **WHEN** an endpoint refuses the pinned load because the backend answering it has not reached height N yet
- **THEN** the node runs the same sequence, height first, on the next configured endpoint

#### Scenario: The set is refreshed even when the watcher cannot start
- **WHEN** the pinned load succeeds but the websocket endpoint cannot be reached
- **THEN** the node gates on the newly loaded set and the attempt counts as failed

### Requirement: A reload replaces the whole authorised-agent set without exposing an empty or partial one

A loaded set SHALL replace what the node holds as a whole. The node MUST have loaded the full set before changing anything, MUST build each of the three derived structures aside, and MUST publish each with a single atomic swap, so that a reader of any one structure sees either the previous set or the new one and never an empty or half-built one. The three structures are swapped one after another rather than as one unit, exactly as a single authorisation event already updates them.

The noise-key map is shared with the topology refresher, which owns its nym-node entries. The new map MUST be the nym-node entries present at that moment plus the loaded agents, assembled under the map's update permit. A load that fails MUST leave all three structures untouched.

#### Scenario: Agents that are gone are dropped and new ones added
- **WHEN** the node holds agents A, B and C and a reload returns A, B and D
- **THEN** A, B and D are in the routing set, the noise-key map and, where they announce one, the identity set
- **AND** C is in none of the three

#### Scenario: Nym-node noise entries survive a reload
- **WHEN** the noise-key map holds nym-node entries put there by the topology refresher and a reload is published
- **THEN** those entries are still present afterwards

#### Scenario: A failed reload keeps the current set
- **WHEN** no configured endpoint can serve the height query and the pinned load
- **THEN** the node keeps gating on the set it already holds

### Requirement: Node startup fails without a working watcher unless the operator allows it

At boot a node SHALL make up to 3 attempts at the load-and-start sequence, 10 seconds apart, and MUST refuse to start when all of them fail.

The node SHALL accept a run-time flag `--allow-nyx-watcher-startup-failure` (environment variable `NYMNODE_ALLOW_NYX_WATCHER_STARTUP_FAILURE`). When it is set the node makes a single attempt, and when that attempt fails it MUST log an error stating that every network-monitor agent is rejected until an authorised-agent set has been loaded, MUST start with whatever set the attempt published (none when the load itself failed, the whole loaded set when only the watcher could not be started), and MUST keep retrying on the supervised schedule of the next requirement. The flag is a run-time argument only: it MUST NOT be written to the node's configuration file.

#### Scenario: A node without a reachable chain endpoint refuses to start
- **WHEN** none of the three attempts succeeds and the flag is not set
- **THEN** startup is aborted with the last failure

#### Scenario: A transient failure at boot is ridden out
- **WHEN** the first attempt fails and the second succeeds
- **THEN** the node starts normally

#### Scenario: The flag lets the node start with an empty set
- **WHEN** the flag is set and the single attempt fails to load the authorised-agent set
- **THEN** the node starts, logs the error, and rejects every network-monitor agent
- **AND** its supervisor keeps attempting the load-and-start sequence

#### Scenario: The flag lets the node start with a loaded set but no watcher
- **WHEN** the flag is set and the single attempt loads the authorised-agent set but cannot start the watcher
- **THEN** the node starts, logs the error, and gates on the loaded set
- **AND** its supervisor keeps attempting the load-and-start sequence

#### Scenario: The flag applies to one run only
- **WHEN** a node is started once with the flag and later started again without it
- **THEN** the later start applies the default rule and refuses to start without a working watcher

### Requirement: A node replaces a chain watcher that has ended, with a growing randomised wait

A node SHALL supervise its chain watcher. When the watcher ends for any reason other than node shutdown, which covers a watcher that gave up on a stall and one whose websocket reconnection attempts were exhausted, the node MUST first wait until every task of that watcher has stopped, so that a block it was still processing cannot be applied after the reload; it MUST then wait, run the load-and-start sequence again, and repeat until a watcher is running. The node process MUST NOT exit or restart on account of its watcher, and the authorised-agent set it holds MUST stay in use until a load succeeds.

The wait before an attempt MUST start at 30 seconds, double after every attempt, and never exceed 15 minutes. Each actual wait MUST be drawn at random between half of that value and the value itself, so that nodes which noticed the same chain-wide stall do not all reload at once. Once a watcher has stayed up for 15 minutes the wait MUST return to 30 seconds. These timings are constants, not configuration. Every attempt MUST be logged, and the node MUST count restart attempts, successful or not, in its prometheus metrics.

On node shutdown the supervisor MUST stop a running watcher and return, including while it is waiting between attempts.

#### Scenario: A stalled watcher is replaced without restarting the node
- **WHEN** the watcher gives up because it cannot advance
- **THEN** the node reloads the authorised-agent set at the current height and starts a new watcher from that height, with the node process still running

#### Scenario: A block in flight is not applied after the reload
- **WHEN** the old watcher is still processing a block at the moment it is cancelled
- **THEN** the reload starts only after that watcher has fully stopped

#### Scenario: A chain-wide outage does not produce a tight loop
- **WHEN** every attempt fails for an hour
- **THEN** the waits grow from 30 seconds to the 15 minute cap and stay there
- **AND** the node keeps gating on the set it already holds

#### Scenario: A watcher that recovered resets the wait
- **WHEN** a watcher has run for 15 minutes and then ends
- **THEN** the wait before the next attempt starts again from 30 seconds

#### Scenario: Shutdown interrupts the wait
- **WHEN** the node is shut down while the supervisor is waiting between attempts
- **THEN** the supervisor returns without making another attempt

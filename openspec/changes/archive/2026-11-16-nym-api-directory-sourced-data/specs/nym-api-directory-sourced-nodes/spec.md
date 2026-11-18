## ADDED Requirements

### Requirement: Node-facing routes are built from the directory at the latest settled snapshot

nym-api SHALL build every response that describes nym-nodes (the skimmed and semi-skimmed topology routes of every version, the described routes and the noise-key route) from the latest settled snapshot alone: the verified directory records, the node details, the rewarded set, the interval and the key rotation state held for that height, with each node's performance read from the same height's performance snapshot. It MUST NOT request anything from a nym-node, and MUST NOT combine values held for different heights in one response.

The response types SHALL stay as they are in every output format. Nodes SHALL be listed in ascending node id order, the epoch id and key rotation id SHALL be those in force at the snapshot's height, and `refreshed_at` SHALL be the block time of that height, so that two instances that have settled the same height return identical bodies.

#### Scenario: Two instances agree
- **WHEN** two nym-api instances have both settled height `H` and a client asks each for the same topology route with the same parameters
- **THEN** both return the same nodes in the same order with the same values, epoch id, key rotation id and `refreshed_at`

#### Scenario: A snapshot that has not settled is not served
- **WHEN** a newer height has been produced but the tip has not reached it plus the settle lag
- **THEN** the routes still serve the node data of the previous settled height

#### Scenario: No node is contacted
- **WHEN** nym-api runs through startup, several snapshot intervals and an epoch change
- **THEN** it makes no HTTP request to any nym-node

### Requirement: A node is listed only when the directory describes it at the snapshot height

A node SHALL be listed only when, at the snapshot's height, it is bonded, every one of its directory entries verifies against its bonded identity key, its `NodeInformation` entry is present and decodes, and it publishes a sphinx key for the key rotation in force at that height. A node failing any of these MUST be absent from every route of this capability, whatever role the rewarded set gives it. The first two conditions SHALL be decided by the same shared rule the epoch driver uses for its candidates.

A node that started unbonding has no directory entries from its unbond transaction on, so it SHALL leave the listing at the first snapshot taken after that transaction, not at the epoch end.

#### Scenario: A node with an entry that does not verify is not listed
- **WHEN** one of a bonded node's directory entries does not verify against its bonded identity key
- **THEN** the node is absent from the topology and described routes

#### Scenario: A node without a sphinx key for the rotation is not listed
- **WHEN** the key rotation in force at the snapshot's height is 12 and a bonded node publishes a sphinx key for rotation 11 only
- **THEN** the node is not listed at that height, and is listed at the first later snapshot at which its key for rotation 12 is in the directory

#### Scenario: An unbonding node leaves at the next snapshot
- **WHEN** a node holding the mixnode role sends its unbond transaction between two snapshot heights
- **THEN** it is listed at the earlier height and absent from the later one, although the rewarded set still names it until the epoch ends

### Requirement: Nodes announcing non-public addresses are not listed

A node whose `NodeInformation` announces any IP address that is not publicly routable (private, loopback, link-local, shared, reserved or documentation ranges) MUST NOT be listed. The rule SHALL be switched off only by `[directory.debug] unsafe_allow_illegal_ips`, which the hidden `--unsafe-allow-illegal-ips` flag sets, for local test networks; it is off by default.

#### Scenario: A private address keeps a node out
- **WHEN** a bonded node's directory record announces `10.0.0.5` and the override is not set
- **THEN** the node is absent from every route of this capability

#### Scenario: The override lists it
- **WHEN** the same node is read by a nym-api started with `--unsafe-allow-illegal-ips`
- **THEN** the node is listed

### Requirement: Served values come from the directory, with fixed placeholders for what it does not hold

Each listed node's values SHALL come from its verified directory entries and its node details at the snapshot's height: identity, node id and contract node type from the node details; addresses, hostname, ports, location, modes and `build_version` from `NodeInformation`; the current and, when published, pre-announced sphinx keys from the sphinx keys entry; service provider addresses, wireguard details and Lewes Protocol details from their entries; the noise key, the acceptance of the operator terms and the node's own Lewes Protocol signature from the fields nodes publish for them.

A node that does not publish one of those three fields SHALL be served without it: no noise key, terms not accepted, and no Lewes Protocol details at all; the v3 skimmed routes, whose node type requires Lewes Protocol details, leave such a node out. Lewes Protocol content MUST NOT be served without the node's own signature over it.

The declared `exit_nr` and `exit_ipr` roles SHALL both equal the node's exit mode, and the declared mixnode and entry roles its mixnode and entry modes.

Values the directory does not hold SHALL be served as fixed placeholders: in `build_information`, `binary_name` SHALL be `nym-node`, `build_timestamp` the Unix epoch and every field other than `build_version` the string `unknown`; `network_requester.uses_exit_policy` SHALL be `true` wherever a network requester is served; `last_polled` SHALL be the block time of the snapshot's height.

#### Scenario: The build version is the published one
- **WHEN** a node's `NodeInformation` carries binary version `1.20.0`
- **THEN** its served `build_information.build_version` is `1.20.0`, `binary_name` is `nym-node`, `build_timestamp` is the Unix epoch and the remaining fields are `unknown`

#### Scenario: Exit without entry is exit-capable
- **WHEN** a node's modes declare exit and neither mixnode nor entry
- **THEN** its declared `exit_nr` and `exit_ipr` roles are both true and its declared entry role is false

#### Scenario: A node not publishing the noise key is served without one
- **WHEN** a listed node's `NodeInformation` carries no noise key
- **THEN** it is served with no noise key on the semi-skimmed, described and noise-key routes, and stays listed

#### Scenario: Unsigned Lewes Protocol details are not served
- **WHEN** a listed node publishes Lewes Protocol details without its own signature
- **THEN** it is served with no Lewes Protocol details, and the v3 skimmed routes leave it out

### Requirement: Active-only responses expire when the first snapshot after the epoch is due becomes served

On the responses that carry an `Expires` header (those restricted to active nodes), the header SHALL name the moment the first snapshot taken after the epoch is due to change becomes served, estimated from the served snapshot alone. With `t` the block time of the served height, `Δ` the snapshot interval in blocks, `L` the settle lag in blocks and `b` the fixed block time of 5.687 s: `due` SHALL be the end of the epoch current at the served height plus 2 minutes, `k` the smallest integer of at least 1 for which `t + k·Δ·b` is not before `due`, and the header `t + (k·Δ + L)·b`. Responses not restricted to active nodes SHALL carry no `Expires` header, as before.

#### Scenario: Mid-epoch
- **WHEN** the served height's block time is 12:00:00, its epoch ends at 12:30:00, the interval is 100 blocks and the settle lag 10
- **THEN** `due` is 12:32:00, `k` is 4, and the header is 2,331.67 s after 12:00:00, that is 12:38:51

#### Scenario: An overdue epoch expires at the next snapshot
- **WHEN** the served height's block time is 12:00:00 and its epoch was due to end at 11:55:00
- **THEN** `k` is 1 and the header is 625.57 s after 12:00:00, that is 12:10:25

#### Scenario: Every instance sends the same header
- **WHEN** two instances serve the same settled height
- **THEN** their `Expires` headers on the same active-only route are equal

### Requirement: Address lookups are answered from the directory

`POST /v1/unstable/nym-nodes/by-addresses` SHALL resolve each requested IP address to the node id of the listed node announcing it in the latest settled snapshot, and to no node when none does. The request and response types SHALL stay as they are.

#### Scenario: A listed node's address resolves
- **WHEN** a listed node announces `203.0.113.7` and a caller asks for that address
- **THEN** the response maps it to that node's id

#### Scenario: A node not yet in a settled snapshot is unknown
- **WHEN** a node published its directory entries after the latest settled height
- **THEN** its address resolves to no node until a snapshot holding them has settled

### Requirement: Routes answer 503 while no snapshot has settled

While nym-api holds no settled snapshot, every route of this capability MUST answer 503 Service Unavailable, and MUST NOT serve data from an unsettled height, from a previous run's caches or from a default.

#### Scenario: Three failed heights leave nothing to serve
- **WHEN** an instance fails to produce three consecutive snapshot heights so that its retained window is empty
- **THEN** its topology and described routes answer 503 until a height is produced and settles

### Requirement: The describe refresh endpoint answers 410 Gone

`POST /v1/nym-nodes/refresh-described` SHALL answer 410 Gone for every request, without reading its body, with a plain-text message saying that the endpoint was removed, that nym-api reads node data from the directory, and that the calling nym-node should be upgraded.

#### Scenario: An outdated node is told to upgrade
- **WHEN** a nym-node from an earlier release posts its refresh request at startup
- **THEN** it receives 410 with that message, which it logs in its warning

### Requirement: The utility routes are removed

nym-api SHALL serve no route under `/v1/utility`, and SHALL have no utility bearer setting: neither a `utility_routes_bearer` config field nor a `--utility-routes-bearer` flag.

#### Scenario: The cache refresh route is gone
- **WHEN** a caller posts to `/v1/utility/refresh-mixnet-cache`
- **THEN** it receives 404

#### Scenario: The flag is rejected
- **WHEN** nym-api is started with `--utility-routes-bearer`
- **THEN** argument parsing fails and it does not start

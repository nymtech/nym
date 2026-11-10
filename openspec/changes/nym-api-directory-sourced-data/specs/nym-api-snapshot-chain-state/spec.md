## ADDED Requirements

### Requirement: Mixnet contract state is read beside each snapshot

At each height `H` it snapshots, the producer SHALL also read from the mixnet contract, every query pinned at `H`: the node details of every bonded node, the rewarded set (its metadata and every role's assignment), the interval, the key rotation state, the reward parameters, the contract state, the config score parameters and the nym-node version history. The identity map every attested contract is verified with at `H` SHALL be derived from those node details, so the bond set is read once per height.

These SHALL be held with the retained entry for `H`, for every retained height. They MUST NOT enter any signed snapshot and MUST NOT be served under an attestation route tree. When any of these reads fails, nothing SHALL be retained for `H`, as when verification fails.

nym-api MUST NOT read these values on any other schedule: there SHALL be no separately refreshed mixnet contract cache, and no `[mixnet_contract_cache]` setting SHALL be read. A config file that still holds that section SHALL load.

#### Scenario: One read serves identities and stake
- **WHEN** the producer produces height `H`
- **THEN** it reads the node details once at `H`, verifies every attested contract against the identity keys in them, and makes no separate bond query

#### Scenario: A failed mixnet read retains nothing
- **WHEN** every verified read at `H` succeeds but the rewarded set read at `H` fails
- **THEN** nothing is retained for `H`, the failure is logged, and the next poll tries the newest cadence height again

#### Scenario: The values are those at the height
- **WHEN** the epoch advances two blocks after `H`
- **THEN** the entry retained for `H` holds the interval and rewarded set of the epoch before the advance

#### Scenario: A mid-transition height is kept as it is
- **WHEN** `H` falls between the first and the last transaction of an epoch transition
- **THEN** the entry for `H` is retained like any other, holding the complete rewarded set and the epoch id of the ending epoch

### Requirement: Node families are read beside each snapshot

At each height `H` it snapshots, the producer SHALL also read from the node families contract, every query pinned at `H`, every family, every family member and every pending invitation, and SHALL join them with the node details of the same height into the families data it serves. They SHALL be held with the retained entry for `H` under the same rules as the mixnet contract state: unsigned, never served under an attestation route tree, and a failed read retaining nothing for `H`.

There SHALL be no separately refreshed node families cache, and no `[node_families_cache]` setting SHALL be read. A config file that still holds that section SHALL load.

#### Scenario: Families and stake are from one height
- **WHEN** a family member's stake changes one block after `H`
- **THEN** the families data held for `H` reports the member's stake as it stood at `H`

#### Scenario: A failed families read retains nothing
- **WHEN** the pending invitations read at `H` fails
- **THEN** nothing is retained for `H` and the next poll tries the newest cadence height again

### Requirement: Node age is counted in blocks

A node's age SHALL be the number of blocks between its bonding height and the snapshot's height multiplied by a fixed block time of 5.687 s, and a family's average node age the mean of its members' ages. nym-api MUST NOT look up or estimate a block's timestamp for it and MUST NOT read its own clock for it. The same block time SHALL be used on every network.

#### Scenario: An age from heights alone
- **WHEN** a family's only member bonded at height 26,000,000 and the snapshot's height is 26,100,000
- **THEN** the family's average node age is 568,700 s

#### Scenario: Every instance reports the same age
- **WHEN** two instances serve the same settled height, one of them through an RPC node that pruned the member's bonding height
- **THEN** both report the same average node age

### Requirement: Epoch, rewarded set, bonded node and family routes are served from the latest settled snapshot

Every route that reads mixnet contract state or node families data SHALL read the values held for the latest settled snapshot: among them the current epoch, the reward parameters and the key rotation information, the rewarded set, the bonded nodes and their stake saturation, the config score details, the mixnet-derived part of the circulating supply, and the node families routes. While no snapshot has settled these routes MUST answer 503.

#### Scenario: An epoch change is served once its snapshot settles
- **WHEN** the epoch advances on chain at a height after the latest settled snapshot
- **THEN** the current-epoch and rewarded-set routes keep answering with the previous epoch until a snapshot taken after the advance has settled

#### Scenario: Two instances agree on the epoch
- **WHEN** two instances have settled the same height
- **THEN** their current-epoch, key-rotation-information and rewarded-set responses are identical

#### Scenario: Nothing has settled
- **WHEN** no snapshot has settled
- **THEN** these routes answer 503

### Requirement: Legacy mixnode and gateway bonds are not read

nym-api MUST NOT read legacy mixnode or gateway bonds from the mixnet contract. `GET /v1/legacy/mixnodes` and `GET /v1/legacy/gateways` SHALL keep their response types, SHALL be marked deprecated, and SHALL always answer with a count of 0 and no nodes.

#### Scenario: The legacy routes answer empty
- **WHEN** a client asks `GET /v1/legacy/mixnodes` or `GET /v1/legacy/gateways`, whatever legacy bonds the contract still holds
- **THEN** it receives 200 with `count` 0 and an empty `nodes` list

### Requirement: Account lookups price live reads with the snapshot's reward state

The account route SHALL keep reading an address's balance and delegations from the chain when asked, with its per-address cache, and SHALL take the reward state of the nodes delegated to from the latest settled snapshot. Its answers are not required to be identical across instances.

#### Scenario: Reward state comes from the settled height
- **WHEN** an address delegating to node 7 is looked up
- **THEN** its delegation is read from the chain at the time of the request and valued with node 7's reward state as held for the latest settled snapshot

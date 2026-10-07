# network-monitors-contract Specification

## Purpose
The on-chain authorisation registry for the network-monitor stress-testing fleet: a three-tier admin/orchestrator/agent hierarchy that decides who may probe Nym nodes. It records authorised orchestrators (with their announced ed25519 identity keys) and authorised agents (by socket address and x25519 noise key), and is the shared, auditable source of truth that nym-api reads to authorise result submissions and that every nym-node reads to decide whose probe traffic to accept.
## Requirements
### Requirement: The contract is a three-tier authorisation registry whose admin is fixed at instantiation

The `network-monitors` contract SHALL maintain a chain-backed authorisation hierarchy with three tiers: a single contract admin (in production the Nymtech SA multisig / governance), a set of authorised network-monitor orchestrators, and a set of authorised network-monitor agents. The admin authorises and revokes orchestrators; an orchestrator authorises and revokes agents; an authorised agent is thereby permitted to send stress-test packets to Nym nodes. State MUST be held in exactly three stores: a cw-controllers `Admin` under storage key `contract-admin`, `authorised_orchestrators: Map<&OrchestratorAddress, AuthorisedNetworkMonitorOrchestrator>` under `authorised-orchestrators`, and `authorised_agents: Map<AgentStorageKey, AuthorisedNetworkMonitor>` under `authorised-network-monitors`.

On `instantiate`, the contract MUST set the admin to the message sender (`info.sender`), NOT to a field of the message, and MUST save exactly one initial orchestrator taken from `InstantiateMsg { orchestrator_address }` with `identity_key = None` and `authorised_at = env.block.time`. `instantiate` MUST also record the cw2 contract name (`crate:nym-network-monitors-contract`) and version and set build information. No other configuration is stored.

An `AuthorisedNetworkMonitorOrchestrator` MUST carry `{ address, identity_key: Option<String>, authorised_at }`; an `AuthorisedNetworkMonitor` (agent) MUST carry `{ mixnet_address: SocketAddr, authorised_by, authorised_at, bs58_x25519_noise, noise_version }`.

#### Scenario: Instantiation seeds the admin and the first orchestrator
- **WHEN** the contract is instantiated by an account with `InstantiateMsg { orchestrator_address }`
- **THEN** the admin is set to that instantiating account
- **AND** one orchestrator entry for `orchestrator_address` is stored with `identity_key = None` and `authorised_at` equal to the block time

#### Scenario: The three tiers are distinct authorities
- **WHEN** the registry is inspected
- **THEN** it exposes an admin, a set of orchestrators, and a set of agents, where each agent records which orchestrator authorised it

### Requirement: Only the admin may authorise or revoke orchestrators, and revocation cascades to that orchestrator's agents

`AuthoriseNetworkMonitorOrchestrator { address }` MUST be admin-only (else a cw-controllers admin error). It MUST be a no-op when `address` is already an orchestrator, preserving that entry's original `authorised_at` and `identity_key`; otherwise it MUST save a new entry with `identity_key = None` and `authorised_at = env.block.time`.

`RevokeNetworkMonitorOrchestrator { address }` MUST be admin-only. It MUST remove the orchestrator entry (a no-op if absent) and MUST cascade-delete every agent whose `authorised_by` equals that orchestrator address. This cascade iterates the whole agent map in a single transaction; the in-source `TODO` noting that a very large agent set could exceed a single block's gas is recorded as a known scaling limitation, not current-behaviour risk at present cardinality. Note when reasoning about that cardinality that an agent occupies one entry per announced address, so the map holds roughly two entries per agent rather than one.

`UpdateAdmin { admin }` MUST transfer the admin role to the validated `admin` address via the cw-controllers `Admin`. Because the message field is a required `String`, the admin can be transferred but can never be cleared through the contract's message surface.

#### Scenario: Non-admin cannot authorise an orchestrator
- **WHEN** a non-admin account sends `AuthoriseNetworkMonitorOrchestrator`
- **THEN** the call fails with an admin authorisation error and no orchestrator is added

#### Scenario: Re-authorising an existing orchestrator is a strict no-op
- **WHEN** the admin authorises an address that is already an orchestrator
- **THEN** the existing entry is left untouched, retaining its original `authorised_at` and any announced `identity_key`

#### Scenario: Revoking an orchestrator removes only its agents
- **WHEN** the admin revokes an orchestrator that had authorised some agents
- **THEN** the orchestrator entry is removed and every agent it had authorised is deleted, while agents authorised by other orchestrators remain

### Requirement: An authorised orchestrator self-announces its ed25519 identity key, validated by shape only

`UpdateOrchestratorIdentityKey { key }` MUST update only the calling account's own orchestrator entry. Authorisation is implicit: the sender MUST already have an orchestrator entry, otherwise the call fails with `NotAnOrchestrator`. The `key` MUST be validated as base58 decoding to exactly 32 bytes (an ed25519 public key), failing with `MalformedEd25519OrchestratorIdentityKey` otherwise; the key's validity as a curve point MUST NOT be checked on-chain (a malformed key simply fails downstream signature verification). The validated key MUST be stored verbatim, overwriting any previously announced key.

This announced identity key is the mechanism by which off-chain consumers (notably nym-api) learn the ed25519 public key against which an orchestrator's signed submissions are verified.

#### Scenario: A non-orchestrator cannot announce an identity key
- **WHEN** an account that is not an authorised orchestrator sends `UpdateOrchestratorIdentityKey`
- **THEN** the call fails with `NotAnOrchestrator`

#### Scenario: A malformed key is rejected on shape
- **WHEN** an orchestrator submits a `key` that is not valid base58 or does not decode to exactly 32 bytes
- **THEN** the call fails with `MalformedEd25519OrchestratorIdentityKey`

#### Scenario: A valid key overwrites the previous one
- **WHEN** an authorised orchestrator submits a well-formed 32-byte base58 key
- **THEN** its own entry's `identity_key` is set to that value, replacing any prior key, without a curve-point check

### Requirement: Only an authorised orchestrator may authorise agents, keyed by socket address as an upsert

`AuthoriseNetworkMonitor { mixnet_address, bs58_x25519_noise, noise_version, bs58_ed25519_identity }` MUST be orchestrator-only, failing with `NotAnOrchestrator` for any other sender. `bs58_x25519_noise` MUST be validated as base58 decoding to exactly 32 bytes (an x25519 noise key), failing with `MalformedX25519AgentNoiseKey` otherwise. On success it MUST save an `AuthorisedNetworkMonitor` keyed by `mixnet_address`, recording `authorised_by = info.sender`, `authorised_at = env.block.time`, and the supplied noise key and version. The save MUST be an upsert: re-authorising the same socket address renews the entry (including `authorised_at`), in contrast to orchestrator authorisation which is a no-op for an existing entry.

`bs58_ed25519_identity` MUST be OPTIONAL, and when present MUST be validated as base58 decoding to exactly 32 bytes (an ed25519 public key), failing with a dedicated malformed-identity error otherwise. It records the ed25519 client identity the agent presents when it opens a gateway client session, which is what allows a gateway to grant an unmetered monitor session against a cryptographically verified identity instead of a source IP. The contract MUST NOT require it, MUST NOT infer it, and MUST NOT treat its absence as an error: an entry without one is a validly authorised agent that simply cannot be recognised on the client-session path. Because the save is an upsert and agents re-announce before every test run, entries written before the field existed acquire it without any data migration or backfill.

The contract places NO uniqueness constraint on `bs58_x25519_noise` OR on `bs58_ed25519_identity`: the same noise key MAY appear under several socket addresses, and does so by design, because a single agent authorises one ipv4 and one ipv6 address so that nodes accept its probes over either family. The registry therefore holds roughly TWO entries per agent, both carrying that agent's noise key and, once announced, the same identity key, and nothing on-chain records that a pair of entries belongs to one agent.

An off-chain consumer that needs to recover which entries belong to one agent MUST group them by that noise key; the two entries of one agent are NOT adjacent in the pagination order, which sorts ipv4 before ipv6. The nym-network-monitor orchestrator does exactly this when it rehydrates its agent cache after a restart. That grouping is only sound as long as distinct agents never share a noise key, and the contract does not enforce it, so this is an assumption held by the consumer rather than an on-chain guarantee. A consumer that builds a set of authorised monitor identities MUST likewise tolerate the same identity arriving from several entries.

#### Scenario: One agent's two addresses are two independent entries
- **WHEN** an orchestrator authorises one ipv4 and one ipv6 address for the same agent, both with its noise key
- **THEN** the registry holds two entries sharing that noise key, each independently revocable, with nothing on-chain marking them as one agent

#### Scenario: Only orchestrators can authorise agents
- **WHEN** an account that is not an authorised orchestrator sends `AuthoriseNetworkMonitor`
- **THEN** the call fails with `NotAnOrchestrator`

#### Scenario: A malformed noise key is rejected on shape
- **WHEN** the supplied `bs58_x25519_noise` is not valid base58 or does not decode to exactly 32 bytes
- **THEN** the call fails with `MalformedX25519AgentNoiseKey`

#### Scenario: Re-authorising the same agent renews the entry
- **WHEN** an orchestrator authorises an agent for a socket address that already has an entry
- **THEN** the entry is overwritten with the new `authorised_by`, `authorised_at`, noise key, version, and identity key

#### Scenario: An omitted identity key is accepted
- **WHEN** an orchestrator authorises an agent without supplying `bs58_ed25519_identity`
- **THEN** the entry is saved with no identity recorded, and the agent is authorised for every gate that does not depend on one

#### Scenario: A malformed identity key is rejected on shape
- **WHEN** the supplied `bs58_ed25519_identity` is not valid base58 or does not decode to exactly 32 bytes
- **THEN** the call fails with a malformed-identity error and nothing is saved

#### Scenario: An entry predating the field acquires it on the next announcement
- **WHEN** an agent whose stored entry has no identity re-announces and is authorised again
- **THEN** the upsert records its identity, with no data migration involved

### Requirement: Agents may be revoked individually or wholesale by the admin or any orchestrator

`RevokeNetworkMonitor { address }` MUST succeed for the admin or any authorised orchestrator and MUST fail with `Unauthorized` for anyone else; it removes the agent entry for `address` (a no-op if absent). `RevokeAllNetworkMonitors` MUST likewise be restricted to the admin or any authorised orchestrator (else `Unauthorized`) and MUST clear the entire agent map regardless of which orchestrator authorised each agent.

#### Scenario: An orchestrator revokes a single agent
- **WHEN** an authorised orchestrator sends `RevokeNetworkMonitor` for an existing agent socket address
- **THEN** that agent entry is removed

#### Scenario: Wholesale revocation wipes every agent
- **WHEN** the admin or an orchestrator sends `RevokeAllNetworkMonitors`
- **THEN** all agent entries are removed, including those authorised by other orchestrators

#### Scenario: An unrelated account cannot revoke agents
- **WHEN** an account that is neither the admin nor an orchestrator sends `RevokeNetworkMonitor` or `RevokeAllNetworkMonitors`
- **THEN** the call fails with `Unauthorized`

### Requirement: A revoked orchestrator loses all agent-management authority

Once an orchestrator's entry has been removed, that account MUST no longer be able to authorise agents, update an identity key, or revoke agents; such calls MUST fail with `NotAnOrchestrator` or `Unauthorized` as appropriate. Agent-management authority is derived solely from the presence of the caller's orchestrator entry, so revocation is immediate on the next call.

#### Scenario: A revoked orchestrator cannot authorise agents
- **WHEN** an orchestrator is revoked and then attempts `AuthoriseNetworkMonitor` or `UpdateOrchestratorIdentityKey`
- **THEN** the call fails with `NotAnOrchestrator`

### Requirement: The registry is read through three queries with no single-address membership lookup

The contract SHALL expose exactly three queries. `Admin {}` MUST return the current admin. `NetworkMonitorOrchestrators {}` MUST return all orchestrators ascending by address with NO pagination (the set is expected to stay small). `NetworkMonitorAgents { start_next_after, limit }` MUST be paginated with a default limit of 100 and a hard maximum of 200; `start_next_after` MUST be an exclusive cursor and the response MUST carry a next-page cursor equal to the last returned agent's `mixnet_address`, or `None` when the page is empty.

There MUST be no dedicated "is this address an authorised agent" query; a consumer answering that question MUST page through `NetworkMonitorAgents` (and typically cache the result). The agent primary key MUST order deterministically by socket address (IPv4 before IPv6, then by IP octets, then by port), and this ordering defines the pagination sequence.

#### Scenario: Orchestrators are returned unpaginated
- **WHEN** `NetworkMonitorOrchestrators {}` is queried
- **THEN** every orchestrator is returned in ascending address order in a single response

#### Scenario: Agents are paginated with a capped limit
- **WHEN** `NetworkMonitorAgents { start_next_after, limit }` is queried with a `limit` above 200
- **THEN** at most 200 agents are returned, starting strictly after the `start_next_after` cursor, with a next cursor equal to the last returned agent's socket address

#### Scenario: Membership is derived by paging
- **WHEN** a consumer needs to know whether a given socket address is authorised
- **THEN** it must page through `NetworkMonitorAgents`, because no single-address membership query exists

### Requirement: Migration refreshes build information only

`MigrateMsg` MUST be an empty message. `migrate` MUST refresh build information and MUST guard against a downgrade or a wrong contract name via cw2 (`ensure_from_older_version`), and MUST perform no data migration. The `queued_migrations` module MUST contain no migration logic.

This MUST remain true across the addition of the optional agent identity key. That field is deliberately shaped so that no stored entry needs rewriting: an absent value deserialises as `None` under the new schema, and the existing upsert populates it as agents re-announce. A future contract change that cannot be expressed this way MUST add its logic to `queued_migrations` rather than relaxing this requirement silently.

#### Scenario: Migration performs no data changes
- **WHEN** the contract is migrated to a newer version of the same contract
- **THEN** build information is refreshed, the cw2 version guard passes, and no stored orchestrator or agent data is altered

#### Scenario: Migration performs no data rewrite
- **WHEN** the contract is migrated to a version carrying the optional agent identity key
- **THEN** build information is refreshed, the cw2 version guard runs, and no agent entry is read or rewritten

#### Scenario: Pre-existing entries remain readable after the migration
- **WHEN** an agent entry stored before the migration is queried afterwards
- **THEN** it deserialises with no identity key and every other field unchanged

### Requirement: Schema evolution MUST remain parseable by un-upgraded consumers

The contract's message and response types SHALL only ever be extended in ways that an un-upgraded third-party consumer can still parse. Concretely: a new field on an existing `ExecuteMsg` variant, or on a stored type carried in a query response, is PERMITTED and MUST be optional; introducing a NEW `ExecuteMsg` variant, or changing the type of an existing field, MUST be treated as a breaking fleet change and MUST NOT be used to deliver behaviour that un-upgraded nodes are required to keep observing.

The reason is asymmetric failure. Contract types use `cw_serde`, which does NOT set `deny_unknown_fields`, so a consumer compiled against an older schema silently ignores an unknown field on a variant it recognises and continues to apply the message. A consumer that meets an unrecognised variant, or a field whose type no longer matches, fails deserialisation instead; a Nym node's contract event handler treats that failure as non-fatal, logs that the schema may have changed, and continues processing later blocks. The observable result is not a loud error but a node that has silently stopped learning about agent authorisations and revocations, keeping a stale replay bypass for a revoked address indefinitely.

A change that genuinely requires a new or retyped variant MUST therefore be staged: add the new form alongside the old, wait for the node fleet to carry it, and only then retire the old form.

#### Scenario: An added optional field does not disturb an un-upgraded node
- **WHEN** an orchestrator sends `AuthoriseNetworkMonitor` carrying a field that a node's compiled schema does not know
- **THEN** that node parses the message, ignores the unknown field, and still authorises the agent

#### Scenario: A new variant would silently strand an un-upgraded node
- **WHEN** a hypothetical new `ExecuteMsg` variant is used to authorise or revoke an agent
- **THEN** an un-upgraded node fails to deserialise it, logs that the schema may have changed, continues with later blocks, and never applies the authorisation or revocation


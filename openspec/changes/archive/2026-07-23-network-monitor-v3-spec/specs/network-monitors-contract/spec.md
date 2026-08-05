## ADDED Requirements

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

`RevokeNetworkMonitorOrchestrator { address }` MUST be admin-only. It MUST remove the orchestrator entry (a no-op if absent) and MUST cascade-delete every agent whose `authorised_by` equals that orchestrator address. This cascade iterates the whole agent map in a single transaction; the in-source `TODO` noting that a very large agent set could exceed a single block's gas is recorded as a known scaling limitation, not current-behaviour risk at present cardinality.

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

`AuthoriseNetworkMonitor { mixnet_address, bs58_x25519_noise, noise_version }` MUST be orchestrator-only, failing with `NotAnOrchestrator` for any other sender. `bs58_x25519_noise` MUST be validated as base58 decoding to exactly 32 bytes (an x25519 noise key), failing with `MalformedX25519AgentNoiseKey` otherwise. On success it MUST save an `AuthorisedNetworkMonitor` keyed by `mixnet_address`, recording `authorised_by = info.sender`, `authorised_at = env.block.time`, and the supplied noise key and version. The save MUST be an upsert: re-authorising the same socket address renews the entry (including `authorised_at`), in contrast to orchestrator authorisation which is a no-op for an existing entry.

#### Scenario: Only orchestrators can authorise agents
- **WHEN** an account that is not an authorised orchestrator sends `AuthoriseNetworkMonitor`
- **THEN** the call fails with `NotAnOrchestrator`

#### Scenario: A malformed noise key is rejected on shape
- **WHEN** the supplied `bs58_x25519_noise` is not valid base58 or does not decode to exactly 32 bytes
- **THEN** the call fails with `MalformedX25519AgentNoiseKey`

#### Scenario: Re-authorising the same agent renews the entry
- **WHEN** an orchestrator authorises an agent for a socket address that already has an entry
- **THEN** the entry is overwritten with the new `authorised_by`, `authorised_at`, noise key, and version

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

#### Scenario: Migration performs no data changes
- **WHEN** the contract is migrated to a newer version of the same contract
- **THEN** build information is refreshed, the cw2 version guard passes, and no stored orchestrator or agent data is altered

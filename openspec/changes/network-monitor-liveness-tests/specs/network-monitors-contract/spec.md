## ADDED Requirements

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

## MODIFIED Requirements

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

### Requirement: Migration refreshes build information only

`MigrateMsg` MUST be an empty message. `migrate` MUST refresh build information and MUST guard against a downgrade or a wrong contract name via cw2 (`ensure_from_older_version`), and MUST perform no data migration. The `queued_migrations` module MUST contain no migration logic.

This MUST remain true across the addition of the optional agent identity key. That field is deliberately shaped so that no stored entry needs rewriting: an absent value deserialises as `None` under the new schema, and the existing upsert populates it as agents re-announce. A future contract change that cannot be expressed this way MUST add its logic to `queued_migrations` rather than relaxing this requirement silently.

#### Scenario: Migration performs no data rewrite
- **WHEN** the contract is migrated to a version carrying the optional agent identity key
- **THEN** build information is refreshed, the cw2 version guard runs, and no agent entry is read or rewritten

#### Scenario: Pre-existing entries remain readable after the migration
- **WHEN** an agent entry stored before the migration is queried afterwards
- **THEN** it deserialises with no identity key and every other field unchanged

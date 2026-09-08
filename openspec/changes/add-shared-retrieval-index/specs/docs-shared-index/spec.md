# docs-shared-index Spec Delta

## ADDED Requirements

### Requirement: Shard production from producer repos

Each producer repo (`nym`, `nym-vpn-client`, `websites`) SHALL build its own index shard by running the shared indexer in its CI and upserting chunks to the shared Postgres. Chunks SHALL be keyed by a stable id and carry a content hash computed over the embed text, model, and dimension; a chunk whose hash is unchanged SHALL NOT be re-embedded, and a model or dimension change SHALL invalidate every hash in the shard.

#### Scenario: Unchanged content skips embedding

- **WHEN** a producer repo runs the indexer and a chunk's content hash matches the stored row
- **THEN** the row is left as is and no embedding request is made

#### Scenario: Removed content leaves the index

- **WHEN** a source file or section no longer produces a chunk id that exists in that repo's shard
- **THEN** the indexer deletes the stale row in the same run

### Requirement: Visibility enforced by database roles

Every chunk SHALL carry a `visibility` value (`public` or `private`). The public MCP server's database role SHALL be able to read only `public` rows, enforced by the database (view grant or row-level security), not by application filtering. A producer role SHALL NOT be able to write rows outside its permitted visibility.

#### Scenario: Public endpoint cannot return private chunks

- **WHEN** any query is executed through the public MCP server, regardless of application-code behaviour
- **THEN** the database returns only rows with `visibility = 'public'`

#### Scenario: Private repo cannot publish

- **WHEN** the `websites` CI role attempts to upsert a chunk with `visibility = 'public'`
- **THEN** the database rejects the write

### Requirement: Public MCP surface unchanged

The public MCP endpoint SHALL keep its URL, tool names, argument schemas, and result shapes across the storage migration. `search_docs` and `search_code` SHALL only be exposed when their backing shard contains chunks.

#### Scenario: Existing client after cutover

- **WHEN** a client configured for `https://nym.com/docs/api/mcp` calls `search_docs` after the migration
- **THEN** the call succeeds with the same request and response shapes as before

#### Scenario: Absent or empty shard

- **WHEN** a shard is absent or contains no chunks
- **THEN** the tools backed by that shard are not listed

### Requirement: Private endpoint access

The private MCP endpoint SHALL require a per-person token on every request. Tokens SHALL carry an expiry and SHALL be revocable without redeploying the service. The private endpoint SHALL serve public and private rows through the same tool surface as the public endpoint.

#### Scenario: Expired or revoked token

- **WHEN** a request presents a token that is expired or has been revoked
- **THEN** the request is refused and no query is executed

#### Scenario: External access window ends

- **WHEN** an external reviewer's token passes its expiry
- **THEN** their access ends with no operator action

### Requirement: Per-shard embedding space

Each shard SHALL record the embedding model and dimension used to build it. Queries against a shard SHALL be embedded with that shard's recorded model. Results from shards built with different models SHALL NOT be merged by raw similarity score.

#### Scenario: Mixed-model deployment

- **WHEN** the private shard uses a different embedding model than the public shards
- **THEN** each query is embedded per shard with the matching model and any cross-shard combination is rank-based

### Requirement: Shard freshness is observable

Each shard SHALL expose the timestamp of its last successful index run, and the verification tooling SHALL fail when a shard is stale beyond a configured threshold.

#### Scenario: Silent producer failure

- **WHEN** a producer repo's index CI has not completed successfully within the threshold
- **THEN** the verification check reports that shard as stale and fails

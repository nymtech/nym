# docs-shared-index Spec Delta

## ADDED Requirements

### Requirement: Shard production from producer repos

The public producer repos (`nym`, `nym-vpn-client`) SHALL build their shards by running the shared indexer in their CI and upserting chunks to the public store.

A shard is the set of chunks one producer writes for one repo and one modality. A run owns its shards and, in the same run, deletes rows in them whose ids it no longer produces; it never touches another producer's shards. Chunks SHALL be keyed by a stable id and carry a content hash computed over the embed text, model, and dimension; a chunk whose hash is unchanged SHALL NOT be re-embedded, and a model or dimension change SHALL invalidate every hash in the shard. A chunk id SHALL be unique within the store.

#### Scenario: Unchanged content skips embedding

- **WHEN** a producer repo runs the indexer and a chunk's content hash matches the stored row
- **THEN** the row is left as is and no embedding request is made

#### Scenario: Removed content leaves the index

- **WHEN** a source file or section no longer produces a chunk id that exists in that repo's shard
- **THEN** the indexer deletes the stale row in the same run

### Requirement: Complete language coverage, no silent drops

The indexer SHALL chunk every source language present under a producer repo's configured roots. When it meets a file it cannot chunk, it SHALL report that file by extension and count in the run output; it SHALL NOT skip source silently. A producer repo SHALL either add chunking support for an unhandled language found in its roots or exclude those paths from its roots, so that a language's absence from the index is always a recorded decision, never an accident.

#### Scenario: Unhandled language is surfaced, not dropped

- **WHEN** a configured root holds source files in a language the chunker does not handle (for example Kotlin or Swift in `nym-vpn-client`)
- **THEN** the run reports those files by extension and count, and does not publish a shard that silently omits them

#### Scenario: A root of source yields no chunks

- **WHEN** a configured root contains files that look like source but the run produces no chunks for that root
- **THEN** the run fails rather than upserting an empty-but-valid shard for that root

### Requirement: The public store holds no private row

The public store SHALL contain only public content. The public MCP server SHALL connect only to the public store, so no query it executes can return private content, because that content is not present in the store it reads. No producer path SHALL write private content into the public store; private content is in a separate store.

#### Scenario: Public endpoint cannot return private chunks

- **WHEN** any query is executed through the public MCP server, regardless of application-code behaviour
- **THEN** it reads the public store, which contains no private rows, so no private content can be returned

#### Scenario: A producer writes only public content

- **WHEN** a public producer repo runs the indexer
- **THEN** it upserts chunks of its own public sources only

### Requirement: Public MCP surface unchanged

The public MCP endpoint SHALL keep its URL, tool names, argument schemas, and result shapes across the storage migration. `search_docs` and `search_code` SHALL each be exposed when any shard backing that modality contains chunks, so adding a producer whose shard is not yet built never delists a tool.

#### Scenario: Existing client after cutover

- **WHEN** a client configured for `https://nym.com/docs/api/mcp` calls `search_docs` after the migration
- **THEN** the call succeeds with the same request and response shapes as before

#### Scenario: A new producer's shard is not yet built

- **WHEN** one producer's code shard is absent but another producer's code shard has chunks
- **THEN** `search_code` stays listed; a tool is delisted only when no shard of its modality has any chunks

### Requirement: Per-modality embedding space

Within the store, all rows of one modality SHALL be embedded with one model, recorded on the shard; prose and code MAY use different models (the public store keeps one for prose and one for code). A query for a modality SHALL be embedded with that modality's model and scanned only against those rows. Rows embedded with different models SHALL NOT be compared by raw similarity score; any cross-model combination SHALL be by rank.

#### Scenario: A query is scored only within its model's space

- **WHEN** `search_code` runs against a store
- **THEN** the query is embedded with the model the code rows were built with, and scored only against code rows, never against prose rows

### Requirement: Shard freshness is observable

Each shard SHALL expose the timestamp of its last successful index run, and the verification tooling SHALL fail when a shard is stale beyond a configured threshold.

#### Scenario: Silent producer failure

- **WHEN** a producer repo's index CI has not completed successfully within the threshold
- **THEN** the verification check reports that shard as stale and fails

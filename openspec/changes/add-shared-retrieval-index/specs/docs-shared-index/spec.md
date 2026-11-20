# docs-shared-index Spec Delta

## ADDED Requirements

### Requirement: Shard production from producer repos

The public producer repos (`nym`, `nym-vpn-client`) SHALL build their public shards by running the shared indexer in their CI and upserting chunks to the public store. The private store SHALL be filled by a pipeline on the private infrastructure that runs the same shared indexer over the private `websites` source and over the public repos, embedding all of it with the private model.

A shard is the set of chunks one producer writes for one repo and one modality into one store. A run owns its shards and, in the same run, deletes rows in them whose ids it no longer produces; it never touches another producer's shards. The private-infrastructure pipeline owns every shard in the private store, including the re-embedded public copy. Chunks SHALL be keyed by a stable id and carry a content hash computed over the embed text, model, and dimension; a chunk whose hash is unchanged SHALL NOT be re-embedded, and a model or dimension change SHALL invalidate every hash in the shard. A chunk id SHALL be unique within a store but not across stores: a public chunk exists in both stores under the same id, so an id is a per-store handle and results SHALL NOT be deduplicated across stores by id. A shared id normally maps to the same text in both stores, since both run the same indexer, but the two pipelines deploy independently and may run different indexer versions, so callers SHALL NOT rely on cross-store text identity and SHALL NOT assume an id fetched from one store resolves against the other.

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

### Requirement: Visibility enforced by physical store separation

The public store SHALL contain only public content. Private content SHALL exist only in the private store. The public MCP server SHALL connect only to the public store, so no query it executes can return private content, because that content is not present in the store it reads. The private MCP server SHALL connect only to the private store, which holds the private content beside a re-embedded copy of the public content. No producer path SHALL write private content into the public store.

#### Scenario: Public endpoint cannot return private chunks

- **WHEN** any query is executed through the public MCP server, regardless of application-code behaviour
- **THEN** it reads the public store, which contains no private rows, so no private content can be returned

#### Scenario: Private content cannot reach the public store

- **WHEN** the private-infrastructure pipeline indexes the `websites` source
- **THEN** those chunks are upserted only to the private store, and no producer path upserts them to the public store

### Requirement: Public MCP surface unchanged

The public MCP endpoint SHALL keep its URL, tool names, argument schemas, and result shapes across the storage migration. `search_docs` and `search_code` SHALL each be exposed when any shard backing that modality contains chunks, so adding a producer whose shard is not yet built never delists a tool.

#### Scenario: Existing client after cutover

- **WHEN** a client configured for `https://nym.com/docs/api/mcp` calls `search_docs` after the migration
- **THEN** the call succeeds with the same request and response shapes as before

#### Scenario: A new producer's shard is not yet built

- **WHEN** one producer's code shard is absent but another producer's code shard has chunks
- **THEN** `search_code` stays listed; a tool is delisted only when no shard of its modality has any chunks

### Requirement: Private endpoint access

The private MCP endpoint SHALL require a per-person token on every request. Tokens SHALL carry an expiry and SHALL be revocable without redeploying the service. The private endpoint SHALL serve public and private rows through the same tool surface as the public endpoint.

#### Scenario: Expired or revoked token

- **WHEN** a request presents a token that is expired or has been revoked
- **THEN** the request is refused and no query is executed

#### Scenario: External access window ends

- **WHEN** an external reviewer's token passes its expiry
- **THEN** their access ends with no operator action

### Requirement: Per-modality embedding space

Within a store, all rows of one modality SHALL be embedded with one model, recorded on the shard; prose and code MAY use different models (the public store keeps one for prose and one for code). A query for a modality SHALL be embedded with that (store, modality) model and scanned only against those rows. Rows embedded with different models SHALL NOT be compared by raw similarity score, and the public and private stores SHALL NOT be queried together in one request; any cross-model combination SHALL be by rank.

#### Scenario: A query is scored only within its model's space

- **WHEN** `search_code` runs against a store
- **THEN** the query is embedded with the model that store's code rows were built with, and scored only against code rows, never against prose rows or another store's rows

#### Scenario: Private store may collapse to one space

- **WHEN** the private store embeds both prose and code with a single model (Q1 Option A)
- **THEN** both tools share that one model and the private MCP answers from one embedding space

### Requirement: Shard freshness is observable

Each shard SHALL expose the timestamp of its last successful index run, and the verification tooling SHALL fail when a shard is stale beyond a configured threshold. For the private store's copy of the public content, freshness SHALL be measured against the public sources it copies, by a model-independent source hash or source commit (not the chunk content hash, which folds in the model and so never matches across the two stores), so a run that completed against an older revision is still caught as lagging.

#### Scenario: Silent producer failure

- **WHEN** a producer repo's index CI has not completed successfully within the threshold
- **THEN** the verification check reports that shard as stale and fails

#### Scenario: Private copy lags the public source

- **WHEN** the private store's copy of the public content was last built against an older commit than the current public source, though its run timestamp is recent
- **THEN** the source-content check flags the private copy as stale and fails

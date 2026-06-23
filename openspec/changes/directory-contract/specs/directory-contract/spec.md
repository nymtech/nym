## ADDED Requirements

### Requirement: Namespaced key space
The contract SHALL partition stored entries into explicitly-namespaced classes (initially `node` entries and `curated` entries), with the namespace discriminant forming part of both the storage key and the digest leaf. The namespace SHALL be extensible to future entry types. Keys from different namespaces SHALL NOT collide, and the contract SHALL route write authorization by namespace.

#### Scenario: Cross-namespace keys do not collide
- **WHEN** a node entry and a curated entry share the same id and label
- **THEN** they remain distinct entries with distinct storage keys and distinct digest leaves

#### Scenario: Authorization routed by namespace
- **WHEN** a write targets the node namespace versus the curated namespace
- **THEN** the contract applies identity-key-signature authorization for node entries and admin authorization for curated entries

### Requirement: Node configuration entries
The contract SHALL store node-published configuration as opaque bytes keyed by `(node_id, label)` within the node namespace, where each entry records the data, the block height it was last updated, and the authoring ed25519 signature. The contract SHALL NOT interpret the byte payload.

#### Scenario: Node publishes an entry
- **WHEN** a transaction carries data for `(node_id, label)` with a valid ed25519 signature over `node_id || label || sequence || data` by the node's identity key, the node is bonded and not unbonding, `label` is allowed, and `data` is within the label's `max_size`
- **THEN** the contract stores `{ data, updated_at = current height, signature }`, advances the node's sequence, updates the global digest, and the entry is returned verbatim by a later query

#### Scenario: Opaque payload preserved
- **WHEN** a node publishes arbitrary bytes under an allowed label
- **THEN** the contract stores and returns the exact bytes without parsing or validating their schema

### Requirement: Identity-key write authorization
A write or self-delete SHALL be authorized solely by an ed25519 signature from the node's identity key (fetched from the mixnet bond and base58-decoded to 32 bytes), independent of the transaction sender. The contract SHALL reject the operation if the signature does not verify.

#### Scenario: Valid signature from any relayer
- **WHEN** any account submits a node's correctly-signed write
- **THEN** the contract accepts it regardless of who sent the transaction

#### Scenario: Invalid signature rejected
- **WHEN** a write carries a signature that does not verify against the node's identity key
- **THEN** the contract rejects it and makes no state change

### Requirement: Per-node replay protection
The contract SHALL maintain a strictly-monotonic sequence per `node_id` and SHALL reject any write or delete whose signed sequence is not greater than the stored sequence. The signed payload SHALL bind `node_id`, `label`, and `sequence` so a signature cannot be replayed or moved to another slot. The sequence SHALL persist independently of whether any entry currently exists.

#### Scenario: Stale sequence rejected
- **WHEN** a write carries a sequence less than or equal to the node's stored sequence
- **THEN** the contract rejects it

#### Scenario: Replay after delete rejected
- **WHEN** a node deletes an entry and an old signed write for that slot (with a now-stale sequence) is replayed
- **THEN** the contract rejects it because the sequence did not increase

### Requirement: Bonded-and-not-unbonding precondition
The contract SHALL accept node writes and self-deletes only when the node is bonded and not unbonding, as reported by the mixnet contract.

#### Scenario: Unbonding node rejected
- **WHEN** a node that has begun unbonding submits a write
- **THEN** the contract rejects it

#### Scenario: Unknown node rejected
- **WHEN** a write references a `node_id` with no mixnet bond
- **THEN** the contract rejects it

### Requirement: Node self-deletion
A node SHALL be able to delete its own entry via a signed, sequence-advancing operation, and the deletion SHALL update the global digest.

#### Scenario: Entry deleted and digest updated
- **WHEN** a node submits a valid signed delete for `(node_id, label)`
- **THEN** the contract removes the entry, advances the sequence, and subtracts the entry's leaf from the global digest

### Requirement: Admin-managed label whitelist
The contract SHALL store the set of allowed labels with a per-label maximum byte size, mutable only by the admin. A label `max_size` SHALL NOT exceed the contract hard ceiling of 128 KiB. Writes under a label not in the set SHALL be rejected. Removing a label SHALL be non-destructive.

#### Scenario: Admin adds a label
- **WHEN** the admin adds a label with a `max_size` at or below 128 KiB
- **THEN** subsequent writes under that label are accepted up to that size

#### Scenario: Oversized max_size rejected
- **WHEN** the admin tries to set a label `max_size` above 128 KiB
- **THEN** the contract rejects the operation

#### Scenario: Non-admin label change rejected
- **WHEN** a non-admin account tries to add, change, or remove a label
- **THEN** the contract rejects it

#### Scenario: Non-destructive removal
- **WHEN** the admin removes a label that has existing entries
- **THEN** new writes under that label are rejected, but the existing entries remain readable and counted in the digest

### Requirement: Per-entry size enforcement
The contract SHALL reject a write whose `data` length exceeds the `max_size` configured for its label.

#### Scenario: Oversized data rejected
- **WHEN** a node writes data larger than its label's `max_size`
- **THEN** the contract rejects the write

### Requirement: Curated entries
The contract SHALL store admin-managed curated entries keyed by `(curated_id, label)` within the curated namespace over opaque bytes, writable and removable only by the admin, and SHALL fold them into the same global digest as node entries.

#### Scenario: Admin sets a curated entry
- **WHEN** the admin sets a curated entry (for example a nym-api identity key)
- **THEN** the contract stores it, updates the global digest, and it is queryable

#### Scenario: Non-admin curated write rejected
- **WHEN** a non-admin account tries to set or remove a curated entry
- **THEN** the contract rejects it

### Requirement: Global integrity digest
The contract SHALL maintain a single global incremental multiset digest (LtHash) over all entries (node and curated), with `leaf = canonical(namespace, id, label, value)` (the namespace discriminant included, length-prefixed, so cross-class entries cannot collide), updated on every write and delete using a secure (non-linear) multiset hash. Recomputing the digest over the full set of stored entries SHALL equal the stored digest.

#### Scenario: Digest updated on write
- **WHEN** an entry is added, changed, or removed
- **THEN** the global digest reflects the change by subtracting the old leaf and adding the new leaf

#### Scenario: Digest matches recomputation
- **WHEN** a consumer enumerates all entries and recomputes the multiset hash over their canonical leaves
- **THEN** the result equals the digest stored in contract state

#### Scenario: Identical data leaves digest unchanged
- **WHEN** a node re-publishes byte-identical data for an existing entry
- **THEN** the global digest is unchanged because the leaf is identical

### Requirement: Mixnet unbond callback cleanup
The contract SHALL expose an `OnNymNodeUnbond { node_id }` handler callable only by the configured mixnet contract; it SHALL delete all of that node's entries and update the digest, without unbounded iteration.

#### Scenario: Mixnet callback clears a node
- **WHEN** the configured mixnet contract invokes `OnNymNodeUnbond` for a node that has entries
- **THEN** the contract deletes that node's entries and subtracts their leaves from the global digest

#### Scenario: Unauthorized caller rejected
- **WHEN** any account other than the configured mixnet contract invokes `OnNymNodeUnbond`
- **THEN** the contract rejects it with an unauthorized-callback error

### Requirement: Directory queries
The contract SHALL provide queries for a single entry, all entries of an id, a paginated enumeration of all entries, the global digest, a node's current sequence, the label whitelist, and the contract config. Provable reads SHALL be available via raw store reads of the digest item and of individual entries at their deterministic keys.

#### Scenario: Enumerate the whole directory
- **WHEN** a consumer calls the paginated `all_entries` query
- **THEN** the contract returns entries in deterministic order across pages, covering node and curated entries

#### Scenario: Query a node's sequence
- **WHEN** a relayer queries a node's current sequence
- **THEN** the contract returns it so the next signed write can use a greater value

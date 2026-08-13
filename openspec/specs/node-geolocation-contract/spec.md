# node-geolocation-contract Specification

## Purpose
TBD - created by archiving change verifiable-node-geolocation. Update Purpose after archive.
## Requirements
### Requirement: The contract SHALL key entries by subject class, subject id and a single source discriminant

Entries MUST be keyed `(subject_class, subject_id, source)` in a single store.

`subject_class` MUST be a closed enum, so the key space is not limited to bonded nym-nodes by construction. It currently defines `NymNode` alone; adding a class is a code upgrade and a redeploy rather than an admin transaction, and MUST require no leaf-encoding change and no re-fold of existing entries, because the class lives in the key. A retired class's discriminant MUST NOT be reused.

`subject_id` MUST be opaque bytes whose encoding is fixed per class and documented once. The `NymNode` class MUST encode its id as a big-endian `u32`, so ids order numerically and decode back to `NodeId` for the unbond callback. Every class MUST fix a single id width, and an id of any other width MUST be rejected: `cw-storage-plus` length-prefixes every key component except the last, so a variable width would make that prefix vary and entries would sort by id length before id content. A class whose natural identifier is not a number MUST NOT be forced into one.

`source` MUST be a single discriminant of the form `Measured { method, agent }`, `SelfDeclared`, or `Override`, so that combinations which are meaningless are not representable. `method` MUST be a closed enum; adding a measurement source MUST be a single new variant requiring no leaf-encoding change, because it lives in the key. A retired variant's discriminant MUST NOT be reused.

`Measured` MUST carry the measuring agent, so each authorised agent occupies its own slot and concurrent agents never overwrite one another. `SelfDeclared` MUST carry no writer component, so a subject has exactly one self-declared slot regardless of which agent relayed it. `Override` MUST likewise carry no writer component and MUST name the admin role rather than an address, so transferring the admin role MUST NOT orphan existing overrides.

#### Scenario: Two agents measuring the same node both retain their answers
- **GIVEN** two whitelisted agents A and B
- **WHEN** both submit an `IpInfo` measurement for node 42 reporting different countries
- **THEN** both entries exist under `Measured { IpInfo, A }` and `Measured { IpInfo, B }` for that subject, and neither overwrites the other

#### Scenario: Two agents relaying the same self-declaration converge on one slot
- **GIVEN** two whitelisted agents that both fetch node 42's signed `NymNodeLocation` artifact
- **WHEN** both relay it
- **THEN** both writes target the single `SelfDeclared` slot for that subject and the store holds exactly one entry, not two

#### Scenario: Admin rotation preserves overrides
- **GIVEN** an override entry for node 42
- **WHEN** the admin role is transferred to a different address
- **THEN** the override remains readable under the same key

#### Scenario: Node ids order numerically
- **GIVEN** entries for nym-nodes 9 and 10
- **WHEN** the store is scanned in ascending key order
- **THEN** node 9 precedes node 10

#### Scenario: A subject id of the wrong width for its class is rejected
- **WHEN** an entry is submitted whose subject id is not the fixed width its class requires
- **THEN** it is rejected, since a varying width would make the key's length prefix vary and entries would stop ordering by id content

### Requirement: Every state mutation SHALL be routed through a single digest-maintaining wrapper

The contract MUST maintain an `nym-lthash` accumulator over all digest-committed entries. No handler may write to a store directly; every insert, update and delete MUST go through the one wrapper that also folds the digest.

The wrapper MUST apply: insert folds `add(leaf)`; delete folds `remove(leaf)` computed from the value currently in storage; update folds `remove(old_leaf)` then `add(new_leaf)`. A delete or update MUST read the current value first and subtract the exact bytes previously added.

#### Scenario: Replacing an entry subtracts the exact old leaf
- **GIVEN** an entry whose stored value is V1
- **WHEN** it is overwritten with V2
- **THEN** the accumulator has V1's leaf subtracted and V2's leaf added, and re-folding the full entry set from scratch yields the same digest

#### Scenario: Deleting an entry restores the prior digest
- **GIVEN** a digest D over the entry set
- **WHEN** an entry is added and then deleted
- **THEN** the digest returns to D

### Requirement: The canonical leaf encoding SHALL be domain-separated, length-prefixed and key-committing

Each digest-committed record MUST fold exactly one leaf. The leaf MUST begin with a byte tag identifying the entry class, MUST commit the full key (subject class, subject id, source) as well as the value, and MUST length-prefix every variable-width field. Fixed-width integers MUST use a fixed, documented endianness.

The encoding MUST be byte-for-byte identical in the contract and in any verifying client. It is a frozen wire format: changing it is a breaking migration requiring a re-fold of the whole accumulator, during which no intermediate state is verifiable.

Separation is required only within this contract's own accumulator, which is the only place leaves are summed together: the class tag keeps the entry classes apart and the length prefixes keep records apart within a class. Leaves are not required to be distinguishable from another contract's, since each contract has its own accumulator and an ICS23 proof binds to a specific contract address and storage key, so a leaf from elsewhere can never enter this contract's sum.

#### Scenario: Distinct keys with equal values produce distinct leaves
- **GIVEN** the same location value stored for node 1 and for node 2
- **THEN** their leaves differ

#### Scenario: Length prefixing disambiguates adjacent variable fields
- **GIVEN** two subjects in the same class whose ids are `"ab"` and `"a"`, the second with a value shifted by one byte
- **THEN** their leaves differ

#### Scenario: Entry classes cannot collide
- **GIVEN** a measurement entry and a whitelist entry carrying identical payload bytes
- **THEN** their leaves differ because of the leading class tag

### Requirement: The measurement timestamp SHALL be committed to the digest

Each entry MUST record `checked_at`, sourced from block time, and `checked_at` MUST be part of the leaf. Re-submitting an unchanged location MUST therefore change the digest, so a client that verifies the digest also verifies freshness.

#### Scenario: Resubmitting an unchanged location changes the digest
- **GIVEN** an entry for node 42 measured at height H reporting `DE`
- **WHEN** the same agent resubmits `DE` at a later height
- **THEN** `checked_at` advances and the global digest changes

### Requirement: The full accumulator SHALL be readable at a fixed raw storage key

The contract MUST persist the complete `nym-lthash` accumulator, `DIGEST_LEN` bytes (2048 at `ELEMENTS = 1024`), as a single raw value at a fixed, never-changing storage key. That raw value is what a client obtains an ICS23 proof for, since CosmWasm smart queries carry no proof.

The accumulator MUST NOT be stored as a `cw-storage-plus` `Item`: serde cannot derive for `[u8; DIGEST_LEN]`, and base64-encoding it on every write would be wasteful. It is written and read as raw bytes at the fixed key.

The contract MUST additionally expose the 32-byte collapse (`LtHash::out()`, a BLAKE3 over the accumulator state) through a smart query, as an unproven convenience for consumers that only need to compare digests.

The collapse MUST NOT be persisted separately. The accumulator has to be written on every mutation regardless, in order to support incremental updates, so a stored collapse would be an extra write per transaction for a value any client can compute itself. Storing the accumulator at the proven key also lets a verifying client compare accumulators directly, removing the collapse from the set of things that must be computed identically on both sides.

This layout MUST match the directory contract's, whose client-side digest fetch reads `DIGEST_LEN` bytes at the proven key and reconstructs the accumulator from them. A contract storing a 32-byte collapse there instead would be rejected by that machinery on length.

#### Scenario: The digest key is stable and raw-readable
- **WHEN** a client performs a raw store read at the documented digest key with proofs requested
- **THEN** it receives the full `DIGEST_LEN`-byte accumulator together with an ICS23 proof

#### Scenario: The smart query returns the collapse of the proven accumulator
- **WHEN** the digest smart query and the raw read are performed at the same height
- **THEN** the smart query's 32 bytes equal the BLAKE3 collapse of the accumulator returned by the raw read

### Requirement: The contract SHALL expose paginated enumeration of every digest-committed entry

A client MUST be able to pull the complete committed set through a paged query and recompute the accumulator locally. Enumeration MUST cover every entry class that is folded into the digest, including the agent whitelist. Entries MUST sit at deterministic raw storage keys so per-entry ICS23 proofs are also possible without pulling the whole set.

#### Scenario: Recomputing the pulled set matches the on-chain digest
- **GIVEN** a client that has paged through the entire enumeration at a fixed height
- **WHEN** it folds every returned record's canonical leaf
- **THEN** its accumulator equals the one proven at the contract's digest key at that height

#### Scenario: Pagination terminates
- **WHEN** a client pages with the returned cursor until the cursor is absent
- **THEN** every entry is returned exactly once

### Requirement: Writes SHALL be batched with one accumulator load and save per transaction

The contract MUST accept a batch of entries in one execute message. It MUST load the accumulator once and save it once per transaction, while performing a per-entry read-modify-write so each update subtracts the exact leaf it replaces, including when the same key appears more than once in a batch.

Batches MUST be all-or-nothing. The contract MUST enforce a `MAX_BATCH_SIZE` and reject larger batches.

Because the accumulator is commutative, batch ordering MUST NOT affect the resulting digest, and the contract MUST NOT impose an ordering requirement on submitted entries.

A measurement batch MAY carry the same key more than once, resolving to the last write. A self-declaration relay batch MUST instead reject a repeated subject, for the reason given under the self-declaration requirement below.

#### Scenario: Batch ordering does not affect the digest
- **GIVEN** the same set of entries submitted as two batches in different orders against identical starting state
- **THEN** the resulting digests are identical

#### Scenario: A repeated key within one measurement batch folds correctly
- **GIVEN** a measurement batch containing two writes to the same key
- **WHEN** it is applied
- **THEN** the stored value is the later one and the digest matches a from-scratch re-fold

#### Scenario: One invalid entry fails the whole batch
- **GIVEN** a batch of valid entries containing one that fails validation
- **WHEN** it is submitted
- **THEN** the transaction fails and no entry in the batch is written

#### Scenario: Oversized batches are rejected
- **WHEN** a batch larger than `MAX_BATCH_SIZE` is submitted
- **THEN** the contract rejects it

### Requirement: The agent whitelist SHALL be admin-managed, permission-scoped and digest-committed

The contract MUST hold a whitelist mapping an agent address to its permissions, with `can_measure` and `can_relay_self_declared` as independent flags. Only the admin may add, modify or remove whitelist entries.

The whitelist MUST be folded into the digest as its own entry class, because measurement entries carry no signature and a client verifying the digest would otherwise be unable to tell which writers were authorised. The forgery this prevents is a fabricated whitelist entry supplied alongside genuine measurements.

An agent without `can_measure` MUST NOT write measurement entries. An agent without `can_relay_self_declared` MUST NOT write self-declaration entries. A non-whitelisted sender MUST NOT write either.

#### Scenario: Permission flags are enforced independently
- **GIVEN** an agent whitelisted with `can_measure = true` and `can_relay_self_declared = false`
- **WHEN** it submits a self-declaration relay
- **THEN** the write is rejected while its measurement writes continue to succeed

#### Scenario: A non-whitelisted sender is rejected
- **WHEN** an address absent from the whitelist submits a measurement batch
- **THEN** the transaction is rejected

#### Scenario: The whitelist is covered by the digest
- **GIVEN** a client that has pulled the full enumeration and verified the digest
- **THEN** the whitelist it obtained is covered by that verification, and an added or altered whitelist entry would break the recompute

### Requirement: Authorisation SHALL be evaluated at read time, and removing an agent SHALL immediately invalidate its entries for verifiers

Removing an agent from the whitelist MUST NOT delete its entries, but a conforming client MUST treat entries whose writer is not currently whitelisted as unauthorised. This makes compromise recovery immediate and requires no enumeration by agent.

The contract MUST provide an admin-invokable removal of entries named by explicit key, in batches bounded by `MAX_BATCH_SIZE`, folding each removal into the digest. Reclaiming the space a de-whitelisted agent's entries occupy is one use of it, for storage hygiene rather than as a security control.

Removal MUST NOT be scoped to an agent. The agent is the trailing component of an entry's key, so nothing indexes by it and an agent-scoped sweep would have to scan the whole store on every page, with each page a separate admin transaction. Naming keys explicitly instead puts the pagination in the client, which already pages the enumeration to verify the digest, and makes the on-chain cost proportional to what is deleted rather than to what is stored.

Explicit removal MUST also be able to name entries no agent-scoped sweep could reach. A measurement may name a subject that was never bonded, since measurements do not consult the mixnet contract, and the unbond callback never fires for such a subject; without explicit removal that entry would be permanent.

Naming a key that holds no entry MUST NOT fail the batch, because the admin acts on an enumeration read at an earlier height and an entry it names may since have been removed or replaced.

Explicit removal MUST NOT be able to name a whitelist entry, which is removable only through the whitelist operation, so that revocation and its authorisation meaning stay in one place.

#### Scenario: De-whitelisting neutralises an agent without removing anything
- **GIVEN** an agent with existing measurement entries
- **WHEN** the admin removes it from the whitelist
- **THEN** its entries remain in storage and in the digest, and a conforming client rejects them as unauthorised

#### Scenario: Reclaiming a de-whitelisted agent's entries keeps the digest consistent
- **GIVEN** a de-whitelisted agent whose entries the admin has selected from the enumeration
- **WHEN** the admin removes them by explicit key
- **THEN** only those entries are deleted, each removal subtracts its exact leaf, and a from-scratch re-fold matches the resulting digest

#### Scenario: An entry for a subject that was never bonded can be removed
- **GIVEN** a measurement entry naming a subject that has never been bonded
- **WHEN** the admin removes it by explicit key
- **THEN** it is deleted and its leaf subtracted, this being the only path that can reach it

#### Scenario: Removal is admin-only
- **WHEN** any address other than the admin invokes removal, including the agent that wrote the entry
- **THEN** it is rejected and the entry is unchanged

### Requirement: Relayed self-declarations SHALL be node-signed with a strictly monotonic declared_at

A self-declaration entry MUST carry the subject node's ed25519 signature over a domain-separated payload binding the node id, the payload version, the payload bytes and a node-supplied `declared_at`. The contract MUST verify that signature against the node's identity key as resolved from the mixnet contract, and MUST derive the signed bytes from the payload it is about to store rather than from any re-encoding of it.

The payload version MUST be signed alongside the payload bytes, so that a relayer cannot present bytes signed under one version as another. Otherwise the signature would still verify while the relayer, not the node, decided which format consumers interpret those bytes as.

The contract MUST accept the write only when `declared_at` is strictly greater than the `declared_at` already stored for that subject, which makes replay of a superseded artifact impossible, and only when `declared_at` does not exceed block time by more than `MAX_SKEW`, which prevents a far-future timestamp from permanently freezing the slot. There MUST be no lower bound on `declared_at`; monotonicity alone governs the past.

A rejection caused by the skew bound MUST be distinguishable from a rejection caused by an invalid signature or a stale timestamp.

The subject MUST be bonded and not unbonding in the mixnet contract. An absent bond carries no identity key to verify against, and an unbonding node's entries are about to be deleted by the unbond callback, so accepting one would only add a leaf that is immediately removed.

The contract MUST reject a relay batch in which the same subject appears more than once. Monotonicity is evaluated against stored state, so two artifacts for one subject would both pass validation while whichever was written last would stand regardless of its `declared_at`, letting the relayer rather than the node decide which one wins. Resolving the duplicate against a running value instead would make a batch's validity depend on the order it arrives in, which the batching requirement forbids.

The entry MUST record both `declared_at` (when the node signed) and the relay's `checked_at` (when it reached the chain).

#### Scenario: A superseded artifact cannot be replayed
- **GIVEN** a stored self-declaration with `declared_at = T2`
- **WHEN** an agent relays a validly signed artifact with `declared_at = T1` where `T1 < T2`
- **THEN** the write is rejected and the stored entry is unchanged

#### Scenario: An equal timestamp is rejected
- **GIVEN** a stored self-declaration with `declared_at = T`
- **WHEN** an artifact with `declared_at = T` is relayed
- **THEN** the write is rejected, because the comparison is strict

#### Scenario: A far-future timestamp cannot freeze the slot
- **WHEN** an artifact whose `declared_at` exceeds block time by more than `MAX_SKEW` is relayed
- **THEN** the write is rejected with a skew-specific error

#### Scenario: A slow node clock still works
- **GIVEN** a node whose clock is well behind chain time
- **WHEN** it publishes successive artifacts with increasing `declared_at`
- **THEN** each is accepted, because only monotonicity governs the past

#### Scenario: A signature that does not match the node's identity key is rejected
- **WHEN** an agent relays an artifact whose signature does not verify against the subject's identity key in the mixnet contract
- **THEN** the write is rejected

#### Scenario: A declaration for a node that is not bonded is rejected
- **WHEN** an agent relays an artifact for a subject that has no bond in the mixnet contract, or whose bond is unbonding
- **THEN** the write is rejected

#### Scenario: A subject repeated within one relay batch is rejected
- **GIVEN** a relay batch containing two validly signed artifacts for the same subject
- **WHEN** it is submitted
- **THEN** the whole batch is rejected whichever order the two appear in, so the relayer cannot use ordering to install the older artifact

### Requirement: The contract SHALL treat the location payload as opaque versioned bytes

The stored payload MUST be `{ version: u8, content }` where `content` is raw bytes. The contract MUST NOT parse, validate, normalise or re-serialise `content`, and MUST store and return exactly the bytes it was given.

`content` MUST be stored as bytes rather than as a base64 string, so that neither state nor the digest leaf pays the base64 inflation.

Verbatim storage is required, not merely permitted, because a relayed self-declaration's signature is over these bytes. Any re-serialisation could change field ordering or numeric formatting and break verification against the stored value.

The contract MUST enforce a maximum payload size, since it can no longer reject a malformed payload and an unbounded one would inflate both state and every verifier's recompute. That bound MUST be held in contract state and be admin-adjustable, so that a payload version needing more room, or less, does not require a redeploy.

Under `version = 1` the `content` bytes MUST be UTF-8 JSON, so a web consumer can base64-decode and parse without obtaining a schema. The contract MUST remain agnostic to this: the version byte selects the format, and a later version MAY use a different one. The version byte MUST sit outside `content` so that the format itself, and not merely the schema, can change.

No component anywhere on the path from the node to a verifier may parse and re-emit a payload. JSON key ordering, whitespace and floating-point formatting vary between implementations, and the location payload carries floating-point coordinates, so a re-serialised payload can differ byte-for-byte from the signed original and silently fail verification.

#### Scenario: Stored bytes are returned unchanged
- **GIVEN** a payload submitted as an arbitrary byte sequence
- **WHEN** the entry is queried
- **THEN** the returned `content` is byte-for-byte identical to what was submitted

#### Scenario: A relayed self-declaration still verifies against stored bytes
- **GIVEN** a self-declaration whose signature is over its payload bytes
- **WHEN** the entry is read back from the contract and the signature is checked against the stored `content`
- **THEN** it verifies

#### Scenario: An oversized payload is rejected
- **WHEN** a payload exceeding the maximum size is submitted
- **THEN** the write is rejected

#### Scenario: A payload the contract cannot interpret is still accepted
- **GIVEN** a payload whose bytes do not decode as a valid location
- **WHEN** an authorised agent submits it
- **THEN** the contract accepts and commits it, leaving interpretation to consumers

#### Scenario: A web consumer reads a payload without a schema
- **GIVEN** a stored entry written under `version = 1`
- **WHEN** a consumer base64-decodes `content` and parses it as JSON
- **THEN** it obtains the location fields with no schema or code generation step

#### Scenario: Re-serialising a signed payload breaks verification
- **GIVEN** a node-signed payload containing floating-point coordinates
- **WHEN** it is parsed and re-serialised before being stored
- **THEN** the resulting bytes may differ from the signed original and the signature fails to verify, which is why the relay path stores bytes verbatim

### Requirement: Every source SHALL carry the same uniform Location payload type

Every entry, whether measured, self-declared or admin-overridden, MUST carry the same `Location` payload type. Because the contract stores payloads opaquely, this uniformity is a property of the shared types crate and of producers rather than something the contract enforces.

`Location` MUST carry `two_letter_iso_country_code`, optional coordinates, `city`, `region`, `org`, `postal`, `timezone`, and an optional ASN record carrying `asn`, `name`, `domain`, `route` and the provider's type. This mirrors the shape node status API serves on its dVPN surface, with two deliberate deviations below.

Coordinates MUST be optional, because `0.0, 0.0` is a valid location rather than an absent one and a country-only self-declaration or override would otherwise be indistinguishable from a node genuinely at that point. A consumer rendering the node status API shape MUST substitute `0.0` for absent coordinates, preserving that surface's existing behaviour.

The ASN record MUST store the provider's raw type verbatim rather than a derived classification. A consumer needing node status API's two-value `residential | other` form MUST derive it by testing the raw type for `"isp"`. Storing the derived form instead would permanently collapse distinct provider types, and discarded data cannot be recovered without re-measuring every subject against a metered provider.

Absence within the payload MUST otherwise follow the existing convention: the empty string for unknown text fields, and an absent ASN record where none was determined.

The canonical `Location` type MUST be defined once in the shared types crate, with any HTTP schema derive feature-gated, so contract, service and consumers share one definition rather than converting between two.

The payload MUST NOT carry the subject's own IP addresses in any form, including hashes. IPv4's key space makes an unsalted hash trivially reversible, so a hash of an address is an address, and a public contract salt does not change that.

The ASN record's `route` is the sole permitted exception, and is not a subject address: it is the provider's announced prefix, returned identically for every subject behind that block, so it names the network rather than anything within it. It is retained because node status API already serves it on the surface this payload exists to reproduce, and a field discarded here could only be recovered by re-measuring every subject.

#### Scenario: A self-declaration and a measurement decode to the same type
- **GIVEN** a relayed self-declaration and a measurement for the same subject
- **WHEN** both payloads are decoded
- **THEN** they yield the same `Location` type, and only the entry wrapper differs by carrying an attestation for the self-declaration

#### Scenario: An override carries a full Location
- **WHEN** the admin writes an override
- **THEN** it supplies a complete `Location`, using the empty-string convention for fields it does not know and omitting the ASN record

#### Scenario: The dVPN location shape needs no enrichment
- **GIVEN** any stored entry
- **WHEN** a consumer renders the dVPN location object
- **THEN** every field is derivable from that entry alone, with absent coordinates rendered as `0.0` and the two-value ASN classification derived from the stored raw type

#### Scenario: Absent coordinates are distinguishable from Null Island
- **GIVEN** a self-declaration carrying only a country code
- **WHEN** it is stored and read back
- **THEN** its coordinates are absent rather than `0.0, 0.0`, and a consumer can tell it apart from an entry genuinely located at that point

#### Scenario: A non-ISP provider type survives storage
- **GIVEN** a measurement whose provider type is `hosting`
- **WHEN** the entry is read back
- **THEN** the raw type `hosting` is recoverable, and a consumer deriving the two-value form still obtains the non-residential classification

#### Scenario: No address is recoverable from an entry
- **GIVEN** any stored entry
- **THEN** it contains no IP address field and no value from which an IP address can be recovered

### Requirement: Payload evolution SHALL NOT require a contract migration

Because the digest leaf commits the stored payload bytes and a verifier recomputes without parsing them, changing the `Location` encoding MUST be a payload version bump rather than a contract migration. Entries written under an earlier version MUST remain valid, MUST continue to verify against the digest, and MUST be distinguishable by their `version` byte.

#### Scenario: Mixed payload versions verify against one digest
- **GIVEN** a store containing entries written under payload version 1 and version 2
- **WHEN** a client pages the full enumeration and folds every leaf
- **THEN** the recomputed digest matches the on-chain digest, with no payload parsing required

### Requirement: Node subjects SHALL be cleaned up when the node unbonds

The contract MUST accept an unbond callback from the configured mixnet contract and delete every entry whose subject is that `NymNode` id, across every source, folding each removal into the digest. The sender MUST be the configured mixnet contract. Subjects of other classes MUST be unaffected and remain admin-managed.

#### Scenario: Unbonding removes every entry for that node
- **GIVEN** a node with measurement entries from two agents, a self-declaration and an override
- **WHEN** the mixnet contract signals that it unbonded
- **THEN** all four entries are deleted and the digest reflects each removal

#### Scenario: Only the mixnet contract may invoke the callback
- **WHEN** any other address invokes the unbond callback
- **THEN** it is rejected

### Requirement: Only the admin SHALL write override entries

Override entries MUST be writable and removable by the contract admin alone, MUST use the `Override` source, and MUST be digest-committed like any other entry. Setting and removing are separate admin operations, so an override can be retracted without waiting for the subject to be re-measured. The contract MUST NOT itself apply any precedence between an override and other sources; resolution is a client concern.

#### Scenario: A non-admin cannot write an override
- **WHEN** a whitelisted measurement agent submits an override entry
- **THEN** the write is rejected

#### Scenario: The admin removes an override
- **GIVEN** an override entry for node 42, alongside a measurement and a self-declaration for the same node
- **WHEN** the admin removes the override
- **THEN** only the override entry is deleted, its exact leaf is subtracted from the digest, and the other entries for that node remain

#### Scenario: A non-admin cannot remove an override
- **WHEN** any address other than the admin invokes the override removal
- **THEN** it is rejected and the entry is unchanged

#### Scenario: An override does not suppress other entries
- **GIVEN** an override for node 42
- **WHEN** the entries for node 42 are enumerated
- **THEN** the measurement and self-declared entries are still returned alongside it


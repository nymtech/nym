## ADDED Requirements

### Requirement: Opt-in activation gate

The nym-node directory publisher SHALL run only when both an operator-set `enabled` flag is true and a directory contract address is configured for the network. When either condition is unmet the publisher SHALL NOT start and SHALL NOT emit errors, so the subsystem is inert everywhere the directory is not deployed or not opted into.

#### Scenario: Disabled by configuration
- **WHEN** the node starts with the directory publisher `enabled` flag unset (or false)
- **THEN** no publisher task is spawned and no directory queries or writes are attempted

#### Scenario: No contract address configured
- **WHEN** the `enabled` flag is true but the network details provide no directory contract address
- **THEN** the publisher does not start, and the node logs that directory publishing is inactive because no contract address is configured

#### Scenario: Fully configured
- **WHEN** the `enabled` flag is true and a directory contract address is configured
- **THEN** the publisher task is spawned during node startup

### Requirement: Publisher never gates node operation

The directory publisher SHALL be spawned as a fire-and-forget background task and SHALL isolate all its failures. No error, panic-free failure, contract rejection, or chain/query outage in the publisher SHALL prevent the node from starting or continuing to operate.

#### Scenario: Publisher fails at startup
- **WHEN** the publisher cannot complete startup work (e.g. every query to the chain or nym-api fails)
- **THEN** the node continues starting and running normally, and the failure is logged

#### Scenario: Publisher fails at runtime
- **WHEN** a write or query fails while the node is running
- **THEN** the failure is logged and retried per the publisher's own policy, and node operation is unaffected

### Requirement: Startup preflight confirms the node can write

Before attempting any write, the publisher SHALL run a preflight that (a) resolves the node's `node_id` and confirms the node is bonded (and not unbonding) via the mixnet contract, and (b) confirms the relayer account can pay for writes via the node's `ChainInteractionCapabilities::can_send_transactions()` as reported by nym-api. If either check does not pass, the publisher SHALL log a clear, operator-actionable error naming what to fix (bond the node / fund the account / set up a feegrant) and SHALL NOT attempt any write.

#### Scenario: Node is not bonded
- **WHEN** preflight finds no active (bonded, non-unbonding) bond for the node's identity in the mixnet contract
- **THEN** the publisher logs an actionable error, attempts no writes, and enters the dormant back-off state

#### Scenario: Relayer account cannot fund writes
- **WHEN** the node is bonded but `can_send_transactions()` is false (insufficient tokens and no feegrant), or the annotation is unavailable
- **THEN** the publisher logs an actionable error, attempts no writes, and enters the dormant back-off state

#### Scenario: Preflight passes
- **WHEN** the node is bonded and `can_send_transactions()` is true
- **THEN** the publisher proceeds to seed its cache and process update events

### Requirement: Dormant back-off with automatic recovery

When preflight fails, the publisher SHALL enter a dormant state and re-run preflight on a long back-off interval rather than exiting or spamming writes. When a later preflight passes (e.g. after the operator bonds the node or funds the account), the publisher SHALL resume publishing without requiring a node restart. Preflight state changes SHALL be logged, and the publisher SHALL NOT emit a repeated error on every re-check while the state is unchanged.

#### Scenario: Recovery after funding
- **WHEN** the publisher is dormant because of a failed fundability preflight, and the operator later funds the account
- **THEN** a subsequent back-off re-check passes and the publisher resumes by running an immediate reconcile sweep (re-deriving current state rather than draining the channel), without a restart

#### Scenario: No log spam while dormant
- **WHEN** preflight keeps failing across many back-off re-checks with no change in cause
- **THEN** the failure is logged when the state is first entered, not on every re-check

### Requirement: Single serialized writer owning the sequence

All directory writes for the node SHALL flow through a single publisher task that is the sole consumer of the update channel. Because the node is the only legitimate writer for its `node_id`, this single-writer design SHALL be the means by which the contract's gap-free per-node sequence is respected without cross-task races. The publisher SHALL sign the exact `node_signing_payload(node_id, label, sequence, data)` with the node's ed25519 identity key and relay the transaction from the node's own chain account.

#### Scenario: Concurrent updates are serialized
- **WHEN** multiple update events arrive close together (e.g. a startup burst plus a rotation emit)
- **THEN** the publisher processes them one at a time so each write uses the correct next sequence

#### Scenario: Sequence tracked and refreshed
- **WHEN** the publisher starts, and whenever a write is rejected for a sequence mismatch
- **THEN** it (re)reads the node's expected next sequence from the contract before signing, and retries the write with the corrected sequence

### Requirement: Reconcile-before-write

The publisher SHALL seed a cache of the node's current on-chain entries once at startup (via a single query for all of the node's entries) and SHALL, for each update event, compare the payload's derived canonical bytes against the cached on-chain value and issue a write only when the value is absent or differs. A payload whose bytes match what is already published SHALL NOT produce a transaction.

#### Scenario: Unchanged payload is not re-published
- **WHEN** an update event carries a payload whose canonical bytes equal the currently published entry for that label
- **THEN** no transaction is sent

#### Scenario: Changed or absent payload is published
- **WHEN** an update event carries a payload that is not yet published, or whose canonical bytes differ from the published entry
- **THEN** the publisher writes the new value and updates its cache on success

#### Scenario: Survives restart without redundant writes
- **WHEN** a node restarts and a prior instance already published a payload that is still current
- **THEN** the startup reconcile finds the cached value equal and issues no write for it

### Requirement: Periodic reconciliation and deletion of no-longer-desired entries

The publisher SHALL run a periodic reconcile sweep that computes the node's desired snapshot (the current payload every producer would publish), fetches the node's on-chain entries and the label whitelist, and drives the on-chain state toward the desired snapshot. In addition to setting stale or missing payloads, the sweep SHALL delete any published entry whose label the node recognises (`KnownLabel`) but that is not present in the desired snapshot - cleaning up entries orphaned by a label removed from the whitelist or by a payload that is no longer applicable. The sweep SHALL NOT delete entries under labels the node does not recognise. All deletes SHALL go through the single writer (correct signature and sequence).

#### Scenario: Orphan under a removed label is deleted
- **WHEN** a sweep finds a published entry for a `KnownLabel` that is no longer in the node's desired snapshot (e.g. its label was removed from the whitelist)
- **THEN** the publisher deletes that entry via a signed, correctly-sequenced delete

#### Scenario: Unknown-label entry is never deleted
- **WHEN** a sweep finds a published entry under a label that does not parse to a `KnownLabel`
- **THEN** the publisher leaves the entry untouched (it may have been published by a newer binary) and does not delete it

#### Scenario: Desired entry absent on chain is created by the sweep
- **WHEN** a sweep finds a desired payload that is not published on chain
- **THEN** the publisher writes it, independent of any event wakeup

### Requirement: Payloads are a closed set keyed to the contract label whitelist

Publishable data SHALL be modelled as a closed `DirectoryPayload` enum in which each variant maps to exactly one `KnownLabel` from the directory contract's whitelist. Each payload SHALL have a single canonical byte encoding, defined once and shared with the retrieval client's decode side, so the bytes the node writes are exactly the bytes a client decodes. The encoding SHALL be deterministic (identical inputs produce identical bytes) and SHALL be forward-compatible, such that a later payload version that adds a field remains decodable by an older reader that ignores the unknown field.

#### Scenario: Payload resolves to its label
- **WHEN** the publisher processes a `DirectoryPayload`
- **THEN** it derives the entry's label from the payload variant's `KnownLabel` and writes the payload's canonical bytes as the entry `data`

#### Scenario: Encoding is deterministic
- **WHEN** the same payload value is encoded more than once
- **THEN** the canonical bytes are identical each time

#### Scenario: Added field does not break older readers
- **WHEN** a payload is extended with a new field and encoded
- **THEN** a reader built against the older payload definition still decodes the bytes, ignoring the unknown field

### Requirement: Update events are emitted by independent producers

The publisher's correctness backbone is a reconcile sweep (see "Periodic reconciliation and deletion"); on top of it, the publisher SHALL consume `DirectoryUpdate` wakeups from a channel written to by independent producers for low-latency updates between sweeps. The startup snapshot (the first sweep) SHALL cover every derivable payload, including the sphinx key. The sphinx key rotation producer SHALL emit the current sphinx payload whenever the node's sphinx keys change. Adding a future runtime-updatable payload SHALL require only a new producer holding a sender handle, not a change to the publisher's core loop.

#### Scenario: Startup snapshot covers every payload including sphinx
- **WHEN** the publisher starts and preflight passes, and the node performs no key rotation at that moment
- **THEN** the startup snapshot still derives and reconciles the current sphinx payload (it is not withheld until the next rotation)

#### Scenario: Rotation emits on key change
- **WHEN** the node's sphinx keys change (pre-announce, swap, or purge of a rotation key)
- **THEN** the rotation producer emits the current sphinx payload as a `DirectoryUpdate` for a targeted reconcile

#### Scenario: New runtime source needs no core change
- **WHEN** a future runtime-updatable payload is added
- **THEN** it is introduced as a new producer with its own sender handle, without changing the publisher's consumer loop

### Requirement: Label-whitelist reconciliation across version skew

The node's known labels (`KnownLabel::ALL`, fixed at compile time) and the contract's admin-governed label whitelist (`get_allowed_labels()`) can diverge when the node binary and the deployed contract are at different versions. The publisher SHALL reconcile against the contract's current whitelist and SHALL NOT attempt to write a payload whose label is not currently whitelisted (whether never added, or removed by admin governance), logging a warning instead of issuing a doomed transaction. When the contract advertises a label the node does not recognise, the publisher SHALL log a warning indicating the node binary may be behind the deployed contract.

#### Scenario: Node would publish a label the contract has not whitelisted
- **WHEN** the publisher holds a payload for a label that is absent from the contract's current whitelist (removed, or ahead of the deployed contract)
- **THEN** the publisher skips writing that payload and logs a warning naming the unwhitelisted label

#### Scenario: Contract advertises a label unknown to the node
- **WHEN** the contract's whitelist contains a label that does not parse to a `KnownLabel`
- **THEN** the publisher logs a warning that the node binary may be behind the deployed contract, and continues publishing the labels it does know

#### Scenario: Whitelist change is picked up without a restart
- **WHEN** a label the node publishes is removed from (or added to) the contract whitelist while the node is running
- **THEN** a subsequent whitelist refresh causes the publisher to stop (or resume) writing that label accordingly

### Requirement: Sphinx key entry is published and kept current

As the concrete example that exercises the runtime-republish path, the node SHALL reconcile its sphinx key payload under the contract's `sphinx_key` label on startup and SHALL keep it current across key rotations, in both cases writing only when the payload differs from what is already published (reconcile-before-write) - so an unchanged sphinx key across a restart produces no write.

#### Scenario: Sphinx key published on first run
- **WHEN** a bonded, funded node with publishing enabled starts and has no `sphinx_key` entry on chain
- **THEN** the publisher writes the current sphinx payload under the `sphinx_key` label

#### Scenario: Sphinx key updated after rotation
- **WHEN** the node completes a key rotation that changes its sphinx keys
- **THEN** the publisher writes the updated sphinx payload, replacing the previously published one

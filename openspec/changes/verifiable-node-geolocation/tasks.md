## 1. Spikes and open questions

Both done 2026-08-05; findings recorded in design.md. Outcome: no custom `PrimaryKey` is needed (a stock
`(u8, Vec<u8>, Vec<u8>)` tuple suffices), and the digest requirement was corrected to store the full accumulator rather
than its collapse.

- [x] 1.1 Spike the `(subject_class, subject_id, source)` key as a custom `PrimaryKey`/`KeyDeserialize`, using
  `StoredNodeEntries` in `contracts/directory/src/storage.rs` as the template; confirm prefix scans cover "all entries
  for a subject" and "all measurements for a subject", and that `NymNode` ids order numerically
- [x] 1.2 Confirm the checkpoint/anchor machinery in `common/nym-directory-client` is parameterisable by contract
  address and digest key without modification, reading it on `feat/node-directory-publishing` since it is not on develop

## 2. Shared types crate

- [x] 2.1 Create `common/cosmwasm-smart-contracts/geolocation-contract/` with `SubjectClass`, `Method`, `Source`,
  `LocationEntry`, `AgentPermissions`, and the per-class `subject_id` encoding
- [x] 2.2 Define the canonical `Location` type mirroring node status API's dVPN shape (`http/models/mod.rs:204`):
  country, coordinates, city, region, org, postal, timezone, optional ASN record with `asn`/`name`/`domain`/`route`/
  `kind`. Put the whole payload module behind a non-default `payload` feature the contract never enables, since `f64`
  coordinates in the wasm would fail `cosmwasm-check`, and gate the HTTP schema derive on top of that. Used uniformly by
  every source
- [x] 2.3 Add payload tests: absent coordinates round-trip as absent (never `0.0, 0.0`), a `hosting` provider type
  survives verbatim, and the derived `residential | other` form matches node status API for each raw type
- [x] 2.4 Implement the opaque versioned payload wrapper (`version: u8` + `Binary` `content`, version byte outside
  `content`) and the size bound, which is contract state seeded from `DEFAULT_MAX_PAYLOAD_SIZE` rather than a hardcoded
  constant, since a later payload version may need a different one; version 1 encodes `content` as UTF-8 JSON
- [x] 2.5 Store entries as ordinary `cw-storage-plus` JSON values rather than a hand-rolled byte codec, and add a
  round-trip test pinning that `content` bytes survive verbatim through that encoding. Measured on a realistic entry,
  JSON costs 473 bytes against a compact codec's 304 (598 against 380 with an attestation), which at ten thousand
  entries is roughly a megabyte of state; that was judged not worth a bespoke format with its own truncation handling
  and its own migration story
- [x] 2.6 Implement the canonical `digest_leaf()` with class tags per entry class and length prefixing on every variable
  field. No contract-wide domain tag: leaves are only ever summed into this contract's own accumulator, so the class tag
  is the separation that matters
- [x] 2.7 Add leaf tests: distinct keys with equal values differ, length-prefix disambiguation, class tags cannot
  collide, `checked_at` is committed
- [x] 2.8 Define the domain-separated `NymNodeLocation` signing payload shared by node, service and contract. The
  signing payload hangs off `LocationPayload`, so the signed bytes always come from the payload that will be served, and
  `NymNodeLocation` carries that payload rather than a typed `Location` (it is relayed verbatim, so it is never decoded
  on this path). Both are therefore ungated, and no `payload`-gated code is needed at all. The signed bytes are
  `domain_tag || node_id BE || declared_at LE || version || content`, with `version` bound so a relayer cannot restate
  v1-signed content as v2
- [x] 2.9 **Dropped as premature.** Conformance vectors guard drift between two implementations of the leaf encoding,
  and there is only one: the verifying client imports `digest_leaf()` from this crate. A second implementation, most
  likely a browser verifier, is far enough off that fixtures would be speculative. Revisit when one is actually being
  written; until then design decision 9's drift guard is "there is only one implementation" rather than shared vectors
- [x] 2.10 Define `InstantiateMsg`, `ExecuteMsg`, `QueryMsg` and response types

## 3. Contract storage and digest maintenance

- [x] 3.1 Create `contracts/geolocation/` scaffolding, `Cargo.toml`, `Makefile`, schema binary
- [x] 3.2 Implement the entries store as a stock `Map<(u8, Vec<u8>, Vec<u8>), LocationEntry>` over the key from 1.1,
  with `Source` flattened to bytes by a local helper; `cargo check` that `KeyDeserialize` is implemented for that tuple
  before building on it. With JSON values (2.5) a plain `Map` suffices, so the directory's manual `Path`/`Prefix`
  handling is not needed here. Task 1.1's compile-gated assumption is confirmed: cw-storage-plus 2.0 does implement
  `KeyDeserialize` for the tuple, so the paged enumeration in 5.2 can decode a full key
- [x] 3.3 Implement the whitelist store as a second digest-committed entry class
- [x] 3.4 Implement accumulator load/save as raw `DIGEST_LEN` bytes at the fixed digest key (not a `cw-storage-plus`
  `Item`); expose the 32-byte collapse via smart query only, never persisted
- [x] 3.5 Implement the single digest-maintaining wrapper (insert adds, delete subtracts the stored leaf, update
  subtracts then adds); no handler touches a store directly. Enforced structurally rather than by convention: the two
  digest-committed `Map`s are private to `storage.rs`, so a handler cannot reach them even by mistake. `set_entries`
  folds a whole batch under one accumulator load/save, which is what 4.1 needs
- [x] 3.6 Add a test asserting a from-scratch re-fold matches the incrementally maintained digest across insert, update,
  delete and repeated-key sequences. Also covers batch-order independence (so nobody later "fixes" it by imposing a
  sort), delete-everything returning the accumulator to the identity, no-op removals, and that the key decodes back to
  the typed `(Subject, Source)` it was written under. Mutation-tested: dropping the replacement subtract, the delete
  subtract, the whitelist from the enumeration, or the whitelist add each fail it
- [x] 3.7 Implement `instantiate`: mixnet contract address, admin, initial whitelist, `MAX_SKEW`, `MAX_BATCH_SIZE`, max
  payload size (defaulting to `DEFAULT_MAX_PAYLOAD_SIZE`). Initial agents go in through the digest wrapper, so the
  whitelist is committed from block one rather than only from the first admin transaction. Added
  `ContractConfig::validate`, rejecting a zero `max_batch_size` or `max_payload_size`: both leave the contract
  instantiating and querying normally while rejecting every agent submission, and 4.10's `UpdateConfig` needs the same
  check

## 4. Contract transactions

- [x] 4.1 Implement batched measurement submission with one accumulator load/save per transaction and per-entry
  read-modify-write. Every check runs before any write, so all-or-nothing is structural rather than resting on the
  transaction rolling back. Carries the measurement half of 4.2 and 4.3 with it, since a handler that writes
  unauthorised or unbounded entries should not exist even briefly
- [x] 4.2 Enforce `MAX_BATCH_SIZE`, the configured max payload size and all-or-nothing batch semantics; store payload
  bytes verbatim without parsing. Both batch paths validate everything before writing anything, so all-or-nothing is
  structural rather than resting on the transaction rolling back
- [x] 4.3 Enforce whitelist membership and the `can_measure` permission on measurement writes. Membership is read on
  every write rather than trusted from when an entry was accepted, so de-whitelisting takes effect with nothing to
  invalidate and nothing to enumerate
- [x] 4.4 Implement self-declaration relay: verify the ed25519 signature against the identity key resolved from the
  mixnet contract, enforce the `can_relay_self_declared` permission. The relaying agent appears nowhere in the key, so a
  subject keeps one self-declared slot however many agents relay for it. A relay batch rejects a repeated subject, where
  a measurement batch allows one: monotonicity is checked against stored state, so two declarations for one node would
  both pass and whichever was written last would win regardless of `declared_at`. Resolving the duplicate instead would
  make a batch's validity depend on its ordering, which the spec forbids. Spec amended: the repeated-key scenario is now
  measurement-only, and the relay path gains duplicate-rejection and must-be-bonded requirements with scenarios
- [x] 4.5 Enforce strict `declared_at` monotonicity and the `MAX_SKEW` future bound, with distinguishable errors for
  stale, skewed and bad-signature rejections. Checks are ordered cheapest first, so a replayed or skewed artifact is
  rejected without paying for the cross-contract bond lookup or the ed25519 verify. Strictly greater rather than
  greater-or-equal, so re-relaying an unchanged artifact is a replay rather than a heartbeat: unlike a measurement, a
  self-declaration can only be refreshed by the node signing a new one
- [x] 4.6 Implement admin override set/remove under the `Override` source. Set and remove are separate operations, so
  an override can be retracted without waiting for a re-measurement, and removal touches only the `Override` slot. The
  subject is deliberately not checked against the mixnet contract: the override is the admin's escape hatch, and a
  bonding check would only apply to one of the subject classes the enum is meant to grow. Removing an absent override
  is a no-op rather than an error
- [x] 4.7 Implement admin whitelist add/modify/remove, folding each into the digest. Add and modify are one operation,
  since they differ only in whether a leaf has to be retired first. Removal is non-destructive by design, leaving the
  agent's entries in storage and in the digest for 4.8 to reclaim. The grant's flags go into the event attributes:
  current state is queryable, but who was granted what and when is only in the log
- [x] 4.8 Implement the paginated purge of a de-whitelisted agent's entries. **Reshaped to `RemoveEntries { keys }`**,
  an admin-only batch of explicit `EntryKey`s, after working through how a scoped purge would actually be driven: the
  agent is the trailing key component, so nothing indexes by it and a scoped purge would scan the whole store per page,
  one admin transaction at a time. Naming keys puts the pagination in the client that already pages the enumeration, and
  makes the on-chain cost proportional to what is deleted. It also reaches entries no scoped sweep could, in particular
  a measurement naming a subject that was never bonded, which nothing else can ever delete. Spec requirement and
  scenarios rewritten to match
- [x] 4.9 Implement the mixnet unbond callback deleting every entry for that `NymNode` subject across all sources, with
  sender verification. Every source goes, the admin's override included: the subject has ceased to exist, so nothing
  anyone asserted about where it was remains meaningful. The whitelist is untouched, being a different entry class. The
  sender check is what stops any address clearing a live node's entries as a denial of service. **Also covers 6.4**:
  the App-level test unbonds through the mixnet contract, which is the only shape that proves the dispatch reaches this
  handler, since deps-level tests do not dispatch a `Response`'s sub-messages
- [x] 4.10 Implement `UpdateAdmin` and config updates. `UpdateConfig` applies each provided field and then validates the
  result as a whole, reusing 3.7's `ContractConfig::validate`, so a partial update cannot arrive field by field at a
  configuration instantiation would have refused. Lowering a bound is not retroactive: entries stored under a larger
  `max_payload_size` stay readable and stay in the digest, and shrinking the stored set is `RemoveEntries`' job. The
  resulting tunables go into the event attributes, since current state is queryable but a change is not

## 5. Contract queries

- [ ] 5.1 Single entry, all entries for a subject, and all measurements for a subject
- [ ] 5.2 Paginated enumeration of every digest-committed entry across both classes, with a cursor
- [ ] 5.3 Digest smart query, plus documentation of the fixed raw key for ICS23 proofs
- [ ] 5.4 Whitelist query
- [ ] 5.5 Generate contract schema

## 6. Contract testing

- [ ] 6.1 Unit tests per handler covering the authorisation matrix (non-whitelisted, wrong permission, non-admin
  override, wrong unbond sender)
- [ ] 6.2 Batch tests: ordering independence, repeated key within a batch, oversized rejection, one-bad-entry rollback
- [ ] 6.3 Replay tests: superseded artifact, equal timestamp, far-future timestamp, slow node clock
- [x] 6.4 App-level test of the mixnet unbond callback dispatching to this contract (deps-level tests do not dispatch
  sub-messages). Done in 4.9 as `unbonding_through_the_mixnet_contract_reaches_this_handler`, alongside the deps-level
  handler tests it complements
- [ ] 6.5 End-to-end recompute test: page the full enumeration, fold every leaf, assert equality with the queried
  digest, including a store holding two payload versions
- [ ] 6.6 Measure gas for a full batch and set `MAX_BATCH_SIZE` from the result; record the number in design.md. Measure
  with realistic JSON payloads, not minimal ones: version 1 encodes `content` as JSON rather than protobuf, so entries
  run roughly two to three times larger than a prost equivalent and the batch is bounded by total transaction bytes as
  much as by per-entry gas

## 7. Client integration

- [ ] 7.1 Add query and signing traits for the contract in `nym-validator-client`
- [ ] 7.2 **Gated on the directory merge.** `common/nym-directory-client` is not on develop, so end-to-end client
  verification (anchor, ICS23-prove the digest key, recompute against the pulled set) cannot be built until
  `feat/node-directory-publishing` lands. Per task 1.2, `proof.rs`, `contract_storage_key` and `anchor/checkpoint/*`
  reuse unchanged, but the digest-fetch helper hardcodes the directory's `DIGEST_STATE` and needs the storage key
  threaded through as a parameter, and the top-level client is directory-shaped so this is a sibling rather than reuse.
  The contract and service do not depend on it and ship without it

## 8. Node-side signed location artifact

- [ ] 8.1 Add the `NymNodeLocation` type and its signing to nym-node
- [ ] 8.2 Serve the signed artifact over the node's HTTP API
- [ ] 8.3 Test that the served artifact verifies against the node's identity key using the shared payload from 2.8, and
  that its signed bytes survive a node -> service -> contract -> reader round trip byte-for-byte
- [ ] 8.4 Add a regression test that a parsed-and-reserialised payload with `f64` coordinates fails verification,
  pinning why the relay path must stay verbatim (see the `float_roundtrip` pin at `Cargo.toml:359`)

## 9. Geolocator service

- [ ] 9.1 Create the service crate with config, CLI and logging
- [ ] 9.2 Implement subject discovery from the mixnet contract (bonded nym-nodes are the only subject class the contract
  defines)
- [ ] 9.3 Implement address discovery from node HTTP endpoints, behind a trait so the directory-contract source can
  replace it later
- [ ] 9.4 Implement the geolocation lookup client, with the provider allowance exposed as a metric and a per-cycle
  lookup ceiling
- [ ] 9.5 Ensure resolved addresses are never persisted, logged durably or exposed
- [ ] 9.6 Implement the regular sweep, submitting unchanged results so `checked_at` advances
- [ ] 9.7 Implement the local address baseline and the change-triggered measurement, with cold start recording a
  baseline rather than re-measuring everything
- [ ] 9.8 Implement batching and submission, with self-declaration relays in separate batches
- [ ] 9.9 Implement self-declaration fetch and verbatim relay, forwarding the received bytes without parsing and
  re-emitting them, and tolerating stale rejections
- [ ] 9.10 Ensure a failed lookup submits nothing and leaves the previous entry untouched

## 10. Re-test endpoint

- [ ] 10.1 Implement the HTTP endpoint with bearer-token authentication (unlimited)
- [ ] 10.2 Implement node-identity-signed authentication, restricted to the signing node as subject
- [ ] 10.3 Implement replay protection: signed timestamp, validity window, seen-set
- [ ] 10.4 Implement the burst limit: per-node counter of unchanged node-requested measurements, configurable threshold
  and cooldown, reset on a changed result, read against the contract's current value
- [ ] 10.5 Ensure sweep and bearer-token measurements never increment the burst counter

## 11. Verification and documentation

- [ ] 11.1 `cargo build` and `cargo test` across the workspace and the contracts workspace
- [ ] 11.2 Build the contract wasm and check its size
- [ ] 11.3 Document the trust model and the client verification flow, mirroring `docs/directory/README.md`
- [ ] 11.4 Document the deferred node status API migration, including the payload-width constraint and the cold-start
  cliff at `http/state.rs:431`
- [ ] 11.5 Run `openspec validate verifiable-node-geolocation --strict`

## 1. Spikes and open questions

Task 1.1 is ordered first because it is the only remaining item that could still move the frozen key layout. All design questions are resolved in design.md.

- [ ] 1.1 Spike the `(subject_class, subject_id, source)` key as a custom `PrimaryKey`/`KeyDeserialize`, using `StoredNodeEntries` in `contracts/directory/src/storage.rs` as the template; confirm prefix scans cover "all entries for a subject" and "all measurements for a subject", and that `NymNode` ids order numerically
- [ ] 1.2 Confirm the checkpoint/anchor machinery in `common/nym-directory-client` is parameterisable by contract address and digest key without modification, reading it on `feat/node-directory-publishing` since it is not on develop

## 2. Shared types crate

- [ ] 2.1 Create `common/cosmwasm-smart-contracts/geolocation-contract/` with `SubjectClass`, `Method`, `Source`, `LocationEntry`, `AgentPermissions`, and the per-class `subject_id` encoding
- [ ] 2.2 Define the canonical `Location` type mirroring node status API's dVPN shape (`http/models/mod.rs:204`): country, coordinates, city, region, org, postal, timezone, optional ASN record with `asn`/`name`/`domain`/`route`/`kind`; feature-gate the HTTP schema derive. Used uniformly by every source
- [ ] 2.3 Add payload tests: absent coordinates round-trip as absent (never `0.0, 0.0`), a `hosting` provider type survives verbatim, and the derived `residential | other` form matches node status API for each raw type
- [ ] 2.4 Implement the opaque versioned payload wrapper (`version: u8` + `Binary` `content`, version byte outside `content`) and the `MAX_PAYLOAD_SIZE` constant; version 1 encodes `content` as UTF-8 JSON
- [ ] 2.5 Implement the compact value codec (`to_bytes` / `try_from_bytes`) with round-trip and truncation-rejection tests, storing `content` verbatim
- [ ] 2.6 Implement the canonical `digest_leaf()` with a contract-unique domain tag, class tags per entry class, and length prefixing on every variable field
- [ ] 2.7 Add leaf tests: distinct keys with equal values differ, length-prefix disambiguation, class tags cannot collide, `checked_at` is committed
- [ ] 2.8 Define the domain-separated `NymNodeLocation` signing payload shared by node, service and contract, carrying a full `Location`
- [ ] 2.9 Publish leaf-encoding conformance vectors as fixtures, so contract and verifier cannot drift (design decision 9)
- [ ] 2.10 Define `InstantiateMsg`, `ExecuteMsg`, `QueryMsg` and response types

## 3. Contract storage and digest maintenance

- [ ] 3.1 Create `contracts/geolocation/` scaffolding, `Cargo.toml`, `Makefile`, schema binary
- [ ] 3.2 Implement the entries store over the custom key from 1.1
- [ ] 3.3 Implement the whitelist store as a second digest-committed entry class
- [ ] 3.4 Implement accumulator load/save at the fixed raw digest key, plus the collapse
- [ ] 3.5 Implement the single digest-maintaining wrapper (insert adds, delete subtracts the stored leaf, update subtracts then adds); no handler touches a store directly
- [ ] 3.6 Add a test asserting a from-scratch re-fold matches the incrementally maintained digest across insert, update, delete and repeated-key sequences
- [ ] 3.7 Implement `instantiate`: mixnet contract address, admin, initial whitelist, `MAX_SKEW`, `MAX_BATCH_SIZE`

## 4. Contract transactions

- [ ] 4.1 Implement batched measurement submission with one accumulator load/save per transaction and per-entry read-modify-write
- [ ] 4.2 Enforce `MAX_BATCH_SIZE`, `MAX_PAYLOAD_SIZE` and all-or-nothing batch semantics; store payload bytes verbatim without parsing
- [ ] 4.3 Enforce whitelist membership and the `can_measure` permission on measurement writes
- [ ] 4.4 Implement self-declaration relay: verify the ed25519 signature against the identity key resolved from the mixnet contract, enforce the `can_relay_self_declared` permission
- [ ] 4.5 Enforce strict `declared_at` monotonicity and the `MAX_SKEW` future bound, with distinguishable errors for stale, skewed and bad-signature rejections
- [ ] 4.6 Implement admin override set/remove under the `Override` source
- [ ] 4.7 Implement admin whitelist add/modify/remove, folding each into the digest
- [ ] 4.8 Implement the paginated purge of a de-whitelisted agent's entries
- [ ] 4.9 Implement the mixnet unbond callback deleting every entry for that `NymNode` subject across all sources, with sender verification
- [ ] 4.10 Implement `UpdateAdmin` and config updates

## 5. Contract queries

- [ ] 5.1 Single entry, all entries for a subject, and all measurements for a subject
- [ ] 5.2 Paginated enumeration of every digest-committed entry across both classes, with a cursor
- [ ] 5.3 Digest smart query, plus documentation of the fixed raw key for ICS23 proofs
- [ ] 5.4 Whitelist query
- [ ] 5.5 Generate contract schema

## 6. Contract testing

- [ ] 6.1 Unit tests per handler covering the authorisation matrix (non-whitelisted, wrong permission, non-admin override, wrong unbond sender)
- [ ] 6.2 Batch tests: ordering independence, repeated key within a batch, oversized rejection, one-bad-entry rollback
- [ ] 6.3 Replay tests: superseded artifact, equal timestamp, far-future timestamp, slow node clock
- [ ] 6.4 App-level test of the mixnet unbond callback dispatching to this contract (deps-level tests do not dispatch sub-messages)
- [ ] 6.5 End-to-end recompute test: page the full enumeration, fold every leaf, assert equality with the queried digest, including a store holding two payload versions
- [ ] 6.6 Measure gas for a full batch and set `MAX_BATCH_SIZE` from the result; record the number in design.md. Measure with realistic JSON payloads, not minimal ones: version 1 encodes `content` as JSON rather than protobuf, so entries run roughly two to three times larger than a prost equivalent and the batch is bounded by total transaction bytes as much as by per-entry gas

## 7. Client integration

- [ ] 7.1 Add query and signing traits for the contract in `nym-validator-client`
- [ ] 7.2 **Gated on the directory merge.** `common/nym-directory-client` is not on develop, so end-to-end client verification (anchor, ICS23-prove the digest key, recompute against the pulled set) cannot be built until `feat/node-directory-publishing` lands. Confirm the machinery is reusable unchanged (task 1.2), then do this after that merge. The contract and service do not depend on it and ship without it

## 8. Node-side signed location artifact

- [ ] 8.1 Add the `NymNodeLocation` type and its signing to nym-node
- [ ] 8.2 Serve the signed artifact over the node's HTTP API
- [ ] 8.3 Test that the served artifact verifies against the node's identity key using the shared payload from 2.8, and that its signed bytes survive a node -> service -> contract -> reader round trip byte-for-byte
- [ ] 8.4 Add a regression test that a parsed-and-reserialised payload with `f64` coordinates fails verification, pinning why the relay path must stay verbatim (see the `float_roundtrip` pin at `Cargo.toml:359`)

## 9. Geolocator service

- [ ] 9.1 Create the service crate with config, CLI and logging
- [ ] 9.2 Implement subject discovery from the mixnet contract, plus configured non-node subjects
- [ ] 9.3 Implement address discovery from node HTTP endpoints, behind a trait so the directory-contract source can replace it later
- [ ] 9.4 Implement the geolocation lookup client, with the provider allowance exposed as a metric and a per-cycle lookup ceiling
- [ ] 9.5 Ensure resolved addresses are never persisted, logged durably or exposed
- [ ] 9.6 Implement the regular sweep, submitting unchanged results so `checked_at` advances
- [ ] 9.7 Implement the local address baseline and the change-triggered measurement, with cold start recording a baseline rather than re-measuring everything
- [ ] 9.8 Implement batching and submission, with self-declaration relays in separate batches
- [ ] 9.9 Implement self-declaration fetch and verbatim relay, forwarding the received bytes without parsing and re-emitting them, and tolerating stale rejections
- [ ] 9.10 Ensure a failed lookup submits nothing and leaves the previous entry untouched

## 10. Re-test endpoint

- [ ] 10.1 Implement the HTTP endpoint with bearer-token authentication (unlimited)
- [ ] 10.2 Implement node-identity-signed authentication, restricted to the signing node as subject
- [ ] 10.3 Implement replay protection: signed timestamp, validity window, seen-set
- [ ] 10.4 Implement the burst limit: per-node counter of unchanged node-requested measurements, configurable threshold and cooldown, reset on a changed result, read against the contract's current value
- [ ] 10.5 Ensure sweep and bearer-token measurements never increment the burst counter

## 11. Verification and documentation

- [ ] 11.1 `cargo build` and `cargo test` across the workspace and the contracts workspace
- [ ] 11.2 Build the contract wasm and check its size
- [ ] 11.3 Document the trust model and the client verification flow, mirroring `docs/directory/README.md`
- [ ] 11.4 Document the deferred node status API migration, including the payload-width constraint and the cold-start cliff at `http/state.rs:431`
- [ ] 11.5 Run `openspec validate verifiable-node-geolocation --strict`

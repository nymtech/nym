## 1. Provisional defaults and the one compatibility check

- [ ] 1.1 Adopt the provisional liveness profile defaults (100 packets per target, a 500 packets/second aggregate budget, a wave size of 20) and make every one of them CLI- and env-overridable, so an agent host that cannot sustain the budget is a configuration change rather than a code change
- [ ] 1.2 Size the liveness lease budget, straggler wait and per-target timeouts from those defaults with slack, and make them configurable on the same terms, with no behaviour depending on a specific value
- [ ] 1.3 Confirm on a devnet that an un-upgraded node still ingests an `AuthoriseNetworkMonitor` carrying an unknown field, rather than logging a parse failure and skipping it. The `cw_serde` reading says it will; this assumption is load-bearing for every future contract change and gates the orchestrator deploy, not the migration, which emits no message at all

## 2. Contract and announcement: the agent ed25519 identity

- [x] 2.1 Add an optional `bs58_ed25519_identity` to `ExecuteMsg::AuthoriseNetworkMonitor` and to the stored `AuthorisedNetworkMonitor` in `common/cosmwasm-smart-contracts/network-monitors-contract`, as an added field on the existing variant (never a new variant)
- [x] 2.2 Validate the identity on shape in `try_authorise_network_monitor` (base58 decoding to exactly 32 bytes) with its own error variant, and accept its absence without complaint
- [x] 2.3 Bump the contract version, leave `migrate` as build-information only, and leave `queued_migrations` empty: the field is optional and the agent save is an upsert, so there is no data migration
- [x] 2.4 Regenerate the contract JSON schema
- [x] 2.5 Extend the validator-client signing helper so the authorisation message carries the identity
- [x] 2.6 Contract tests: an omitted identity is accepted, a malformed one is rejected, a re-authorisation records a changed identity, and an entry serialised without the field deserialises with `None`
- [x] 2.7 Add a regression test asserting that a serialised new-form `AuthoriseNetworkMonitor` still deserialises into a struct shaped like the old one, so the fleet-compatibility assumption behind this change is checked in CI rather than assumed
- [x] 2.8 Agent side: derive the ed25519 identity from the x25519 noise private key via a labelled HKDF whose output is the ed25519 seed, and include its base58 public key in the announce request
- [x] 2.9 Orchestrator side: carry the identity on the announce request and in both `AuthoriseNetworkMonitor` messages of the existing single transaction, reject a malformed identity before touching state, hold it in the `KnownAgents` entry, and reset the announced flag when it diverges from the cached one
- [x] 2.10 Drop a rehydrated on-chain pair that does not carry one identity across both of its entries, rather than admitting a cache entry no consumer could use. Superseded the original "tolerate an identity-less entry": the rehydration already drops an unpairable entry and lets the next announcement rebuild it, and the contract's save is an upsert keyed by socket address, so an entry predating the field is overwritten in place with no revocation step. Keeping the cached identity non-optional also stops the orchestrator handing work to an agent whose on-chain entry cannot grant a gateway session
- [x] 2.11 Count identity divergence on the existing agent-details-changed counter, and distinguish it from a noise-key or address change in the log rather than in a second series. Superseded the original "add a counter for identity divergence": the identity is derived from the noise key, so it cannot diverge without the noise key diverging, and the rehydration drop rule from 2.10 removed the one case (an identity-less entry restored after a restart) that would have given a dedicated series real traffic
- [x] 2.12 Unit-test that a changed identity alone resets the announced flag so both addresses are re-authorised, that a rehydrated on-chain pair not carrying one shared identity is dropped (absent on either entry, or disagreeing across the two), and that the agent's derivation matches a pinned known-answer vector, so a change to the HKDF label or algorithm fails in CI rather than silently invalidating every identity already announced on chain. The malformed-identity half of the original wording has no seam left to test: the announce request carries a typed `ed25519::PublicKey`, so a malformed value is rejected during deserialisation and is unrepresentable in the handler - testing it would only exercise `serde`

## 3. Shared request/response types

- [ ] 3.1 Add a `TestKind` enum (`stress`, `liveness`) to `nym-network-monitor-orchestrator-requests`
- [ ] 3.2 Replace `TestRunAssignment` with a kind-tagged assignment carrying per-kind payloads: stress and mixnode-liveness (node address, node ips, noise key, sphinx key, key rotation id, probe profile) and gateway-liveness (additionally the client websocket port and the gateway identity key)
- [ ] 3.3 Make the assignment response carry a wave (a list of targets) for liveness and a single target for stress, with an empty response still meaning "no work"
- [ ] 3.4 Add a per-signal result shape (`signal` discriminator plus the existing counts, handshake durations and latency distributions) and make the submission request carry the run-level fields plus one or more signals
- [ ] 3.5 Add the liveness submission route constant and the liveness batch content type mirroring the stress `SignedMessage` envelope
- [ ] 3.6 Unit-test that a gateway-liveness run serialises and round-trips both of its signals, and that a single-signal run round-trips unchanged

## 4. Orchestrator storage and migration

- [ ] 4.1 Write migration `03`: create the per-kind work-state table keyed `(node_id, test_kind)` holding `last_tested_at`, `last_testrun_id`, `last_tested_ip`; create the per-signal child table of `testrun`; create the per-kind submission-watermark table; add `test_kind` to `testrun`; add `expires_at` and `test_kind` to `testrun_in_progress`; add the gateway client websocket port and the wss-announced flag to `nym_node`
- [ ] 4.2 In the same migration, backfill: move `nym_node.last_testrun` / `last_tested_ip` into the work-state table under the `stress` kind, derive `expires_at` for any live in-progress row from `started_at` plus the stress budget, carry `metadata.last_submitted_testrun_id` across as the stress stream's watermark, and project existing `testrun` rows as single-signal runs
- [ ] 4.3 Drop `nym_node.last_testrun` and `nym_node.last_tested_ip` once the backfill is in place
- [ ] 4.4 `touch build.rs` and confirm the compiled-in migrations DB picks up the new columns (a "table X has no column named Y" error means the build script did not re-run)
- [ ] 4.5 Update `storage/models.rs`: per-kind work-state row, per-signal row, `TestKind` mapping, and move the address rotation helpers (`announced_ips`, `next_ip_to_test`) onto the per-kind state so a kind rotates independently
- [ ] 4.6 Update the storage manager's insert path to write a run-level row plus its signal rows in one transaction, and the read paths to reassemble them
- [ ] 4.7 Update result eviction to delete signal rows with their run, and in-progress eviction to compare `expires_at` rather than a global cutoff
- [ ] 4.8 Unit-test the per-kind rotation (two kinds advancing independently over one node's address set) and that deleting a node's last testrun leaves its per-kind `last_tested_at` intact

## 5. Orchestrator scheduling

- [ ] 5.1 Rewrite `assign_next_mixnode_testrun` as a kind-aware assignment: choose the kind, filter by that kind's eligible node types and required non-null fields, apply that kind's staleness age, exclude any node with an in-progress row of any kind, apply `liveness_after_stress_cooldown` for liveness, take one target for stress or up to `liveness_wave_size` for liveness, advance each node's per-kind rotation pointer, and insert one in-progress row per target with its lease
- [ ] 5.2 Add the kind-selection policy (which kind an agent is handed when several are due) and the liveness enable flag that switches liveness assignment off without a redeploy
- [ ] 5.3 Extend the node refresher to record the entry-gateway client websocket port and whether a wss entry is announced
- [ ] 5.4 Add the liveness config knobs (staleness interval, lease budget, wave size, cooldown, enable flag) with the provisional defaults from 1.1 and 1.2, all CLI- and env-overridable
- [ ] 5.5 Add prometheus series for liveness assignments, wave sizes, per-kind in-progress counts, lease expiries, and cooldown skips
- [ ] 5.6 Unit-test that a node with an open stress in-progress row is not assigned liveness and vice versa, that a recently stress-tested node is skipped by the cooldown, and that a wave never exceeds `liveness_wave_size`

## 6. nym-node: final-hop delivery for monitors

- [x] 6.1 Replace the unconditional drop of network-monitor final-hop packets in `handle_final_hop` with delivery to a live client session
- [x] 6.2 Suppress the on-disk fallback for network-monitor final-hop packets: when no session is live, drop and count the packet rather than storing it
- [x] 6.3 Add metrics distinguishing a monitor final-hop packet delivered in-session from one dropped for want of a session
- [x] 6.4 Unit-test the fallback decision at the `SharedFinalHopData` level, over an in-memory gateway store with an empty active-clients store: a monitor packet with no live session is dropped and the store is left untouched, and an ordinary packet with no live session is still stored. The delivered branch is deliberately NOT covered, because registering a client in `ActiveClientsStore` from nym-node would need `insert_remote` and the `message_receiver` channel types made public in the gateway crate, which is disproportionate for a one-line early return whose behaviour belongs entirely to `try_push_message_to_client`

## 7. nym-node: ephemeral unmetered monitor client session

- [x] 7.1 Derive a third structure from the authorised-agent set: the announced monitor ed25519 identities, keyed by identity rather than by address, populated from the same startup load and the same nyxd websocket events, tolerating entries with no identity and the same identity arriving from both of an agent's entries
- [x] 7.2 Add an `ephemeral` mode to `BandwidthStorageManager` that seeds a synthetic allowance and performs no read or write against `BandwidthGatewayStorage`
- [x] 7.3 Make the client identity threaded into `ClientDetails` / `BandwidthStorageManager` optional (or a `Persisted` / `Ephemeral` discriminator) so a session with no storage row is representable
- [x] 7.4 Route a client websocket session into an ephemeral monitor session when the registration handshake's verified ed25519 identity is in the set from 7.1, skipping `insert_shared_keys`, `create_bandwidth_entry`, and the stored-message push. The source IP MUST play no part in this decision
- [x] 7.5 Ensure an out-of-bandwidth outcome is reported to the client as a distinguishable error rather than a generic failure, so a proxied gateway or a gateway that has not ingested the identity is diagnosable
- [x] 7.6 Unit-test that a monitor session forwards packets without a credential and leaves the gateway storage untouched, that a session presenting an unannounced identity from an authorised agent IP is metered exactly as before, and that a session presenting an announced identity from an unrelated IP is exempt. Covered at the `finalise_registration` level over an in-memory gateway storage, asserting on the shared-key row each branch does or does not write. The registration HANDSHAKE is deliberately NOT driven: the gate sits strictly after it and this change does not touch it, so driving it would only add flake-prone plumbing over unmodified code. "Forwards packets without a credential" is covered as far as the bandwidth manager the session receives (synthetic allowance, no bandwidth row read); the remaining wiring through `AuthenticatedHandler` is left to the end-to-end exercise in 12.5

## 8. Agent: liveness profile and wave concurrency

- [ ] 8.1 Add the liveness probe profile alongside the stress profile in the tester config, with the aggregate rate budget deriving the per-target rate
- [ ] 8.2 Bind ONE shared ingress listener per invocation and build a `NoiseNetworkView` containing every target's noise key under every address that target is known by
- [ ] 8.3 Make the known-source set the union of every target's announced addresses, canonicalised
- [ ] 8.4 Attribute returned packets to a target by the source address of the connection they arrive on, and accumulate per-target results
- [ ] 8.5 Execute a wave as one concurrent batch with a hard per-target deadline, so the wave's duration is bounded by the slowest single target
- [ ] 8.6 Submit each target's result as soon as that target finishes rather than at the end of the wave
- [ ] 8.7 Unit-test attribution across a wave (several targets returning interleaved packets), and that one target timing out does not extend the others

## 9. Agent: gateway client session and the two-phase probe

- [ ] 9.1 Use the derived ed25519 client identity from 2.8 for the session, with no new on-disk key and no per-test regeneration (the identity must match what was announced)
- [ ] 9.2 Establish the client session at `ws://<assigned-ip>:<clients_ws_port>`, constructed directly from the assignment, ignoring announced hostnames and wss entries and not reusing `ws_entry_address`
- [ ] 9.3 Implement the ingress phase: forward a sphinx packet through the session whose next hop is the agent's own mixnet address, and count arrivals at the shared listener
- [ ] 9.4 Implement the egress phase: send final-hop packets to the gateway's mixnet listener addressed to the agent's own client session, and count arrivals on that session
- [ ] 9.5 Hold the session open across both phases and their drain windows, and use one address family for both legs with the sphinx return hop matching it
- [ ] 9.6 Produce two signals with a fixed two-signal denominator: a phase that produced nothing scores zero, a phase-1 failure does not abort the run, and a session that cannot be established yields two zero signals
- [ ] 9.7 Unit-test the scoring rules of 9.6, including that a healthy-ingress / dead-egress run scores 0.5 rather than 1.0

## 10. Orchestrator: per-kind submission

- [ ] 10.1 Split the result submitter into one stream per kind, each reading and advancing its own watermark and posting to its own endpoint
- [ ] 10.2 Convert a liveness run into its submission shape: the average over the kind's fixed signal set with a missing signal counted as zero, carrying the per-signal breakdown
- [ ] 10.3 Keep the strictly-increasing timestamp behaviour per stream
- [ ] 10.4 Expose the per-signal breakdown and the test kind on the operator read surface (`/v1/results/*`)
- [ ] 10.5 Unit-test that submitting one stream does not advance the other's watermark, and that a failed post leaves its own watermark unmoved

## 11. nym-api: liveness ingest and shadow-weighted component

- [ ] 11.1 Add the liveness batch endpoint applying the same ordered validation as the stress endpoint (staleness, contract membership, per-signer monotonicity, signature)
- [ ] 11.2 Scope the per-signer replay high-water mark per endpoint so the stress and liveness streams cannot invalidate each other
- [ ] 11.3 Accept gateway-capable nodes on the liveness endpoint, and store each result with its per-signal breakdown, deduplicating on `(testrun_id, submitter_pubkey)`
- [ ] 11.4 Aggregate liveness results (average performance plus a reachability flag over a window) into a liveness score
- [ ] 11.5 Add the liveness performance component to the provider behind its own `use_*`, `minimum_available_*` and `*_score_weight` flags, with the weight defaulting to ZERO
- [ ] 11.6 Add the divergence metric comparing a node's aggregated liveness score against the v1 routing score, bucketed by whether the node announces a wss entry
- [ ] 11.7 Unit-test that interleaved stress and liveness submissions from one signer are both accepted, that a gateway entry is accepted on the liveness endpoint and dropped on the stress one, and that a zero-weight liveness component leaves detailed performance unchanged

## 12. Verification

- [ ] 12.1 `cargo build` the touched workspaces (contracts, nym-node, gateway, nym-api, nym-network-monitor-v3) and confirm no new warnings in the changed crates
- [ ] 12.2 `cargo test` the touched crates, including the contract tests and the orchestrator's sqlx-backed storage tests
- [ ] 12.3 Migrate the contract on a devnet and confirm an existing agent entry still reads back, then confirm a re-announcement populates its identity with no migration logic involved
- [ ] 12.4 Exercise mixnode liveness end to end against a testnet node and confirm a non-zero score with correct per-address attribution
- [ ] 12.5 Exercise gateway liveness end to end against a testnet gateway carrying task groups 6 and 7, and confirm both signals are non-zero
- [ ] 12.6 Confirm an un-upgraded gateway yields a zero egress signal and a non-zero ingress signal, so the divergence bucket behaves as designed
- [ ] 12.7 Confirm the orchestrator migration applies to a copy of a live orchestrator database with its staleness positions, rotation pointers and watermark preserved

## 1. Spikes that set the numbers

- [ ] 1.1 Measure the agent's achievable aggregate send rate in the fan-out shape (many sockets each at a low rate) rather than the existing single-connection batched figure, and record the result in design.md Open Question 1
- [ ] 1.2 From 1.1, fix the liveness profile defaults (per-target packet count, aggregate rate budget, straggler wait, per-target timeouts) and the resulting per-target worst case
- [ ] 1.3 From 1.2, fix `liveness_wave_size` and the liveness lease budget so that one concurrent wave completes well inside its lease
- [ ] 1.4 Confirm against a live wss-configured gateway that its plain client websocket port is still bound, and record the answer in design.md Open Question 3
- [ ] 1.5 Determine whether `insert_shared_keys` and `create_bandwidth_entry` can both be skipped cleanly for a monitor session; this decides whether the agent's client identity is ephemeral or derived (design.md Open Question 4)

## 2. Shared request/response types

- [ ] 2.1 Add a `TestKind` enum (`stress`, `liveness`) to `nym-network-monitor-orchestrator-requests`
- [ ] 2.2 Replace `TestRunAssignment` with a kind-tagged assignment carrying per-kind payloads: stress and mixnode-liveness (node address, node ips, noise key, sphinx key, key rotation id, probe profile) and gateway-liveness (additionally the client websocket port and the gateway identity key)
- [ ] 2.3 Make the assignment response carry a wave (a list of targets) for liveness and a single target for stress, with an empty response still meaning "no work"
- [ ] 2.4 Add a per-signal result shape (`signal` discriminator plus the existing counts, handshake durations and latency distributions) and make the submission request carry the run-level fields plus one or more signals
- [ ] 2.5 Add the liveness submission route constant and the liveness batch content type mirroring the stress `SignedMessage` envelope
- [ ] 2.6 Unit-test that a gateway-liveness run serialises and round-trips both of its signals, and that a single-signal run round-trips unchanged

## 3. Orchestrator storage and migration

- [ ] 3.1 Write migration `03`: create the per-kind work-state table keyed `(node_id, test_kind)` holding `last_tested_at`, `last_testrun_id`, `last_tested_ip`; create the per-signal child table of `testrun`; create the per-kind submission-watermark table; add `test_kind` to `testrun`; add `expires_at` and `test_kind` to `testrun_in_progress`; add the gateway client websocket port and the wss-announced flag to `nym_node`
- [ ] 3.2 In the same migration, backfill: move `nym_node.last_testrun` / `last_tested_ip` into the work-state table under the `stress` kind, derive `expires_at` for any live in-progress row from `started_at` plus the stress budget, carry `metadata.last_submitted_testrun_id` across as the stress stream's watermark, and project existing `testrun` rows as single-signal runs
- [ ] 3.3 Drop `nym_node.last_testrun` and `nym_node.last_tested_ip` once the backfill is in place
- [ ] 3.4 `touch build.rs` and confirm the compiled-in migrations DB picks up the new columns (a "table X has no column named Y" error means the build script did not re-run)
- [ ] 3.5 Update `storage/models.rs`: per-kind work-state row, per-signal row, `TestKind` mapping, and move the address rotation helpers (`announced_ips`, `next_ip_to_test`) onto the per-kind state so a kind rotates independently
- [ ] 3.6 Update the storage manager's insert path to write a run-level row plus its signal rows in one transaction, and the read paths to reassemble them
- [ ] 3.7 Update result eviction to delete signal rows with their run, and in-progress eviction to compare `expires_at` rather than a global cutoff
- [ ] 3.8 Unit-test the per-kind rotation (two kinds advancing independently over one node's address set) and that deleting a node's last testrun leaves its per-kind `last_tested_at` intact

## 4. Orchestrator scheduling

- [ ] 4.1 Rewrite `assign_next_mixnode_testrun` as a kind-aware assignment: choose the kind, filter by that kind's eligible node types and required non-null fields, apply that kind's staleness age, exclude any node with an in-progress row of any kind, apply `liveness_after_stress_cooldown` for liveness, take one target for stress or up to `liveness_wave_size` for liveness, advance each node's per-kind rotation pointer, and insert one in-progress row per target with its lease
- [ ] 4.2 Add the kind-selection policy (which kind an agent is handed when several are due) and the liveness enable flag that switches liveness assignment off without a redeploy
- [ ] 4.3 Extend the node refresher to record the entry-gateway client websocket port and whether a wss entry is announced
- [ ] 4.4 Add the liveness config knobs (staleness interval, lease budget, wave size, cooldown, enable flag) with the defaults from 1.2 and 1.3, all CLI- and env-overridable
- [ ] 4.5 Add prometheus series for liveness assignments, wave sizes, per-kind in-progress counts, lease expiries, and cooldown skips
- [ ] 4.6 Unit-test that a node with an open stress in-progress row is not assigned liveness and vice versa, that a recently stress-tested node is skipped by the cooldown, and that a wave never exceeds `liveness_wave_size`

## 5. nym-node: final-hop delivery for monitors

- [ ] 5.1 Replace the unconditional drop of network-monitor final-hop packets in `handle_final_hop` with delivery to a live client session
- [ ] 5.2 Suppress the on-disk fallback for network-monitor final-hop packets: when no session is live, drop and count the packet rather than storing it
- [ ] 5.3 Add metrics distinguishing a monitor final-hop packet delivered in-session from one dropped for want of a session
- [ ] 5.4 Unit-test both branches of 5.1 and 5.2, asserting that nothing is written to the store on the drop path

## 6. nym-node: ephemeral unmetered monitor client session

- [ ] 6.1 Add an `ephemeral` mode to `BandwidthStorageManager` that seeds a synthetic allowance and performs no read or write against `BandwidthGatewayStorage`
- [ ] 6.2 Make the client identity threaded into `ClientDetails` / `BandwidthStorageManager` optional (or a `Persisted` / `Ephemeral` discriminator) so a session with no storage row is representable
- [ ] 6.3 Recognise a client websocket connection whose source ip is in the authorised network-monitor set and route it into an ephemeral monitor session, skipping `insert_shared_keys`, `create_bandwidth_entry`, and the stored-message push
- [ ] 6.4 Ensure an out-of-bandwidth outcome is reported to the client as a distinguishable error rather than a generic failure, so a proxied gateway whose exemption missed is diagnosable
- [ ] 6.5 Unit-test that a monitor session forwards packets without a credential and leaves the gateway storage untouched, and that a non-monitor session is metered exactly as before

## 7. Agent: liveness profile and wave concurrency

- [ ] 7.1 Add the liveness probe profile alongside the stress profile in the tester config, with the aggregate rate budget deriving the per-target rate
- [ ] 7.2 Bind ONE shared ingress listener per invocation and build a `NoiseNetworkView` containing every target's noise key under every address that target is known by
- [ ] 7.3 Make the known-source set the union of every target's announced addresses, canonicalised
- [ ] 7.4 Attribute returned packets to a target by the source address of the connection they arrive on, and accumulate per-target results
- [ ] 7.5 Execute a wave as one concurrent batch with a hard per-target deadline, so the wave's duration is bounded by the slowest single target
- [ ] 7.6 Submit each target's result as soon as that target finishes rather than at the end of the wave
- [ ] 7.7 Unit-test attribution across a wave (several targets returning interleaved packets), and that one target timing out does not extend the others

## 8. Agent: gateway client session and the two-phase probe

- [ ] 8.1 Obtain the ed25519 client identity per the outcome of 1.5: ephemeral per test, or derived from the x25519 noise private key via a labelled HKDF whose output is used directly as the ed25519 seed (no CSPRNG seeding, no new on-disk key)
- [ ] 8.2 Establish the client session at `ws://<assigned-ip>:<clients_ws_port>`, constructed directly from the assignment, ignoring announced hostnames and wss entries and not reusing `ws_entry_address`
- [ ] 8.3 Implement the ingress phase: forward a sphinx packet through the session whose next hop is the agent's own mixnet address, and count arrivals at the shared listener
- [ ] 8.4 Implement the egress phase: send final-hop packets to the gateway's mixnet listener addressed to the agent's own client session, and count arrivals on that session
- [ ] 8.5 Hold the session open across both phases and their drain windows, and use one address family for both legs with the sphinx return hop matching it
- [ ] 8.6 Produce two signals with a fixed two-signal denominator: a phase that produced nothing scores zero, a phase-1 failure does not abort the run, and a session that cannot be established yields two zero signals
- [ ] 8.7 Unit-test the scoring rules of 8.6, including that a healthy-ingress / dead-egress run scores 0.5 rather than 1.0

## 9. Orchestrator: per-kind submission

- [ ] 9.1 Split the result submitter into one stream per kind, each reading and advancing its own watermark and posting to its own endpoint
- [ ] 9.2 Convert a liveness run into its submission shape: the average over the kind's fixed signal set with a missing signal counted as zero, carrying the per-signal breakdown
- [ ] 9.3 Keep the strictly-increasing timestamp behaviour per stream
- [ ] 9.4 Expose the per-signal breakdown and the test kind on the operator read surface (`/v1/results/*`)
- [ ] 9.5 Unit-test that submitting one stream does not advance the other's watermark, and that a failed post leaves its own watermark unmoved

## 10. nym-api: liveness ingest and shadow-weighted component

- [ ] 10.1 Add the liveness batch endpoint applying the same ordered validation as the stress endpoint (staleness, contract membership, per-signer monotonicity, signature)
- [ ] 10.2 Scope the per-signer replay high-water mark per endpoint so the stress and liveness streams cannot invalidate each other
- [ ] 10.3 Accept gateway-capable nodes on the liveness endpoint, and store each result with its per-signal breakdown, deduplicating on `(testrun_id, submitter_pubkey)`
- [ ] 10.4 Aggregate liveness results (average performance plus a reachability flag over a window) into a liveness score
- [ ] 10.5 Add the liveness performance component to the provider behind its own `use_*`, `minimum_available_*` and `*_score_weight` flags, with the weight defaulting to ZERO
- [ ] 10.6 Add the divergence metric comparing a node's aggregated liveness score against the v1 routing score, bucketed by whether the node announces a wss entry
- [ ] 10.7 Unit-test that interleaved stress and liveness submissions from one signer are both accepted, that a gateway entry is accepted on the liveness endpoint and dropped on the stress one, and that a zero-weight liveness component leaves detailed performance unchanged

## 11. Verification

- [ ] 11.1 `cargo build` the touched workspaces (nym-node, gateway, nym-api, nym-network-monitor-v3) and confirm no new warnings in the changed crates
- [ ] 11.2 `cargo test` the touched crates, including the orchestrator's sqlx-backed storage tests
- [ ] 11.3 Exercise mixnode liveness end to end against a testnet node and confirm a non-zero score with correct per-address attribution
- [ ] 11.4 Exercise gateway liveness end to end against a testnet gateway carrying tasks 5 and 6, and confirm both signals are non-zero
- [ ] 11.5 Confirm an un-upgraded gateway yields a zero egress signal and a non-zero ingress signal, so the divergence bucket behaves as designed
- [ ] 11.6 Confirm the migration applies to a copy of a live orchestrator database with its staleness positions, rotation pointers and watermark preserved

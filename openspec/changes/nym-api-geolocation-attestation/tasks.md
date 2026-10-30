Paths are under `nym-api/src` unless stated otherwise. Tasks may leave the tree red or hold temporary `todo!()`s; each group ends compiling, with what a later group fills stubbed by `todo!()`, so the group's own tests run.

## 0. Housekeeping

- [x] 0.1 `openspec archive node-status-api-geolocation-from-contract`, as it stands (its design, proposal and tasks still name `ProvenTrustAnchor`; that is the historical record), keeping the date the tool stamps; then `git diff openspec/specs` against its deltas block by block (one ADDED, two MODIFIED and one REMOVED block across `node-status-api-monitoring` and `node-status-api-http`); if the archive aborts, sync those blocks by hand and archive with `--skip-specs`; `openspec validate --specs --strict`
- [x] 0.2 `common/network-defaults/src/sandbox.rs`: `DIRECTORY_CONTRACT_ADDRESS` and `GEOLOCATION_CONTRACT_ADDRESS` set to the values in `envs/sandbox.env`; `PERFORMANCE_CONTRACT_ADDRESS` stays empty; `cargo check -p nym-network-defaults`

## 1. The raw verified read

- [x] 1.1 `common/nym-geolocation-client/src/client.rs`: `verified_records(height) -> Result<VerifiedRecords<GeolocationRecord>, VerificationError>`, the paged read at `height` through `verify_chain_read`, returned unchanged; `verified_geolocation` becomes that call followed by the existing `From`
- [x] 1.2 Test over a test type implementing the pinned geolocation and mixnet query traits (the `verify_chain_read` test pattern in `nym-contract-anchor`) serving the existing fixture records and digest at `H`: the raw read returns the fixture records, the whitelisted-agent records among them, and their accumulator; the grouped read at `H` equals `VerifiedGeolocation::from` of the raw read's result; a set that does not recompute fails both reads
- [x] 1.3 `cargo test -p nym-geolocation-client`, `cargo check -p nym-node-status-api` (its caller is unchanged)

## 2. The producer

- [x] 2.1 `nym-api/Cargo.toml`: `nym-geolocation-client`. `geolocation/mod.rs` and `geolocation/source.rs`: `GeolocationSnapshotSource` holding the contract address and a `GeolocationClient<SharedContractAnchor, QueryHttpRpcNyxdClient>`; `new(&SharedChainAnchor)` takes the address from `geolocation_contract_address()` (failing construction, so startup, without it, with the siblings' `context(..)`) and builds the anchor with `nym_geolocation_client::key::digest_item_key()`; `AttestedSnapshotSource` with `Record = GeolocationRecord`, `Auxiliary = ()`, `NAME = "geolocation"`, and `verified_set` mapping `verified_records(height)` to a `VerifiedSet` (accumulator, records as returned, node identities, `()`)
- [x] 2.2 `support/http/state/contract_snapshots.rs`: a `geolocation` field; `support/cli/run.rs`: `attested_snapshots::start_refresher` for the geolocation source right after the directory's, before the startup wait, persisting to `cache_file("geolocation_snapshots")`, with the shared `[directory.debug]` knobs; the module declared beside `directory`
- [x] 2.3 `cargo check -p nym-api`

## 3. The routes

- [x] 3.1 `nym-api/nym-api-requests`: `models/geolocation.rs` with `GeolocationEntriesRecordsResponse { height, entries: PaginatedResponse<GeolocationRecord> }`, documented like the performance one, no TS export; the manifest takes `nym-geolocation-contract-common` with its `utoipa` feature
- [x] 3.2 `geolocation/handlers.rs`: `/snapshot` redirecting to `/snapshot/latest`, `/snapshot/latest` (503 while none has settled), `/snapshot/{height}` and `/entries/{height}/records` (the generic state's paging: default 200, maximum 500; 404 outside the window), each with its own `utoipa` path under the tag "Nym Geolocation" documenting the 404 and 503; mounted at `/v1/geolocation` in `support/http/router.rs` beside `/v1/performance`. No node-identities route
- [x] 3.3 `cargo check -p nym-api-requests`, `cargo check -p nym-api --all-targets` (`utoipauto` collects the new paths at compile time, so a failing `utoipa` attribute shows up here)

## 4. The HTTP attestation source

- [ ] 4.1 `common/client-libs/validator-client` `NymApiClientExt`: `geolocation_snapshot_latest`, `geolocation_snapshot_at`, the paged `/v1/geolocation/entries/{height}/records`, all records at a height (pages of 500, failing when a page adds nothing before the announced total or overruns it, as the performance loop does), and the `SnapshotData<GeolocationRecord>` at a height assembled from those records and `get_directory_entries_identities` at the same height, beside the performance methods
- [ ] 4.2 `common/nym-geolocation-client/src/http.rs`: `NymApiGeolocationSource<C: NymApiClientExt>`'s three methods through task 4.1's, every error mapped to `AttestationSourceError::Transport`; `not_implemented` and its test removed; the module doc corrected to the `/entries/{height}/records` shape and to say the producer exists; the manifest enables the validator client's `http-client` feature, as the performance client's does (the geolocation client's does not today)
- [ ] 4.3 Tests as the performance source has them: each method returns the producer's response; the snapshot data joins every page of the records with the directory's identities at the same height; a client error maps to a transport error; records pages that never reach their announced total fail rather than page forever; a missing identity map maps to a transport error
- [ ] 4.4 `cargo test -p nym-geolocation-client`, `cargo check -p nym-validator-client --all-targets`

## 5. Verification and close-out

- [ ] 5.1 The whole `cargo test -p nym-api --bin nym-api`, `cargo check -p nym-api --all-targets`, `cargo check -p nym-api-requests`, `cargo test -p nym-geolocation-client`, `cargo check -p nym-validator-client -p nym-node-status-api -p nym-network-defaults`, `cargo fmt` clean
- [ ] 5.2 Read the full diff: no `todo!` or `unimplemented!`, every new `unwrap`/`expect` in test code or justified, every scenario mapped to a test or to design Decision 10's untested list; the sibling clients and node-status-api untouched apart from what task 1.1 required
- [ ] 5.3 `docs/geolocation/README.md`: the nym-api producer and its `/v1/geolocation` routes in the implementation map, the status updated (the HTTP source is no longer a stub)
- [ ] 5.4 Sync the main specs: `openspec archive` (the MODIFIED blocks keep every scenario title, so it should apply; if it aborts, sync by hand and archive with `--skip-specs`), write the Purpose of `geolocation-attestation-provider`, then `git diff openspec/specs` and `openspec validate --specs --strict`
- [ ] 5.5 PR description in the job scratchpad, calling out: the geolocation address now required on every nym-api (set on mainnet, filled for sandbox), localnet unable to start on this stack as before (pre-existing, directory address), the archived node-status-api change's spec sync riding along, the three new routes; one line for this PR in `~/workspace/global-performance-issues/plan.md`'s PR list

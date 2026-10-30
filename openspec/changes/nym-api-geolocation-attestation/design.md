## Context

nym-api attests two contracts today through one mechanism, `support/attested_snapshots/`: a per-contract source implementing `AttestedSnapshotSource` (the record type, an auxiliary value, the contract address, and the verified set at a height), one generic `start_refresher(source, &chain_anchor, ..)` per contract returning its `AttestedSnapshotsState`, one persisted file per contract, and the `[directory.debug]` knobs shared by all. The provider signs a `DigestSnapshot` (chain id, contract, height, app hash, accumulator, `node_identities_hash`) from what the source returns, with the app hash from the `SharedChainAnchor` built once at startup. `ContractSnapshots` on `AppState` groups the states, and each contract has its own route tree (`/v1/directory`, `/v1/performance`) with handlers in its own module. The decisions behind that shape, inherited here, are in the archived `2026-10-23-nym-api-performance-attestation` design (Decisions 2, 3 and 11): one cadence and retention by height so every producer holds the same heights, one chain anchor, one refresher and file per contract so one contract's failure never stops another's snapshots, warm-ups before the startup wait, and the contract address required on every nym-api.

The geolocation contract (`contracts/geolocation`, deployed on mainnet and sandbox) commits one record set to its digest: location entries keyed `(subject_class, subject_id, source)` with an opaque signed payload and `checked_at`, and the whitelisted-agent entries, which it folds into the same accumulator so that a verifying client can tell which measured entries were authorised. `SubjectClass` and `Subject` have one variant, `NymNode`. `common/nym-geolocation-client` verifies the whole set at a height through the shared `verify_chain_read` (digest proven under the contract's digest key, records paged at H, the mixnet bond set at H as the identity map) and returns `VerifiedGeolocation`, a view grouped per subject with the whitelist, which node-status-api resolves with `DefaultResolutionPolicy`. Its `http.rs` holds `NymApiGeolocationSource`, an `AttestationSource` whose three methods return a "not implemented" transport error, waiting for this producer. `nym-performance-client`'s HTTP source and the validator client's `NymApiClientExt` performance methods are the template for filling it.

The geolocation contract's address is in network-defaults (`GEOLOCATION_CONTRACT_ADDRESS`, filled on mainnet, empty on sandbox where `envs/sandbox.env` carries it), reachable through `NymContractsProvider::geolocation_contract_address`, and listed by `/v1/network/nym-contracts` since PR4a. The geolocation common crate has a `utoipa` feature (`GeolocationRecord: ToSchema`), used by `nym-geolocator`. The change `node-status-api-geolocation-from-contract` merged in #7168 with its change folder but was never archived.

This change is the first on `feat/nym-api-geolocation-attestation`, which is PR4b's head (#7365, stacked on #7351, #7313, #7298, #7224, #7223).

## Goals / Non-Goals

**Goals:**
- Every nym-api produces attested geolocation snapshots: the committed set re-served verbatim, signed, retained and served under `/v1/geolocation`, so that `NymApiGeolocationSource` has a server and a client without a chain connection can verify geolocation at a quorum-attested height, the same height that serves the directory and performance.
- The HTTP attestation source for geolocation implemented, with its client methods.
- The sandbox network defaults carrying the deployed directory and geolocation addresses.
- The merged node-status-api geolocation change archived, its deltas in the main specs.

**Non-Goals:**
- Any resolved, decoded or per-node geolocation view in nym-api. A later change adds `/v3/nym-nodes/geolocation/{node_id}` (the default policy applied at the latest settled height) and a detailed route with the full grouped records; the retained records are what it will compute from.
- A node-identities route under `/v1/geolocation`; the directory tree serves the map for every attested contract.
- Changes to the geolocation contract, the geolocator, or node-status-api's own proven read.
- Localnet: it deploys neither the directory nor the geolocation contract, and a localnet nym-api already fails startup on this stack for the missing directory address. Fixed properly in its own change.
- Sharing the mixnet bond read between the three producers (Decision 9).

## Decisions

### 1. A records-verbatim source with no auxiliary value

`GeolocationSnapshotSource` implements `AttestedSnapshotSource` with `Record = GeolocationRecord`, `Auxiliary = ()` and `NAME = "geolocation"`. At a boundary H it takes the raw verified read (Decision 2), and returns the accumulator, the records exactly as read, and the bond map. The records include the whitelisted-agent entries: they are in the digest, a verifying client folds them, and they are what lets a client decide which measured entries carried authority at H, so the producer cannot leave them out or serve them apart. The records stay in the order the contract pages them (key order: subject class, subject id, source), which nothing relies on today.

The auxiliary value is `()` because nothing unproven needs to be read beside the records: every field a consumer wants, the payload and `checked_at` included, is inside the digest leaf. Performance needed its auxiliary for medians and weights that live outside its digest; geolocation has no such data.

Alternatives considered: a decoded or resolved view held beside the snapshot (derivable from the retained records on request, so it is a view on the retained entry when a route wants it, not a chain read and not persisted state); serving the whitelist on a route of its own (splits the set a client must recompute over).

### 2. The raw verified read lives on the geolocation client, and only there

`GeolocationClient` gains `verified_records(height) -> Result<VerifiedRecords<GeolocationRecord>, VerificationError>`: the same paged read and `verify_chain_read` the grouped read performs, returning the anchor's result unchanged. `verified_geolocation` becomes that call followed by the existing `From<VerifiedRecords<GeolocationRecord>>`, so the two reads cannot diverge. node-status-api is untouched.

Why not widen `VerifiedGeolocation` with the accumulator, the record list and the identity map, as the sibling result types carry them: the grouped view would then hold every payload twice, and the producer would build a grouping it discards. Why not add the same raw method to the directory and performance clients for symmetry: the user weighed it and declined. It would buy consistency and let the directory producer skip a parsed view it discards (about a thousand node-signature verifications per boundary, tens of milliseconds), nothing else; attested consumers already get `VerifiedRecords` from the anchor, and the retained snapshots hold raw records for any later route. If the sibling clients are touched for another reason, the alignment can ride along.

### 3. The address is required on every nym-api

`GeolocationSnapshotSource::new` fails without `geolocation_contract_address()`, so startup fails, as it does for the directory and performance. The user's standing rule applies: enablement is never decided by address presence, and no flag is wanted, so a producer that silently does not run is worse than a startup failure. It costs nothing on mainnet, where the constant is filled and node-status-api already serves from the contract, and nothing on sandbox once Decision 7 lands. Localnet is already failing for the directory (Non-Goals).

### 4. The route tree mirrors performance's, identities from the directory

`/v1/geolocation/snapshot/latest` (503 while none has settled), `/snapshot/{height}`, `/entries/{height}/records` (paged through the generic state, default 200, maximum 500, 404 outside the retained window), plus the `/snapshot` redirect, mounted beside `/v1/performance` in `support/http/router.rs`. The records response is `GeolocationEntriesRecordsResponse { height, entries: PaginatedResponse<GeolocationRecord> }` in nym-api-requests, which takes the geolocation common crate with its existing `utoipa` feature, as it takes the performance one. No node-identities route: the map at H is the bond set the shared read returns, hashed identically into every contract's snapshot at H, and served by the directory tree (`directory-attestation-provider`, whole-directory serving requirement). A height whose directory snapshot failed while the geolocation one succeeded has no identities to serve, accepted for performance already.

The stub's doc comment names `/{height}/records`; the deployed trees serve `/entries/{height}/records`, and the spec pins only that the source addresses the directory's cadence heights, so the deployed shape is followed and the comment corrected.

### 5. Layout and names

`nym-api/src/geolocation/{mod.rs, source.rs, handlers.rs}`, flat: nym-api has no other geolocation code to separate an `attestation` submodule from, and the later V3 routes get their own file beside these. Cache file `geolocation_snapshots`; OpenAPI tag "Nym Geolocation", with `utoipauto` collecting the paths; `ContractSnapshots.geolocation`. Startup order in `run.rs`: the geolocation producer right after the directory's, before the startup wait; it joins no `StartupCaches` entry, since nothing internal consumes it, exactly like the directory.

### 6. The sibling specs stop naming contracts

`directory-attestation-provider`'s produce-only-what-was-verified and whole-directory serving requirements, and `nym-api-startup`'s startup requirement, enumerate "the directory, performance". They are reworded to "every attested contract", scenarios included where they enumerate, so that this and any further contract leaves them alone. Scenario titles are kept, because `openspec archive` reads a renamed scenario as a dropped one and aborts; if a title has to change after all, the archive is the hand sync with `--skip-specs` used for PR4b.

Alternative considered: adding geolocation to each list, keeping the explicit style. Declined by the user: the lists would grow with every contract.

### 7. Sandbox network defaults from `sandbox.env`

`common/network-defaults/src/sandbox.rs` gets the directory address `n1ayrk6w...` and the geolocation address `n1yn0mxz...` from `envs/sandbox.env`, which the user confirmed as the deployed truth. The performance constant stays empty: that contract is not on sandbox, and an empty constant parses to no address, which is what the performance window's startup check expects to find until it is deployed. Every other sandbox constant already matches the env file (checked line by line).

### 8. The node-status-api geolocation change is archived first

Task 0 runs `openspec archive node-status-api-geolocation-from-contract` on this branch: its deltas (one ADDED, two MODIFIED and one REMOVED block across `node-status-api-monitoring` and `node-status-api-http`, 36 of 36 tasks checked, strict-valid) have never reached the main specs although the code merged in #7168. It is archived as it stands: its design, proposal and tasks still name `ProvenTrustAnchor`, which PR4b replaced, but those are the historical record; only the spec deltas get applied, and they are clean. The folder keeps the date the tool stamps. The REMOVED block against two unusually detailed specs is the shape that has made past archives abort or drop prose, so the check is `git diff openspec/specs` against the deltas block by block, not the exit code.

### 9. The bond read stays per producer

Each producer reads the mixnet bond set at H inside `verify_chain_read`, so with three producers the same pages are fetched three times per boundary. PR4b accepted that for two to keep the producers' failures decoupled; the user chose to leave it for three. Sharing it would need a caching client wrapper or an identity-map parameter on the shared read, both in shared code, and the longer direction makes it moot: a digest-proven directory entry already binds a node's identity (the contract verified the signature against the bond at write time), so the directory is the eventual identity source for consumers.

Cost per boundary (every 100 blocks, about 8 minutes at ~5 s blocks), at about 1100 bonded nodes with about one entry each and the contract's default page of 50: about 22 record pages (~0.5 MB), the 2 anchor queries, about 22 bond pages (~0.4 MB), about 46 queries and ~0.9 MB in all. Across the three producers that is about 130 queries per boundary, 66 of them the same bond pages.

### 10. Testing

The client crate's HTTP source gets the five tests the performance source has: each method returns the producer's response, the snapshot data joins every records page with the directory's identities at the same height, a client error maps to a transport error, records pages that never reach their announced total fail rather than page forever, a missing identity map fails. The stub's "reports itself unimplemented" test goes with the stub. The raw read's relation to the grouped read is pinned by one test over the existing fixtures: the grouped read equals the grouping of the raw read's records.

In nym-api, one test only: a populated geolocation snapshot cache, every record shape and every `Option` set, round-trips through the bincode on-disk format, since `GeolocationRecord` had never been persisted there. Otherwise the source is wiring over `GeolocationClient` and the shared anchor (concrete types, as performance's untested source), the handlers are the generic state's accessors (as performance's untested handlers), and the generic machinery has its own tests. The validator-client methods stay untested, as the performance ones are. Untested by design: the source's construction failure on a missing address (it is the sibling sources' `context(..)?`), the `run.rs` wiring, the sandbox constants.

## Risks / Trade-offs

- [A localnet nym-api cannot start] → pre-existing on this stack for the directory address; stated in the PR; fixed in its own change.
- [A geolocation contract failure or an RPC fault at a boundary] → the generic rule: nothing is signed or retained for that height, the next poll tries the newest boundary again, and the directory and performance snapshots are unaffected because each has its own refresher and file.
- [Three copies of the bond read] → about 44 redundant queries and ~0.8 MB per boundary, accepted (Decision 9).
- [Light-client mode] → still rejected by configuration validation; the precondition noted in the stack's plan (a producer without a cache file warms up only the last boundary in that mode) applies to this producer too and is fixed in the change that enables the mode.
- [The archive of the node-status-api change aborts or drops prose] → `git diff openspec/specs` checked block by block against the deltas before the task is done.
- [The repeated-leaf weakness of the digest] → covers geolocation verification as it covers the siblings; producers are first-party, so it is not re-raised here.

## Migration Plan

Deploy is the nym-api release. Mainnet: the constant carries the address, the producer warms up its three boundaries at startup (the startup production runs before the cache wait, so startup takes the three verified reads longer), and `/v1/geolocation` serves from the first settled snapshot; no operator action. Sandbox: the filled constant, or the env entry already there. Rollback is the previous binary; the `geolocation_snapshots` cache file is ignored by a binary that does not know it. Consumers: `NymApiGeolocationSource` starts working against upgraded producers; a mixed fleet gives transport errors from old producers, which an `AttestedTrustAnchor` treats as non-answers.

## Open Questions

None for this change.

## Context

This change is the last of the stacked PRs that get network-monitor-v3 measurements into the performance contract and serve them verifiably: the contract side (`performance-contract-verifiable-routing-view`), the retrieval library (`performance-retrieval-client`), the orchestrator's submission (`network-monitor-contract-submission`) and nym-api's consumer (`nym-api-performance-contract-consumer`), all archived, then this one. They are reviewed separately and merged together. An orchestrator follow-up then adds a reachability input to the config score and faster liveness, and must be deployed before the switch is flipped.

What exists, as archived:

- **The performance contract** commits each node's newest bundle as a `LatestPerformance { node_id, epoch_id, score }` leaf of an LtHash digest stored raw at a provable key. The leaf commits the score only, so per-kind medians are readable but unproven (`EpochPerformancePaged`, `NodePerformance`). `LatestPerformancePaged` pages exactly the committed set.
- **`nym-performance-client`**: `PerformanceClient::verified_performance(H)` reads the committed set and the mixnet bonds at H, every page pinned, and checks them against the digest a `TrustAnchor` trusts for H; `VerifiedPerformance::routing_view()` gives every node bonded at H with its entry or nothing. `anchor().trusted_app_hash(H)` gives the app hash a producer signs. Attested consumers build the same result from `AttestedTrustAnchor::verified_records(H)`, but no nym-api source serves performance yet.
- **nym-api's directory producer** (`nym-api/src/directory/`): a `CacheItemProvider` that, at the directory contract's `snapshot_interval` boundaries (100 blocks by default), verifies the whole directory at H through `DirectoryClient`, signs a `DigestSnapshot`, keeps the newest `retention_count` (3) in a persisted cache, and serves them under `/v1/directory`, the latest promoted once the tip is `settle_lag` blocks past it. Two defects: `support/cli/run.rs` passes `retention_count` where `DirectoryState::new` expects `settle_lag` (3 instead of 10), and `RawCachedDirectory::parse` builds an empty `ParsedDirectory` placeholder. Only the proven anchor is reachable, since validation rejects `trusted_rpc_node = false`.
- **4a's consumer**: the window (`node_performance/contract_window/`, about 260 queries at startup) resolving each node's rewarding figure with the contract's own rule, read by contract-mode rewarding and selection and by the temporary `contract-divergence` route, all behind `[performance_provider].read_performance_contract`. In contract mode the node status refresher writes an empty annotation map, so published performance would read 0 for every node.

The annotation types: `NodeAnnotationV2 { current_role, chain_interaction_capabilities, detailed_performance: DetailedNodePerformanceV2 { performance_score, routing_score, config_score, stress_testing_score, liveness_score } }`. Only `/v2/nym-nodes/annotation/{node_id}` serves V2 directly (JSON/YAML); every other route converts to V1. The topology routes (skimmed and semi-skimmed, v2 and v3) read only `performance_score`, through V1's `last_24h_performance`; the breakdown is served by the per-node annotation and performance routes. The annotation cache is persisted with bincode.

## Goals / Non-Goals

**Goals:**

- Every nym-api serves attested performance snapshots a client without a chain connection can verify, under the directory's cadence and retained window.
- In contract mode, the performance every route publishes is the figure the contract committed, verified by nym-api against the digest.
- The directory and performance producers share one mechanism, and the directory's settle lag is the configured one.
- No configuration decides whether a nym-api reads the performance contract; the window lives only where rewarding runs.
- nym-api never publishes, rewards or selects from caches it has not built yet: startup waits for them.

**Non-Goals:**

- **The rewarding figure on annotations** (Decision 6).
- **Subset attestation for performance.** No subset routes; the directory's subset stubs stay as they are.
- **A geolocation producer.** The generic mechanism makes it a source implementation plus routes, later.
- **A client wired to the attested performance read.** The HTTP source is provided; a topology provider that verifies is later work.
- **Contract or message changes.**
- **Persisting the window** (deferred in 4a).

## Decisions

### 1. No read flag: every nym-api reads the performance contract, and the window lives where rewarding runs

`[performance_provider].read_performance_contract` goes. It was added by `nym-api-performance-contract-consumer`, which has not been released, so nothing deployed sets it; nym-api denies no unknown fields, so a config that does still loads, the value ignored. Every nym-api runs the performance producer (Decision 3), like the directory producer, so it reads the contract at every snapshot boundary. The address is assumed present and a missing one fails startup, as for the directory: the contract is initialised before this binary is deployed.

The window is held only by a nym-api with `[rewarding].enabled`. It serves contract-mode rewarding and selection and, with the switch off, the `contract-divergence` route. That is the only place either consumer exists: the epoch operations run only there, and the gap route compares nym-api's own figure, which only the monitoring nym-api that also rewards computes. Elsewhere the route answers 503 naming the window as the missing side. Serving needs no window (Decision 6), so a nym-api that does not reward pays none of its cost. Enablement follows `[rewarding].enabled`, the explicit flag of the feature consuming the window, never the presence of an address.

`use_performance_contract_data` (the switch) requires nothing. On the rewarding nym-api it decides which figure rewarding and selection use, and on every nym-api which figure is published (Decision 5).

Startup: the window rule of 4a applies to the rewarding nym-api only. With the switch on, the window must load before the epoch operations start, and a contract that never received a submission fails startup; with it off, the load is best effort (logged, retried, the gap route answering 503 until it succeeds). A nym-api that does not reward has no performance-specific startup check beyond the producer's own (the chain tip, the address, the snapshot interval). Accepted: flipping such a nym-api before the contract's first submission publishes 0 for every node with no startup error; the deploy order (contract fed, weeks of shadow, then the flip) excludes it.

`AppState` keeps `Option<PerformanceContractWindow>`, now `Some` exactly on the rewarding nym-api, and the switch. The `PerformanceMode` enum considered in 4a's review is not needed: the combinations it modelled were the flag's.

Alternatives considered: every nym-api holding a window (about 260 queries at each startup and 50 per epoch everywhere, for a window nothing there uses, plus a re-page cap after long poll failures); keeping a flag for the window alone (a knob to set on one nym-api, renamed since everyone reads the contract anyway); keeping the flag gating the producer as well (every attestation source has to set it before clients can reach a quorum, unlike the directory).

### 2. One snapshot mechanism, generic over the contract

The directory producer's pieces that are not directory-shaped become generic, in a new nym-api module (`support/attested_snapshots`):

- **The cache data**: `last_polled_height` and the retained snapshots by height, each holding the `SignedDigestSnapshot`, the verified records as served, the node-identity map, and an auxiliary value the source chooses (`()` for the directory, Decision 4), with `update` (insert, record the tip, prune to the retention count), `remove_stale`, `most_recent_entry(settle_lag)` and `get_entry`. `ParsedDirectory` and `parse` are dropped.
- **The provider**: a `CacheItemProvider` over a per-contract source. It reads the directory contract's `snapshot_interval` once, warms up the newest `retention_count` boundaries at startup, oldest first (each best effort, logged and skipped when it cannot be served), and at each poll takes the newest boundary strictly below the tip unless it is held. The cadence helpers (`expected_retained_heights`, `latest_snapshot_height`) move with it unchanged. The source supplies what differs: the contract address, the verified record set and node identities at H, the trusted app hash at H, and the auxiliary value. The provider builds and signs the `DigestSnapshot` (chain id, contract, H, app hash, accumulator, `node_identities_hash`).
- **The state**: `latest_snapshot`, `snapshot_at`, `paged_entries_at`, `node_identities_at`, over the cache and the settle lag. The directory's state wraps it and keeps its two subset stubs.

The directory moves onto it with its routes, responses and on-disk bytes unchanged: the retained entry holds the same three fields in the same order, and the directory's auxiliary `()` and the dropped empty `ParsedDirectory` both encode to nothing. Each contract keeps its own refresher and persisted file, so one contract's failure never stops the other's snapshots; the chain anchor is shared (Decision 3). Both read their knobs from `[directory.debug]` (`retention_count`, `settle_lag`, `polling_interval`), documented there as shared by every attested contract. That keeps the cadence and the retained window identical across contracts, which `directory-attestation-provider` requires so a consumer can join two contracts at one height.

The settle lag passed to the state is `[directory.debug].settle_lag`, fixing the pre-existing bug: the directory's latest snapshot is then promoted 10 blocks after its boundary instead of 3. The PR description calls this out.

Alternatives considered: copying the directory pattern for performance and extracting later; one refresher snapshotting every contract at one height (couples the contracts' failures and rewrites the deployed directory cache); a new shared config section, or one per contract (a fixup for no gain, or knobs that must never differ).

### 3. The performance producer

Trust in the chain is shared by every attested contract, in a new nym-api module (`support/chain_anchor`): a `SharedChainAnchor` built once at startup, a proven read through the RPC node when `[directory.debug].trusted_rpc_node` is set and otherwise a light client seeded from a checkpoint, which owns the persisted light-client head (`nyx_checkpoint.json`) and the chain id both producers sign. Each contract takes a `ContractAnchor` from it: the shared anchor's app hash at H, and the contract's own digest proven against it under the contract's bare item key. In light-client mode each header is therefore verified once for every contract, and the head file has a single writer. The light-client mode stays rejected by configuration validation until checkpoint distribution is deployed; with this structure, enabling it then covers every contract at once. The directory's checkpoint loading moves into that module unchanged. To make the shared anchor contract-free, `nym-contract-anchor` splits its traits: `ChainAnchor` (the app hash at a height, implemented by `ProvenChainAnchor` and `LightClientAnchor`, which no longer take a contract) and `TrustAnchor: ChainAnchor` (one contract's digest), with a generic `ContractAnchor<A, C>` pairing a chain anchor with a contract's digest proof; `AttestedTrustAnchor` implements both. This breaks the crate's public API (`ProvenTrustAnchor` is gone, `LightClientAnchor::new` loses its contract and key), accepted by the user. The performance source wraps `PerformanceClient` with its `ContractAnchor` on the performance contract's address and the bare key `storage_keys::DIGEST_STATE`. At a boundary H it calls `verified_performance(H)` and signs the app hash from `anchor().trusted_app_hash(H)`, the accumulator it verified and the hash of the bond map at H, then retains the entries as served (`entries.values()`) and the bond map. A set that fails verification is not signed or retained; the failure is logged, and the next poll tries the newest boundary again. This is the shadow period's digest self-check: every nym-api verifies every snapshot it produces. A contract without submissions verifies as the empty set and is signed as such.

It runs on every nym-api, independent of the switch, warms up during startup before anything waits on the caches (Decision 11), and persists its cache to its own file (`performance_snapshots`). A file that fails to load falls back to an empty value, which the warm-up refills. The warm-up produces its retained heights oldest first, so Decision 4's skip rule only moves forward in height; the directory's warm-up uses the same order, which changes nothing for it.

Routes under `/v1/performance`, mirroring `/v1/directory` without its subset tree and without a node-identities route: `/snapshot/latest` (503 while none has settled), `/snapshot/{height}` and `/entries/{height}/records` (paged, default 200, max 500; 404 for a height outside the retained window). The records are `LatestPerformance` exactly as committed. The identity map is the mixnet bond set at H, built by the shared read both producers use, so at a shared cadence height it equals the directory's and both snapshots sign the same `node_identities_hash`; clients take it from `/v1/directory/entries/{height}/node-identities` (user's call). The producer still reads and keeps its own copy, to hash it and to build the routing view. Accepted: a height whose directory snapshot failed while the performance one succeeded has no identities to serve, so a no-RPC client verifies another retained height. The performance contract's common crate gains an optional `utoipa` feature deriving `ToSchema` on `LatestPerformance`, as the directory's does for its record.

Cost per boundary (100 blocks, about 8 minutes at ~5 s blocks) on every nym-api, at about 1100 bonded nodes and the contracts' default page sizes: the header at H + 1 and the digest proof inside the anchor (2), the `LatestPerformancePaged` pages (100 per page, about 11), the mixnet bond pages for the identity map (50 per page, about 22), the producer's own `trusted_app_hash` header (1) and the header at H for its block time (1, Decision 7): about 37 queries and roughly 0.5 MB of responses, the bonds being most of it. Decision 4 adds about 2 queries to most boundaries and 11 to 22 to the one or two per epoch that follow submissions, so about 280 queries an hour, plus the refresher's tip poll every 30 s (120 an hour). The startup warm-up does three boundaries, about 120 to 180 queries. The directory producer reads the same bond map at the same heights; sharing that read would save about 22 queries and 0.4 MB per boundary but couple the two producers' failures (Decision 2), so it is not done.

### 4. Unproven medians and weights beside each snapshot

Topology routes carry only the proven figure; the annotation routes may serve unproven values (user's call). So at each boundary H, after verification, the performance source also reads, every query pinned at H:

- the weights in force at H (`CurrentWeights {}`), one query every boundary;
- the per-node medians of the newest epoch among the snapshot's entries and of the epoch before it (the newest is the largest entry epoch, so finding it costs no query), through `EpochPerformancePaged`, about 11 queries and 150 KB per epoch. Entries from older epochs, the tail of nodes no longer measured, get no medians, and there is no per-node fallback: the data serves only the annotation routes (user's call). Two epochs rather than one cover the minutes while an epoch's chunks land, when the nodes whose chunk has not arrived yet still sit at the previous epoch.

Most boundaries need no pages at all, since the two epochs overlap from one boundary to the next. The source reads `LastSubmittedMeasurement` at H first. When it equals the submission recorded at the previous boundary, nothing was submitted in between and the medians held are still exact at H, so they are reused. When it differs, each of the two epochs not older than the recorded submission's is re-paged (normally the newest alone; at an epoch boundary also the previous one, which may have taken a late chunk), and the medians held for an older one are kept. Older epochs never need re-reading: an epoch is frozen once its transition begins. So a boundary costs about 2 queries when nothing landed and 11 to 22 after submissions, about 35 an epoch instead of about 150. The recorded submission is persisted with the cache as a key of its fields `(block_height, epoch_id, sender, last node_id)`, not as the `LastSubmission` struct, whose `Measurements` use `skip_serializing_if`, which bincode cannot round-trip; a restart within the retained window therefore re-pages nothing unless something landed.

They are held as the snapshot's auxiliary value: the weights, the medians per node, and the block time of H (Decision 7). They are never signed and never served on `/v1/performance`; only the annotation builder reads them (Decision 5). Pinning at H makes the medians exactly the bundles behind the committed scores, since the current epoch's bundles can change after H. The reads are part of the snapshot (user's call): if any fails, nothing is retained for H and the next poll retries it, the same as a failed verification. Accepted: a persistent failure of these unproven reads stops every performance snapshot, the proven ones included, until it is fixed.

The validator client's `PinnedPerformanceQueryClient` gains three pinned reads: the current weights, the last submission, and one epoch's `EpochPerformancePaged` pages. The epoch-keyed page needs its query built from the epoch, so the pinned paging helper takes the query it pages as a parameter rather than only from the page type.

Alternatives considered: no medians (the annotation routes would show config, stress and liveness 0 in contract mode); reading them in the annotation refresher at each refresh (chain reads back in a refresher contract mode had freed of them, repeated per refresh rather than once per snapshot); the medians of every entry's own epoch, paging epochs that hold many entries and querying `NodePerformance` for the rest (complete, but up to about one query per node in a scattered tail, for auxiliary data); only the newest epoch (default sub-scores for the not-yet-submitted nodes during each submission); the skip state in memory only (a restart re-pages both epochs); per-source-epoch weights through a `Weights` helper shared with the contract's `score` (exact for entries older than an `UpdateWeights`, but touches the contract's frozen scoring code; rejected as overcomplicated); decoupling the auxiliary reads from the snapshot (my recommendation: an auxiliary failure would only blank the sub-scores; the user kept them atomic).

### 5. Contract-mode annotations from the verified routing view

With the switch on, the node status refresher's contract-mode branch builds the annotation map from the performance state's latest settled snapshot instead of writing an empty one. For every node in the snapshot's bond map with a committed entry:

- `performance_score`: the entry's score, 0 when the entry has none. This is the only field the topology routes publish, and it is exactly the committed value.
- `source_epoch`: the entry's epoch (Decision 7).
- `config_score.score`, `stress_testing_score.score`, `liveness_score.score`: the entry's bundle medians from the snapshot's auxiliary value when the entry's epoch is one of the two the producer read (Decision 4), each 0 when the kind has no median; `was_reachable` is whether the stress or liveness median is present (a monitor collected a sample of that kind). An entry from an older epoch has no medians, so all its sub-scores stay at their defaults. The config breakdown fields (versions behind, terms, chain capabilities) have no source in the contract and stay at their defaults.
- `routing_score`: recomputed from the medians as the weighted mean of the liveness and stress medians present, renormalised over their weights, under the weights at H (the contract's score before its config factor); 0 when neither is present. An entry from an epoch before an `UpdateWeights` is recomputed under the newer weights, accepted since the field is informational and retires with the legacy path. The formula is a small pure function in nym-api, not shared with the contract.
- `current_role` from the current rewarded set; `chain_interaction_capabilities` none (contract mode does not query them, 4a).
- `computed_at`: the block time of the snapshot's height H (Decision 7).

A bonded node without a committed entry, never measured, gets no annotation and is served 0, like a node never annotated in legacy mode. There is no describe-based filter: whether a node is alive comes from the network monitor's measurements, and nym-api and other components will read the directory contract rather than self-descriptions. An entry with no score is annotated with performance 0, its source epoch and its medians (user's call), which shows it was measured and why it has no figure. Such an entry is a bundle holding values that `Weights::score` cannot score: no config median, or only zero-weighted kinds measured (the contract rejects a submission with every kind absent). The orchestrator's submission filter and the initial weights rule it out; it needs a weights update zeroing a kind, or a different submitter.

When no snapshot has settled, the refresh leaves the map as it is and logs. The branch is near-unreachable: the producer's warm-up runs at startup before the annotation cache is waited on (Decision 11), over the same heights and RPC as the directory's, so it lacks a snapshot only when the RPC cannot serve any of the last three boundaries, a transient failure hits all three fetches, or verification fails. Leaving the map never publishes an all-zero topology; the accepted cost is that right after a flip in that case the legacy map loaded from disk stays published until a snapshot settles. On a start with no fresh persisted map, the annotation cache then stays unbuilt, so startup waits until a snapshot settles, consistent with Decision 11. 4a's reason for writing an empty map was that contract mode had no source at all, so legacy figures would have been served indefinitely; that no longer holds.

The refresher's triggers are unchanged (mixnet contract cache, describe cache, refresh requests, its fallback interval), and so is its 305 s rate limit apart from the first refresh (Decision 11). A new snapshot settles about every 8 minutes, so annotations trail it by up to about 5 minutes, and a submission reaches them within about 15; the figure changes once per epoch, so a "snapshot settled" trigger was not worth a new signal and a contract-mode exception to the rate limit (user's call). The builder is a pure function over the snapshot's entries, bond map and auxiliary value, and the rewarded set.

As consequences: the v1 monitor's route preparer, which weights candidates by `performance_score`, follows the contract's figure in contract mode instead of every candidate taking its default weight; and the temporary `liveness-divergence` route answers 503 in contract mode, naming nym-api's own figure as missing, as `contract-divergence` does (user's call): its inputs would be the contract's liveness median and the recomputed routing figure, whose difference (about 0.2 x (liveness - stress) for a mixnode, 0 for a gateway) is not the comparison the route exists for.

### 6. No rewarding figure on the annotations

The annotations carry the routing figure only, on every nym-api (user's call). Nothing in nym-api would read a rewarding figure there (contract-mode rewarding and selection read the window directly), serving must not assume the same nym-api rewards, and the pay figure is on chain: `RewardingScore { epoch_id, node_id }` runs the contract's own rule, and each `RewardNode` carries the performance it paid. The two figures differ only in rare cases: a partial gap longer than 24 epochs (one test kind omitted, or a still-bonded node no longer tested, while R advances on the others), where routing keeps the old entry and rewarding pays zero, readable off the source epoch; and the minutes after an `UpdateWeights` before the next submission re-scores a node. A global outage does not separate them, because R anchoring freezes rewarding at the same newest bundles the routing view serves. This is what lets serving run without the window (Decision 1).

Alternatives considered: the rewarding figure on every nym-api (every nym-api holds a window); only where a window exists (the same route answers differently across nym-apis).

### 7. Two optional fields on `DetailedNodePerformanceV2`

- `source_epoch: Option<EpochId>`: the performance contract epoch the figure comes from; set in contract mode, absent in legacy mode.
- `computed_at: Option<OffsetDateTime>`: when the figure was taken, in both modes. In contract mode it is the block time of the snapshot's height H (user's call), so it moves only when a new snapshot settles and shows a stalled producer; the anchor's `trusted_app_hash` returns only the hash, so the producer reads the header at H for it (one query per boundary) and keeps it beside the snapshot. In legacy mode it is the time of the refresh that computed the figure, and an annotation carried over (4a's rule) keeps its own, so a carried figure's age is visible.

They sit on the performance struct rather than the annotation because the carry-over keeps `detailed_performance` as a whole. Both are `Option`s without `skip_serializing_if`, since the annotation cache is persisted with bincode, which cannot round-trip a skipped field; serde reads a missing `Option` as `None`, so a JSON reader of an older nym-api still decodes. The V1 types are unchanged, so no bincode client is affected; `/v2/nym-nodes/annotation/{node_id}` serves JSON and YAML only. The TS bindings are regenerated with the crate's usual feature set, never `--all-features`. No snapshot height is added: `source_epoch` dates the figure, and a verifier reads `/v1/performance` at a height it quorum'd (user's call).

The persisted annotation cache changes shape, so its file is renamed and an older binary's file is never decoded as the new one (a misaligned bincode read could in principle decode into wrong values rather than fail). Since nothing is served before the annotation cache is built (Decision 11), the upgrade restart publishes no zeros: it waits for the first refresh instead.

### 8. The HTTP attestation source for performance

`nym-performance-client` gains `NymApiPerformanceSource<C: NymApiClientExt>`, implementing `AttestationSource` with `type Record = LatestPerformance`, modelled on the directory's `NymApiAttestationSource`: it holds the client and the expected signer identity, fetches the latest snapshot, a snapshot at a height and the snapshot data at a height (all pages of the performance records, then the node identities from the directory tree at the same height, Decision 3), and maps every client failure to `AttestationSourceError::Transport`. The client methods go on validator-client `NymApiClientExt` beside the directory's (`performance_snapshot_latest`, `performance_snapshot_at`, the paged records, and the snapshot data assembled with the existing `get_directory_entries_identities`). An `AttestedTrustAnchor` over these sources then yields `VerifiedPerformance::from(anchor.verified_records(H))`, which PR2 already tests through mock sources.

### 9. The rewarding view in one module

The contract-mode rules move from the epoch operations onto the window, as methods on `WindowData` beside `figure`: the rewarding performance (the figure's score, or zero) and the selection performance (the score, 1 in fault mode for any score above zero, nothing without a score), with fault mode read from the window's own data rather than computed by the caller. Their tests move with them, so the `#[cfg(test)] ContractFigure::with_score` helper goes. The epoch operations call the methods; the fault-mode warning stays where selection runs. The window already imports nothing from nym-api except through its refresher (X from the mixnet contract cache), so the whole rewarding view lifts into the dedicated rewarding binary later as one module.

Alternatives considered: a crate in `common/` (compile-time isolation, but a 900-line move, a public API with one consumer and a licence choice, while the binary still needs the epoch operations extracted); leaving the rules in the epoch operations.

### 10. Configuration

- Removed: `[performance_provider].read_performance_contract` and its template line. The switch's validation no longer requires it.
- `use_performance_contract_data`: its doc and template comment say it decides the figure rewarding and selection use on the rewarding nym-api, and on every nym-api the published one (the verified routing figure in contract mode).
- `contract_polling_interval`: its doc says it applies to the window, held only with `[rewarding].enabled`.
- `[directory.debug]`: `retention_count`, `settle_lag` and `polling_interval` documented as shared by every attested contract.

### 11. Startup waits for its caches

nym-api must not publish performance 0 while its caches build (user's call). Today it does: the annotation cache is the only cache with a placeholder value (an empty map, `node_status_api/cache/mod.rs:62`), which the topology routes serve as performance 0, while every other uninitialised cache answers 503 (`node_status_api/models.rs:406`). The annotation refresher's first refresh also waits for the describe cache's first full pass, even when a fresh persisted describe file is loaded, and its 305 s rate limit counts from its own start, so today the first annotation refresh lands about 305 s after startup and a restart with an annotation file older than that serves zeros for about 5 minutes. Legacy epoch operations do not wait for annotations either (`epoch_operations/mod.rs:254-259`): an epoch ending in that window would pay every node zero (`helpers.rs:214`) and leave selection without candidates (`rewarded_set_assignment.rs:352`).

So startup becomes: build and start the cache refreshers (mixnet contract, describe, node families, annotations), with the directory and performance warm-ups running synchronously as now; wait until those four caches hold values, a value loaded from a fresh persisted file counting; then spawn everything else: the window, the DKG controller, the ecash background cleaner, the network monitor and uptime updater, the epoch advancer, key rotation, the signers cache refresher and the HTTP server. Each task keeps its own `naive_wait_for_initial_values`, redundant after the wait but keeping it correct on its own. Supporting changes:

- the annotation cache loses its placeholder: it is uninitialised until loaded from a fresh file or refreshed once;
- the annotation refresher's readiness check asks whether the caches it reads hold values, not whether their first refresh has finished, so persisted describe data is enough; in contract mode it reads only the mixnet contract cache and the performance snapshots. Its first refresh runs as soon as they do, outside the 305 s rate limit;
- a cache whose first build fails is retried every 30 s until it first succeeds (user's call), then at its normal interval; today a failure waits for the next tick (150 s for the mixnet cache, 305 s for annotations, 600 s for node families). This lives in `CacheRefresher` and in the annotation refresher;
- while waiting, startup logs which caches are still pending. There is no cap: nothing correct can be served before the caches exist, and an RPC outage already fails startup in the directory warm-up.

Exempt from the wait: the signers cache, which probes this nym-api's own HTTP and starts after a delay for that reason (`support/config/mod.rs:860`); it moves after the wait, so its 30 s delay still covers the HTTP server coming up, and a fresh install no longer self-reports as down for a whole 600 s refresh interval. The window on the rewarding nym-api in shadow mode, best effort by Decision 1. The on-demand caches (contract details, chain status, network monitors, address info), which fill on first request.

On a restart the wait is seconds; on a fresh install, or after more than 75 minutes down, it is the describe pass (about 1100 nodes at 50 concurrent) and the first annotation refresh, during which nym-api serves nothing, ecash routes included, and DKG and ecash tasks start that much later. A cache that never builds keeps startup waiting. Specified as its own capability, `nym-api-startup`, since most of it does not concern performance.

Alternatives considered: gating only the HTTP server (the legacy epoch operations would still read an unbuilt annotation cache); answering 503 on the annotation routes until ready (route-by-route, and the epoch operations unchanged); only the refresher relaxation (zeros on a normal restart shrink to seconds, a fresh install still publishes them); gating every cache, the signers cache included (it would always record this signer as down on its first refresh); retrying a first build at the normal intervals (one transient node-families error would hold startup for 10 minutes).

### 12. Testing

Offline only, with deterministic fixtures.

- **The generic cache data and cadence**: the directory's existing tests move with them (prune to retention, remove stale, settle lag, recording the tip after warm-up, saturation, the cadence helpers), plus a bincode round trip of a populated performance cache, auxiliary value and recorded submission key included.
- **The medians read**, over a test type implementing the pinned performance query trait: the newest entry epoch and the one before it are paged at H; an unchanged last submission pages nothing and reuses the medians; a changed one re-pages those of the two not older than the recorded submission's epoch and keeps the held medians of an older one; a failed read fails the snapshot; the weights and the block time come from H.
- **The routing recompute**: liveness and stress renormalised over the kinds present, one kind alone, neither, and agreement with the contract's score divided by its config median where no rounding intervenes.
- **The annotation builder**: performance from the committed score; sub-scores and `was_reachable` from the medians; an entry from an older epoch with default sub-scores; a never-measured node without an annotation; an unscoreable entry at 0 with its epoch and medians; the source epoch and `computed_at` (the snapshot's block time) set; the role from the rewarded set.
- **The V2 fields**: the annotation cache round-trips through bincode with both set; a legacy carried annotation keeps its `computed_at`.
- **The HTTP source**: each method delegates to the right client method, the snapshot data joins the performance records with the directory's identities at the same height, and a client error maps to a transport error, as the directory's tests do.
- **Startup**: a `CacheRefresher` whose provider fails its first build retries after 30 s and then keeps its normal interval (paused tokio time); the startup wait returns once every cache holds a value, persisted values counting.
- **The window's methods**: the rewarding and selection tests that 4a wrote against the epoch operations, moved.
- **Config**: a configuration still setting `read_performance_contract` loads; the switch validates on its own.

Not unit-tested, each a few lines over tested pieces:

- the performance source's own reads around the tested auxiliary read: the header at H for the block time (a `?` in `verified_set`, read through the RPC client, whose trait is too wide to fake for one call);
- the startup wiring: the order of `run.rs` around the wait, the producer and the window per `[rewarding].enabled`, a missing address failing startup, the settle lag passed to the state ("The configured settle lag decides latest");
- the handlers, which delegate to the tested state, the gap route's 503 for a nym-api without a window, and `liveness-divergence`'s 503 in contract mode;
- the annotation refresher around the builder: its contract-mode branch, leaving the map when no snapshot has settled, its readiness check and its first refresh outside the rate limit.

## Risks / Trade-offs

- [Every nym-api reads the performance contract] -> A contract address missing from the network details stops every nym-api, including externally run ones. The mainnet constant ships with this release; environments without the contract need its address before upgrading.
- [The directory's latest moves 7 blocks later] -> Clients see a latest snapshot about 35 seconds older than before; it now matches the documented settle lag.
- [Unproven sub-scores] -> The annotation routes' config, stress, liveness and routing values in contract mode are read from the contract without proof; topology routes never use them. Nodes whose entry is older than the epoch before the newest show default sub-scores beside their committed score.
- [A persistent failure of the unproven reads] -> Stops every performance snapshot, proven output included, since the reads are atomic with the snapshot (user's call); logged on every poll.
- [Startup serves nothing until its caches are built] -> Seconds on a restart; on a fresh install or after more than 75 minutes down, the describe pass plus the first annotation refresh, ecash routes and DKG included. A cache that never builds keeps startup waiting, logged.
- [No identities for a height whose directory snapshot failed] -> A no-RPC client verifies another retained height; both producers read the same RPC at the same heights, so they normally fail together.
- [A non-rewarding nym-api flipped on an empty contract] -> Publishes 0 for every node without failing; excluded by the deploy order.

## Migration Plan

No contract, message or database migration. The persisted annotation cache gets a new file name, so the upgrade restart waits for its first refresh before serving; the directory cache keeps its name and bytes; the performance cache is new.

Deployment: the performance contract's address must be in every nym-api's network details (the mainnet constant set in this release, or the environment). Every nym-api then produces performance snapshots; the rewarding nym-api also holds the window. With the switch off nothing published changes apart from the directory's settle lag and the two absent V2 fields. The flip (`use_performance_contract_data = true`) waits for the orchestrator's aliveness follow-up; it then switches rewarding and selection on the rewarding nym-api and published performance on every nym-api that sets it.

Rollback: switch off (legacy figures again under the unchanged scoring section) and restart; rolling the binary back to `nym-api-performance-contract-consumer` also brings back its read flag (off by default).

## Open Questions

None for this change.

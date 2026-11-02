## Context

`support/attested_snapshots` is generic over a per-contract `AttestedSnapshotSource`: `AttestedSnapshotsProvider<S>` produces one contract's snapshot at a height and polls for the newest boundary as a `CacheItemProvider`, a `CacheRefresher` per contract writes its `SharedCache<AttestedSnapshotsCacheData<S>>` and its own file, and `AttestedSnapshotsState<S>` serves the routes. `run.rs` calls `start_refresher` three times, each awaiting its warm-up, with a `SharedChainAnchor` (`support/chain_anchor/`) cloned into each so the three contract anchors prove against one chain anchor. Each contract's verified read goes through its contract client (`verified_directory`, `verified_performance`, `verified_records`), each a wrapper over `nym-contract-anchor`'s `verify_chain_read` (`whole_set.rs:49`) that pages the mixnet bond set at the height itself, then parses or groups the records in a way the producer discards.

The three contracts agree on heights by construction but fail independently, so a contract can miss a height the others hold and the identity map, served only by the directory tree, can be missing for a performance or geolocation height. The user ruled that independent failure is not a requirement, that rebuilding every cache at the upgrade is fine, and that the snapshot interval changes so rarely, if ever, that a change is a coordinated restart rather than something to follow at runtime.

## Goals / Non-Goals

**Goals:**
- Every cadence height a nym-api produces holds every attested contract, and every tree's latest is the same height.
- One bond read per height.
- Less machinery than today, not more: one cache, one file, one loop, no generics over contracts.
- No change to routes, responses or configuration.

**Non-Goals:**
- Following an on-chain interval change without a restart.
- Keeping the per-contract cache files readable.
- Serving the identity map from any other tree, or a "newest common height" route: with no gaps there is nothing for them to bridge.
- Changing node-status-api, the attestation format, or the contract clients' behaviour.

## Decisions

### 1. One retained entry per height, one cache, one file

`RetainedHeight` holds the directory, performance and geolocation `AttestedSnapshot`s at one height plus the performance auxiliary; `AttestedSnapshotsCacheData` is `{ last_polled_height, entries: BTreeMap<Height, RetainedHeight> }`, concrete, persisted to `attested_snapshots`. All or nothing is then structural: an entry exists for a height or it does not, there is no partial height to reconcile, no second and third file to fall out of step, and no "produce again for the contracts that lack it" logic. The generic `AttestedSnapshotsCacheData<S>`, `RetainedEntry<S>` and `AttestedSnapshotsUpdate<S>` go, with the test that decoded the directory's old file layout: that file is no longer loaded, and a startup refills three heights at about the cost today's performance and geolocation warm-ups already pay.

Alternative considered, and proposed in the first draft of this change: per-contract caches and files kept, with a producer keeping them in step. Rejected once the user allowed the caches to be rebuilt: the in-step logic (partial heights, re-production, three writes) was the bulk of that design.

### 2. Handlers read their contract's snapshot through accessors

`ContractSnapshots { cache, settle_lag }`, the state the routes hold in place of today's per-contract `AttestedSnapshotsState`s, keeps the height logic (`latest_entry`, `entry_at`, the settle lag) over `RetainedHeight`s and has two accessors per contract, `latest_directory` and `directory_at(height)` and their performance and geolocation counterparts, each yielding the contract's `AttestedSnapshot<R>` under the cache's read guard. What a route takes from a snapshot lives on `AttestedSnapshot<R>`: `digest_snapshot`, `node_identities` and `paged_records` (default 200, maximum 500). The annotation refresher and the v3 performance routes, which read the performance auxiliary, take the whole entry. A view per contract, the state paired with a projection to its snapshot, was considered and dropped: it held the whole state only to reach one field of it.

### 3. The producer calls the shared read, with the identity map supplied

`verify_chain_read` takes the node-identity map as a value, in place of the query client it read the bond set through. The clients read it with `node_identities_at(&self.client, height)` before the call, the read they made inside it before; the producer reads the map once per height with the same function and passes each contract's read a copy, which each contract's snapshot stores anyway. Only the records stay a future, so that the large read still runs only once the digest is established; a client's bond read now precedes the digest, which wastes it only when the anchor fails. A future for the map, which kept the old order, was considered and rejected as less readable: the producer would only have wrapped a map it already holds. The contract clients are otherwise untouched: the producer no longer goes through their reads, since it only wants the raw `VerifiedRecords` those reads wrap and discard into a parse or a grouping. The producer holds each contract's `ContractAnchor` and reads each contract through a method of its own, `verified_directory`, `verified_geolocation` and `verified_performance`, each `verify_chain_read` over the contract's pinned records query through the chain anchor's one query client; The performance auxiliary is read by a method of its own, `performance_auxiliary`, once all three contracts verified, with the value held for the newest retained height below: every contract read then has one shape, and a height that fails verification reads no auxiliary, at the cost of the auxiliary reads no longer overlapping the contract reads. A source struct per contract was dropped: each held only its address, its anchor and a copy of the query client, for the producer as its only user, and the address a signed snapshot names now comes from `ContractAnchor::contract()`. `verified_performance` sorts its records by node id, as the client's grouping did: the per-node lookup's binary search relies on it, and the digest commits no order, so an untrusted RPC under the light client could reorder them.

Alternatives considered: a new read on each client taking a bond reader, with a nym-api type implementing `PinnedMixnetQueryClient` to carry the bonds (three public API additions and a type pretending to be a chain client, for one call site); passing the bonds rather than the parsed map (parses the same thousand keys three times).

### 4. A concrete producer, no sources

`AttestedSnapshotsProducer` holds the chain anchor, the three contract anchors as named fields, the signing keys, the knobs, the cache and the file, and runs the warm-up and the poll loop itself. Producing a height is: take the app hash the chain anchor trusts for H, read the identity map at H, run the three contracts' reads concurrently with `join` (so every failure at H is logged, not only the first), and when all succeeded read the performance auxiliary and sign each contract's `DigestSnapshot` into a `RetainedHeight`. The app hash comes first so that nothing is read when trust at H cannot be established. No cache guard is held across a chain read, the warm-up included: today's warm-up holds the write guard across its reads, and a producer reading its previous auxiliary through the cache under it would never return. The file is written after the warm-up and after a poll that retained a height, not on every poll as `CacheRefresher` does: every warm-up replaces the persisted `last_polled_height` with the tip, so a poll that retained nothing has nothing worth writing. With three fixed contracts the `AttestedSnapshotSource` trait and `VerifiedSet<S>` abstract over nothing, so they go; signing is one function generic over the record type. `CacheRefresher` is not reused: its one-provider, one-cache loop is a few lines here, and nothing subscribes to the attested refreshers' notifications or refresh requesters.

### 5. The chain anchor is the producer's

Only the producer needs the chain anchor now, so `support/chain_anchor/` moves under `support/attested_snapshots/`, the type is renamed `NyxChainAnchor` (the plain `ChainAnchor` is the anchor crate's trait, which it holds), with `NyxContractAnchor` for a contract's anchor over it, and loses its `Clone`, `start` builds it (proven or light client, from `[directory.debug]` and the checkpoint file) and the producer's constructor builds each contract's anchor over it. The sharing that matters, three contract anchors over one chain anchor so the app hash at H is established once and a light-client head has one writer, is the `Arc` inside it, unchanged.

### 6. The interval is read once at startup

As today. A change on chain is a coordinated governance action with restarts; following it at runtime would cost a query per poll or stale-interval reasoning for an event that may never happen.

### 7. Startup order

The producer's startup production runs where the performance producer's ran, before the node annotations' refresher starts, since in contract mode the annotations are built from the performance snapshots. The three contracts' reads at each height run concurrently, so the startup production takes about as long as one contract's took.

### 8. Testing

The step from three read results to a `RetainedHeight` or a failure is a function over the results, tested without a chain: one failure yields none, all successes yield the entry. The cache data's height logic (prune, settle lag, previous entry) keeps its tests over a `RetainedHeight` fixture, `AttestedSnapshot::paged_records` keeps the paging test and the accessors the latest and out-of-window tests, and one round-trip test persists a populated `RetainedHeight` (every performance auxiliary option set, every geolocation record shape) through the on-disk format. Untested by design: the producer's chain reads, its warm-up and poll loop, and the `run.rs` wiring, as the poll loop is untested today.

## Risks / Trade-offs

- [A fault in one contract stops every contract's new snapshots] → intended; each failure is logged naming its contract, and the retained window keeps serving until it ages out (three boundaries, about 25 minutes at the defaults).
- [The directory, which consumers already rely on, now depends on the newest contracts' health] → accepted with the ruling that every contract is required; a nym-api missing any contract address already fails startup.
- [The per-contract cache files are abandoned] → a startup refills the retained heights; the old files can be deleted by operators or left.
- [Startup production is all or nothing per height] → a contract failing at startup loses that height for all; the others are still tried, and polls fill forward.
- [An interval change needs restarts] → a coordinated governance action; stated in the PR.

## Migration Plan

Deploy is the nym-api release. The first start finds no `attested_snapshots` file and produces its retained heights from the chain, three heights at one bond read and three concurrent contract reads each; the old `directory`, `performance_snapshots` and `geolocation_snapshots` files are ignored. Under the light client (`trusted_rpc_node = false`) a prompt restart refills only the newest of them: the restored checkpoint sits just past the newest height the previous binary produced, the light client cannot verify below the height that checkpoint commits, so the older heights are logged and skipped and the window fills forward over two intervals. No configuration or operator action. Rollback is the previous binary, which ignores `attested_snapshots` and refills its own files.

## Open Questions

None.

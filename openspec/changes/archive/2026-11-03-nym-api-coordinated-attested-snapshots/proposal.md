## Why

nym-api attests three contracts (directory, performance, geolocation) through three refreshers that poll independently, each reading the chain tip, the mixnet bond set at every cadence height and its own contract there, and each failing alone. A contract can therefore miss a height the others hold, the trees' `/snapshot/latest` can disagree, and a performance or geolocation snapshot at a height the directory missed has no identity map to serve, since only the directory tree serves it. Independent failure was never a real requirement: every nym-api is required to attest every contract, so a fault in any one of them is a fault of the deployment, to surface rather than route around. Producing the contracts together as one entry per height removes the gaps, the triple bond read and most of the machinery in one move. Now, because the stack has not shipped: nothing outside relies on per-contract production, and the only shipped snapshot file is a cache that a startup refills.

## What Changes

- **One production for every attested contract.** A single poll loop per nym-api, at `[directory.debug].polling_interval`, produces the newest cadence height H for every contract together: the mixnet bond set at H read once and supplied to every contract's verified read, the three reads run concurrently, and the height retained only when every one succeeded, as one entry holding every contract's snapshot. A failure in any contract retains nothing for H, is logged naming the contract, and the next poll tries the newest cadence height again. The startup production follows the same rule height by height, oldest first.
- **One cache and one file.** The retained heights live in one cache persisted to one file, `attested_snapshots`, in place of the three per-contract caches and files (`directory`, `performance_snapshots`, `geolocation_snapshots`), which are no longer read or written; a nym-api starting on this release refills its retained heights from the chain. Each contract keeps its own route tree, responses and signed snapshot format; the handlers read their contract's snapshots through per-contract accessors over the one cache.
- **The producer reads through the shared whole-set read.** The producer calls `nym-contract-anchor`'s `verify_chain_read` directly for each contract, under that contract's anchor, rather than through the contract clients' reads, whose parsed or grouped results the producer discarded. The shared read's chain path takes its node-identity map from the caller (the clients pass the bond read at H they made themselves, the producer the map it read once), so the clients' behaviour is unchanged.
- **A concrete producer owning the chain anchor.** The per-contract source trait and sources, the generic per-contract provider, cache data, state and refresher go; one producer builds the chain anchor from `[directory.debug]` and the checkpoint file, keeps the chain id, holds each contract's anchor over it, and reads each contract through a method of its own.
- **Unchanged**: the routes and their responses, the `[directory.debug]` knobs, the cadence rule (interval read once at startup, as today), retention by height and the settle lag.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `directory-attestation-provider`: a new requirement that every attested contract is produced together at each cadence height, as one retained entry in one persisted file, with one bond read.
- `performance-attestation-provider`: the cadence and retained-window requirement, the produce-only-what-was-verified requirement and the persistence requirement follow the joint production and the one file.
- `geolocation-attestation-provider`: the same three requirements, likewise.
- `nym-api-startup`: the startup production is one production for every attested contract.
- `contract-trust-anchor`: the whole-set chain read takes its node-identity map from the caller; the clients' migration requirement no longer names nym-api's directory refresher, which is gone.

## Impact

**Code:**
- `common/nym-contract-anchor/src/whole_set.rs`: `verify_chain_read` takes the identity map as a value in place of its query client; its three callers (`nym-directory-client`, `nym-performance-client`, `nym-geolocation-client`) pass their own bond read, with no behaviour change.
- `nym-api/src/support/attested_snapshots/`: the per-contract provider, cache data, update, retained entry, source trait and `start_refresher` replaced by `RetainedHeight`, one concrete cache data, one producer with its warm-up and poll loop, and the per-contract view the handlers read through; the chain anchor moves in from `support/chain_anchor/`.
- `common/nym-contract-anchor/src/anchor/contract.rs`: `ContractAnchor::contract()`, the address a signed snapshot names.
- `nym-api/src/{directory/attestation,node_performance/attestation,geolocation}/source.rs`: removed, their reads becoming the producer's `verified_directory`, `verified_performance` and `verified_geolocation`, each through `verify_chain_read`.
- `nym-api/src/support/caching/`: `CacheRefresher`'s update-function path removed, the attested refreshers having been its last user.
- `nym-api/src/support/cli/run.rs`: one start in place of three and the chain anchor's; `ContractSnapshots` is the one state, with per-contract accessors the handlers read through; the annotation refresher's performance reads and the v3 performance routes read its whole entry.

**Behaviour:** a fault in any attested contract stops every contract's new snapshots on that nym-api, logged per contract, until it clears. Every retained height holds every contract, and every tree's latest is the same height. Per height, one bond read replaces three.

**Deployment:** no configuration or route change. The three per-contract cache files are ignored and can be deleted; the first start refills the retained heights (three heights, each one bond read and three concurrent contract reads; under the light client, after a prompt restart, only the newest, the window then filling forward). Rollback is the previous binary, which refills its own files.

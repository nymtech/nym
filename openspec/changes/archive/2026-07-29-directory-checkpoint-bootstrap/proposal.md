## Why

`LightClientAnchor` gives trustless directory retrieval, but only from a `Checkpoint` the caller must supply out-of-band, and nothing in the system produces or refreshes one. As a result the nym-api directory producer's light-client path is a dead `bail!("unimplemented external checkpoint retrieval")` (`nym-api/src/directory/cache/mod.rs`), production clients have no way to obtain a checkpoint, and the anchor cannot be deployed end-to-end. This change adds the missing weak-subjectivity bootstrap: a self-authenticating, root-signed checkpoint that can be hardcoded and republished anywhere, plus local persistence so long-running nodes refresh themselves without ever needing a fresh checkpoint.

## What Changes

- **New root-signed checkpoint datum**: a `SignedCheckpoint` wrapping the existing `Checkpoint` (full signed header + both validator sets) plus an advisory `created_at`, signed by a single hardcoded root key. Self-authenticating (trust = root signature), so it can be published in any untrusted channel. **No expiry field** - staleness is derived at load from the checkpoint's own signed block time plus the trusting period.
- **Single root key by reuse**: the existing upgrade-mode attester key is generalised into the directory checkpoint root. The shared network-default constant/env var is renamed to a domain-neutral name with a **backward-compatible env alias** so existing `.env` files keep working. **BREAKING (operational)**: rotating this key now rotates both upgrade-mode attestation and directory checkpoints.
- **Domain separation**: the checkpoint signing payload carries a distinct domain tag so a root signature over a checkpoint can never validate as an upgrade-mode attestation (or any other root-signed payload), and vice versa.
- **Checkpoint providers + loader**: a `CheckpointProvider` abstraction tried in priority order - locally stored verified head, then hardcoded-constant, then an HTTPS `.wellknown` fallback (mirroring `UPGRADE_MODE_ATTESTATION_URL`) - and a loader that verifies the root signature, checks staleness, and constructs a `LightClientAnchor` from the first valid source. DNS/bulletin-board/node-served sources are deferred.
- **Verified-head persistence**: a `CheckpointStore` seam lets a `LightClientAnchor` persist its light-client-verified head and reseed from the newest valid of {baked seed, persisted head}, so a node offline for less than the trusting period recovers with no fresh checkpoint.
- **Trusting period raised to 18 days** (from 14), below nyx's 21-day unbonding period, for a between-release refresh safety net.
- **Offline minting dev-tool**: a maintainer binary that fetches a checkpoint from a trusted RPC, self-verifies it, signs it with the root key, and regenerates the hardcoded constant file for commit at release.
- **Producer wiring**: the nym-api producer's light-client source anchor is bootstrapped from the checkpoint layer (replacing the `bail!`) and persists its verified head.

## Capabilities

### New Capabilities
- `directory-checkpoint-bootstrap`: the root-signed checkpoint datum format and domain-separated signing payload, verification against the single hardcoded root key, the checkpoint provider abstraction (hardcoded + HTTPS well-known) and loader, load-time staleness derivation, and the offline minting tool.

### Modified Capabilities
- `tendermint-light-client-anchor`: add verified-head persistence via a `CheckpointStore` (reseed from the newest valid of seed vs persisted head; ignore stale/corrupt); pin the production trusting period to 18 days with an invariant that it stay below the chain unbonding period, read from a single source shared with the loader's staleness check.
- `directory-retrieval-client`: the production `LightClientAnchor` path is bootstrapped from the checkpoint layer instead of requiring a caller-supplied checkpoint.
- `directory-attestation-provider`: the producer's (already-permitted) light-client source anchor is obtained from the checkpoint layer, replacing the unimplemented checkpoint-retrieval path, and the producer persists its verified head.

## Impact

- **Code**: new datum type + verify + `CheckpointProvider`/loader + `CheckpointStore` + store-aware `LightClientAnchor` constructor in `nym-directory-client` (reusing `nym-directory-attestation`'s signing-payload helpers); producer wiring in `nym-api/src/directory/cache/`; root-key constant/env rename + alias in `common/network-defaults` (with a dedicated regenerated checkpoint constant file); accessor update in the upgrade-mode consumers (`gateway_tasks.rs`, `old_config_v12.rs`, credential-proxy). nym-node upgrade-mode CLI/env override knobs are left unchanged.
- **New dev binary** for offline checkpoint minting (maintainer-only, not the user-facing nym-cli).
- **Config/deploy**: new canonical env var (with old-name fallback) and a new HTTPS `.wellknown/directory/checkpoint.json` published by the root operator; trusting-period change to 18 days.
- **Deferred (single trailing ops task)**: signing and committing the real mainnet checkpoint constant + publishing the well-known file, gated only on root-key access. All code and tests land beforehand using a test root key and existing `MockRpcClient` nyx fixtures.

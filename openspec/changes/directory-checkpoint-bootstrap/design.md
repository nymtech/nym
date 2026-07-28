## Context

`LightClientAnchor` (capability `tendermint-light-client-anchor`) verifies a chain `app_hash` via the Tendermint light-client skipping protocol, but only starting from a `Checkpoint { height, signed_header, validators, next_validators }` the caller must supply out-of-band. Nothing produces or refreshes that checkpoint today:

- The nym-api directory producer (`nym-api/src/directory/cache/mod.rs`) selects a source anchor and, for the non-`trusted_rpc_node` branch, hits `bail!("unimplemented external checkpoint retrieval")`.
- The retrieval client's "Light-client anchor for production use" requirement assumes a checkpoint exists but never says where it comes from.

This change is the weak-subjectivity bootstrap that fills that gap. It reuses infrastructure already in the tree: `fetch_checkpoint` (`common/nym-directory-client/src/anchor/mod.rs`) builds a `Checkpoint` from `commit`/`validators`; `nym-directory-attestation` is the signer-agnostic protocol crate that already domain-separates signed payloads; `MockRpcClient` (behind the `mocks` feature) plus real nyx fixtures (heights 24499896+) already drive the anchor tests; and `common/network-defaults` already carries the upgrade-mode attester key and a `.wellknown` attestation URL as the pattern to mirror.

Constraints: `common/network-defaults` is deliberately dependency-starved (imported by the ecash contract) and stores keys as bs58 strings parsed downstream. nyx has a 21-day unbonding period. `Checkpoint` already derives `Serialize`/`Deserialize`.

## Goals / Non-Goals

**Goals:**
- Supply a `Checkpoint` to `LightClientAnchor` from a self-authenticating, root-signed datum that can be hardcoded or republished in any untrusted channel.
- Unblock the nym-api producer's light-client source anchor (remove the `bail!`).
- Let long-running nodes recover across restarts without a fresh checkpoint, as long as downtime is under the trusting period.
- Land all code and tests without access to the real root key; gate only the production data artifact on the key.

**Non-Goals:**
- An online/automated re-signing service. Refresh is a maintainer-run offline ceremony at release cadence.
- DNS-TXT, bulletin-board, or node/api-served checkpoint sources (deferred).
- An operator early-retire mechanism for checkpoints (no explicit expiry field; deferred).
- Changing the attested (`AttestedTrustAnchor`) or proven (`ProvenTrustAnchor`) trust paths.

## Decisions

### Single root key, reusing the upgrade-mode attester key
Reuse the existing upgrade-mode attester key as the directory checkpoint root rather than minting a second hardcoded root. It is already a per-network, operationally-managed root of trust present in every `.env`; a second root would double the key-management surface with no security gain. The shared network-default constant/env-var is renamed to a domain-neutral name (e.g. `ROOT_ATTESTER_*`) with a backward-compatible fallback to the old `UPGRADE_MODE_ATTESTER_ED25519_PUBKEY` env name, registered in both `setup_env` and `export_to_env_if_not_set` for a deprecation window. The nym-node upgrade-mode override knobs (`--upgrade-mode-attester-public-key`, `NYMNODE_UPGRADE_MODE_ATTESTER_PUBKEY`) are left unchanged - only the shared default is generalised.
- *Alternative (rejected)*: a dedicated new checkpoint root key - more key management, no benefit.
- *Consequence*: rotating this key rotates both subsystems; called out as an operational note.

### Domain separation is the one hard crypto requirement
The checkpoint signing payload carries a distinct domain tag, mirroring how `UpgradeModeAttestationContent` already tags `"upgrade_mode"`. Because the two payloads produce different signed bytes and each verifier deserializes only its own struct, a root signature over a checkpoint can never validate as an upgrade-mode attestation (or a snapshot/subset/node-entry signature) and vice versa. This makes key reuse safe.

### No expiry field; derive staleness from the signed block time
The `Checkpoint` already embeds a signed block timestamp, and the trusting period is compiled in, so the loader derives staleness as `header_time + trusting_period < now` and fails loud at load. An explicit signed `expiry` would add an operator obligation and buy only early-retire, which is out of scope. We keep an advisory, authenticated `created_at` (rfc3339, inside the signed content, using the existing `time::serde::rfc3339` pattern) purely so humans/tooling can see when a datum was minted; it is documented as not a validity bound.
- *Alternative (rejected)*: signed `expiry` clamped to the trusting period - redundant with the derived check for v1.

### Trusting period = 18 days
Raised from 14 to give a between-release refresh safety net (biweekly releases). It stays below nyx's 21-day unbonding period with a 3-day margin, preserving the weak-subjectivity guarantee (a client stops trusting the checkpoint before any validator bonded at checkpoint height could unbond into non-slashability). The value lives in one place (`nyx_default_options`) and the loader's staleness check reads that same constant so they cannot drift.
- *Alternative (rejected)*: keep 14 (more margin but no between-release net); go higher (erodes the margin below unbonding).

### Datum, verify, providers, and loader all live in `nym-directory-client`
`SignedCheckpoint { checkpoint: Checkpoint, created_at, root_signature }` embeds the whole checkpoint (no minimal/re-expand form, so no extra RPC at load). The datum type, its verification, the `CheckpointProvider` abstraction, its impls, and the loader all live in `nym-directory-client`, because `Checkpoint` wraps tendermint `SignedHeader`/`ValidatorSet` types that already live there and that `nym-directory-attestation` deliberately avoids (that crate depends only on `serde`/`nym-crypto`/`nym-lthash`). The wrapper reuses `nym-directory-attestation`'s existing domain-tag + length-prefix signing-payload helpers (already a dependency of `nym-directory-client`), so the checkpoint stays consistent with the crate's other signed payloads without forcing tendermint deps into it. `network-defaults` stays dependency-starved, holding the bs58 root key and the serialized datum as strings parsed downstream via `nym-crypto`.

### Canonical signing payload via protobuf, no hand-crafting
The root signs `domain_tag || chain_id || height || sha256(proto_encode(checkpoint))`, where the fixed-width wrapper follows `nym-directory-attestation`'s existing convention and `proto_encode` is Tendermint's own `Protobuf` encoding of the checkpoint (its native canonical form, via `tendermint-proto`/prost already transitively present through the light-client stack). This avoids hand-serializing a nested header + validator sets and stays consistent with the protobuf choice for node->contract directory data. Determinism rests on using one encoder version on both signer and verifier - which we fully control, and these types contain no maps, so field ordering is stable.
- *Alternative (rejected for v1)*: sign over Tendermint's already-network-canonical `block_hash` + `validators_hash` + `next_validators_hash` instead of re-encoding - sidesteps the determinism caveat but adds hash-matching verify logic; proto-encode-and-hash is less code.
- *Alternative (rejected)*: mirror upgrade-mode's `serde_json::to_string` signing - fragile JSON canonicalization over complex tendermint types.

### Sources are an ordered provider chain: stored -> hardcoded -> HTTPS
Because the datum is self-authenticating, a source needs only availability, not trust. The loader tries providers in strict priority order and uses the first that yields a valid (non-stale, and for signed sources sig-verified) checkpoint:
1. **stored** - the locally persisted, previously light-client-verified head (freshest and free: no network, trusted at filesystem-integrity level, staleness-checked);
2. **hardcoded** - the compiled-in signed datum from a dedicated regenerated `network-defaults` file (offline seed for a fresh boot);
3. **HTTPS** - a signed datum fetched from a configurable, env-overridable `.wellknown/directory/checkpoint.json` URL mirroring `UPGRADE_MODE_ATTESTATION_URL` (last resort when the seed has aged out).

DNS/bulletin-board/node-served are deferred.

### Verified-head persistence: read side is a provider, write side is on the anchor
The head `LightClientAnchor` advances to is itself a valid `Checkpoint`, transitively trusted from the seed, so persisting it lets a node recover without a fresh checkpoint. This splits into two halves: the **read** side is the top-priority `stored` provider in the loader chain above; the **write** side is an injected `CheckpointStore` on the anchor that saves the advanced head once per producer refresher tick. Stale (outside trusting period) or corrupt persisted state is ignored by the `stored` provider in favour of the next source. The store is a collaborator of the light-client anchor only.
- *Transient imperfection*: if a new binary ships a hardcoded seed at a higher height than an old stored head, the loader starts from the (valid but lower) stored head; it self-corrects as soon as the anchor advances and overwrites the store.
- *Alternative (rejected)*: `persist`/`restore` methods on the shared `DirectoryTrustAnchor` trait - would force no-op defaults on the proven and attested anchors (a smell); the store belongs to the light-client anchor alone.

### Offline minting as a maintainer dev-binary
A dedicated binary (`tools/internal/nyx-checkpoint-updater`, not the user-facing nym-cli, to isolate root-key handling) fetches a checkpoint from a trusted `--rpc` (defaulting to `latest - 2` so the self-verify hop's `height + 2` block is committed), signs it with the root key, self-verifies it, and regenerates the compiled-in datum. Self-verify advances the minted checkpoint exactly one light-client hop via `verify_checkpoint_advances_one_hop` (factored out of `LightClientAnchor::verify_hop` so it needs neither an anchor instance nor a dummy directory contract address), failing loud on incoherence before anything is written. The regenerated artifact is a `SignedCheckpoint` JSON datum written wholesale to `directory_checkpoint.json`, which a static `directory_checkpoint.rs` wrapper embeds via `include_str!` - so the tool never rewrites Rust source and emits no header comment; the provenance a header comment would carry (`created_at` + height) is instead authenticated inside the signed datum, and the source `--rpc` belongs in the commit message. The private key is a `clap` argument with `env = "..."` so tests pass an explicit value and production reads it from the environment. Output is deterministic given pinned inputs (ed25519 RFC 8032 + a `--minted-at` override). The datum file is located by walking up from the current directory (or an explicit `--repo-root`/`--out`), not a compile-time path, so the binary works when compiled in one place and run from another.

## Risks / Trade-offs

- **Hardcoded seed ages out of the trusting period** → it is a bootstrap seed, not a standalone answer: fresh binaries ship a fresh seed (minted at release), long-running nodes self-advance via persistence, and cold clients on stale binaries fall back to the HTTPS well-known file or a redeploy (correct fail-closed behaviour).
- **18 days consumes most of the margin below unbonding** → single-source-of-truth constant plus a spec invariant that the trusting period stay comfortably below unbonding; revisit if nyx shortens unbonding.
- **Key reuse across two subsystems** → strict domain separation on the signed payload (mandatory), and an explicit operational note that rotation affects both.
- **Minting trusts its RPC at mint time (garbage-in = validly-signed-garbage)** → the tool self-verifies before writing and operators point it at their own node; optional multi-RPC cross-check is a later nicety.
- **Persisted head trusts local disk** → same trust boundary as the node's identity key/config; corrupt/unparseable state is ignored in favour of the seed.

## Migration Plan

1. Land all code, the minting tool, and tests against a **test** root key (ed25519 is key-agnostic; existing `MockRpcClient` nyx fixtures drive verification). This includes the network-defaults rename with the backward-compatible env alias, so existing `.env` files keep working.
2. **Trailing ops task (gated on root-key access)**: run the minting tool with the real mainnet key to regenerate and commit the hardcoded constant file, and publish the `.wellknown/directory/checkpoint.json` file. Non-mainnet networks can ship dev-key-signed checkpoints immediately.
3. Rollback: the producer retains the proven-RPC anchor path (`trusted_rpc_node`), and the retrieval client retains `ProvenTrustAnchor`/`AttestedTrustAnchor`, so the light-client bootstrap can be disabled without losing directory functionality.

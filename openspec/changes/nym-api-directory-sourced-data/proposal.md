## Why

Two nym-api instances asked the same question at the same moment can give different answers, because each builds its node data from its own HTTP polling of every nym-node (the describe cache, every 4500 s) and from its own unpinned reads of the mixnet and node families contracts (every 150 s and 600 s). Nodes now publish to the directory contract everything the describe cache used to fetch (the last three fields landed in #7399), and nym-api already reads the directory, performance and geolocation contracts at one shared cadence height, so every instance can serve the same data by reading everything at that height and nothing from the nodes.

## What Changes

- nym-api stops querying nym-nodes. The describe cache, its refresher, the nym-api key rotation controller that re-triggered it and the per-node refresh request path are removed.
- Topology routes (skimmed and semi-skimmed, every version), described routes, the noise-key route and `POST /v1/unstable/nym-nodes/by-addresses` are built from the verified directory at the latest settled snapshot height. Response types do not change (JSON, YAML, bincode); only the source of the values does.
- The data the mixnet contract cache held (node details, rewarded set, interval, reward parameters, key rotation state, config score parameters and version history, contract state) and the node families data are read inside the attested snapshot production at the snapshot height, held with that height and served from the latest settled one. The 150 s and 600 s refreshers, their cache files and their config sections are removed.
- **BREAKING** A node is listed only when it is bonded at the snapshot height, every one of its directory entries verifies, its node information decodes, it publishes a sphinx key for the snapshot's key rotation, and its announced addresses are public. A node that starts unbonding leaves at the first snapshot after its unbond transaction, not at the epoch end.
- **BREAKING** Values the directory does not hold are served as fixed placeholders: `build_information` keeps only the real `build_version` (binary name "nym-node", build timestamp the Unix epoch, the rest "unknown"), `uses_exit_policy` is `true`, `last_polled` is the snapshot's block time. `exit_nr` and `exit_ipr` both follow the node's exit mode alone.
- **BREAKING** nym-api no longer reads legacy mixnode or gateway bonds. `GET /v1/legacy/mixnodes` and `GET /v1/legacy/gateways` are kept with their response types, marked deprecated, and always answer an empty list.
- **BREAKING** Epoch, rewarded set, key rotation, bonded-node, config-score-detail and circulating-supply routes follow the chain with a delay of up to one snapshot interval plus the settle lag (about 110 blocks, 10 minutes on mainnet) instead of 150 s.
- **BREAKING** `POST /v1/nym-nodes/refresh-described` answers 410 Gone with a message telling the operator to upgrade nym-node. Both `/v1/utility` routes are removed, with the `utility_routes_bearer` config field and the `--utility-routes-bearer` flag.
- **BREAKING** The override of the public-address rule is renamed to say it is unsafe: `[directory.debug] unsafe_allow_illegal_ips` and the hidden `--unsafe-allow-illegal-ips` flag replace `[describe_cache.debug] allow_illegal_ips` and `--allow-illegal-ips`.
- The `Expires` header on active-only responses names the moment the first snapshot taken after the epoch is due to change becomes served, computed from the snapshot's block time so every instance sends the same value.
- Average node age in node families becomes blocks since bonding times a fixed block time, with no block-timestamp lookups.
- nym-node reads key rotation information from the chain first and falls back to nym-api, at startup, in its running key rotation controller and in the rotation-id helper. Its stuck-epoch check is fixed and kept in the running controller only, the clock-based rotation-id check is removed, and it no longer posts to `refresh-described`.
- The part of the epoch driver's node rule that nym-api shares (every entry verifies, node information decodes) moves into shared code. The driver's behaviour does not change.

## Capabilities

### New Capabilities
- `nym-api-directory-sourced-nodes`: how nym-api builds its node-facing routes from the directory at the latest settled snapshot: which nodes are listed, where each served value comes from, the placeholders, the public-address rule and its override, the `Expires` rule, the address lookup, the behaviour with no settled snapshot, and the retired `refresh-described` and utility routes.
- `nym-api-snapshot-chain-state`: the mixnet contract state and node families data read beside each snapshot at its height, retained with it and served from the latest settled one, including the node age rule and the account lookup exception.
- `nym-node-key-rotation-source`: where nym-node reads key rotation information from (chain first, nym-api as fallback), how it treats a stuck epoch, and that it no longer asks nym-api to refresh its description.

### Modified Capabilities
- `nym-api-startup`: the startup wait covers only the settled snapshot, since the mixnet contract, describe and node families caches no longer exist; the retry rule for those caches' first build is removed.

## Impact

- **nym-api**: `node_describe_cache/`, `key_rotation/`, the refreshers under `mixnet_contract_cache/` and `node_families/cache/` are removed; `support/attested_snapshots/` gains the mixnet and node families reads and a per-height node view; about 25 readers of the contract cache in about 12 files are re-pointed at the latest settled snapshot; `nym_nodes`, `unstable_routes` and `utility_routes` handlers change source or go; `support/config` loses three sections and one field and gains one.
- **validator-client**: new height-pinned queries on the mixnet contract (rewarded set, interval, key rotation state, reward parameters, contract state, config score parameters, version history) and on the node families contract (families, members, pending invitations).
- **nym-directory-client / nym-epoch-driver**: the shared part of the node rule moves from the driver binary into `nym-directory-client`; the driver calls it.
- **nym-node**: `node/key_rotation/controller.rs`, `node/helpers.rs`, `node/mod.rs`, `node/nym_apis_client.rs`, `node/nyx_client.rs`.
- **nym-api-requests**: `KeyRotationDetails::is_epoch_stuck` fixed, `expected_current_rotation_id` removed (public API).
- **Operators**: nodes that do not yet publish the #7399 fields are served with no noise key, terms shown as not accepted and no Lewes Protocol details. A deployment passing `--utility-routes-bearer` or `--allow-illegal-ips` fails to start; removed config sections are ignored silently and their cache files are left on disk.
- **Clients behind HTTP caches**: active-only responses carry the new `Expires` value.
- **Release**: no runtime switch. The change ships in a release a few releases after nodes started publishing the #7399 fields; merging it is the gate. The localnet orchestrator, already broken, still calls the removed routes and is not fixed here.

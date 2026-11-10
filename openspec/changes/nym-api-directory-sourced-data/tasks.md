## 1. Height-pinned queries (validator-client)

- [ ] 1.1 Add the single-value reads to `PinnedMixnetQueryClient`: rewarded set metadata and each role's assignment (with a `get_rewarded_set_at_height` helper mirroring the unpinned one), current interval details, key rotation state, reward parameters, contract state, config score parameters
- [ ] 1.2 Add the paged reads to `PinnedMixnetQueryClient` through `PinnedPage` and `collect_pages_at_height`, each with its `get_all_*_at_height` helper: legacy mixnodes, legacy gateways, gateway ids, nym-node version history
- [ ] 1.3 Add a pinned query trait for the node families contract with the paged reads of families, family members and pending invitations and their `get_all_*_at_height` helpers

## 2. Shared node rule

- [ ] 2.1 Add a method on `DirectoryNode` (`common/nym-directory-client/src/verify.rs`) that returns a decoded payload only when every entry of the node verifies, with unit tests for an unverified node, a missing entry and an undecodable entry
- [ ] 2.2 Make `node_roles` in `nym-epoch-driver/src/network_view.rs` use it, leaving the driver's role derivation, unbonding filter and tests as they are

## 3. Snapshot production holds mixnet and families data

- [ ] 3.1 Read the node details at `H` in `produce_height` in place of the bond read, deriving the identity map from them
- [ ] 3.2 Read the rest of the mixnet contract state at `H` (task 1.1 and 1.2 queries) and hold it in `RetainedHeight`, moving `MixnetContractCacheData` and its accessors out of the refreshed cache
- [ ] 3.3 Read the node families contract at `H`, join it with the node details of `H` into `NodeFamiliesCacheData`, and hold it in `RetainedHeight`
- [ ] 3.4 Compute node age as blocks between the bonding height and `H` times one fixed block-time constant (5.687 s), removing the block-timestamp lookup and estimation code; unit test with the spec's example
- [ ] 3.5 Make a failed mixnet or families read retain nothing for `H` and log what failed; tests for each
- [ ] 3.6 Update the `RetainedHeight` fixtures (`empty_retained_height`, `populated_entry`, the `populated_cache()` replacement) and the on-disk round-trip test; confirm a file in the previous shape loads as an empty cache

## 4. Per-height node view

- [ ] 4.1 Write the conversion from one node's verified directory entries, node details, key rotation id and block time to the described type: directory values, #7399 fields through the existing reader conversions, exit flags from the exit mode, the `build_information`, `uses_exit_policy` and `last_polled` placeholders, no Lewes Protocol details without the node's signature; unit tests for each scenario of the "Served values" requirement
- [ ] 4.2 Write the listing rule: the shared rule of task 2.1, a sphinx key for the rotation in force at `H`, and public addresses through `is_global_ip` unless `unsafe_allow_illegal_ips`; unit tests for each scenario of the two listing requirements
- [ ] 4.3 Build the view (listed nodes in ascending node id order, plus the address index) when a height is produced and when retained heights are loaded from the file, without writing it to the file

## 5. Routes read the latest settled snapshot

- [ ] 5.1 Give `AppState` accessors for the mixnet state, the families data and the node view of the latest settled snapshot, answering 503 when none has settled; remove the contract cache, describe cache and families cache handles
- [ ] 5.2 Re-point every reader of the mixnet contract cache: epoch, reward parameters, key rotation information, rewarded set, bonded nodes, stake saturation, legacy mixnodes and gateways, config score details, circulating supply, annotations, the account collector
- [ ] 5.3 Re-point the node families routes
- [ ] 5.4 Build the skimmed and semi-skimmed routes of every version, the described routes and the noise-key route from the node view, with `refreshed_at` the block time of the height
- [ ] 5.5 Compute `Expires` on active-only responses as the design's D7 states; unit tests with the spec's mid-epoch and overdue examples
- [ ] 5.6 Answer `by-addresses` from the view's address index
- [ ] 5.7 Make `refresh-described` answer 410 Gone with the upgrade message, removing the forced-refresh state and its request verification
- [ ] 5.8 Remove the `/v1/utility` routes, the `utility_routes_bearer` config field, the `--utility-routes-bearer` flag and its environment variable

## 6. Removal and configuration

- [ ] 6.1 Delete `node_describe_cache/`, nym-api's `key_rotation/` controller, the mixnet contract cache refresher and the node families refresher, with their wiring in `support/cli/run.rs`
- [ ] 6.2 Reduce `StartupCaches` to the settled snapshot and update its tests
- [ ] 6.3 Remove the `[describe_cache]`, `[mixnet_contract_cache]` and `[node_families_cache]` config structs, defaults and template entries, keeping old config files loadable
- [ ] 6.4 Add `[directory.debug] unsafe_allow_illegal_ips` to the config and its template, and rename the hidden flag to `--unsafe-allow-illegal-ips`
- [ ] 6.5 Remove dependencies and code left unused by the removals

## 7. nym-node

- [ ] 7.1 Add the chain read of the key rotation details (interval and key rotation state) to the shared `NyxClient`
- [ ] 7.2 Make `setup_key_rotation` and the controller's `try_get_key_rotation_info` read the chain first and fall back to nym-api, logging the fallback
- [ ] 7.3 Make `get_current_rotation_id` take the shared `NyxClient` instead of chain URLs: chain first, nym-api as fallback, no failure on a stuck epoch; build the client before the call in `NymNode::new`, and build one for the read in `NymNode::initialise`, the sphinx key reset command and the v9 config migration
- [ ] 7.4 Fix `KeyRotationDetails::is_epoch_stuck` with unit tests for the spec's two controller scenarios, and delete `expected_current_rotation_id` with the controller's mismatch branch
- [ ] 7.5 Remove the startup refresh request (`try_refresh_remote_nym_api_cache`, `broadcast_force_refresh`)

## 8. Verification and wrap-up

- [ ] 8.1 Run `cargo fmt`, `cargo check --workspace` and the tests of the touched crates (validator-client, nym-directory-client, nym-epoch-driver, nym-api, nym-api-requests, nym-node)
- [ ] 8.2 Run the build locally against mainnet and diff its described and skimmed responses against the live nym-api with a script kept in `~/workspace/claude-notes/directory-topology-issues/`, recording the result there; rerun once nodes publishing the #7399 fields are deployed
- [ ] 8.3 Draft the PR description listing every route whose behaviour changes and the operator-facing breaks (removed flags, renamed override, ignored config sections)
- [ ] 8.4 Archive the change inside the PR and check `git diff openspec/specs`

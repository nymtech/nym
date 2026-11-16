## Context

nym-api builds its node-facing responses from three sources, two of which are private to each instance:

- The describe cache polls every nym-node over HTTP every 4500 s. What an instance holds depends on which nodes it reached and when, so the node set, keys, addresses and declared roles differ between instances.
- The mixnet contract cache refreshes every 150 s at whatever height the RPC is at, and the node families cache every 600 s. Around an epoch change instances disagree on roles, epoch id and key rotation id.
- The attested snapshots (directory, performance, geolocation) are read at one shared cadence height: every snapshot interval (100 blocks), with "latest" advertised after a settle lag (10 blocks), three heights retained, all contracts kept or dropped together.

Nodes publish to the directory everything the describe cache fetched, the last three fields (noise key, operator terms, the node's own Lewes Protocol signature) since #7399. The epoch driver already selects the rewarded set from the directory at a cadence height. This change makes the third source the only one.

Constraints:

- Response types stay exactly as they are in JSON, YAML and bincode (bincode is positional). Only the source of the values changes.
- There is no per-node fallback to the describe request: it would bring divergence back for those nodes.
- The change stacks on the epoch driver stack (#7398 and the PRs under it) and contains #7399.

## Goals / Non-Goals

**Goals:**

- Every nym-api instance serves the same node, epoch and family data once it has settled the same snapshot height.
- nym-api makes no request to any nym-node.
- nym-node no longer depends on nym-api for key rotation while its chain connection works.

**Non-Goals:**

- Ecash and signer data, `/v1/api-status`, chain status and contract details stay per-instance.
- Account lookups stay in their simplest form: live balance and delegation reads priced with the node reward state of the latest snapshot. They are not identical across instances.
- The frozen monitor-history routes and the account route moving to the node status API, nym-nodes using their own directory copy for the routing filter, the localnet orchestrator, and the network monitor lowering the config score of nodes announcing non-public addresses are follow-ups.
- The epoch driver's selection does not change.

## Decisions

### D1. Everything nym-api serves about nodes and the epoch comes from one retained height

The snapshot production at height H additionally reads, every query pinned at H:

- the node details of every bonded node (this replaces the bond read: the identity map every contract is verified with is derived from it, as the epoch driver already does);
- the rest of what the mixnet contract cache held, except the legacy mixnode and gateway bonds (D10): rewarded set, interval, key rotation state, reward parameters, contract state, config score parameters and version history;
- the node families contract: families, members, pending invitations.

They are held in the same retained entry as the three attested snapshots, are never signed and never served under the attestation route trees, and a height is retained only when every one of these reads succeeded, like the performance auxiliary reads today. Routes read the latest settled entry.

Why not keep a second, faster cache for epoch data: two cadences is what makes instances disagree, and a route combining the rewarded set of one height with the directory of another would again need reconciling. Cost accepted: epoch changes and new bonds are visible after up to one interval plus the settle lag (about 110 blocks) instead of 150 s.

The data is kept for all three retained heights, not only the latest. The served height (latest settled) and the newest produced height (waiting out the settle lag) are different heights for about 10 blocks of every 100, so at least two must be held, and the retained entry is already the unit that is kept and evicted.

### D2. Height-pinned queries are added where they are missing

`PinnedMixnetQueryClient` has node details and bonds only. It gains the single-value reads (rewarded set metadata and role assignments, interval, key rotation state, reward parameters, contract state, config score parameters) and the paged version history through the existing `PinnedPage` and `collect_pages_at_height`. A pinned trait is added for the node families contract with its three paged reads. The helpers live in validator-client next to the unpinned ones.

### D3. A per-height node view is derived once, not per request

For each retained height nym-api derives a node view from the directory records that height already verified and holds, the node details and the key rotation id at that height: the listed nodes in ascending node id order, each with the full description the described routes serve, from which the skimmed and semi-skimmed forms, the noise keys and the address index are taken.

It is derived when a height is produced and again when retained heights are loaded from the file, and is not written to the file. Deriving per request would decode several thousand records for every call. Persisting it would let a release that changes the mapping keep serving the previous release's mapping from disk until those heights age out.

### D4. One node rule, shared where it is shared

The part common to the epoch driver and nym-api (a node counts only when every one of its directory entries verifies against its bonded identity and its node information decodes) moves into `DirectoryNode::payload` in `nym-directory-client`, which today decodes an entry whether or not the node's entries verify. It returns `Result<T, PayloadError>`: `Unverified` unless every entry of the node verifies, `Missing` when the node publishes no entry under the label, `Undecodable` otherwise. `Missing` stays explicit because nym-node writes no entry for a payload whose encoding is empty, so a reader decides per payload whether a missing entry means "none" (service providers) or rules the node out (node information); decoding empty bytes instead would give a default `NodeInformation` that passes. The method is replaced rather than joined by a gated sibling: its only caller, the driver, already checks the flag first, and `known_labels` stays public for a reader that wants unverified bytes. Each side adds its own conditions:

- the driver keeps its role derivation from the modes and its unbonding filter, unchanged;
- nym-api additionally requires a sphinx key for the key rotation current at the height and public addresses (D6).

The driver is not given the two extra conditions. That would change reward-affecting selection, need the next epoch's rotation id, make one late key submission cost a node its role for a whole epoch, and add the address override to the driver. Cost accepted: a node can hold a role and be missing from the served topology until its key appears, or for the whole epoch with a non-public address.

A node that starts unbonding has its directory entries removed by the unbond transaction itself, so it has no node information at the next snapshot and leaves the listing there. Keeping it until the epoch end would mean carrying records over from an older height.

### D5. Served values: directory first, fixed placeholders for what it does not hold

- Identity, node id and contract node type come from the node details; addresses, hostname, ports, location, modes and `build_version` from the node information; sphinx keys from the sphinx keys entry (current rotation, plus the pre-announced one when present); service provider addresses, wireguard and Lewes Protocol details from their entries; the noise key, terms acceptance and the node's own Lewes Protocol signature from the #7399 fields, converted with the reader conversions that PR added.
- A node that does not publish a #7399 field is served without it: no noise key, terms not accepted, no Lewes Protocol details. Unsigned Lewes Protocol content is never served.
- `exit_nr` and `exit_ipr` are both the node's exit mode. Today both equal entry mode and exit mode together, so they never differ; following the exit mode alone matches the role the driver assigns. The only nodes whose flags change are those running exit without entry.
- `build_information`: `build_version` from the directory (six readers parse it as semver), `binary_name` "nym-node", `build_timestamp` the Unix epoch, every other field "unknown". `network_requester.uses_exit_policy` is `true`. `last_polled` and `refreshed_at` are the block time of the height.

### D6. The public-address rule stays in nym-api, its override is marked unsafe

A node announcing any address that is not publicly routable is not listed. The check is `nym_bin_common::ip_check::is_global_ip` on every directory address, the rule nym-node applies to its own addresses at startup unless run with `--local`. It is stricter than the describe cache's `HostInformation::check_ips`, which refused only unspecified, loopback and multicast addresses. The override exists for local test networks only, so it is named `[directory.debug] unsafe_allow_illegal_ips` with the hidden flag `--unsafe-allow-illegal-ips`.

### D7. `Expires` names the first snapshot after the epoch is due to change

On the routes that carry the header today (active-only responses), with `t` the block time of the served height, `Δ` the snapshot interval, `L` the settle lag and `b` the fixed block time:

1. `due` = the current epoch's end plus the existing 2 minute transition margin.
2. `k` = the smallest integer of at least 1 with `t + k·Δ·b >= due`.
3. `Expires` = `t + (k·Δ + L)·b`.

When the epoch is already overdue at the served height this is the next snapshot. `Δ` is the interval of the cadence the served height was taken on, held with the height; `L` is the instance's settle lag. Every instance computes the same value from the same height. Expiring at every snapshot was rejected: the active set only changes with the epoch, so it would give up most of the caching. Keeping the epoch rule unchanged was rejected: for up to 10 minutes after each epoch end the header would lie in the past.

### D8. One fixed block time

`b` is one constant, 5.687 s (the mainnet whole-chain average measured against the archival node), used for the `Expires` estimate and for node age: average node age is blocks between the bonding height and the snapshot height times `b`. No block timestamps are looked up or estimated. The same constant is used on every network.

### D9. With no settled snapshot, routes answer 503

This is what the performance routes do already and `AppState::latest_snapshot` returns. Startup waits for a settled snapshot only. After three consecutive failed heights an instance holds nothing and answers 503 on every route reading the snapshot, where the contract cache would have kept serving its last value: an instance that cannot read the chain consistently stops answering instead of serving something no other instance serves.

### D10. Retired routes and settings

- `POST /v1/nym-nodes/refresh-described` answers 410 Gone with a plain-text body saying the endpoint was removed, nym-api reads node data from the directory, and the node should be upgraded. nym-node logs the body of such a response in its warning.
- Both `/v1/utility` routes are removed with their bearer setting and flag, an explicit exception to keeping routes.
- Legacy mixnode and gateway bonds are no longer read. `GET /v1/legacy/mixnodes` and `GET /v1/legacy/gateways` stay with their response types, are marked deprecated in OpenAPI and on the handler, and always answer an empty list, since no route serving nodes lists legacy bonds any more.
- The `[describe_cache]`, `[mixnet_contract_cache]` and `[node_families_cache]` sections are no longer read; a config file still holding them loads without complaint. Their cache files are left on disk.

### D11. nym-node reads key rotation from the chain first

The interval and the key rotation state are read through the node's shared chain client (`NyxClient`), and nym-api is asked only when that read fails. This applies to startup (`setup_key_rotation`, which today aborts when nym-api is unreachable), the running controller and the rotation-id helper, which takes the client instead of a list of chain URLs.

- In `NymNode::new` the helper is called a few lines before the client is built; the client is built first.
- The three commands that run without a node (initialisation, the sphinx key reset and the v9 config migration) have no shared instance, so each builds the same client for its one read. A mnemonic is at hand in all three: initialisation generates it (before the read), the reset loads it from the config's path, the migration from the old config's path.

The helper's own loop over the chain URLs goes: the client talks to one configured URL, and a failed read falls back to nym-api like everywhere else. A plain query client for the three commands was rejected: it would keep a second way of reading the chain in the node for the sake of not loading a mnemonic.

The stuck-epoch check (`KeyRotationDetails::is_epoch_stuck`) never fires today because it tests `1 - progress` in the branch where progress exceeds 1. It is fixed and kept in the running controller, where it logs a warning and retries every 240 s. It is removed from the rotation-id helper: the rotation id follows the on-chain epoch, so it is correct while the epoch is stuck, and refusing to start (or to reset keys, or to migrate a config) during a stall protects nothing. `expected_current_rotation_id` always returns the reported epoch's id and is deleted with its branch; every case it was meant to catch has the epoch past its nominal end, which the "key rotation is overdue" branch below it already handles.

### D12. Unchanged attested contracts are not read again, and pages are read at the contract maximum

A production makes roughly 250 queries per height on mainnet, most of them pages of the directory (about 130 at the default page of 50) and geolocation (about 50) records. Moving the mixnet state onto the cadence lowers the total (about 40 queries per height instead of about 120 from the 150 s refresher over the same blocks), but the attested reads are repeated even when nothing changed.

- Each contract's verified read first establishes the digest trusted at `H` (one proof query; `verify_chain_read` already does this before reading). When it equals the digest signed in that contract's snapshot of the newest retained entry below `H`, that snapshot's records are reused: they already verify against an equal accumulator, so the guarantee is the one a fresh read would give. The snapshot is still signed for `H`, with the identity map read at `H`. Without a retained entry below `H` (a fresh start, or a skipped height) every record is read.
- Every whole-set read asks for the contract's maximum page: directory and geolocation 100 (default 50), performance latest and epoch performance 200 (default 100), halving those reads where they still happen.

Expected effect: geolocation is reused on most heights and performance on most heights within an epoch; how often the directory is unchanged across 100 blocks is unknown until it is measured. Reading only the changed entries was rejected for now: the contracts have no "changed since height" query, and adding one (an index by update height plus deletion tracking) is a contract migration.

## Risks / Trade-offs

- [More reads per height, so more ways for an instance to skip one] → the height is all-or-nothing by design; the next poll retries the newest height; three misses in a row lead to 503 (D9), which lets clients move to another instance.
- [Nodes not yet publishing the #7399 fields are served with no noise key, terms not accepted and no Lewes Protocol details; the v3 skimmed routes, which list only nodes with Lewes Protocol details, leave them out altogether] → the change ships a few releases after nodes started publishing; there is no measured threshold.
- [A node holding a role can be missing from the topology: no sphinx key for the rotation yet, or a non-public address] → accepted (D4); the config score follow-up addresses the address case.
- [Epoch and key rotation changes reach clients and not-yet-upgraded nym-nodes up to about 10 minutes late] → within the overlap in which nodes keep the previous sphinx key; upgraded nodes read the chain.
- [On nym-node's nym-api fallback path, the snapshot delay plus the transition can exceed the 20% threshold, so the fixed stuck check can warn when nothing is stuck] → fallback path only, the effect is a warning and a slower retry.
- [`Expires` is an estimate over up to one epoch: about 2 s off at today's block time, up to about 2 minutes at the 3% drift seen historically, and it can land one snapshot early or late] → early costs one extra fetch, late keeps the old set cached about 10 minutes longer; a failed height at the expiry leaves the header in the past for one interval.
- [A node running exit without entry becomes flagged exit-capable] → matches the role the driver already gives it.
- [A testnet config with `allow_illegal_ips` under `[describe_cache.debug]` silently loses the setting; a deployment passing `--allow-illegal-ips` or `--utility-routes-bearer` fails to start] → called out in the PR description.
- [`is_epoch_stuck` changes behaviour and `expected_current_rotation_id` disappears for any external user of `nym-api-requests`] → no other caller in the repository.
- [The persisted snapshot file changes shape] → a file that fails to load is already treated as an empty cache and refilled by the startup production.

## Migration Plan

1. Before merging, run the new build locally against mainnet and diff its described and skimmed responses against the live nym-api with a throwaway script kept outside the repository. Differences are expected only in the placeholders and, until nodes publishing them are deployed, in the three #7399 fields; rerun once they are.
2. Merge when the release it will ship in is a few releases after the one in which nodes started publishing the #7399 fields. There is no runtime switch: the deploy is the cutover. nym-node and nym-api ship in the same release.
3. During the rollout instances on the old and new release serve different data; they converge once all are upgraded.
4. Rollback is a redeploy of the previous release. Its caches are rebuilt at startup (their files on disk are reused only if still within their age limits), and the snapshot file written by the new release fails to load there and is refilled.

## Open Questions

- Whether the local read-only run for the comparison needs anything beyond an RPC with enough history has not been checked. If it is not simple, the comparison is dropped.

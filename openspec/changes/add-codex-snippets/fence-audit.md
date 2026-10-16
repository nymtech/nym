# Audit of the Rust fences in the codex Nym network pages (task 2.1)

Date: 2026-10-16. Source: `codex/content/specifications/nym-network/technical-details/`, compared with this repository at `max/codex-snippets`. The compare ignores whitespace and `//` comment lines.

| # | Page and line | Source item | Class | Citation |
|---|---|---|---|---|
| 1 | `signers-monitor/quorum-and-dead-mans-switch.mdx:67` | `nym-signers-monitor/src/test_result.rs:39` | whole, current | `TestResult::quorum_unavailable` |
| 2 | `discovery-urls/network-details-model.mdx:10` | `common/network-defaults/src/network/v1.rs:84` | whole, current. The page text says `network.rs`; the struct is in `network/v1.rs` | `network/v1.rs#NymNetworkDetails` |
| 3 | `discovery-urls/network-details-model.mdx:44` | `common/network-defaults/src/network/v2.rs:35` and `:46` | two items in one fence | `network/v2.rs#NymNetworkDetails`, `network/v2.rs#NetworkingSpecifics` |
| 4 | `rust-sdk/sending-and-receiving.mdx:11` | `sdk/rust/nym-sdk/src/mixnet/traits.rs:29` | whole, current (a trait method signature) | trait method: qualifier case |
| 5 | `verifiable-geolocation/node-status-api-migration.mdx:47` | `nym-node-status-api/.../http/state.rs:480`, inside `build_dvpn_gateway_list` (`:356`) | part of a body (re-wrapped) | P21: cite the whole function, or keep the fence |
| 6 | `nym-geolocator/address-source.mdx:11` | `nym-geolocator/src/node_scraper/address_source/mod.rs:27` | elided: no `pub(crate)`, no doc comments | `AddressSource` |
| 7 | `ecash-contract/instantiation-and-config.mdx:36` | `contracts/ecash/src/helpers.rs:25` | elided (one field) | `Config` |
| 8 | `mixnet-contract/bonding-and-nodes.mdx:10` | `common/cosmwasm-smart-contracts/mixnet-contract/src/nym_node.rs:241` | whole, current | `NymNodeDetails` |
| 9 | `mixnet-contract/bonding-and-nodes.mdx:21` | same file `:364` | elided (one field) | `NymNode` |
| 10 | `mixnet-contract/delegations.mdx:10` | `.../mixnet-contract/src/delegation.rs:39` | elided (one field) | `Delegation` |
| 11 | `mixnet-contract/delegations.mdx:35` | `.../mixnet-contract/src/mixnode.rs:324`, inside `distribute_rewards` | part of a body | P21 |
| 12 | `mixnet-contract/delegations.mdx:46` | same file `:352` | whole, current | `NodeRewarding::determine_delegation_reward` (a second fn of this name is in `rewarding/simulator/mod.rs`, another file) |
| 13 | `mixnet-contract/rewarding-and-epochs.mdx:47` | describes `node_reward` (`mixnode.rs:262`) | a formula, not code | keep the fence; language `text` |
| 14 | `nym-pool-contract/allowance-types.mdx:9` | `.../nym-pool-contract/src/types.rs:203` | whole, current | `BasicAllowance` |
| 15 | same page `:28` | same file `:285` | elided (fields) | `ClassicPeriodicAllowance` |
| 16 | same page `:46` | same file `:352` | whole, current | `ClassicPeriodicAllowance::determine_period_can_spend` |
| 17 | same page `:67` | same file `:377` | elided (comments, one re-wrapped line) | `ClassicPeriodicAllowance::try_update_state`: `try_update_state` has 3 impls in this file, the main qualifier case |
| 18 | same page `:91` | same file `:431` | elided (fields) | `CumulativePeriodicAllowance` |
| 19 | same page `:112` | same file `:525` and `:532` | two items in one fence | `CumulativePeriodicAllowance::missed_periods`, `CumulativePeriodicAllowance::determine_spendable` |
| 20 | same page `:148` | same file `:614` | whole, current | `DelayedAllowance` |
| 21 | `nym-pool-contract/messages-and-queries.mdx:9` | `.../nym-pool-contract/src/msg.rs:17` | whole, current | `InstantiateMsg` |
| 22 | same page `:47` | `.../nym-pool-contract/src/types.rs:14` | whole, current | `TransferRecipient` |
| 23 | same page `:101` | `.../nym-pool-contract/src/msg.rs:123` | whole (the source has `{ // }`, the fence `{}`) | `MigrateMsg` |
| 24 | `data-observatory/storage-and-migrations.mdx:25` | `nym-data-observatory/src/db/mod.rs:7` | whole, current | `queries` (a module) |

Totals: 12 whole and current, 7 elided, 2 part of a body, 2 with two items, 1 formula.

Effect on the plan:

- An elided fence becomes the whole item after the citation, so the page shows more lines than today (7 fences). The plan accepts this (D5); the PR of task 2.9 lists each one.
- Qualifier cases for task 2.3: a method in an `impl` block (`Type::method`), the same method name in three impls of one file (row 17), a trait method in a trait definition (row 4), a module (row 24), two items with one name in two files of one crate (row 2/3, solved by the file path).
- The two part-of-a-body fences (rows 5, 11) wait for P21. Its recommendation is to cite the whole function: `build_dvpn_gateway_list` is 131 lines, so for row 5 a fence that stays may serve the reader better.
- Row 17 shows a condition that looks wrong in the source: `if env.block.time.seconds() > self.period_duration_secs` compares a timestamp with a duration, so it is true for any real block time. The fence copies it as it is. This is not a docs fault; it is for the contract owner to check.

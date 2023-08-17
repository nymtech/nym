# Changelog

## [Unreleased]

## [v1.5.0] (2023-08-16)
- Generate json schema for all used contracts #3693 ([#3693])

[#3233]: https://github.com/nymtech/nym/pull/3693


## [v1.4.0] (2023-04-25)
- Allow mixnode operators to decrease their bond amount without having to rebond (will require a lot of testing EXACT reward values to make sure the "unit delegation" isn't broken afterwards) ([#3233])
- Fix a few clippy warnings in contract test code ([#3340])
- Add --all-targets to clippy for contracts ([#3337])
- A branch with all clippy warnings dealt with in contracts ([#3294])

[#3233]: https://github.com/nymtech/nym/issues/3233
[#3340]: https://github.com/nymtech/nym/pull/3340
[#3337]: https://github.com/nymtech/nym/pull/3337
[#3294]: https://github.com/nymtech/nym/pull/3294

## [v1.3.1] (2023-04-18)
- Add a query to the vesting contract for the amount of delegated tokens towards a particular `mix_id` (might be needed by NG) ([#3228])

[#3228]: https://github.com/nymtech/nym/issues/3228

## [v1.3.0] (2023-04-04)
- change in-contract signatures to include nonces and to sign entire payloads for family-related operations ([#3125])
- change in-contract signatures to include nonces and to sign entire payloads for node bonding (will require wallet changes) ([#3067])
- removed migration code from mixnet and vesting contracts ([#3207])

[#3125]: https://github.com/nymtech/nym/issues/3125
[#3067]: https://github.com/nymtech/nym/issues/3067
[#3207]: https://github.com/nymtech/nym/pull/3207

## [v1.2.0] (2023-03-21)

- Fix contracts and nym-api audit findings ([#3026])

[#3026]: https://github.com/nymtech/nym/issues/3026

## [v1.1.4] (2023-02-21)

- Problem 142 (rust-side) ([#3024])

## [nym-contracts-v1.1.3](https://github.com/nymtech/nym/tree/nym-contracts-v1.1.3) (2022-01-25)

### Added

- vesting-contract: `GetAccountsPaged` and `GetAccountsVestingCoinsPaged` queries for querying multiple accounts simultaneously ([#2791])

[#2791]: https://github.com/nymtech/nym/pull/2791

## [nym-contracts-v1.1.2](https://github.com/nymtech/nym/tree/nym-contracts-v1.1.2) (2022-12-07)

### Added

- Added migration code to the mixnet contract to allow updating stored vesting contract address to make it easier to deploy any future environments ([#1759],[#1769])
- Added an option to pledge additional tokens without the need to rebond minxode ([#1679])
- Added support for node families ([#1670])

[#1670]: https://github.com/nymtech/nym/pull/1670
[#1679]: https://github.com/nymtech/nym/pull/1679
[#1759]: https://github.com/nymtech/nym/pull/1759
[#1769]: https://github.com/nymtech/nym/pull/1769

## [nym-contracts-v1.1.0](https://github.com/nymtech/nym/tree/nym-contracts-v1.1.0) (2022-11-09)

### Changed
- mixnet-contract: rework of rewarding ([#1472]), which includes, but is not limited to:
  - internal reward accounting was modified to be similar to the ideas presented in Cosmos' F1 paper, which results in throughput gains and no storage or gas cost bloat over time,
  - introduced internal queues for pending epoch and interval events that only get resolved once relevant epoch/interval rolls over
  - the contract no longer stores any historical information regarding past epochs/parameters/stake state for the purposes of rewarding
  - a lot of queries got renamed to keep naming more consistent,
  - introduced new utility-based queries such as a query for reward estimation for the current epoch,
  - mixnodes are now identified by a monotonously increasing `mix_id`
  - bonding now results in getting fresh `mix_id` and thus if given node decides to unbond and rebond, it will lose all its delegations,
  - mixnode operators are now allowed to set their operating costs as opposed to having fixed value of 40nym/interval
  - rewarding parameters are now correctly updated at an **interval** end
  - rewarding parameters now include a staking supply scale factor attribute (beta in the tokenomics paper)
  - node performance can now be more granular with internal `Decimal` representation as opposed to an `u8`
  - node profit margin can now be more granular with internal `Decimal` representation as opposed to an `u8`
  - mixnode operators are now allowed to change their configuration options, such as port information, without having to unbond
  - mixnode unbonding is no longer instantaneous, instead it happens once an epoch rolls over
  - it is now possible to query for operator and node history to see how often (and with what parameters) they rebonded
  - other minor bugfixes and changes
  - ...
  - new exciting bugs to find and squash

- vesting-contract: optional locked token pledge cap per account ([#1687]), defaults to 10%
- vesting-contract: updated internal delegation storage due to mixnet contract revamp ([#1472])

### Added
- vesting-contract: added query for obtaining contract build information ([#1726])

[#1472]: https://github.com/nymtech/nym/pull/1472
[#1687]: https://github.com/nymtech/nym/pull/1687
[#1726]: https://github.com/nymtech/nym/pull/1726


## [nym-contracts-v1.0.2](https://github.com/nymtech/nym/tree/nym-contracts-v1.0.2) (2022-09-13)

### Added

- vesting-contract: added queries for delegation timestamps and paged query for all vesting delegations in the contract ([#1569])

### Changed

- mixnet-contract: compounding delegator rewards now happens instantaneously as opposed to having to wait for the current epoch to finish ([#1571])

### Fixed

- vesting-contract: the contract now correctly stores delegations with their timestamp as opposed to using block height ([#1544])
- mixnet-contract: compounding delegator rewards is now possible even if the associated mixnode had already unbonded ([#1571])
- mixnet-contract: fixed reward accumulation after claiming rewards ([#1613])

[#1544]: https://github.com/nymtech/nym/pull/1544
[#1569]: https://github.com/nymtech/nym/pull/1569
[#1571]: https://github.com/nymtech/nym/pull/1571
[#1613]: https://github.com/nymtech/nym/pull/1613

## [nym-contracts-v1.0.1](https://github.com/nymtech/nym/tree/nym-contracts-v1.0.1) (2022-06-22)

### Added

- mixnet-contract: Added ClaimOperatorReward and ClaimDelegatorReward messages ([#1292])
- mixnet-contract: Replace all naked `-` with `saturating_sub`.
- mixnet-contract: Added staking_supply field to ContractStateParams.
- mixnet-contract: Added a query to get MixnodeBond by identity key ([#1369]).
- mixnet-contract: Added a query to get GatewayBond by identity key ([#1369]).
- vesting-contract: Added ClaimOperatorReward and ClaimDelegatorReward messages ([#1292])
- vesting-contract: Added limit to the amount of tokens one can pledge ([#1331])

### Fixed

- mixnet-contract: `estimated_delegator_reward` calculation ([#1284])
- mixnet-contract: delegator and operator rewards use lambda and sigma instead of lambda_ticked and sigma_ticked ([#1284])
- mixnet-contract: removed `expect` in `query_delegator_reward` and queries containing invalid proxy address should now return a more human-readable error ([#1257])
- mixnet-contract: replaced integer division with fixed for performance calculations ([#1284])
- mixnet-contract: Under certain circumstances nodes could not be unbonded ([#1255](https://github.com/nymtech/nym/issues/1255)) ([#1258])
- mixnet-contract: Using correct staking supply when distributing rewards. ([#1373])
- vesting-contract: replaced `checked_sub` with `saturating_sub` to fix the underflow in `get_vesting_tokens` ([#1275])


[#1255]: https://github.com/nymtech/nym/pull/1255
[#1257]: https://github.com/nymtech/nym/pull/1257
[#1258]: https://github.com/nymtech/nym/pull/1258
[#1275]: https://github.com/nymtech/nym/pull/1275
[#1284]: https://github.com/nymtech/nym/pull/1284
[#1292]: https://github.com/nymtech/nym/pull/1292
[#1331]: https://github.com/nymtech/nym/pull/1331
[#1369]: https://github.com/nymtech/nym/pull/1369
[#1373]: https://github.com/nymtech/nym/pull/1373

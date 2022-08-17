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
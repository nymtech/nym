// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::delegations::storage as delegations_storage;
use crate::interval::storage as interval_storage;
use crate::mixnodes;
use crate::mixnodes::storage as mixnodes_storage;
use cosmwasm_std::{coin, Coin, Decimal, Deps, StdResult};
use mixnet_contract_common::mixnode::MixNodeDetails;
use mixnet_contract_common::reward_params::{NodeRewardParams, Performance, RewardingParams};
use mixnet_contract_common::rewarding::helpers::truncate_reward;
use mixnet_contract_common::rewarding::{
    EstimatedCurrentEpochRewardResponse, PendingRewardResponse,
};
use mixnet_contract_common::{Delegation, NodeId};

pub(crate) fn query_rewarding_params(deps: Deps<'_>) -> StdResult<RewardingParams> {
    storage::REWARDING_PARAMS.load(deps.storage)
}

fn pending_operator_reward(mix_details: Option<MixNodeDetails>) -> PendingRewardResponse {
    match mix_details {
        Some(mix_details) => PendingRewardResponse {
            amount_staked: Some(mix_details.original_pledge().clone()),
            amount_earned: Some(mix_details.pending_operator_reward()),
            amount_earned_detailed: Some(mix_details.pending_detailed_operator_reward()),
            mixnode_still_fully_bonded: !mix_details.is_unbonding(),
        },
        None => PendingRewardResponse::default(),
    }
}

pub fn query_pending_operator_reward(
    deps: Deps,
    owner: String,
) -> StdResult<PendingRewardResponse> {
    let owner_address = deps.api.addr_validate(&owner)?;
    // in order to determine operator's reward we need to know its original pledge and thus
    // we have to load the entire thing
    let mix_details = mixnodes::helpers::get_mixnode_details_by_owner(deps.storage, owner_address)?;
    Ok(pending_operator_reward(mix_details))
}

pub fn query_pending_mixnode_operator_reward(
    deps: Deps,
    mix_id: NodeId,
) -> StdResult<PendingRewardResponse> {
    // in order to determine operator's reward we need to know its original pledge and thus
    // we have to load the entire thing
    let mix_details = mixnodes::helpers::get_mixnode_details_by_id(deps.storage, mix_id)?;
    Ok(pending_operator_reward(mix_details))
}

pub fn query_pending_delegator_reward(
    deps: Deps,
    owner: String,
    mix_id: NodeId,
    proxy: Option<String>,
) -> StdResult<PendingRewardResponse> {
    let owner_address = deps.api.addr_validate(&owner)?;
    let proxy = proxy
        .map(|proxy| deps.api.addr_validate(&proxy))
        .transpose()?;

    let mix_rewarding = match storage::MIXNODE_REWARDING.may_load(deps.storage, mix_id)? {
        Some(mix_rewarding) => mix_rewarding,
        None => return Ok(PendingRewardResponse::default()),
    };

    let storage_key = Delegation::generate_storage_key(mix_id, &owner_address, proxy.as_ref());
    let delegation = match delegations_storage::delegations().may_load(deps.storage, storage_key)? {
        Some(delegation) => delegation,
        None => return Ok(PendingRewardResponse::default()),
    };

    let detailed_reward = mix_rewarding.determine_delegation_reward(&delegation);
    let delegator_reward = mix_rewarding.pending_delegator_reward(&delegation);

    // check if the mixnode isnt in the process of unbonding (or has already unbonded)
    let is_bonded = matches!(mixnodes_storage::mixnode_bonds().may_load(deps.storage, mix_id)?, Some(mix_bond) if !mix_bond.is_unbonding);

    Ok(PendingRewardResponse {
        amount_staked: Some(delegation.amount),
        amount_earned: Some(delegator_reward),
        amount_earned_detailed: Some(detailed_reward),
        mixnode_still_fully_bonded: is_bonded,
    })
}

fn zero_reward(
    original_stake: Coin,
    current_value: Decimal,
) -> EstimatedCurrentEpochRewardResponse {
    EstimatedCurrentEpochRewardResponse {
        estimation: Some(coin(0, &original_stake.denom)),
        detailed_estimation_amount: Some(Decimal::zero()),
        current_stake_value: Some(truncate_reward(current_value, &original_stake.denom)),
        current_stake_value_detailed_amount: Some(current_value),
        original_stake: Some(original_stake),
    }
}

pub(crate) fn query_estimated_current_epoch_operator_reward(
    deps: Deps<'_>,
    mix_id: NodeId,
    estimated_performance: Performance,
) -> StdResult<EstimatedCurrentEpochRewardResponse> {
    let mix_details = match mixnodes::helpers::get_mixnode_details_by_id(deps.storage, mix_id)? {
        None => return Ok(EstimatedCurrentEpochRewardResponse::empty_response()),
        Some(mix_details) => mix_details,
    };

    let amount_staked = mix_details.original_pledge().clone();
    let mix_rewarding = mix_details.rewarding_details;
    let current_value = mix_rewarding.operator;

    // if node is currently not in the rewarded set, the performance is 0,
    // or the node has either unbonded or is in the process of unbonding,
    // the calculations are trivial - the rewards are 0
    if mix_details.bond_information.is_unbonding {
        return Ok(zero_reward(amount_staked, current_value));
    }

    let node_status = match interval_storage::REWARDED_SET.may_load(deps.storage, mix_id)? {
        None => return Ok(zero_reward(amount_staked, current_value)),
        Some(node_status) => node_status,
    };

    if estimated_performance.is_zero() {
        return Ok(zero_reward(amount_staked, current_value));
    }

    let rewarding_params = storage::REWARDING_PARAMS.load(deps.storage)?;
    let interval = interval_storage::current_interval(deps.storage)?;

    let node_reward_params = NodeRewardParams::new(estimated_performance, node_status.is_active());
    let node_reward = mix_rewarding.node_reward(&rewarding_params, node_reward_params);
    let reward_distribution = mix_rewarding.determine_reward_split(
        node_reward,
        estimated_performance,
        interval.epochs_in_interval(),
    );

    Ok(EstimatedCurrentEpochRewardResponse {
        estimation: Some(truncate_reward(
            reward_distribution.operator,
            &amount_staked.denom,
        )),
        detailed_estimation_amount: Some(reward_distribution.operator),
        current_stake_value: Some(truncate_reward(current_value, &amount_staked.denom)),
        current_stake_value_detailed_amount: Some(current_value),
        original_stake: Some(amount_staked),
    })
}

pub(crate) fn query_estimated_current_epoch_delegator_reward(
    deps: Deps<'_>,
    owner: String,
    mix_id: NodeId,
    proxy: Option<String>,
    estimated_performance: Performance,
) -> StdResult<EstimatedCurrentEpochRewardResponse> {
    let owner_address = deps.api.addr_validate(&owner)?;
    let proxy = proxy
        .map(|proxy| deps.api.addr_validate(&proxy))
        .transpose()?;

    let mix_rewarding = match storage::MIXNODE_REWARDING.may_load(deps.storage, mix_id)? {
        Some(mix_rewarding) => mix_rewarding,
        None => return Ok(EstimatedCurrentEpochRewardResponse::empty_response()),
    };

    let storage_key = Delegation::generate_storage_key(mix_id, &owner_address, proxy.as_ref());
    let delegation = match delegations_storage::delegations().may_load(deps.storage, storage_key)? {
        Some(delegation) => delegation,
        None => return Ok(EstimatedCurrentEpochRewardResponse::empty_response()),
    };

    let staked_dec = Decimal::from_atomics(delegation.amount.amount, 0).unwrap();
    let current_value = staked_dec + mix_rewarding.determine_delegation_reward(&delegation);
    let amount_staked = delegation.amount;

    // check if the mixnode isnt in the process of unbonding (or has already unbonded)
    let is_bonded = matches!(mixnodes_storage::mixnode_bonds().may_load(deps.storage, mix_id)?, Some(mix_bond) if !mix_bond.is_unbonding);

    if !is_bonded {
        return Ok(zero_reward(amount_staked, current_value));
    }

    // if node is currently not in the rewarded set, the performance is 0,
    // or the node has either unbonded or is in the process of unbonding,
    // the calculations are trivial - the rewards are 0
    let node_status = match interval_storage::REWARDED_SET.may_load(deps.storage, mix_id)? {
        None => return Ok(zero_reward(amount_staked, current_value)),
        Some(node_status) => node_status,
    };

    if estimated_performance.is_zero() {
        return Ok(zero_reward(amount_staked, current_value));
    }

    let rewarding_params = storage::REWARDING_PARAMS.load(deps.storage)?;
    let interval = interval_storage::current_interval(deps.storage)?;

    let node_reward_params = NodeRewardParams::new(estimated_performance, node_status.is_active());
    let node_reward = mix_rewarding.node_reward(&rewarding_params, node_reward_params);
    let reward_distribution = mix_rewarding.determine_reward_split(
        node_reward,
        estimated_performance,
        interval.epochs_in_interval(),
    );

    if reward_distribution.delegates.is_zero() {
        return Ok(zero_reward(amount_staked, current_value));
    }

    let reward_share = current_value / mix_rewarding.delegates * reward_distribution.delegates;

    Ok(EstimatedCurrentEpochRewardResponse {
        estimation: Some(truncate_reward(reward_share, &amount_staked.denom)),
        detailed_estimation_amount: Some(reward_share),
        current_stake_value: Some(truncate_reward(current_value, &amount_staked.denom)),
        current_stake_value_detailed_amount: Some(current_value),
        original_stake: Some(amount_staked),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::test_helpers;
    use crate::support::tests::test_helpers::TestSetup;
    use cosmwasm_std::Uint128;

    #[test]
    fn querying_for_rewarding_params() {
        // not much to test here. after contract is initialised, the query must always be valid
        let deps = test_helpers::init_contract();
        let res = query_rewarding_params(deps.as_ref());

        assert!(res.is_ok())
    }

    #[cfg(test)]
    mod querying_for_pending_operator_reward {
        use super::*;
        use crate::mixnodes::transactions::try_remove_mixnode;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::{coin, Decimal};
        use mixnet_contract_common::rewarding::helpers::truncate_reward;

        #[test]
        fn for_non_existent_node() {
            let test = TestSetup::new();
            let owner = "mix-owner";

            let res = query_pending_operator_reward(test.deps(), owner.into()).unwrap();
            let res2 = query_pending_mixnode_operator_reward(test.deps(), 42).unwrap();
            assert_eq!(res, res2);

            assert!(res.amount_earned.is_none());
            assert!(res.amount_earned_detailed.is_none());
            assert!(res.amount_staked.is_none());
            assert!(!res.mixnode_still_fully_bonded);
        }

        #[test]
        fn for_unrewarded_node() {
            let mut test = TestSetup::new();
            let owner = "mix-owner";

            let initial_stake = Uint128::new(1_000_000_000_000);
            let mix_id = test.add_dummy_mixnode(owner, Some(initial_stake));

            let res = query_pending_operator_reward(test.deps(), owner.into()).unwrap();
            let res2 = query_pending_mixnode_operator_reward(test.deps(), mix_id).unwrap();
            assert_eq!(res, res2);

            let expected_actual = coin(0, TEST_COIN_DENOM);

            assert_eq!(res.amount_earned.unwrap(), expected_actual);
            assert_eq!(res.amount_earned_detailed.unwrap(), Decimal::zero());
            assert_eq!(res.amount_staked.unwrap().amount, initial_stake);
            assert!(res.mixnode_still_fully_bonded);
        }

        #[test]
        fn for_node_with_pending_reward() {
            let mut test = TestSetup::new();
            let owner = "mix-owner";
            let initial_stake = Uint128::new(1_000_000_000_000);
            let mix_id = test.add_dummy_mixnode(owner, Some(initial_stake));

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);

            let mut total_earned = Decimal::zero();
            let dist = test.reward_with_distribution(mix_id, test_helpers::performance(100.0));
            total_earned += dist.operator;

            let res = query_pending_operator_reward(test.deps(), owner.into()).unwrap();
            let res2 = query_pending_mixnode_operator_reward(test.deps(), mix_id).unwrap();
            assert_eq!(res, res2);

            let expected_actual = truncate_reward(total_earned, TEST_COIN_DENOM);

            assert_eq!(res.amount_earned.unwrap(), expected_actual);
            assert_eq!(res.amount_earned_detailed.unwrap(), total_earned);
            assert_eq!(res.amount_staked.unwrap().amount, initial_stake);
            assert!(res.mixnode_still_fully_bonded);

            // reward it few more times for good measure
            for _ in 0..10 {
                test.skip_to_next_epoch_end();
                let dist = test.reward_with_distribution(mix_id, test_helpers::performance(100.0));
                total_earned += dist.operator;

                let res = query_pending_operator_reward(test.deps(), owner.into()).unwrap();
                let res2 = query_pending_mixnode_operator_reward(test.deps(), mix_id).unwrap();
                assert_eq!(res, res2);

                let expected_actual = truncate_reward(total_earned, TEST_COIN_DENOM);

                assert_eq!(res.amount_earned.unwrap(), expected_actual);
                assert_eq!(res.amount_earned_detailed.unwrap(), total_earned);
                assert_eq!(res.amount_staked.unwrap().amount, initial_stake);
                assert!(res.mixnode_still_fully_bonded);
            }
        }

        #[test]
        fn for_node_that_is_unbonding() {
            let mut test = TestSetup::new();
            let owner = "mix-owner";
            let initial_stake = Uint128::new(1_000_000_000_000);
            let mix_id = test.add_dummy_mixnode(owner, Some(initial_stake));

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);

            let mut total_earned = Decimal::zero();
            let dist = test.reward_with_distribution(mix_id, test_helpers::performance(100.0));
            total_earned += dist.operator;

            let sender = mock_info(owner, &[]);
            try_remove_mixnode(test.deps_mut(), sender).unwrap();

            let res = query_pending_operator_reward(test.deps(), owner.into()).unwrap();
            let res2 = query_pending_mixnode_operator_reward(test.deps(), mix_id).unwrap();
            assert_eq!(res, res2);

            let expected_actual = truncate_reward(total_earned, TEST_COIN_DENOM);
            assert_eq!(res.amount_earned.unwrap(), expected_actual);
            assert_eq!(res.amount_earned_detailed.unwrap(), total_earned);
            assert_eq!(res.amount_staked.unwrap().amount, initial_stake);
            assert!(!res.mixnode_still_fully_bonded);
        }

        #[test]
        fn for_node_that_has_unbonded() {
            let mut test = TestSetup::new();
            let owner = "mix-owner";
            let initial_stake = Uint128::new(1_000_000_000_000);
            let mix_id = test.add_dummy_mixnode(owner, Some(initial_stake));

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);

            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            let sender = mock_info(owner, &[]);
            try_remove_mixnode(test.deps_mut(), sender).unwrap();
            test.execute_all_pending_events();

            let res = query_pending_operator_reward(test.deps(), owner.into()).unwrap();
            let res2 = query_pending_mixnode_operator_reward(test.deps(), mix_id).unwrap();
            assert_eq!(res, res2);

            // if you unbonded, you don't have any pending stuff as you've already claimed it
            // by unbonding
            assert!(res.amount_earned.is_none());
            assert!(res.amount_earned_detailed.is_none());
            assert!(res.amount_staked.is_none());
            assert!(!res.mixnode_still_fully_bonded);
        }
    }

    #[cfg(test)]
    mod querying_for_pending_delegator_reward {
        use super::*;
        use crate::mixnodes::transactions::try_remove_mixnode;
        use crate::rewards::transactions::try_withdraw_delegator_reward;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use crate::support::tests::test_helpers::get_bank_send_msg;
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::{coin, Decimal};
        use mixnet_contract_common::rewarding::helpers::truncate_reward;

        #[test]
        fn for_non_existent_delegation() {
            let test = TestSetup::new();
            let delegator = "delegator";

            let res =
                query_pending_delegator_reward(test.deps(), delegator.into(), 42, None).unwrap();

            assert!(res.amount_earned.is_none());
            assert!(res.amount_earned_detailed.is_none());
            assert!(res.amount_staked.is_none());
            assert!(!res.mixnode_still_fully_bonded);
        }

        #[test]
        fn for_unrewarded_delegator() {
            let mut test = TestSetup::new();
            let owner = "delegator";

            let initial_stake = Uint128::new(100_000_000);
            let mix_id = test.add_dummy_mixnode("mix-owner", Some(Uint128::new(1_000_000_000_000)));
            test.add_immediate_delegation(owner, initial_stake, mix_id);

            let res =
                query_pending_delegator_reward(test.deps(), owner.into(), mix_id, None).unwrap();

            let expected_actual = coin(0, TEST_COIN_DENOM);

            assert_eq!(res.amount_earned.unwrap(), expected_actual);
            assert_eq!(res.amount_earned_detailed.unwrap(), Decimal::zero());
            assert_eq!(res.amount_staked.unwrap().amount, initial_stake);
            assert!(res.mixnode_still_fully_bonded);
        }

        #[test]
        fn for_delegator_with_pending_reward() {
            let mut test = TestSetup::new();
            let owner = "delegator";

            let initial_stake = Uint128::new(100_000_000);
            let mix_id = test.add_dummy_mixnode("mix-owner", Some(Uint128::new(1_000_000_000_000)));
            test.add_immediate_delegation(owner, initial_stake, mix_id);

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);

            let mut total_earned = Decimal::zero();
            let dist = test.reward_with_distribution(mix_id, test_helpers::performance(100.0));
            total_earned += dist.delegates;

            let res =
                query_pending_delegator_reward(test.deps(), owner.into(), mix_id, None).unwrap();

            let expected_actual = truncate_reward(total_earned, TEST_COIN_DENOM);

            assert_eq!(res.amount_earned.unwrap(), expected_actual);
            assert_eq!(res.amount_earned_detailed.unwrap(), total_earned);
            assert_eq!(res.amount_staked.unwrap().amount, initial_stake);
            assert!(res.mixnode_still_fully_bonded);

            // reward it few more times for good measure
            for _ in 0..10 {
                test.skip_to_next_epoch_end();
                let dist = test.reward_with_distribution(mix_id, test_helpers::performance(100.0));
                total_earned += dist.delegates;

                let res = query_pending_delegator_reward(test.deps(), owner.into(), mix_id, None)
                    .unwrap();

                let expected_actual = truncate_reward(total_earned, TEST_COIN_DENOM);

                assert_eq!(res.amount_earned.unwrap(), expected_actual);
                assert_eq!(res.amount_earned_detailed.unwrap(), total_earned);
                assert_eq!(res.amount_staked.unwrap().amount, initial_stake);
                assert!(res.mixnode_still_fully_bonded);
            }
        }

        #[test]
        fn for_node_that_is_unbonding() {
            let mut test = TestSetup::new();
            let owner = "delegator";

            let initial_stake = Uint128::new(100_000_000);
            let mix_id = test.add_dummy_mixnode("mix-owner", Some(Uint128::new(1_000_000_000_000)));
            test.add_immediate_delegation(owner, initial_stake, mix_id);

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);

            let mut total_earned = Decimal::zero();
            let dist = test.reward_with_distribution(mix_id, test_helpers::performance(100.0));
            total_earned += dist.delegates;

            let sender = mock_info("mix-owner", &[]);
            try_remove_mixnode(test.deps_mut(), sender).unwrap();

            let res =
                query_pending_delegator_reward(test.deps(), owner.into(), mix_id, None).unwrap();

            let expected_actual = truncate_reward(total_earned, TEST_COIN_DENOM);
            assert_eq!(res.amount_earned.unwrap(), expected_actual);
            assert_eq!(res.amount_earned_detailed.unwrap(), total_earned);
            assert_eq!(res.amount_staked.unwrap().amount, initial_stake);
            assert!(!res.mixnode_still_fully_bonded);
        }

        #[test]
        fn for_node_that_has_unbonded() {
            let mut test = TestSetup::new();
            let owner = "delegator";

            let initial_stake = Uint128::new(100_000_000);
            let mix_id = test.add_dummy_mixnode("mix-owner", Some(Uint128::new(1_000_000_000_000)));
            test.add_immediate_delegation(owner, initial_stake, mix_id);

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);

            let mut total_earned = Decimal::zero();
            let dist = test.reward_with_distribution(mix_id, test_helpers::performance(100.0));
            total_earned += dist.delegates;

            let sender = mock_info("mix-owner", &[]);
            try_remove_mixnode(test.deps_mut(), sender).unwrap();
            test.execute_all_pending_events();

            let res =
                query_pending_delegator_reward(test.deps(), owner.into(), mix_id, None).unwrap();

            let expected_actual = truncate_reward(total_earned, TEST_COIN_DENOM);
            assert_eq!(res.amount_earned.unwrap(), expected_actual);
            assert_eq!(res.amount_earned_detailed.unwrap(), total_earned);
            assert_eq!(res.amount_staked.unwrap().amount, initial_stake);
            assert!(!res.mixnode_still_fully_bonded);
        }

        #[test]
        fn always_equals_to_what_can_be_withdrawn() {
            // we've already tested withdraw reward to calculate values correctly
            // even if there are multiple delegators joined at different times when the reward has to be split
            let mut test = TestSetup::new();
            let del1 = "delegator1";
            let del2 = "delegator2";
            let del3 = "delegator3";
            let del4 = "delegator4";

            let mix_id = test.add_dummy_mixnode("mix-owner", Some(Uint128::new(1_000_000_000_000)));
            test.add_immediate_delegation(del1, 123_456_789u32, mix_id);
            test.add_immediate_delegation(del2, 150_000_000u32, mix_id);

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);

            test.skip_to_next_epoch_end();
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));
            test.skip_to_next_epoch_end();
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            test.add_immediate_delegation(del3, 500_000_000u32, mix_id);
            test.skip_to_next_epoch_end();
            test.reward_with_distribution(mix_id, test_helpers::performance(85.0));
            test.skip_to_next_epoch_end();
            test.reward_with_distribution(mix_id, test_helpers::performance(5.0));

            test.add_immediate_delegation(del4, 5_000_000u32, mix_id);
            test.skip_to_next_epoch_end();
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            test.add_immediate_delegation(del2, 250_000_000u32, mix_id);
            test.skip_to_next_epoch_end();
            test.reward_with_distribution(mix_id, test_helpers::performance(98.0));
            test.skip_to_next_epoch_end();
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            test.remove_immediate_delegation(del3, mix_id);
            test.skip_to_next_epoch_end();
            test.reward_with_distribution(mix_id, test_helpers::performance(98.0));
            test.skip_to_next_epoch_end();
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            let pending1 =
                query_pending_delegator_reward(test.deps(), del1.into(), mix_id, None).unwrap();
            let pending2 =
                query_pending_delegator_reward(test.deps(), del2.into(), mix_id, None).unwrap();
            let pending3 =
                query_pending_delegator_reward(test.deps(), del3.into(), mix_id, None).unwrap();
            let pending4 =
                query_pending_delegator_reward(test.deps(), del4.into(), mix_id, None).unwrap();

            let actual1_res =
                try_withdraw_delegator_reward(test.deps_mut(), mock_info(del1, &[]), mix_id)
                    .unwrap();
            let (_, actual1) = get_bank_send_msg(&actual1_res).unwrap();
            assert_eq!(pending1.amount_earned.unwrap(), actual1[0]);

            let actual2_res =
                try_withdraw_delegator_reward(test.deps_mut(), mock_info(del2, &[]), mix_id)
                    .unwrap();
            let (_, actual2) = get_bank_send_msg(&actual2_res).unwrap();
            assert_eq!(pending2.amount_earned.unwrap(), actual2[0]);

            // the amount is none because we have removed our delegation!
            assert!(pending3.amount_earned.is_none());

            let actual4_res =
                try_withdraw_delegator_reward(test.deps_mut(), mock_info(del4, &[]), mix_id)
                    .unwrap();
            let (_, actual4) = get_bank_send_msg(&actual4_res).unwrap();
            assert_eq!(pending4.amount_earned.unwrap(), actual4[0]);
        }
    }

    #[cfg(test)]
    mod querying_for_estimated_current_epoch_operator_reward {
        use super::*;
        use crate::mixnodes::transactions::try_remove_mixnode;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use cosmwasm_std::testing::mock_info;

        fn expected_current_operator(
            test: &TestSetup,
            mix_id: NodeId,
            initial_stake: Uint128,
        ) -> EstimatedCurrentEpochRewardResponse {
            let mix_rewarding = test.mix_rewarding(mix_id);
            EstimatedCurrentEpochRewardResponse {
                estimation: Some(coin(0, TEST_COIN_DENOM)),
                detailed_estimation_amount: Some(Decimal::zero()),
                current_stake_value: Some(truncate_reward(mix_rewarding.operator, TEST_COIN_DENOM)),
                current_stake_value_detailed_amount: Some(mix_rewarding.operator),
                original_stake: Some(coin(initial_stake.u128(), TEST_COIN_DENOM)),
            }
        }

        #[test]
        fn when_node_doesnt_exist() {
            let test = TestSetup::new();
            let res = query_estimated_current_epoch_operator_reward(
                test.deps(),
                42,
                test_helpers::performance(100.0),
            )
            .unwrap();
            assert_eq!(res, EstimatedCurrentEpochRewardResponse::empty_response())
        }

        #[test]
        fn when_node_is_unbonding() {
            let mut test = TestSetup::new();
            let initial_stake = Uint128::new(1_000_000_000_000);
            let owner = "mix-owner";
            let mix_id = test.add_dummy_mixnode(owner, Some(initial_stake));

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            let sender = mock_info(owner, &[]);
            try_remove_mixnode(test.deps_mut(), sender).unwrap();

            let res = query_estimated_current_epoch_operator_reward(
                test.deps(),
                mix_id,
                test_helpers::performance(100.0),
            )
            .unwrap();

            let expected = expected_current_operator(&test, mix_id, initial_stake);
            assert_eq!(res, expected)
        }

        #[test]
        fn when_node_has_already_unbonded() {
            let mut test = TestSetup::new();
            let initial_stake = Uint128::new(1_000_000_000_000);
            let owner = "mix-owner";
            let mix_id = test.add_dummy_mixnode(owner, Some(initial_stake));

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            let sender = mock_info(owner, &[]);
            try_remove_mixnode(test.deps_mut(), sender).unwrap();
            test.execute_all_pending_events();

            let res = query_estimated_current_epoch_operator_reward(
                test.deps(),
                mix_id,
                test_helpers::performance(100.0),
            )
            .unwrap();
            assert_eq!(res, EstimatedCurrentEpochRewardResponse::empty_response())
        }

        #[test]
        fn when_node_is_not_in_the_rewarded_set() {
            let mut test = TestSetup::new();
            let initial_stake = Uint128::new(1_000_000_000_000);
            let mix_id = test.add_dummy_mixnode("mix-owner", Some(initial_stake));

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));
            test.update_rewarded_set(vec![]);

            let res = query_estimated_current_epoch_operator_reward(
                test.deps(),
                mix_id,
                test_helpers::performance(100.0),
            )
            .unwrap();

            let expected = expected_current_operator(&test, mix_id, initial_stake);
            assert_eq!(res, expected)
        }

        #[test]
        fn when_estimated_performance_is_zero() {
            let mut test = TestSetup::new();
            let initial_stake = Uint128::new(1_000_000_000_000);
            let mix_id = test.add_dummy_mixnode("mix-owner", Some(initial_stake));

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            let res = query_estimated_current_epoch_operator_reward(
                test.deps(),
                mix_id,
                test_helpers::performance(0.0),
            )
            .unwrap();

            let expected = expected_current_operator(&test, mix_id, initial_stake);
            assert_eq!(res, expected)
        }

        #[test]
        fn with_correct_parameters_matches_actual_distribution() {
            let mut test = TestSetup::new();
            let initial_stake = Uint128::new(1_000_000_000_000);
            let mix_id = test.add_dummy_mixnode("mix-owner", Some(initial_stake));

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            let mix_rewarding = test.mix_rewarding(mix_id);
            let res = query_estimated_current_epoch_operator_reward(
                test.deps(),
                mix_id,
                test_helpers::performance(95.0),
            )
            .unwrap();

            test.skip_to_next_epoch_end();
            let dist = test.reward_with_distribution(mix_id, test_helpers::performance(95.0));

            let expected = EstimatedCurrentEpochRewardResponse {
                original_stake: Some(coin(initial_stake.u128(), TEST_COIN_DENOM)),
                current_stake_value: Some(truncate_reward(mix_rewarding.operator, TEST_COIN_DENOM)),
                current_stake_value_detailed_amount: Some(mix_rewarding.operator),
                estimation: Some(truncate_reward(dist.operator, TEST_COIN_DENOM)),
                detailed_estimation_amount: Some(dist.operator),
            };

            assert_eq!(res, expected)
        }
    }

    #[cfg(test)]
    mod querying_for_estimated_current_epoch_delegator_reward {
        use super::*;
        use crate::mixnodes::transactions::try_remove_mixnode;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use cosmwasm_std::testing::mock_info;

        fn expected_current_delegator(
            test: &TestSetup,
            mix_id: NodeId,
            owner: &str,
        ) -> EstimatedCurrentEpochRewardResponse {
            let mix_rewarding = test.mix_rewarding(mix_id);
            let delegation = test.delegation(mix_id, owner, &None);

            let staked_dec = Decimal::from_atomics(delegation.amount.amount, 0).unwrap();
            let current_value = staked_dec + mix_rewarding.determine_delegation_reward(&delegation);
            let amount_staked = delegation.amount;

            EstimatedCurrentEpochRewardResponse {
                estimation: Some(coin(0, TEST_COIN_DENOM)),
                detailed_estimation_amount: Some(Decimal::zero()),
                current_stake_value: Some(truncate_reward(current_value, TEST_COIN_DENOM)),
                current_stake_value_detailed_amount: Some(current_value),
                original_stake: Some(amount_staked),
            }
        }

        #[test]
        fn when_delegation_doesnt_exist() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            let res = query_estimated_current_epoch_delegator_reward(
                test.deps(),
                "foomper".into(),
                mix_id,
                None,
                test_helpers::performance(100.0),
            )
            .unwrap();

            assert_eq!(res, EstimatedCurrentEpochRewardResponse::empty_response())
        }

        #[test]
        fn when_node_is_unbonding() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let initial_stake = Uint128::new(1_000_000_000);
            let owner = "delegator";
            test.add_immediate_delegation(owner, initial_stake, mix_id);

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            let sender = mock_info("mix-owner", &[]);
            try_remove_mixnode(test.deps_mut(), sender).unwrap();

            let res = query_estimated_current_epoch_delegator_reward(
                test.deps(),
                owner.into(),
                mix_id,
                None,
                test_helpers::performance(100.0),
            )
            .unwrap();

            let expected = expected_current_delegator(&test, mix_id, owner);
            assert_eq!(res, expected)
        }

        #[test]
        fn when_node_has_already_unbonded() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let initial_stake = Uint128::new(1_000_000_000);
            let owner = "delegator";
            test.add_immediate_delegation(owner, initial_stake, mix_id);

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            let sender = mock_info("mix-owner", &[]);
            try_remove_mixnode(test.deps_mut(), sender).unwrap();
            test.execute_all_pending_events();

            let res = query_estimated_current_epoch_delegator_reward(
                test.deps(),
                owner.into(),
                mix_id,
                None,
                test_helpers::performance(100.0),
            )
            .unwrap();

            let expected = expected_current_delegator(&test, mix_id, owner);
            assert_eq!(res, expected)
        }

        #[test]
        fn when_node_is_not_in_the_rewarded_set() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let initial_stake = Uint128::new(1_000_000_000);
            let owner = "delegator";
            test.add_immediate_delegation(owner, initial_stake, mix_id);

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));
            test.update_rewarded_set(vec![]);

            let res = query_estimated_current_epoch_delegator_reward(
                test.deps(),
                owner.into(),
                mix_id,
                None,
                test_helpers::performance(100.0),
            )
            .unwrap();

            let expected = expected_current_delegator(&test, mix_id, owner);
            assert_eq!(res, expected)
        }

        #[test]
        fn when_estimated_performance_is_zero() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let initial_stake = Uint128::new(1_000_000_000);
            let owner = "delegator";
            test.add_immediate_delegation(owner, initial_stake, mix_id);

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            let res = query_estimated_current_epoch_delegator_reward(
                test.deps(),
                owner.into(),
                mix_id,
                None,
                test_helpers::performance(0.0),
            )
            .unwrap();

            let expected = expected_current_delegator(&test, mix_id, owner);
            assert_eq!(res, expected)
        }

        #[test]
        fn with_correct_parameters_matches_actual_distribution_for_single_delegator() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let initial_stake = Uint128::new(1_000_000_000);
            let owner = "delegator";
            test.add_immediate_delegation(owner, initial_stake, mix_id);

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);

            let mix_rewarding = test.mix_rewarding(mix_id);
            let res = query_estimated_current_epoch_delegator_reward(
                test.deps(),
                owner.into(),
                mix_id,
                None,
                test_helpers::performance(95.0),
            )
            .unwrap();

            test.skip_to_next_epoch_end();
            let dist = test.reward_with_distribution(mix_id, test_helpers::performance(95.0));

            let expected = EstimatedCurrentEpochRewardResponse {
                original_stake: Some(coin(initial_stake.u128(), TEST_COIN_DENOM)),
                current_stake_value: Some(truncate_reward(
                    mix_rewarding.delegates,
                    TEST_COIN_DENOM,
                )),
                current_stake_value_detailed_amount: Some(mix_rewarding.delegates),
                estimation: Some(truncate_reward(dist.delegates, TEST_COIN_DENOM)),
                detailed_estimation_amount: Some(dist.delegates),
            };

            assert_eq!(res, expected)
        }

        #[test]
        fn with_correct_parameters_matches_actual_distribution_for_three_delegators() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let initial_stake1 = Uint128::new(1_000_000_000);
            let initial_stake2 = Uint128::new(45_000_000_000);
            let initial_stake3 = Uint128::new(8_500_000_000);

            let initial_stake1_dec = Decimal::from_atomics(initial_stake1, 0).unwrap();
            let initial_stake2_dec = Decimal::from_atomics(initial_stake2, 0).unwrap();
            let initial_stake3_dec = Decimal::from_atomics(initial_stake3, 0).unwrap();
            let del1 = "delegator1";
            let del2 = "delegator2";
            let del3 = "delegator3";
            test.add_immediate_delegation(del1, initial_stake1, mix_id);
            test.add_immediate_delegation(del2, initial_stake2, mix_id);

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);
            test.reward_with_distribution(mix_id, test_helpers::performance(95.0));

            test.add_immediate_delegation(del3, initial_stake3, mix_id);
            test.skip_to_next_epoch_end();
            test.reward_with_distribution(mix_id, test_helpers::performance(85.0));

            let mix_rewarding = test.mix_rewarding(mix_id);

            let ress = [del1, del2, del3]
                .iter()
                .map(|owner| {
                    query_estimated_current_epoch_delegator_reward(
                        test.deps(),
                        owner.to_string(),
                        mix_id,
                        None,
                        test_helpers::performance(95.0),
                    )
                    .unwrap()
                })
                .collect::<Vec<_>>();

            // as verified by other tests those values are correct
            let est1 = query_pending_delegator_reward(test.deps(), del1.into(), mix_id, None)
                .unwrap()
                .amount_earned_detailed
                .unwrap();
            let est2 = query_pending_delegator_reward(test.deps(), del2.into(), mix_id, None)
                .unwrap()
                .amount_earned_detailed
                .unwrap();
            let est3 = query_pending_delegator_reward(test.deps(), del3.into(), mix_id, None)
                .unwrap()
                .amount_earned_detailed
                .unwrap();

            let cur1 = initial_stake1_dec + est1;
            let cur2 = initial_stake2_dec + est2;
            let cur3 = initial_stake3_dec + est3;

            test.skip_to_next_epoch_end();
            let dist = test.reward_with_distribution(mix_id, test_helpers::performance(95.0));

            let share1 = cur1 / mix_rewarding.delegates * dist.delegates;
            let share2 = cur2 / mix_rewarding.delegates * dist.delegates;
            let share3 = cur3 / mix_rewarding.delegates * dist.delegates;

            let expected1 = EstimatedCurrentEpochRewardResponse {
                original_stake: Some(coin(initial_stake1.u128(), TEST_COIN_DENOM)),
                current_stake_value: Some(truncate_reward(cur1, TEST_COIN_DENOM)),
                current_stake_value_detailed_amount: Some(cur1),
                estimation: Some(truncate_reward(share1, TEST_COIN_DENOM)),
                detailed_estimation_amount: Some(share1),
            };
            assert_eq!(ress[0], expected1);

            let expected2 = EstimatedCurrentEpochRewardResponse {
                original_stake: Some(coin(initial_stake2.u128(), TEST_COIN_DENOM)),
                current_stake_value: Some(truncate_reward(cur2, TEST_COIN_DENOM)),
                current_stake_value_detailed_amount: Some(cur2),
                estimation: Some(truncate_reward(share2, TEST_COIN_DENOM)),
                detailed_estimation_amount: Some(share2),
            };
            assert_eq!(ress[1], expected2);

            let expected3 = EstimatedCurrentEpochRewardResponse {
                original_stake: Some(coin(initial_stake3.u128(), TEST_COIN_DENOM)),
                current_stake_value: Some(truncate_reward(cur3, TEST_COIN_DENOM)),
                current_stake_value_detailed_amount: Some(cur3),
                estimation: Some(truncate_reward(share3, TEST_COIN_DENOM)),
                detailed_estimation_amount: Some(share3),
            };
            assert_eq!(ress[2], expected3);
        }
    }
}

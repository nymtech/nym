// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::delegations::storage as delegations_storage;
use crate::mixnodes;
use crate::mixnodes::storage as mixnodes_storage;
use cosmwasm_std::{Deps, StdResult};
use mixnet_contract_common::mixnode::MixNodeDetails;
use mixnet_contract_common::reward_params::RewardingParams;
use mixnet_contract_common::rewarding::PendingRewardResponse;
use mixnet_contract_common::{Delegation, NodeId};

pub(crate) fn query_rewarding_params(deps: Deps<'_>) -> StdResult<RewardingParams> {
    storage::REWARDING_PARAMS.load(deps.storage)
}

fn pending_operator_reward(mix_details: Option<MixNodeDetails>) -> PendingRewardResponse {
    match mix_details {
        Some(mix_rewarding) => PendingRewardResponse {
            amount_staked: Some(mix_rewarding.original_pledge().clone()),
            amount_earned: Some(mix_rewarding.pending_operator_reward()),
            amount_earned_detailed: Some(mix_rewarding.pending_detailed_operator_reward()),
            mixnode_still_fully_bonded: !mix_rewarding.is_unbonding(),
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
}

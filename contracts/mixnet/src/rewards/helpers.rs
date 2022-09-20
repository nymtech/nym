// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::delegations::storage as delegations_storage;
use crate::interval::storage as interval_storage;
use cosmwasm_std::{Coin, Decimal, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::mixnode::{MixNodeDetails, MixNodeRewarding};
use mixnet_contract_common::Delegation;

/// Recomputes rewarding parameters (such as staking supply, saturation point, etc) based on
/// pending changes currently stored in `PENDING_REWARD_POOL_CHANGE`.
pub(crate) fn apply_reward_pool_changes(
    store: &mut dyn Storage,
) -> Result<(), MixnetContractError> {
    let mut rewarding_params = storage::REWARDING_PARAMS.load(store)?;
    let pending_pool_change = storage::PENDING_REWARD_POOL_CHANGE.load(store)?;
    let interval = interval_storage::current_interval(store)?;

    let reward_pool = rewarding_params.interval.reward_pool - pending_pool_change.removed
        + pending_pool_change.added;
    let staking_supply = rewarding_params.interval.staking_supply + pending_pool_change.removed;
    let epoch_reward_budget = reward_pool
        / Decimal::from_atomics(interval.epochs_in_interval(), 0).unwrap()
        * rewarding_params.interval.interval_pool_emission;
    let stake_saturation_point =
        staking_supply / Decimal::from_atomics(rewarding_params.rewarded_set_size, 0).unwrap();

    rewarding_params.interval.reward_pool = reward_pool;
    rewarding_params.interval.staking_supply = staking_supply;
    rewarding_params.interval.epoch_reward_budget = epoch_reward_budget;
    rewarding_params.interval.stake_saturation_point = stake_saturation_point;

    storage::PENDING_REWARD_POOL_CHANGE.save(store, &Default::default())?;
    storage::REWARDING_PARAMS.save(store, &rewarding_params)?;

    Ok(())
}

pub(crate) fn withdraw_operator_reward(
    store: &mut dyn Storage,
    mix_details: MixNodeDetails,
) -> Result<Coin, MixnetContractError> {
    let mix_id = mix_details.mix_id();
    let mut mix_rewarding = mix_details.rewarding_details;
    let original_pledge = mix_details.bond_information.original_pledge;
    let reward = mix_rewarding.withdraw_operator_reward(&original_pledge);

    // save updated rewarding info
    storage::MIXNODE_REWARDING.save(store, mix_id, &mix_rewarding)?;
    Ok(reward)
}

pub(crate) fn withdraw_delegator_reward(
    store: &mut dyn Storage,
    delegation: Delegation,
    mut mix_rewarding: MixNodeRewarding,
) -> Result<Coin, MixnetContractError> {
    let mix_id = delegation.node_id;
    let mut updated_delegation = delegation.clone();
    let reward = mix_rewarding.withdraw_delegator_reward(&mut updated_delegation)?;

    // save updated delegation and mix rewarding info
    delegations_storage::delegations().replace(
        store,
        delegation.storage_key(),
        Some(&updated_delegation),
        Some(&delegation),
    )?;
    storage::MIXNODE_REWARDING.save(store, mix_id, &mix_rewarding)?;
    Ok(reward)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixnodes::helpers::get_mixnode_details_by_id;
    use crate::rewards::models::RewardPoolChange;
    use crate::support::tests::test_helpers::{assert_decimals, performance, TestSetup};
    use cosmwasm_std::Uint128;
    use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;

    #[test]
    fn applying_reward_pool_changes() {
        let mut test = TestSetup::new();

        let epochs_in_interval = test.current_interval().epochs_in_interval();
        let epochs_in_interval_dec = Decimal::from_atomics(epochs_in_interval, 0).unwrap();
        let start_rewarding_params = test.rewarding_params();

        // nothing changes if pending changes are empty
        apply_reward_pool_changes(test.deps_mut().storage).unwrap();
        assert_eq!(start_rewarding_params, test.rewarding_params());

        // normal case of having distributed some rewards
        let distributed_rewards = Decimal::from_atomics(100_000_000u32, 0).unwrap();
        storage::PENDING_REWARD_POOL_CHANGE
            .save(
                test.deps_mut().storage,
                &RewardPoolChange {
                    removed: distributed_rewards,
                    added: Default::default(),
                },
            )
            .unwrap();
        apply_reward_pool_changes(test.deps_mut().storage).unwrap();

        let updated_rewarding_params = test.rewarding_params();

        // updates reward pool
        assert_eq!(
            updated_rewarding_params.interval.reward_pool,
            start_rewarding_params.interval.reward_pool - distributed_rewards
        );

        // updates staking supply
        assert_eq!(
            updated_rewarding_params.interval.staking_supply,
            start_rewarding_params.interval.staking_supply + distributed_rewards
        );

        // updates epoch rewarding budget
        assert_eq!(
            updated_rewarding_params.interval.epoch_reward_budget,
            updated_rewarding_params.interval.reward_pool / epochs_in_interval_dec
                * updated_rewarding_params.interval.interval_pool_emission
        );

        // updates stake saturation point
        assert_eq!(
            updated_rewarding_params.interval.stake_saturation_point,
            updated_rewarding_params.interval.staking_supply
                / Decimal::from_atomics(updated_rewarding_params.rewarded_set_size, 0).unwrap()
        );

        // resets changes back to 0
        assert_eq!(
            storage::PENDING_REWARD_POOL_CHANGE
                .load(test.deps().storage)
                .unwrap(),
            Default::default()
        );

        // future case of having to also increase the reward pool
        let added_credentials = Decimal::from_atomics(50_000_000u32, 0).unwrap();
        storage::PENDING_REWARD_POOL_CHANGE
            .save(
                test.deps_mut().storage,
                &RewardPoolChange {
                    removed: distributed_rewards,
                    added: added_credentials,
                },
            )
            .unwrap();
        apply_reward_pool_changes(test.deps_mut().storage).unwrap();

        let updated_rewarding_params2 = test.rewarding_params();

        // updates reward pool
        assert_eq!(
            updated_rewarding_params2.interval.reward_pool,
            updated_rewarding_params.interval.reward_pool - distributed_rewards + added_credentials
        );

        // updates staking supply
        assert_eq!(
            updated_rewarding_params2.interval.staking_supply,
            updated_rewarding_params.interval.staking_supply + distributed_rewards
        );

        // updates epoch rewarding budget
        assert_eq!(
            updated_rewarding_params2.interval.epoch_reward_budget,
            updated_rewarding_params2.interval.reward_pool / epochs_in_interval_dec
                * updated_rewarding_params2.interval.interval_pool_emission
        );

        // updates stake saturation point
        assert_eq!(
            updated_rewarding_params2.interval.stake_saturation_point,
            updated_rewarding_params2.interval.staking_supply
                / Decimal::from_atomics(updated_rewarding_params2.rewarded_set_size, 0).unwrap()
        );

        // resets changes back to 0
        assert_eq!(
            storage::PENDING_REWARD_POOL_CHANGE
                .load(test.deps().storage)
                .unwrap(),
            Default::default()
        );
    }

    #[test]
    fn withdrawing_operator_reward() {
        let mut test = TestSetup::new();

        let pledge = Uint128::new(250_000_000);
        let pledge_dec = Decimal::from_atomics(250_000_000u32, 0).unwrap();
        let mix_id = test.add_dummy_mixnode("mix-owner", Some(pledge));

        // no rewards
        let mix_details = get_mixnode_details_by_id(test.deps().storage, mix_id)
            .unwrap()
            .unwrap();
        let res = withdraw_operator_reward(test.deps_mut().storage, mix_details).unwrap();
        assert_eq!(res.amount, Uint128::zero());

        test.skip_to_next_epoch_end();
        test.update_rewarded_set(vec![mix_id]);
        let dist1 = test.reward_with_distribution(mix_id, performance(100.0));

        test.skip_to_next_epoch_end();
        let dist2 = test.reward_with_distribution(mix_id, performance(100.0));

        test.skip_to_next_epoch_end();
        let dist3 = test.reward_with_distribution(mix_id, performance(100.0));

        let mix_details = get_mixnode_details_by_id(test.deps().storage, mix_id)
            .unwrap()
            .unwrap();
        let res = withdraw_operator_reward(test.deps_mut().storage, mix_details).unwrap();
        assert_eq!(
            res.amount,
            truncate_reward_amount(dist1.operator + dist2.operator + dist3.operator)
        );
        let updated = test.mix_rewarding(mix_id);
        assert_eq!(updated.operator, pledge_dec)
    }

    #[test]
    fn withdrawing_delegator_reward() {
        let mut test = TestSetup::new();

        let delegation_amount = Uint128::new(2500_000_000);
        let delegation_dec = Decimal::from_atomics(2500_000_000u32, 0).unwrap();
        let mix_id = test.add_dummy_mixnode("mix-owner", None);
        let delegator = "delegator";
        test.add_immediate_delegation(delegator, delegation_amount, mix_id);

        // no rewards
        let delegation = test.delegation(mix_id, delegator, &None);
        let mix_rewarding = test.mix_rewarding(mix_id);
        let res =
            withdraw_delegator_reward(test.deps_mut().storage, delegation, mix_rewarding).unwrap();
        assert_eq!(res.amount, Uint128::zero());

        test.skip_to_next_epoch_end();
        test.update_rewarded_set(vec![mix_id]);
        let dist1 = test.reward_with_distribution(mix_id, performance(100.0));

        test.skip_to_next_epoch_end();
        let dist2 = test.reward_with_distribution(mix_id, performance(100.0));

        test.skip_to_next_epoch_end();
        let dist3 = test.reward_with_distribution(mix_id, performance(100.0));

        let delegation_pre = test.delegation(mix_id, delegator, &None);
        let mix_rewarding = test.mix_rewarding(mix_id);
        let res = withdraw_delegator_reward(
            test.deps_mut().storage,
            delegation_pre.clone(),
            mix_rewarding,
        )
        .unwrap();
        let reward = dist1.delegates + dist2.delegates + dist3.delegates;
        assert_eq!(res.amount, truncate_reward_amount(reward));
        let updated = test.mix_rewarding(mix_id);
        assert_decimals(updated.delegates, delegation_dec);

        let delegation_post = test.delegation(mix_id, delegator, &None);
        assert_ne!(
            delegation_pre.cumulative_reward_ratio,
            delegation_post.cumulative_reward_ratio
        )
    }
}

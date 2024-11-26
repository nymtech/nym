// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::delegations::storage as delegations_storage;
use crate::interval::storage as interval_storage;
use crate::nodes::storage::read_assigned_roles;
use cosmwasm_std::{Coin, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::helpers::{IntoBaseDecimal, NodeBond, NodeDetails};
use mixnet_contract_common::mixnode::NodeRewarding;
use mixnet_contract_common::nym_node::Role;
use mixnet_contract_common::{Delegation, EpochState, EpochStatus, NodeId};

pub(crate) fn update_and_save_last_rewarded(
    storage: &mut dyn Storage,
    mut current_epoch_status: EpochStatus,
    new_last_rewarded: NodeId,
) -> Result<(), MixnetContractError> {
    let is_done = current_epoch_status.update_last_rewarded(new_last_rewarded)?;
    if is_done {
        current_epoch_status.state = EpochState::ReconcilingEvents
    }
    interval_storage::save_current_epoch_status(storage, &current_epoch_status)?;
    Ok(())
}

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
    let staking_supply = rewarding_params.interval.staking_supply
        + rewarding_params.interval.staking_supply_scale_factor * pending_pool_change.removed;
    let epoch_reward_budget = reward_pool / interval.epochs_in_interval().into_base_decimal()?
        * rewarding_params.interval.interval_pool_emission;
    let stake_saturation_point =
        staking_supply / rewarding_params.rewarded_set_size().into_base_decimal()?;

    rewarding_params.interval.reward_pool = reward_pool;
    rewarding_params.interval.staking_supply = staking_supply;
    rewarding_params.interval.epoch_reward_budget = epoch_reward_budget;
    rewarding_params.interval.stake_saturation_point = stake_saturation_point;

    storage::PENDING_REWARD_POOL_CHANGE.save(store, &Default::default())?;
    storage::REWARDING_PARAMS.save(store, &rewarding_params)?;

    Ok(())
}

pub(crate) fn withdraw_operator_reward<D>(
    store: &mut dyn Storage,
    node_details: D,
) -> Result<Coin, MixnetContractError>
where
    D: NodeDetails,
{
    let (bond_info, mut node_rewarding, _) = node_details.split();
    let node_id = bond_info.node_id();
    let original_pledge = bond_info.original_pledge();
    let reward = node_rewarding.withdraw_operator_reward(original_pledge)?;

    // save updated rewarding info
    storage::NYMNODE_REWARDING.save(store, node_id, &node_rewarding)?;
    Ok(reward)
}

pub(crate) fn withdraw_delegator_reward(
    store: &mut dyn Storage,
    delegation: Delegation,
    mut mix_rewarding: NodeRewarding,
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

pub(crate) fn ensure_assignment(
    storage: &dyn Storage,
    node_id: NodeId,
    role: Role,
) -> Result<(), MixnetContractError> {
    // that's a bit expensive to read the whole thing each time, but I'm not sure if there's a much better way
    // (creating a reverse map would be more expensive in the long run due to writes being more costly than reads)
    let assignment = read_assigned_roles(storage, role)?;
    if !assignment.contains(&node_id) {
        return Err(MixnetContractError::IncorrectEpochRole { node_id, role });
    }
    Ok(())
}

// this is **ONLY** to be used in queries
// unless a better way can be figured out
pub(crate) fn expensive_role_lookup(
    storage: &dyn Storage,
    node_id: NodeId,
) -> Result<Option<Role>, MixnetContractError> {
    if ensure_assignment(storage, node_id, Role::EntryGateway).is_ok() {
        return Ok(Some(Role::EntryGateway));
    }
    if ensure_assignment(storage, node_id, Role::ExitGateway).is_ok() {
        return Ok(Some(Role::ExitGateway));
    }
    if ensure_assignment(storage, node_id, Role::Layer1).is_ok() {
        return Ok(Some(Role::Layer1));
    }
    if ensure_assignment(storage, node_id, Role::Layer2).is_ok() {
        return Ok(Some(Role::Layer2));
    }
    if ensure_assignment(storage, node_id, Role::Layer3).is_ok() {
        return Ok(Some(Role::Layer3));
    }
    if ensure_assignment(storage, node_id, Role::Standby).is_ok() {
        return Ok(Some(Role::Standby));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixnodes::helpers::get_mixnode_details_by_id;
    use crate::rewards::models::RewardPoolChange;
    use crate::support::tests::test_helpers::{assert_decimals, TestSetup};
    use cosmwasm_std::Uint128;
    use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;

    #[test]
    fn applying_reward_pool_changes() {
        let mut test = TestSetup::new();

        let epochs_in_interval = test.current_interval().epochs_in_interval();
        let epochs_in_interval_dec = epochs_in_interval.into_base_decimal().unwrap();
        let start_rewarding_params = test.rewarding_params();

        // nothing changes if pending changes are empty
        apply_reward_pool_changes(test.deps_mut().storage).unwrap();
        assert_eq!(start_rewarding_params, test.rewarding_params());

        // normal case of having distributed some rewards
        let distributed_rewards = 100_000_000u32.into_base_decimal().unwrap();
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
                / updated_rewarding_params.dec_rewarded_set_size()
        );

        // resets changes back to 0
        assert_eq!(
            storage::PENDING_REWARD_POOL_CHANGE
                .load(test.deps().storage)
                .unwrap(),
            Default::default()
        );

        // future case of having to also increase the reward pool
        let added_credentials = 50_000_000u32.into_base_decimal().unwrap();
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
                / updated_rewarding_params2.dec_rewarded_set_size()
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
        let pledge_dec = 250_000_000u32.into_base_decimal().unwrap();
        let mix_id = test.add_legacy_mixnode("mix-owner", Some(pledge));
        let active_params = test.active_node_params(100.0);

        // no rewards
        let mix_details = get_mixnode_details_by_id(test.deps().storage, mix_id)
            .unwrap()
            .unwrap();
        let res = withdraw_operator_reward(test.deps_mut().storage, mix_details).unwrap();
        assert_eq!(res.amount, Uint128::zero());

        test.skip_to_next_epoch_end();
        test.force_change_mix_rewarded_set(vec![mix_id]);
        let dist1 = test.reward_with_distribution_ignore_state(mix_id, active_params);

        test.skip_to_next_epoch_end();
        let dist2 = test.reward_with_distribution_ignore_state(mix_id, active_params);

        test.skip_to_next_epoch_end();
        let dist3 = test.reward_with_distribution_ignore_state(mix_id, active_params);

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
        let active_params = test.active_node_params(100.0);

        let delegation_amount = Uint128::new(2_500_000_000);
        let delegation_dec = 2_500_000_000_u32.into_base_decimal().unwrap();
        let mix_id = test.add_legacy_mixnode("mix-owner", None);
        let delegator = "delegator";
        test.add_immediate_delegation(delegator, delegation_amount, mix_id);

        // no rewards
        let delegation = test.delegation(mix_id, delegator, &None);
        let mix_rewarding = test.mix_rewarding(mix_id);
        let res =
            withdraw_delegator_reward(test.deps_mut().storage, delegation, mix_rewarding).unwrap();
        assert_eq!(res.amount, Uint128::zero());

        test.skip_to_next_epoch_end();
        test.force_change_mix_rewarded_set(vec![mix_id]);
        let dist1 = test.reward_with_distribution_ignore_state(mix_id, active_params);

        test.skip_to_next_epoch_end();
        let dist2 = test.reward_with_distribution_ignore_state(mix_id, active_params);

        test.skip_to_next_epoch_end();
        let dist3 = test.reward_with_distribution_ignore_state(mix_id, active_params);

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

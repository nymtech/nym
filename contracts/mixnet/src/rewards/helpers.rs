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
    mix_rewarding.withdraw_delegator_reward(&mut updated_delegation)?;

    // save updated delegation and mix rewarding info
    delegations_storage::delegations().replace(
        store,
        delegation.storage_key(),
        Some(&updated_delegation),
        Some(&delegation),
    )?;
    storage::MIXNODE_REWARDING.save(store, mix_id, &mix_rewarding)?;

    todo!()
}

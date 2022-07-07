// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::interval::storage as interval_storage;
use cosmwasm_std::{Decimal, Storage};
use mixnet_contract_common::error::MixnetContractError;

/// Recomputes rewarding parameters (such as staking supply, saturation point, etc) based on
/// pending changes currently stored in `PENDING_REWARD_POOL_CHANGE`.
pub(crate) fn recompute_interval_rewarding_params(
    store: &mut dyn Storage,
) -> Result<(), MixnetContractError> {
    let mut rewarding_params = storage::REWARDING_PARAMS.load(store)?;
    let mut pending_pool_change = storage::PENDING_REWARD_POOL_CHANGE.load(store)?;
    let interval = interval_storage::current_interval(store)?;

    let reward_pool = rewarding_params.interval.reward_pool - pending_pool_change.removed
        + pending_pool_change.added;
    let staking_supply = rewarding_params.interval.staking_supply + pending_pool_change.removed;
    let epoch_reward_budget = reward_pool
        / Decimal::from_atomics(interval.epochs_in_interval(), 0).unwrap()
        * rewarding_params.interval.interval_pool_emission;
    let stake_saturation_point = staking_supply
        / Decimal::from_atomics(rewarding_params.epoch.rewarded_set_size, 0).unwrap();

    rewarding_params.interval.reward_pool = reward_pool;
    rewarding_params.interval.staking_supply = staking_supply;
    rewarding_params.interval.epoch_reward_budget = epoch_reward_budget;
    rewarding_params.interval.stake_saturation_point = stake_saturation_point;

    storage::PENDING_REWARD_POOL_CHANGE.save(store, &Default::default())?;
    storage::REWARDING_PARAMS.save(store, &rewarding_params)?;

    Ok(())
}

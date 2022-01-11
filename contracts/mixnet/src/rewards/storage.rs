// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use config::defaults::TOTAL_SUPPLY;
use cosmwasm_std::{StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map, U32Key};
use mixnet_contract_common::{IdentityKey, RewardingStatus};

pub(crate) const REWARD_POOL: Item<Uint128> = Item::new("pool");
pub(crate) const REWARDING_STATUS: Map<(U32Key, IdentityKey), RewardingStatus> = Map::new("rm");

// approximately 1 day (assuming 5s per block)
pub(crate) const MINIMUM_BLOCK_AGE_FOR_REWARDING: u64 = 17280;

// approximately 30min (assuming 5s per block)
pub(crate) const MAX_REWARDING_DURATION_IN_BLOCKS: u64 = 360;

#[allow(dead_code)]
pub fn incr_reward_pool(
    amount: Uint128,
    storage: &mut dyn Storage,
) -> Result<Uint128, ContractError> {
    REWARD_POOL.update(storage, |mut current_pool| {
        current_pool += amount;
        Ok(current_pool)
    })
}

pub fn decr_reward_pool(
    storage: &mut dyn Storage,
    amount: Uint128,
) -> Result<Uint128, ContractError> {
    REWARD_POOL.update(storage, |current_pool| {
        let stake = current_pool
            .checked_sub(amount)
            .map_err(|_| ContractError::OutOfFunds {
                to_remove: amount.u128(),
                reward_pool: current_pool.u128(),
            })?;

        Ok(stake)
    })
}

pub fn circulating_supply(storage: &dyn Storage) -> StdResult<Uint128> {
    let reward_pool = REWARD_POOL.load(storage)?;
    Ok(Uint128::new(TOTAL_SUPPLY) - reward_pool)
}

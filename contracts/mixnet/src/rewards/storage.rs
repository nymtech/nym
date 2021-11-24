// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::INITIAL_REWARD_POOL;
use crate::error::ContractError;
use config::defaults::TOTAL_SUPPLY;
use cosmwasm_std::{Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

const REWARD_POOL_PREFIX: &[u8] = b"pool";

// approximately 1 day (assuming 5s per block)
pub(crate) const MINIMUM_BLOCK_AGE_FOR_REWARDING: u64 = 17280;

// approximately 30min (assuming 5s per block)
pub(crate) const MAX_REWARDING_DURATION_IN_BLOCKS: u64 = 360;

fn reward_pool(storage: &dyn Storage) -> ReadonlySingleton<Uint128> {
    singleton_read(storage, REWARD_POOL_PREFIX)
}

pub fn mut_reward_pool(storage: &mut dyn Storage) -> Singleton<Uint128> {
    singleton(storage, REWARD_POOL_PREFIX)
}

pub fn reward_pool_value(storage: &dyn Storage) -> Uint128 {
    match reward_pool(storage).load() {
        Ok(value) => value,
        Err(_e) => Uint128(INITIAL_REWARD_POOL),
    }
}

#[allow(dead_code)]
pub fn incr_reward_pool(
    amount: Uint128,
    storage: &mut dyn Storage,
) -> Result<Uint128, ContractError> {
    let stake = reward_pool_value(storage).saturating_add(amount);
    mut_reward_pool(storage).save(&stake)?;
    Ok(stake)
}

pub fn decr_reward_pool(
    amount: Uint128,
    storage: &mut dyn Storage,
) -> Result<Uint128, ContractError> {
    let stake = match reward_pool_value(storage).checked_sub(amount) {
        Ok(stake) => stake,
        Err(_e) => {
            return Err(ContractError::OutOfFunds {
                to_remove: amount.u128(),
                reward_pool: reward_pool_value(storage).u128(),
            })
        }
    };
    mut_reward_pool(storage).save(&stake)?;
    Ok(stake)
}

pub fn circulating_supply(storage: &dyn Storage) -> Uint128 {
    let reward_pool = reward_pool_value(storage).u128();
    Uint128(TOTAL_SUPPLY - reward_pool)
}

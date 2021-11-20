// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use crate::contract::INITIAL_REWARD_POOL;
use crate::mixnet_params::state::State;

use crate::{error::ContractError};
use config::defaults::TOTAL_SUPPLY;
use cosmwasm_std::{StdResult, Storage, Uint128};
use cosmwasm_storage::{
    singleton, singleton_read, ReadonlySingleton,
    Singleton,
};

use mixnet_contract::{
    Layer, LayerDistribution, StateParams,
};



// storage prefixes
// all of them must be unique and presumably not be a prefix of a different one
// keeping them as short as possible is also desirable as they are part of each stored key
// it's not as important for singletons, but is a nice optimisation for buckets

// singletons
const CONFIG_KEY: &[u8] = b"config";
const LAYER_DISTRIBUTION_KEY: &[u8] = b"layers";
const REWARD_POOL_PREFIX: &[u8] = b"pool";

// buckets
pub const PREFIX_MIXNODES: &[u8] = b"mn";
pub const PREFIX_MIXNODES_OWNERS: &[u8] = b"mo";
pub const PREFIX_GATEWAYS: &[u8] = b"gt";
pub const PREFIX_GATEWAYS_OWNERS: &[u8] = b"go";

pub const PREFIX_MIX_DELEGATION: &[u8] = b"md";
pub const PREFIX_REVERSE_MIX_DELEGATION: &[u8] = b"dm";

pub const PREFIX_REWARDED_MIXNODES: &[u8] = b"rm";

// Contract-level stuff

// TODO Unify bucket and mixnode storage functions

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, CONFIG_KEY)
}

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

pub(crate) fn read_state_params(storage: &dyn Storage) -> StateParams {
    // note: In any other case, I wouldn't have attempted to unwrap this result, but in here
    // if we fail to load the stored state we would already be in the undefined behaviour land,
    // so we better just blow up immediately.
    config_read(storage).load().unwrap().params
}

pub fn layer_distribution(storage: &mut dyn Storage) -> Singleton<LayerDistribution> {
    singleton(storage, LAYER_DISTRIBUTION_KEY)
}

pub fn layer_distribution_read(storage: &dyn Storage) -> ReadonlySingleton<LayerDistribution> {
    singleton_read(storage, LAYER_DISTRIBUTION_KEY)
}

pub(crate) fn read_layer_distribution(storage: &dyn Storage) -> LayerDistribution {
    // note: In any other case, I wouldn't have attempted to unwrap this result, but in here
    // if we fail to load the stored state we would already be in the undefined behaviour land,
    // so we better just blow up immediately.
    layer_distribution_read(storage).load().unwrap()
}

pub fn increment_layer_count(storage: &mut dyn Storage, layer: Layer) -> StdResult<()> {
    let mut distribution = layer_distribution(storage).load()?;
    match layer {
        Layer::Gateway => distribution.gateways += 1,
        Layer::One => distribution.layer1 += 1,
        Layer::Two => distribution.layer2 += 1,
        Layer::Three => distribution.layer3 += 1,
    }
    layer_distribution(storage).save(&distribution)
}

pub fn decrement_layer_count(storage: &mut dyn Storage, layer: Layer) -> StdResult<()> {
    let mut distribution = layer_distribution(storage).load()?;
    // It can't possibly go below zero, if it does, it means there's a serious error in the contract logic
    match layer {
        Layer::Gateway => {
            distribution.gateways = distribution
                .gateways
                .checked_sub(1)
                .expect("tried to subtract from unsigned zero!")
        }
        Layer::One => {
            distribution.layer1 = distribution
                .layer1
                .checked_sub(1)
                .expect("tried to subtract from unsigned zero!")
        }
        Layer::Two => {
            distribution.layer2 = distribution
                .layer2
                .checked_sub(1)
                .expect("tried to subtract from unsigned zero!")
        }
        Layer::Three => {
            distribution.layer3 = distribution
                .layer3
                .checked_sub(1)
                .expect("tried to subtract from unsigned zero!")
        }
    };
    layer_distribution(storage).save(&distribution)
}

#[cfg(test)]
mod tests {
    
    
    
    
    
    
    
}

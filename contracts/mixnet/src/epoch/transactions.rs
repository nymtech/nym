// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::error::ContractError;
use cosmwasm_std::{Env, Order, Response, StdResult, Storage};
use mixnet_contract_common::{Epoch, IdentityKey, NodeStatus};
use std::collections::HashMap;

pub fn try_write_rewarded_set(
    rewarded_set: HashMap<IdentityKey, NodeStatus>,
    storage: &mut dyn Storage,
    env: Env,
) -> Result<Response, ContractError> {
    let current_epoch = storage::CURRENT_EPOCH.load(storage)?.id();
    let block_height = env.block.height;
    for (key, value) in rewarded_set {
        storage::REWARDED_SET.save(storage, (block_height, key), &value)?;
    }
    storage::REWARDED_SET_HEIGHTS_FOR_EPOCH.save(storage, (current_epoch, block_height), &())?;
    Ok(Response::default())
}

pub fn try_clear_rewarded_set(storage: &mut dyn Storage) -> Result<Response, ContractError> {
    let keys: StdResult<Vec<(u64, String)>> = storage::REWARDED_SET
        .keys_de(storage, None, None, Order::Ascending)
        .collect();
    for key in keys? {
        storage::REWARDED_SET.remove(storage, key)
    }
    Ok(Response::default())
}

pub fn try_set_current_epoch(
    epoch: Epoch,
    storage: &mut dyn Storage,
) -> Result<Response, ContractError> {
    storage::CURRENT_EPOCH.save(storage, &epoch)?;
    Ok(Response::default())
}

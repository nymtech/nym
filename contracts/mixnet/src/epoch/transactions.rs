// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::error::ContractError;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Order, Response, StdResult, Storage};
use mixnet_contract_common::events::new_change_rewarded_set_event;
use mixnet_contract_common::{Epoch, IdentityKey, RewardedSetNodeStatus};
use std::collections::HashMap;

pub fn try_write_rewarded_set(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    rewarded_set: Vec<IdentityKey>,
    active_set_size: u32,
) -> Result<Response, ContractError> {
    let state = mixnet_params_storage::CONTRACT_STATE.load(deps.storage)?;

    // check if this is executed by the permitted validator, if not reject the transaction
    if info.sender != state.rewarding_validator_address {
        return Err(ContractError::Unauthorized);
    }

    // sanity check to make sure the sending validator is in sync with the contract state
    // (i.e. so that we'd known that top k nodes are actually expected to be active)
    if active_set_size != state.params.mixnode_active_set_size {
        return Err(ContractError::UnexpectedActiveSetSize {
            received: active_set_size,
            expected: state.params.mixnode_active_set_size,
        });
    }

    if rewarded_set.len() as u32 > state.params.mixnode_rewarded_set_size {
        return Err(ContractError::UnexpectedRewardedSetSize {
            received: rewarded_set.len() as u32,
            expected: state.params.mixnode_rewarded_set_size,
        });
    }

    let current_epoch = storage::CURRENT_EPOCH.load(deps.storage)?.id();
    let block_height = env.block.height;
    let num_nodes = rewarded_set.len();

    storage::save_rewarded_set(deps.storage, block_height, active_set_size, rewarded_set)?;
    storage::REWARDED_SET_HEIGHTS_FOR_EPOCH.save(
        deps.storage,
        (current_epoch, block_height),
        &0u8,
    )?;

    Ok(Response::new().add_event(new_change_rewarded_set_event(
        state.params.mixnode_active_set_size,
        state.params.mixnode_rewarded_set_size,
        num_nodes as u32,
        current_epoch,
    )))
}

// pub fn try_clear_rewarded_set(storage: &mut dyn Storage) -> Result<Response, ContractError> {
//     let keys: StdResult<Vec<(u64, String)>> = storage::REWARDED_SET
//         .keys_de(storage, None, None, Order::Ascending)
//         .collect();
//     for key in keys? {
//         storage::REWARDED_SET.remove(storage, key)
//     }
//     Ok(Response::default())
// }

pub fn try_set_current_epoch(
    epoch: Epoch,
    storage: &mut dyn Storage,
) -> Result<Response, ContractError> {
    storage::CURRENT_EPOCH.save(storage, &epoch)?;
    Ok(Response::default())
}

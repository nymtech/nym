// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::error::ContractError;
use crate::error::ContractError::EpochNotInProgress;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdResult, Storage};
use mixnet_contract_common::events::{
    new_advance_epoch_event, new_change_rewarded_set_event, new_set_current_epoch_event,
};
use mixnet_contract_common::{Epoch, IdentityKey};

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

    let last_update = storage::CURRENT_REWARDED_SET_HEIGHT.load(deps.storage)?;
    let block_height = env.block.height;

    if last_update + (crate::contract::REWARDED_SET_REFRESH_BLOCKS as u64) > block_height {
        return Err(ContractError::TooFrequentRewardedSetUpdate {
            last_update,
            minimum_delay: crate::contract::REWARDED_SET_REFRESH_BLOCKS,
            current_height: block_height,
        });
    }

    let current_epoch = storage::CURRENT_EPOCH.load(deps.storage)?.id();
    let num_nodes = rewarded_set.len();

    storage::save_rewarded_set(deps.storage, block_height, active_set_size, rewarded_set)?;
    storage::REWARDED_SET_HEIGHTS_FOR_EPOCH.save(
        deps.storage,
        (current_epoch, block_height),
        &0u8,
    )?;
    storage::CURRENT_REWARDED_SET_HEIGHT.save(deps.storage, &block_height)?;

    Ok(Response::new().add_event(new_change_rewarded_set_event(
        state.params.mixnode_active_set_size,
        state.params.mixnode_rewarded_set_size,
        num_nodes as u32,
        current_epoch,
    )))
}

pub fn try_set_current_epoch(
    env: Env,
    storage: &mut dyn Storage,
) -> Result<Response, ContractError> {
    let current_stored = storage::CURRENT_EPOCH.load(storage)?;
    let new_current = current_stored.current_with_timestamp(env.block.time.seconds() as i64);

    if new_current != current_stored {
        storage::CURRENT_EPOCH.save(storage, &new_current)?;
    }

    Ok(Response::new().add_event(new_set_current_epoch_event(new_current)))
}

pub fn try_advance_epoch(env: Env, storage: &mut dyn Storage) -> Result<Response, ContractError> {
    let new_epoch = storage::advance_epoch(storage)?;

    if !new_epoch.contains_timestamp(env.block.time.seconds() as i64) {
        // the reason for this check is as follows:
        // nobody, even trusted validators, should be able to continuously keep advancing epochs,
        // because otherwise it would be possible for them to continuously keep rewarding nodes.
        //
        // Therefore, even if "trusted" validator, responsible for rewarding, is malicious,
        // they can't send rewards more often than every `REWARDED_SET_REFRESH_BLOCKS`
        // and changing this value requires going through governance and having agreement of
        // the super-majority of the validators (by stake)
        return Err(EpochNotInProgress {
            current_block_time: env.block.time.seconds(),
            epoch_start: new_epoch.start_unix_timestamp(),
            epoch_end: new_epoch.end_unix_timestamp(),
        });
    }

    Ok(Response::new().add_event(new_advance_epoch_event(new_epoch)))
}

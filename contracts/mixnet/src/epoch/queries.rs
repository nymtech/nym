// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::error::ContractError;
use cosmwasm_std::{Env, Order, StdResult, Storage};
use cw_storage_plus::Bound;
use mixnet_contract_common::{
    Epoch, EpochRewardedSetHeightsResponse, IdentityKey, PagedRewardedSetResponse,
    RewardedSetNodeStatus, RewardedSetUpdateDetails,
};

pub fn query_current_epoch(storage: &dyn Storage) -> Result<Epoch, ContractError> {
    Ok(storage::CURRENT_EPOCH.load(storage)?)
}

pub(crate) fn query_rewarded_set_refresh_minimum_blocks() -> u32 {
    crate::contract::REWARDED_SET_REFRESH_BLOCKS
}

pub fn query_rewarded_set_heights_for_epoch(
    storage: &dyn Storage,
    epoch_id: u32,
) -> Result<EpochRewardedSetHeightsResponse, ContractError> {
    // I don't think we have to deal with paging here as at most we're going to have 720 values here
    // and I think the validators are capable of performing 720 storage reads at once if they're only
    // reading u64 (+ u8) values...
    let heights = storage::REWARDED_SET_HEIGHTS_FOR_EPOCH
        .prefix(epoch_id)
        .range(storage, None, None, Order::Ascending)
        .map(|val| val.map(|(height, _)| height))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(EpochRewardedSetHeightsResponse { epoch_id, heights })
}

// note: I have removed the `query_rewarded_set_for_epoch`, because I don't think it's appropriate
// for the contract to go through so much data (i.e. all "rewarded" sets of particular epoch) in one go.
// To achieve the same result, the client would have to instead first call `query_rewarded_set_heights_for_epoch`
// to learn the heights used in given epoch and then for each of them `query_rewarded_set` for that particular height.

pub fn query_current_rewarded_set_height(storage: &dyn Storage) -> Result<u64, ContractError> {
    Ok(storage::CURRENT_REWARDED_SET_HEIGHT.load(storage)?)
}

fn query_rewarded_set_at_height(
    storage: &dyn Storage,
    height: u64,
    start_after: Option<IdentityKey>,
    limit: u32,
) -> Result<Vec<(IdentityKey, RewardedSetNodeStatus)>, ContractError> {
    let start = start_after.map(Bound::exclusive);

    let rewarded_set = storage::REWARDED_SET
        .prefix(height)
        .range(storage, start, None, Order::Ascending)
        .take(limit as usize)
        .collect::<StdResult<_>>()?;
    Ok(rewarded_set)
}

pub fn query_rewarded_set(
    storage: &dyn Storage,
    height: Option<u64>,
    start_after: Option<IdentityKey>,
    limit: Option<u32>,
) -> Result<PagedRewardedSetResponse, ContractError> {
    let height = match height {
        Some(height) => height,
        None => query_current_rewarded_set_height(storage)?,
    };
    let limit = limit
        .unwrap_or(storage::REWARDED_NODE_DEFAULT_LIMIT)
        .min(storage::REWARDED_NODE_MAX_LIMIT);
    let paged_result = query_rewarded_set_at_height(storage, height, start_after, limit)?;

    let start_next_after = if paged_result.len() > limit as usize {
        paged_result.last().map(|res| res.0.clone())
    } else {
        None
    };

    Ok(PagedRewardedSetResponse {
        identities: paged_result,
        start_next_after,
        at_height: height,
    })
}

// this was all put together into the same query so that all information would be synced together
pub fn query_rewarded_set_update_details(
    env: Env,
    storage: &dyn Storage,
) -> Result<RewardedSetUpdateDetails, ContractError> {
    Ok(RewardedSetUpdateDetails {
        refresh_rate_blocks: query_rewarded_set_refresh_minimum_blocks(),
        last_refreshed_block: query_current_rewarded_set_height(storage)?,
        current_height: env.block.height,
    })
}

// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::error::ContractError;
use cosmwasm_std::{Order, StdResult, Storage};
use cw_storage_plus::Bound;
use mixnet_contract_common::{Epoch, IdentityKey, PagedRewardedSetResponse, RewardedSetNodeStatus};
use std::collections::{HashMap, HashSet};

pub fn query_current_epoch(storage: &dyn Storage) -> Result<Epoch, ContractError> {
    Ok(storage::CURRENT_EPOCH.load(storage)?)
}

pub(crate) fn query_rewarded_set_refresh_minimum_blocks() -> u32 {
    crate::contract::REWARDED_SET_REFRESH_BLOCKS
}

pub fn query_rewarded_set_for_epoch(
    epoch: Option<Epoch>,
    filter: Option<RewardedSetNodeStatus>,
    storage: &dyn Storage,
) -> Result<HashSet<IdentityKey>, ContractError> {
    todo!("rethinking this one")

    // let epoch = epoch.unwrap_or(storage::CURRENT_EPOCH.load(storage)?);
    // let heights: Vec<u64> = storage::REWARDED_SET_HEIGHTS_FOR_EPOCH
    //     .prefix_de(epoch.id())
    //     .range_de(storage, None, None, Order::Descending)
    //     .scan((), |_, x| x.ok())
    //     .map(|(height, _)| height)
    //     .collect();
    // let mut rewarded_set = HashSet::new();
    // for height in heights {
    //     let nodes: HashSet<IdentityKey> = storage::REWARDED_SET
    //         .prefix_de(height)
    //         .range_de(storage, None, None, Order::Ascending)
    //         .scan((), |_, x| x.ok())
    //         .filter(|(_identity_key, node_status)| {
    //             filter.is_none() || Some(node_status) == filter.as_ref()
    //         })
    //         .map(|(identity_key, _node_status)| identity_key)
    //         .collect();
    //     rewarded_set = rewarded_set.union(&nodes).map(|x| x.to_owned()).collect();
    // }
    // Ok(rewarded_set)
}

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
        .prefix_de(height)
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

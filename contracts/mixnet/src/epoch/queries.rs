// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::error::ContractError;
use cosmwasm_std::{Order, StdResult, Storage};
use mixnet_contract_common::{Epoch, IdentityKey, NodeStatus};
use std::collections::{HashMap, HashSet};

pub fn query_current_epoch(storage: &dyn Storage) -> Result<Epoch, ContractError> {
    Ok(storage::CURRENT_EPOCH.load(storage)?)
}

pub(crate) fn query_rewarded_set_refresh_secs() -> u32 {
    crate::contract::REWARDED_SET_REFRESH_SECS
}

pub fn query_rewarded_set_for_epoch(
    epoch: Option<Epoch>,
    filter: Option<NodeStatus>,
    storage: &dyn Storage,
) -> Result<HashSet<IdentityKey>, ContractError> {
    let epoch = epoch.unwrap_or(storage::CURRENT_EPOCH.load(storage)?);
    let heights: Vec<u64> = storage::REWARDED_SET_HEIGHTS_FOR_EPOCH
        .prefix_de(epoch.id())
        .range_de(storage, None, None, Order::Descending)
        .scan((), |_, x| x.ok())
        .map(|(height, _)| height)
        .collect();
    let mut rewarded_set = HashSet::new();
    for height in heights {
        let nodes: HashSet<IdentityKey> = storage::REWARDED_SET
            .prefix_de(height)
            .range_de(storage, None, None, Order::Ascending)
            .scan((), |_, x| x.ok())
            .filter(|(_identity_key, node_status)| {
                filter.is_none() || Some(node_status) == filter.as_ref()
            })
            .map(|(identity_key, _node_status)| identity_key)
            .collect();
        rewarded_set = rewarded_set.union(&nodes).map(|x| x.to_owned()).collect();
    }
    Ok(rewarded_set)
}

pub fn query_current_rewarded_set_height(storage: &dyn Storage) -> Result<u64, ContractError> {
    if let Some(Ok(height)) = storage::REWARDED_SET_HEIGHTS_FOR_EPOCH
        .keys_de(storage, None, None, Order::Descending)
        .next()
    {
        Ok(height.1)
    } else {
        Err(ContractError::RewardSetHeightMapEmpty)
    }
}

pub fn query_rewarded_set_at_height(
    height: u64,
    storage: &dyn Storage,
) -> Result<HashMap<IdentityKey, NodeStatus>, ContractError> {
    let rewarded_set: StdResult<Vec<_>> = storage::REWARDED_SET
        .prefix_de(height)
        .range(storage, None, None, Order::Ascending)
        .collect();
    Ok(rewarded_set?.into_iter().collect())
}

pub fn query_rewarded_set(
    storage: &dyn Storage,
) -> Result<HashMap<IdentityKey, NodeStatus>, ContractError> {
    let latest_height = query_current_rewarded_set_height(storage)?;
    query_rewarded_set_at_height(latest_height, storage)
}

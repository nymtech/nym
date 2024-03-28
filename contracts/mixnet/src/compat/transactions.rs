// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnodes::helpers::get_mixnode_details_by_owner;
use crate::mixnodes::transactions::{
    try_decrease_mixnode_pledge, try_increase_mixnode_pledge, try_update_mixnode_cost_params,
};
use crate::nodes::helpers::get_node_details_by_owner;
use crate::nodes::transactions::{
    try_decrease_nym_node_pledge, try_increase_nym_node_pledge, try_update_nym_node_cost_params,
};
use crate::rewards::transactions::{
    try_withdraw_mixnode_operator_reward, try_withdraw_nym_node_operator_reward,
};
use crate::support::helpers::{
    ensure_operating_cost_within_range, ensure_profit_margin_within_range,
};
use cosmwasm_std::{Coin, DepsMut, Env, MessageInfo, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::NodeCostParams;

pub(crate) fn try_increase_pledge(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    // check if owns mixnode or nymnode and change accordingly
    if let Some(nym_node_details) = get_node_details_by_owner(deps.storage, info.sender.clone())? {
        try_increase_nym_node_pledge(deps, env, info.funds, nym_node_details)
    } else if let Some(legacy_mixnode_details) =
        get_mixnode_details_by_owner(deps.storage, info.sender.clone())?
    {
        try_increase_mixnode_pledge(deps, env, info.funds, legacy_mixnode_details)
    } else {
        Err(MixnetContractError::NoAssociatedNodeBond { owner: info.sender })
    }
}

pub fn try_decrease_pledge(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    decrease_by: Coin,
) -> Result<Response, MixnetContractError> {
    // check if owns mixnode or nymnode and change accordingly
    if let Some(nym_node_details) = get_node_details_by_owner(deps.storage, info.sender.clone())? {
        try_decrease_nym_node_pledge(deps, env, decrease_by, nym_node_details)
    } else if let Some(legacy_mixnode_details) =
        get_mixnode_details_by_owner(deps.storage, info.sender.clone())?
    {
        try_decrease_mixnode_pledge(deps, env, decrease_by, legacy_mixnode_details)
    } else {
        Err(MixnetContractError::NoAssociatedNodeBond { owner: info.sender })
    }
}

pub(crate) fn try_update_cost_params(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    new_costs: NodeCostParams,
) -> Result<Response, MixnetContractError> {
    // ensure the profit margin is within the defined range
    ensure_profit_margin_within_range(deps.storage, new_costs.profit_margin_percent)?;

    // ensure the operating cost is within the defined range
    ensure_operating_cost_within_range(deps.storage, &new_costs.interval_operating_cost)?;

    // check if owns mixnode or nymnode and change accordingly
    if let Some(nym_node_details) = get_node_details_by_owner(deps.storage, info.sender.clone())? {
        try_update_nym_node_cost_params(deps, env, new_costs, nym_node_details)
    } else if let Some(legacy_mixnode_details) =
        get_mixnode_details_by_owner(deps.storage, info.sender.clone())?
    {
        try_update_mixnode_cost_params(deps, env, new_costs, legacy_mixnode_details)
    } else {
        Err(MixnetContractError::NoAssociatedNodeBond { owner: info.sender })
    }
}

pub(crate) fn try_withdraw_operator_reward(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    // check if owns mixnode or nymnode and change accordingly
    if let Some(nym_node_details) = get_node_details_by_owner(deps.storage, info.sender.clone())? {
        try_withdraw_nym_node_operator_reward(deps, nym_node_details)
    } else if let Some(legacy_mixnode_details) =
        get_mixnode_details_by_owner(deps.storage, info.sender.clone())?
    {
        try_withdraw_mixnode_operator_reward(deps, legacy_mixnode_details)
    } else {
        Err(MixnetContractError::NoAssociatedNodeBond { owner: info.sender })
    }
}

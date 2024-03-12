// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::gateways::storage::gateways;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::helpers::save_new_mixnode;
use crate::support::helpers::ensure_no_existing_bond;
use cosmwasm_std::{Coin, DepsMut, Env, MessageInfo, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::{Gateway, GatewayBond, MixNode, MixNodeCostParams};

pub fn admin_add_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    mixnode: MixNode,
    cost_params: MixNodeCostParams,
    pledge: Coin,
    owner: String,
) -> Result<Response, MixnetContractError> {
    let state = mixnet_params_storage::CONTRACT_STATE.load(deps.storage)?;

    // check if this is executed by the owner, if not reject the transaction
    if info.sender != state.owner {
        return Err(MixnetContractError::Unauthorized);
    }

    let owner = deps.api.addr_validate(&owner)?;

    ensure_no_existing_bond(&owner, deps.storage)?;

    save_new_mixnode(deps.storage, env, mixnode, cost_params, owner, None, pledge)?;

    Ok(Response::new())
}

pub fn admin_add_gateway(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    gateway: Gateway,
    pledge: Coin,
    owner: String,
) -> Result<Response, MixnetContractError> {
    let state = mixnet_params_storage::CONTRACT_STATE.load(deps.storage)?;

    // check if this is executed by the owner, if not reject the transaction
    if info.sender != state.owner {
        return Err(MixnetContractError::Unauthorized);
    }

    let owner = deps.api.addr_validate(&owner)?;

    ensure_no_existing_bond(&owner, deps.storage)?;

    let bond = GatewayBond::new(pledge, owner, env.block.height, gateway, None);
    gateways().save(deps.storage, bond.identity(), &bond)?;

    Ok(Response::new())
}

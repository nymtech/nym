// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! State-mutating execute handlers. Each entry is currently a stub returning
//! an empty response; concrete implementations will be filled in as the
//! corresponding tickets land.

use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use node_families_contract_common::{Config, NodeFamiliesContractError, NodeFamilyId};
use nym_mixnet_contract_common::NodeId;

pub(crate) fn try_update_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    config: Config,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, config);
    Ok(Response::default())
}

pub(crate) fn try_create_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: String,
    description: String,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, name, description);
    Ok(Response::default())
}

pub(crate) fn try_disband_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info);
    Ok(Response::default())
}

pub(crate) fn try_invite_to_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, node_id);
    Ok(Response::default())
}

pub(crate) fn try_revoke_family_invitation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, node_id);
    Ok(Response::default())
}

pub(crate) fn try_accept_family_invitation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    family_id: NodeFamilyId,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, family_id, node_id);
    Ok(Response::default())
}

pub(crate) fn try_reject_family_invitation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    family_id: NodeFamilyId,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, family_id, node_id);
    Ok(Response::default())
}

pub(crate) fn try_leave_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, node_id);
    Ok(Response::default())
}

pub(crate) fn try_kick_from_family(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, NodeFamiliesContractError> {
    let _ = (deps, env, info, node_id);
    Ok(Response::default())
}

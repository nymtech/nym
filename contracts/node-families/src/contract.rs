// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! CosmWasm entry points for the node families contract.

use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use node_families_contract_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NodeFamiliesContractError, QueryMsg,
};
use nym_contracts_common::set_build_information;

const CONTRACT_NAME: &str = "crate:nym-node-families-contract";

/// Contract semver, taken from `Cargo.toml` at build time. Bumped on every
/// release; recorded in cw2 storage so migrations can detect the source version.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// One-time initialisation of contract storage on code instantiation.
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, NodeFamiliesContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    set_build_information!(deps.storage)?;

    // NYM_POOL_STORAGE.initialise(deps, env, info.sender, &msg.pool_denomination, msg.grants)?;

    Ok(Response::default())
}

/// State-mutating dispatcher. Concrete handlers live in [`crate::transactions`]
/// and are wired up here as variants are added to [`ExecuteMsg`].
#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, NodeFamiliesContractError> {
    todo!()
}

/// Read-only dispatcher. Concrete handlers live in [`crate::queries`] and are
/// wired up here as variants are added to [`QueryMsg`].
#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, NodeFamiliesContractError> {
    todo!()
}

/// Migration entry point.
///
/// Refreshes recorded build information and ensures the existing on-chain
/// contract version is at most the current `CONTRACT_VERSION` (i.e. forbids
/// downgrades). Any data migrations are dispatched via
/// [`crate::queued_migrations`].
#[entry_point]
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, NodeFamiliesContractError> {
    set_build_information!(deps.storage)?;
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Default::default())
}

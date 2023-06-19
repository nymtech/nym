//! The nym name service contract is for users to register names for the nym addresses.

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]

pub use nym_name_service_common::error::{NameServiceError, Result};

use nym_name_service_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response};

mod contract;
mod state;

pub mod constants;

#[cfg(test)]
mod integration_tests;
#[cfg(test)]
mod test_helpers;

/// Contract entry point for instantiation.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response> {
    contract::instantiate(deps, env, info, msg)
}

/// Contract entry point for migrations.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut<'_>, env: Env, msg: MigrateMsg) -> Result<Response, NameServiceError> {
    contract::migrate(deps, env, msg)
}

/// Contract entry point for execution
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, NameServiceError> {
    contract::execute(deps, env, info, msg)
}

/// Contract entry point for queries
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> Result<Binary> {
    contract::query(deps, env, msg)
}

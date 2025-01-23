// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std22::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use nym_pool_contract_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NymPoolContractError, QueryMsg,
};

// need to manually implement all `entry_points` for now since we're importing cosmwasm 2.2 under different name
// and #[entry_point] does not understand it

#[cfg(target_arch = "wasm32")]
mod __wasm_export_instantiate {
    #[no_mangle]
    extern "C" fn instantiate(ptr0: u32, ptr1: u32, ptr2: u32) -> u32 {
        cosmwasm_std22::do_instantiate(&super::instantiate, ptr0, ptr1, ptr2)
    }
}

#[cfg(target_arch = "wasm32")]
mod __wasm_export_execute {
    #[no_mangle]
    extern "C" fn execute(ptr0: u32, ptr1: u32, ptr2: u32) -> u32 {
        cosmwasm_std22::do_execute(&super::execute, ptr0, ptr1, ptr2)
    }
}

#[cfg(target_arch = "wasm32")]
mod __wasm_export_query {
    #[no_mangle]
    extern "C" fn query(ptr0: u32, ptr1: u32) -> u32 {
        cosmwasm_std22::do_query(&super::query, ptr0, ptr1)
    }
}

#[cfg(target_arch = "wasm32")]
mod __wasm_export_migrate {
    #[no_mangle]
    extern "C" fn migrate(ptr0: u32, ptr1: u32) -> u32 {
        cosmwasm_std22::do_migrate(&super::migrate, ptr0, ptr1)
    }
}

// #[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, NymPoolContractError> {
    todo!()
}

// #[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, NymPoolContractError> {
    todo!()
}

// #[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, NymPoolContractError> {
    todo!()
}

// #[entry_point]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, NymPoolContractError> {
    todo!()
}

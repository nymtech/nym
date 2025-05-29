// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::queries::query_admin;
use crate::storage::NYM_PERFORMANCE_CONTRACT_STORAGE;
use crate::transactions::try_update_contract_admin;
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use nym_contracts_common::set_build_information;
use nym_performance_contract_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NymPerformanceContractError, QueryMsg,
};

const CONTRACT_NAME: &str = "crate:nym-performance-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, NymPerformanceContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    set_build_information!(deps.storage)?;

    let _ = msg;

    NYM_PERFORMANCE_CONTRACT_STORAGE.initialise(deps, env, info.sender)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, NymPerformanceContractError> {
    let _ = env;

    match msg {
        ExecuteMsg::UpdateAdmin { admin } => try_update_contract_admin(deps, info, admin),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, NymPerformanceContractError> {
    let _ = env;
    match msg {
        QueryMsg::Admin {} => Ok(to_json_binary(&query_admin(deps)?)?),
    }
}

#[entry_point]
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, NymPerformanceContractError> {
    set_build_information!(deps.storage)?;
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod contract_instantiaton {
        use super::*;
        use crate::storage::NYM_PERFORMANCE_CONTRACT_STORAGE;
        use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};

        #[test]
        fn sets_contract_admin_to_the_message_sender() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let init_msg = InstantiateMsg {};

            let some_sender = deps.api.addr_make("some_sender");
            instantiate(
                deps.as_mut(),
                env,
                message_info(&some_sender, &[]),
                init_msg,
            )?;

            NYM_PERFORMANCE_CONTRACT_STORAGE
                .contract_admin
                .assert_admin(deps.as_ref(), &some_sender)?;

            Ok(())
        }
    }
}

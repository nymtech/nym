// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::queries::query_admin;
use crate::storage::NYM_POOL_STORAGE;
use crate::transactions::try_update_contract_admin;
use cosmwasm_std::{
    entry_point, to_binary, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use nym_contracts_common::set_build_information_cw22;
use nym_pool_contract_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NymPoolContractError, QueryMsg,
};

const CONTRACT_NAME: &str = "crate:nym-pool-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, NymPoolContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    set_build_information_cw22!(deps.storage)?;

    NYM_POOL_STORAGE.initialise(deps, env, info.sender, &msg.pool_denomination, msg.grants)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, NymPoolContractError> {
    match msg {
        ExecuteMsg::UpdateAdmin { admin } => try_update_contract_admin(deps, info, admin),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, NymPoolContractError> {
    match msg {
        QueryMsg::Admin {} => Ok(to_json_binary(&query_admin(deps)?)?),
        QueryMsg::GetAvailableTokens {} => todo!(),
        QueryMsg::GetTotalLockedTokens {} => todo!(),
        QueryMsg::GetLockedTokens { grantee } => todo!(),
        QueryMsg::GetLockedTokensPaged { limit, start_after } => todo!(),
    }
}

#[entry_point]
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, NymPoolContractError> {
    set_build_information_cw22!(deps.storage)?;
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod contract_instantiaton {
        use super::*;
        use crate::storage::NYM_POOL_STORAGE;
        use crate::testing::TEST_DENOM;
        use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};

        #[test]
        fn sets_contract_admin_to_the_message_sender() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let init_msg = InstantiateMsg {
                pool_denomination: TEST_DENOM.to_string(),
                grants: Default::default(),
            };

            let some_sender = deps.api.addr_make("some_sender");
            instantiate(
                deps.as_mut(),
                env,
                message_info(&some_sender, &[]),
                init_msg,
            )?;

            NYM_POOL_STORAGE
                .contract_admin
                .assert_admin(deps.as_ref(), &some_sender)?;

            Ok(())
        }

        #[test]
        fn sets_the_pool_denomination() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let init_msg = InstantiateMsg {
                pool_denomination: "some_denom".to_string(),
                grants: Default::default(),
            };

            let some_sender = deps.api.addr_make("some_sender");
            instantiate(
                deps.as_mut(),
                env,
                message_info(&some_sender, &[]),
                init_msg,
            )?;

            assert_eq!(
                NYM_POOL_STORAGE
                    .pool_denomination
                    .load(deps.as_ref().storage)?,
                "some_denom"
            );

            Ok(())
        }

        #[cfg(test)]
        mod setting_initial_grants {
            use super::*;

            #[test]
            fn with_empty_map() {
                //
            }

            #[test]
            fn with_insufficient_tokens() {
                //
            }

            #[test]
            fn with_valid_request() {
                //
            }
        }

        #[test]
        fn sets_pool_value_to_transferred_tokens() -> anyhow::Result<()> {
            todo!()
        }
    }
}

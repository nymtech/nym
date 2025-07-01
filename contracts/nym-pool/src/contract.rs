// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::queries::{
    query_admin, query_available_tokens, query_grant, query_granter, query_granters_paged,
    query_grants_paged, query_locked_tokens, query_locked_tokens_paged, query_total_locked_tokens,
};
use crate::storage::NYM_POOL_STORAGE;
use crate::transactions::{
    try_add_new_granter, try_grant_allowance, try_lock_allowance, try_remove_expired,
    try_revoke_grant, try_revoke_granter, try_unlock_allowance, try_update_contract_admin,
    try_use_allowance, try_use_locked_allowance, try_withdraw_allowance,
    try_withdraw_locked_allowance,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use nym_contracts_common::set_build_information;
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
    set_build_information!(deps.storage)?;

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
        ExecuteMsg::UpdateAdmin {
            admin,
            update_granter_set,
        } => try_update_contract_admin(deps, env, info, admin, update_granter_set),
        ExecuteMsg::GrantAllowance { grantee, allowance } => {
            try_grant_allowance(deps, env, info, grantee, *allowance)
        }
        ExecuteMsg::RevokeAllowance { grantee } => try_revoke_grant(deps, env, info, grantee),
        ExecuteMsg::UseAllowance { recipients } => try_use_allowance(deps, env, info, recipients),
        ExecuteMsg::WithdrawAllowance { amount } => try_withdraw_allowance(deps, env, info, amount),
        ExecuteMsg::LockAllowance { amount } => try_lock_allowance(deps, env, info, amount),
        ExecuteMsg::UnlockAllowance { amount } => try_unlock_allowance(deps, env, info, amount),
        ExecuteMsg::UseLockedAllowance { recipients } => {
            try_use_locked_allowance(deps, env, info, recipients)
        }
        ExecuteMsg::WithdrawLockedAllowance { amount } => {
            try_withdraw_locked_allowance(deps, env, info, amount)
        }
        ExecuteMsg::AddNewGranter { granter } => try_add_new_granter(deps, env, info, granter),
        ExecuteMsg::RevokeGranter { granter } => try_revoke_granter(deps, env, info, granter),
        ExecuteMsg::RemoveExpiredGrant { grantee } => try_remove_expired(deps, env, info, grantee),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, NymPoolContractError> {
    match msg {
        QueryMsg::Admin {} => Ok(to_json_binary(&query_admin(deps)?)?),
        QueryMsg::GetAvailableTokens {} => Ok(to_json_binary(&query_available_tokens(deps, env)?)?),
        QueryMsg::GetTotalLockedTokens {} => Ok(to_json_binary(&query_total_locked_tokens(deps)?)?),
        QueryMsg::GetLockedTokens { grantee } => {
            Ok(to_json_binary(&query_locked_tokens(deps, grantee)?)?)
        }
        QueryMsg::GetLockedTokensPaged { limit, start_after } => Ok(to_json_binary(
            &query_locked_tokens_paged(deps, limit, start_after)?,
        )?),
        QueryMsg::GetGrant { grantee } => Ok(to_json_binary(&query_grant(deps, env, grantee)?)?),
        QueryMsg::GetGranter { granter } => Ok(to_json_binary(&query_granter(deps, granter)?)?),
        QueryMsg::GetGrantersPaged { limit, start_after } => Ok(to_json_binary(
            &query_granters_paged(deps, limit, start_after)?,
        )?),
        QueryMsg::GetGrantsPaged { limit, start_after } => Ok(to_json_binary(
            &query_grants_paged(deps, env, limit, start_after)?,
        )?),
    }
}

#[entry_point]
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, NymPoolContractError> {
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

        #[test]
        fn adds_sender_to_set_of_initial_granters() -> anyhow::Result<()> {
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

            let granter = query_granter(deps.as_ref(), some_sender.to_string())?;
            assert!(granter.information.is_some());

            Ok(())
        }

        #[cfg(test)]
        mod setting_initial_grants {
            use super::*;
            use cosmwasm_std::{coin, Order, Storage};
            use nym_contracts_common_testing::deps_with_balance;
            use nym_pool_contract_common::{Allowance, BasicAllowance, Grant, GranteeAddress};
            use std::collections::HashMap;

            fn all_grants(storage: &dyn Storage) -> HashMap<GranteeAddress, Grant> {
                NYM_POOL_STORAGE
                    .grants
                    .range(storage, None, None, Order::Ascending)
                    .collect::<Result<HashMap<_, _>, _>>()
                    .unwrap()
            }

            #[test]
            fn with_empty_map() -> anyhow::Result<()> {
                let mut deps = mock_dependencies();
                let env = mock_env();
                let grants = HashMap::new();
                let init_msg = InstantiateMsg {
                    pool_denomination: TEST_DENOM.to_string(),
                    grants,
                };

                let some_sender = deps.api.addr_make("some_sender");
                instantiate(
                    deps.as_mut(),
                    env,
                    message_info(&some_sender, &[]),
                    init_msg,
                )?;

                assert!(all_grants(&deps.storage).is_empty());
                Ok(())
            }

            #[test]
            fn with_insufficient_tokens() -> anyhow::Result<()> {
                // limited grant
                let mut deps = mock_dependencies();
                let env = mock_env();
                let mut grants = HashMap::new();
                grants.insert(
                    deps.api.addr_make("grantee1").to_string(),
                    Allowance::Basic(BasicAllowance {
                        spend_limit: Some(coin(100, TEST_DENOM)),
                        expiration_unix_timestamp: None,
                    }),
                );
                let init_msg = InstantiateMsg {
                    pool_denomination: TEST_DENOM.to_string(),
                    grants,
                };

                let some_sender = deps.api.addr_make("some_sender");
                let res = instantiate(
                    deps.as_mut(),
                    env,
                    message_info(&some_sender, &[]),
                    init_msg,
                );
                assert!(res.is_err());

                // unlimited grant
                let mut deps = mock_dependencies();
                let env = mock_env();
                let mut grants = HashMap::new();
                grants.insert(
                    deps.api.addr_make("grantee1").to_string(),
                    Allowance::Basic(BasicAllowance::unlimited()),
                );
                let init_msg = InstantiateMsg {
                    pool_denomination: TEST_DENOM.to_string(),
                    grants,
                };

                let some_sender = deps.api.addr_make("some_sender");
                let res = instantiate(
                    deps.as_mut(),
                    env,
                    message_info(&some_sender, &[]),
                    init_msg,
                );
                assert!(res.is_err());

                Ok(())
            }

            #[test]
            fn with_valid_request() -> anyhow::Result<()> {
                let env = mock_env();
                let mut deps = deps_with_balance(&env);
                let mut grants = HashMap::new();
                grants.insert(
                    deps.api.addr_make("grantee1").to_string(),
                    Allowance::Basic(BasicAllowance {
                        spend_limit: Some(coin(100, TEST_DENOM)),
                        expiration_unix_timestamp: None,
                    }),
                );
                grants.insert(
                    deps.api.addr_make("grantee2").to_string(),
                    Allowance::Basic(BasicAllowance {
                        spend_limit: Some(coin(200, TEST_DENOM)),
                        expiration_unix_timestamp: None,
                    }),
                );
                grants.insert(
                    deps.api.addr_make("grantee3").to_string(),
                    Allowance::Basic(BasicAllowance {
                        spend_limit: Some(coin(300, TEST_DENOM)),
                        expiration_unix_timestamp: None,
                    }),
                );
                let init_msg = InstantiateMsg {
                    pool_denomination: TEST_DENOM.to_string(),
                    grants,
                };

                let some_sender = deps.api.addr_make("some_sender");
                instantiate(
                    deps.as_mut(),
                    env,
                    message_info(&some_sender, &[coin(600, TEST_DENOM)]),
                    init_msg,
                )?;

                assert_eq!(all_grants(&deps.storage).len(), 3);
                Ok(())
            }
        }
    }
}

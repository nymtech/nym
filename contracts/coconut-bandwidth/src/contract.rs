// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

use coconut_bandwidth_contract_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

use crate::error::ContractError;
use crate::queries::query_spent_credentials_paged;
use crate::state::{Config, ADMIN, CONFIG};
use crate::transactions;

/// Instantiate the contract.
///
/// `deps` contains Storage, API and Querier
/// `msg` is the contract initialization message, sort of like a constructor call.
#[entry_point]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let multisig_addr = deps.api.addr_validate(&msg.multisig_addr)?;
    let pool_addr = deps.api.addr_validate(&msg.pool_addr)?;

    ADMIN.set(deps.branch(), Some(multisig_addr.clone()))?;

    let cfg = Config {
        multisig_addr,
        pool_addr,
    };
    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::default())
}

/// Handle an incoming message
#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::DepositFunds { data } => transactions::deposit_funds(deps, env, info, data),
        ExecuteMsg::SpendCredential { data } => {
            transactions::spend_credential(deps, env, info, data)
        }
        ExecuteMsg::ReleaseFunds { funds } => transactions::release_funds(deps, env, info, funds),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetSpentCredentials { limit, start_after } => {
            to_binary(&query_spent_credentials_paged(deps, start_after, limit)?)
        }
    }
}

#[entry_point]
pub fn migrate(_deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::helpers::*;
    use config::defaults::MIX_DENOM;
    use cosmwasm_std::coins;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn initialize_contract() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let msg = InstantiateMsg {
            multisig_addr: String::from(MULTISIG_CONTRACT),
            pool_addr: String::from(POOL_CONTRACT),
        };
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Contract balance should be 0
        assert_eq!(
            coins(0, MIX_DENOM.base),
            vec![deps
                .as_ref()
                .querier
                .query_balance(env.contract.address, MIX_DENOM.base)
                .unwrap()]
        );
    }
}

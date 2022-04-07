// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use coconut_bandwidth_contract_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

use crate::error::ContractError;
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
        ExecuteMsg::ReleaseFunds { funds } => transactions::release_funds(deps, env, info, funds),
    }
}

#[entry_point]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!();
}

#[entry_point]
pub fn migrate(_deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::helpers::*;
    use coconut_bandwidth_contract_common::deposit::DepositData;
    use config::defaults::DENOM;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, Addr};
    use cw_multi_test::Executor;
    use serde::de::Unexpected::Str;

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
            coins(0, DENOM),
            vec![deps
                .as_ref()
                .querier
                .query_balance(env.contract.address, DENOM)
                .unwrap()]
        );
    }

    #[test]
    fn deposit_and_release() {
        let init_funds = coins(10, DENOM);
        let deposit_funds = coins(1, DENOM);
        let release_funds = coins(2, DENOM);
        let mut app = mock_app(&init_funds);
        let multisig_addr = String::from(MULTISIG_CONTRACT);
        let pool_addr = String::from(POOL_CONTRACT);

        let code_id = app.store_code(contract_bandwidth());
        let msg = InstantiateMsg {
            multisig_addr: multisig_addr.clone(),
            pool_addr: pool_addr.clone(),
        };
        let contract_addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked(OWNER),
                &msg,
                &[],
                "bandwidth",
                None,
            )
            .unwrap();

        let msg = ExecuteMsg::DepositFunds {
            data: DepositData::new(
                String::from("info"),
                String::from("id"),
                String::from("enc"),
            ),
        };
        app.execute_contract(
            Addr::unchecked(OWNER),
            contract_addr.clone(),
            &msg,
            &deposit_funds,
        )
        .unwrap();

        // try to release more then it's in the contract
        let msg = ExecuteMsg::ReleaseFunds {
            funds: release_funds[0].clone(),
        };
        let err = app
            .execute_contract(
                Addr::unchecked(multisig_addr.clone()),
                contract_addr.clone(),
                &msg,
                &[],
            )
            .unwrap_err();
        assert_eq!(ContractError::NotEnoughFunds, err.downcast().unwrap());

        let msg = ExecuteMsg::ReleaseFunds {
            funds: deposit_funds[0].clone(),
        };
        app.execute_contract(
            Addr::unchecked(multisig_addr),
            contract_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();
        let pool_bal = app.wrap().query_balance(pool_addr, DENOM).unwrap();
        assert_eq!(pool_bal, deposit_funds[0]);
    }
}

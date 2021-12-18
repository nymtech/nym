// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod error;
mod queries;
mod storage;
mod support;
mod transactions;

use cosmwasm_std::{
    entry_point, to_binary, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response,
};

use crate::error::ContractError;
use bandwidth_claim_contract::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

/// Instantiate the contract.
///
/// `deps` contains Storage, API and Querier
/// `env` contains block, message and contract info
/// `msg` is the contract initialization message, sort of like a constructor call.
#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

/// Handle an incoming message
#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::LinkPayment { data } => transactions::link_payment(deps, env, info, data),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    let query_res = match msg {
        QueryMsg::GetPayments { start_after, limit } => {
            to_binary(&queries::query_payments_paged(deps, start_after, limit)?)
        }
    };

    Ok(query_res?)
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Default::default())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use bandwidth_claim_contract::payment::PagedPaymentResponse;
    use config::defaults::DENOM;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn initialize_contract() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // payments should be empty after initialization
        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::GetPayments {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();
        let page: PagedPaymentResponse = from_binary(&res).unwrap();
        assert_eq!(0, page.payments.len()); // there are no payments in the list when it's just been initialized

        // Contract balance should match what we initialized it as
        assert_eq!(
            coins(0, DENOM),
            vec![deps
                .as_ref()
                .querier
                .query_balance(env.contract.address, DENOM)
                .unwrap()]
        );
    }
}

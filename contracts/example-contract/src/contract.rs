// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Coin, Deps, DepsMut, Env, MessageInfo, QueryResponse,
    Response,
};
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

pub(crate) const COUNTER: Item<u32> = Item::new("counter");

#[derive(Serialize, Deserialize)]
pub struct InstantiateMsg {
    initial_counter: u32,
}

#[derive(Serialize, Deserialize)]
pub enum ExecuteMsg {
    IncrementCounter {},
    DecrementCounter {},
    SetCounter { to: u32 },
}

#[derive(Serialize, Deserialize)]
pub enum QueryMsg {
    GetCounter {},
}

#[derive(Serialize, Deserialize)]
pub struct MigrateMsg {}

#[entry_point]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, String> {
    COUNTER
        .save(deps.storage, &msg.initial_counter)
        .map_err(|err| err.to_string())?;
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, String> {
    match msg {
        ExecuteMsg::IncrementCounter {} => {
            let current_value = COUNTER.load(deps.storage).map_err(|err| err.to_string())?;
            COUNTER
                .save(deps.storage, &(current_value + 1))
                .map_err(|err| err.to_string())?;
        }
        ExecuteMsg::DecrementCounter {} => {
            let current_value = COUNTER.load(deps.storage).map_err(|err| err.to_string())?;
            COUNTER
                .save(deps.storage, &(current_value - 1))
                .map_err(|err| err.to_string())?;
        }
        ExecuteMsg::SetCounter { to } => {
            COUNTER
                .save(deps.storage, &(to))
                .map_err(|err| err.to_string())?;
        }
    }
    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> Result<QueryResponse, String> {
    let query_res = match msg {
        QueryMsg::GetCounter {} => {
            to_json_binary(&COUNTER.load(deps.storage).map_err(|err| err.to_string())?)
        }
    };

    Ok(query_res.map_err(|err| err.to_string())?)
}

#[entry_point]
pub fn migrate(mut deps: DepsMut<'_>, _env: Env, msg: MigrateMsg) -> Result<Response, String> {
    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::from_json;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};

    #[test]
    fn it_works() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let init_msg = InstantiateMsg {
            initial_counter: 123,
        };

        let sender = message_info(&deps.api.addr_make("mock-sender"), &[]);
        instantiate(deps.as_mut(), env, sender, init_msg).unwrap();

        assert_eq!(COUNTER.load(deps.as_ref().storage).unwrap(), 123);

        let msg = ExecuteMsg::IncrementCounter {};
        let sender = message_info(&deps.api.addr_make("mock-sender"), &[]);

        execute(deps.as_mut(), mock_env(), sender, msg).unwrap();
        assert_eq!(COUNTER.load(deps.as_ref().storage).unwrap(), 124);

        let msg = QueryMsg::GetCounter {};
        let query_res = query(deps.as_ref(), mock_env(), msg).unwrap();
        println!("raw binary: {:?}", query_res);
        println!("string: {:?}", String::from_utf8_lossy(&query_res));
        println!("deserialised: {:?}", from_json::<u32>(query_res).unwrap());
    }
}

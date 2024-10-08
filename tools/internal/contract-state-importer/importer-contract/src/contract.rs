// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ExecuteMsg;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, QueryResponse, Response, StdError};

#[cw_serde]
pub struct EmptyMessage {}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn instantiate(
    _: DepsMut<'_>,
    _: Env,
    _: MessageInfo,
    _: EmptyMessage,
) -> Result<Response, StdError> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, StdError> {
    for (key, value) in msg.pairs {
        let key = base85::decode(&key).unwrap();
        let value = base85::decode(&value).unwrap();
        deps.storage.set(&key, &value);
    }

    Ok(Default::default())
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn query(_: Deps<'_>, _: Env, _: EmptyMessage) -> Result<QueryResponse, StdError> {
    Ok(Default::default())
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn migrate(_deps: DepsMut<'_>, _env: Env, _msg: EmptyMessage) -> Result<Response, StdError> {
    Ok(Default::default())
}

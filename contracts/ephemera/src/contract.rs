// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use crate::peers::queries::query_peers_paged;
use crate::peers::transactions::try_register_peer;
use crate::state::{State, STATE};
use cosmwasm_std::{
    entry_point, to_binary, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response,
};
use cw4::Cw4Contract;
use nym_ephemera_common::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

/// Instantiate the contract.
///
/// `deps` contains Storage, API and Querier
/// `env` contains block, message and contract info
/// `msg` is the contract initialization message, sort of like a constructor call.
#[entry_point]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let group_addr = Cw4Contract(deps.api.addr_validate(&msg.group_addr).map_err(|_| {
        ContractError::InvalidGroup {
            addr: msg.group_addr.clone(),
        }
    })?);

    let state = State {
        group_addr,
        mix_denom: msg.mix_denom,
    };
    STATE.save(deps.storage, &state)?;

    Ok(Response::default())
}

/// Handle an incoming message
#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::RegisterPeer { peer_info } => try_register_peer(deps, info, peer_info),
    }
}

#[entry_point]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    let response = match msg {
        QueryMsg::GetPeers { limit, start_after } => {
            to_binary(&query_peers_paged(deps, start_after, limit)?)?
        }
    };

    Ok(response)
}

#[entry_point]
pub fn migrate(_deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::Addr;

    #[test]
    fn initialize_contract() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let init_msg = InstantiateMsg {
            group_addr: "group_addr".to_string(),
            mix_denom: "uatom".to_string(),
        };

        let sender = mock_info("sender", &[]);
        let res = instantiate(deps.as_mut(), env, sender, init_msg);
        assert!(res.is_ok());

        let expected_state = State {
            group_addr: Cw4Contract::new(Addr::unchecked("group_addr")),
            mix_denom: "uatom".to_string(),
        };
        let state = STATE.load(deps.as_ref().storage).unwrap();
        assert_eq!(state, expected_state);
    }
}

// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::queries::query_admin;
use crate::storage::NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE;
use crate::transactions::{try_propose_or_vote, try_update_contract_admin};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use nym_contracts_common::set_build_information;
use nym_offline_signers_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NymOfflineSignersContractError, QueryMsg,
};

const CONTRACT_NAME: &str = "crate:nym-offline-signers-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, NymOfflineSignersContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    set_build_information!(deps.storage)?;

    let _ = msg;
    NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE.initialise(deps, env, info.sender)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, NymOfflineSignersContractError> {
    match msg {
        ExecuteMsg::UpdateAdmin { admin } => try_update_contract_admin(deps, info, admin),
        ExecuteMsg::ProposeOrVote { signer } => try_propose_or_vote(deps, env, info, signer),
    }
}

#[entry_point]
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> Result<Binary, NymOfflineSignersContractError> {
    match msg {
        QueryMsg::Admin {} => Ok(to_json_binary(&query_admin(deps)?)?),
    }
}

#[entry_point]
pub fn migrate(
    deps: DepsMut,
    _: Env,
    _msg: MigrateMsg,
) -> Result<Response, NymOfflineSignersContractError> {
    set_build_information!(deps.storage)?;
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod contract_instantiation {
        use super::*;
        use crate::storage::NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE;
        use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};

        #[test]
        fn sets_contract_admin_to_the_message_sender() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let some_sender = deps.api.addr_make("some_sender");
            let dummy_dkg_contract = deps.api.addr_make("dkg_contract");

            instantiate(
                deps.as_mut(),
                env,
                message_info(&some_sender, &[]),
                InstantiateMsg {
                    dkg_contract_address: dummy_dkg_contract.to_string(),
                },
            )?;

            NYM_OFFLINE_SIGNERS_CONTRACT_STORAGE
                .contract_admin
                .assert_admin(deps.as_ref(), &some_sender)?;

            Ok(())
        }
    }
}

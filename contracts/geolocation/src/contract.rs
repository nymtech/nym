// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::queries::query_admin;
use crate::transactions::try_update_contract_admin;
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use nym_contracts_common::set_build_information;
use nym_geolocation_contract_common::{
    ExecuteMsg, GeolocationContractError, InstantiateMsg, MigrateMsg, QueryMsg,
};

const CONTRACT_NAME: &str = "crate:nym-geolocation-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, GeolocationContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    set_build_information!(deps.storage)?;

    todo!();

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, GeolocationContractError> {
    match msg {
        ExecuteMsg::UpdateAdmin { admin } => try_update_contract_admin(deps, info, admin),
        ExecuteMsg::SubmitMeasurements { measurements } => todo!(),
        ExecuteMsg::RelaySelfDeclarations { declarations } => todo!(),
        ExecuteMsg::SetOverride { subject, payload } => todo!(),
        ExecuteMsg::RemoveOverride { subject } => todo!(),
        ExecuteMsg::SetWhitelistedAgent { agent, permissions } => todo!(),
        ExecuteMsg::RemoveWhitelistedAgent { agent } => todo!(),
        ExecuteMsg::PurgeAgentEntries { agent, limit } => todo!(),
        ExecuteMsg::UpdateConfig {
            max_skew_secs,
            max_batch_size,
            max_payload_size,
        } => todo!(),
        ExecuteMsg::OnNymNodeUnbond { node_id } => todo!(),
    }
}

#[entry_point]
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> Result<Binary, GeolocationContractError> {
    match msg {
        QueryMsg::Admin {} => Ok(to_json_binary(&query_admin(deps)?)?),
        QueryMsg::Config {} => todo!(),
        QueryMsg::Entry { subject, source } => todo!(),
        QueryMsg::SubjectEntries { subject } => todo!(),
        QueryMsg::NymNodeEntries { node_id } => todo!(),
        QueryMsg::SubjectMeasurements { subject } => todo!(),
        QueryMsg::AllRecords { start_after, limit } => todo!(),
        QueryMsg::Digest {} => todo!(),
        QueryMsg::Whitelist {} => todo!(),
    }
}

#[entry_point]
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, GeolocationContractError> {
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
        use crate::storage::GEOLOCATION_CONTRACT_STORAGE;
        use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
        use cosmwasm_std::Addr;

        #[test]
        fn sets_contract_admin_to_the_message_sender() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let init_msg = InstantiateMsg {
                mixnet_contract_address: deps.api.addr_make("mixnet-contract").to_string(),
                initial_whitelist: vec![],
                max_skew_secs: None,
                max_batch_size: None,
                max_payload_size: None,
            };

            let some_sender = deps.api.addr_make("some_sender");
            instantiate(
                deps.as_mut(),
                env,
                message_info(&some_sender, &[]),
                init_msg,
            )?;

            GEOLOCATION_CONTRACT_STORAGE
                .contract_admin
                .assert_admin(deps.as_ref(), &some_sender)?;

            Ok(())
        }
    }
}

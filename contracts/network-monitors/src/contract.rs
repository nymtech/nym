// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::queries::{
    query_admin, query_network_monitor_agents, query_network_monitor_orchestrators,
};
use crate::storage::NETWORK_MONITORS_CONTRACT_STORAGE;
use crate::transactions::{
    try_authorise_network_monitor, try_authorise_network_monitor_orchestrator,
    try_revoke_all_network_monitors, try_revoke_network_monitor,
    try_revoke_network_monitor_orchestrator, try_update_contract_admin,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use nym_contracts_common::set_build_information;
use nym_network_monitors_contract_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NetworkMonitorsContractError, QueryMsg,
};

const CONTRACT_NAME: &str = "crate:nym-network-monitors-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, NetworkMonitorsContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    set_build_information!(deps.storage)?;

    let orchestrator = deps.api.addr_validate(&msg.orchestrator_address)?;
    NETWORK_MONITORS_CONTRACT_STORAGE.initialise(deps, env, info.sender, orchestrator)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, NetworkMonitorsContractError> {
    match msg {
        ExecuteMsg::UpdateAdmin { admin } => try_update_contract_admin(deps, info, admin),
        ExecuteMsg::AuthoriseNetworkMonitorOrchestrator { address } => {
            try_authorise_network_monitor_orchestrator(deps, env, info, address)
        }
        ExecuteMsg::RevokeNetworkMonitorOrchestrator { address } => {
            try_revoke_network_monitor_orchestrator(deps, info, address)
        }
        ExecuteMsg::AuthoriseNetworkMonitor { address } => {
            try_authorise_network_monitor(deps, env, info, address)
        }
        ExecuteMsg::RevokeNetworkMonitor { address } => {
            try_revoke_network_monitor(deps, info, address)
        }
        ExecuteMsg::RevokeAllNetworkMonitors => try_revoke_all_network_monitors(deps, info),
    }
}

#[entry_point]
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> Result<Binary, NetworkMonitorsContractError> {
    match msg {
        QueryMsg::Admin {} => Ok(to_json_binary(&query_admin(deps)?)?),
        QueryMsg::NetworkMonitorOrchestrators {} => {
            Ok(to_json_binary(&query_network_monitor_orchestrators(deps)?)?)
        }
        QueryMsg::NetworkMonitorAgents {
            start_next_after,
            limit,
        } => Ok(to_json_binary(&query_network_monitor_agents(
            deps,
            start_next_after,
            limit,
        )?)?),
    }
}

#[entry_point]
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, NetworkMonitorsContractError> {
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
        use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
        use cosmwasm_std::Addr;

        #[test]
        fn sets_contract_admin_to_the_message_sender() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let init_msg = InstantiateMsg {
                orchestrator_address: deps.api.addr_make("foo").to_string(),
            };

            let some_sender = deps.api.addr_make("some_sender");
            instantiate(
                deps.as_mut(),
                env,
                message_info(&some_sender, &[]),
                init_msg,
            )?;

            NETWORK_MONITORS_CONTRACT_STORAGE
                .contract_admin
                .assert_admin(deps.as_ref(), &some_sender)?;

            Ok(())
        }

        #[test]
        fn sets_the_initial_orchestrator() -> anyhow::Result<()> {
            let mut deps = mock_dependencies();
            let env = mock_env();
            let admin = deps.api.addr_make("some_sender");

            let bad_addr = "foo".to_string();
            let good_addr = deps.api.addr_make("foo").to_string();

            let bad_init_msg = InstantiateMsg {
                orchestrator_address: bad_addr.clone(),
            };

            let good_init_msg = InstantiateMsg {
                orchestrator_address: good_addr.clone(),
            };

            let res = instantiate(
                deps.as_mut(),
                env.clone(),
                message_info(&admin, &[]),
                bad_init_msg,
            );
            assert!(res.is_err());

            let is_orchestrator = NETWORK_MONITORS_CONTRACT_STORAGE
                .is_orchestrator(deps.as_ref(), &Addr::unchecked(&good_addr))?;
            assert!(!is_orchestrator);

            instantiate(deps.as_mut(), env, message_info(&admin, &[]), good_init_msg)?;

            let is_orchestrator = NETWORK_MONITORS_CONTRACT_STORAGE
                .is_orchestrator(deps.as_ref(), &Addr::unchecked(&good_addr))?;
            assert!(is_orchestrator);

            Ok(())
        }
    }
}

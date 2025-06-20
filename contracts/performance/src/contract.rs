// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::queries::{
    query_admin, query_epoch_measurements_paged, query_epoch_performance_paged,
    query_full_historical_performance_paged, query_network_monitor_details,
    query_network_monitors_paged, query_node_measurements, query_node_performance,
    query_node_performance_paged, query_retired_network_monitors_paged,
};
use crate::storage::NYM_PERFORMANCE_CONTRACT_STORAGE;
use crate::transactions::{
    try_authorise_network_monitor, try_batch_submit_performance_results,
    try_remove_epoch_measurements, try_remove_node_measurements, try_retire_network_monitor,
    try_submit_performance_results, try_update_contract_admin,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use nym_contracts_common::set_build_information;
use nym_performance_contract_common::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, NymPerformanceContractError, QueryMsg,
};

const CONTRACT_NAME: &str = "crate:nym-performance-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, NymPerformanceContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    set_build_information!(deps.storage)?;

    let mixnet_contract_address = deps.api.addr_validate(&msg.mixnet_contract_address)?;

    NYM_PERFORMANCE_CONTRACT_STORAGE.initialise(
        deps,
        env,
        info.sender,
        mixnet_contract_address.clone(),
        msg.authorised_network_monitors,
    )?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, NymPerformanceContractError> {
    match msg {
        ExecuteMsg::UpdateAdmin { admin } => try_update_contract_admin(deps, info, admin),
        ExecuteMsg::Submit { epoch, data } => {
            try_submit_performance_results(deps, info, epoch, data)
        }
        ExecuteMsg::BatchSubmit { epoch, data } => {
            try_batch_submit_performance_results(deps, info, epoch, data)
        }
        ExecuteMsg::AuthoriseNetworkMonitor { address } => {
            try_authorise_network_monitor(deps, env, info, address)
        }
        ExecuteMsg::RetireNetworkMonitor { address } => {
            try_retire_network_monitor(deps, env, info, address)
        }
        ExecuteMsg::RemoveNodeMeasurements { epoch_id, node_id } => {
            try_remove_node_measurements(deps, info, epoch_id, node_id)
        }
        ExecuteMsg::RemoveEpochMeasurements { epoch_id } => {
            try_remove_epoch_measurements(deps, info, epoch_id)
        }
    }
}

#[entry_point]
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> Result<Binary, NymPerformanceContractError> {
    match msg {
        QueryMsg::Admin {} => Ok(to_json_binary(&query_admin(deps)?)?),
        QueryMsg::NodePerformance { epoch_id, node_id } => Ok(to_json_binary(
            &query_node_performance(deps, epoch_id, node_id)?,
        )?),
        QueryMsg::NodePerformancePaged {
            node_id,
            start_after,
            limit,
        } => Ok(to_json_binary(&query_node_performance_paged(
            deps,
            node_id,
            start_after,
            limit,
        )?)?),
        QueryMsg::EpochPerformancePaged {
            epoch_id,
            start_after,
            limit,
        } => Ok(to_json_binary(&query_epoch_performance_paged(
            deps,
            epoch_id,
            start_after,
            limit,
        )?)?),
        QueryMsg::FullHistoricalPerformancePaged { start_after, limit } => Ok(to_json_binary(
            &query_full_historical_performance_paged(deps, start_after, limit)?,
        )?),
        QueryMsg::NetworkMonitor { address } => Ok(to_json_binary(
            &query_network_monitor_details(deps, address)?,
        )?),
        QueryMsg::NetworkMonitorsPaged { start_after, limit } => Ok(to_json_binary(
            &query_network_monitors_paged(deps, start_after, limit)?,
        )?),
        QueryMsg::RetiredNetworkMonitorsPaged { start_after, limit } => Ok(to_json_binary(
            &query_retired_network_monitors_paged(deps, start_after, limit)?,
        )?),
        QueryMsg::NodeMeasurements { epoch_id, node_id } => Ok(to_json_binary(
            &query_node_measurements(deps, epoch_id, node_id)?,
        )?),
        QueryMsg::EpochMeasurementsPaged {
            epoch_id,
            start_after,
            limit,
        } => Ok(to_json_binary(&query_epoch_measurements_paged(
            deps,
            epoch_id,
            start_after,
            limit,
        )?)?),
    }
}

#[entry_point]
pub fn migrate(
    deps: DepsMut,
    _: Env,
    _msg: MigrateMsg,
) -> Result<Response, NymPerformanceContractError> {
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
        use crate::storage::NYM_PERFORMANCE_CONTRACT_STORAGE;
        use crate::testing::PreInitContract;
        use cosmwasm_std::testing::message_info;

        #[test]
        fn sets_contract_admin_to_the_message_sender() -> anyhow::Result<()> {
            // we need to mock dependencies in a state where mixnet contract has already been instantiated
            // (we query it at init)
            let mut pre_init = PreInitContract::new();
            let env = pre_init.env();
            let mixnet_contract_address = pre_init.mixnet_contract_address.to_string();
            let some_sender = pre_init.addr_make("some_sender");
            let deps = pre_init.deps_mut();

            instantiate(
                deps,
                env,
                message_info(&some_sender, &[]),
                InstantiateMsg {
                    mixnet_contract_address,
                    authorised_network_monitors: vec![],
                },
            )?;

            let deps = pre_init.deps();

            NYM_PERFORMANCE_CONTRACT_STORAGE
                .contract_admin
                .assert_admin(deps, &some_sender)?;

            Ok(())
        }
    }
}

// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::queries::{
    query_admin, query_current_weights, query_epoch_measurements_paged,
    query_epoch_performance_paged, query_full_historical_performance_paged, query_last_known_epoch,
    query_last_submission, query_network_monitor_details, query_network_monitors_paged,
    query_node_measurements, query_node_performance, query_node_performance_paged,
    query_retired_network_monitors_paged, query_rewarding_inputs, query_rewarding_score,
    query_weights_at,
};
use crate::storage::NYM_PERFORMANCE_CONTRACT_STORAGE;
use crate::transactions::{
    try_authorise_network_monitor, try_batch_submit_performance_results,
    try_remove_epoch_measurements, try_remove_node_measurements, try_retire_network_monitor,
    try_submit_performance_results, try_update_contract_admin, try_update_weights,
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
        msg.initial_weights,
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
            try_submit_performance_results(deps, env, info, epoch, data)
        }
        ExecuteMsg::BatchSubmit { epoch, data } => {
            try_batch_submit_performance_results(deps, env, info, epoch, data)
        }
        ExecuteMsg::UpdateWeights { weights } => try_update_weights(deps, info, weights),
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
        QueryMsg::RewardingInputs { epoch_id, node_id } => Ok(to_json_binary(
            &query_rewarding_inputs(deps, epoch_id, node_id)?,
        )?),
        QueryMsg::RewardingScore { epoch_id, node_id } => Ok(to_json_binary(
            &query_rewarding_score(deps, epoch_id, node_id)?,
        )?),
        QueryMsg::LastKnownEpoch { node_id } => {
            Ok(to_json_binary(&query_last_known_epoch(deps, node_id)?)?)
        }
        QueryMsg::WeightsAt { epoch_id } => Ok(to_json_binary(&query_weights_at(deps, epoch_id)?)?),
        QueryMsg::CurrentWeights {} => Ok(to_json_binary(&query_current_weights(deps)?)?),
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
        QueryMsg::LastSubmittedMeasurement {} => Ok(to_json_binary(&query_last_submission(deps)?)?),
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
        use crate::testing::{liveness_only_weights, p, PreInitContract};
        use cosmwasm_std::testing::message_info;
        use cosmwasm_std::Decimal;
        use nym_contracts_common::Percent;
        use nym_performance_contract_common::constants::storage_keys;
        use nym_performance_contract_common::{EpochWeights, Weights};

        /// An instantiate message for the embedded mixnet contract with no initial monitors.
        fn init_msg(pre_init: &PreInitContract, initial_weights: Weights) -> InstantiateMsg {
            InstantiateMsg {
                mixnet_contract_address: pre_init.mixnet_contract_address.to_string(),
                authorised_network_monitors: vec![],
                initial_weights,
            }
        }

        #[test]
        fn rejects_an_invalid_mixnet_contract_address() -> anyhow::Result<()> {
            let mut pre_init = PreInitContract::new();
            let env = pre_init.env();
            let sender = pre_init.addr_make("some_sender");
            let msg = InstantiateMsg {
                mixnet_contract_address: "definitely-not-valid-account".to_string(),
                authorised_network_monitors: vec![],
                initial_weights: liveness_only_weights(),
            };

            assert!(
                instantiate(pre_init.deps_mut(), env, message_info(&sender, &[]), msg).is_err()
            );

            let deps = pre_init.deps();
            assert!(deps
                .storage
                .get(storage_keys::CONTRACT_ADMIN.as_bytes())
                .is_none());
            assert!(NYM_PERFORMANCE_CONTRACT_STORAGE
                .mixnet_contract_address
                .may_load(deps.storage)?
                .is_none());

            Ok(())
        }

        #[test]
        fn sets_contract_admin_to_the_message_sender() -> anyhow::Result<()> {
            // we need to mock dependencies in a state where mixnet contract has already been instantiated
            // (we query it at init)
            let mut pre_init = PreInitContract::new();
            let env = pre_init.env();
            let some_sender = pre_init.addr_make("some_sender");
            let msg = init_msg(&pre_init, liveness_only_weights());

            instantiate(
                pre_init.deps_mut(),
                env,
                message_info(&some_sender, &[]),
                msg,
            )?;

            let deps = pre_init.deps();

            NYM_PERFORMANCE_CONTRACT_STORAGE
                .contract_admin
                .assert_admin(deps, &some_sender)?;

            Ok(())
        }

        #[test]
        fn stores_the_initial_weights_under_the_creation_epoch() -> anyhow::Result<()> {
            let mut pre_init = PreInitContract::new();
            let env = pre_init.env();
            let sender = pre_init.addr_make("some_sender");
            let weights = Weights {
                liveness: p("0.7"),
                stress: p("0.3"),
            };
            let msg = init_msg(&pre_init, weights);

            instantiate(pre_init.deps_mut(), env, message_info(&sender, &[]), msg)?;

            // the embedded mixnet contract starts at epoch 0, so that is the creation epoch
            let deps = pre_init.deps();
            assert_eq!(
                NYM_PERFORMANCE_CONTRACT_STORAGE.weights_at(deps.storage, 0)?,
                Some(EpochWeights {
                    effective_from: 0,
                    weights,
                })
            );

            Ok(())
        }

        #[test]
        fn rejects_invalid_initial_weights_without_persisting_anything() -> anyhow::Result<()> {
            let mut pre_init = PreInitContract::new();
            let env = pre_init.env();
            let sender = pre_init.addr_make("some_sender");
            let msg = init_msg(
                &pre_init,
                Weights {
                    liveness: p("0.7"),
                    stress: Percent::zero(),
                },
            );

            let res =
                instantiate(pre_init.deps_mut(), env, message_info(&sender, &[]), msg).unwrap_err();
            assert_eq!(
                res,
                NymPerformanceContractError::WeightsDoNotSumToOne {
                    total: Decimal::percent(70)
                }
            );

            // nothing of the contract's own state was written; cw2 and the build information
            // precede the check inside the entry point, and a failed tx reverts them on-chain
            let deps = pre_init.deps();
            assert!(deps
                .storage
                .get(storage_keys::CONTRACT_ADMIN.as_bytes())
                .is_none());
            assert!(NYM_PERFORMANCE_CONTRACT_STORAGE
                .mixnet_contract_address
                .may_load(deps.storage)?
                .is_none());
            assert!(NYM_PERFORMANCE_CONTRACT_STORAGE
                .weights
                .is_empty(deps.storage));

            Ok(())
        }
    }
}

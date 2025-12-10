// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::storage::{MeasurementKind, NYM_PERFORMANCE_CONTRACT_STORAGE, retrieval_limits};
use cosmwasm_std::{Addr, Deps, Order, StdResult};
use cw_controllers::AdminResponse;
use cw_storage_plus::Bound;
use nym_performance_contract_common::{
    AllNodeMeasurementsResponse, EpochId, EpochMeasurementsPagedResponse, EpochNodePerformance,
    EpochPerformancePagedResponse, FullHistoricalPerformancePagedResponse, HistoricalPerformance,
    LastSubmission, NetworkMonitorInformation, NetworkMonitorResponse,
    NetworkMonitorsPagedResponse, NodeId, NodeMeasurements, NodeMeasurementsPerKindResponse,
    NodePerformance, NodePerformancePagedResponse, NodePerformanceResponse, NodeResults,
    NymPerformanceContractError, RetiredNetworkMonitorsPagedResponse,
};

pub fn query_admin(deps: Deps) -> Result<AdminResponse, NymPerformanceContractError> {
    NYM_PERFORMANCE_CONTRACT_STORAGE
        .contract_admin
        .query_admin(deps)
        .map_err(Into::into)
}

pub fn query_node_performance(
    deps: Deps,
    epoch_id: EpochId,
    node_id: NodeId,
) -> Result<NodePerformanceResponse, NymPerformanceContractError> {
    let performance =
        NYM_PERFORMANCE_CONTRACT_STORAGE.try_load_performance(deps.storage, epoch_id, node_id)?;
    Ok(NodePerformanceResponse { performance })
}

pub fn query_node_measurements_for_kind(
    deps: Deps,
    epoch_id: EpochId,
    node_id: NodeId,
    measurement_kind: String,
) -> Result<NodeMeasurementsPerKindResponse, NymPerformanceContractError> {
    let measurements = NYM_PERFORMANCE_CONTRACT_STORAGE.try_load_measurement_kind(
        deps.storage,
        epoch_id,
        node_id,
        measurement_kind,
    )?;

    Ok(NodeMeasurementsPerKindResponse { measurements })
}

pub fn query_all_node_measurements(
    deps: Deps,
    epoch_id: EpochId,
    node_id: NodeId,
) -> Result<AllNodeMeasurementsResponse, NymPerformanceContractError> {
    let measurements = NYM_PERFORMANCE_CONTRACT_STORAGE.performance_results.results;

    // retrieve a list of currently defined measurements, only return results for those
    // (storage may contain measurements that have since been deleted by admin -
    // this way, they won't be retrieved)
    let possible_measurements = NYM_PERFORMANCE_CONTRACT_STORAGE
        .performance_results
        .defined_measurements(deps.storage)?;
    let mut node_measurements = HashMap::new();
    for measure_name in possible_measurements {
        let key = (epoch_id, node_id, measure_name.clone());
        let node_measurement = measurements.may_load(deps.storage, key)?;
        node_measurements.insert(measure_name, node_measurement);
    }

    Ok(AllNodeMeasurementsResponse {
        measurements: node_measurements,
    })
}

pub fn query_node_performance_paged(
    deps: Deps,
    node_id: NodeId,
    start_after: Option<EpochId>,
    limit: Option<u32>,
) -> Result<NodePerformancePagedResponse, NymPerformanceContractError> {
    let current_epoch_id = NYM_PERFORMANCE_CONTRACT_STORAGE.current_mixnet_epoch_id(deps)?;

    let start = match start_after {
        None => NYM_PERFORMANCE_CONTRACT_STORAGE
            .mixnet_epoch_id_at_creation
            .load(deps.storage)?,
        Some(start_after) => start_after + 1,
    };

    let mut performance = Vec::new();

    if current_epoch_id < start {
        return Ok(NodePerformancePagedResponse {
            node_id,
            performance,
            start_next_after: None,
        });
    }

    let limit = limit
        .unwrap_or(retrieval_limits::NODE_PERFORMANCE_DEFAULT_LIMIT)
        .min(retrieval_limits::NODE_PERFORMANCE_MAX_LIMIT) as usize;

    for epoch_id in (start..=current_epoch_id).take(limit) {
        performance.push(EpochNodePerformance {
            epoch: epoch_id,
            performance: NYM_PERFORMANCE_CONTRACT_STORAGE.try_load_performance(
                deps.storage,
                epoch_id,
                node_id,
            )?,
        })
    }

    let start_next_after = performance.last().and_then(|last| {
        if last.epoch != current_epoch_id {
            Some(last.epoch)
        } else {
            None
        }
    });

    Ok(NodePerformancePagedResponse {
        node_id,
        performance,
        start_next_after,
    })
}

pub fn query_epoch_performance_paged(
    deps: Deps,
    epoch_id: EpochId,
    start_after: Option<NodeId>,
    limit: Option<u32>,
) -> Result<EpochPerformancePagedResponse, NymPerformanceContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::NODE_EPOCH_PERFORMANCE_DEFAULT_LIMIT)
        .min(retrieval_limits::NODE_EPOCH_PERFORMANCE_MAX_LIMIT) as usize;

    let start = start_after.map(|node_id| Bound::exclusive((node_id + 1, String::new())));

    let performance = NYM_PERFORMANCE_CONTRACT_STORAGE
        .performance_results
        .results
        .sub_prefix(epoch_id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|record| {
            record.map(|((node_id, measurement_kind), results)| NodePerformance {
                node_id,
                performance: results.median(),
                measurement_kind,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = performance.last().map(|last| last.node_id);

    Ok(EpochPerformancePagedResponse {
        epoch_id,
        performance,
        start_next_after,
    })
}

pub fn query_epoch_measurements_paged(
    deps: Deps,
    epoch_id: EpochId,
    start_after: Option<NodeId>,
    limit: Option<u32>,
) -> Result<EpochMeasurementsPagedResponse, NymPerformanceContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::NODE_EPOCH_MEASUREMENTS_DEFAULT_LIMIT)
        .min(retrieval_limits::NODE_EPOCH_MEASUREMENTS_MAX_LIMIT) as usize;

    let start = start_after.map(|node_id| Bound::exclusive((node_id + 1, String::new())));

    // because API aggregates per NodeId, and the storage doesn't, we have to
    // first collect all different measurements for a node and use an
    // intermediary struct to map from storage to the object returned on the API
    let mut measurements_per_node: HashMap<NodeId, Vec<(MeasurementKind, NodeResults)>> =
        HashMap::new();
    let measurements = NYM_PERFORMANCE_CONTRACT_STORAGE
        .performance_results
        .results
        .sub_prefix(epoch_id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|record| {
            record.inspect(|((node_id, kind), measurements)| {
                measurements_per_node
                    .entry(*node_id)
                    .and_modify(|vec| vec.push((kind.to_string(), measurements.to_owned())))
                    .or_insert_with(|| vec![(kind.to_string(), measurements.to_owned())]);
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    // transforming collected data into a returning type
    let mut returning = Vec::new();
    for (node_id, measurements_per_kind) in measurements_per_node.into_iter() {
        let mut measurements = HashMap::new();
        for (measurement_kind, results) in measurements_per_kind {
            measurements.insert(measurement_kind, results);
        }
        returning.push(NodeMeasurements {
            node_id,
            measurements_per_kind: measurements,
        });
    }

    // storage keeps nodes in ascending order for pagination
    // intermediary hashmap doesn't have deterministic order so we need to order
    // explicitly here before returning
    returning.sort_by_key(|elem| elem.node_id);
    let start_next_after = measurements.last().map(|((last, _), _)| *last);

    Ok(EpochMeasurementsPagedResponse {
        epoch_id,
        measurements: returning,
        start_next_after,
    })
}

pub fn query_full_historical_performance_paged(
    deps: Deps,
    start_after: Option<(EpochId, NodeId)>,
    limit: Option<u32>,
) -> Result<FullHistoricalPerformancePagedResponse, NymPerformanceContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::NODE_HISTORICAL_PERFORMANCE_DEFAULT_LIMIT)
        .min(retrieval_limits::NODE_HISTORICAL_PERFORMANCE_MAX_LIMIT) as usize;

    let start = start_after.map(|(n, e)| Bound::exclusive((n, e, String::new())));

    let performance = NYM_PERFORMANCE_CONTRACT_STORAGE
        .performance_results
        .results
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|record| {
            record.map(|((epoch_id, node_id, _), results)| HistoricalPerformance {
                // TODO dz map kind as well
                epoch_id,
                node_id,
                performance: results.median(),
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = performance.last().map(|last| (last.epoch_id, last.node_id));

    Ok(FullHistoricalPerformancePagedResponse {
        performance,
        start_next_after,
    })
}

fn get_network_monitor_information(
    deps: Deps,
    address: &Addr,
) -> Result<Option<NetworkMonitorInformation>, NymPerformanceContractError> {
    let Some(details) = NYM_PERFORMANCE_CONTRACT_STORAGE
        .network_monitors
        .authorised
        .may_load(deps.storage, address)?
    else {
        return Ok(None);
    };

    let current_submission_metadata = NYM_PERFORMANCE_CONTRACT_STORAGE
        .performance_results
        .submission_metadata
        .load(deps.storage, address)?;

    Ok(Some(NetworkMonitorInformation {
        details,
        current_submission_metadata,
    }))
}

pub fn query_network_monitor_details(
    deps: Deps,
    address: String,
) -> Result<NetworkMonitorResponse, NymPerformanceContractError> {
    let address = deps.api.addr_validate(&address)?;

    Ok(NetworkMonitorResponse {
        info: get_network_monitor_information(deps, &address)?,
    })
}

pub fn query_network_monitors_paged(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<NetworkMonitorsPagedResponse, NymPerformanceContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::NETWORK_MONITORS_DEFAULT_LIMIT)
        .min(retrieval_limits::NETWORK_MONITORS_MAX_LIMIT) as usize;

    let addr = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;
    let start = addr.as_ref().map(Bound::exclusive);

    let info = NYM_PERFORMANCE_CONTRACT_STORAGE
        .network_monitors
        .authorised
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|record| {
            record.and_then(|(address, details)| {
                NYM_PERFORMANCE_CONTRACT_STORAGE
                    .performance_results
                    .submission_metadata
                    .load(deps.storage, &address)
                    .map(|current_submission_metadata| NetworkMonitorInformation {
                        details,
                        current_submission_metadata,
                    })
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = info.last().map(|last| last.details.address.to_string());

    Ok(NetworkMonitorsPagedResponse {
        info,
        start_next_after,
    })
}

pub fn query_retired_network_monitors_paged(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<RetiredNetworkMonitorsPagedResponse, NymPerformanceContractError> {
    let limit = limit
        .unwrap_or(retrieval_limits::RETIRED_NETWORK_MONITORS_DEFAULT_LIMIT)
        .min(retrieval_limits::RETIRED_NETWORK_MONITORS_MAX_LIMIT) as usize;

    let addr = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;
    let start = addr.as_ref().map(Bound::exclusive);

    let info = NYM_PERFORMANCE_CONTRACT_STORAGE
        .network_monitors
        .retired
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|record| record.map(|(_, details)| details))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = info.last().map(|last| last.details.address.to_string());

    Ok(RetiredNetworkMonitorsPagedResponse {
        info,
        start_next_after,
    })
}

pub fn query_last_submission(deps: Deps) -> Result<LastSubmission, NymPerformanceContractError> {
    NYM_PERFORMANCE_CONTRACT_STORAGE
        .last_performance_submission
        .load(deps.storage)
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{
        PerformanceContractTesterExt, epoch_node_performance_unchecked, init_contract_tester,
    };
    use nym_contracts_common_testing::{AdminExt, ChainOpts, ContractOpts, RandExt};
    use nym_performance_contract_common::{ExecuteMsg, LastSubmittedData, NodePerformance};

    #[cfg(test)]
    mod admin_query {
        use super::*;
        use crate::testing::init_contract_tester;
        use nym_contracts_common_testing::{AdminExt, ChainOpts, ContractOpts, RandExt};
        use nym_performance_contract_common::ExecuteMsg;

        #[test]
        fn returns_current_admin() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let initial_admin = test.admin_unchecked();

            // initial
            let res = query_admin(test.deps())?;
            assert_eq!(res.admin, Some(initial_admin.to_string()));

            let new_admin = test.generate_account();

            // sanity check
            assert_ne!(initial_admin, new_admin);

            // after update
            test.execute_msg(
                initial_admin.clone(),
                &ExecuteMsg::UpdateAdmin {
                    admin: new_admin.to_string(),
                },
            )?;

            let updated_admin = query_admin(test.deps())?;
            assert_eq!(updated_admin.admin, Some(new_admin.to_string()));

            Ok(())
        }
    }

    #[test]
    fn querying_node_measurements_kind() -> anyhow::Result<()> {
        let mut test = init_contract_tester();

        // test setup
        let nm1 = test.generate_account();
        let nm2 = test.generate_account();
        test.authorise_network_monitor(&nm1)?;
        test.authorise_network_monitor(&nm2)?;

        let admin = test.admin_unchecked();
        let kind_mixnet = String::from("mixnet");
        let kind_wireguard = String::from("wireguard");
        test.execute_raw(
            admin.clone(),
            ExecuteMsg::DefineMeasurementKind {
                measurement_kind: kind_mixnet.clone(),
            },
        )?;
        test.execute_raw(
            admin,
            ExecuteMsg::DefineMeasurementKind {
                measurement_kind: kind_wireguard.clone(),
            },
        )?;

        let node1 = test.bond_dummy_nymnode()?;
        let node2 = test.bond_dummy_nymnode()?;

        let epoch_id = 10;
        test.set_mixnet_epoch(epoch_id)?;

        let deps = test.deps();

        // ===== Test: undefined measurement kind =====
        let undefined_kind = String::from("undefined");
        let res = query_node_measurements_for_kind(deps, epoch_id, node1, undefined_kind);
        assert!(res.is_err());
        assert!(matches!(
            res.unwrap_err(),
            NymPerformanceContractError::UnsupportedMeasurementKind { .. }
        ));

        // ===== Test: query returns None for defined kind with no data =====
        let res = query_node_measurements_for_kind(deps, epoch_id, node1, kind_mixnet.clone())?;
        assert!(res.measurements.is_none());

        // ===== Test happy path: single measurement from one monitor =====
        test.insert_raw_performance(&nm1, node1, kind_mixnet.clone(), "0.5")?;
        let res =
            query_node_measurements_for_kind(test.deps(), epoch_id, node1, kind_mixnet.clone())?;
        let measurements = res.measurements.unwrap();
        assert_eq!(measurements.inner().len(), 1);
        assert_eq!(measurements.inner()[0], "0.5".parse()?);

        // Verify against raw storage
        let expected = test.read_raw_scores(epoch_id, node1, kind_mixnet.clone())?;
        assert_eq!(measurements.inner(), expected.inner());

        // ===== Test: multiple measurements from different monitors =====
        // each monitor can only submit once per (epoch, node) pair
        test.insert_raw_performance(&nm2, node1, kind_mixnet.clone(), "0.3")?;
        let res =
            query_node_measurements_for_kind(test.deps(), epoch_id, node1, kind_mixnet.clone())?;
        let measurements = res.measurements.unwrap();
        assert_eq!(measurements.inner().len(), 2);

        // ===== Test: multiple measurement kinds are independent =====
        // we need a new epoch since monitors already submitted in this epoch
        let epoch_11 = 11;
        test.set_mixnet_epoch(epoch_11)?;

        // now submit
        test.insert_raw_performance(&nm1, node1, kind_wireguard.clone(), "0.8")?;
        test.insert_raw_performance(&nm2, node1, kind_wireguard.clone(), "0.9")?;

        // verify data for submitted kind is there
        let res_wireguard_e11 =
            query_node_measurements_for_kind(test.deps(), epoch_11, node1, kind_wireguard.clone())?;
        let wg_measurements = res_wireguard_e11.measurements.unwrap();
        assert_eq!(wg_measurements.inner().len(), 2);
        assert_eq!(wg_measurements.inner()[0], "0.8".parse()?);
        assert_eq!(wg_measurements.inner()[1], "0.9".parse()?);

        // not submitted for this kind in this epoch: should have no data
        let res_mixnet_e11 =
            query_node_measurements_for_kind(test.deps(), epoch_11, node1, kind_mixnet.clone())?;
        assert!(res_mixnet_e11.measurements.is_none());

        // however, mixnet kind should still have old data in previous epoch
        let res_mixnet_e10 =
            query_node_measurements_for_kind(test.deps(), epoch_id, node1, kind_mixnet.clone())?;
        let mixnet_measurements = res_mixnet_e10.measurements.unwrap();
        assert_eq!(mixnet_measurements.inner().len(), 2);
        assert_eq!(mixnet_measurements.inner()[0], "0.3".parse()?);
        assert_eq!(mixnet_measurements.inner()[1], "0.5".parse()?);

        // ===== Test: different epochs are independent =====
        // advance epoch again & submit something
        let epoch_12 = 12;
        test.set_mixnet_epoch(epoch_12)?;
        test.insert_raw_performance(&nm1, node1, kind_mixnet.clone(), "0.25")?;

        // epoch 12 should have new data
        let res_epoch12 =
            query_node_measurements_for_kind(test.deps(), epoch_12, node1, kind_mixnet.clone())?;
        assert!(res_epoch12.measurements.is_some());
        let epoch12_measurements = res_epoch12.measurements.unwrap();
        assert_eq!(epoch12_measurements.inner().len(), 1);
        assert_eq!(epoch12_measurements.inner()[0], "0.25".parse()?);

        // epoch 10 should still have old data
        let res_epoch10 =
            query_node_measurements_for_kind(test.deps(), epoch_id, node1, kind_mixnet.clone())?;
        assert_eq!(res_epoch10.measurements.unwrap().inner().len(), 2);

        // epoch 11 with mixnet (no data) should return None
        let res_epoch11_mixnet =
            query_node_measurements_for_kind(test.deps(), epoch_11, node1, kind_mixnet.clone())?;
        assert!(res_epoch11_mixnet.measurements.is_none());

        // ===== Test: different nodes are independent =====
        // nm1 can now submit for node2 in epoch 12 since node2 > node1
        test.insert_raw_performance(&nm1, node2, kind_mixnet.clone(), "0.42")?;

        // Query node1 in epoch 12 - should have nm1's data
        let res_node1_e12 =
            query_node_measurements_for_kind(test.deps(), epoch_12, node1, kind_mixnet.clone())?;
        assert_eq!(res_node1_e12.measurements.unwrap().inner().len(), 1);

        // Query node2 in epoch 12 - should have different data
        let res_node2_e12 =
            query_node_measurements_for_kind(test.deps(), epoch_12, node2, kind_mixnet.clone())?;
        let node2_measurements = res_node2_e12.measurements.unwrap();
        assert_eq!(node2_measurements.inner().len(), 1);
        assert_eq!(node2_measurements.inner()[0], "0.42".parse()?);

        // Query node2 in epoch 10 (no data) - should return None
        let res_node2_e10 =
            query_node_measurements_for_kind(test.deps(), epoch_id, node2, kind_mixnet.clone())?;
        assert!(res_node2_e10.measurements.is_none());

        // verify against raw data
        let raw_scores = test.read_raw_scores(epoch_id, node1, kind_mixnet.clone())?;
        let query_result =
            query_node_measurements_for_kind(test.deps(), epoch_id, node1, kind_mixnet.clone())?;
        assert_eq!(
            query_result.measurements.unwrap().inner(),
            raw_scores.inner()
        );

        Ok(())
    }

    #[test]
    fn querying_node_performance_paged() -> anyhow::Result<()> {
        let mut test = init_contract_tester();

        let node_id = test.bond_dummy_nymnode()?;
        let nm = test.generate_account();
        test.authorise_network_monitor(&nm)?;
        let measurement_kind = test.define_dummy_measurement_kind().unwrap();

        // epoch 0
        test.insert_raw_performance(&nm, node_id, measurement_kind.clone(), "0")?;

        // epoch 1
        test.advance_mixnet_epoch()?;
        test.insert_raw_performance(&nm, node_id, measurement_kind.clone(), "0.1")?;

        // epoch 2
        test.advance_mixnet_epoch()?;
        test.insert_raw_performance(&nm, node_id, measurement_kind.clone(), "0.2")?;

        // epoch 3
        test.advance_mixnet_epoch()?;
        test.insert_raw_performance(&nm, node_id, measurement_kind.clone(), "0.3")?;

        // epoch 4
        test.advance_mixnet_epoch()?;
        test.insert_raw_performance(&nm, node_id, measurement_kind.clone(), "0.4")?;

        // epoch 5
        test.advance_mixnet_epoch()?;
        test.insert_raw_performance(&nm, node_id, measurement_kind.clone(), "0.5")?;

        let deps = test.deps();
        let res = query_node_performance_paged(deps, node_id, Some(5), None)?;
        assert!(res.start_next_after.is_none());
        assert!(res.performance.is_empty());

        let res = query_node_performance_paged(deps, node_id, Some(42), None)?;
        assert!(res.start_next_after.is_none());
        assert!(res.performance.is_empty());

        let res = query_node_performance_paged(deps, node_id, Some(4), None)?;
        assert!(res.start_next_after.is_none());
        assert_eq!(
            res.performance,
            vec![epoch_node_performance_unchecked(
                5,
                measurement_kind.clone(),
                "0.5"
            )]
        );

        let res = query_node_performance_paged(deps, node_id, Some(2), None)?;
        assert!(res.start_next_after.is_none());
        assert_eq!(
            res.performance,
            vec![
                epoch_node_performance_unchecked(3, measurement_kind.clone(), "0.3"),
                epoch_node_performance_unchecked(4, measurement_kind.clone(), "0.4"),
                epoch_node_performance_unchecked(5, measurement_kind.clone(), "0.5"),
            ]
        );

        let res = query_node_performance_paged(deps, node_id, None, None)?;
        assert!(res.start_next_after.is_none());
        assert_eq!(
            res.performance,
            vec![
                epoch_node_performance_unchecked(0, measurement_kind.clone(), "0"),
                epoch_node_performance_unchecked(1, measurement_kind.clone(), "0.1"),
                epoch_node_performance_unchecked(2, measurement_kind.clone(), "0.2"),
                epoch_node_performance_unchecked(3, measurement_kind.clone(), "0.3"),
                epoch_node_performance_unchecked(4, measurement_kind.clone(), "0.4"),
                epoch_node_performance_unchecked(5, measurement_kind.clone(), "0.5"),
            ]
        );

        let res = query_node_performance_paged(deps, node_id, Some(2), Some(1))?;
        assert_eq!(res.start_next_after, Some(3));
        assert_eq!(
            res.performance,
            vec![epoch_node_performance_unchecked(
                3,
                measurement_kind.clone(),
                "0.3"
            )]
        );

        Ok(())
    }

    #[test]
    fn querying_epoch_performance_paged() -> anyhow::Result<()> {
        let mut test = init_contract_tester();

        let nm = test.generate_account();
        test.authorise_network_monitor(&nm)?;
        let measurement_kind = test.define_dummy_measurement_kind().unwrap();

        let mut nodes = Vec::new();
        for _ in 0..10 {
            nodes.push(test.bond_dummy_nymnode()?);
        }

        let epoch_id = 5;
        test.set_mixnet_epoch(epoch_id)?;

        test.insert_raw_performance(&nm, nodes[1], measurement_kind.clone(), "0.1")?;
        test.insert_raw_performance(&nm, nodes[2], measurement_kind.clone(), "0.2")?;
        test.insert_raw_performance(&nm, nodes[3], measurement_kind.clone(), "0.3")?;
        // 4 is missing
        test.insert_raw_performance(&nm, nodes[5], measurement_kind.clone(), "0.5")?;
        test.insert_raw_performance(&nm, nodes[6], measurement_kind.clone(), "0.6")?;

        let deps = test.deps();
        let res = query_epoch_performance_paged(deps, epoch_id, Some(nodes[6]), None)?;
        assert!(res.start_next_after.is_none());
        assert!(res.performance.is_empty());

        let res = query_epoch_performance_paged(deps, epoch_id, Some(42), None)?;
        assert!(res.start_next_after.is_none());
        assert!(res.performance.is_empty());

        let res = query_epoch_performance_paged(deps, epoch_id, Some(nodes[4]), None)?;
        assert_eq!(res.start_next_after, Some(nodes[6]));
        assert_eq!(
            res.performance,
            vec![
                NodePerformance {
                    node_id: nodes[5],
                    performance: "0.5".parse()?,
                    measurement_kind: measurement_kind.clone()
                },
                NodePerformance {
                    node_id: nodes[6],
                    performance: "0.6".parse()?,
                    measurement_kind: measurement_kind.clone()
                }
            ]
        );
        let res = query_epoch_performance_paged(deps, epoch_id, Some(nodes[3]), None)?;
        assert_eq!(res.start_next_after, Some(nodes[6]));
        assert_eq!(
            res.performance,
            vec![
                NodePerformance {
                    node_id: nodes[5],
                    performance: "0.5".parse()?,
                    measurement_kind: measurement_kind.clone()
                },
                NodePerformance {
                    node_id: nodes[6],
                    performance: "0.6".parse()?,
                    measurement_kind: measurement_kind.clone()
                }
            ]
        );

        let res = query_epoch_performance_paged(deps, epoch_id, Some(nodes[2]), None)?;
        assert_eq!(res.start_next_after, Some(nodes[6]));
        assert_eq!(
            res.performance,
            vec![
                NodePerformance {
                    node_id: nodes[3],
                    performance: "0.3".parse()?,
                    measurement_kind: measurement_kind.clone()
                },
                NodePerformance {
                    node_id: nodes[5],
                    performance: "0.5".parse()?,
                    measurement_kind: measurement_kind.clone()
                },
                NodePerformance {
                    node_id: nodes[6],
                    performance: "0.6".parse()?,
                    measurement_kind: measurement_kind.clone()
                }
            ]
        );

        let res = query_epoch_performance_paged(deps, epoch_id, None, None)?;
        assert_eq!(res.start_next_after, Some(nodes[6]));
        assert_eq!(
            res.performance,
            vec![
                NodePerformance {
                    node_id: nodes[1],
                    performance: "0.1".parse()?,
                    measurement_kind: measurement_kind.clone()
                },
                NodePerformance {
                    node_id: nodes[2],
                    performance: "0.2".parse()?,
                    measurement_kind: measurement_kind.clone()
                },
                NodePerformance {
                    node_id: nodes[3],
                    performance: "0.3".parse()?,
                    measurement_kind: measurement_kind.clone()
                },
                NodePerformance {
                    node_id: nodes[5],
                    performance: "0.5".parse()?,
                    measurement_kind: measurement_kind.clone()
                },
                NodePerformance {
                    node_id: nodes[6],
                    performance: "0.6".parse()?,
                    measurement_kind: measurement_kind.clone()
                }
            ]
        );

        let res = query_epoch_performance_paged(deps, epoch_id, Some(nodes[2]), Some(1))?;
        assert_eq!(res.start_next_after, Some(nodes[3]));
        assert_eq!(
            res.performance,
            vec![NodePerformance {
                node_id: nodes[3],
                performance: "0.3".parse()?,
                measurement_kind: measurement_kind.clone()
            }]
        );

        Ok(())
    }

    #[test]
    fn querying_epoch_measurements_paged() -> anyhow::Result<()> {
        let mut test = init_contract_tester();

        let nm = test.generate_account();
        test.authorise_network_monitor(&nm)?;
        let measurement_kind = test.define_dummy_measurement_kind().unwrap();

        let mut nodes = Vec::new();
        for _ in 0..10 {
            nodes.push(test.bond_dummy_nymnode()?);
        }

        let epoch_id = 5;
        test.set_mixnet_epoch(epoch_id)?;

        test.insert_raw_performance(&nm, nodes[1], measurement_kind.clone(), "0.1")?;
        test.insert_raw_performance(&nm, nodes[2], measurement_kind.clone(), "0.2")?;
        test.insert_raw_performance(&nm, nodes[3], measurement_kind.clone(), "0.3")?;
        // 4 is missing
        test.insert_raw_performance(&nm, nodes[5], measurement_kind.clone(), "0.5")?;
        test.insert_raw_performance(&nm, nodes[6], measurement_kind.clone(), "0.6")?;

        let deps = test.deps();

        // query starting after nodes[6]
        let res = query_epoch_measurements_paged(deps, epoch_id, Some(nodes[6]), None)?;
        assert!(res.start_next_after.is_none());
        assert!(res.measurements.is_empty());

        // query after non-existent high node ID
        let res = query_epoch_measurements_paged(deps, epoch_id, Some(42), None)?;
        assert!(res.start_next_after.is_none());
        assert!(res.measurements.is_empty());

        // query starting after nodes[4] (should return nodes 5 and 6)
        let res = query_epoch_measurements_paged(deps, epoch_id, Some(nodes[4]), None)?;
        assert_eq!(res.start_next_after, Some(nodes[6]));
        assert_eq!(res.measurements.len(), 2);

        assert_eq!(res.measurements[0].node_id, nodes[5]);
        assert_eq!(res.measurements[1].node_id, nodes[6]);

        // verify returned data against raw results
        let node5_results = res.measurements[0]
            .measurements_per_kind
            .get(&measurement_kind)
            .unwrap();
        let expected_results =
            test.read_raw_scores(epoch_id, nodes[5], measurement_kind.clone())?;
        assert_eq!(node5_results.inner(), expected_results.inner());

        let node6_results = res.measurements[1]
            .measurements_per_kind
            .get(&measurement_kind)
            .unwrap();
        let expected_results =
            test.read_raw_scores(epoch_id, nodes[6], measurement_kind.clone())?;
        assert_eq!(node6_results.inner(), expected_results.inner());

        // query starting after nodes[3]
        // should skip nodes[3] entirely and start from nodes[5] (nodes[4] doesn't exist)
        let res = query_epoch_measurements_paged(deps, epoch_id, Some(nodes[3]), None)?;
        assert_eq!(res.start_next_after, Some(nodes[6]));
        // Verify only nodes[5] and nodes[6] are present (nodes[3] is skipped)
        assert_eq!(res.measurements.len(), 2);
        assert_eq!(res.measurements[0].node_id, nodes[5]);
        assert_eq!(res.measurements[1].node_id, nodes[6]);

        // query with start_after = nodes[2]
        let res = query_epoch_measurements_paged(deps, epoch_id, Some(nodes[2]), None)?;
        assert_eq!(res.start_next_after, Some(nodes[6]));
        assert_eq!(res.measurements.len(), 3);
        // only nodes[3], nodes[5], nodes[6] are present (nodes[2] is skipped)
        assert_eq!(res.measurements[0].node_id, nodes[3]);
        assert_eq!(res.measurements[1].node_id, nodes[5]);
        assert_eq!(res.measurements[2].node_id, nodes[6]);

        // measurements HashMap structure for all nodes
        for measurement in &res.measurements {
            assert!(
                measurement
                    .measurements_per_kind
                    .contains_key(&measurement_kind)
            );
        }

        // query from beginning (no start_after) - should return all nodes
        let res = query_epoch_measurements_paged(deps, epoch_id, None, None)?;
        assert_eq!(res.start_next_after, Some(nodes[6]));
        assert_eq!(res.measurements.len(), 5);
        // verify all expected nodes are present IN SORTED ORDER
        assert_eq!(res.measurements[0].node_id, nodes[1]);
        assert_eq!(res.measurements[1].node_id, nodes[2]);
        assert_eq!(res.measurements[2].node_id, nodes[3]);
        assert_eq!(res.measurements[3].node_id, nodes[5]);
        assert_eq!(res.measurements[4].node_id, nodes[6]);

        // query with custom limit
        // With limit=1, we fetch 1 storage item starting from nodes[3] (nodes[2] + 1)
        let res = query_epoch_measurements_paged(deps, epoch_id, Some(nodes[2]), Some(1))?;
        assert_eq!(res.start_next_after, Some(nodes[3]));
        assert_eq!(res.measurements.len(), 1);
        assert_eq!(res.measurements[0].node_id, nodes[3]);

        Ok(())
    }

    #[test]
    fn querying_epoch_measurements_paged_multiple_kinds() -> anyhow::Result<()> {
        let mut test = init_contract_tester();

        // Use two different network monitors for different measurement kinds
        let nm1 = test.generate_account();
        let nm2 = test.generate_account();
        test.authorise_network_monitor(&nm1)?;
        test.authorise_network_monitor(&nm2)?;

        // define two different measurement kinds
        let admin = test.admin_unchecked();
        let kind1 = String::from("mixnet");
        let kind2 = String::from("wireguard");

        test.execute_raw(
            admin.clone(),
            ExecuteMsg::DefineMeasurementKind {
                measurement_kind: kind1.clone(),
            },
        )?;
        test.execute_raw(
            admin,
            ExecuteMsg::DefineMeasurementKind {
                measurement_kind: kind2.clone(),
            },
        )?;

        // bond some nodes
        let node1 = test.bond_dummy_nymnode()?;
        let node2 = test.bond_dummy_nymnode()?;
        let node3 = test.bond_dummy_nymnode()?;

        let epoch_id = 10;
        test.set_mixnet_epoch(epoch_id)?;

        // both measurement kinds (different network monitors)
        test.insert_raw_performance(&nm1, node1, kind1.clone(), "0.11")?;
        test.insert_raw_performance(&nm2, node1, kind2.clone(), "0.12")?;

        // only first kind
        test.insert_raw_performance(&nm1, node2, kind1.clone(), "0.21")?;

        // both kinds (different network monitors)
        test.insert_raw_performance(&nm1, node3, kind1.clone(), "0.31")?;
        test.insert_raw_performance(&nm2, node3, kind2.clone(), "0.32")?;

        let deps = test.deps();

        // query all measurements for this epoch
        let res = query_epoch_measurements_paged(deps, epoch_id, None, None)?;
        assert_eq!(res.epoch_id, epoch_id);
        assert_eq!(res.measurements.len(), 3);

        assert_eq!(res.measurements[0].node_id, node1);
        assert_eq!(res.measurements[1].node_id, node2);
        assert_eq!(res.measurements[2].node_id, node3);

        let node1_measurements = &res.measurements[0];
        let node2_measurements = &res.measurements[1];
        let node3_measurements = &res.measurements[2];

        // verify node 1 has 2 measurement kinds
        assert_eq!(node1_measurements.measurements_per_kind.len(), 2);
        assert!(
            node1_measurements
                .measurements_per_kind
                .contains_key(&kind1)
        );
        assert!(
            node1_measurements
                .measurements_per_kind
                .contains_key(&kind2)
        );

        // raw data for node 1
        let node1_kind1_results = node1_measurements
            .measurements_per_kind
            .get(&kind1)
            .unwrap();
        let expected = test.read_raw_scores(epoch_id, node1, kind1.clone())?;
        assert_eq!(node1_kind1_results.inner(), expected.inner());

        let node1_kind2_results = node1_measurements
            .measurements_per_kind
            .get(&kind2)
            .unwrap();
        let expected = test.read_raw_scores(epoch_id, node1, kind2.clone())?;
        assert_eq!(node1_kind2_results.inner(), expected.inner());

        // node 2 has only 1 measurement kind
        assert_eq!(node2_measurements.measurements_per_kind.len(), 1);
        assert!(
            node2_measurements
                .measurements_per_kind
                .contains_key(&kind1)
        );
        assert!(
            !node2_measurements
                .measurements_per_kind
                .contains_key(&kind2)
        );

        // raw data for node 2
        let node2_kind1_results = node2_measurements
            .measurements_per_kind
            .get(&kind1)
            .unwrap();
        let expected = test.read_raw_scores(epoch_id, node2, kind1.clone())?;
        assert_eq!(node2_kind1_results.inner(), expected.inner());

        // node 3 has 2 measurement kinds
        assert_eq!(node3_measurements.measurements_per_kind.len(), 2);
        assert!(
            node3_measurements
                .measurements_per_kind
                .contains_key(&kind1)
        );
        assert!(
            node3_measurements
                .measurements_per_kind
                .contains_key(&kind2)
        );

        // raw data for node 3
        let node3_kind1_results = node3_measurements
            .measurements_per_kind
            .get(&kind1)
            .unwrap();
        let expected = test.read_raw_scores(epoch_id, node3, kind1.clone())?;
        assert_eq!(node3_kind1_results.inner(), expected.inner());

        let node3_kind2_results = node3_measurements
            .measurements_per_kind
            .get(&kind2)
            .unwrap();
        let expected = test.read_raw_scores(epoch_id, node3, kind2.clone())?;
        assert_eq!(node3_kind2_results.inner(), expected.inner());

        // pagination with multiple kinds - query after node1
        let res = query_epoch_measurements_paged(deps, epoch_id, Some(node1), None)?;
        assert_eq!(res.measurements.len(), 2); // node2 and node3 only (node1 is skipped)
        assert_eq!(res.measurements[0].node_id, node2);
        assert_eq!(res.measurements[1].node_id, node3);
        assert_eq!(res.measurements[0].measurements_per_kind.len(), 1); // only latency
        assert_eq!(res.measurements[1].measurements_per_kind.len(), 2); // both kinds

        // pagination after node2 - should only return node3
        let res = query_epoch_measurements_paged(deps, epoch_id, Some(node2), None)?;
        assert_eq!(res.measurements.len(), 1); // only node3
        assert_eq!(res.measurements[0].node_id, node3);
        assert_eq!(res.measurements[0].measurements_per_kind.len(), 2); // both kinds

        Ok(())
    }

    #[test]
    fn last_submission_query() -> anyhow::Result<()> {
        let mut test = init_contract_tester();

        let env = test.env();

        let id1 = test.bond_dummy_nymnode()?;
        let id2 = test.bond_dummy_nymnode()?;

        // initial
        let data = query_last_submission(test.deps())?;
        assert_eq!(
            data,
            LastSubmission {
                block_height: env.block.height,
                block_time: env.block.time,
                data: None,
            }
        );

        let nm1 = test.generate_account();
        let nm2 = test.generate_account();
        test.authorise_network_monitor(&nm1)?;
        test.authorise_network_monitor(&nm2)?;
        test.set_mixnet_epoch(10)?;
        let measurement_kind = test.define_dummy_measurement_kind().unwrap();

        test.insert_raw_performance(&nm1, id1, measurement_kind.clone(), "0.2")?;

        let data = query_last_submission(test.deps())?;
        assert_eq!(
            data,
            LastSubmission {
                block_height: env.block.height,
                block_time: env.block.time,
                data: Some(LastSubmittedData {
                    sender: nm1.clone(),
                    epoch_id: 10,
                    data: NodePerformance {
                        node_id: id1,
                        performance: "0.2".parse()?,
                        measurement_kind: measurement_kind.clone(),
                    },
                }),
            }
        );

        test.next_block();
        let env = test.env();

        test.insert_epoch_performance(&nm2, 5, id2, measurement_kind.clone(), "0.3".parse()?)?;

        // note that even though it's "earlier" data, last submission is still updated accordingly
        let data = query_last_submission(test.deps())?;
        assert_eq!(
            data,
            LastSubmission {
                block_height: env.block.height,
                block_time: env.block.time,
                data: Some(LastSubmittedData {
                    sender: nm2.clone(),
                    epoch_id: 5,
                    data: NodePerformance {
                        node_id: id2,
                        performance: "0.3".parse()?,
                        measurement_kind: measurement_kind.clone(),
                    },
                }),
            }
        );

        Ok(())
    }

    #[test]
    #[ignore]
    // TODO uncomment test:
    // currently logic for stale submission doesn't work well with different measurement kinds
    fn last_submission_query_multiple_kinds() -> anyhow::Result<()> {
        let mut test = init_contract_tester();
        let env = test.env();

        // Bond one node and authorize one monitor
        let id1 = test.bond_dummy_nymnode()?;
        let nm1 = test.generate_account();
        test.authorise_network_monitor(&nm1)?;
        test.set_mixnet_epoch(10)?;

        // Define TWO measurement kinds
        let measurement_mixnet = MeasurementKind::from("mixnet");
        let measurement_dvpn = MeasurementKind::from("dvpn");
        let admin = test.admin_unchecked();
        test.execute_raw(
            admin.clone(),
            ExecuteMsg::DefineMeasurementKind {
                measurement_kind: measurement_dvpn.clone(),
            },
        )?;
        test.execute_raw(
            admin,
            ExecuteMsg::DefineMeasurementKind {
                measurement_kind: measurement_mixnet.clone(),
            },
        )?;

        // no submissions yet
        let data = query_last_submission(test.deps())?;
        assert_eq!(
            data,
            LastSubmission {
                block_height: env.block.height,
                block_time: env.block.time,
                data: None,
            }
        );

        // Submit first measurement kind in epoch 10
        test.insert_raw_performance(&nm1, id1, measurement_dvpn.clone(), "0.75")?;

        let data = query_last_submission(test.deps())?;
        assert_eq!(
            data,
            LastSubmission {
                block_height: env.block.height,
                block_time: env.block.time,
                data: Some(LastSubmittedData {
                    sender: nm1.clone(),
                    epoch_id: 10,
                    data: NodePerformance {
                        node_id: id1,
                        performance: "0.75".parse()?,
                        measurement_kind: measurement_dvpn.clone(),
                    },
                }),
            }
        );

        let env = test.env();

        // submit second measurement kind: same monitor, same node, same epoch
        test.insert_raw_performance(&nm1, id1, measurement_mixnet.clone(), "0.85")?;

        // verify that last submission is updated with the new measurement kind
        let data = query_last_submission(test.deps())?;
        assert_eq!(
            data,
            LastSubmission {
                block_height: env.block.height,
                block_time: env.block.time,
                data: Some(LastSubmittedData {
                    sender: nm1.clone(),
                    epoch_id: 10,
                    data: NodePerformance {
                        node_id: id1,
                        performance: "0.85".parse()?,
                        measurement_kind: measurement_mixnet.clone(),
                    },
                }),
            }
        );

        // verify both measurements are stored independently in the same epoch
        let bandwidth_results = test.read_raw_scores(10, id1, measurement_dvpn.clone())?;
        assert_eq!(bandwidth_results.inner().len(), 1);
        assert_eq!(bandwidth_results.inner()[0], "0.75".parse()?);

        let latency_results = test.read_raw_scores(10, id1, measurement_mixnet.clone())?;
        assert_eq!(latency_results.inner().len(), 1);
        assert_eq!(latency_results.inner()[0], "0.85".parse()?);

        Ok(())
    }
}

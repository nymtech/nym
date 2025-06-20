// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::{retrieval_limits, NYM_PERFORMANCE_CONTRACT_STORAGE};
use cosmwasm_std::{Addr, Deps, Order, StdResult};
use cw_controllers::AdminResponse;
use cw_storage_plus::Bound;
use nym_performance_contract_common::{
    EpochId, EpochMeasurementsPagedResponse, EpochNodePerformance, EpochPerformancePagedResponse,
    FullHistoricalPerformancePagedResponse, HistoricalPerformance, NetworkMonitorInformation,
    NetworkMonitorResponse, NetworkMonitorsPagedResponse, NodeId, NodeMeasurement,
    NodeMeasurementsResponse, NodePerformance, NodePerformancePagedResponse,
    NodePerformanceResponse, NymPerformanceContractError, RetiredNetworkMonitorsPagedResponse,
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

pub fn query_node_measurements(
    deps: Deps,
    epoch_id: EpochId,
    node_id: NodeId,
) -> Result<NodeMeasurementsResponse, NymPerformanceContractError> {
    let measurements = NYM_PERFORMANCE_CONTRACT_STORAGE
        .performance_results
        .results
        .may_load(deps.storage, (epoch_id, node_id))?;
    Ok(NodeMeasurementsResponse { measurements })
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

    let start = start_after.map(Bound::exclusive);

    let performance = NYM_PERFORMANCE_CONTRACT_STORAGE
        .performance_results
        .results
        .prefix(epoch_id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|record| {
            record.map(|(node_id, results)| NodePerformance {
                node_id,
                performance: results.median(),
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

    let start = start_after.map(Bound::exclusive);

    let measurements = NYM_PERFORMANCE_CONTRACT_STORAGE
        .performance_results
        .results
        .prefix(epoch_id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|record| {
            record.map(|(node_id, measurements)| NodeMeasurement {
                node_id,
                measurements,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = measurements.last().map(|last| last.node_id);

    Ok(EpochMeasurementsPagedResponse {
        epoch_id,
        measurements,
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

    let start = start_after.map(Bound::exclusive);

    let performance = NYM_PERFORMANCE_CONTRACT_STORAGE
        .performance_results
        .results
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|record| {
            record.map(|((epoch_id, node_id), results)| HistoricalPerformance {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{init_contract_tester, PerformanceContractTesterExt};
    use nym_contracts_common_testing::{ContractOpts, RandExt};

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
    fn querying_node_performance_paged() -> anyhow::Result<()> {
        let mut test = init_contract_tester();

        let node_id = test.bond_dummy_nymnode()?;
        let nm = test.generate_account();
        test.authorise_network_monitor(&nm)?;

        // epoch 0
        test.insert_raw_performance(&nm, node_id, "0")?;

        // epoch 1
        test.advance_mixnet_epoch()?;
        test.insert_raw_performance(&nm, node_id, "0.1")?;

        // epoch 2
        test.advance_mixnet_epoch()?;
        test.insert_raw_performance(&nm, node_id, "0.2")?;

        // epoch 3
        test.advance_mixnet_epoch()?;
        test.insert_raw_performance(&nm, node_id, "0.3")?;

        // epoch 4
        test.advance_mixnet_epoch()?;
        test.insert_raw_performance(&nm, node_id, "0.4")?;

        // epoch 5
        test.advance_mixnet_epoch()?;
        test.insert_raw_performance(&nm, node_id, "0.5")?;

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
            vec![EpochNodePerformance {
                epoch: 5,
                performance: Some("0.5".parse()?),
            }]
        );

        let res = query_node_performance_paged(deps, node_id, Some(2), None)?;
        assert!(res.start_next_after.is_none());
        assert_eq!(
            res.performance,
            vec![
                EpochNodePerformance {
                    epoch: 3,
                    performance: Some("0.3".parse()?),
                },
                EpochNodePerformance {
                    epoch: 4,
                    performance: Some("0.4".parse()?),
                },
                EpochNodePerformance {
                    epoch: 5,
                    performance: Some("0.5".parse()?),
                }
            ]
        );

        let res = query_node_performance_paged(deps, node_id, None, None)?;
        assert!(res.start_next_after.is_none());
        assert_eq!(
            res.performance,
            vec![
                EpochNodePerformance {
                    epoch: 0,
                    performance: Some("0".parse()?),
                },
                EpochNodePerformance {
                    epoch: 1,
                    performance: Some("0.1".parse()?),
                },
                EpochNodePerformance {
                    epoch: 2,
                    performance: Some("0.2".parse()?),
                },
                EpochNodePerformance {
                    epoch: 3,
                    performance: Some("0.3".parse()?),
                },
                EpochNodePerformance {
                    epoch: 4,
                    performance: Some("0.4".parse()?),
                },
                EpochNodePerformance {
                    epoch: 5,
                    performance: Some("0.5".parse()?),
                }
            ]
        );

        let res = query_node_performance_paged(deps, node_id, Some(2), Some(1))?;
        assert_eq!(res.start_next_after, Some(3));
        assert_eq!(
            res.performance,
            vec![EpochNodePerformance {
                epoch: 3,
                performance: Some("0.3".parse()?),
            }]
        );

        Ok(())
    }

    #[test]
    fn querying_epoch_performance_paged() -> anyhow::Result<()> {
        let mut test = init_contract_tester();

        let nm = test.generate_account();
        test.authorise_network_monitor(&nm)?;

        let mut nodes = Vec::new();
        for _ in 0..10 {
            nodes.push(test.bond_dummy_nymnode()?);
        }

        let epoch_id = 5;
        test.set_mixnet_epoch(epoch_id)?;

        test.insert_raw_performance(&nm, nodes[1], "0.1")?;
        test.insert_raw_performance(&nm, nodes[2], "0.2")?;
        test.insert_raw_performance(&nm, nodes[3], "0.3")?;
        // 4 is missing
        test.insert_raw_performance(&nm, nodes[5], "0.5")?;
        test.insert_raw_performance(&nm, nodes[6], "0.6")?;

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
                },
                NodePerformance {
                    node_id: nodes[6],
                    performance: "0.6".parse()?,
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
                },
                NodePerformance {
                    node_id: nodes[6],
                    performance: "0.6".parse()?,
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
                },
                NodePerformance {
                    node_id: nodes[5],
                    performance: "0.5".parse()?,
                },
                NodePerformance {
                    node_id: nodes[6],
                    performance: "0.6".parse()?,
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
                },
                NodePerformance {
                    node_id: nodes[2],
                    performance: "0.2".parse()?,
                },
                NodePerformance {
                    node_id: nodes[3],
                    performance: "0.3".parse()?,
                },
                NodePerformance {
                    node_id: nodes[5],
                    performance: "0.5".parse()?,
                },
                NodePerformance {
                    node_id: nodes[6],
                    performance: "0.6".parse()?,
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
            }]
        );

        Ok(())
    }
}

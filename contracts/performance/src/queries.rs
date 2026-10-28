// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::{retrieval_limits, NYM_PERFORMANCE_CONTRACT_STORAGE};
use cosmwasm_std::{Addr, Deps, Order, StdResult};
use cw_controllers::AdminResponse;
use cw_storage_plus::Bound;
use nym_performance_contract_common::{
    EpochId, EpochMeasurementsPagedResponse, EpochPerformancePagedResponse, EpochWeights,
    FullHistoricalPerformancePagedResponse, HistoricalPerformance, LastKnownEpochResponse,
    LastSubmission, NetworkMonitorInformation, NetworkMonitorResponse,
    NetworkMonitorsPagedResponse, NodeId, NodeMeasurements, NodeMeasurementsResponse,
    NodePerformance, NodePerformancePagedResponse, NodePerformanceResponse,
    NymPerformanceContractError, RetiredNetworkMonitorsPagedResponse, RewardingInputsResponse,
    RewardingScoreResponse, WeightsResponse,
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
    let limit = limit
        .unwrap_or(retrieval_limits::NODE_PERFORMANCE_DEFAULT_LIMIT)
        .min(retrieval_limits::NODE_PERFORMANCE_MAX_LIMIT) as usize;

    let start = match start_after {
        None => NYM_PERFORMANCE_CONTRACT_STORAGE
            .mixnet_epoch_id_at_creation
            .load(deps.storage)?,
        Some(start_after) => start_after.saturating_add(1),
    };

    // the history ends at the node's last-known epoch, so no page walks past it; without a
    // pointer the node has never been measured and there is nothing to page through
    let Some(last_known) = NYM_PERFORMANCE_CONTRACT_STORAGE
        .performance_results
        .last_known_epoch
        .may_load(deps.storage, node_id)?
    else {
        return Ok(NodePerformancePagedResponse {
            node_id,
            performance: Vec::new(),
            start_next_after: None,
        });
    };

    // the limit bounds the epochs visited, which keeps a page's cost fixed; epochs without a
    // bundle (monitor outages) are walked over but not reported
    let mut performance = Vec::new();
    let mut last_visited = None;
    for epoch_id in (start..=last_known).take(limit) {
        last_visited = Some(epoch_id);
        if let Some(epoch_performance) = NYM_PERFORMANCE_CONTRACT_STORAGE.try_load_performance(
            deps.storage,
            epoch_id,
            node_id,
        )? {
            performance.push(epoch_performance);
        }
    }

    // there is more only when the page stopped short of the pointer
    let start_next_after = last_visited.filter(|&last| last < last_known);

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

    // the weights are a property of the epoch, so one lookup serves every node on the page
    let weights = NYM_PERFORMANCE_CONTRACT_STORAGE.weights_at(deps.storage, epoch_id)?;

    let performance = NYM_PERFORMANCE_CONTRACT_STORAGE
        .performance_results
        .results
        .prefix(epoch_id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|record| {
            record.map(|(node_id, bundle)| {
                let medians = bundle.medians();
                NodePerformance {
                    node_id,
                    medians,
                    score: weights
                        .as_ref()
                        .and_then(|weights| weights.weights.score(medians)),
                }
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
            record.map(|(node_id, measurements)| NodeMeasurements {
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

    // entries arrive in (epoch, node) order, so the weights are looked up once per run of
    // entries sharing an epoch rather than once per entry
    let mut weights_epoch: Option<EpochId> = None;
    let mut weights: Option<EpochWeights> = None;

    let mut performance = Vec::new();
    for record in NYM_PERFORMANCE_CONTRACT_STORAGE
        .performance_results
        .results
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
    {
        let ((epoch_id, node_id), bundle) = record?;
        if weights_epoch != Some(epoch_id) {
            weights = NYM_PERFORMANCE_CONTRACT_STORAGE.weights_at(deps.storage, epoch_id)?;
            weights_epoch = Some(epoch_id);
        }

        let medians = bundle.medians();
        performance.push(HistoricalPerformance {
            epoch_id,
            node_id,
            medians,
            score: weights
                .as_ref()
                .and_then(|weights| weights.weights.score(medians)),
        });
    }

    let start_next_after = performance.last().map(|last| (last.epoch_id, last.node_id));

    Ok(FullHistoricalPerformancePagedResponse {
        performance,
        start_next_after,
    })
}

pub fn query_rewarding_inputs(
    deps: Deps,
    epoch_id: EpochId,
    node_id: NodeId,
) -> Result<RewardingInputsResponse, NymPerformanceContractError> {
    NYM_PERFORMANCE_CONTRACT_STORAGE.resolve_rewarding_inputs(deps.storage, epoch_id, node_id)
}

/// The score field of `query_rewarding_inputs`, for a consumer that needs only the value.
pub fn query_rewarding_score(
    deps: Deps,
    epoch_id: EpochId,
    node_id: NodeId,
) -> Result<RewardingScoreResponse, NymPerformanceContractError> {
    let score = query_rewarding_inputs(deps, epoch_id, node_id)?.score;
    Ok(RewardingScoreResponse { score })
}

pub fn query_last_known_epoch(
    deps: Deps,
    node_id: NodeId,
) -> Result<LastKnownEpochResponse, NymPerformanceContractError> {
    let epoch_id = NYM_PERFORMANCE_CONTRACT_STORAGE
        .performance_results
        .last_known_epoch
        .may_load(deps.storage, node_id)?;
    Ok(LastKnownEpochResponse { epoch_id })
}

pub fn query_weights_at(
    deps: Deps,
    epoch_id: EpochId,
) -> Result<WeightsResponse, NymPerformanceContractError> {
    let weights = NYM_PERFORMANCE_CONTRACT_STORAGE.weights_at(deps.storage, epoch_id)?;
    Ok(WeightsResponse { weights })
}

pub fn query_current_weights(deps: Deps) -> Result<WeightsResponse, NymPerformanceContractError> {
    let current_epoch_id = NYM_PERFORMANCE_CONTRACT_STORAGE.current_mixnet_epoch_id(deps)?;
    query_weights_at(deps, current_epoch_id)
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

    #[cfg(test)]
    mod rewarding_queries {
        use super::*;
        use crate::testing::{
            init_contract_tester, liveness_only_weights, p, scored_submission,
            PerformanceContractTesterExt,
        };
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
        use nym_contracts_common_testing::{AdminExt, ContractOpts};
        use nym_performance_contract_common::constants::MAX_FALLBACK_LOOKBACK_EPOCHS;
        use nym_performance_contract_common::{EpochWeights, ExecuteMsg, Weights};

        #[test]
        fn the_score_query_is_the_score_of_the_inputs_query() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let nm = test.new_authorised_network_monitor();
            let node_id = test.bond_dummy_nymnode()?;
            test.submit_at_epoch(&nm, 10, scored_submission(node_id, "0.8", "1"));

            // a direct hit, a fallback, and a request with nothing within the lookback
            let beyond_lookback = 10 + MAX_FALLBACK_LOOKBACK_EPOCHS + 1;
            for (epoch_id, source) in [(10, Some(10)), (12, Some(10)), (beyond_lookback, None)] {
                let inputs = query_rewarding_inputs(test.deps(), epoch_id, node_id)?;
                assert_eq!(inputs.source.as_ref().map(|source| source.epoch_id), source);

                let score = query_rewarding_score(test.deps(), epoch_id, node_id)?;
                assert_eq!(score.score, inputs.score);
            }

            // and the values are the expected ones, not merely equal to each other
            assert_eq!(
                query_rewarding_score(test.deps(), 12, node_id)?.score,
                Some(p("0.8"))
            );
            assert_eq!(
                query_rewarding_score(test.deps(), beyond_lookback, node_id)?.score,
                None
            );

            Ok(())
        }

        #[test]
        fn last_known_epoch_exposes_the_pointer() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let nm = test.new_authorised_network_monitor();
            let node_id = test.bond_dummy_nymnode()?;

            assert_eq!(query_last_known_epoch(test.deps(), node_id)?.epoch_id, None);

            test.submit_at_epoch(&nm, 7, scored_submission(node_id, "0.8", "1"));
            assert_eq!(
                query_last_known_epoch(test.deps(), node_id)?.epoch_id,
                Some(7)
            );

            Ok(())
        }

        #[test]
        fn weights_queries_resolve_per_epoch() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            test.set_mixnet_epoch(10)?;

            let updated = Weights {
                liveness: p("0.7"),
                stress: p("0.3"),
            };
            test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::UpdateWeights { weights: updated },
            )?;

            // the running epoch keeps the creation weights, the next one carries the update
            let creation = Some(EpochWeights {
                effective_from: 0,
                weights: liveness_only_weights(),
            });
            assert_eq!(query_weights_at(test.deps(), 10)?.weights, creation);
            assert_eq!(query_current_weights(test.deps())?.weights, creation);
            assert_eq!(
                query_weights_at(test.deps(), 11)?.weights,
                Some(EpochWeights {
                    effective_from: 11,
                    weights: updated,
                })
            );

            // once the mixnet moves on, the current weights follow
            test.set_mixnet_epoch(11)?;
            assert_eq!(
                query_current_weights(test.deps())?
                    .weights
                    .map(|weights| weights.effective_from),
                Some(11)
            );

            Ok(())
        }
    }

    #[cfg(test)]
    mod node_history {
        use super::*;
        use crate::testing::{
            init_contract_tester, p, scored_submission, PerformanceContractTesterExt,
        };
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
        use nym_contracts_common::Percent;
        use nym_contracts_common_testing::ContractOpts;

        /// The page reduced to what the assertions care about.
        fn epochs_and_scores(
            res: &NodePerformancePagedResponse,
        ) -> Vec<(EpochId, Option<Percent>)> {
            res.performance
                .iter()
                .map(|entry| (entry.epoch_id, entry.score))
                .collect()
        }

        #[test]
        fn querying_node_performance_paged() -> anyhow::Result<()> {
            let mut test = init_contract_tester();

            let node_id = test.bond_dummy_nymnode()?;
            let nm = test.new_authorised_network_monitor();

            // one scored bundle per epoch from 0 to 5; config 100% makes the score the liveness
            for (epoch, liveness) in ["0", "0.1", "0.2", "0.3", "0.4", "0.5"].iter().enumerate() {
                if epoch > 0 {
                    test.advance_mixnet_epoch()?;
                }
                test.submit_scored(&nm, node_id, liveness);
            }

            let deps = test.deps();
            let res = query_node_performance_paged(deps, node_id, Some(5), None)?;
            assert!(res.start_next_after.is_none());
            assert!(res.performance.is_empty());

            let res = query_node_performance_paged(deps, node_id, Some(42), None)?;
            assert!(res.start_next_after.is_none());
            assert!(res.performance.is_empty());

            let res = query_node_performance_paged(deps, node_id, Some(4), None)?;
            assert!(res.start_next_after.is_none());
            assert_eq!(epochs_and_scores(&res), vec![(5, Some(p("0.5")))]);

            let res = query_node_performance_paged(deps, node_id, Some(2), None)?;
            assert!(res.start_next_after.is_none());
            assert_eq!(
                epochs_and_scores(&res),
                vec![
                    (3, Some(p("0.3"))),
                    (4, Some(p("0.4"))),
                    (5, Some(p("0.5")))
                ]
            );

            let res = query_node_performance_paged(deps, node_id, None, None)?;
            assert!(res.start_next_after.is_none());
            assert_eq!(
                epochs_and_scores(&res),
                vec![
                    (0, Some(p("0"))),
                    (1, Some(p("0.1"))),
                    (2, Some(p("0.2"))),
                    (3, Some(p("0.3"))),
                    (4, Some(p("0.4"))),
                    (5, Some(p("0.5"))),
                ]
            );

            let res = query_node_performance_paged(deps, node_id, Some(2), Some(1))?;
            assert_eq!(res.start_next_after, Some(3));
            assert_eq!(epochs_and_scores(&res), vec![(3, Some(p("0.3")))]);

            Ok(())
        }

        #[test]
        fn omits_gaps_and_stops_at_the_last_known_epoch() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let node_id = test.bond_dummy_nymnode()?;
            let nm = test.new_authorised_network_monitor();

            for epoch in [2, 3, 5] {
                test.submit_at_epoch(&nm, epoch, scored_submission(node_id, "0.5", "1"));
            }
            // the mixnet moves far ahead, but the node's history ends at its pointer
            test.set_mixnet_epoch(40)?;

            let deps = test.deps();
            let all = |res: &NodePerformancePagedResponse| {
                res.performance
                    .iter()
                    .map(|entry| entry.epoch_id)
                    .collect::<Vec<_>>()
            };

            // gaps are walked over, not reported, and the walk ends at the pointer
            let res = query_node_performance_paged(deps, node_id, None, None)?;
            assert_eq!(all(&res), vec![2, 3, 5]);
            assert_eq!(res.start_next_after, None);

            // the limit bounds the epochs visited, so a page can resume mid-gap
            let res = query_node_performance_paged(deps, node_id, Some(1), Some(2))?;
            assert_eq!(all(&res), vec![2, 3]);
            assert_eq!(res.start_next_after, Some(3));

            let res = query_node_performance_paged(deps, node_id, Some(3), None)?;
            assert_eq!(all(&res), vec![5]);
            assert_eq!(res.start_next_after, None);

            // a page that visits only empty epochs still makes progress
            let res = query_node_performance_paged(deps, node_id, None, Some(2))?;
            assert!(res.performance.is_empty());
            assert_eq!(res.start_next_after, Some(1));

            // starting at or past the pointer yields nothing
            for start_after in [5, 42] {
                let res = query_node_performance_paged(deps, node_id, Some(start_after), None)?;
                assert!(res.performance.is_empty());
                assert_eq!(res.start_next_after, None);
            }

            // and so does a node that was never measured
            let unmeasured = test.bond_dummy_nymnode()?;
            let res = query_node_performance_paged(test.deps(), unmeasured, None, None)?;
            assert!(res.performance.is_empty());
            assert_eq!(res.start_next_after, None);

            Ok(())
        }
    }

    #[cfg(test)]
    mod epoch_pages {
        use super::*;
        use crate::testing::{
            init_contract_tester, p, scored_submission, PerformanceContractTesterExt,
        };
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
        use nym_contracts_common::Percent;
        use nym_contracts_common_testing::{AdminExt, ContractOpts};
        use nym_performance_contract_common::{ExecuteMsg, Measurements, NodeSubmission, Weights};

        fn nodes_and_scores(res: &EpochPerformancePagedResponse) -> Vec<(NodeId, Option<Percent>)> {
            res.performance
                .iter()
                .map(|entry| (entry.node_id, entry.score))
                .collect()
        }

        /// A bundle whose stress only counts once stress carries a weight.
        fn with_stress(node_id: NodeId) -> NodeSubmission {
            NodeSubmission {
                node_id,
                measurements: Measurements::default()
                    .with_liveness(p("1"))
                    .with_stress(p("0.5"))
                    .with_config(p("1")),
            }
        }

        fn seventy_thirty() -> ExecuteMsg {
            ExecuteMsg::UpdateWeights {
                weights: Weights {
                    liveness: p("0.7"),
                    stress: p("0.3"),
                },
            }
        }

        #[test]
        fn querying_epoch_performance_paged() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let nm = test.new_authorised_network_monitor();
            let nodes = test.bond_dummy_nymnodes(10);

            let epoch_id = 5;
            test.set_mixnet_epoch(epoch_id)?;

            // node 1 reports a config of 50%, so its score is half its liveness; the others
            // report 100%, so their score is the liveness itself
            test.submit_now(&nm, scored_submission(nodes[1], "0.1", "0.5"));
            test.submit_scored(&nm, nodes[2], "0.2");
            test.submit_scored(&nm, nodes[3], "0.3");
            // 4 is missing
            test.submit_scored(&nm, nodes[5], "0.5");
            test.submit_scored(&nm, nodes[6], "0.6");

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
                nodes_and_scores(&res),
                vec![(nodes[5], Some(p("0.5"))), (nodes[6], Some(p("0.6")))]
            );

            let res = query_epoch_performance_paged(deps, epoch_id, Some(nodes[3]), None)?;
            assert_eq!(res.start_next_after, Some(nodes[6]));
            assert_eq!(
                nodes_and_scores(&res),
                vec![(nodes[5], Some(p("0.5"))), (nodes[6], Some(p("0.6")))]
            );

            let res = query_epoch_performance_paged(deps, epoch_id, Some(nodes[2]), None)?;
            assert_eq!(res.start_next_after, Some(nodes[6]));
            assert_eq!(
                nodes_and_scores(&res),
                vec![
                    (nodes[3], Some(p("0.3"))),
                    (nodes[5], Some(p("0.5"))),
                    (nodes[6], Some(p("0.6"))),
                ]
            );

            let res = query_epoch_performance_paged(deps, epoch_id, None, None)?;
            assert_eq!(res.start_next_after, Some(nodes[6]));
            assert_eq!(
                nodes_and_scores(&res),
                vec![
                    (nodes[1], Some(p("0.05"))),
                    (nodes[2], Some(p("0.2"))),
                    (nodes[3], Some(p("0.3"))),
                    (nodes[5], Some(p("0.5"))),
                    (nodes[6], Some(p("0.6"))),
                ]
            );
            // the unweighted medians sit beside the score
            assert_eq!(res.performance[0].medians.liveness, Some(p("0.1")));
            assert_eq!(res.performance[0].medians.config, Some(p("0.5")));

            let res = query_epoch_performance_paged(deps, epoch_id, Some(nodes[2]), Some(1))?;
            assert_eq!(res.start_next_after, Some(nodes[3]));
            assert_eq!(nodes_and_scores(&res), vec![(nodes[3], Some(p("0.3")))]);

            Ok(())
        }

        #[test]
        fn an_epoch_page_scores_under_the_weights_of_that_epoch() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let nm = test.new_authorised_network_monitor();
            let node_id = test.bond_dummy_nymnode()?;

            // epoch 5 under the creation weights, then 70/30 from epoch 6 onwards
            test.submit_at_epoch(&nm, 5, with_stress(node_id));
            test.execute_raw(test.admin_unchecked(), seventy_thirty())?;
            test.submit_at_epoch(&nm, 6, with_stress(node_id));

            let page_5 = query_epoch_performance_paged(test.deps(), 5, None, None)?;
            assert_eq!(nodes_and_scores(&page_5), vec![(node_id, Some(p("1")))]);

            let page_6 = query_epoch_performance_paged(test.deps(), 6, None, None)?;
            assert_eq!(nodes_and_scores(&page_6), vec![(node_id, Some(p("0.85")))]);

            Ok(())
        }

        #[test]
        fn a_limit_above_the_maximum_is_capped() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let nm = test.new_authorised_network_monitor();
            let max = retrieval_limits::NODE_EPOCH_PERFORMANCE_MAX_LIMIT as usize;
            let nodes = test.bond_dummy_nymnodes(max + 5);
            for &node_id in &nodes {
                test.submit_scored(&nm, node_id, "0.5");
            }

            let res = query_epoch_performance_paged(test.deps(), 0, None, Some(10_000))?;
            assert_eq!(res.performance.len(), max);
            assert_eq!(res.start_next_after, Some(nodes[max - 1]));

            Ok(())
        }

        #[test]
        fn full_history_spans_epochs_in_key_order() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let nm = test.new_authorised_network_monitor();
            let nodes = test.bond_dummy_nymnodes(5);

            // (9, nodes[4]) under the creation weights, then 70/30 from epoch 10 onwards for
            // (10, nodes[0]) and (10, nodes[3])
            test.submit_at_epoch(&nm, 9, with_stress(nodes[4]));
            test.execute_raw(test.admin_unchecked(), seventy_thirty())?;
            test.submit_at_epoch(&nm, 10, with_stress(nodes[0]));
            test.submit_now(&nm, scored_submission(nodes[3], "0.8", "1"));

            let deps = test.deps();
            let entries = |res: &FullHistoricalPerformancePagedResponse| {
                res.performance
                    .iter()
                    .map(|entry| (entry.epoch_id, entry.node_id, entry.score))
                    .collect::<Vec<_>>()
            };

            let res = query_full_historical_performance_paged(deps, None, None)?;
            assert_eq!(
                entries(&res),
                vec![
                    (9, nodes[4], Some(p("1"))),
                    (10, nodes[0], Some(p("0.85"))),
                    (10, nodes[3], Some(p("0.8"))),
                ]
            );
            assert_eq!(res.start_next_after, Some((10, nodes[3])));

            // pages resume from the last key returned
            let res = query_full_historical_performance_paged(deps, None, Some(2))?;
            assert_eq!(entries(&res).len(), 2);
            assert_eq!(res.start_next_after, Some((10, nodes[0])));

            let res = query_full_historical_performance_paged(deps, Some((10, nodes[0])), None)?;
            assert_eq!(entries(&res), vec![(10, nodes[3], Some(p("0.8")))]);
            assert_eq!(res.start_next_after, Some((10, nodes[3])));

            let res = query_full_historical_performance_paged(deps, Some((10, nodes[3])), None)?;
            assert!(res.performance.is_empty());
            assert_eq!(res.start_next_after, None);

            Ok(())
        }
    }

    #[cfg(test)]
    mod network_monitors {
        use super::*;
        use crate::testing::{
            init_contract_tester, scored_submission, PerformanceContractTesterExt,
        };
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
        use nym_contracts_common_testing::{AdminExt, ContractOpts};
        use nym_performance_contract_common::ExecuteMsg;

        #[test]
        fn a_monitors_information_includes_its_cursor() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let nm = test.new_authorised_network_monitor();
            let node_id = test.bond_dummy_nymnode()?;
            test.submit_at_epoch(&nm, 10, scored_submission(node_id, "0.5", "1"));

            let info = query_network_monitor_details(test.deps(), nm.to_string())?
                .info
                .expect("the monitor is authorised");
            assert_eq!(info.details.address, nm);
            assert_eq!(info.current_submission_metadata.last_submitted_epoch_id, 10);
            assert_eq!(
                info.current_submission_metadata.last_submitted_node_id,
                node_id
            );

            // the paged listing carries the same record
            let listed = query_network_monitors_paged(test.deps(), None, None)?;
            assert_eq!(listed.start_next_after, Some(nm.to_string()));
            assert_eq!(listed.info, vec![info]);

            // an address that was never authorised has no information
            let stranger = test.addr_make("stranger");
            assert_eq!(
                query_network_monitor_details(test.deps(), stranger.to_string())?.info,
                None
            );

            // once retired, the monitor moves to the retired listing
            test.execute_raw(
                test.admin_unchecked(),
                ExecuteMsg::RetireNetworkMonitor {
                    address: nm.to_string(),
                },
            )?;
            assert_eq!(
                query_network_monitor_details(test.deps(), nm.to_string())?.info,
                None
            );
            let retired = query_retired_network_monitors_paged(test.deps(), None, None)?;
            assert_eq!(retired.info.len(), 1);
            assert_eq!(retired.info[0].details.address, nm);
            assert_eq!(retired.start_next_after, Some(nm.to_string()));

            Ok(())
        }
    }

    #[cfg(test)]
    mod raw_measurements {
        use super::*;
        use crate::testing::{init_contract_tester, p, values, PerformanceContractTesterExt};
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
        use nym_contracts_common_testing::ContractOpts;

        #[test]
        fn the_stored_bundles_are_served_verbatim() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let nm1 = test.new_authorised_network_monitor();
            let nm2 = test.new_authorised_network_monitor();
            let nodes = test.bond_dummy_nymnodes(3);
            test.set_mixnet_epoch(10)?;

            test.submit_scored(&nm1, nodes[0], "0.9");
            test.submit_scored(&nm2, nodes[0], "0.8");
            test.submit_scored(&nm1, nodes[2], "0.5");

            let res = query_node_measurements(test.deps(), 10, nodes[0])?;
            let bundle = res.measurements.expect("the bundle exists");
            assert_eq!(
                values(bundle.liveness.as_ref().unwrap()),
                vec![p("0.8"), p("0.9")]
            );
            assert_eq!(
                values(bundle.config.as_ref().unwrap()),
                vec![p("1"), p("1")]
            );
            assert!(bundle.stress.is_none());

            // no fallback: a missing epoch is simply absent
            assert_eq!(
                query_node_measurements(test.deps(), 9, nodes[0])?.measurements,
                None
            );

            let node_ids = |res: &EpochMeasurementsPagedResponse| {
                res.measurements
                    .iter()
                    .map(|entry| entry.node_id)
                    .collect::<Vec<_>>()
            };

            let res = query_epoch_measurements_paged(test.deps(), 10, None, None)?;
            assert_eq!(node_ids(&res), vec![nodes[0], nodes[2]]);
            assert_eq!(res.start_next_after, Some(nodes[2]));

            let res = query_epoch_measurements_paged(test.deps(), 10, Some(nodes[0]), None)?;
            assert_eq!(node_ids(&res), vec![nodes[2]]);
            assert_eq!(res.start_next_after, Some(nodes[2]));

            Ok(())
        }
    }

    #[cfg(test)]
    mod last_submission {
        use super::*;
        use crate::testing::{
            init_contract_tester, liveness_submission, PerformanceContractTesterExt,
        };
        use mixnet_contract::testable_mixnet_contract::EmbeddedMixnetContractExt;
        use nym_contracts_common_testing::{ChainOpts, ContractOpts};
        use nym_performance_contract_common::LastSubmittedData;

        #[test]
        fn last_submission_query() -> anyhow::Result<()> {
            let mut test = init_contract_tester();
            let env = test.env();

            let id1 = test.bond_dummy_nymnode()?;
            let id2 = test.bond_dummy_nymnode()?;

            // initial
            assert_eq!(
                query_last_submission(test.deps())?,
                LastSubmission {
                    block_height: env.block.height,
                    block_time: env.block.time,
                    data: None,
                }
            );

            let nm1 = test.new_authorised_network_monitor();
            let nm2 = test.new_authorised_network_monitor();
            test.set_mixnet_epoch(10)?;

            let first = liveness_submission(id1, "0.2");
            test.submit_now(&nm1, first);
            assert_eq!(
                query_last_submission(test.deps())?,
                LastSubmission {
                    block_height: env.block.height,
                    block_time: env.block.time,
                    data: Some(LastSubmittedData {
                        sender: nm1.clone(),
                        epoch_id: 10,
                        data: first,
                    }),
                }
            );

            // a later block and another monitor move the record on
            test.next_block();
            let env = test.env();
            let second = liveness_submission(id2, "0.3");
            test.submit_now(&nm2, second);
            let after_second = LastSubmission {
                block_height: env.block.height,
                block_time: env.block.time,
                data: Some(LastSubmittedData {
                    sender: nm2.clone(),
                    epoch_id: 10,
                    data: second,
                }),
            };
            assert_eq!(query_last_submission(test.deps())?, after_second);

            // a submission for an earlier epoch is rejected by the freeze and leaves the record alone
            let res = NYM_PERFORMANCE_CONTRACT_STORAGE
                .submit_performance_data(
                    test.deps_mut(),
                    env,
                    &nm1,
                    5,
                    liveness_submission(id2, "0.4"),
                )
                .unwrap_err();
            assert_eq!(
                res,
                NymPerformanceContractError::EpochNotCurrent {
                    epoch_id: 5,
                    current_epoch_id: 10,
                }
            );
            assert_eq!(query_last_submission(test.deps())?, after_second);

            Ok(())
        }
    }
}

// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The contract divergence gauge.
//!
//! # THIS ROUTE IS TEMPORARY AND WILL BE REMOVED
//!
//! It exists for the shadow period before rewarding and active-set selection switch to the
//! performance contract: it shows, node by node, where nym-api's own figure and the contract's
//! disagree and why, so that the switch is flipped on evidence and the operators it would affect
//! can be told first. Once the switch is on, DELETE this module and its response types. Nothing
//! else reads them, and it lives under `/unstable` so that removing it breaks no compatibility
//! promise.

use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_performance::contract_window::ContractFigure;
use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::state::AppState;
use crate::unstable_routes::helpers::refreshed_at;
use axum::extract::{Query, State};
use nym_api_requests::models::described::v3::NymNodeDescriptionV3;
use nym_api_requests::models::v3::{
    ContractDivergenceCategory, ContractDivergenceCategoryCounts, ContractDivergenceResponse,
    DescribedNodeDetails, NodeContractDivergence, NodeContractFigure, NymApiFigure,
};
use nym_api_requests::models::{DisplayRole, NodeAnnotationV2, OffsetDateTimeJsonSchemaWrapper};
use nym_contracts_common::NaiveFloat;
use nym_http_api_common::{FormattedResponse, OutputParamsV2};
use nym_mixnet_contract_common::{EpochId, NodeId};
use nym_topology::CachedEpochRewardedSet;
use std::collections::HashMap;

/// The difference at which nym-api's figure and the contract's count as divergent.
const DIVERGENCE_THRESHOLD: f64 = 0.2;

pub type ContractDivergence = AxumResult<FormattedResponse<ContractDivergenceResponse>>;

/// The first category that matches, or `None` for a node neither side has anything for.
fn categorise(
    nym_api: Option<f64>,
    contract: Option<&ContractFigure>,
    current_epoch: EpochId,
) -> Option<ContractDivergenceCategory> {
    let Some(contract) = contract else {
        // nothing in the contract is only worth reporting when nym-api scores the node
        return nym_api
            .is_some_and(|performance| performance > 0.0)
            .then_some(ContractDivergenceCategory::Missing);
    };
    // unscoreable and contract-only exclude each other, so their order does not matter
    let Some(score) = contract.score else {
        return Some(ContractDivergenceCategory::Unscoreable);
    };
    let Some(nym_api) = nym_api else {
        return Some(ContractDivergenceCategory::ContractOnly);
    };
    // a stale contract figure is never reported as a measurement disagreement
    if contract.source_epoch < current_epoch {
        return Some(ContractDivergenceCategory::Fallback);
    }

    if (nym_api - score.naive_to_f64()).abs() >= DIVERGENCE_THRESHOLD {
        Some(ContractDivergenceCategory::Divergent)
    } else {
        Some(ContractDivergenceCategory::Agree)
    }
}

fn nym_api_figure(annotation: &NodeAnnotationV2) -> NymApiFigure {
    let performance = &annotation.detailed_performance;
    NymApiFigure {
        performance: performance.performance_score,
        routing_score: performance.routing_score.score,
        config_score: performance.config_score.score,
    }
}

fn node_contract_figure(figure: &ContractFigure) -> NodeContractFigure {
    NodeContractFigure {
        source_epoch: figure.source_epoch,
        score: figure.score.map(|score| score.naive_to_f64()),
        liveness: figure.medians.liveness.map(|median| median.naive_to_f64()),
        stress: figure.medians.stress.map(|median| median.naive_to_f64()),
        config: figure.medians.config.map(|median| median.naive_to_f64()),
    }
}

fn described_details(described: &NymNodeDescriptionV3) -> DescribedNodeDetails {
    let roles = &described.description.declared_role;
    DescribedNodeDetails {
        version: described.version().to_string(),
        mixnode: roles.mixnode,
        entry: roles.entry,
        exit: roles.can_operate_exit_gateway(),
    }
}

/// Every bonded node in its category, counted per declared role, with a row for each one not in
/// agreement.
#[allow(clippy::too_many_arguments)]
fn build_response(
    refreshed_at: OffsetDateTimeJsonSchemaWrapper,
    current_epoch: EpochId,
    contract_figure: impl Fn(NodeId) -> Option<ContractFigure>,
    bonded: impl Iterator<Item = NodeId>,
    annotations: &HashMap<NodeId, NodeAnnotationV2>,
    described: &DescribedNodes,
    rewarded_set: &CachedEpochRewardedSet,
) -> ContractDivergenceResponse {
    let mut counts = ContractDivergenceCategoryCounts::default();
    let mut nodes = Vec::new();

    for node_id in bonded {
        let annotation = annotations.get(&node_id);
        let contract = contract_figure(node_id);
        let nym_api =
            annotation.map(|annotation| annotation.detailed_performance.performance_score);
        let Some(category) = categorise(nym_api, contract.as_ref(), current_epoch) else {
            continue;
        };

        let description = described.get_node(&node_id);
        counts
            .category_mut(category)
            .add(description.map(|description| &description.description.declared_role));
        if category == ContractDivergenceCategory::Agree {
            continue;
        }

        nodes.push(NodeContractDivergence {
            node_id,
            category,
            nym_api: annotation.map(nym_api_figure),
            contract: contract.as_ref().map(node_contract_figure),
            rewarded_set_role: rewarded_set.role(node_id).map(DisplayRole::from),
            // what identifies the groups the orchestrator cannot describe
            described: description
                .filter(|_| category == ContractDivergenceCategory::Missing)
                .map(described_details),
        });
    }

    ContractDivergenceResponse {
        refreshed_at,
        epoch_id: current_epoch,
        threshold: DIVERGENCE_THRESHOLD,
        counts,
        nodes,
    }
}

/// Compare each bonded node's nym-api figure with the performance contract's at the current epoch.
///
/// TEMPORARY: this route exists only for the shadow period before rewarding switches to the
/// performance contract, and is to be deleted once it has. See the module docs.
///
/// Each node lands in the first matching category (missing, contract-only, unscoreable,
/// fallback, divergent, agree); every category is counted, and every node not in agreement gets
/// a row. Served only while nym-api holds both figures: with the performance contract read and
/// nym-api still computing its own figure.
#[utoipa::path(
    operation_id = "v3_contract_divergence",
    tag = "Unstable Nym Nodes v3",
    get,
    params(OutputParamsV2),
    path = "/contract-divergence",
    context_path = "/v3/unstable/nym-nodes",
    responses(
        (status = 200, content(
            (ContractDivergenceResponse = "application/json"),
            (ContractDivergenceResponse = "application/yaml"),
        )),
        (status = 503, description = "this nym-api does not hold both figures, or has not loaded the performance contract yet"),
    )
)]
pub(super) async fn contract_divergence(
    state: State<AppState>,
    Query(output): Query<OutputParamsV2>,
) -> ContractDivergence {
    let Some(window) = &state.performance_contract_window else {
        return Err(AxumErrorResponse::service_unavailable(
            "this nym-api does not read the performance contract",
        ));
    };
    if state.use_performance_contract_data {
        return Err(AxumErrorResponse::service_unavailable(
            "this nym-api runs in contract mode, so it computes no figure of its own to compare",
        ));
    }
    let Some(window) = window.read().await else {
        return Err(AxumErrorResponse::service_unavailable(
            "the performance contract has not been loaded yet",
        ));
    };

    let describe_cache = state.describe_nodes_cache_data().await?;
    let status_cache = state.node_status_cache().cache().await?;
    let rewarded_set = state.rewarded_set().await?;
    let bonded = state.nym_contract_cache().nym_nodes().await;

    // the oldest of the three caches, since a row pairs figures from each
    let refreshed = refreshed_at([
        status_cache.timestamp(),
        describe_cache.timestamp(),
        window.refreshed_at(),
    ]);

    Ok(output.to_response(build_response(
        refreshed,
        window.current_epoch(),
        |node_id| window.figure(node_id),
        bonded.iter().map(|node| node.node_id()),
        &status_cache.node_annotations,
        &describe_cache,
        &rewarded_set,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_api_requests::models::described::v3::mock_nym_node_description;
    use nym_api_requests::models::v3::ContractDivergenceCounts;
    use nym_api_requests::models::{DetailedNodePerformanceV2, RoutingScore};
    use nym_contracts_common::Percent;
    use nym_validator_client::nyxd::contract_traits::performance_query_client::KindMedians;
    use std::collections::HashSet;
    use time::OffsetDateTime;

    const X: EpochId = 100;

    fn p(value: &str) -> Percent {
        value.parse().unwrap()
    }

    fn at(source_epoch: EpochId, score: Option<&str>) -> ContractFigure {
        ContractFigure {
            source_epoch,
            medians: KindMedians {
                liveness: Some(p("0.9")),
                stress: None,
                config: Some(p("1")),
            },
            score: score.map(p),
        }
    }

    fn annotated(performance: f64) -> NodeAnnotationV2 {
        let mut detailed_performance = DetailedNodePerformanceV2::default();
        detailed_performance.performance_score = performance;
        detailed_performance.routing_score = RoutingScore::new(performance);
        NodeAnnotationV2 {
            current_role: None,
            chain_interaction_capabilities: None,
            detailed_performance,
        }
    }

    fn described(node_id: NodeId, mixnode: bool, entry: bool) -> NymNodeDescriptionV3 {
        let mut node = mock_nym_node_description(node_id.into());
        node.node_id = node_id;
        node.description.declared_role.mixnode = mixnode;
        node.description.declared_role.entry = entry;
        node.description.declared_role.exit_nr = false;
        node.description.declared_role.exit_ipr = false;
        node
    }

    fn described_nodes(nodes: impl IntoIterator<Item = NymNodeDescriptionV3>) -> DescribedNodes {
        DescribedNodes {
            nodes: nodes.into_iter().map(|node| (node.node_id, node)).collect(),
            addresses_cache: HashMap::new(),
        }
    }

    /// One node's category, with nym-api's figure `nym_api` and the contract's `contract`.
    fn category(
        nym_api: Option<f64>,
        contract: Option<ContractFigure>,
    ) -> Option<ContractDivergenceCategory> {
        categorise(nym_api, contract.as_ref(), X)
    }

    #[test]
    fn each_node_lands_in_the_first_matching_category() {
        use ContractDivergenceCategory::*;

        assert_eq!(category(Some(0.9), None), Some(Missing));
        assert_eq!(category(None, Some(at(X, Some("0.7")))), Some(ContractOnly));
        assert_eq!(category(Some(0.9), Some(at(X, None))), Some(Unscoreable));
        assert_eq!(category(None, Some(at(X, None))), Some(Unscoreable));
        assert_eq!(
            category(Some(0.9), Some(at(X - 1, Some("0.8")))),
            Some(Fallback)
        );
        assert_eq!(
            category(Some(0.9), Some(at(X, Some("0.6")))),
            Some(Divergent)
        );
        assert_eq!(category(Some(0.9), Some(at(X, Some("0.8")))), Some(Agree));
    }

    #[test]
    fn a_stale_contract_figure_is_fallback_rather_than_divergent() {
        assert_eq!(
            category(Some(0.9), Some(at(X - 1, Some("0.2")))),
            Some(ContractDivergenceCategory::Fallback)
        );

        // its row still carries both figures, so the divergence stays visible
        let response = response(
            HashMap::from([(1, at(X - 1, Some("0.2")))]),
            HashMap::from([(1, annotated(0.9))]),
            described_nodes([]),
            CachedEpochRewardedSet::default(),
        );
        let [row] = response.nodes.as_slice() else {
            panic!("expected one row, got {:?}", response.nodes);
        };
        assert_eq!(row.nym_api.map(|figure| figure.performance), Some(0.9));
        assert_eq!(row.contract.and_then(|figure| figure.score), Some(0.2));
    }

    #[test]
    fn both_sides_scoring_zero_agree() {
        assert_eq!(
            category(Some(0.0), Some(at(X, Some("0")))),
            Some(ContractDivergenceCategory::Agree)
        );
    }

    #[test]
    fn a_node_neither_side_has_anything_for_is_not_listed() {
        assert_eq!(category(None, None), None);
        assert_eq!(category(Some(0.0), None), None);
    }

    #[test]
    fn figures_within_the_threshold_agree() {
        assert_eq!(
            category(Some(0.9), Some(at(X, Some("0.75")))),
            Some(ContractDivergenceCategory::Agree)
        );
        assert_eq!(
            category(Some(0.9), Some(at(X, Some("0.65")))),
            Some(ContractDivergenceCategory::Divergent)
        );
    }

    fn response(
        figures: HashMap<NodeId, ContractFigure>,
        annotations: HashMap<NodeId, NodeAnnotationV2>,
        described: DescribedNodes,
        rewarded_set: CachedEpochRewardedSet,
    ) -> ContractDivergenceResponse {
        let mut bonded: Vec<_> = figures.keys().chain(annotations.keys()).copied().collect();
        bonded.sort();
        bonded.dedup();

        build_response(
            OffsetDateTime::UNIX_EPOCH.into(),
            X,
            |node_id| figures.get(&node_id).copied(),
            bonded.into_iter(),
            &annotations,
            &described,
            &rewarded_set,
        )
    }

    #[test]
    fn counts_are_per_declared_role_and_an_undescribed_node_counts_in_the_total_only() {
        let (dual_role, undescribed) = (1, 2);
        let response = response(
            HashMap::from([
                (dual_role, at(X, Some("0.5"))),
                (undescribed, at(X, Some("0.7"))),
            ]),
            HashMap::from([(dual_role, annotated(0.9))]),
            described_nodes([described(dual_role, true, true)]),
            CachedEpochRewardedSet::default(),
        );

        assert_eq!(
            response.counts.divergent,
            ContractDivergenceCounts {
                total: 1,
                mixnode: 1,
                entry: 1,
                exit: 0,
            }
        );
        assert_eq!(
            response.counts.contract_only,
            ContractDivergenceCounts {
                total: 1,
                ..Default::default()
            }
        );
    }

    #[test]
    fn rows_carry_both_sides_and_the_rewarded_set_role() {
        let node_id = 1;
        let response = response(
            HashMap::from([(node_id, at(X, Some("0.6")))]),
            HashMap::from([(node_id, annotated(0.9))]),
            described_nodes([described(node_id, true, false)]),
            CachedEpochRewardedSet {
                layer1: HashSet::from([node_id]),
                ..Default::default()
            },
        );

        let [row] = response.nodes.as_slice() else {
            panic!("expected one row, got {:?}", response.nodes);
        };
        assert_eq!(row.category, ContractDivergenceCategory::Divergent);
        assert_eq!(row.nym_api.map(|figure| figure.performance), Some(0.9));
        assert_eq!(row.contract.and_then(|figure| figure.score), Some(0.6));
        assert_eq!(row.contract.map(|figure| figure.source_epoch), Some(X));
        assert!(matches!(row.rewarded_set_role, Some(DisplayRole::Layer1)));
        assert_eq!(row.described, None);
    }

    #[test]
    fn missing_and_contract_only_rows_carry_what_each_side_has() {
        let (missing, contract_only) = (1, 2);
        let response = response(
            HashMap::from([(contract_only, at(X, Some("0.7")))]),
            HashMap::from([(missing, annotated(0.9))]),
            described_nodes([
                described(missing, true, false),
                described(contract_only, false, true),
            ]),
            CachedEpochRewardedSet::default(),
        );

        let row = |node_id| {
            response
                .nodes
                .iter()
                .find(|row| row.node_id == node_id)
                .unwrap()
        };
        assert_eq!(
            row(missing).described,
            Some(DescribedNodeDetails {
                version: described(missing, true, false).version().to_string(),
                mixnode: true,
                entry: false,
                exit: false,
            })
        );
        assert_eq!(row(contract_only).described, None);

        assert!(row(missing).nym_api.is_some());
        assert!(row(missing).contract.is_none());
        assert!(row(contract_only).nym_api.is_none());
        assert!(row(contract_only).contract.is_some());
    }

    #[test]
    fn an_agreeing_node_is_counted_without_a_row() {
        let response = response(
            HashMap::from([(1, at(X, Some("0")))]),
            HashMap::from([(1, annotated(0.0))]),
            described_nodes([]),
            CachedEpochRewardedSet::default(),
        );

        assert_eq!(response.counts.agree.total, 1);
        assert!(response.nodes.is_empty());
    }
}

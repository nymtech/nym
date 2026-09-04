// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The liveness divergence gauge.
//!
//! # THIS ROUTE IS TEMPORARY AND WILL BE REMOVED
//!
//! It exists for one purpose: to decide whether the v3 liveness score can be given a non-zero
//! weight, and separately whether the v1 routing score can be retired. Liveness currently ships at
//! weight zero precisely because two populations score zero on it for reasons unrelated to their
//! forwarding capability - nodes that have not ingested their agents' on-chain authorisations, and
//! gateways not yet carrying the final-hop and monitor-session behaviour. This route is how that
//! population is sized and watched as the fleet upgrades, so that raising the weight is a
//! measurement rather than a guess.
//!
//! Once that decision is made, DELETE this module and its response types. Nothing else reads them,
//! and it lives under `/unstable` rather than the stable tree so that removing it breaks no
//! compatibility promise. Do not build anything durable on top of it.

use crate::node_status_api::models::AxumResult;
use crate::support::http::state::AppState;
use crate::unstable_routes::helpers::refreshed_at;
use axum::extract::{Query, State};
use nym_api_requests::models::described::v3::NymNodeDescriptionV3;
use nym_api_requests::models::v3::{
    DivergenceNodeRole, LivenessDivergenceResponse, NodeLivenessDivergence,
};
use nym_api_requests::models::NodeAnnotationV2;
use nym_http_api_common::{FormattedResponse, OutputParamsV2};
use nym_mixnet_contract_common::NodeId;
use std::collections::HashMap;

pub type LivenessDivergence = AxumResult<FormattedResponse<LivenessDivergenceResponse>>;

/// The role a node declares, or `None` if it declares neither and is therefore out of scope for
/// liveness entirely.
///
/// Mirrors the orchestrator's `NodeType::from_roles`, which classifies on
/// `(mixnode_enabled, gateway_enabled)` and treats neither-set as `Unknown`. `declared_role.entry`
/// is that same `gateway_enabled` flag. Keeping the ineligible case as `None` is what lets the
/// response omit those nodes rather than carry an `unknown` variant no consumer could act on.
fn divergence_role(described: &NymNodeDescriptionV3) -> Option<DivergenceNodeRole> {
    match (
        described.description.declared_role.mixnode,
        described.description.declared_role.entry,
    ) {
        (true, true) => Some(DivergenceNodeRole::Both),
        (true, false) => Some(DivergenceNodeRole::Mixnode),
        (false, true) => Some(DivergenceNodeRole::Gateway),
        (false, false) => None,
    }
}

fn build_response<'a>(
    nym_nodes: impl Iterator<Item = &'a NymNodeDescriptionV3>,
    annotations: &HashMap<NodeId, NodeAnnotationV2>,
) -> Vec<NodeLivenessDivergence> {
    let mut nodes = Vec::new();

    for nym_node in nym_nodes {
        // a node out of scope for liveness has no divergence to report: it is not that it scored
        // badly, it is that the orchestrator never assigns it a liveness test
        let Some(role) = divergence_role(nym_node) else {
            continue;
        };

        // an annotation is missing only in the window before the status cache has first produced
        // one, where a default reads as both scores zero - a zero divergence, which is honest
        // enough for a gauge and better than omitting the node from the coverage count
        let annotation = annotations
            .get(&nym_node.node_id)
            .cloned()
            .unwrap_or_default();
        let performance = annotation.detailed_performance;

        let liveness_score = performance.liveness_score.score;
        let routing_score = performance.routing_score.score;

        nodes.push(NodeLivenessDivergence {
            node_id: nym_node.node_id,
            liveness_score,
            routing_score,
            divergence: liveness_score - routing_score,
            was_reachable: performance.liveness_score.was_reachable,
            role,
        });
    }

    nodes
}

/// Compare each node's aggregated v3 liveness score against the v1 monitor's routing score.
///
/// TEMPORARY: this route exists only to inform the decision to weight liveness and to retire the
/// v1 routing score, and is to be deleted once that decision is made. See the module docs.
///
/// Every liveness-eligible bonded node is listed, including those with no sample yet, so that
/// coverage and comparison are readable from one response. Nodes the orchestrator would never
/// assign a liveness test are omitted rather than reported as diverging.
#[utoipa::path(
    operation_id = "v3_liveness_divergence",
    tag = "Unstable Nym Nodes v3",
    get,
    params(OutputParamsV2),
    path = "/liveness-divergence",
    context_path = "/v3/unstable/nym-nodes",
    responses(
        (status = 200, content(
            (LivenessDivergenceResponse = "application/json"),
            (LivenessDivergenceResponse = "application/yaml"),
        ))
    )
)]
pub(super) async fn liveness_divergence(
    state: State<AppState>,
    Query(output): Query<OutputParamsV2>,
) -> LivenessDivergence {
    let describe_cache = state.describe_nodes_cache_data().await?;
    let status_cache = &state.node_status_cache();
    let annotations = status_cache.node_annotations().await?;

    let nodes = build_response(describe_cache.all_nym_nodes(), &annotations);

    // the older of the two caches, since a divergence row pairs a score from each
    let refreshed = refreshed_at([
        status_cache.cache_timestamp().await,
        describe_cache.timestamp(),
    ]);

    Ok(output.to_response(LivenessDivergenceResponse {
        refreshed_at: refreshed,
        nodes,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_api_requests::models::described::v3::mock_nym_node_description;
    use nym_api_requests::models::{
        ConfigScoreV2, DetailedNodePerformanceV2, LivenessScore, RoutingScore, StressTestingScore,
    };

    fn described(node_id: NodeId, mixnode: bool, entry: bool) -> NymNodeDescriptionV3 {
        let mut node = mock_nym_node_description(node_id.into());
        node.node_id = node_id;
        node.description.declared_role.mixnode = mixnode;
        node.description.declared_role.entry = entry;
        node
    }

    fn annotated(liveness: LivenessScore, routing: f64) -> NodeAnnotationV2 {
        NodeAnnotationV2 {
            current_role: None,
            chain_interaction_capabilities: None,
            detailed_performance: DetailedNodePerformanceV2::new(
                0.0,
                RoutingScore::new(routing),
                ConfigScoreV2::default(),
                StressTestingScore::unreachable(),
                liveness,
            ),
        }
    }

    #[test]
    fn divergence_is_liveness_minus_routing_and_negative_while_the_fleet_lags() {
        let nodes = [described(1, true, false)];
        let annotations = HashMap::from([(
            1,
            annotated(
                LivenessScore {
                    score: 0.5,
                    was_reachable: true,
                },
                0.9,
            ),
        )]);

        let rows = build_response(nodes.iter(), &annotations);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].liveness_score, 0.5);
        assert_eq!(rows[0].routing_score, 0.9);
        // negative: liveness scores it worse than v1 does, the expected rollout direction
        assert_eq!(rows[0].divergence, -0.4);
        assert!(rows[0].was_reachable);
    }

    /// The role split is the one dimension the gauge keeps, so each classification must land where
    /// the orchestrator would put it. One decoy per role combination.
    #[test]
    fn each_role_combination_is_classified_as_the_orchestrator_would() {
        let nodes = [
            described(1, true, false),
            described(2, false, true),
            described(3, true, true),
            // declares neither: `NodeType::Unknown` to the orchestrator, never assigned a test
            described(4, false, false),
        ];
        let annotations = HashMap::new();

        let rows = build_response(nodes.iter(), &annotations);

        let roles: Vec<_> = rows.iter().map(|r| (r.node_id, r.role)).collect();
        assert_eq!(
            roles,
            vec![
                (1, DivergenceNodeRole::Mixnode),
                (2, DivergenceNodeRole::Gateway),
                (3, DivergenceNodeRole::Both),
            ],
            "the node declaring neither role must be omitted, not reported as diverging"
        );
    }

    /// A node with no sample must still be listed, since coverage is half of what the gauge is
    /// read for: it is the difference between "the fleet is fine" and "we have measured nothing".
    #[test]
    fn an_unmeasured_node_is_listed_with_was_reachable_false() {
        let nodes = [described(1, true, false)];
        let annotations = HashMap::from([(1, annotated(LivenessScore::unreachable(), 0.9))]);

        let rows = build_response(nodes.iter(), &annotations);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].liveness_score, 0.0);
        assert!(
            !rows[0].was_reachable,
            "an unmeasured node must be distinguishable from one that delivered nothing"
        );
        assert_eq!(rows[0].divergence, -0.9);
    }
}

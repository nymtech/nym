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
use crate::node_performance::contract_window::WindowData;
use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::support::http::state::AppState;
use crate::unstable_routes::helpers::refreshed_at;
use axum::extract::{Query, State};
use nym_api_requests::models::v3::ContractDivergenceResponse;
use nym_api_requests::models::{NodeAnnotationV2, OffsetDateTimeJsonSchemaWrapper};
use nym_http_api_common::{FormattedResponse, OutputParamsV2};
use nym_mixnet_contract_common::NodeId;
use nym_topology::CachedEpochRewardedSet;
use std::collections::HashMap;

/// The difference at which nym-api's figure and the contract's count as divergent.
const DIVERGENCE_THRESHOLD: f64 = 0.2;

pub type ContractDivergence = AxumResult<FormattedResponse<ContractDivergenceResponse>>;

/// The side nym-api lacks for a comparison, if any: the contract's with the read flag off, its
/// own with the switch on (contract mode computes no figure of nym-api's own).
fn unavailable_side(read_flag_on: bool, switch_on: bool) -> Option<&'static str> {
    let _ = (read_flag_on, switch_on);
    todo!("task 6.2")
}

/// Every bonded node in the first category that matches, with the counts and a row for each
/// node not in agreement.
fn build_response(
    refreshed_at: OffsetDateTimeJsonSchemaWrapper,
    window: &WindowData,
    bonded: impl Iterator<Item = NodeId>,
    annotations: &HashMap<NodeId, NodeAnnotationV2>,
    described: &DescribedNodes,
    rewarded_set: &CachedEpochRewardedSet,
) -> ContractDivergenceResponse {
    let _ = (
        refreshed_at,
        window,
        bonded,
        annotations,
        described,
        rewarded_set,
        DIVERGENCE_THRESHOLD,
    );
    todo!("task 6.2")
}

/// Compare each bonded node's nym-api figure with the performance contract's at the current epoch.
///
/// TEMPORARY: this route exists only for the shadow period before rewarding switches to the
/// performance contract, and is to be deleted once it has. See the module docs.
///
/// Each node lands in the first matching category (missing, contract-only, unscoreable,
/// fallback, divergent, agree); every category is counted, and every node not in agreement gets
/// a row. Served only while nym-api holds both figures: with the performance contract read and
/// rewarding still on nym-api's own figure.
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
    if let Some(missing) = unavailable_side(
        state.performance_contract_window.is_some(),
        state.use_performance_contract_data,
    ) {
        return Err(AxumErrorResponse::service_unavailable(missing));
    }

    let window = match &state.performance_contract_window {
        Some(window) => window.read().await,
        None => None,
    };
    let Some(window) = window else {
        return Err(AxumErrorResponse::service_unavailable(
            "the performance contract has not been loaded yet",
        ));
    };

    let describe_cache = state.describe_nodes_cache_data().await?;
    let status_cache = state.node_status_cache();
    let annotations = status_cache.node_annotations().await?;
    let rewarded_set = state.rewarded_set().await?;
    let bonded = state.nym_contract_cache().nym_nodes().await;

    // the oldest of the three caches, since a row pairs figures from each
    let refreshed = refreshed_at([
        status_cache.cache_timestamp().await,
        describe_cache.timestamp(),
        window.refreshed_at(),
    ]);

    Ok(output.to_response(build_response(
        refreshed,
        &window,
        bonded.iter().map(|node| node.node_id()),
        &annotations,
        &describe_cache,
        &rewarded_set,
    )))
}

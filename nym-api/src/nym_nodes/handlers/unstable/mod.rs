// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//!   All routes/nodes are split into three tiers:
//!
//!   `/skimmed`
//!     - used by clients
//!     - returns the very basic information for routing purposes
//!
//!   `/semi-skimmed`
//!     - used by other nodes/VPN
//!     - returns more additional information such noise keys
//!
//!   `/full-fat`
//!     - used by explorers, et al.
//!     - returns almost everything there is about the nodes
//!
//!   There's also additional split based on the role:
//!   - `?role` => filters based on the specific role (mixnode/gateway/(in the future: entry/exit))
//!   - `/mixnodes/<tier>` => only returns mixnode role data
//!   - `/gateway/<tier>` => only returns (entry) gateway role data

use crate::node_status_api::models::{AxumErrorResponse, AxumResult};
use crate::nym_nodes::handlers::unstable::full_fat::nodes_detailed;
use crate::nym_nodes::handlers::unstable::semi_skimmed::nodes_expanded;
use crate::nym_nodes::handlers::unstable::skimmed::{
    entry_gateways_basic_active, entry_gateways_basic_all, exit_gateways_basic_active,
    exit_gateways_basic_all, mixnodes_basic_active, mixnodes_basic_all, nodes_basic_active,
    nodes_basic_all,
};
use crate::support::http::helpers::PaginationRequest;
use crate::support::http::state::AppState;
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use nym_api_requests::nym_nodes::{
    NodeRoleQueryParam, NodesByAddressesRequestBody, NodesByAddressesResponse,
};
use nym_http_api_common::{FormattedResponse, Output, OutputParams};
use serde::Deserialize;
use std::collections::HashMap;
use tower_http::compression::CompressionLayer;

pub(crate) mod full_fat;
mod helpers;
pub(crate) mod semi_skimmed;
pub(crate) mod skimmed;

#[allow(deprecated)]
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .nest(
            "/skimmed",
            Router::new()
                .route("/", get(nodes_basic_all))
                .route("/active", get(nodes_basic_active))
                .nest(
                    "/mixnodes",
                    Router::new()
                        .route("/active", get(mixnodes_basic_active))
                        .route("/all", get(mixnodes_basic_all)),
                )
                .nest(
                    "/entry-gateways",
                    Router::new()
                        .route("/active", get(entry_gateways_basic_active))
                        .route("/all", get(entry_gateways_basic_all)),
                )
                .nest(
                    "/exit-gateways",
                    Router::new()
                        .route("/active", get(exit_gateways_basic_active))
                        .route("/all", get(exit_gateways_basic_all)),
                ),
        )
        .nest(
            "/semi-skimmed",
            Router::new().route("/", get(nodes_expanded)),
        )
        .nest("/full-fat", Router::new().route("/", get(nodes_detailed)))
        .route("/gateways/skimmed", get(skimmed::deprecated_gateways_basic))
        .route("/mixnodes/skimmed", get(skimmed::deprecated_mixnodes_basic))
        .route("/by-addresses", post(nodes_by_addresses))
        .layer(CompressionLayer::new())
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
struct NodesParamsWithRole {
    #[param(inline)]
    role: Option<NodeRoleQueryParam>,

    #[allow(dead_code)]
    semver_compatibility: Option<String>,
    no_legacy: Option<bool>,
    page: Option<u32>,
    per_page: Option<u32>,

    // Identifier for the current epoch of the topology state. When sent by a client we can check if
    // the client already knows about the latest topology state, allowing a `no-updates` response
    // instead of wasting bandwidth serving an unchanged topology.
    epoch_id: Option<u32>,

    output: Option<Output>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct NodesParams {
    #[allow(dead_code)]
    semver_compatibility: Option<String>,
    no_legacy: Option<bool>,
    page: Option<u32>,
    per_page: Option<u32>,

    // Identifier for the current epoch of the topology state. When sent by a client we can check if
    // the client already knows about the latest topology state, allowing a `no-updates` response
    // instead of wasting bandwidth serving an unchanged topology.
    epoch_id: Option<u32>,
    output: Option<Output>,
}

impl From<NodesParamsWithRole> for NodesParams {
    fn from(params: NodesParamsWithRole) -> Self {
        NodesParams {
            semver_compatibility: params.semver_compatibility,
            no_legacy: params.no_legacy,
            page: params.page,
            per_page: params.per_page,
            epoch_id: params.epoch_id,
            output: params.output,
        }
    }
}

impl<'a> From<&'a NodesParams> for PaginationRequest {
    fn from(params: &'a NodesParams) -> Self {
        PaginationRequest {
            output: params.output,
            page: params.page,
            per_page: params.per_page,
        }
    }
}

#[utoipa::path(
    tag = "Unstable Nym Nodes",
    post,
    request_body = NodesByAddressesRequestBody,
    path = "/by-addresses",
    context_path = "/v1/unstable/nym-nodes",
    responses(
        (status = 200, content(
            (NodesByAddressesResponse = "application/json"),
            (NodesByAddressesResponse = "application/yaml"),
            (NodesByAddressesResponse = "application/bincode")
        ))
    ),
    params(OutputParams)
)]
async fn nodes_by_addresses(
    Query(output): Query<OutputParams>,
    state: State<AppState>,
    Json(body): Json<NodesByAddressesRequestBody>,
) -> AxumResult<FormattedResponse<NodesByAddressesResponse>> {
    // if the request is too big, simply reject it
    if body.addresses.len() > 100 {
        return Err(AxumErrorResponse::bad_request(
            "requested too many addresses",
        ));
    }

    let output = output.output.unwrap_or_default();

    // TODO: perhaps introduce different cache because realistically nym-api will receive
    // request for the same couple addresses from all nodes in quick succession
    let describe_cache = state.describe_nodes_cache_data().await?;

    let mut existence = HashMap::new();
    for address in body.addresses {
        existence.insert(address, describe_cache.node_with_address(address));
    }

    Ok(output.to_response(NodesByAddressesResponse { existence }))
}

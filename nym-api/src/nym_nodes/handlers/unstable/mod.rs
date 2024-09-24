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

use crate::nym_nodes::handlers::unstable::full_fat::nodes_detailed;
use crate::nym_nodes::handlers::unstable::semi_skimmed::nodes_expanded;
use crate::nym_nodes::handlers::unstable::skimmed::{
    deprecated_gateways_basic, deprecated_mixnodes_basic, entry_gateways_basic_active,
    entry_gateways_basic_all, exit_gateways_basic_active, exit_gateways_basic_all,
    mixnodes_basic_active, mixnodes_basic_all, nodes_basic, nodes_basic_active,
    nodes_basic_standby,
};
use crate::support::http::helpers::PaginationRequest;
use crate::support::http::state::AppState;
use axum::routing::get;
use axum::Router;
use nym_api_requests::nym_nodes::NodeRoleQueryParam;
use serde::Deserialize;

pub(crate) mod full_fat;
pub(crate) mod semi_skimmed;
pub(crate) mod skimmed;

#[allow(deprecated)]
pub(crate) fn nym_node_routes_unstable() -> Router<AppState> {
    Router::new()
        .nest(
            "/skimmed",
            Router::new()
                .route("/", get(nodes_basic))
                .route("/active", get(nodes_basic_active))
                .route("/standby", get(nodes_basic_standby))
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
        .route("/gateways/skimmed", get(deprecated_gateways_basic))
        .route("/mixnodes/skimmed", get(deprecated_mixnodes_basic))
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
struct NodesParamsWithRole {
    #[param(inline)]
    role: Option<NodeRoleQueryParam>,

    semver_compatibility: Option<String>,
    no_legacy: bool,
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
struct NodesParams {
    semver_compatibility: Option<String>,
    no_legacy: bool,
    page: Option<u32>,
    per_page: Option<u32>,
}

impl From<NodesParamsWithRole> for NodesParams {
    fn from(params: NodesParamsWithRole) -> Self {
        NodesParams {
            semver_compatibility: params.semver_compatibility,
            no_legacy: params.no_legacy,
            page: params.page,
            per_page: params.per_page,
        }
    }
}

impl<'a> From<&'a NodesParams> for PaginationRequest {
    fn from(params: &'a NodesParams) -> Self {
        PaginationRequest {
            page: params.page,
            per_page: params.per_page,
        }
    }
}

// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//!   All routes/nodes are split into three tiers:
//!
//!   `/skimmed`
//!     - used by clients
//!     - returns the very basic information for routing purposes
//!
//!   `/semi-skimmed`
//!     - used by other nodes/VPN
//!     - returns more additional information such as noise keys
//!
//!   `/full-fat`
//!     - used by explorers, et al.
//!     - returns almost everything there is about the nodes
//!
//!   There's also additional split based on the role:
//!   - `?role` => filters based on the specific role (mixnode/gateway/(in the future: entry/exit))
//!   - `/mixnodes/<tier>` => only returns mixnode role data
//!   - `/gateway/<tier>` => only returns (entry) gateway role data

use crate::support::http::state::AppState;
use crate::unstable_routes::v1::nym_nodes::full_fat::nodes_detailed;
use crate::unstable_routes::v1::nym_nodes::handlers::nodes_by_addresses;
use crate::unstable_routes::v1::nym_nodes::semi_skimmed::nodes_expanded;
use axum::routing::{get, post};
use axum::Router;
use tower_http::compression::CompressionLayer;

#[allow(deprecated)]
use crate::unstable_routes::v1::nym_nodes::skimmed::{
    entry_gateways_basic_active, entry_gateways_basic_all, exit_gateways_basic_active,
    exit_gateways_basic_all, mixnodes_basic_active, mixnodes_basic_all, nodes_basic_active,
    nodes_basic_all,
};

pub(crate) mod full_fat;
pub(crate) mod handlers;
pub(crate) mod helpers;
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

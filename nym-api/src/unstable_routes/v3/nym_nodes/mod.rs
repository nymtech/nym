// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::http::state::AppState;
use crate::unstable_routes::v3::nym_nodes::liveness_divergence::liveness_divergence;
use crate::unstable_routes::v3::nym_nodes::semi_skimmed::nodes_expanded;
use crate::unstable_routes::v3::nym_nodes::skimmed::{
    entry_gateways_basic_all, exit_gateways_basic_all, mixnodes_basic_active, mixnodes_basic_all,
    nodes_basic_all,
};
use axum::routing::get;
use axum::Router;
use tower_http::compression::CompressionLayer;

pub(crate) mod liveness_divergence;
pub(crate) mod semi_skimmed;
pub(crate) mod skimmed;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .nest(
            "/skimmed",
            Router::new()
                .route("/", get(nodes_basic_all))
                .nest(
                    "/mixnodes",
                    Router::new()
                        .route("/active", get(mixnodes_basic_active))
                        .route("/all", get(mixnodes_basic_all)),
                )
                .route("/entry-gateways", get(entry_gateways_basic_all))
                .route("/exit-gateways", get(exit_gateways_basic_all)),
        )
        .nest(
            "/semi-skimmed",
            Router::new().route("/", get(nodes_expanded)),
        )
        // TEMPORARY, to be deleted once liveness is either weighted or dropped - see the module
        .route("/liveness-divergence", get(liveness_divergence))
        .layer(CompressionLayer::new())
}

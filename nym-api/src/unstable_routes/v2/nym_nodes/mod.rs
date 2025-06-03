// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::support::http::state::AppState;
use crate::unstable_routes::v2::nym_nodes::skimmed::{
    entry_gateways_basic_all, exit_gateways_basic_all, mixnodes_basic_active, mixnodes_basic_all,
    nodes_basic_all,
};
use axum::routing::get;
use axum::Router;
use tower_http::compression::CompressionLayer;

pub(crate) mod helpers;
pub(crate) mod skimmed;

#[allow(deprecated)]
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
        .layer(CompressionLayer::new())
}

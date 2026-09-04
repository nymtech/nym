// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::http::state::AppState;
use crate::unstable_routes::v3::nym_nodes::liveness_divergence::liveness_divergence;
use crate::unstable_routes::v3::nym_nodes::semi_skimmed::nodes_expanded;
use axum::routing::get;
use axum::Router;
use tower_http::compression::CompressionLayer;

pub(crate) mod liveness_divergence;
pub(crate) mod semi_skimmed;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .nest(
            "/semi-skimmed",
            Router::new().route("/", get(nodes_expanded)),
        )
        // TEMPORARY, to be deleted once liveness is either weighted or dropped - see the module
        .route("/liveness-divergence", get(liveness_divergence))
        .layer(CompressionLayer::new())
}

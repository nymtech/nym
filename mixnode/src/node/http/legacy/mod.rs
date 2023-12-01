// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::legacy::description::description;
use crate::node::http::legacy::hardware::hardware;
use crate::node::http::legacy::state::MixnodeAppState;
use crate::node::http::legacy::stats::stats;
use crate::node::http::legacy::verloc::verloc;
use crate::node::node_description::NodeDescription;
use axum::http::{StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;

pub(crate) mod description;
pub(crate) mod hardware;
pub(crate) mod state;
pub(crate) mod stats;
pub(crate) mod verloc;

pub(crate) async fn not_found(uri: Uri) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        format!("I couldn't find '{uri}'. Try something else?"),
    )
}

pub(crate) mod api_routes {
    pub(crate) const VERLOC: &str = "/verloc";
    pub(crate) const DESCRIPTION: &str = "/description";
    pub(crate) const STATS: &str = "/stats";
    pub(crate) const HARDWARE: &str = "/hardware";
}

pub(crate) fn routes<S: Send + Sync + 'static + Clone>(
    state: MixnodeAppState,
    descriptor: NodeDescription,
) -> Router<S> {
    Router::new()
        .route(api_routes::VERLOC, get(verloc))
        .route(
            api_routes::DESCRIPTION,
            get(move |query| description(descriptor, query)),
        )
        .route(api_routes::STATS, get(stats))
        .route(api_routes::HARDWARE, get(hardware))
        .fallback(not_found)
        .with_state(state)
}

// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use axum::{
    http::{HeaderValue, Request},
    middleware::Next,
    response::IntoResponse,
};
use hyper::header::{HOST, USER_AGENT};
use tracing::{debug, info};

/// Simple logger for requests
pub async fn logger<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    let method = req.method().to_string();
    let uri = req.uri().to_string();
    let agent = header_map(
        req.headers().get(USER_AGENT),
        "Unknown User Agent".to_string(),
    );

    let host = header_map(req.headers().get(HOST), "Unknown Host".to_string());

    info!("[{host}] requester {method}: {uri} via {agent}");

    next.run(req).await
}

fn header_map(header: Option<&HeaderValue>, msg: String) -> String {
    header
        .map(|x| x.to_str().unwrap_or(&msg).to_string())
        .unwrap_or(msg)
}

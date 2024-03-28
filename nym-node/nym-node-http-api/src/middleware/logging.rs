// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::{
    extract::ConnectInfo,
    http::{HeaderValue, Request},
    middleware::Next,
    response::IntoResponse,
};
use colored::*;
use hyper::header::{HOST, USER_AGENT};
use std::net::SocketAddr;
use tracing::info;

/// Simple logger for requests
pub async fn logger<B>(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<B>,
    next: Next<B>,
) -> impl IntoResponse {
    let method = req.method().to_string().green();
    let uri = req.uri().to_string().blue();
    let agent = header_map(
        req.headers().get(USER_AGENT),
        "Unknown User Agent".to_string(),
    );

    let host = header_map(req.headers().get(HOST), "Unknown Host".to_string());

    let res = next.run(req).await;
    let status = res.status();
    let print_status = if status.is_client_error() || status.is_server_error() {
        status.to_string().red()
    } else if status.is_success() {
        status.to_string().green()
    } else {
        status.to_string().yellow()
    };

    info!(target: "incoming request", "[{addr} -> {host}] {method} '{uri}': {print_status} / agent: {agent}");

    res
}

fn header_map(header: Option<&HeaderValue>, msg: String) -> String {
    header
        .map(|x| x.to_str().unwrap_or(&msg).to_string())
        .unwrap_or(msg)
}

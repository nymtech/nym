// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::extract::{ConnectInfo, Request};
use axum::middleware::Next;
use axum::response::IntoResponse;
use colored::Colorize;
use http::header::{HOST, USER_AGENT};
use http::HeaderValue;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Instant;
use tracing::info;

#[derive(Serialize, Deserialize, FromForm, Debug, JsonSchema)]
pub struct PaginationRequest {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

// TODO dz move this fn to a "common" place so this can be exported ?
pub async fn logger(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> impl IntoResponse {
    // TODO dz use `OriginalUri` extractor to get full URI even for nested
    // routers if routes aren't logged correctly in handlers
    fn header_map(header: Option<&HeaderValue>, msg: String) -> String {
        header
            .map(|x| x.to_str().unwrap_or(&msg).to_string())
            .unwrap_or(msg)
    }

    let method = request.method().to_string().green();
    let uri = request.uri().to_string().blue();
    let agent = header_map(
        request.headers().get(USER_AGENT),
        "Unknown User Agent".to_string(),
    );

    let host = header_map(request.headers().get(HOST), "Unknown Host".to_string());

    let start = Instant::now();
    // run request through all middleware, incl. extractors
    let res = next.run(request).await;
    let time_taken = start.elapsed();
    let status = res.status();
    let print_status = if status.is_client_error() || status.is_server_error() {
        status.to_string().red()
    } else if status.is_success() {
        status.to_string().green()
    } else {
        status.to_string().yellow()
    };

    let taken = "time taken".bold();

    let time_taken = match time_taken.as_millis() {
        ms if ms > 500 => format!("{taken}: {}", format!("{ms}ms").red()),
        ms if ms > 200 => format!("{taken}: {}", format!("{ms}ms").yellow()),
        ms if ms > 50 => format!("{taken}: {}", format!("{ms}ms").bright_yellow()),
        ms => format!("{taken}: {ms}ms"),
    };

    let agent_str = "agent".bold();

    info!("[{addr} -> {host}] {method} '{uri}': {print_status} {time_taken} {agent_str}: {agent}");

    res
}

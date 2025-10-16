// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use axum::extract::Request;
use axum::http::header::{HOST, USER_AGENT};
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum_client_ip::InsecureClientIp;
use colored::Colorize;
use std::time::Instant;
use tracing::{debug, info};

enum LogLevel {
    Debug,
    Info,
}

pub async fn log_request_info(
    insecure_client_ip: InsecureClientIp,
    request: Request,
    next: Next,
) -> impl IntoResponse {
    log_request(insecure_client_ip, request, next, LogLevel::Info).await
}

pub async fn log_request_debug(
    insecure_client_ip: InsecureClientIp,
    request: Request,
    next: Next,
) -> impl IntoResponse {
    log_request(insecure_client_ip, request, next, LogLevel::Debug).await
}

/// Simple logger for requests
async fn log_request(
    InsecureClientIp(addr): InsecureClientIp,
    request: Request,
    next: Next,
    level: LogLevel,
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

    // Extract traceparent from headers if it exists
    #[cfg(feature = "otel")]
    let traceparent = request
        .headers()
        .get("traceparent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    #[cfg(not(feature = "otel"))]
    let traceparent: Option<String> = None;

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

    match level {
        LogLevel::Debug => debug!(
            "[{addr} -> {host}] {method} '{uri}': {print_status} {time_taken} {agent_str}: {agent} traceparent: {traceparent:?}",
        ),
        LogLevel::Info => info!(
            "[{addr} -> {host}] {method} '{uri}': {print_status} {time_taken} {agent_str}: {agent} traceparent: {traceparent:?}"
        ),
    }

    res
}

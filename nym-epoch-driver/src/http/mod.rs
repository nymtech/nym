// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Context;
use axum::Router;
use nym_http_api_common::middleware::logging::log_request_info;
use nym_task::ShutdownToken;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;

mod metrics;

/// The driver's HTTP API, which serves its Prometheus metrics and nothing else.
pub(crate) struct HttpServer {
    router: Router,
    listener: TcpListener,
    shutdown_token: ShutdownToken,
}

impl HttpServer {
    /// Fails when `bind_address` cannot be bound.
    pub(crate) async fn bind(
        bind_address: SocketAddr,
        metrics_bearer_token: String,
        shutdown_token: ShutdownToken,
    ) -> anyhow::Result<Self> {
        let router = Router::new()
            .nest(
                "/v1",
                Router::new().nest("/metrics", metrics::routes(metrics_bearer_token)),
            )
            .layer(axum::middleware::from_fn(log_request_info));

        let listener = TcpListener::bind(bind_address)
            .await
            .with_context(|| format!("failed to bind the http api to {bind_address}"))?;
        info!("serving the http api on {bind_address}");

        Ok(HttpServer {
            router,
            listener,
            shutdown_token,
        })
    }

    pub(crate) async fn run(self) -> std::io::Result<()> {
        // the request logger takes the client's address from the connection info
        axum::serve(
            self.listener,
            self.router
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move { self.shutdown_token.cancelled().await })
        .await
    }
}

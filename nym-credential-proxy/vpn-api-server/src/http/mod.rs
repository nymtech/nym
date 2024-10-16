// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::VpnApiError;
use crate::http::router::build_router;
use crate::http::state::ApiState;
use axum::Router;
use std::net::SocketAddr;
use tokio_util::sync::CancellationToken;
use tracing::info;

pub mod helpers;
pub mod middleware;
pub mod router;
pub mod state;
pub mod types;

pub struct HttpServer {
    bind_address: SocketAddr,
    cancellation: CancellationToken,
    router: Router,
}

impl HttpServer {
    pub fn new(bind_address: SocketAddr, state: ApiState, auth_token: String) -> Self {
        HttpServer {
            bind_address,
            cancellation: state.cancellation_token(),
            router: build_router(state, auth_token),
        }
    }

    pub async fn run_forever(self) -> Result<(), VpnApiError> {
        let address = self.bind_address;
        info!("starting the http server on http://{address}");

        let listener = tokio::net::TcpListener::bind(address)
            .await
            .map_err(|source| VpnApiError::SocketBindFailure { address, source })?;

        let cancellation = self.cancellation;

        axum::serve(
            listener,
            self.router
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move { cancellation.cancelled().await })
        .await
        .map_err(|source| VpnApiError::HttpServerFailure { source })
    }
}

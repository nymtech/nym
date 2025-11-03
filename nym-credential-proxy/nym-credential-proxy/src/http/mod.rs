// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::router::build_router;
use crate::http::state::ApiState;
use nym_credential_proxy_lib::error::CredentialProxyError;
use std::net::SocketAddr;
use tracing::info;

pub mod router;
pub mod state;

pub struct HttpServer {
    bind_address: SocketAddr,
    state: ApiState,
    auth_token: String,
}

impl HttpServer {
    pub fn new(bind_address: SocketAddr, state: ApiState, auth_token: String) -> Self {
        HttpServer {
            bind_address,
            state,
            auth_token,
        }
    }

    pub fn spawn_as_task(self) {
        let cancellation = self.state.ticketbooks().shutdown_token();

        // not the best name, but that's due to the branch rotting,
        // where refactoring would be counter-productive
        let ticketbook_manager = self.state.ticketbooks().clone();
        ticketbook_manager.try_spawn_in_background(async move {
            let address = self.bind_address;
            let router = build_router(self.state, self.auth_token);
            info!("starting the http server on http://{address}");

            let listener = tokio::net::TcpListener::bind(address)
                .await
                .map_err(|source| CredentialProxyError::SocketBindFailure { address, source })?;

            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async move { cancellation.cancelled().await })
            .await
            .map_err(|source| CredentialProxyError::HttpServerFailure { source })
        });
    }
}

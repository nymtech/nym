// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::Router;
use nym_task::ShutdownToken;
use std::net::SocketAddr;
use tracing::{error, info};

pub(crate) mod burst;
pub(crate) mod error;
pub(crate) mod replay;
pub(crate) mod router;
pub(crate) mod state;

/// Binds to `bind_address` and serves the given router until the shutdown token is cancelled.
/// The listener is created with `into_make_service_with_connect_info` so handlers can
/// extract the peer [`SocketAddr`].
pub(crate) async fn run_http_server(
    router: Router,
    bind_address: SocketAddr,
    shutdown_token: ShutdownToken,
) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(bind_address)
        .await
        .inspect_err(|err| error!("couldn't bind to address {bind_address}: {err}"))?;

    info!("starting http api server on {bind_address}");

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move { shutdown_token.cancelled().await })
    .await?;

    Ok(())
}

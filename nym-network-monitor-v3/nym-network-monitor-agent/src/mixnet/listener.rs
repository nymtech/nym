// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet::connection_handler::{ConnectionHandler, SharedHandlerData};
use nym_task::ShutdownToken;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinSet;
use tracing::{debug, error, info};

/// Accepts the connections on which a wave's targets return their test packets.
///
/// ONE listener serves every target of a wave, and it reads nothing itself: each accepted connection
/// gets a [`ConnectionHandler`] of its own, which refuses it or upgrades it and delivers what arrives
/// to the target its source resolves to.
pub(crate) struct MixnetListener {
    /// Local TCP listener.
    tcp_listener: tokio::net::TcpListener,

    /// Cloned into each connection's handler.
    shared_handler_data: SharedHandlerData,

    /// Global shutdown token
    shutdown: ShutdownToken,
}

impl MixnetListener {
    /// Binds the listener, ready to be started with [`run`](Self::run).
    pub(crate) async fn new(
        bind_address: SocketAddr,
        shared_handler_data: SharedHandlerData,
        shutdown: ShutdownToken,
    ) -> anyhow::Result<Self> {
        info!("attempting to run mixnet listener on {bind_address}");

        let tcp_listener = tokio::net::TcpListener::bind(bind_address)
            .await
            .inspect_err(|err| {
                error!("Failed to the mixnet listener bind to {bind_address}: {err}")
            })?;

        Ok(Self {
            tcp_listener,
            shared_handler_data,
            shutdown,
        })
    }

    /// Accepts connections until the shutdown token is cancelled, then waits for the handlers out.
    ///
    /// They watch the same token, so they return of their own accord rather than being aborted
    /// mid-read, and anything already off the wire still reaches its target. Draining them here is
    /// what makes the run's teardown ordered rather than racing the tester's harvest.
    pub(crate) async fn run(self, on_start: Arc<Notify>) {
        on_start.notify_one();

        let mut handlers = JoinSet::new();

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    debug!("mixnet listener: received shutdown");
                    while handlers.join_next().await.is_some() {}
                    return;
                }
                accepted = self.tcp_listener.accept() => {
                    let Ok((socket, source)) = accepted else {
                        error!("failed to accept a TCP connection from the mixnet listener");
                        continue;
                    };

                    let handler = ConnectionHandler::new(self.shared_handler_data.clone(), source);
                    handlers.spawn(async move { handler.handle_connection(socket).await });
                }
            }
        }
    }
}

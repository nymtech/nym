// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use axum::extract::ConnectInfo;
use axum::middleware::AddExtension;
use axum::serve::Serve;
use axum::Router;
use nym_task::TaskClient;
use std::net::SocketAddr;
use tracing::{debug, error};

pub use router::{api, HttpServerConfig, NymNodeRouter};

pub mod error;
pub mod helpers;
pub mod middleware;
pub mod router;
pub mod state;

type InnerService = IntoMakeServiceWithConnectInfo<Router, SocketAddr>;
type ConnectInfoExt = AddExtension<Router, ConnectInfo<SocketAddr>>;
pub type ServeService = Serve<InnerService, ConnectInfoExt>;

pub struct NymNodeHttpServer {
    task_client: Option<TaskClient>,
    inner: ServeService,
}

impl NymNodeHttpServer {
    pub(crate) fn new(inner: ServeService) -> Self {
        NymNodeHttpServer {
            task_client: None,
            inner,
        }
    }

    #[must_use]
    pub fn with_task_client(mut self, task_client: TaskClient) -> Self {
        self.task_client = Some(task_client);
        self
    }

    async fn run_server_forever(server: ServeService) {
        if let Err(err) = server.await {
            error!("the HTTP server has terminated with the error: {err}");
        } else {
            error!("the HTTP server has terminated with producing any errors");
        }
    }

    pub async fn run(self) {
        if let Some(mut task_client) = self.task_client {
            tokio::select! {
                _ = task_client.recv_with_delay() => {
                    debug!("NymNodeHTTPServer: Received shutdown");
                }
                _ = Self::run_server_forever(self.inner) => { }
            }
        } else {
            Self::run_server_forever(self.inner).await
        }

        debug!("NymNodeHTTPServer: Exiting");
    }
}

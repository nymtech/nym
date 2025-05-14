// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub(crate) mod helpers;
pub(crate) mod openapi;
pub(crate) mod router;
pub(crate) mod state;

use nym_task::TaskManager;
pub(crate) use router::RouterBuilder;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::unstable_routes;

pub(crate) const TASK_MANAGER_TIMEOUT_S: u64 = 10;

/// Shutdown goes 2 directions:
/// 1. signal background tasks to gracefully finish
/// 2. signal server itself
///
/// These are done through separate shutdown handles. Of course, shut down server
/// AFTER you have shut down BG tasks (or past their grace period).
pub(crate) struct ShutdownHandles {
    task_manager: TaskManager,
    axum_shutdown_button: ShutdownAxum,
    /// Tokio JoinHandle for axum server's task
    axum_join_handle: AxumJoinHandle,
}

impl ShutdownHandles {
    /// Cancellation token is given to Axum server constructor. When the token
    /// receives a shutdown signal, Axum server will shut down gracefully.
    pub(crate) fn new(
        task_manager: TaskManager,
        axum_server_handle: AxumJoinHandle,
        shutdown_button: CancellationToken,
    ) -> Self {
        Self {
            task_manager,
            axum_shutdown_button: ShutdownAxum(shutdown_button.clone()),
            axum_join_handle: axum_server_handle,
        }
    }

    pub(crate) fn task_manager_mut(&mut self) -> &mut TaskManager {
        &mut self.task_manager
    }

    /// Signal server to shut down, then return join handle to its
    /// `tokio` task
    ///
    /// https://tikv.github.io/doc/tokio/task/struct.JoinHandle.html
    #[must_use]
    pub(crate) fn shutdown_axum(self) -> AxumJoinHandle {
        self.axum_shutdown_button.0.cancel();
        self.axum_join_handle
    }
}

struct ShutdownAxum(CancellationToken);

type AxumJoinHandle = JoinHandle<std::io::Result<()>>;

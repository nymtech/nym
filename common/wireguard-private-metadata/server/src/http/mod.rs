// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_wireguard::WgApiWrapper;
use std::sync::Arc;
use tokio::task::JoinHandle;

pub(crate) mod openapi;
pub(crate) mod router;
pub(crate) mod state;

/// Shutdown goes 2 directions:
/// 1. signal background tasks to gracefully finish
/// 2. signal server itself
///
/// These are done through separate shutdown handles. Of course, shut down server
/// AFTER you have shut down BG tasks (or past their grace period).
#[allow(unused)]
pub struct ShutdownHandles {
    /// Tokio JoinHandle for axum server's task
    axum_join_handle: AxumJoinHandle,
    /// Wireguard API for kernel interactions
    wg_api: Arc<WgApiWrapper>,
}

impl ShutdownHandles {
    /// Cancellation token is given to Axum server constructor. When the token
    /// receives a shutdown signal, Axum server will shut down gracefully.
    pub fn new(axum_join_handle: AxumJoinHandle, wg_api: Arc<WgApiWrapper>) -> Self {
        Self {
            axum_join_handle,
            wg_api,
        }
    }
}

type AxumJoinHandle = JoinHandle<std::io::Result<()>>;

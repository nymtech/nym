// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(target_os = "linux")]
mod error;
#[cfg(target_os = "linux")]
mod http;
#[cfg(target_os = "linux")]
mod models;
#[cfg(target_os = "linux")]
mod network;
#[cfg(target_os = "linux")]
mod transceiver;

#[cfg(target_os = "linux")]
pub use http::{
    router::{ApiHttpServer, RouterBuilder, RouterWithState},
    state::AppState,
    ShutdownHandles,
};
#[cfg(target_os = "linux")]
pub use transceiver::PeerControllerTransceiver;

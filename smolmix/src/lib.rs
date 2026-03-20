// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

#![doc = include_str!("ARCHITECTURE.md")]

mod bridge;
mod device;
mod error;
mod tunnel;

pub use error::SmolmixError;
pub use tunnel::{NetworkEnvironment, TcpStream, Tunnel, UdpSocket};

/// Initialise the default tracing/logging subscriber.
pub fn init_logging() {
    nym_bin_common::logging::setup_tracing_logger();
}

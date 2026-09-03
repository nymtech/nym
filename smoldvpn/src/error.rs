// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// Errors from the `smoldvpn` datapath.
#[derive(Error, Debug)]
pub enum DvpnError {
    /// A transport (UDP / QUIC bridge) operation failed.
    #[error("transport error: {0}")]
    Transport(String),

    /// The underlying smol-core stack errored.
    #[error("stack error: {0}")]
    Stack(#[from] nym_smol_core::SmolCoreError),

    /// Socket / IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Setup or teardown was cancelled via the `CancellationToken`.
    #[error("cancelled")]
    Cancelled,

    /// Invalid configuration.
    #[error("invalid configuration: {0}")]
    Config(String),

    /// QUIC bridging was requested for a one-hop tunnel (only valid on the
    /// two-hop entry leg).
    #[error("QUIC bridge is only valid on the two-hop entry leg")]
    QuicRequiresTwoHop,

    /// The QUIC bridge handshake / connection failed.
    #[error("QUIC bridge error: {0}")]
    Bridge(String),
}

/// Result alias for the datapath.
pub type Result<T> = std::result::Result<T, DvpnError>;

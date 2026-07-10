// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

use thiserror::Error;

/// Errors from the `smol-dvpn` datapath.
#[derive(Error, Debug)]
pub enum DvpnError {
    /// A boringtun WireGuard operation failed.
    #[error("WireGuard error: {0}")]
    WireGuard(String),

    /// A transport (UDP / QUIC bridge) operation failed.
    #[error("transport error: {0}")]
    Transport(String),

    /// The underlying smol-core stack errored.
    #[error("stack error: {0}")]
    Stack(#[from] smol_core::SmolCoreError),

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

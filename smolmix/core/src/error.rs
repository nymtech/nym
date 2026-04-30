// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SmolmixError {
    /// The internal channel between the bridge and the network device was closed.
    ///
    /// This typically means the bridge task has exited — either because the tunnel
    /// was shut down or the mixnet connection was lost.
    #[error("Channel closed")]
    ChannelClosed,

    /// The IPR handshake has not completed.
    ///
    /// Returned from [`Tunnel::from_stream`](crate::Tunnel::from_stream) when
    /// the provided `IpMixStream` is not in a connected state.
    #[error("Not connected to IPR")]
    NotConnected,

    /// An error from the Nym SDK (mixnet client, gateway connection, etc.).
    #[error("Nym SDK error: {0}")]
    NymSdk(#[from] nym_sdk::Error),

    /// An I/O error from the underlying TCP/UDP socket operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

use thiserror::Error;

/// Error type for all fallible `smol-core` operations.
#[derive(Error, Debug)]
pub enum SmolCoreError {
    /// A transport/device channel was closed.
    #[error("Channel closed")]
    ChannelClosed,

    /// Underlying socket / stack I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// DNS query timed out.
    #[error("DNS query for {name} timed out")]
    DnsTimeout { name: String },

    /// DNS message could not be encoded/decoded.
    #[error("DNS protocol error: {0}")]
    DnsProto(String),

    /// DNS lookup returned no address records.
    #[error("no address records for {name}")]
    DnsNoRecords { name: String },
}

/// Convenient result alias for `smol-core`.
pub type Result<T> = std::result::Result<T, SmolCoreError>;

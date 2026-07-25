// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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

    /// The DNS server returned a failure response code (e.g. SERVFAIL, REFUSED) — distinct from a
    /// genuinely empty/NXDOMAIN result, and (unlike NXDOMAIN) typically retryable.
    #[error("DNS server returned {rcode} for {name}")]
    DnsServerFailure { name: String, rcode: String },

    /// The DNS response was truncated (TC bit set); it is incomplete and RFC 1035 requires a TCP
    /// retry, which this datagram-only resolver does not perform.
    #[error("DNS response for {name} was truncated (TC bit set)")]
    DnsTruncated { name: String },
}

/// Convenient result alias for `smol-core`.
pub type Result<T> = std::result::Result<T, SmolCoreError>;

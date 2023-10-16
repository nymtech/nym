// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::EXIT_POLICY_FIELD_NAME;
use std::net::AddrParseError;
use thiserror::Error;

/// Error from an unparsable or invalid policy.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PolicyError {
    #[cfg(feature = "client")]
    #[error("failed to fetch the remote policy: {source}")]
    ClientError {
        #[from]
        source: reqwest::Error,
    },

    #[error("/{mask} is not a valid mask for an IpV4 address")]
    InvalidIpV4Mask { mask: u8 },

    #[error("/{mask} is not a valid mask for an IpV6 address")]
    InvalidIpV6Mask { mask: u8 },

    #[error("'{action}' is not a valid policy action")]
    InvalidPolicyAction { action: String },

    #[error("'{addr}' is not a valid Ip address: {source}")]
    MalformedIpAddress {
        addr: String,
        #[source]
        source: AddrParseError,
    },

    /// Attempted to use a bitmask with the address "*".
    #[error("attempted to use a bitmask ('/{mask}') with the address '*'")]
    MaskWithStar { mask: String },

    /// Attempted to use a bitmask with the address "*4".
    #[error("attempted to use a bitmask ('/{mask}') with the address '*4'")]
    MaskWithV4Star { mask: String },

    /// Attempted to use a bitmask with the address "*6".
    #[error("attempted to use a bitmask ('/{mask}') with the address '*6'")]
    MaskWithV6Star { mask: String },

    #[error("'/{mask}' is not a valid mask")]
    InvalidMask { mask: String },

    /// A port was not a number in the range 1..65535
    #[error(
        "the provided port '{raw}' was either malformed or was not in the valid 1..65535 range"
    )]
    InvalidPort { raw: String },

    /// A port range had its starting-point higher than its ending point.
    #[error("the provided port range ({start}-{end}) was invalid. either the start was 0 or it was greater than the end.")]
    InvalidRange { start: u16, end: u16 },

    #[error("could not parse '{raw}' into a valid policy address:port pattern")]
    MalformedAddressPortPattern { raw: String },

    #[error("could not parse '{raw}' into a valid address policy")]
    MalformedAddressPolicy { raw: String },

    #[error(
        "the provided exit policy entry does not start with the expected '{}' prefix: '{entry}'",
        EXIT_POLICY_FIELD_NAME
    )]
    NoExitPolicyPrefix { entry: String },
}

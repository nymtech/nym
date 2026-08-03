// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_validator_client::nym_api::error::NymAPIError;
use nym_validator_client::nyxd::error::NyxdError;
use std::net::SocketAddr;

/// Errors produced while provisioning a dVPN session.
#[derive(thiserror::Error, Debug)]
pub enum SessionError {
    /// Failed to construct or talk to the nyxd chain client.
    #[error("nyxd chain client error: {0}")]
    Nyxd(#[from] NyxdError),

    /// A nym-api request failed.
    #[error("nym-api error: {0}")]
    Api(#[from] NymAPIError),

    /// The configured network has no usable validator/API endpoint.
    #[error("network details contain no usable {which} endpoint")]
    MissingEndpoint { which: &'static str },

    /// A URL in the network details could not be parsed.
    #[error("invalid {which} url {url}: {source}")]
    InvalidUrl {
        which: &'static str,
        url: String,
        #[source]
        source: url::ParseError,
    },

    /// Credential storage could not be opened.
    #[error("credential storage error: {0}")]
    Storage(String),

    /// Ticketbook deposit / issuance failed.
    #[error("ticketbook issuance error: {0}")]
    Issuance(String),

    /// Provisioning did not complete within its overall budget — most likely the ecash
    /// signers / nym-apis are unresponsive. Any deposit already made is recorded in the
    /// fetcher's pending-request store and is recovered (without re-depositing) on a retry.
    #[error(
        "provisioning timed out after {after:?} — the ecash signers are likely unresponsive; \
         any deposit already made is recoverable from the pending-request store on retry"
    )]
    ProvisioningTimeout { after: std::time::Duration },

    /// No gateway matched the requested identity.
    #[error("no gateway found with identity {0}")]
    GatewayNotFound(String),

    /// No WireGuard-capable gateway is available for the requested role.
    #[error("no WireGuard-capable gateway available for the requested role")]
    NoWireguardGateway,

    /// No WireGuard-capable gateway matched the requested country code.
    #[error("no WireGuard-capable gateway found in country {0}")]
    NoCountryMatch(String),

    /// A QUIC-bridge entry gateway was required but none is available for the
    /// requested selection (directory unavailable, or no QUIC gateway matches).
    #[error("no QUIC-bridge gateway available for selection: {spec}")]
    NoQuicGateway { spec: String },

    #[error("entry and exit resolved to the same gateway ({0}); a two-hop tunnel needs distinct gateways")]
    SameGatewaySelected(String),

    /// A selected gateway advertised malformed LP data.
    #[error("gateway {identity} advertised malformed LP data: {reason}")]
    MalformedGateway { identity: String, reason: String },

    /// Gateway registration failed.
    #[error("registration with gateway at {address} failed: {source}")]
    Registration {
        address: SocketAddr,
        #[source]
        source: nym_registration_client::LpClientError,
    },

    /// The setup/issuance phase was cancelled via the CancellationToken.
    #[error("session setup was cancelled")]
    Cancelled,

    #[error("failed to generate randomness: {0}")]
    RngFailure(#[from] getrandom04::Error),
}

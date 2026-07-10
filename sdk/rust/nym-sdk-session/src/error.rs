// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

use std::net::SocketAddr;

/// Errors produced while provisioning a dVPN session.
#[derive(thiserror::Error, Debug)]
pub enum SessionError {
    /// Failed to construct or talk to the nyxd chain client.
    #[error("chain client error: {0}")]
    Chain(String),

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

    /// No gateway matched the requested identity.
    #[error("no gateway found with identity {0}")]
    GatewayNotFound(String),

    /// No WireGuard-capable gateway is available for the requested role.
    #[error("no WireGuard-capable gateway available for the requested role")]
    NoWireguardGateway,

    /// No WireGuard-capable gateway matched the requested country code.
    #[error("no WireGuard-capable gateway found in country {0}")]
    NoCountryMatch(String),

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
}

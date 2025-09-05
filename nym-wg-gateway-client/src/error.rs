// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_credentials_interface::TicketType;
use nym_gateway_directory::NodeIdentity;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("received invalid response from gateway authenticator")]
    InvalidGatewayAuthResponse,

    #[error("unknown authenticator version number")]
    UnsupportedAuthenticatorVersion,

    #[error(transparent)]
    AuthenticatorClientError(#[from] nym_authenticator_client::Error),

    #[error(transparent)]
    MetadataClientError(#[from] nym_wg_metadata_client::error::MetadataClientError),

    #[error("error that should stop auto retrying")]
    NoRetry {
        #[source]
        source: nym_authenticator_client::Error,
    },

    #[error("verification failure")]
    VerificationFailed(#[source] nym_authenticator_requests::Error),

    #[error("failed to parse entry gateway socket addr")]
    FailedToParseEntryGatewaySocketAddr(#[source] std::net::AddrParseError),

    #[error("failed to get {ticketbook_type} ticket")]
    GetTicket {
        ticketbook_type: TicketType,
        #[source]
        source: nym_bandwidth_controller::error::BandwidthControllerError,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum ErrorMessage {
    #[error("out of bandwidth for gateway: {gateway_id}")]
    OutOfBandwidth { gateway_id: Box<NodeIdentity> },

    #[error("gateway {gateway_id} is erroring out")]
    ErrorsFromGateway { gateway_id: Box<NodeIdentity> },
}

// Result type based on our error type
pub type Result<T> = std::result::Result<T, Error>;

// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_http_api_client::HttpClientError;
use nym_validator_client::nym_api::error::NymAPIError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("identity not formatted correctly: {identity}")]
    NodeIdentityFormattingError {
        identity: String,
        source: nym_crypto::asymmetric::ed25519::Ed25519RecoveryError,
    },

    #[error("recipient is not formatted correctly: {address}")]
    RecipientFormattingError {
        address: String,
        source: nym_sphinx::addressing::clients::RecipientFormattingError,
    },

    #[error(transparent)]
    ValidatorClientError(#[from] nym_validator_client::ValidatorClientError),

    #[error(transparent)]
    VpnApiClientError(#[from] nym_vpn_api_client::error::VpnApiClientError),

    #[error("failed to resolve gateway hostname: {hostname}")]
    FailedToDnsResolveGateway {
        hostname: String,
        source: nym_http_api_client::HickoryDnsError,
    },

    #[error("resolved hostname {0} but no IP address found")]
    ResolvedHostnameButNoIp(String),

    #[error("timed out while attempting to resolve hostname: {hostname}")]
    HostnameResolutionTimeout { hostname: String },

    #[error("failed to lookup described gateways")]
    FailedToLookupDescribedGateways(#[source] nym_validator_client::ValidatorClientError),

    #[error("failed to lookup skimmed gateways")]
    FailedToLookupSkimmedGateways(#[source] nym_validator_client::ValidatorClientError),

    #[error("failed to lookup skimmed nodes")]
    FailedToLookupSkimmedNodes(#[source] nym_validator_client::ValidatorClientError),

    #[error("requested gateway not found in the remote list: {0}")]
    RequestedGatewayIdNotFound(String),

    #[error("missing ip packet router address for gateway")]
    MissingIpPacketRouterAddress,

    #[error("missing hostname or ip address for gateway")]
    MissingHostnameOrIpAddress { gateway_identity: String },

    #[error("no matching gateway found: {requested_identity}")]
    NoMatchingGateway { requested_identity: String },

    #[error(
        "no entry gateway available for location {requested_location}, available countries: {available_countries:?}"
    )]
    NoMatchingEntryGatewayForLocation {
        requested_location: String,
        available_countries: Vec<String>,
    },

    #[error(
        "no exit gateway available for location {requested_location}, available countries: {available_countries:?}"
    )]
    NoMatchingExitGatewayForLocation {
        requested_location: String,
        available_countries: Vec<String>,
    },

    #[error("no matching gateway found after selecting low latency: {requested_identity}")]
    NoMatchingGatewayAfterSelectingLowLatency { requested_identity: String },

    #[error("failed to select gateway randomly")]
    FailedToSelectGatewayRandomly,

    #[error("gateway {0} doesn't have a description available")]
    NoGatewayDescriptionAvailable(String),

    #[error("failed to lookup gateway ip for gateway {0}")]
    FailedToLookupIp(String),

    #[error("the url {url} doesn't parse to a host and/or a port: {reason}")]
    UrlError { url: url::Url, reason: String },

    #[error("the provided gateway information is malformed")]
    MalformedGateway,

    #[error("no connectivity")]
    Offline,

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] Box<HttpClientError>),

    #[error("Nym API error: {source}")]
    NymApi { source: Box<NymAPIError> },

    #[error("operation cancelled")]
    Cancelled,

    #[error("invalid score value: {0}. Valid values are: offline, low, medium, high")]
    InvalidScoreValue(String),
}

impl Error {
    /// Returns true when no gateways matching the search criteria could be found, except when the gateway is constrained to identity
    pub fn is_unmatched_non_specific_gateway(&self) -> bool {
        matches!(
            self,
            Error::NoMatchingEntryGatewayForLocation { .. }
                | Error::NoMatchingExitGatewayForLocation { .. }
                | Error::FailedToSelectGatewayRandomly
        )
    }
}

// Result type based on our error type
pub type Result<T> = std::result::Result<T, Error>;

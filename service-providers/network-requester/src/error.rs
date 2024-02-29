// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub use nym_client_core::error::ClientCoreError;

use nym_exit_policy::policy::PolicyError;
use nym_id_lib::NymIdError;
use nym_socks5_requests::{RemoteAddress, Socks5RequestError};
use std::net::SocketAddr;

#[derive(thiserror::Error, Debug)]
pub enum NetworkRequesterError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("client-core error: {0}")]
    ClientCoreError(#[from] ClientCoreError),

    #[error("encountered an error while trying to handle a provider request: {source}")]
    ProviderRequestError {
        #[from]
        source: Socks5RequestError,
    },

    #[error("failed to load configuration file: {0}")]
    FailedToLoadConfig(String),

    // TODO: add more details here
    #[error("Failed to validate the loaded config")]
    ConfigValidationFailure,

    #[error("failed local version check, client and config mismatch")]
    FailedLocalVersionCheck,

    #[error("failed to setup mixnet client: {source}")]
    FailedToSetupMixnetClient { source: nym_sdk::Error },

    #[error("failed to connect to mixnet: {source}")]
    FailedToConnectToMixnet { source: nym_sdk::Error },

    #[error("the entity wrapping the network requester has disconnected")]
    DisconnectedParent,

    #[error("the provided socket address, '{addr}' is not covered by the exit policy!")]
    AddressNotCoveredByExitPolicy { addr: SocketAddr },

    #[error(
        "could not resolve socket address for the provided remote address '{remote}': {source}"
    )]
    CouldNotResolveHost {
        remote: RemoteAddress,
        source: std::io::Error,
    },

    #[error("the provided address: '{remote}' was somehow resolved to an empty list of socket addresses")]
    EmptyResolvedAddresses { remote: RemoteAddress },

    #[error("failed to apply the exit policy: {source}")]
    ExitPolicyFailure {
        #[from]
        source: PolicyError,
    },

    #[error("the url provided for the upstream exit policy source is malformed: {source}")]
    MalformedExitPolicyUpstreamUrl {
        #[source]
        source: reqwest::Error,
    },

    #[error("can't setup an exit policy without any upstream urls")]
    NoUpstreamExitPolicy,

    #[error(transparent)]
    NymIdError(#[from] NymIdError),
}

// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_http_api_client::HttpClientError;
use std::io;
use std::net::SocketAddr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NymNodeHttpError {
    #[error("failed to bind the HTTP API to {bind_address}: {source}")]
    HttpBindFailure {
        bind_address: SocketAddr,
        source: io::Error,
    },

    #[error("failed to use nym-node requests: {source}")]
    RequestError {
        #[from]
        source: nym_node_requests::error::Error,
    },

    #[error(transparent)]
    KeyRecoveryError {
        #[from]
        source: nym_crypto::asymmetric::x25519::KeyRecoveryError,
    },

    #[error("error building or using HTTP client: {source}")]
    ClientError {
        #[from]
        source: HttpClientError,
    },
}

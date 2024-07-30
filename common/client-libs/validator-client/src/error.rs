// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nym_api;
pub use tendermint_rpc::error::Error as TendermintRpcError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidatorClientError {
    #[error("nym api request failed: {source}")]
    NymAPIError {
        #[from]
        source: nym_api::error::NymAPIError,
    },

    #[error("Tendermint RPC request failure: {0}")]
    TendermintErrorRpc(#[from] TendermintRpcError),

    #[error("One of the provided URLs was malformed - {0}")]
    MalformedUrlProvided(#[from] url::ParseError),

    #[error("nyxd request failed: {0}")]
    NyxdError(#[from] crate::nyxd::error::NyxdError),

    #[error("No validator API url has been provided")]
    NoAPIUrlAvailable,
}

// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_validator_client::nyxd::error::NyxdError;
use std::net::IpAddr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ForwardTravelError {
    #[error("received a connection request from a forbidden address: '{address}'")]
    DisallowedIngressAddress { address: IpAddr },

    #[error("received a request to open connection to a forbidden address: '{address}'")]
    DisallowedEgressAddress { address: IpAddr },

    #[error("no valid nyxd urls are available for topology queries")]
    NoNyxdUrlsAvailable,

    #[error("nyxd interaction failure: {source}")]
    NyxdFailure {
        #[from]
        source: NyxdError,
    }, 
    
    #[error("the current epoch appears to be stuck")]
    StuckEpoch,
}

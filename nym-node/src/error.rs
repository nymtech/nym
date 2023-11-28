// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::wireguard::error::WireguardError;
use std::net::{IpAddr, SocketAddr};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NymNodeError {
    #[error("failed to bind the HTTP API to {bind_address}: {source}")]
    HttpBindFailure {
        bind_address: SocketAddr,
        source: hyper::Error,
    },

    #[error("this node hasn't set any valid public addresses to announce. Please modify [host.public_ips] section of your config")]
    NoPublicIps,

    #[error("this node attempted to announce an invalid public address: {address}. Please modify [host.public_ips] section of your config. Alternatively, if you wanted to use it in the local setting, run the node with the '--local' flag.")]
    InvalidPublicIp { address: IpAddr },

    #[error("failed to use nym-node requests: {source}")]
    RequestError {
        #[from]
        source: nym_node_requests::error::Error,
    },

    #[error(transparent)]
    WireguardError {
        #[from]
        source: WireguardError,
    },

    #[error(transparent)]
    KeyRecoveryError {
        #[from]
        source: nym_crypto::asymmetric::encryption::KeyRecoveryError,
    },

    #[error("unimplemented")]
    Unimplemented,
}

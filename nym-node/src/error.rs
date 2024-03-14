// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::wireguard::error::WireguardError;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NymNodeError {
    #[error("could not find an existing config file at '{}' and fresh node initialisation has been disabled", config_path.display())]
    ForbiddenInitialisation { config_path: PathBuf },

    #[error("could not derive path to data directory of this nym node")]
    DataDirDerivationFailure,

    #[error(
    "failed to load config file for using path '{}'. detailed message: {source}", path.display()
    )]
    ConfigLoadFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
    "failed to save config file for id {id} using path '{}'. detailed message: {source}", path.display()
    )]
    ConfigSaveFailure {
        id: String,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

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

    #[error("failed to load {keys} keys from {:?} (private key) and {:?} (public key): {err}", .paths.private_key_path, .paths.public_key_path)]
    KeyPairLoadFailure {
        keys: String,
        paths: nym_pemstore::KeyPairPath,
        #[source]
        err: io::Error,
    },

    #[error("failed to load {key} key from '{}': {err}", path.display())]
    KeyLoadFailure {
        key: String,
        path: PathBuf,
        #[source]
        err: io::Error,
    },

    #[error("failed to store {keys} keys to {:?} (private key) and {:?} (public key): {err}", .paths.private_key_path, .paths.public_key_path)]
    KeyPairStoreFailure {
        keys: String,
        paths: nym_pemstore::KeyPairPath,
        #[source]
        err: io::Error,
    },

    #[error("unimplemented")]
    Unimplemented,
}

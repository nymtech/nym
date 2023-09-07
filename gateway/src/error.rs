// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_validator_client::nyxd::AccountId;
use nym_validator_client::ValidatorClientError;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum GatewayError {
    #[error("failed to load {keys} keys from {:?} (private key) and {:?} (public key): {err}", .paths.private_key_path, .paths.public_key_path)]
    KeyPairLoadFailure {
        keys: String,
        paths: nym_pemstore::KeyPairPath,
        #[source]
        err: std::io::Error,
    },

    #[error(
        "failed to load config file for id {id} using path {path}. detailed message: {source}"
    )]
    ConfigLoadFailure {
        id: String,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
    "failed to load config file for network requester (gateway {id}) using path {path}. detailed message: {source}"
    )]
    NetworkRequesterConfigLoadFailure {
        id: String,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "failed to save config file for id {id} using path {path}. detailed message: {source}"
    )]
    ConfigSaveFailure {
        id: String,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("the configured version of the gateway ({config_version}) is incompatible with the binary version ({binary_version})")]
    LocalVersionCheckFailure {
        binary_version: String,
        config_version: String,
    },

    #[error("could not obtain the information about current gateways on the network: {source}")]
    NetworkGatewaysQueryFailure {
        #[source]
        source: ValidatorClientError,
    },

    #[error("address {account} has an invalid bech32 prefix. it uses '{actual_prefix}' while '{expected_prefix}' was expected")]
    InvalidBech32AccountPrefix {
        account: AccountId,
        expected_prefix: String,
        actual_prefix: String,
    },

    #[error("Path to network requester configuration file hasn't been specified.")]
    UnspecifiedNetworkRequesterConfig,

    #[error("local network requester has been terminated")]
    TerminatedNetworkRequester,
}

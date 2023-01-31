// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::io;
use std::path::PathBuf;
use thiserror::Error;
use validator_client::nyxd::AccountId;
use validator_client::ValidatorClientError;

#[derive(Debug, Error)]
pub(crate) enum GatewayError {
    #[error(
        "failed to load config file for id {id} using path {path}. detailed message: {source}"
    )]
    ConfigLoadFailure {
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

    #[error("another node on the network seems to have identical announce-host ({host})! Their identity is {remote_identity}, while ours is {local_identity}")]
    DuplicateNodeHost {
        host: String,
        local_identity: String,
        remote_identity: String,
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
}

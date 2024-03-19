// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::storage::error::StorageError;
use nym_ip_packet_router::error::IpPacketRouterError;
use nym_network_requester::error::{ClientCoreError, NetworkRequesterError};
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::ValidatorClientError;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum GatewayError {
    #[error("failed to load {keys} keys from '{}' (private key) and '{}' (public key): {err}", .paths.private_key_path.display(), .paths.public_key_path.display())]
    KeyPairLoadFailure {
        keys: String,
        paths: nym_pemstore::KeyPairPath,
        #[source]
        err: io::Error,
    },

    #[error("failed to load {key} public key from '{}': {err}", .path.display())]
    PublicKeyLoadFailure {
        key: String,
        path: PathBuf,
        #[source]
        err: io::Error,
    },

    #[error(
        "failed to load config file for id {id} using path '{}'. detailed message: {source}", path.display()
    )]
    ConfigLoadFailure {
        id: String,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "failed to load custom topology using path '{}'. detailed message: {source}", file_path.display()
        )]
    CustomTopologyLoadFailure {
        file_path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(
    "failed to load config file for network requester (gateway-id: '{id}') using path '{}'. detailed message: {source}", path.display()
    )]
    NetworkRequesterConfigLoadFailure {
        id: String,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
        "failed to load config file for ip packet router (gateway-id: '{id}') using path '{}'. detailed message: {source}",
        path.display()
    )]
    IpPacketRouterConfigLoadFailure {
        id: String,
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

    #[error("storage failure: {source}")]
    StorageError {
        #[from]
        source: StorageError,
    },

    #[error("Path to network requester configuration file hasn't been specified. Perhaps try to run `setup-network-requester`?")]
    UnspecifiedNetworkRequesterConfig,

    #[error("Path to ip packet router configuration file hasn't been specified. Perhaps try to run `setup-ip-packet-router`?")]
    UnspecifiedIpPacketRouterConfig,

    #[error("there was an issue with the local network requester: {source}")]
    NetworkRequesterFailure {
        #[from]
        source: NetworkRequesterError,
    },

    #[error("there was an issue with the local ip packet router: {source}")]
    IpPacketRouterFailure {
        #[from]
        source: IpPacketRouterError,
    },

    #[error("failed to startup local network requester")]
    NetworkRequesterStartupFailure,

    #[error("failed to startup local ip packet router")]
    IpPacketRouterStartupFailure,

    #[error("there are no nym API endpoints available")]
    NoNymApisAvailable,

    #[error("there are no nyxd endpoints available")]
    NoNyxdAvailable,

    #[error("there was an issue attempting to use the validator [nyxd]: {source}")]
    ValidatorFailure {
        #[from]
        source: NyxdError,
    },

    // TODO: in the future this should work the other way, i.e. NymNode depending on Gateway errors
    #[error(transparent)]
    NymNodeError(#[from] nym_node::error::NymNodeError),

    #[error("there was an issue with wireguard IP network: {source}")]
    IpNetworkError {
        #[from]
        source: ipnetwork::IpNetworkError,
    },
}

impl From<ClientCoreError> for GatewayError {
    fn from(value: ClientCoreError) -> Self {
        // if we ever get a client core error, it must have come from the network requester
        GatewayError::NetworkRequesterFailure {
            source: value.into(),
        }
    }
}

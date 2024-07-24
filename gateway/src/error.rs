// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::storage::error::StorageError;
use nym_authenticator::error::AuthenticatorError;
use nym_ip_packet_router::error::IpPacketRouterError;
use nym_network_requester::error::{ClientCoreError, NetworkRequesterError};
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::{AccountId, Coin};
use nym_validator_client::ValidatorClientError;
use std::io;
use std::net::IpAddr;
use std::path::PathBuf;
use thiserror::Error;

pub use crate::node::client_handling::websocket::connection_handler::authenticated::RequestHandlingError;

#[derive(Debug, Error)]
pub enum GatewayError {
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
        source: io::Error,
    },

    #[error(
    "failed to load config file for network requester (gateway-id: '{id}') using path '{}'. detailed message: {source}", path.display()
    )]
    NetworkRequesterConfigLoadFailure {
        id: String,
        path: PathBuf,
        source: io::Error,
    },

    #[error(
        "failed to load config file for ip packet router (gateway-id: '{id}') using path '{}'. detailed message: {source}",
        path.display()
    )]
    IpPacketRouterConfigLoadFailure {
        id: String,
        path: PathBuf,
        source: io::Error,
    },

    #[error(
        "failed to load config file for authenticator (gateway-id: '{id}') using path '{}'. detailed message: {source}",
        path.display()
    )]
    AuthenticatorConfigLoadFailure {
        id: String,
        path: PathBuf,
        source: io::Error,
    },

    #[error(
        "failed to load config file for wireguard (gateway-id: '{id}') using path '{}'. detailed message: {source}",
        path.display()
    )]
    WireguardConfigLoadFailure {
        id: String,
        path: PathBuf,
        source: io::Error,
    },

    #[error(
        "failed to save config file for id {id} using path '{}'. detailed message: {source}", path.display()
    )]
    ConfigSaveFailure {
        id: String,
        path: PathBuf,
        source: io::Error,
    },

    #[error("the configured version of the gateway ({config_version}) is incompatible with the binary version ({binary_version})")]
    LocalVersionCheckFailure {
        binary_version: String,
        config_version: String,
    },

    #[error("could not obtain the information about current gateways on the network: {source}")]
    NetworkGatewaysQueryFailure { source: ValidatorClientError },

    #[error("address {account} has an invalid bech32 prefix. it uses '{actual_prefix}' while '{expected_prefix}' was expected")]
    InvalidBech32AccountPrefix {
        account: AccountId,
        expected_prefix: String,
        actual_prefix: String,
    },

    #[error("this node has insufficient balance to run as zk-nym entry node since it won't be capable of redeeming received credentials. it's account ({account}) has a balance of only {balance}")]
    InsufficientNodeBalance { account: AccountId, balance: Coin },

    #[error("storage failure: {source}")]
    StorageError {
        #[from]
        source: StorageError,
    },

    #[error("Path to network requester configuration file hasn't been specified. Perhaps try to run `setup-network-requester`?")]
    UnspecifiedNetworkRequesterConfig,

    #[error("Path to ip packet router configuration file hasn't been specified. Perhaps try to run `setup-ip-packet-router`?")]
    UnspecifiedIpPacketRouterConfig,

    #[error("Path to authenticator configuration file hasn't been specified. Perhaps try to run `setup-authenticator`?")]
    UnspecifiedAuthenticatorConfig,

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

    #[error("there was an issue with the local authenticator: {source}")]
    AuthenticatorFailure {
        #[from]
        source: AuthenticatorError,
    },

    #[error("failed to startup local network requester")]
    NetworkRequesterStartupFailure,

    #[error("failed to startup local ip packet router")]
    IpPacketRouterStartupFailure,

    #[error("failed to startup local authenticator")]
    AuthenticatorStartupFailure,

    #[error("there are no nym API endpoints available")]
    NoNymApisAvailable,

    #[error("there are no nyxd endpoints available")]
    NoNyxdAvailable,

    #[error("there was an issue attempting to use the validator [nyxd]: {source}")]
    ValidatorFailure {
        #[from]
        source: NyxdError,
    },

    #[error(transparent)]
    ClientRequestFailure {
        #[from]
        source: RequestHandlingError,
    },

    #[error("failed to catch an interrupt: {source}")]
    ShutdownFailure {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("this node hasn't set any valid public addresses to announce. Please modify [host.public_ips] section of your config")]
    NoPublicIps,

    #[error("this node attempted to announce an invalid public address: {address}. Please modify [host.public_ips] section of your config. Alternatively, if you wanted to use it in the local setting, run the node with the '--local' flag.")]
    InvalidPublicIp { address: IpAddr },

    #[error(transparent)]
    NymNodeHttpError(#[from] nym_node_http_api::NymNodeHttpError),

    #[error("there was an issue with wireguard IP network: {source}")]
    IpNetworkError {
        #[from]
        source: ipnetwork::IpNetworkError,
    },

    #[cfg(all(feature = "wireguard", target_os = "linux"))]
    #[error("failed to remove wireguard interface: {0}")]
    WireguardInterfaceError(#[from] defguard_wireguard_rs::error::WireguardInterfaceError),

    #[cfg(all(feature = "wireguard", target_os = "linux"))]
    #[error("wireguard not set")]
    WireguardNotSet,

    #[error("failed to start authenticator: {source}")]
    AuthenticatorStartError {
        source: Box<dyn std::error::Error + Send + Sync>,
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

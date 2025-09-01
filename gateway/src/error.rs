// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub use crate::node::client_handling::websocket::connection_handler::authenticated::RequestHandlingError;
use crate::node::internal_service_providers::authenticator::error::AuthenticatorError;
use crate::node::internal_service_providers::network_requester::error::NetworkRequesterError;
use crate::service_providers::ip_packet_router::error::IpPacketRouterError;
use nym_client_core::error::ClientCoreError;
use nym_gateway_stats_storage::error::StatsStorageError;
use nym_gateway_storage::error::GatewayStorageError;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::{AccountId, Coin};
use nym_validator_client::ValidatorClientError;
use std::net::IpAddr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GatewayError {
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
        source: GatewayStorageError,
    },

    #[error("stats storage failure: {source}")]
    StatsStorageError {
        #[from]
        source: StatsStorageError,
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
    AuthenticatorFailure { source: Box<AuthenticatorError> },

    #[error("failed to startup local {typ}")]
    ServiceProviderStartupFailure { typ: &'static str },

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

    #[error("there was an issue with wireguard IP network: {source}")]
    IpNetworkError {
        #[from]
        source: ipnetwork::IpNetworkError,
    },

    #[error("the current multisig contract is not using 'AbsolutePercentage' threshold!")]
    InvalidMultisigThreshold,

    #[error("failed to remove wireguard interface: {0}")]
    WireguardInterfaceError(#[from] defguard_wireguard_rs::error::WireguardInterfaceError),

    #[error("internal wireguard error {0}")]
    InternalWireguardError(String),

    #[error("failed to start authenticator: {source}")]
    AuthenticatorStartError {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("{0}")]
    CredentialVefiricationError(#[from] nym_credential_verification::Error),
}

impl From<ClientCoreError> for GatewayError {
    fn from(value: ClientCoreError) -> Self {
        // if we ever get a client core error, it must have come from the network requester
        GatewayError::NetworkRequesterFailure {
            source: value.into(),
        }
    }
}

impl From<AuthenticatorError> for GatewayError {
    fn from(error: AuthenticatorError) -> Self {
        GatewayError::AuthenticatorFailure {
            source: Box::new(error),
        }
    }
}

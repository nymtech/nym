// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crypto::asymmetric::identity::Ed25519RecoveryError;
use gateway_client::error::GatewayClientError;
use validator_client::ValidatorClientError;

#[derive(thiserror::Error, Debug)]
pub enum ClientCoreError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Gateway client error: {0}")]
    GatewayClientError(#[from] GatewayClientError),
    #[error("Ed25519 error: {0}")]
    Ed25519RecoveryError(#[from] Ed25519RecoveryError),
    #[error("Validator client error: {0}")]
    ValidatorClientError(#[from] ValidatorClientError),

    #[error("No gateway with id: {0}")]
    NoGatewayWithId(String),
    #[error("No gateways on network")]
    NoGatewaysOnNetwork,
    #[error("List of validator apis is empty")]
    ListOfValidatorApisIsEmpty,
    #[error("Could not load existing gateway configuration: {0}")]
    CouldNotLoadExistingGatewayConfiguration(std::io::Error),
    #[error("The current network topology seem to be insufficient to route any packets through")]
    InsufficientNetworkTopology,
}

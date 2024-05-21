// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::storage::wasm_client_traits::WasmClientStorageError;
use crate::topology::WasmTopologyError;
use nym_client_core::client::base_client::storage::gateways_storage::BadGateway;
use nym_client_core::error::ClientCoreError;
use nym_crypto::asymmetric::identity::Ed25519RecoveryError;
use nym_gateway_client::error::GatewayClientError;
use nym_sphinx::addressing::clients::RecipientFormattingError;
use nym_sphinx::anonymous_replies::requests::InvalidAnonymousSenderTagRepresentation;
use nym_topology::NymTopologyError;
use nym_validator_client::ValidatorClientError;
use thiserror::Error;
use wasm_utils::wasm_error;

#[derive(Debug, Error)]
pub enum WasmCoreError {
    #[error("experienced an issue with internal client components: {source}")]
    BaseClientError {
        #[from]
        source: ClientCoreError,
    },

    #[error("The provided gateway identity is invalid: {source}")]
    InvalidGatewayIdentity { source: Ed25519RecoveryError },

    #[error("Gateway communication failure: {source}")]
    GatewayClientError {
        #[from]
        source: GatewayClientError,
    },

    #[error("failed to query nym api: {source}")]
    NymApiError {
        #[from]
        source: ValidatorClientError,
    },

    #[error("The provided wasm topology was invalid: {source}")]
    WasmTopologyError {
        #[from]
        source: WasmTopologyError,
    },

    #[error("The provided nym topology was invalid: {source}")]
    TopologyError {
        #[from]
        source: NymTopologyError,
    },

    #[error("{raw} is not a valid url: {source}")]
    MalformedUrl {
        raw: String,
        source: url::ParseError,
    },

    #[error("Network topology is currently unavailable")]
    UnavailableNetworkTopology,

    #[error("Mixnode {mixnode_identity} is not present in the current network topology")]
    NonExistentMixnode { mixnode_identity: String },

    #[error("Gateway {gateway_identity} is not present in the current network topology")]
    NonExistentGateway { gateway_identity: String },

    #[error("{raw} is not a valid Nym network recipient: {source}")]
    MalformedRecipient {
        raw: String,
        source: RecipientFormattingError,
    },

    #[error("{raw} is not a valid Nym AnonymousSenderTag: {source}")]
    MalformedSenderTag {
        raw: String,
        source: InvalidAnonymousSenderTagRepresentation,
    },

    #[error(transparent)]
    BaseStorageError {
        #[from]
        source: wasm_storage::error::StorageError,
    },

    #[error(transparent)]
    ClientStorageError {
        #[from]
        source: WasmClientStorageError,
    },

    #[error(transparent)]
    MalformedGateway {
        #[from]
        source: BadGateway,
    },

    #[error("this client has already registered with a gateway: {gateway_id:?}")]
    AlreadyRegistered { gateway_id: String },
}

wasm_error!(WasmCoreError);

// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::identity::Ed25519RecoveryError;
use nym_gateway_requests::registration::handshake::shared_key::SharedKeyConversionError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BadGateway {
    #[error("{typ} is not a valid gateway type")]
    InvalidGatewayType { typ: String },

    #[error("the provided gateway identity {gateway_id} is malformed: {source}")]
    MalformedGatewayIdentity {
        gateway_id: String,

        #[source]
        source: Ed25519RecoveryError,
    },

    #[error("the account owner of gateway {gateway_id} ({raw_owner}) is malformed: {source}")]
    MalformedGatewayOwnerAccountAddress {
        gateway_id: String,

        raw_owner: String,

        #[source]
        source: cosmrs::ErrorReport,
    },

    #[error("the shared keys provided for gateway {gateway_id} are malformed: {source}")]
    MalformedSharedKeys {
        gateway_id: String,

        #[source]
        source: SharedKeyConversionError,
    },

    #[error(
        "the listening address of gateway {gateway_id} ({raw_listener}) is malformed: {source}"
    )]
    MalformedListener {
        gateway_id: String,

        raw_listener: String,

        #[source]
        source: url::ParseError,
    },
}

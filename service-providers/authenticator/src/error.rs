// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::error::ClientCoreError;
use nym_id::NymIdError;

#[derive(thiserror::Error, Debug)]
pub enum AuthenticatorError {
    #[error("client-core error: {0}")]
    ClientCoreError(#[from] ClientCoreError),

    // TODO: add more details here
    #[error("failed to validate the loaded config")]
    ConfigValidationFailure,

    #[error("the entity wrapping the network requester has disconnected")]
    DisconnectedParent,

    #[error("received empty packet")]
    EmptyPacket,

    #[error("failed local version check, client and config mismatch")]
    FailedLocalVersionCheck,

    #[error("failed to connect to mixnet: {source}")]
    FailedToConnectToMixnet { source: nym_sdk::Error },

    #[error("failed to deserialize tagged packet: {source}")]
    FailedToDeserializeTaggedPacket { source: bincode::Error },

    #[error("failed to load configuration file: {0}")]
    FailedToLoadConfig(String),

    #[error("failed to send packet to mixnet: {source}")]
    FailedToSendPacketToMixnet { source: nym_sdk::Error },

    #[error("failed to serialize response packet: {source}")]
    FailedToSerializeResponsePacket { source: Box<bincode::ErrorKind> },

    #[error("failed to setup mixnet client: {source}")]
    FailedToSetupMixnetClient { source: nym_sdk::Error },

    #[error("internal error: {0}")]
    InternalError(String),

    #[error("received packet has an invalid version: {0}")]
    InvalidPacketVersion(u8),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("mac does not verify")]
    MacVerificationFailure,

    #[error("no more space in the network")]
    NoFreeIp,

    #[error(transparent)]
    NymIdError(#[from] NymIdError),

    #[error("registration is not in progress for the given key")]
    RegistrationNotInProgress,
}

pub type Result<T> = std::result::Result<T, AuthenticatorError>;

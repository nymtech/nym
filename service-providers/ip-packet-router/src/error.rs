// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::net::SocketAddr;

pub use nym_client_core::error::ClientCoreError;
use nym_exit_policy::PolicyError;
use nym_id::NymIdError;
use nym_ip_packet_requests::sign::SignatureError;
use nym_service_provider_requests_common::{ProtocolError, ServiceProviderType};

#[derive(thiserror::Error, Debug)]
pub enum IpPacketRouterError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("client-core error: {0}")]
    ClientCoreError(#[from] ClientCoreError),

    #[cfg(target_os = "linux")]
    #[error("tun device error: {0}")]
    TunDeviceError(#[from] nym_tun::tun_device::TunDeviceError),

    #[error("failed to load configuration file: {0}")]
    FailedToLoadConfig(String),

    // TODO: add more details here
    #[error("failed to validate the loaded config")]
    ConfigValidationFailure,

    #[error("failed to setup mixnet client: {source}")]
    FailedToSetupMixnetClient { source: Box<nym_sdk::Error> },

    #[error("failed to connect to mixnet: {source}")]
    FailedToConnectToMixnet { source: Box<nym_sdk::Error> },

    #[error("the entity wrapping the ip packet router has disconnected")]
    DisconnectedParent,

    #[error("received packet has an invalid version: {0}")]
    InvalidPacketVersion(u8),

    #[error("failed to serialize response packet: {source}")]
    FailedToSerializeResponsePacket { source: Box<bincode::ErrorKind> },

    #[error("failed to deserialize tagged packet: {source}")]
    FailedToDeserializeTaggedPacket { source: bincode::Error },

    #[error("failed to parse incoming packet: {source}")]
    PacketParseFailed { source: etherparse::ReadError },

    #[error("parsed packet is missing IP header")]
    PacketMissingIpHeader,

    #[error("parsed packet is missing transport header")]
    PacketMissingTransportHeader,

    #[error("failed to write packet to tun")]
    FailedToWritePacketToTun,

    #[error("failed to send packet to mixnet: {source}")]
    FailedToSendPacketToMixnet { source: Box<nym_sdk::Error> },

    #[error("failed to encode mixnet message: {source}")]
    FailedToEncodeMixnetMessage {
        source: nym_ip_packet_requests::codec::Error,
    },

    #[error("the provided socket address, '{addr}' is not covered by the exit policy!")]
    AddressNotCoveredByExitPolicy { addr: SocketAddr },

    #[error("failed to apply the exit policy: {source}")]
    ExitPolicyFailure {
        #[from]
        source: PolicyError,
    },

    #[error("the url provided for the upstream exit policy source is malformed: {source}")]
    MalformedExitPolicyUpstreamUrl {
        #[source]
        source: reqwest::Error,
    },

    #[error("can't setup an exit policy without any upstream urls")]
    NoUpstreamExitPolicy,

    #[error("no recipient in response packet")]
    NoRecipientInResponse,

    #[error("failed to update client activity")]
    FailedToUpdateClientActivity,

    #[error(transparent)]
    ConfigUpgradeFailure(#[from] nym_client_core::config::ConfigUpgradeFailure),

    #[error(transparent)]
    NymIdError(#[from] NymIdError),

    #[error("received empty packet")]
    EmptyPacket,

    #[error("failed to verify request: {source}")]
    FailedToVerifyRequest { source: SignatureError },

    #[error("invalid reply-to address in the response")]
    InvalidReplyTo,

    #[error("missing sender tag in the request")]
    MissingSenderTag,

    #[error("unsupported response: {0}")]
    UnsupportedResponse(String),

    #[error("invalid service provider type: {0}")]
    InvalidServiceProviderType(ServiceProviderType),

    #[error("failed to deserialize protocol: {source}")]
    FailedToDeserializeProtocol { source: ProtocolError },
}

pub type Result<T> = std::result::Result<T, IpPacketRouterError>;

// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

mod v6;
mod v7;
mod v8;

use nym_bin_common::build_information::BinaryBuildInformationOwned;
use nym_ip_packet_requests::{
    v6::response::IpPacketResponse as IpPacketResponseV6,
    v7::response::IpPacketResponse as IpPacketResponseV7,
    v8::response::IpPacketResponse as IpPacketResponseV8, IpPair,
};

use super::ClientVersion;
use crate::service_providers::ip_packet_router::clients::ConnectedClientId;
use crate::service_providers::ip_packet_router::error::IpPacketRouterError;

pub(crate) struct VersionedResponse {
    pub(crate) version: ClientVersion,
    pub(crate) reply_to: ConnectedClientId,
    pub(crate) response: Response,
}

#[derive(Debug, Clone)]
pub(crate) enum Response {
    StaticConnect {
        request_id: u64,
        reply: StaticConnectResponse,
    },
    DynamicConnect {
        request_id: u64,
        reply: DynamicConnectResponse,
    },
    // Disconnect is not yet implemented
    #[allow(unused)]
    Disconnect {
        request_id: u64,
        reply: DisconnectResponse,
    },
    Pong {
        request_id: u64,
    },
    Health {
        request_id: u64,
        reply: Box<HealthResponse>,
    },
    Info {
        request_id: u64,
        reply: InfoResponse,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum StaticConnectResponse {
    Success,
    Failure(StaticConnectFailureReason),
}

#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum StaticConnectFailureReason {
    #[error("requested ip address is already in use")]
    RequestedIpAlreadyInUse,

    #[error("client already connected")]
    ClientAlreadyConnected,

    #[allow(unused)]
    #[error("request timestamp is out of date")]
    OutOfDateTimestamp,

    #[allow(unused)]
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone)]
pub(crate) enum DynamicConnectResponse {
    Success(DynamicConnectSuccess),
    Failure(DynamicConnectFailureReason),
}

#[derive(Debug, Clone)]
pub(crate) struct DynamicConnectSuccess {
    pub(crate) ips: IpPair,
}

#[derive(Clone, Debug, thiserror::Error)]
pub(crate) enum DynamicConnectFailureReason {
    #[error("no available ip address")]
    NoAvailableIp,

    #[allow(unused)]
    #[error("{0}")]
    Other(String),
}

// Disconnect is not yet implemented
#[allow(unused)]
#[derive(Debug, Clone)]
pub(crate) enum DisconnectResponse {
    Success,
    Failure(DisconnectFailureReason),
}

// Disconnect is not yet implemented
#[allow(unused)]
#[derive(Debug, Clone, thiserror::Error)]
pub(crate) enum DisconnectFailureReason {
    #[error("requested client is not currently connected")]
    ClientNotConnected,

    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone)]
pub(crate) struct HealthResponse {
    pub(crate) build_info: BinaryBuildInformationOwned,
    pub(crate) routable: Option<bool>,
}

impl VersionedResponse {
    pub(crate) fn try_into_bytes(self) -> Result<Vec<u8>, IpPacketRouterError> {
        match self.version {
            ClientVersion::V6 => IpPacketResponseV6::try_from(self)?.to_bytes(),
            ClientVersion::V7 => IpPacketResponseV7::try_from(self)?.to_bytes(),
            ClientVersion::V8 => IpPacketResponseV8::try_from(self)?.to_bytes(),
        }
        .map_err(|err| IpPacketRouterError::FailedToSerializeResponsePacket { source: err })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct InfoResponse {
    pub(crate) reply: InfoResponseReply,
    pub(crate) level: InfoLevel,
}

#[derive(Clone, Debug, thiserror::Error)]
pub(crate) enum InfoResponseReply {
    #[allow(unused)]
    #[error("{msg}")]
    Generic { msg: String },

    #[allow(unused)]
    #[error(
        "version mismatch: response is v{request_version} and response is v{response_version}"
    )]
    VersionMismatch {
        request_version: u8,
        response_version: u8,
    },

    #[error("destination failed exit policy filter check: {dst}")]
    ExitPolicyFilterCheckFailed { dst: String },
}

#[derive(Clone, Debug)]
pub(crate) enum InfoLevel {
    #[allow(unused)]
    Info,
    Warn,
    #[allow(unused)]
    Error,
}

impl From<StaticConnectFailureReason> for StaticConnectResponse {
    fn from(failure: StaticConnectFailureReason) -> Self {
        StaticConnectResponse::Failure(failure)
    }
}

impl From<DynamicConnectSuccess> for DynamicConnectResponse {
    fn from(success: DynamicConnectSuccess) -> Self {
        DynamicConnectResponse::Success(success)
    }
}

impl From<DynamicConnectFailureReason> for DynamicConnectResponse {
    fn from(failure: DynamicConnectFailureReason) -> Self {
        DynamicConnectResponse::Failure(failure)
    }
}

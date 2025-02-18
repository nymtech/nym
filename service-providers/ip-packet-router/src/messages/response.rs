// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_bin_common::build_information::BinaryBuildInformationOwned;
use nym_ip_packet_requests::{v7, v8, IpPair};

use crate::{
    clients::ConnectedClientId,
    error::{IpPacketRouterError, Result},
};

use super::ClientVersion;

pub(crate) struct VersionedResponse {
    pub(crate) version: ClientVersion,
    pub(crate) reply_to: ConnectedClientId,
    pub(crate) response: Response,
}

impl VersionedResponse {
    pub(crate) fn try_into_bytes(self) -> Result<Vec<u8>> {
        match self.version {
            ClientVersion::V7 => v7::response::IpPacketResponse::try_from(self)?.to_bytes(),
            ClientVersion::V8 => v8::response::IpPacketResponse::from(self).to_bytes(),
        }
        .map_err(|err| IpPacketRouterError::FailedToSerializeResponsePacket { source: err })
    }
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

impl From<StaticConnectFailureReason> for StaticConnectResponse {
    fn from(failure: StaticConnectFailureReason) -> Self {
        StaticConnectResponse::Failure(failure)
    }
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

#[derive(Debug, Clone)]
pub(crate) struct DynamicConnectSuccess {
    pub(crate) ips: IpPair,
}

#[derive(Clone, Debug, thiserror::Error)]
pub(crate) enum DynamicConnectFailureReason {
    #[error("client already connected")]
    ClientAlreadyConnected,

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

impl TryFrom<VersionedResponse> for v7::response::IpPacketResponse {
    type Error = IpPacketRouterError;

    fn try_from(response: VersionedResponse) -> std::result::Result<Self, Self::Error> {
        Ok(match response.response {
            Response::StaticConnect { request_id, reply } => v7::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v7::response::IpPacketResponseData::StaticConnect(
                    v7::response::StaticConnectResponse {
                        request_id,
                        reply_to: response.reply_to.into_nym_address()?,
                        reply: match reply {
                            StaticConnectResponse::Success => {
                                v7::response::StaticConnectResponseReply::Success
                            }
                            StaticConnectResponse::Failure(err) => {
                                v7::response::StaticConnectResponseReply::Failure(err.into())
                            }
                        },
                    },
                ),
            },
            Response::DynamicConnect { request_id, reply } => v7::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v7::response::IpPacketResponseData::DynamicConnect(
                    v7::response::DynamicConnectResponse {
                        request_id,
                        reply_to: response.reply_to.into_nym_address()?,
                        reply: match reply {
                            DynamicConnectResponse::Success(DynamicConnectSuccess { ips }) => {
                                v7::response::DynamicConnectResponseReply::Success(
                                    v7::response::DynamicConnectSuccess { ips },
                                )
                            }
                            DynamicConnectResponse::Failure(err) => {
                                v7::response::DynamicConnectResponseReply::Failure(err.into())
                            }
                        },
                    },
                ),
            },
            Response::Disconnect { request_id, reply } => v7::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v7::response::IpPacketResponseData::Disconnect(
                    v7::response::DisconnectResponse {
                        request_id,
                        reply_to: response.reply_to.into_nym_address()?,
                        reply: match reply {
                            DisconnectResponse::Success => {
                                v7::response::DisconnectResponseReply::Success
                            }
                            DisconnectResponse::Failure(err) => {
                                v7::response::DisconnectResponseReply::Failure(err.into())
                            }
                        },
                    },
                ),
            },
            Response::Pong { request_id } => v7::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v7::response::IpPacketResponseData::Pong(v7::response::PongResponse {
                    request_id,
                    reply_to: response.reply_to.into_nym_address()?,
                }),
            },
            Response::Health { request_id, reply } => v7::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v7::response::IpPacketResponseData::Health(v7::response::HealthResponse {
                    request_id,
                    reply_to: response.reply_to.into_nym_address()?,
                    reply: v7::response::HealthResponseReply {
                        build_info: reply.build_info,
                        routable: reply.routable,
                    },
                }),
            },
            Response::Info { request_id, reply } => v7::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v7::response::IpPacketResponseData::Info(v7::response::InfoResponse {
                    request_id,
                    reply_to: response.reply_to.into_nym_address()?,
                    reply: reply.reply.into(),
                    level: reply.level.into(),
                }),
            },
        })
    }
}

impl From<VersionedResponse> for v8::response::IpPacketResponse {
    fn from(response: VersionedResponse) -> Self {
        match response.response {
            Response::StaticConnect { request_id, reply } => v8::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v8::response::IpPacketResponseData::StaticConnect(
                    v8::response::StaticConnectResponse {
                        request_id,
                        reply: match reply {
                            StaticConnectResponse::Success => {
                                v8::response::StaticConnectResponseReply::Success
                            }
                            StaticConnectResponse::Failure(err) => {
                                v8::response::StaticConnectResponseReply::Failure(err.into())
                            }
                        },
                    },
                ),
            },
            Response::DynamicConnect { request_id, reply } => v8::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v8::response::IpPacketResponseData::DynamicConnect(
                    v8::response::DynamicConnectResponse {
                        request_id,
                        reply: match reply {
                            DynamicConnectResponse::Success(DynamicConnectSuccess { ips }) => {
                                v8::response::DynamicConnectResponseReply::Success(
                                    v8::response::DynamicConnectSuccess { ips },
                                )
                            }
                            DynamicConnectResponse::Failure(err) => {
                                v8::response::DynamicConnectResponseReply::Failure(err.into())
                            }
                        },
                    },
                ),
            },
            Response::Disconnect { request_id, reply } => v8::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v8::response::IpPacketResponseData::Disconnect(
                    v8::response::DisconnectResponse {
                        request_id,
                        reply: match reply {
                            DisconnectResponse::Success => {
                                v8::response::DisconnectResponseReply::Success
                            }
                            DisconnectResponse::Failure(err) => {
                                v8::response::DisconnectResponseReply::Failure(err.into())
                            }
                        },
                    },
                ),
            },
            Response::Pong { request_id } => v8::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v8::response::IpPacketResponseData::Pong(v8::response::PongResponse {
                    request_id,
                }),
            },
            Response::Health { request_id, reply } => v8::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v8::response::IpPacketResponseData::Health(v8::response::HealthResponse {
                    request_id,
                    reply: v8::response::HealthResponseReply {
                        build_info: reply.build_info,
                        routable: reply.routable,
                    },
                }),
            },
            Response::Info { request_id, reply } => v8::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v8::response::IpPacketResponseData::Info(v8::response::InfoResponse {
                    request_id,
                    reply: reply.reply.into(),
                    level: reply.level.into(),
                }),
            },
        }
    }
}

impl From<StaticConnectFailureReason> for v7::response::StaticConnectFailureReason {
    fn from(reason: StaticConnectFailureReason) -> Self {
        match reason {
            StaticConnectFailureReason::RequestedIpAlreadyInUse => {
                v7::response::StaticConnectFailureReason::RequestedIpAlreadyInUse
            }
            StaticConnectFailureReason::ClientAlreadyConnected => {
                v7::response::StaticConnectFailureReason::RequestedNymAddressAlreadyInUse
            }
            StaticConnectFailureReason::OutOfDateTimestamp => {
                v7::response::StaticConnectFailureReason::OutOfDateTimestamp
            }
            StaticConnectFailureReason::Other(err) => {
                v7::response::StaticConnectFailureReason::Other(err)
            }
        }
    }
}

impl From<StaticConnectFailureReason> for v8::response::StaticConnectFailureReason {
    fn from(reason: StaticConnectFailureReason) -> Self {
        match reason {
            StaticConnectFailureReason::RequestedIpAlreadyInUse => {
                v8::response::StaticConnectFailureReason::RequestedIpAlreadyInUse
            }
            StaticConnectFailureReason::ClientAlreadyConnected => {
                v8::response::StaticConnectFailureReason::ClientAlreadyConnected
            }
            StaticConnectFailureReason::OutOfDateTimestamp => {
                v8::response::StaticConnectFailureReason::OutOfDateTimestamp
            }
            StaticConnectFailureReason::Other(err) => {
                v8::response::StaticConnectFailureReason::Other(err)
            }
        }
    }
}

impl From<DynamicConnectFailureReason> for v7::response::DynamicConnectFailureReason {
    fn from(reason: DynamicConnectFailureReason) -> Self {
        match reason {
            DynamicConnectFailureReason::ClientAlreadyConnected => {
                v7::response::DynamicConnectFailureReason::RequestedNymAddressAlreadyInUse
            }
            DynamicConnectFailureReason::NoAvailableIp => {
                v7::response::DynamicConnectFailureReason::NoAvailableIp
            }
            DynamicConnectFailureReason::Other(err) => {
                v7::response::DynamicConnectFailureReason::Other(err)
            }
        }
    }
}

impl From<DynamicConnectFailureReason> for v8::response::DynamicConnectFailureReason {
    fn from(reason: DynamicConnectFailureReason) -> Self {
        match reason {
            DynamicConnectFailureReason::ClientAlreadyConnected => {
                v8::response::DynamicConnectFailureReason::ClientAlreadyConnected
            }
            DynamicConnectFailureReason::NoAvailableIp => {
                v8::response::DynamicConnectFailureReason::NoAvailableIp
            }
            DynamicConnectFailureReason::Other(err) => {
                v8::response::DynamicConnectFailureReason::Other(err)
            }
        }
    }
}

impl From<DisconnectFailureReason> for v7::response::DisconnectFailureReason {
    fn from(reason: DisconnectFailureReason) -> Self {
        match reason {
            DisconnectFailureReason::ClientNotConnected => {
                v7::response::DisconnectFailureReason::RequestedNymAddressNotConnected
            }
            DisconnectFailureReason::Other(err) => {
                v7::response::DisconnectFailureReason::Other(err)
            }
        }
    }
}

impl From<DisconnectFailureReason> for v8::response::DisconnectFailureReason {
    fn from(reason: DisconnectFailureReason) -> Self {
        match reason {
            DisconnectFailureReason::ClientNotConnected => {
                v8::response::DisconnectFailureReason::ClientNotConnected
            }
            DisconnectFailureReason::Other(err) => {
                v8::response::DisconnectFailureReason::Other(err)
            }
        }
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

impl From<InfoResponseReply> for v7::response::InfoResponseReply {
    fn from(reply: InfoResponseReply) -> Self {
        match reply {
            InfoResponseReply::Generic { msg } => v7::response::InfoResponseReply::Generic { msg },
            InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            } => v7::response::InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            },
            InfoResponseReply::ExitPolicyFilterCheckFailed { dst } => {
                v7::response::InfoResponseReply::ExitPolicyFilterCheckFailed { dst }
            }
        }
    }
}

impl From<InfoResponseReply> for v8::response::InfoResponseReply {
    fn from(reply: InfoResponseReply) -> Self {
        match reply {
            InfoResponseReply::Generic { msg } => v8::response::InfoResponseReply::Generic { msg },
            InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            } => v8::response::InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            },
            InfoResponseReply::ExitPolicyFilterCheckFailed { dst } => {
                v8::response::InfoResponseReply::ExitPolicyFilterCheckFailed { dst }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum InfoLevel {
    #[allow(unused)]
    Info,
    Warn,
    #[allow(unused)]
    Error,
}

impl From<InfoLevel> for v7::response::InfoLevel {
    fn from(level: InfoLevel) -> Self {
        match level {
            InfoLevel::Info => v7::response::InfoLevel::Info,
            InfoLevel::Warn => v7::response::InfoLevel::Warn,
            InfoLevel::Error => v7::response::InfoLevel::Error,
        }
    }
}

impl From<InfoLevel> for v8::response::InfoLevel {
    fn from(level: InfoLevel) -> Self {
        match level {
            InfoLevel::Info => v8::response::InfoLevel::Info,
            InfoLevel::Warn => v8::response::InfoLevel::Warn,
            InfoLevel::Error => v8::response::InfoLevel::Error,
        }
    }
}

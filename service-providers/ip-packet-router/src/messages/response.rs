// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_bin_common::build_information::BinaryBuildInformationOwned;
use nym_ip_packet_requests::{
    v6::{
        self,
        response::{
            DisconnectFailureReason as DisconnectFailureReasonV6,
            DisconnectResponse as DisconnectResponseV6,
            DisconnectResponseReply as DisconnectResponseReplyV6,
            DynamicConnectFailureReason as DynamicConnectFailureReasonV6,
            DynamicConnectResponse as DynamicConnectResponseV6,
            DynamicConnectResponseReply as DynamicConnectResponseReplyV6,
            DynamicConnectSuccess as DynamicConnectSuccessV6, HealthResponse as HealthResponseV6,
            HealthResponseReply as HealthResponseReplyV6, InfoResponse as InfoResponseV6,
            IpPacketResponse as IpPacketResponseV6, IpPacketResponseData as IpPacketResponseDataV6,
            PongResponse as PongResponseV6,
            StaticConnectFailureReason as StaticConnectFailureReasonV6,
            StaticConnectResponse as StaticConnectResponseV6,
            StaticConnectResponseReply as StaticConnectResponseReplyV6,
            InfoLevel as InfoLevelV6,
        },
    },
    v7::{
        self,
        response::{
            DisconnectFailureReason as DisconnectFailureReasonV7,
            DisconnectResponse as DisconnectResponseV7,
            DisconnectResponseReply as DisconnectResponseReplyV7,
            DynamicConnectFailureReason as DynamicConnectFailureReasonV7,
            DynamicConnectResponse as DynamicConnectResponseV7,
            DynamicConnectResponseReply as DynamicConnectResponseReplyV7,
            DynamicConnectSuccess as DynamicConnectSuccessV7, HealthResponse as HealthResponseV7,
            HealthResponseReply as HealthResponseReplyV7, InfoResponse as InfoResponseV7,
            IpPacketResponse as IpPacketResponseV7, IpPacketResponseData as IpPacketResponseDataV7,
            PongResponse as PongResponseV7,
            StaticConnectFailureReason as StaticConnectFailureReasonV7,
            StaticConnectResponse as StaticConnectResponseV7,
            StaticConnectResponseReply as StaticConnectResponseReplyV7,
            InfoLevel as InfoLevelV7,
        },
    },
    v8::{
        self,
        response::{
            ControlResponse as ControlResponseV8,
            DisconnectFailureReason as DisconnectFailureReasonV8,
            DisconnectResponse as DisconnectResponseV8,
            DisconnectResponseReply as DisconnectResponseReplyV8,
            DynamicConnectFailureReason as DynamicConnectFailureReasonV8,
            DynamicConnectResponse as DynamicConnectResponseV8,
            DynamicConnectResponseReply as DynamicConnectResponseReplyV8,
            DynamicConnectSuccess as DynamicConnectSuccessV8, HealthResponse as HealthResponseV8,
            HealthResponseReply as HealthResponseReplyV8, InfoResponse as InfoResponseV8,
            IpPacketResponse as IpPacketResponseV8, IpPacketResponseData as IpPacketResponseDataV8,
            PongResponse as PongResponseV8,
            StaticConnectFailureReason as StaticConnectFailureReasonV8,
            StaticConnectResponse as StaticConnectResponseV8,
            StaticConnectResponseReply as StaticConnectResponseReplyV8,
            InfoLevel as InfoLevelV8,
        },
    },
    IpPair,
};

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
            ClientVersion::V6 => IpPacketResponseV6::try_from(self)?.to_bytes(),
            ClientVersion::V7 => IpPacketResponseV7::try_from(self)?.to_bytes(),
            ClientVersion::V8 => IpPacketResponseV8::from(self).to_bytes(),
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

impl TryFrom<VersionedResponse> for IpPacketResponseV6 {
    type Error = IpPacketRouterError;

    fn try_from(response: VersionedResponse) -> std::result::Result<Self, Self::Error> {
        Ok(match response.response {
            Response::StaticConnect { request_id, reply } => IpPacketResponseV6 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV6::StaticConnect(StaticConnectResponseV6 {
                    request_id,
                    reply_to: response.reply_to.into_nym_address()?,
                    reply: match reply {
                        StaticConnectResponse::Success => StaticConnectResponseReplyV6::Success,
                        StaticConnectResponse::Failure(err) => {
                            StaticConnectResponseReplyV6::Failure(err.into())
                        }
                    },
                }),
            },
            Response::DynamicConnect { request_id, reply } => IpPacketResponseV6 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV6::DynamicConnect(DynamicConnectResponseV6 {
                    request_id,
                    reply_to: response.reply_to.into_nym_address()?,
                    reply: match reply {
                        DynamicConnectResponse::Success(DynamicConnectSuccess { ips }) => {
                            DynamicConnectResponseReplyV6::Success(DynamicConnectSuccessV6 { ips })
                        }
                        DynamicConnectResponse::Failure(err) => {
                            DynamicConnectResponseReplyV6::Failure(err.into())
                        }
                    },
                }),
            },
            Response::Disconnect { request_id, reply } => IpPacketResponseV6 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV6::Disconnect(DisconnectResponseV6 {
                    request_id,
                    reply_to: response.reply_to.into_nym_address()?,
                    reply: match reply {
                        DisconnectResponse::Success => DisconnectResponseReplyV6::Success,
                        DisconnectResponse::Failure(err) => {
                            DisconnectResponseReplyV6::Failure(err.into())
                        }
                    },
                }),
            },
            Response::Pong { request_id } => IpPacketResponseV6 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV6::Pong(PongResponseV6 {
                    request_id,
                    reply_to: response.reply_to.into_nym_address()?,
                }),
            },
            Response::Health { request_id, reply } => IpPacketResponseV6 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV6::Health(HealthResponseV6 {
                    request_id,
                    reply_to: response.reply_to.into_nym_address()?,
                    reply: HealthResponseReplyV6 {
                        build_info: reply.build_info,
                        routable: reply.routable,
                    },
                }),
            },
            Response::Info { request_id, reply } => IpPacketResponseV6 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV6::Info(InfoResponseV6 {
                    request_id,
                    reply_to: response.reply_to.into_nym_address()?,
                    reply: reply.reply.into(),
                    level: reply.level.into(),
                }),
            },
        })
    }
}

impl TryFrom<VersionedResponse> for IpPacketResponseV7 {
    type Error = IpPacketRouterError;

    fn try_from(response: VersionedResponse) -> std::result::Result<Self, Self::Error> {
        Ok(match response.response {
            Response::StaticConnect { request_id, reply } => IpPacketResponseV7 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV7::StaticConnect(StaticConnectResponseV7 {
                    request_id,
                    reply_to: response.reply_to.into_nym_address()?,
                    reply: match reply {
                        StaticConnectResponse::Success => StaticConnectResponseReplyV7::Success,
                        StaticConnectResponse::Failure(err) => {
                            StaticConnectResponseReplyV7::Failure(err.into())
                        }
                    },
                }),
            },
            Response::DynamicConnect { request_id, reply } => IpPacketResponseV7 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV7::DynamicConnect(DynamicConnectResponseV7 {
                    request_id,
                    reply_to: response.reply_to.into_nym_address()?,
                    reply: match reply {
                        DynamicConnectResponse::Success(DynamicConnectSuccess { ips }) => {
                            DynamicConnectResponseReplyV7::Success(DynamicConnectSuccessV7 { ips })
                        }
                        DynamicConnectResponse::Failure(err) => {
                            DynamicConnectResponseReplyV7::Failure(err.into())
                        }
                    },
                }),
            },
            Response::Disconnect { request_id, reply } => IpPacketResponseV7 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV7::Disconnect(DisconnectResponseV7 {
                    request_id,
                    reply_to: response.reply_to.into_nym_address()?,
                    reply: match reply {
                        DisconnectResponse::Success => DisconnectResponseReplyV7::Success,
                        DisconnectResponse::Failure(err) => {
                            DisconnectResponseReplyV7::Failure(err.into())
                        }
                    },
                }),
            },
            Response::Pong { request_id } => IpPacketResponseV7 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV7::Pong(PongResponseV7 {
                    request_id,
                    reply_to: response.reply_to.into_nym_address()?,
                }),
            },
            Response::Health { request_id, reply } => IpPacketResponseV7 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV7::Health(HealthResponseV7 {
                    request_id,
                    reply_to: response.reply_to.into_nym_address()?,
                    reply: HealthResponseReplyV7 {
                        build_info: reply.build_info,
                        routable: reply.routable,
                    },
                }),
            },
            Response::Info { request_id, reply } => IpPacketResponseV7 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV7::Info(InfoResponseV7 {
                    request_id,
                    reply_to: response.reply_to.into_nym_address()?,
                    reply: reply.reply.into(),
                    level: reply.level.into(),
                }),
            },
        })
    }
}

impl From<VersionedResponse> for IpPacketResponseV8 {
    fn from(response: VersionedResponse) -> Self {
        match response.response {
            Response::StaticConnect { request_id, reply } => IpPacketResponseV8 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV8::Control(ControlResponseV8::StaticConnect(
                    StaticConnectResponseV8 {
                        request_id,
                        reply: match reply {
                            StaticConnectResponse::Success => StaticConnectResponseReplyV8::Success,
                            StaticConnectResponse::Failure(err) => {
                                StaticConnectResponseReplyV8::Failure(err.into())
                            }
                        },
                    },
                )),
            },
            Response::DynamicConnect { request_id, reply } => IpPacketResponseV8 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV8::Control(ControlResponseV8::DynamicConnect(
                    DynamicConnectResponseV8 {
                        request_id,
                        reply: match reply {
                            DynamicConnectResponse::Success(DynamicConnectSuccess { ips }) => {
                                DynamicConnectResponseReplyV8::Success(DynamicConnectSuccessV8 {
                                    ips,
                                })
                            }
                            DynamicConnectResponse::Failure(err) => {
                                DynamicConnectResponseReplyV8::Failure(err.into())
                            }
                        },
                    },
                )),
            },
            Response::Disconnect { request_id, reply } => IpPacketResponseV8 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV8::Control(ControlResponseV8::Disconnect(
                    DisconnectResponseV8 {
                        request_id,
                        reply: match reply {
                            DisconnectResponse::Success => DisconnectResponseReplyV8::Success,
                            DisconnectResponse::Failure(err) => {
                                DisconnectResponseReplyV8::Failure(err.into())
                            }
                        },
                    },
                )),
            },
            Response::Pong { request_id } => IpPacketResponseV8 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV8::Control(ControlResponseV8::Pong(PongResponseV8 {
                    request_id,
                })),
            },
            Response::Health { request_id, reply } => IpPacketResponseV8 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV8::Control(ControlResponseV8::Health(
                    HealthResponseV8 {
                        request_id,
                        reply: HealthResponseReplyV8 {
                            build_info: reply.build_info,
                            routable: reply.routable,
                        },
                    },
                )),
            },
            Response::Info { request_id, reply } => IpPacketResponseV8 {
                version: response.version.into_u8(),
                data: IpPacketResponseDataV8::Control(ControlResponseV8::Info(InfoResponseV8 {
                    request_id,
                    reply: reply.reply.into(),
                    level: reply.level.into(),
                })),
            },
        }
    }
}

impl From<StaticConnectFailureReason> for StaticConnectFailureReasonV6 {
    fn from(reason: StaticConnectFailureReason) -> Self {
        match reason {
            StaticConnectFailureReason::RequestedIpAlreadyInUse => {
                StaticConnectFailureReasonV6::RequestedIpAlreadyInUse
            }
            StaticConnectFailureReason::ClientAlreadyConnected => {
                StaticConnectFailureReasonV6::RequestedNymAddressAlreadyInUse
            }
            StaticConnectFailureReason::OutOfDateTimestamp => {
                StaticConnectFailureReasonV6::Other("unexpected timestamp".to_string())
            }
            StaticConnectFailureReason::Other(err) => StaticConnectFailureReasonV6::Other(err),
        }
    }
}

impl From<StaticConnectFailureReason> for StaticConnectFailureReasonV7 {
    fn from(reason: StaticConnectFailureReason) -> Self {
        match reason {
            StaticConnectFailureReason::RequestedIpAlreadyInUse => {
                StaticConnectFailureReasonV7::RequestedIpAlreadyInUse
            }
            StaticConnectFailureReason::ClientAlreadyConnected => {
                StaticConnectFailureReasonV7::RequestedNymAddressAlreadyInUse
            }
            StaticConnectFailureReason::OutOfDateTimestamp => {
                StaticConnectFailureReasonV7::OutOfDateTimestamp
            }
            StaticConnectFailureReason::Other(err) => StaticConnectFailureReasonV7::Other(err),
        }
    }
}

impl From<StaticConnectFailureReason> for StaticConnectFailureReasonV8 {
    fn from(reason: StaticConnectFailureReason) -> Self {
        match reason {
            StaticConnectFailureReason::RequestedIpAlreadyInUse => {
                StaticConnectFailureReasonV8::RequestedIpAlreadyInUse
            }
            StaticConnectFailureReason::ClientAlreadyConnected => {
                StaticConnectFailureReasonV8::ClientAlreadyConnected
            }
            StaticConnectFailureReason::OutOfDateTimestamp => {
                StaticConnectFailureReasonV8::OutOfDateTimestamp
            }
            StaticConnectFailureReason::Other(err) => StaticConnectFailureReasonV8::Other(err),
        }
    }
}

impl From<DynamicConnectFailureReason> for DynamicConnectFailureReasonV6 {
    fn from(reason: DynamicConnectFailureReason) -> Self {
        match reason {
            DynamicConnectFailureReason::ClientAlreadyConnected => {
                DynamicConnectFailureReasonV6::RequestedNymAddressAlreadyInUse
            }
            DynamicConnectFailureReason::NoAvailableIp => {
                DynamicConnectFailureReasonV6::NoAvailableIp
            }
            DynamicConnectFailureReason::Other(err) => DynamicConnectFailureReasonV6::Other(err),
        }
    }
}

impl From<DynamicConnectFailureReason> for DynamicConnectFailureReasonV7 {
    fn from(reason: DynamicConnectFailureReason) -> Self {
        match reason {
            DynamicConnectFailureReason::ClientAlreadyConnected => {
                DynamicConnectFailureReasonV7::RequestedNymAddressAlreadyInUse
            }
            DynamicConnectFailureReason::NoAvailableIp => {
                DynamicConnectFailureReasonV7::NoAvailableIp
            }
            DynamicConnectFailureReason::Other(err) => DynamicConnectFailureReasonV7::Other(err),
        }
    }
}

impl From<DynamicConnectFailureReason> for DynamicConnectFailureReasonV8 {
    fn from(reason: DynamicConnectFailureReason) -> Self {
        match reason {
            DynamicConnectFailureReason::ClientAlreadyConnected => {
                DynamicConnectFailureReasonV8::ClientAlreadyConnected
            }
            DynamicConnectFailureReason::NoAvailableIp => {
                DynamicConnectFailureReasonV8::NoAvailableIp
            }
            DynamicConnectFailureReason::Other(err) => DynamicConnectFailureReasonV8::Other(err),
        }
    }
}

impl From<DisconnectFailureReason> for DisconnectFailureReasonV6 {
    fn from(reason: DisconnectFailureReason) -> Self {
        match reason {
            DisconnectFailureReason::ClientNotConnected => {
                DisconnectFailureReasonV6::RequestedNymAddressNotConnected
            }
            DisconnectFailureReason::Other(err) => DisconnectFailureReasonV6::Other(err),
        }
    }
}

impl From<DisconnectFailureReason> for DisconnectFailureReasonV7 {
    fn from(reason: DisconnectFailureReason) -> Self {
        match reason {
            DisconnectFailureReason::ClientNotConnected => {
                DisconnectFailureReasonV7::RequestedNymAddressNotConnected
            }
            DisconnectFailureReason::Other(err) => DisconnectFailureReasonV7::Other(err),
        }
    }
}

impl From<DisconnectFailureReason> for DisconnectFailureReasonV8 {
    fn from(reason: DisconnectFailureReason) -> Self {
        match reason {
            DisconnectFailureReason::ClientNotConnected => {
                DisconnectFailureReasonV8::ClientNotConnected
            }
            DisconnectFailureReason::Other(err) => DisconnectFailureReasonV8::Other(err),
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

impl From<InfoResponseReply> for InfoResponseReplyV6 {
    fn from(reply: InfoResponseReply) -> Self {
        match reply {
            InfoResponseReply::Generic { msg } => InfoResponseReplyV6::Generic { msg },
            InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            } => InfoResponseReplyV6::VersionMismatch {
                request_version,
                response_version,
            },
            InfoResponseReply::ExitPolicyFilterCheckFailed { dst } => {
                InfoResponseReplyV6::ExitPolicyFilterCheckFailed { dst }
            }
        }
    }
}

impl From<InfoResponseReply> for InfoResponseReplyV7 {
    fn from(reply: InfoResponseReply) -> Self {
        match reply {
            InfoResponseReply::Generic { msg } => InfoResponseReplyV7::Generic { msg },
            InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            } => InfoResponseReplyV7::VersionMismatch {
                request_version,
                response_version,
            },
            InfoResponseReply::ExitPolicyFilterCheckFailed { dst } => {
                InfoResponseReplyV7::ExitPolicyFilterCheckFailed { dst }
            }
        }
    }
}

impl From<InfoResponseReply> for InfoResponseReplyV8 {
    fn from(reply: InfoResponseReply) -> Self {
        match reply {
            InfoResponseReply::Generic { msg } => InfoResponseReplyV8::Generic { msg },
            InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            } => InfoResponseReplyV8::VersionMismatch {
                request_version,
                response_version,
            },
            InfoResponseReply::ExitPolicyFilterCheckFailed { dst } => {
                InfoResponseReplyV8::ExitPolicyFilterCheckFailed { dst }
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

impl From<InfoLevel> for InfoLevelV6 {
    fn from(level: InfoLevel) -> Self {
        match level {
            InfoLevel::Info => InfoLevelV6::Info,
            InfoLevel::Warn => InfoLevelV6::Warn,
            InfoLevel::Error => InfoLevelV6::Error,
        }
    }
}

impl From<InfoLevel> for InfoLevelV7 {
    fn from(level: InfoLevel) -> Self {
        match level {
            InfoLevel::Info => InfoLevelV7::Info,
            InfoLevel::Warn => InfoLevelV7::Warn,
            InfoLevel::Error => InfoLevelV7::Error,
        }
    }
}

impl From<InfoLevel> for InfoLevelV8 {
    fn from(level: InfoLevel) -> Self {
        match level {
            InfoLevel::Info => InfoLevelV8::Info,
            InfoLevel::Warn => InfoLevelV8::Warn,
            InfoLevel::Error => InfoLevelV8::Error,
        }
    }
}

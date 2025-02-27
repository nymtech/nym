// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_ip_packet_requests::v7::response::{
    DisconnectFailureReason as DisconnectFailureReasonV7,
    DisconnectResponse as DisconnectResponseV7,
    DisconnectResponseReply as DisconnectResponseReplyV7,
    DynamicConnectFailureReason as DynamicConnectFailureReasonV7,
    DynamicConnectResponse as DynamicConnectResponseV7,
    DynamicConnectResponseReply as DynamicConnectResponseReplyV7,
    DynamicConnectSuccess as DynamicConnectSuccessV7, HealthResponse as HealthResponseV7,
    HealthResponseReply as HealthResponseReplyV7, InfoLevel as InfoLevelV7,
    InfoResponse as InfoResponseV7, InfoResponseReply as InfoResponseReplyV7,
    IpPacketResponse as IpPacketResponseV7, IpPacketResponseData as IpPacketResponseDataV7,
    PongResponse as PongResponseV7, StaticConnectFailureReason as StaticConnectFailureReasonV7,
    StaticConnectResponse as StaticConnectResponseV7,
    StaticConnectResponseReply as StaticConnectResponseReplyV7,
};

use crate::error::IpPacketRouterError;

use super::{
    DisconnectFailureReason, DisconnectResponse, DynamicConnectFailureReason,
    DynamicConnectResponse, DynamicConnectSuccess, HealthResponse, InfoLevel, InfoResponseReply,
    Response, StaticConnectFailureReason, StaticConnectResponse, VersionedResponse,
};

impl TryFrom<VersionedResponse> for IpPacketResponseV7 {
    type Error = IpPacketRouterError;

    fn try_from(response: VersionedResponse) -> std::result::Result<Self, Self::Error> {
        let version = response.version.into_u8();
        let reply_to = response.reply_to.into_nym_address()?;
        let data = match response.response {
            Response::StaticConnect { request_id, reply } => {
                IpPacketResponseDataV7::StaticConnect(StaticConnectResponseV7 {
                    request_id,
                    reply_to,
                    reply: reply.into(),
                })
            }
            Response::DynamicConnect { request_id, reply } => {
                IpPacketResponseDataV7::DynamicConnect(DynamicConnectResponseV7 {
                    request_id,
                    reply_to,
                    reply: reply.into(),
                })
            }
            Response::Disconnect { request_id, reply } => {
                IpPacketResponseDataV7::Disconnect(DisconnectResponseV7 {
                    request_id,
                    reply_to,
                    reply: reply.into(),
                })
            }
            Response::Pong { request_id } => IpPacketResponseDataV7::Pong(PongResponseV7 {
                request_id,
                reply_to,
            }),
            Response::Health { request_id, reply } => {
                IpPacketResponseDataV7::Health(HealthResponseV7 {
                    request_id,
                    reply_to,
                    reply: (*reply).into(),
                })
            }
            Response::Info { request_id, reply } => IpPacketResponseDataV7::Info(InfoResponseV7 {
                request_id,
                reply_to,
                reply: reply.reply.into(),
                level: reply.level.into(),
            }),
        };
        Ok(IpPacketResponseV7 { version, data })
    }
}

impl From<StaticConnectResponse> for StaticConnectResponseReplyV7 {
    fn from(response: StaticConnectResponse) -> Self {
        match response {
            StaticConnectResponse::Success => StaticConnectResponseReplyV7::Success,
            StaticConnectResponse::Failure(err) => {
                StaticConnectResponseReplyV7::Failure(err.into())
            }
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

impl From<DynamicConnectResponse> for DynamicConnectResponseReplyV7 {
    fn from(response: DynamicConnectResponse) -> Self {
        match response {
            DynamicConnectResponse::Success(DynamicConnectSuccess { ips }) => {
                DynamicConnectResponseReplyV7::Success(DynamicConnectSuccessV7 { ips })
            }
            DynamicConnectResponse::Failure(err) => {
                DynamicConnectResponseReplyV7::Failure(err.into())
            }
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

impl From<DisconnectResponse> for DisconnectResponseReplyV7 {
    fn from(response: DisconnectResponse) -> Self {
        match response {
            DisconnectResponse::Success => DisconnectResponseReplyV7::Success,
            DisconnectResponse::Failure(err) => DisconnectResponseReplyV7::Failure(err.into()),
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

impl From<HealthResponse> for HealthResponseReplyV7 {
    fn from(response: HealthResponse) -> Self {
        HealthResponseReplyV7 {
            build_info: response.build_info,
            routable: response.routable,
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

impl From<InfoLevel> for InfoLevelV7 {
    fn from(level: InfoLevel) -> Self {
        match level {
            InfoLevel::Info => InfoLevelV7::Info,
            InfoLevel::Warn => InfoLevelV7::Warn,
            InfoLevel::Error => InfoLevelV7::Error,
        }
    }
}

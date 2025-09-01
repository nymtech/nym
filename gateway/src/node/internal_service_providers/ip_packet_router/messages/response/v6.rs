// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::{
    DisconnectFailureReason, DisconnectResponse, DynamicConnectFailureReason,
    DynamicConnectResponse, DynamicConnectSuccess, HealthResponse, InfoLevel, InfoResponseReply,
    Response, StaticConnectFailureReason, StaticConnectResponse, VersionedResponse,
};
use crate::service_providers::ip_packet_router::error::IpPacketRouterError;
use nym_ip_packet_requests::v6::response::{
    DisconnectFailureReason as DisconnectFailureReasonV6,
    DisconnectResponse as DisconnectResponseV6,
    DisconnectResponseReply as DisconnectResponseReplyV6,
    DynamicConnectFailureReason as DynamicConnectFailureReasonV6,
    DynamicConnectResponse as DynamicConnectResponseV6,
    DynamicConnectResponseReply as DynamicConnectResponseReplyV6,
    DynamicConnectSuccess as DynamicConnectSuccessV6, HealthResponse as HealthResponseV6,
    HealthResponseReply as HealthResponseReplyV6, InfoLevel as InfoLevelV6,
    InfoResponse as InfoResponseV6, InfoResponseReply as InfoResponseReplyV6,
    IpPacketResponse as IpPacketResponseV6, IpPacketResponseData as IpPacketResponseDataV6,
    PongResponse as PongResponseV6, StaticConnectFailureReason as StaticConnectFailureReasonV6,
    StaticConnectResponse as StaticConnectResponseV6,
    StaticConnectResponseReply as StaticConnectResponseReplyV6,
};

impl TryFrom<VersionedResponse> for IpPacketResponseV6 {
    type Error = IpPacketRouterError;

    fn try_from(response: VersionedResponse) -> std::result::Result<Self, Self::Error> {
        let version = response.version.into_u8();
        let reply_to = response.reply_to.into_nym_address()?;
        let data = match response.response {
            Response::StaticConnect { request_id, reply } => {
                IpPacketResponseDataV6::StaticConnect(StaticConnectResponseV6 {
                    request_id,
                    reply_to,
                    reply: reply.into(),
                })
            }
            Response::DynamicConnect { request_id, reply } => {
                IpPacketResponseDataV6::DynamicConnect(DynamicConnectResponseV6 {
                    request_id,
                    reply_to,
                    reply: reply.into(),
                })
            }
            Response::Disconnect { request_id, reply } => {
                IpPacketResponseDataV6::Disconnect(DisconnectResponseV6 {
                    request_id,
                    reply_to,
                    reply: reply.into(),
                })
            }
            Response::Pong { request_id } => IpPacketResponseDataV6::Pong(PongResponseV6 {
                request_id,
                reply_to,
            }),
            Response::Health { request_id, reply } => {
                IpPacketResponseDataV6::Health(HealthResponseV6 {
                    request_id,
                    reply_to,
                    reply: (*reply).into(),
                })
            }
            Response::Info { request_id, reply } => IpPacketResponseDataV6::Info(InfoResponseV6 {
                request_id,
                reply_to,
                reply: reply.reply.into(),
                level: reply.level.into(),
            }),
        };
        Ok(IpPacketResponseV6 { version, data })
    }
}

impl From<StaticConnectResponse> for StaticConnectResponseReplyV6 {
    fn from(response: StaticConnectResponse) -> Self {
        match response {
            StaticConnectResponse::Success => StaticConnectResponseReplyV6::Success,
            StaticConnectResponse::Failure(err) => {
                StaticConnectResponseReplyV6::Failure(err.into())
            }
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

impl From<DynamicConnectResponse> for DynamicConnectResponseReplyV6 {
    fn from(response: DynamicConnectResponse) -> Self {
        match response {
            DynamicConnectResponse::Success(DynamicConnectSuccess { ips }) => {
                DynamicConnectResponseReplyV6::Success(DynamicConnectSuccessV6 { ips })
            }
            DynamicConnectResponse::Failure(err) => {
                DynamicConnectResponseReplyV6::Failure(err.into())
            }
        }
    }
}

impl From<DynamicConnectFailureReason> for DynamicConnectFailureReasonV6 {
    fn from(reason: DynamicConnectFailureReason) -> Self {
        match reason {
            DynamicConnectFailureReason::NoAvailableIp => {
                DynamicConnectFailureReasonV6::NoAvailableIp
            }
            DynamicConnectFailureReason::Other(err) => DynamicConnectFailureReasonV6::Other(err),
        }
    }
}

impl From<DisconnectResponse> for DisconnectResponseReplyV6 {
    fn from(response: DisconnectResponse) -> Self {
        match response {
            DisconnectResponse::Success => DisconnectResponseReplyV6::Success,
            DisconnectResponse::Failure(err) => DisconnectResponseReplyV6::Failure(err.into()),
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

impl From<HealthResponse> for HealthResponseReplyV6 {
    fn from(response: HealthResponse) -> Self {
        HealthResponseReplyV6 {
            build_info: response.build_info,
            routable: response.routable,
        }
    }
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

impl From<InfoLevel> for InfoLevelV6 {
    fn from(level: InfoLevel) -> Self {
        match level {
            InfoLevel::Info => InfoLevelV6::Info,
            InfoLevel::Warn => InfoLevelV6::Warn,
            InfoLevel::Error => InfoLevelV6::Error,
        }
    }
}

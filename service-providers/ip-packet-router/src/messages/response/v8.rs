// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_ip_packet_requests::v8::response::{
    ControlResponse as ControlResponseV8, DisconnectFailureReason as DisconnectFailureReasonV8,
    DisconnectResponse as DisconnectResponseV8,
    DisconnectResponseReply as DisconnectResponseReplyV8,
    DynamicConnectFailureReason as DynamicConnectFailureReasonV8,
    DynamicConnectResponse as DynamicConnectResponseV8,
    DynamicConnectResponseReply as DynamicConnectResponseReplyV8,
    DynamicConnectSuccess as DynamicConnectSuccessV8, HealthResponse as HealthResponseV8,
    HealthResponseReply as HealthResponseReplyV8, InfoLevel as InfoLevelV8,
    InfoResponse as InfoResponseV8, InfoResponseReply as InfoResponseReplyV8,
    IpPacketResponse as IpPacketResponseV8, IpPacketResponseData as IpPacketResponseDataV8,
    PongResponse as PongResponseV8, StaticConnectFailureReason as StaticConnectFailureReasonV8,
    StaticConnectResponse as StaticConnectResponseV8,
    StaticConnectResponseReply as StaticConnectResponseReplyV8,
};

use super::{
    DisconnectFailureReason, DisconnectResponse, DynamicConnectFailureReason,
    DynamicConnectResponse, DynamicConnectSuccess, HealthResponse, InfoLevel, InfoResponseReply,
    Response, StaticConnectFailureReason, StaticConnectResponse, VersionedResponse,
};

impl From<VersionedResponse> for IpPacketResponseV8 {
    fn from(response: VersionedResponse) -> Self {
        let version = response.version.into_u8();
        let data =
            match response.response {
                Response::StaticConnect { request_id, reply } => IpPacketResponseDataV8::Control(
                    Box::new(ControlResponseV8::StaticConnect(StaticConnectResponseV8 {
                        request_id,
                        reply: reply.into(),
                    })),
                ),
                Response::DynamicConnect { request_id, reply } => {
                    IpPacketResponseDataV8::Control(Box::new(ControlResponseV8::DynamicConnect(
                        DynamicConnectResponseV8 {
                            request_id,
                            reply: reply.into(),
                        },
                    )))
                }
                Response::Disconnect { request_id, reply } => IpPacketResponseDataV8::Control(
                    Box::new(ControlResponseV8::Disconnect(DisconnectResponseV8 {
                        request_id,
                        reply: reply.into(),
                    })),
                ),
                Response::Pong { request_id } => IpPacketResponseDataV8::Control(Box::new(
                    ControlResponseV8::Pong(PongResponseV8 { request_id }),
                )),
                Response::Health { request_id, reply } => IpPacketResponseDataV8::Control(
                    Box::new(ControlResponseV8::Health(Box::new(HealthResponseV8 {
                        request_id,
                        reply: (*reply).into(),
                    }))),
                ),
                Response::Info { request_id, reply } => IpPacketResponseDataV8::Control(Box::new(
                    ControlResponseV8::Info(InfoResponseV8 {
                        request_id,
                        reply: reply.reply.into(),
                        level: reply.level.into(),
                    }),
                )),
            };

        IpPacketResponseV8 { version, data }
    }
}

impl From<StaticConnectResponse> for StaticConnectResponseReplyV8 {
    fn from(reply: StaticConnectResponse) -> Self {
        match reply {
            StaticConnectResponse::Success => StaticConnectResponseReplyV8::Success,
            StaticConnectResponse::Failure(err) => {
                StaticConnectResponseReplyV8::Failure(err.into())
            }
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

impl From<DynamicConnectResponse> for DynamicConnectResponseReplyV8 {
    fn from(reply: DynamicConnectResponse) -> Self {
        match reply {
            DynamicConnectResponse::Success(DynamicConnectSuccess { ips }) => {
                DynamicConnectResponseReplyV8::Success(DynamicConnectSuccessV8 { ips })
            }
            DynamicConnectResponse::Failure(err) => {
                DynamicConnectResponseReplyV8::Failure(err.into())
            }
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

impl From<DisconnectResponse> for DisconnectResponseReplyV8 {
    fn from(reply: DisconnectResponse) -> Self {
        match reply {
            DisconnectResponse::Success => DisconnectResponseReplyV8::Success,
            DisconnectResponse::Failure(err) => DisconnectResponseReplyV8::Failure(err.into()),
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

impl From<HealthResponse> for HealthResponseReplyV8 {
    fn from(response: HealthResponse) -> Self {
        HealthResponseReplyV8 {
            build_info: response.build_info,
            routable: response.routable,
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

impl From<InfoLevel> for InfoLevelV8 {
    fn from(level: InfoLevel) -> Self {
        match level {
            InfoLevel::Info => InfoLevelV8::Info,
            InfoLevel::Warn => InfoLevelV8::Warn,
            InfoLevel::Error => InfoLevelV8::Error,
        }
    }
}

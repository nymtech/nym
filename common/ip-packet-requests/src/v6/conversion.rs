// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{v6, v7};

//impl From<v7::response::IpPacketResponse> for v6::response::IpPacketResponse {
//    fn from(response: v7::response::IpPacketResponse) -> Self {
//        v6::response::IpPacketResponse {
//            version: 6,
//            data: response.data.into(),
//        }
//    }
//}
//
//impl From<v7::response::IpPacketResponseData> for v6::response::IpPacketResponseData {
//    fn from(data: v7::response::IpPacketResponseData) -> Self {
//        match data {
//            v7::response::IpPacketResponseData::StaticConnect(r) => {
//                v6::response::IpPacketResponseData::StaticConnect(r.into())
//            }
//            v7::response::IpPacketResponseData::DynamicConnect(r) => {
//                v6::response::IpPacketResponseData::DynamicConnect(r.into())
//            }
//            v7::response::IpPacketResponseData::Disconnect(r) => {
//                v6::response::IpPacketResponseData::Disconnect(r.into())
//            }
//            v7::response::IpPacketResponseData::UnrequestedDisconnect(r) => {
//                v6::response::IpPacketResponseData::UnrequestedDisconnect(r.into())
//            }
//            v7::response::IpPacketResponseData::Data(r) => {
//                v6::response::IpPacketResponseData::Data(r.into())
//            }
//            v7::response::IpPacketResponseData::Pong(r) => {
//                v6::response::IpPacketResponseData::Pong(r.into())
//            }
//            v7::response::IpPacketResponseData::Health(r) => {
//                v6::response::IpPacketResponseData::Health(r.into())
//            }
//            v7::response::IpPacketResponseData::Info(r) => {
//                v6::response::IpPacketResponseData::Info(r.into())
//            }
//        }
//    }
//}

// impl From<v7::response::StaticConnectResponse> for v6::response::StaticConnectResponse {
//     fn from(response: v7::response::StaticConnectResponse) -> Self {
//         v6::response::StaticConnectResponse {
//             request_id: response.request_id,
//             reply_to: response.reply_to,
//             reply: response.reply.into(),
//         }
//     }
// }
//
// impl From<v7::response::StaticConnectResponseReply> for v6::response::StaticConnectResponseReply {
//     fn from(reply: v7::response::StaticConnectResponseReply) -> Self {
//         match reply {
//             v7::response::StaticConnectResponseReply::Success => {
//                 v6::response::StaticConnectResponseReply::Success
//             }
//             v7::response::StaticConnectResponseReply::Failure(r) => {
//                 v6::response::StaticConnectResponseReply::Failure(r.into())
//             }
//         }
//     }
// }

impl From<v7::response::StaticConnectFailureReason> for v6::response::StaticConnectFailureReason {
    fn from(failure: v7::response::StaticConnectFailureReason) -> Self {
        match failure {
            v7::response::StaticConnectFailureReason::RequestedIpAlreadyInUse => {
                v6::response::StaticConnectFailureReason::RequestedIpAlreadyInUse
            }
            v7::response::StaticConnectFailureReason::RequestedNymAddressAlreadyInUse => {
                v6::response::StaticConnectFailureReason::RequestedNymAddressAlreadyInUse
            }
            v7::response::StaticConnectFailureReason::OutOfDateTimestamp => {
                v6::response::StaticConnectFailureReason::Other("out of date timestamp".to_string())
            }
            v7::response::StaticConnectFailureReason::Other(reason) => {
                v6::response::StaticConnectFailureReason::Other(reason)
            }
        }
    }
}

// impl From<v7::response::DynamicConnectResponse> for v6::response::DynamicConnectResponse {
//     fn from(response: v7::response::DynamicConnectResponse) -> Self {
//         v6::response::DynamicConnectResponse {
//             request_id: response.request_id,
//             reply_to: response.reply_to,
//             reply: response.reply.into(),
//         }
//     }
// }
//
// impl From<v7::response::DynamicConnectResponseReply> for v6::response::DynamicConnectResponseReply {
//     fn from(reply: v7::response::DynamicConnectResponseReply) -> Self {
//         match reply {
//             v7::response::DynamicConnectResponseReply::Success(r) => {
//                 v6::response::DynamicConnectResponseReply::Success(r.into())
//             }
//             v7::response::DynamicConnectResponseReply::Failure(r) => {
//                 v6::response::DynamicConnectResponseReply::Failure(r.into())
//             }
//         }
//     }
// }
//
// impl From<v7::response::DynamicConnectSuccess> for v6::response::DynamicConnectSuccess {
//     fn from(success: v7::response::DynamicConnectSuccess) -> Self {
//         v6::response::DynamicConnectSuccess { ips: success.ips }
//     }
// }
//
impl From<v7::response::DynamicConnectFailureReason> for v6::response::DynamicConnectFailureReason {
    fn from(failure: v7::response::DynamicConnectFailureReason) -> Self {
        match failure {
            v7::response::DynamicConnectFailureReason::RequestedNymAddressAlreadyInUse => {
                v6::response::DynamicConnectFailureReason::RequestedNymAddressAlreadyInUse
            }
            v7::response::DynamicConnectFailureReason::NoAvailableIp => {
                v6::response::DynamicConnectFailureReason::NoAvailableIp
            }
            v7::response::DynamicConnectFailureReason::Other(err) => {
                v6::response::DynamicConnectFailureReason::Other(err)
            }
        }
    }
}
//
// impl From<v7::response::DisconnectResponse> for v6::response::DisconnectResponse {
//     fn from(response: v7::response::DisconnectResponse) -> Self {
//         v6::response::DisconnectResponse {
//             request_id: response.request_id,
//             reply_to: response.reply_to,
//             reply: response.reply.into(),
//         }
//     }
// }
//
// impl From<v7::response::DisconnectResponseReply> for v6::response::DisconnectResponseReply {
//     fn from(reply: v7::response::DisconnectResponseReply) -> Self {
//         match reply {
//             v7::response::DisconnectResponseReply::Success => {
//                 v6::response::DisconnectResponseReply::Success
//             }
//             v7::response::DisconnectResponseReply::Failure(r) => {
//                 v6::response::DisconnectResponseReply::Failure(r.into())
//             }
//         }
//     }
// }
//
// impl From<v7::response::DisconnectFailureReason> for v6::response::DisconnectFailureReason {
//     fn from(failure: v7::response::DisconnectFailureReason) -> Self {
//         match failure {
//             v7::response::DisconnectFailureReason::RequestedNymAddressNotConnected => {
//                 v6::response::DisconnectFailureReason::RequestedNymAddressNotConnected
//             }
//             v7::response::DisconnectFailureReason::Other(err) => {
//                 v6::response::DisconnectFailureReason::Other(err)
//             }
//         }
//     }
// }
//
// impl From<v7::response::UnrequestedDisconnect> for v6::response::UnrequestedDisconnect {
//     fn from(response: v7::response::UnrequestedDisconnect) -> Self {
//         v6::response::UnrequestedDisconnect {
//             reply_to: response.reply_to,
//             reason: response.reason.into(),
//         }
//     }
// }
//
// impl From<v7::response::UnrequestedDisconnectReason> for v6::response::UnrequestedDisconnectReason {
//     fn from(reason: v7::response::UnrequestedDisconnectReason) -> Self {
//         match reason {
//             v7::response::UnrequestedDisconnectReason::ClientMixnetTrafficTimeout => {
//                 v6::response::UnrequestedDisconnectReason::ClientMixnetTrafficTimeout
//             }
//             v7::response::UnrequestedDisconnectReason::ClientTunTrafficTimeout => {
//                 v6::response::UnrequestedDisconnectReason::ClientTunTrafficTimeout
//             }
//             v7::response::UnrequestedDisconnectReason::Other(err) => {
//                 v6::response::UnrequestedDisconnectReason::Other(err)
//             }
//         }
//     }
// }
//
// impl From<v7::response::DataResponse> for v6::response::DataResponse {
//     fn from(response: v7::response::DataResponse) -> Self {
//         v6::response::DataResponse {
//             ip_packet: response.ip_packet,
//         }
//     }
// }
//
// impl From<v7::response::PongResponse> for v6::response::PongResponse {
//     fn from(response: v7::response::PongResponse) -> Self {
//         v6::response::PongResponse {
//             request_id: response.request_id,
//             reply_to: response.reply_to,
//         }
//     }
// }
//
// impl From<v7::response::HealthResponse> for v6::response::HealthResponse {
//     fn from(response: v7::response::HealthResponse) -> Self {
//         v6::response::HealthResponse {
//             request_id: response.request_id,
//             reply_to: response.reply_to,
//             reply: response.reply.into(),
//         }
//     }
// }
//
// impl From<v7::response::HealthResponseReply> for v6::response::HealthResponseReply {
//     fn from(reply: v7::response::HealthResponseReply) -> Self {
//         v6::response::HealthResponseReply {
//             build_info: reply.build_info,
//             routable: reply.routable,
//         }
//     }
// }
//
// impl From<v7::response::InfoResponse> for v6::response::InfoResponse {
//     fn from(response: v7::response::InfoResponse) -> Self {
//         v6::response::InfoResponse {
//             request_id: response.request_id,
//             reply_to: response.reply_to,
//             reply: response.reply.into(),
//             level: response.level.into(),
//         }
//     }
// }
//
impl From<v7::response::InfoResponseReply> for v6::response::InfoResponseReply {
    fn from(reply: v7::response::InfoResponseReply) -> Self {
        match reply {
            v7::response::InfoResponseReply::Generic { msg } => {
                v6::response::InfoResponseReply::Generic { msg }
            }
            v7::response::InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            } => v6::response::InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            },
            v7::response::InfoResponseReply::ExitPolicyFilterCheckFailed { dst } => {
                v6::response::InfoResponseReply::ExitPolicyFilterCheckFailed { dst }
            }
        }
    }
}

impl From<v7::response::InfoLevel> for v6::response::InfoLevel {
    fn from(level: v7::response::InfoLevel) -> Self {
        match level {
            v7::response::InfoLevel::Info => v6::response::InfoLevel::Info,
            v7::response::InfoLevel::Warn => v6::response::InfoLevel::Warn,
            v7::response::InfoLevel::Error => v6::response::InfoLevel::Error,
        }
    }
}

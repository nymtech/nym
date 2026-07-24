// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_ip_packet_requests::v10::response::{
    ConnectResponse as ConnectResponseV10, ConnectResponseReply as ConnectResponseReplyV10,
    ConnectSuccess as ConnectSuccessV10, ControlResponse as ControlResponseV10,
    IpPacketResponse as IpPacketResponseV10, IpPacketResponseData as IpPacketResponseDataV10,
};
// Non-connect branches reuse the v8 wire types, so v8's `From` impls apply.
use nym_ip_packet_requests::v8::response::{
    DisconnectResponse as DisconnectResponseV8, HealthResponse as HealthResponseV8,
    InfoResponse as InfoResponseV8, PongResponse as PongResponseV8,
};

use crate::error::IpPacketRouterError;

use super::{DynamicConnectResponse, DynamicConnectSuccess, Response, VersionedResponse};

impl TryFrom<VersionedResponse> for IpPacketResponseV10 {
    type Error = IpPacketRouterError;

    fn try_from(response: VersionedResponse) -> Result<Self, Self::Error> {
        let version = response.version.into_u8();
        let data = match response.response {
            Response::DynamicConnect { request_id, reply } => IpPacketResponseDataV10::Control(
                Box::new(ControlResponseV10::Connect(ConnectResponseV10 {
                    request_id,
                    reply: reply.into(),
                })),
            ),
            Response::Disconnect { request_id, reply } => IpPacketResponseDataV10::Control(
                Box::new(ControlResponseV10::Disconnect(DisconnectResponseV8 {
                    request_id,
                    reply: reply.into(),
                })),
            ),
            Response::Pong { request_id } => IpPacketResponseDataV10::Control(Box::new(
                ControlResponseV10::Pong(PongResponseV8 { request_id }),
            )),
            Response::Health { request_id, reply } => IpPacketResponseDataV10::Control(Box::new(
                ControlResponseV10::Health(Box::new(HealthResponseV8 {
                    request_id,
                    reply: (*reply).into(),
                })),
            )),
            Response::Info { request_id, reply } => IpPacketResponseDataV10::Control(Box::new(
                ControlResponseV10::Info(InfoResponseV8 {
                    request_id,
                    reply: reply.reply.into(),
                    level: reply.level.into(),
                }),
            )),
        };

        Ok(IpPacketResponseV10 { version, data })
    }
}

impl From<DynamicConnectResponse> for ConnectResponseReplyV10 {
    fn from(reply: DynamicConnectResponse) -> Self {
        match reply {
            DynamicConnectResponse::Success(DynamicConnectSuccess { ips }) => {
                // Report the egress TUN MTU so the client can size its MTU to fit.
                ConnectResponseReplyV10::Success(ConnectSuccessV10 {
                    ips,
                    mtu: nym_tun::configured_ipr_tun_mtu(),
                })
            }
            DynamicConnectResponse::Failure(err) => ConnectResponseReplyV10::Failure(err.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_ip_packet_requests::IpPair;
    use nym_ip_packet_requests::response_helpers::parse_connect_response_v10;
    use std::net::{Ipv4Addr, Ipv6Addr};

    // Server builds a v10 success (stamping its TUN MTU), wire round-trips, client
    // parser reads back the IPs + MTU.
    #[test]
    fn connect_success_reports_tun_mtu_across_the_wire() {
        let ips = IpPair::new(Ipv4Addr::new(10, 0, 0, 2), Ipv6Addr::LOCALHOST);

        // Server side: internal success -> v10 reply (MTU stamped here).
        let reply: ConnectResponseReplyV10 =
            DynamicConnectResponse::Success(DynamicConnectSuccess { ips }).into();
        let response = IpPacketResponseV10 {
            version: nym_ip_packet_requests::v10::VERSION,
            data: IpPacketResponseDataV10::Control(Box::new(ControlResponseV10::Connect(
                ConnectResponseV10 {
                    request_id: 7,
                    reply,
                },
            ))),
        };

        let bytes = response.to_bytes().unwrap();
        let decoded = IpPacketResponseV10::from_bytes(&bytes).unwrap();

        // Client side: the shared parser extracts IPs + MTU.
        let success = parse_connect_response_v10(decoded).unwrap();
        assert_eq!(success.ips, ips);
        assert_eq!(success.mtu, nym_tun::configured_ipr_tun_mtu());
    }
}

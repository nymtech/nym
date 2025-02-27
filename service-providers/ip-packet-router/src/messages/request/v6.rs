// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_ip_packet_requests::v6::request::{
    DataRequest as DataRequestV6, DisconnectRequest as DisconnectRequestV6,
    DynamicConnectRequest as DynamicConnectRequestV6, HealthRequest as HealthRequestV6,
    IpPacketRequest as IpPacketRequestV6, IpPacketRequestData as IpPacketRequestDataV6,
    PingRequest as PingRequestV6, StaticConnectRequest as StaticConnectRequestV6,
};

use crate::error::IpPacketRouterError;

use super::{
    ClientVersion, ControlRequest, DataRequest, DisconnectRequest, DynamicConnectRequest,
    HealthRequest, IpPacketRequest, PingRequest, StaticConnectRequest,
};

impl TryFrom<IpPacketRequestV6> for IpPacketRequest {
    type Error = IpPacketRouterError;

    fn try_from(request: IpPacketRequestV6) -> Result<Self, Self::Error> {
        let version = ClientVersion::V6;
        Ok(match request.data {
            IpPacketRequestDataV6::Data(inner) => Self::Data((inner, version).into()),
            IpPacketRequestDataV6::StaticConnect(inner) => {
                Self::Control(ControlRequest::StaticConnect((inner, version).into()))
            }
            IpPacketRequestDataV6::DynamicConnect(inner) => {
                Self::Control(ControlRequest::DynamicConnect((inner, version).into()))
            }
            IpPacketRequestDataV6::Disconnect(inner) => {
                Self::Control(ControlRequest::Disconnect((inner, version).into()))
            }
            IpPacketRequestDataV6::Ping(inner) => {
                Self::Control(ControlRequest::Ping((inner, version).into()))
            }
            IpPacketRequestDataV6::Health(inner) => {
                Self::Control(ControlRequest::Health((inner, version).into()))
            }
        })
    }
}

impl From<(DataRequestV6, ClientVersion)> for DataRequest {
    fn from((request, version): (DataRequestV6, ClientVersion)) -> Self {
        Self {
            version,
            ip_packets: request.ip_packets,
        }
    }
}

impl From<(StaticConnectRequestV6, ClientVersion)> for StaticConnectRequest {
    fn from((request, version): (StaticConnectRequestV6, ClientVersion)) -> Self {
        Self {
            version,
            request_id: request.request_id,
            sent_by: request.reply_to.into(),
            ips: request.ips,
            buffer_timeout: request.buffer_timeout,
        }
    }
}

impl From<(DynamicConnectRequestV6, ClientVersion)> for DynamicConnectRequest {
    fn from((request, version): (DynamicConnectRequestV6, ClientVersion)) -> Self {
        Self {
            version,
            request_id: request.request_id,
            sent_by: request.reply_to.into(),
            buffer_timeout: request.buffer_timeout,
        }
    }
}

impl From<(DisconnectRequestV6, ClientVersion)> for DisconnectRequest {
    fn from((request, version): (DisconnectRequestV6, ClientVersion)) -> Self {
        Self {
            version,
            request_id: request.request_id,
            sent_by: request.reply_to.into(),
        }
    }
}

impl From<(PingRequestV6, ClientVersion)> for PingRequest {
    fn from((request, version): (PingRequestV6, ClientVersion)) -> Self {
        Self {
            version,
            request_id: request.request_id,
            sent_by: request.reply_to.into(),
        }
    }
}

impl From<(HealthRequestV6, ClientVersion)> for HealthRequest {
    fn from((request, version): (HealthRequestV6, ClientVersion)) -> Self {
        Self {
            version,
            request_id: request.request_id,
            sent_by: request.reply_to.into(),
        }
    }
}

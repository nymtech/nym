// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_ip_packet_requests::v7::request::{
    DataRequest as DataRequestV7, DisconnectRequest as DisconnectRequestV7,
    DynamicConnectRequest as DynamicConnectRequestV7, HealthRequest as HealthRequestV7,
    IpPacketRequest as IpPacketRequestV7, IpPacketRequestData as IpPacketRequestDataV7,
    PingRequest as PingRequestV7, StaticConnectRequest as StaticConnectRequestV7,
};

use crate::error::IpPacketRouterError;

use super::{
    ClientVersion, ControlRequest, DataRequest, DisconnectRequest, DynamicConnectRequest,
    HealthRequest, IpPacketRequest, PingRequest, StaticConnectRequest,
};

impl From<IpPacketRequestV7> for IpPacketRequest {
    fn from(request: IpPacketRequestV7) -> Result<Self, Self::Error> {
        let version = ClientVersion::V7;
        match request.data {
            IpPacketRequestDataV7::Data(inner) => Self::Data((inner, version).into()),
            IpPacketRequestDataV7::StaticConnect(inner) => Self::Control(
                ControlRequest::StaticConnect((inner.request, version).into()),
            ),
            IpPacketRequestDataV7::DynamicConnect(inner) => Self::Control(
                ControlRequest::DynamicConnect((inner.request, version).into()),
            ),
            IpPacketRequestDataV7::Disconnect(inner) => {
                Self::Control(ControlRequest::Disconnect((inner.request, version).into()))
            }
            IpPacketRequestDataV7::Ping(inner) => {
                Self::Control(ControlRequest::Ping((inner, version).into()))
            }
            IpPacketRequestDataV7::Health(inner) => {
                Self::Control(ControlRequest::Health((inner, version).into()))
            }
        }
    }
}

impl From<(DataRequestV7, ClientVersion)> for DataRequest {
    fn from((request, version): (DataRequestV7, ClientVersion)) -> Self {
        Self {
            version,
            ip_packets: request.ip_packets,
        }
    }
}

impl From<(StaticConnectRequestV7, ClientVersion)> for StaticConnectRequest {
    fn from((request, version): (StaticConnectRequestV7, ClientVersion)) -> Self {
        Self {
            version,
            request_id: request.request_id,
            sent_by: request.reply_to.into(),
            ips: request.ips,
            buffer_timeout: request.buffer_timeout,
        }
    }
}

impl From<(DynamicConnectRequestV7, ClientVersion)> for DynamicConnectRequest {
    fn from((request, version): (DynamicConnectRequestV7, ClientVersion)) -> Self {
        Self {
            version,
            request_id: request.request_id,
            sent_by: request.reply_to.into(),
            buffer_timeout: request.buffer_timeout,
        }
    }
}

impl From<(DisconnectRequestV7, ClientVersion)> for DisconnectRequest {
    fn from((request, version): (DisconnectRequestV7, ClientVersion)) -> Self {
        Self {
            version,
            request_id: request.request_id,
            sent_by: request.reply_to.into(),
        }
    }
}

impl From<(PingRequestV7, ClientVersion)> for PingRequest {
    fn from((request, version): (PingRequestV7, ClientVersion)) -> Self {
        Self {
            version,
            request_id: request.request_id,
            sent_by: request.reply_to.into(),
        }
    }
}

impl From<(HealthRequestV7, ClientVersion)> for HealthRequest {
    fn from((request, version): (HealthRequestV7, ClientVersion)) -> Self {
        Self {
            version,
            request_id: request.request_id,
            sent_by: request.reply_to.into(),
        }
    }
}

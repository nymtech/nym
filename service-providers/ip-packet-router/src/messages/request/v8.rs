// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_ip_packet_requests::v8::request::{
    ConnectRequest as ConnectRequestV8, ControlRequest as ControlRequestV8,
    DataRequest as DataRequestV8, DisconnectRequest as DisconnectRequestV8,
    HealthRequest as HealthRequestV8, IpPacketRequest as IpPacketRequestV8,
    IpPacketRequestData as IpPacketRequestDataV8, PingRequest as PingRequestV8,
};
use nym_sdk::mixnet::AnonymousSenderTag;

use super::{
    ClientVersion, ControlRequest, DataRequest, DisconnectRequest, DynamicConnectRequest,
    HealthRequest, IpPacketRequest, PingRequest,
};

impl From<(IpPacketRequestV8, AnonymousSenderTag)> for IpPacketRequest {
    fn from((request, sender_tag): (IpPacketRequestV8, AnonymousSenderTag)) -> Self {
        let version = ClientVersion::V8;
        match request.data {
            IpPacketRequestDataV8::Data(inner) => Self::Data((inner, version).into()),
            IpPacketRequestDataV8::Control(inner) => {
                Self::Control((*inner, sender_tag, version).into())
            }
        }
    }
}

impl From<(DataRequestV8, ClientVersion)> for DataRequest {
    fn from((request, version): (DataRequestV8, ClientVersion)) -> Self {
        Self {
            version,
            ip_packets: request.ip_packets,
        }
    }
}

impl From<(ControlRequestV8, AnonymousSenderTag, ClientVersion)> for ControlRequest {
    fn from(
        (request, sender_tag, version): (ControlRequestV8, AnonymousSenderTag, ClientVersion),
    ) -> Self {
        match request {
            ControlRequestV8::Connect(inner) => {
                ControlRequest::DynamicConnect((inner, sender_tag, version).into())
            }
            ControlRequestV8::Disconnect(inner) => {
                ControlRequest::Disconnect((inner, sender_tag, version).into())
            }
            ControlRequestV8::Ping(inner) => {
                ControlRequest::Ping((inner, sender_tag, version).into())
            }
            ControlRequestV8::Health(inner) => {
                ControlRequest::Health((inner, sender_tag, version).into())
            }
        }
    }
}

impl From<(ConnectRequestV8, AnonymousSenderTag, ClientVersion)> for DynamicConnectRequest {
    fn from(
        (request, sender_tag, version): (ConnectRequestV8, AnonymousSenderTag, ClientVersion),
    ) -> Self {
        Self {
            version,
            request_id: request.request_id,
            sent_by: sender_tag.into(),
            buffer_timeout: request.buffer_timeout,
        }
    }
}

impl From<(DisconnectRequestV8, AnonymousSenderTag, ClientVersion)> for DisconnectRequest {
    fn from(
        (request, sender_tag, version): (DisconnectRequestV8, AnonymousSenderTag, ClientVersion),
    ) -> Self {
        Self {
            version,
            request_id: request.request_id,
            sent_by: sender_tag.into(),
        }
    }
}

impl From<(PingRequestV8, AnonymousSenderTag, ClientVersion)> for PingRequest {
    fn from(
        (request, sender_tag, version): (PingRequestV8, AnonymousSenderTag, ClientVersion),
    ) -> Self {
        Self {
            version,
            request_id: request.request_id,
            sent_by: sender_tag.into(),
        }
    }
}

impl From<(HealthRequestV8, AnonymousSenderTag, ClientVersion)> for HealthRequest {
    fn from(
        (request, sender_tag, version): (HealthRequestV8, AnonymousSenderTag, ClientVersion),
    ) -> Self {
        Self {
            version,
            request_id: request.request_id,
            sent_by: sender_tag.into(),
        }
    }
}

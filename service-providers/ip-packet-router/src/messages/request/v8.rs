// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_ip_packet_requests::v8::request::{
    ControlRequest as ControlRequestV8, DataRequest as DataRequestV8,
    DisconnectRequest as DisconnectRequestV8, DynamicConnectRequest as DynamicConnectRequestV8,
    HealthRequest as HealthRequestV8, IpPacketRequest as IpPacketRequestV8,
    IpPacketRequestData as IpPacketRequestDataV8, PingRequest as PingRequestV8,
    StaticConnectRequest as StaticConnectRequestV8,
};
use nym_sdk::mixnet::AnonymousSenderTag;

use crate::error::IpPacketRouterError;

use super::{
    ClientVersion, ConnectedClientId, ControlRequest, DataRequest, DisconnectRequest,
    DynamicConnectRequest, HealthRequest, IpPacketRequest, PingRequest, StaticConnectRequest,
};

impl TryFrom<(IpPacketRequestV8, Option<AnonymousSenderTag>, ClientVersion)> for IpPacketRequest {
    type Error = IpPacketRouterError;

    fn try_from(
        (request, sender_tag, version): (
            IpPacketRequestV8,
            Option<AnonymousSenderTag>,
            ClientVersion,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(match request.data {
            IpPacketRequestDataV8::Data(inner) => Self::Data((inner, version).into()),
            IpPacketRequestDataV8::Control(inner) => {
                Self::Control((*inner, sender_tag, version).try_into()?)
            }
        })
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

impl TryFrom<(ControlRequestV8, Option<AnonymousSenderTag>, ClientVersion)> for ControlRequest {
    type Error = IpPacketRouterError;

    fn try_from(
        (request, sender_tag, version): (
            ControlRequestV8,
            Option<AnonymousSenderTag>,
            ClientVersion,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(match request {
            ControlRequestV8::StaticConnect(inner) => {
                ControlRequest::StaticConnect((inner.request, sender_tag, version).try_into()?)
            }
            ControlRequestV8::DynamicConnect(inner) => {
                ControlRequest::DynamicConnect((inner.request, sender_tag, version).try_into()?)
            }
            ControlRequestV8::Disconnect(inner) => {
                ControlRequest::Disconnect((inner.request, sender_tag, version).try_into()?)
            }
            ControlRequestV8::Ping(inner) => {
                ControlRequest::Ping((inner, sender_tag, version).try_into()?)
            }
            ControlRequestV8::Health(inner) => {
                ControlRequest::Health((inner, sender_tag, version).try_into()?)
            }
        })
    }
}

impl
    TryFrom<(
        StaticConnectRequestV8,
        Option<AnonymousSenderTag>,
        ClientVersion,
    )> for StaticConnectRequest
{
    type Error = IpPacketRouterError;

    fn try_from(
        (request, sender_tag, version): (
            StaticConnectRequestV8,
            Option<AnonymousSenderTag>,
            ClientVersion,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            version,
            request_id: request.request_id,
            sent_by: ConnectedClientId::try_from((request.sender, sender_tag))?,
            ips: request.ips,
            buffer_timeout: request.buffer_timeout,
        })
    }
}

impl
    TryFrom<(
        DynamicConnectRequestV8,
        Option<AnonymousSenderTag>,
        ClientVersion,
    )> for DynamicConnectRequest
{
    type Error = IpPacketRouterError;

    fn try_from(
        (request, sender_tag, version): (
            DynamicConnectRequestV8,
            Option<AnonymousSenderTag>,
            ClientVersion,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            version,
            request_id: request.request_id,
            sent_by: ConnectedClientId::try_from((request.sender, sender_tag))?,
            buffer_timeout: request.buffer_timeout,
        })
    }
}

impl
    TryFrom<(
        DisconnectRequestV8,
        Option<AnonymousSenderTag>,
        ClientVersion,
    )> for DisconnectRequest
{
    type Error = IpPacketRouterError;

    fn try_from(
        (request, sender_tag, version): (
            DisconnectRequestV8,
            Option<AnonymousSenderTag>,
            ClientVersion,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            version,
            request_id: request.request_id,
            sent_by: ConnectedClientId::try_from((request.sender, sender_tag))?,
        })
    }
}

impl TryFrom<(PingRequestV8, Option<AnonymousSenderTag>, ClientVersion)> for PingRequest {
    type Error = IpPacketRouterError;

    fn try_from(
        (request, sender_tag, version): (PingRequestV8, Option<AnonymousSenderTag>, ClientVersion),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            version,
            request_id: request.request_id,
            sent_by: ConnectedClientId::try_from((request.sender, sender_tag))?,
        })
    }
}

impl TryFrom<(HealthRequestV8, Option<AnonymousSenderTag>, ClientVersion)> for HealthRequest {
    type Error = IpPacketRouterError;

    fn try_from(
        (request, sender_tag, version): (
            HealthRequestV8,
            Option<AnonymousSenderTag>,
            ClientVersion,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            version,
            request_id: request.request_id,
            sent_by: ConnectedClientId::try_from((request.sender, sender_tag))?,
        })
    }
}

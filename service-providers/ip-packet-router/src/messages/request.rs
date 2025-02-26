// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use core::fmt;

use nym_ip_packet_requests::{
    v7::request::IpPacketRequest as IpPacketRequestV7,
    v8::request::{
        ControlRequest as ControlRequestV8, DataRequest as DataRequestV8,
        DisconnectRequest as DisconnectRequestV8, DynamicConnectRequest as DynamicConnectRequestV8,
        HealthRequest as HealthRequestV8, IpPacketRequest as IpPacketRequestV8,
        IpPacketRequestData as IpPacketRequestDataV8, PingRequest as PingRequestV8,
        StaticConnectRequest as StaticConnectRequestV8,
    },
    IpPair,
};
use nym_sdk::mixnet::{AnonymousSenderTag, ReconstructedMessage};

use crate::{clients::ConnectedClientId, error::IpPacketRouterError};

use super::ClientVersion;

// The internal representation of the request after deserialization, valid for all versions
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum IpPacketRequest {
    Data(DataRequest),
    Control(ControlRequest),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DataRequest {
    pub(crate) version: ClientVersion,
    pub(crate) ip_packets: bytes::Bytes,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ControlRequest {
    StaticConnect(StaticConnectRequest),
    DynamicConnect(DynamicConnectRequest),
    Disconnect(DisconnectRequest),
    Ping(PingRequest),
    Health(HealthRequest),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StaticConnectRequest {
    pub(crate) version: ClientVersion,
    pub(crate) request_id: u64,
    pub(crate) sent_by: ConnectedClientId,
    pub(crate) ips: IpPair,
    pub(crate) buffer_timeout: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DynamicConnectRequest {
    pub(crate) version: ClientVersion,
    pub(crate) request_id: u64,
    pub(crate) sent_by: ConnectedClientId,
    pub(crate) buffer_timeout: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisconnectRequest {
    pub(crate) version: ClientVersion,
    pub(crate) request_id: u64,
    pub(crate) sent_by: ConnectedClientId,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PingRequest {
    pub(crate) version: ClientVersion,
    pub(crate) request_id: u64,
    pub(crate) sent_by: ConnectedClientId,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HealthRequest {
    pub(crate) version: ClientVersion,
    pub(crate) request_id: u64,
    pub(crate) sent_by: ConnectedClientId,
}

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

impl TryFrom<&ReconstructedMessage> for IpPacketRequest {
    type Error = IpPacketRouterError;

    fn try_from(reconstructed: &ReconstructedMessage) -> Result<Self, Self::Error> {
        let request_version = *reconstructed
            .message
            .first()
            .ok_or(IpPacketRouterError::EmptyPacket)?;

        let (deserialized, version) = match request_version {
            7 => {
                let request_v7 = IpPacketRequestV7::from_reconstructed_message(reconstructed)
                    .map_err(
                        |source| IpPacketRouterError::FailedToDeserializeTaggedPacket { source },
                    )?;
                request_v7
                    .verify()
                    .map_err(|source| IpPacketRouterError::FailedToVerifyRequest { source })?;
                (IpPacketRequestV8::from(request_v7), ClientVersion::V7)
            }
            8 => {
                let request_v8 = IpPacketRequestV8::from_reconstructed_message(reconstructed)
                    .map_err(
                        |source| IpPacketRouterError::FailedToDeserializeTaggedPacket { source },
                    )?;
                request_v8
                    .verify()
                    .map_err(|source| IpPacketRouterError::FailedToVerifyRequest { source })?;
                (request_v8, ClientVersion::V8)
            }
            _ => {
                log::info!("Received packet with invalid version: v{request_version}");
                return Err(IpPacketRouterError::InvalidPacketVersion(request_version));
            }
        };

        IpPacketRequest::try_from((deserialized, reconstructed.sender_tag, version))
    }
}

impl fmt::Display for IpPacketRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpPacketRequest::Data(_) => write!(f, "Data"),
            IpPacketRequest::Control(control) => write!(f, "{control}"),
        }
    }
}

impl fmt::Display for ControlRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ControlRequest::StaticConnect(_) => write!(f, "StaticConnect"),
            ControlRequest::DynamicConnect(_) => write!(f, "DynamicConnect"),
            ControlRequest::Disconnect(_) => write!(f, "Disconnect"),
            ControlRequest::Ping(_) => write!(f, "Ping"),
            ControlRequest::Health(_) => write!(f, "Health"),
        }
    }
}

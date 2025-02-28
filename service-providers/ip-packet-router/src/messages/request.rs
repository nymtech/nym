// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

mod v6;
mod v7;
mod v8;

use nym_ip_packet_requests::{
    v6::request::IpPacketRequest as IpPacketRequestV6,
    v7::request::IpPacketRequest as IpPacketRequestV7,
    v8::request::IpPacketRequest as IpPacketRequestV8, IpPair,
};
use nym_sdk::mixnet::ReconstructedMessage;
use nym_service_provider_requests_common::{Protocol, ServiceProviderType};
use std::fmt;

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

impl TryFrom<&ReconstructedMessage> for IpPacketRequest {
    type Error = IpPacketRouterError;

    fn try_from(reconstructed: &ReconstructedMessage) -> Result<Self, Self::Error> {
        let request_version = *reconstructed
            .message
            .first_chunk::<2>()
            .ok_or(IpPacketRouterError::EmptyPacket)?;

        // With version v8 and onwards, the type of the service provider is included in the
        // header.
        if request_version[0] >= 8 {
            let protocol = Protocol::try_from(&request_version)
                .map_err(|source| IpPacketRouterError::FailedToDeserializeProtocol { source })?;

            if protocol.service_provider_type != ServiceProviderType::IpPacketRouter {
                return Err(IpPacketRouterError::InvalidServiceProviderType(
                    protocol.service_provider_type,
                ));
            }
        }

        let request_version = request_version[0];
        match request_version {
            6 => {
                let request_v6 = IpPacketRequestV6::from_reconstructed_message(reconstructed)
                    .map_err(
                        |source| IpPacketRouterError::FailedToDeserializeTaggedPacket { source },
                    )?;
                Ok(IpPacketRequest::from(request_v6))
            }
            7 => {
                let request_v7 = IpPacketRequestV7::from_reconstructed_message(reconstructed)
                    .map_err(
                        |source| IpPacketRouterError::FailedToDeserializeTaggedPacket { source },
                    )?;
                request_v7
                    .verify()
                    .map_err(|source| IpPacketRouterError::FailedToVerifyRequest { source })?;
                Ok(IpPacketRequest::from(request_v7))
            }
            8 => {
                let request_v8 = IpPacketRequestV8::from_reconstructed_message(reconstructed)
                    .map_err(
                        |source| IpPacketRouterError::FailedToDeserializeTaggedPacket { source },
                    )?;
                let sender_tag = reconstructed
                    .sender_tag
                    .ok_or(IpPacketRouterError::MissingSenderTag)?;
                Ok(IpPacketRequest::from((request_v8, sender_tag)))
            }
            _ => {
                log::info!("Received packet with invalid version: v{request_version}");
                Err(IpPacketRouterError::InvalidPacketVersion(request_version))
            }
        }
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

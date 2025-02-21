// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

mod request;
mod response;

pub(crate) use request::*;
pub(crate) use response::*;

use std::fmt;

use nym_ip_packet_requests::{v7, v8};
use nym_sdk::mixnet::{AnonymousSenderTag, ReconstructedMessage};

use crate::error::{IpPacketRouterError, Result};

// After deserializing incoming reconstructed messages, we support multiple versions of the request
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DeserializedIpPacketRequest {
    V7(v7::request::IpPacketRequest),
    V8((v8::request::IpPacketRequest, AnonymousSenderTag)),
}

impl DeserializedIpPacketRequest {
    pub(crate) fn version(&self) -> u8 {
        match self {
            DeserializedIpPacketRequest::V7(_) => 7,
            DeserializedIpPacketRequest::V8(_) => 8,
        }
    }

    pub(crate) fn verify(&self) -> Result<()> {
        match self {
            DeserializedIpPacketRequest::V7(request) => request.verify(),
            DeserializedIpPacketRequest::V8(request) => request.0.verify(),
        }
        .map_err(|err| IpPacketRouterError::FailedToVerifyRequest { source: err })
    }
}

impl fmt::Display for DeserializedIpPacketRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeserializedIpPacketRequest::V7(request) => write!(f, "{request}"),
            DeserializedIpPacketRequest::V8((request, _)) => write!(f, "{request}"),
        }
    }
}

impl From<v7::request::IpPacketRequest> for DeserializedIpPacketRequest {
    fn from(request: v7::request::IpPacketRequest) -> Self {
        DeserializedIpPacketRequest::V7(request)
    }
}

impl From<(v8::request::IpPacketRequest, AnonymousSenderTag)> for DeserializedIpPacketRequest {
    fn from(request: (v8::request::IpPacketRequest, AnonymousSenderTag)) -> Self {
        DeserializedIpPacketRequest::V8(request)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SupportedClientVersion {
    V7,
    V8,
}

impl SupportedClientVersion {
    pub(crate) fn new(request_version: u8) -> Option<Self> {
        match request_version {
            7 => Some(SupportedClientVersion::V7),
            8 => Some(SupportedClientVersion::V8),
            _ => None,
        }
    }

    pub(crate) fn into_u8(self) -> u8 {
        match self {
            SupportedClientVersion::V7 => 7,
            SupportedClientVersion::V8 => 8,
        }
    }
}

pub(crate) fn deserialize_request(
    reconstructed: &ReconstructedMessage,
) -> Result<(DeserializedIpPacketRequest, SupportedClientVersion)> {
    let request_version = *reconstructed
        .message
        .first()
        .ok_or(IpPacketRouterError::EmptyPacket)?;

    // Check version of the request and convert to the latest version if necessary
    let request = match request_version {
        7 => v7::request::IpPacketRequest::from_reconstructed_message(reconstructed)
            .map(DeserializedIpPacketRequest::from)
            .map_err(|source| IpPacketRouterError::FailedToDeserializeTaggedPacket { source }),
        8 => {
            let sender_tag = reconstructed
                .sender_tag
                .ok_or(IpPacketRouterError::EmptyPacket)?;
            v8::request::IpPacketRequest::from_reconstructed_message(reconstructed)
                .map(|r| DeserializedIpPacketRequest::from((r, sender_tag)))
                .map_err(|source| IpPacketRouterError::FailedToDeserializeTaggedPacket { source })
        }
        _ => {
            log::info!("Received packet with invalid version: v{request_version}");
            Err(IpPacketRouterError::InvalidPacketVersion(request_version))
        }
    };

    let Some(request_version) = SupportedClientVersion::new(request_version) else {
        return Err(IpPacketRouterError::InvalidPacketVersion(request_version));
    };

    // Tag the request with the version of the request
    request.map(|r| (r, request_version))
}

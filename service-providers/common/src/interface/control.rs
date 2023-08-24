// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::interface::{Serializable, ServiceProviderMessagingError};
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum ControlRequest {
    Health,
    BinaryInfo,
    SupportedRequestVersions,
}

#[repr(u8)]
enum ControlRequestTag {
    /// Value tag representing [`Health`] variant of the [`ControlRequest`]
    Health = 0x00,

    /// Value tag representing [`BinaryInfo`] variant of the [`ControlRequest`]
    BinaryInfo = 0x01,

    /// Value tag representing [`SupportedRequestVersions`] variant of the [`ControlRequest`]
    RequestVersions = 0x02,
}

impl TryFrom<u8> for ControlRequestTag {
    type Error = ServiceProviderMessagingError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (Self::Health as u8) => Ok(Self::Health),
            _ if value == (Self::BinaryInfo as u8) => Ok(Self::BinaryInfo),
            _ if value == (Self::RequestVersions as u8) => Ok(Self::RequestVersions),
            received => Err(ServiceProviderMessagingError::InvalidControlRequestTag { received }),
        }
    }
}

impl Serializable for ControlRequest {
    type Error = ServiceProviderMessagingError;

    fn into_bytes(self) -> Vec<u8> {
        // current variants do not require sending any data apart from the tag
        vec![self.tag() as u8]
    }

    fn try_from_bytes(b: &[u8]) -> Result<Self, ServiceProviderMessagingError> {
        if b.is_empty() {
            return Err(ServiceProviderMessagingError::EmptyControlRequest);
        }

        let request_tag = ControlRequestTag::try_from(b[0])?;
        match request_tag {
            ControlRequestTag::Health => Ok(ControlRequest::Health),
            ControlRequestTag::BinaryInfo => Ok(ControlRequest::BinaryInfo),
            ControlRequestTag::RequestVersions => Ok(ControlRequest::SupportedRequestVersions),
        }
    }
}

impl ControlRequest {
    fn tag(&self) -> ControlRequestTag {
        match self {
            ControlRequest::Health => ControlRequestTag::Health,
            ControlRequest::BinaryInfo => ControlRequestTag::BinaryInfo,
            ControlRequest::SupportedRequestVersions => ControlRequestTag::RequestVersions,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BinaryInformation {
    pub binary_name: String,
    pub build_information: BinaryBuildInformationOwned,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SupportedVersions {
    pub interface_version: String,
    pub provider_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    message: String,
}

#[derive(Debug, Serialize)]
pub enum ControlResponse {
    Health,
    BinaryInfo(Box<BinaryInformation>),
    SupportedRequestVersions(SupportedVersions),
    Error(ErrorResponse),
}

#[repr(u8)]
enum ControlResponseTag {
    /// Value tag representing [`Health`] variant of the [`ControlResponse`]
    Health = 0x00,

    /// Value tag representing [`BinaryInfo`] variant of the [`ControlResponse`]
    BinaryInfo = 0x01,

    /// Value tag representing [`SupportedRequestVersions`] variant of the [`ControlResponse`]
    SupportedRequestVersions = 0x02,

    /// Value tag representing [`Error`] variant of the [`ControlResponse`]
    Error = 0xFF,
}

impl TryFrom<u8> for ControlResponseTag {
    type Error = ServiceProviderMessagingError;

    fn try_from(value: u8) -> Result<Self, ServiceProviderMessagingError> {
        match value {
            _ if value == (Self::Health as u8) => Ok(Self::Health),
            _ if value == (Self::BinaryInfo as u8) => Ok(Self::BinaryInfo),
            _ if value == (Self::SupportedRequestVersions as u8) => {
                Ok(Self::SupportedRequestVersions)
            }
            _ if value == (Self::Error as u8) => Ok(Self::Error),
            received => Err(ServiceProviderMessagingError::InvalidControlResponseTag { received }),
        }
    }
}

impl Serializable for ControlResponse {
    type Error = ServiceProviderMessagingError;

    fn into_bytes(self) -> Vec<u8> {
        std::iter::once(self.tag() as u8)
            .chain(self.serialize_inner())
            .collect()
    }

    fn try_from_bytes(b: &[u8]) -> Result<Self, ServiceProviderMessagingError> {
        if b.is_empty() {
            return Err(ServiceProviderMessagingError::EmptyControlResponse);
        }

        let response_tag = ControlResponseTag::try_from(b[0])?;
        match response_tag {
            ControlResponseTag::Health => Ok(ControlResponse::Health),
            ControlResponseTag::BinaryInfo => match serde_json::from_slice(&b[1..]) {
                Ok(binary_info) => Ok(ControlResponse::BinaryInfo(binary_info)),
                Err(source) => Err(
                    ServiceProviderMessagingError::MalformedBinaryInfoControlResponse { source },
                ),
            },
            ControlResponseTag::SupportedRequestVersions => match serde_json::from_slice(&b[1..]) {
                Ok(supported_versions) => Ok(ControlResponse::SupportedRequestVersions(
                    supported_versions,
                )),
                Err(source) => {
                    Err(ServiceProviderMessagingError::MalformedErrorControlResponse { source })
                }
            },
            ControlResponseTag::Error => match serde_json::from_slice(&b[1..]) {
                Ok(error_response) => Ok(ControlResponse::Error(error_response)),
                Err(source) => {
                    Err(ServiceProviderMessagingError::MalformedErrorControlResponse { source })
                }
            },
        }
    }
}

impl ControlResponse {
    fn tag(&self) -> ControlResponseTag {
        match self {
            ControlResponse::Health => ControlResponseTag::Health,
            ControlResponse::BinaryInfo(_) => ControlResponseTag::BinaryInfo,
            ControlResponse::SupportedRequestVersions(_) => {
                ControlResponseTag::SupportedRequestVersions
            }
            ControlResponse::Error(_) => ControlResponseTag::Error,
        }
    }

    fn serialize_inner(self) -> Vec<u8> {
        match self {
            ControlResponse::Health => Vec::new(),
            // TODO: is serde_json the right choice for this?
            ControlResponse::BinaryInfo(info) => {
                // As per serde_json documentation:
                // ```
                // Serialization can fail if `T`'s implementation of `Serialize` decides to
                // fail, or if `T` contains a map with non-string keys.
                // ```
                //
                // And since `BinaryInformation` does not contain any maps and its serialization
                // is fully derived with serde macros, the below cannot possibly fail,
                // so the unwrap is fine
                // (unless the serde's macro is bugged but at this point we're already out of luck)
                serde_json::to_vec(&info).unwrap()
            }
            ControlResponse::SupportedRequestVersions(supported_versions) => {
                serde_json::to_vec(&supported_versions).unwrap()
            }
            ControlResponse::Error(error_response) => serde_json::to_vec(&error_response).unwrap(),
        }
    }
}

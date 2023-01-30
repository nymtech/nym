// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use control::{BinaryInformation, ControlRequest, ControlResponse, SupportedVersions};
pub use request::{Request, RequestContent, ServiceProviderRequest};
pub use response::{Response, ResponseContent, ServiceProviderResponse};
pub use version::{ProviderInterfaceVersion, RequestVersion, Version};

use std::fmt::{Display, Formatter};
use thiserror::Error;

mod control;
mod request;
mod response;
mod version;

#[derive(Debug, Error)]
pub enum ServiceProviderMessagingError {
    #[error("{received} does not correspond to any valid request tag")]
    InvalidRequestTag { received: u8 },

    #[error("{received} does not correspond to any valid response tag")]
    InvalidResponseTag { received: u8 },

    #[error("{received} does not correspond to any valid control request tag")]
    InvalidControlRequestTag { received: u8 },

    #[error("{received} does not correspond to any valid control response tag")]
    InvalidControlResponseTag { received: u8 },

    #[error("request did not contain any data")]
    EmptyRequest,

    #[error("response did not contain any data")]
    EmptyResponse,

    #[error("request did not contain enough data to get deserialized. Got only {received} bytes.")]
    IncompleteRequest { received: usize },

    #[error(
        "response did not contain enough data to get deserialized. Got only {received} bytes."
    )]
    IncompleteResponse { received: usize },

    #[error("control request did not contain any data")]
    EmptyControlRequest,

    #[error("control response did not contain any data")]
    EmptyControlResponse,

    #[error("Received provider data response while control was expected")]
    UnexpectedProviderDataResponse,

    #[error("Received control response while provider data was expected")]
    UnexpectedControlResponse,

    #[error("Attempted to use control request in 'Legacy' mode")]
    ControlRequestInLegacyMode,

    #[error("the received binary information control response was malformed: {source}")]
    MalformedBinaryInfoControlResponse { source: serde_json::Error },

    #[error("the received error control response was malformed: {source}")]
    MalformedErrorControlResponse { source: serde_json::Error },

    #[error("the received supported versions control response was malformed: {source}")]
    MalformedSupportedVersionsResponse { source: serde_json::Error },
}

// can't use 'normal' trait (i.e. Serialize/Deserialize from serde) as `Socks5Message` uses custom serialization
// and we don't want to break backwards compatibility, plus being able to know the expected protocol version
// ahead of time is very useful.
pub trait Serializable: Sized {
    type Error;

    fn into_bytes(self) -> Vec<u8>;

    fn try_from_bytes(b: &[u8]) -> Result<Self, Self::Error>;
}

#[derive(Debug)]
pub struct EmptyMessage;

#[derive(Debug, Clone)]
pub struct Empty;

impl Display for Empty {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "n/a")
    }
}

impl Version for Empty {}

impl ServiceProviderRequest for EmptyMessage {
    type ProtocolVersion = Empty;
    type Response = EmptyMessage;
    type Error = ServiceProviderMessagingError;

    fn provider_specific_version(&self) -> Self::ProtocolVersion {
        Empty
    }

    fn max_supported_version() -> Self::ProtocolVersion {
        Empty
    }
}

impl ServiceProviderResponse for EmptyMessage {}

impl Serializable for EmptyMessage {
    type Error = ServiceProviderMessagingError;

    fn into_bytes(self) -> Vec<u8> {
        Vec::new()
    }

    fn try_from_bytes(_b: &[u8]) -> Result<Self, Self::Error> {
        Ok(EmptyMessage)
    }
}
